//! Linux x86_64 diagnostic template; compiled only in the isolated update profiler.
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    os::fd::{AsRawFd, FromRawFd},
    ptr,
    sync::atomic::{Ordering, fence},
    time::Duration,
};

#[derive(Debug, PartialEq)]
struct Record {
    kind: u32,
    misc: u16,
    payload: Vec<u8>,
}

fn decode(bytes: &[u8]) -> io::Result<Vec<Record>> {
    let mut records = Vec::new();
    let mut offset = 0;
    while offset < bytes.len() {
        if bytes.len() - offset < 8 {
            return Err(io::Error::other("truncated perf header"));
        }
        let kind = u32::from_ne_bytes(bytes[offset..offset + 4].try_into().unwrap());
        let misc = u16::from_ne_bytes(bytes[offset + 4..offset + 6].try_into().unwrap());
        let size = u16::from_ne_bytes(bytes[offset + 6..offset + 8].try_into().unwrap()) as usize;
        if size < 8 || size > bytes.len() - offset || (kind == 9 && size != 16) {
            return Err(io::Error::other("invalid perf record size"));
        }
        records.push(Record {
            kind,
            misc,
            payload: bytes[offset + 8..offset + size].to_vec(),
        });
        offset += size;
    }
    Ok(records)
}

// Caller supplies the mapped ring; only committed, not-yet-released bytes are copied.
unsafe fn ring_bytes(ring: *const u8, size: usize, tail: u64, head: u64) -> io::Result<Vec<u8>> {
    let count = head
        .checked_sub(tail)
        .filter(|n| *n <= size as u64)
        .filter(|_| size != 0)
        .ok_or_else(|| io::Error::other("perf ring overrun"))? as usize;
    let start = tail as usize % size;
    let first = count.min(size - start);
    let mut bytes = vec![0; count];
    unsafe {
        ptr::copy_nonoverlapping(ring.add(start), bytes.as_mut_ptr(), first);
        ptr::copy_nonoverlapping(ring, bytes.as_mut_ptr().add(first), count - first);
    }
    Ok(bytes)
}

