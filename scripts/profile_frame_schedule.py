"""Trace only the native fixture's main-thread context switches; never establish acceptance."""
import argparse
import ctypes
import hashlib
import json
import mmap
import os
from pathlib import Path
import platform
import signal
import struct
import subprocess
import time


def decode(data):
    records = []
    offset = 0
    while offset < len(data):
        if len(data) - offset < 8:
            raise ValueError('truncated perf header')
        kind, misc, size = struct.unpack_from('IHH', data, offset)
        if size < 8 or size > len(data) - offset:
            raise ValueError('invalid perf record size')
        payload = data[offset + 8:offset + size]
        record = dict(kind=kind, misc=misc, payload_hex=payload.hex())
        if kind == 14:
            if len(payload) != 16:
                raise ValueError('invalid context-switch record')
            pid, tid, timestamp = struct.unpack('IIQ', payload)
            record.update(pid=pid, tid=tid, time_ns=timestamp,
                          out=bool(misc & (1 << 13)), preempt=bool(misc & (1 << 14)))
        records.append(record)
        offset += size
    return records


def off_cpu(records, pid):
    intervals, previous, pending = [], 0, None
    for index, record in enumerate(records):
        if record['kind'] != 14:
            raise ValueError('non-switch perf record retained; incomplete scheduling evidence')
        if record['pid'] != pid or record['tid'] != pid or record['time_ns'] < previous:
            raise ValueError('switch identity or time mismatch')
        previous = record['time_ns']
        if record['out']:
            if pending is not None:
                raise ValueError('missing switch-in')
            pending = record
        elif pending is not None:
            intervals.append(dict(start_ns=pending['time_ns'], end_ns=record['time_ns'],
                                  preempt=pending['preempt']))
            pending = None
        elif index != 0:
            raise ValueError('missing switch-out')
    return intervals, pending


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--backend', choices=['wgpu', 'tiny-skia'], required=True)
    parser.add_argument('--rows', type=int, required=True)
    parser.add_argument('--narrow', action='store_true')
    args = parser.parse_args()
    if (platform.system(), platform.machine()) != ('Linux', 'x86_64'):
        raise ValueError('requires Linux x86_64 perf ABI and memory ordering')
    args.output.mkdir(parents=True, exist_ok=False)
    command = [str(args.binary.resolve()), '--backend', args.backend, '--rows', str(args.rows),
               '--output', str(args.output / 'frames')] + (['--narrow'] if args.narrow else [])
    manifest = dict(diagnostic_only=True, command=command, clock_id=1,
                    script_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                    binary_sha256=hashlib.sha256(args.binary.read_bytes()).hexdigest())
    fd = None
    ring = None
    records = []
    with (args.output / 'process.log').open('x') as log:
        process = subprocess.Popen(command, stdout=log, stderr=subprocess.STDOUT)
        try:
            os.kill(process.pid, signal.SIGSTOP)
            stopped, status = os.waitpid(process.pid, os.WUNTRACED)
            if stopped != process.pid or not os.WIFSTOPPED(status):
                raise ValueError('could not stop owned child before profiling')
            attr = (ctypes.c_uint64 * 16)()
            attr[0] = (128 << 32) | 1  # PERF_TYPE_SOFTWARE
            attr[1] = 9  # PERF_COUNT_SW_DUMMY
            attr[3] = 6  # PERF_SAMPLE_TID | PERF_SAMPLE_TIME
            attr[5] = (1 << 5) | (1 << 6) | (1 << 18) | (1 << 25) | (1 << 26)
            ctypes.cast(ctypes.byref(attr, 92), ctypes.POINTER(ctypes.c_int32))[0] = 1
            libc = ctypes.CDLL(None, use_errno=True)
            opened = libc.syscall(298, ctypes.byref(attr), process.pid, -1, -1, 8)
            if opened < 0:
                raise OSError(ctypes.get_errno(), os.strerror(ctypes.get_errno()))
            fd = opened
            page = mmap.PAGESIZE
            ring = mmap.mmap(fd, page * 257, flags=mmap.MAP_SHARED,
                             prot=mmap.PROT_READ | mmap.PROT_WRITE)
            data_offset, size = struct.unpack_from('QQ', ring, 1040)
            if (data_offset, size) != (page, page * 256):
                raise ValueError('unexpected perf ring ABI')
            manifest.update(pid=process.pid, attached_ns=time.monotonic_ns())
            os.kill(process.pid, signal.SIGCONT)
            deadline = time.monotonic() + 75
            while True:
                head, tail = struct.unpack_from('QQ', ring, 1024)
                if head < tail or head - tail > size:
                    raise ValueError('perf ring overrun')
                count, start = head - tail, tail % size
                first = min(count, size - start)
                data = ring[data_offset + start:data_offset + start + first] + ring[data_offset:data_offset + count - first]
                records.extend(decode(data))
                struct.pack_into('Q', ring, 1032, head)
                if process.poll() is not None:
                    # The producer has exited; drain its final publication once more.
                    if struct.unpack_from('Q', ring, 1024)[0] == head:
                        break
                if time.monotonic() >= deadline:
                    raise TimeoutError('native diagnostic exceeded 75 seconds')
                time.sleep(.05)
            manifest['exit_code'] = process.returncode
            intervals, pending = off_cpu(records, process.pid)
            manifest.update(intervals=intervals, unmatched_final_out=pending)
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()
            (args.output / 'switches.json').write_text(json.dumps(records))
            (args.output / 'manifest.json').write_text(json.dumps(manifest, indent=2))
            metadata = args.output / 'frames/metadata.txt'
            if metadata.exists():
                with metadata.open('a') as file:
                    file.write('diagnostic_draw=schedule\n')
            if ring is not None:
                ring.close()
            if fd is not None:
                os.close(fd)
    if manifest['exit_code'] != 0:
        raise ValueError('native diagnostic did not complete')
    print(args.output / 'manifest.json')


if __name__ == '__main__':
    main()
