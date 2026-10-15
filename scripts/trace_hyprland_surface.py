"""Bounded, version-pinned compositor observation. Requires sudo; changes no settings."""
import argparse
from contextlib import ExitStack, contextmanager
import ctypes
import hashlib
import json
import mmap
import os
from pathlib import Path
import struct
import time

from profile_frame_schedule import drain_ring

HYPRLAND = Path('/usr/bin/Hyprland')
WAYLAND = Path('/usr/lib/libwayland-server.so.0')
HASHES = {HYPRLAND: '55da553be71222566ee73b973d2f56dbb9939044d8f22f81aa54b66c83f2f6d1',
          WAYLAND: '7e89d8a177a78343a7cf92e3c1c756dc98a437489de4861ff6de907c7e48af89'}
# shortcut: offsets support only the hashed binaries; reverify entries and getters after upgrades.
SYMBOLS = {'schedule': 0x9628b0, 'commit': 0x966d20, 'feedback': 0x963090}


def file_offset(elf, address):
    if elf[:6] != b'\x7fELF\x02\x01':
        raise ValueError('requires little-endian ELF64')
    start = struct.unpack_from('<Q', elf, 32)[0]
    size, count = struct.unpack_from('<HH', elf, 54)
    if size != 56:
        raise ValueError('unexpected ELF program header')
    for index in range(count):
        kind, _, offset, virtual, _, length, _, _ = struct.unpack_from('<IIQQQQQQ', elf, start + size * index)
        if kind == 1 and virtual <= address < virtual + length:
            return offset + address - virtual
    raise ValueError('probe address outside file-backed load segments')


def sample(record, compositor_pid):
    if record['kind'] != 9:
        return None
    payload = bytes.fromhex(record['payload_hex'])
    if len(payload) != 32:
        raise ValueError('unexpected uprobe sample length')
    pid, tid, timestamp, abi, surface = struct.unpack('<IIQQQ', payload)
    if pid != compositor_pid or tid != compositor_pid or abi != 2:
        raise ValueError('unexpected uprobe identity/register ABI')
    return dict(time_ns=timestamp, surface_pointer=surface)


class IOV(ctypes.Structure):
    _fields_ = [('base', ctypes.c_void_p), ('length', ctypes.c_size_t)]


def memory_reader(libc, pid):
    libc.process_vm_readv.restype = ctypes.c_ssize_t
    def read(address, length):
        buffer = ctypes.create_string_buffer(length)
        local = IOV(ctypes.cast(buffer, ctypes.c_void_p), length)
        remote = IOV(address, length)
        result = libc.process_vm_readv(pid, ctypes.byref(local), 1, ctypes.byref(remote), 1, 0)
        if result != length:
            raise OSError(ctypes.get_errno(), 'compositor identity read failed')
        return buffer.raw
    return read


def identify(read, surface):
    _, resource, client = struct.unpack('<QQQ', read(surface + 0x648, 24))
    pid = struct.unpack('<I', read(client + 0xa8, 4))[0]
    wl_resource = struct.unpack('<Q', read(resource + 0x1a0, 8))[0]
    wire_id = struct.unpack('<I', read(wl_resource + 16, 4))[0]
    return pid, wire_id


@contextmanager
def probe(libc, pid, elf, address):
    path = ctypes.create_string_buffer(os.fsencode(HYPRLAND))
    attr = (ctypes.c_uint64 * 16)()
    attr[0] = (128 << 32) | int(Path('/sys/bus/event_source/devices/uprobe/type').read_text())
    attr[2] = 1
    attr[3] = (1 << 1) | (1 << 2) | (1 << 12)  # TID, TIME, REGS_USER
    attr[5] = (1 << 5) | (1 << 6) | (1 << 25)
    attr[7] = ctypes.addressof(path)
    attr[8] = file_offset(elf, address)
    attr[10] = 1 << 5  # PERF_REG_X86_DI
    ctypes.cast(ctypes.byref(attr, 92), ctypes.POINTER(ctypes.c_int32))[0] = 1
    fd = libc.syscall(298, ctypes.byref(attr), pid, -1, -1, 8)
    if fd < 0:
        raise OSError(ctypes.get_errno(), 'open compositor uprobe')
    try:
        with mmap.mmap(fd, mmap.PAGESIZE * 257, flags=mmap.MAP_SHARED,
                       prot=mmap.PROT_READ | mmap.PROT_WRITE) as ring:
            try:
                yield fd, ring
            finally:
                libc.ioctl(fd, 0x2401, 0)
    finally:
        os.close(fd)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--pid', type=int, required=True)
    parser.add_argument('--client-pid-file', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    if os.geteuid() != 0:
        raise PermissionError('run this bounded observer with sudo; never provide passwords to the agent')
    if Path(f'/proc/{args.pid}/comm').read_text().strip() != 'Hyprland':
        raise ValueError('expected compositor main thread')
    for path, expected in HASHES.items():
        if hashlib.sha256(path.read_bytes()).hexdigest() != expected:
            raise ValueError(f'unsupported binary version: {path}')
    elf = HYPRLAND.read_bytes()
    args.output.mkdir(parents=True, exist_ok=True)
    if any((args.output / name).exists() for name in ['ready.json', 'events.json']):
        raise ValueError('fresh output files required')
    libc = ctypes.CDLL(None, use_errno=True)
    read = memory_reader(libc, args.pid)
    events = []
    complete = False
    with ExitStack() as resources:
        rings = {name: resources.enter_context(probe(libc, args.pid, elf, address)) for name, address in SYMBOLS.items()}
        metadata = dict(diagnostic_only=True, pid=args.pid, clock_id=1, seconds=60,
                        client_pid_file=str(args.client_pid_file.resolve()),
                        started_ns=time.monotonic_ns(), script_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                        hashes={str(k):v for k,v in HASHES.items()})
        with (args.output / 'ready.json').open('x') as file:
            json.dump(metadata, file, indent=2)
        print('READY — collecting for 60 seconds; no compositor pause or setting change', flush=True)
        deadline = time.monotonic() + 60
        observed_pids = set()
        def collect():
            wanted = None
            try:
                value = args.client_pid_file.read_text().strip()
            except FileNotFoundError:
                value = ''
            if value.isdigit():
                wanted = int(value)
                observed_pids.add(wanted)
            for name, (_, ring) in rings.items():
                for raw in drain_ring(ring):
                    event = dict(probe=name, raw=raw)
                    parsed = sample(raw, args.pid)
                    if parsed is not None:
                        event.update(parsed)
                        if wanted is not None:
                            try:
                                owner, wire_id = identify(read, parsed['surface_pointer'])
                                event.update(owner_pid=owner, wire_id=wire_id, wanted_pid=wanted)
                            except OSError as error:
                                event['identity_error'] = str(error)
                    events.append(event)
                    if len(events) >= 200000:
                        raise ValueError('trace overflow')
        try:
            while time.monotonic() < deadline:
                collect()
                time.sleep(.02)
            for fd, _ in rings.values():
                if libc.ioctl(fd, 0x2401, 0) != 0:
                    raise OSError(ctypes.get_errno(), 'disable compositor probe')
            collect()
            complete = True
        finally:
            with (args.output / 'events.json').open('x') as file:
                json.dump(dict(metadata=metadata, complete=complete,
                               observed_client_pids=sorted(observed_pids),
                               chart_identity_observed=any(e.get('owner_pid') in observed_pids for e in events),
                               events=events), file)
    print('Trace closed; all temporary probes removed', flush=True)


if __name__ == '__main__':
    main()
