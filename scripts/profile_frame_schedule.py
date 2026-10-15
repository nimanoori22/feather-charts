"""Trace only the native fixture's main-thread context switches; never establish acceptance."""
import argparse
from contextlib import contextmanager, ExitStack
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


def switch_events(pid):
    attr = (ctypes.c_uint64 * 16)()
    attr[0] = (128 << 32) | 1
    attr[1] = 9  # PERF_COUNT_SW_DUMMY
    attr[3] = 6  # PERF_SAMPLE_TID | PERF_SAMPLE_TIME
    attr[5] = (1 << 5) | (1 << 6) | (1 << 18) | (1 << 25) | (1 << 26)
    ctypes.cast(ctypes.byref(attr, 92), ctypes.POINTER(ctypes.c_int32))[0] = 1
    return perf_ring(pid, attr)


def fault_events(pid, major=False):
    attr = (ctypes.c_uint64 * 16)()
    attr[0] = (128 << 32) | 1
    attr[1] = 6 if major else 5  # PERF_COUNT_SW_PAGE_FAULTS_MAJ/MIN
    attr[2] = 1
    attr[3] = 7  # IP, TID, TIME from the faulting user context
    attr[5] = (1 << 5) | (1 << 6) | (1 << 25)
    ctypes.cast(ctypes.byref(attr, 92), ctypes.POINTER(ctypes.c_int32))[0] = 1
    return perf_ring(pid, attr)


@contextmanager
def perf_ring(pid, attr):
    libc = ctypes.CDLL(None, use_errno=True)
    fd = libc.syscall(298, ctypes.byref(attr), pid, -1, -1, 8)
    if fd < 0:
        raise OSError(ctypes.get_errno(), os.strerror(ctypes.get_errno()))
    try:
        page = mmap.PAGESIZE
        with mmap.mmap(fd, page * 257, flags=mmap.MAP_SHARED,
                       prot=mmap.PROT_READ | mmap.PROT_WRITE) as ring:
            if struct.unpack_from('QQ', ring, 1040) != (page, page * 256):
                raise ValueError('unexpected perf ring ABI')
            yield ring
    finally:
        os.close(fd)


def drain_ring(ring):
    head, tail, data_offset, size = struct.unpack_from('QQQQ', ring, 1024)
    if size == 0 or data_offset < 1056 or data_offset + size > len(ring):
        raise ValueError('invalid perf ring bounds')
    if head < tail or head - tail > size:
        raise ValueError('perf ring overrun')
    count, start = head - tail, tail % size
    first = min(count, size - start)
    data = ring[data_offset + start:data_offset + start + first] + ring[data_offset:data_offset + count - first]
    records = decode(data)
    struct.pack_into('Q', ring, 1032, head)
    return records


def wait_for_observer(ready, client_pid, compositor_pid, timeout=600):
    deadline = time.monotonic() + timeout
    while True:
        if ready.exists():
            metadata = json.loads(ready.read_text())
            if (metadata.get('client_pid'), metadata.get('pid'), metadata.get('clock_id')) != (client_pid, compositor_pid, 1):
                raise ValueError('observer did not acknowledge the owned client and compositor')
            return metadata
        if time.monotonic() >= deadline:
            raise TimeoutError('observer acknowledgement missing; owned child was not resumed')
        time.sleep(.05)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--backend', choices=['wgpu', 'tiny-skia'], required=True)
    parser.add_argument('--rows', type=int, required=True)
    parser.add_argument('--narrow', action='store_true')
    parser.add_argument('--compositor-pid', type=int)
    parser.add_argument('--client-pid-file', type=Path)
    parser.add_argument('--observer-ready', type=Path)
    args = parser.parse_args()
    if args.observer_ready is not None:
        if args.client_pid_file is None or args.compositor_pid is None:
            parser.error('--observer-ready requires --client-pid-file and --compositor-pid')
        if args.observer_ready.exists():
            raise ValueError('fresh observer acknowledgement required')
    if (platform.system(), platform.machine()) != ('Linux', 'x86_64'):
        raise ValueError('requires Linux x86_64 perf ABI and memory ordering')
    if args.compositor_pid is not None:
        if Path(f'/proc/{args.compositor_pid}/comm').read_text().strip() != 'Hyprland':
            raise ValueError('compositor observer requires the identified Hyprland main thread')
    args.output.mkdir(parents=True, exist_ok=False)
    command = [str(args.binary.resolve()), '--backend', args.backend, '--rows', str(args.rows),
               '--output', str(args.output / 'frames')] + (['--narrow'] if args.narrow else [])
    manifest = dict(diagnostic_only=True, command=command, clock_id=1,
                    script_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                    binary_sha256=hashlib.sha256(args.binary.read_bytes()).hexdigest())
    records = []
    compositor_records = []
    with (args.output / 'process.log').open('x') as log, ExitStack() as resources:
        process = subprocess.Popen(command, stdout=log, stderr=subprocess.STDOUT)
        try:
            os.kill(process.pid, signal.SIGSTOP)
            stopped, status = os.waitpid(process.pid, os.WUNTRACED)
            if stopped != process.pid or not os.WIFSTOPPED(status):
                raise ValueError('could not stop owned child before profiling')
            if args.client_pid_file is not None:
                with args.client_pid_file.open('x') as file:
                    file.write(str(process.pid))
            manifest['pid'] = process.pid
            if args.observer_ready is not None:
                print(f'Waiting for observer acknowledgement for chart PID {process.pid}', flush=True)
                manifest['observer'] = wait_for_observer(args.observer_ready, process.pid, args.compositor_pid)
            ring = resources.enter_context(switch_events(process.pid))
            compositor = (resources.enter_context(switch_events(args.compositor_pid))
                          if args.compositor_pid is not None else None)
            manifest['attached_ns'] = time.monotonic_ns()
            os.kill(process.pid, signal.SIGCONT)
            deadline = time.monotonic() + 75
            while True:
                records.extend(drain_ring(ring))
                if compositor is not None:
                    compositor_records.extend(drain_ring(compositor))
                if process.poll() is not None:
                    # The producer has exited; drain its final publication once more.
                    head, tail = struct.unpack_from('QQ', ring, 1024)
                    if head == tail:
                        break
                if time.monotonic() >= deadline:
                    raise TimeoutError('native diagnostic exceeded 75 seconds')
                time.sleep(.05)
            manifest['exit_code'] = process.returncode
            intervals, pending = off_cpu(records, process.pid)
            manifest.update(intervals=intervals, unmatched_final_out=pending)
            if compositor is not None:
                intervals, pending = off_cpu(compositor_records, args.compositor_pid)
                manifest['compositor'] = dict(pid=args.compositor_pid, intervals=intervals,
                                              unmatched_final_out=pending)
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()
            (args.output / 'switches.json').write_text(json.dumps(records))
            if args.compositor_pid is not None:
                (args.output / 'compositor-switches.json').write_text(json.dumps(compositor_records))
            (args.output / 'manifest.json').write_text(json.dumps(manifest, indent=2))
            metadata = args.output / 'frames/metadata.txt'
            if metadata.exists():
                with metadata.open('a') as file:
                    file.write('diagnostic_draw=schedule\n')
    if manifest['exit_code'] != 0:
        raise ValueError('native diagnostic did not complete')
    print(args.output / 'manifest.json')


if __name__ == '__main__':
    main()