pub struct Sampler {
    file: File,
    mapping: *mut u8,
    mapping_len: usize,
    offset: usize,
    size: usize,
    samples: Vec<(usize, Duration, Vec<Record>)>,
}
impl Sampler {
    pub fn new(samples: usize) -> io::Result<Self> {
        let mut attr = [0_u64; 8];
        attr[0] = 64_u64 << 32;
        attr[1] = 3; // PERF_COUNT_HW_CACHE_MISSES
        attr[2] = 512;
        attr[3] = 1; // PERF_SAMPLE_IP
        attr[4] = 3; // time enabled/running
        attr[5] = 1 | (1 << 5) | (1 << 6); // disabled, user-only, precise_ip=0
        let fd = unsafe { libc::syscall(libc::SYS_perf_event_open, attr.as_ptr(), 0, -1, -1, 8) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        let file = unsafe { File::from_raw_fd(fd as i32) };
        let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
        if page < 4096 {
            return Err(io::Error::other("unsupported perf metadata page"));
        }
        let mapping_len = page as usize * 9;
        let mapping = unsafe {
            libc::mmap(
                ptr::null_mut(),
                mapping_len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                file.as_raw_fd(),
                0,
            )
        };
        if mapping == libc::MAP_FAILED {
            return Err(io::Error::last_os_error());
        }
        let mut this = Self {
            file,
            mapping: mapping.cast(),
            mapping_len,
            offset: 0,
            size: 0,
            samples: Vec::with_capacity(samples + 20),
        };
        // Linux perf_event_mmap_page ABI; the build script verifies these header offsets.
        this.offset = unsafe { ptr::read_volatile(this.mapping.add(1040).cast::<u64>()) } as usize;
        this.size = unsafe { ptr::read_volatile(this.mapping.add(1048).cast::<u64>()) } as usize;
        if this.offset != page as usize || this.size != page as usize * 8 {
            return Err(io::Error::other("unexpected perf ring layout"));
        }
        let path = std::env::var_os("FEATHER_IP_MAPS")
            .ok_or_else(|| io::Error::other("FEATHER_IP_MAPS required"))?;
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?
            .write_all(&fs::read("/proc/self/maps")?)?;
        Ok(this)
    }
    fn enabled(&self, enabled: bool) -> io::Result<()> {
        let command = if enabled { 0x2400 } else { 0x2401 };
        if unsafe { libc::ioctl(self.file.as_raw_fd(), command, 0) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
    pub fn before(&self) {
        self.enabled(true).expect("enable perf sampling");
    }
    pub fn after(&mut self, sample: usize, wall: Duration) {
        self.enabled(false).expect("disable perf sampling");
        let head = unsafe { ptr::read_volatile(self.mapping.add(1024).cast::<u64>()) };
        fence(Ordering::Acquire);
        let tail = unsafe { ptr::read_volatile(self.mapping.add(1032).cast::<u64>()) };
        // Only committed ring bytes are read; the kernel may not overwrite them until tail advances.
        let bytes = unsafe { ring_bytes(self.mapping.add(self.offset), self.size, tail, head) }
            .expect("complete perf ring");
        let records = decode(&bytes).expect("valid perf records");
        fence(Ordering::SeqCst);
        unsafe {
            ptr::write_volatile(self.mapping.add(1032).cast::<u64>(), head);
        }
        self.samples.push((sample, wall, records));
    }
    pub fn report(&mut self) {
        let mut bytes = [0; 24];
        self.file
            .read_exact(&mut bytes)
            .expect("perf counter totals");
        let totals: [u64; 3] = std::array::from_fn(|i| {
            u64::from_ne_bytes(bytes[i * 8..(i + 1) * 8].try_into().unwrap())
        });
        println!(
            "diagnostic_ip_sampling=true event=cache-misses period=512 precise_ip_requested=0 count={} enabled_ns={} running_ns={}",
            totals[0], totals[1], totals[2]
        );
        for (sample, wall, records) in &self.samples {
            println!("sample_update sample={sample} wall_ns={}", wall.as_nanos());
            for record in records {
                if record.kind == 9 {
                    println!(
                        "sample_ip sample={sample} ip=0x{:x} misc=0x{:x}",
                        u64::from_ne_bytes(record.payload.as_slice().try_into().unwrap()),
                        record.misc
                    );
                } else {
                    println!(
                        "sample_other sample={sample} kind={} misc={} payload={:x?}",
                        record.kind, record.misc, record.payload
                    );
                }
            }
        }
    }
}
impl Drop for Sampler {
    fn drop(&mut self) {
        let _ = self.enabled(false);
        unsafe {
            libc::munmap(self.mapping.cast(), self.mapping_len);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wrapped_samples_retain_identity_and_reject_corruption() {
        let mut record = vec![];
        record.extend(9_u32.to_ne_bytes());
        record.extend(0x4000_u16.to_ne_bytes());
        record.extend(16_u16.to_ne_bytes());
        record.extend(0x1234_u64.to_ne_bytes());
        let mut ring = [0; 32];
        for (i, byte) in record.iter().enumerate() {
            ring[(28 + i) % 32] = *byte;
        }
        let copy = |tail, head| unsafe { ring_bytes(ring.as_ptr(), ring.len(), tail, head) };
        let bytes = copy(28, 44).unwrap();
        assert_eq!(bytes, record);
        assert_eq!(
            decode(&bytes).unwrap(),
            vec![Record {
                kind: 9,
                misc: 0x4000,
                payload: 0x1234_u64.to_ne_bytes().to_vec(),
            }]
        );
        assert!(decode(&bytes[..15]).is_err());
        assert!(decode(&bytes[..3]).is_err());
        assert!(decode(&[0; 8]).is_err());
        let mut wrong_layout = bytes.clone();
        wrong_layout[6..8].copy_from_slice(&24_u16.to_ne_bytes());
        wrong_layout.extend([0; 8]);
        assert!(decode(&wrong_layout).is_err());
        assert!(copy(0, 33).is_err());
        assert!(copy(10, 9).is_err());
        assert!(unsafe { ring_bytes(std::ptr::null(), 0, 0, 0) }.is_err());
    }
}
