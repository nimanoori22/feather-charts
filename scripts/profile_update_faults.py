"""Observe the owned update diagnostic's user page faults and scheduling, not acceptance."""
import argparse
from contextlib import ExitStack
import hashlib
import json
import os
from pathlib import Path
import platform
import signal
import struct
import subprocess
import time

from profile_frame_schedule import fault_events, switch_events, drain_ring, off_cpu


def decode_fault(record, pid):
    if record['kind'] != 9:
        raise ValueError('lost or unknown fault record; incomplete evidence')
    payload = bytes.fromhex(record['payload_hex'])
    if len(payload) != 24:
        raise ValueError('invalid fault sample ABI')
    ip, owner, tid, timestamp = struct.unpack('QIIQ', payload)
    if owner != pid or tid != pid or record['misc'] & 7 != 2:
        raise ValueError('fault must identify the owned user thread')
    return dict(ip=ip, time_ns=timestamp)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--rows', type=int, choices=[1200, 100000], required=True)
    args = parser.parse_args()
    if (platform.system(), platform.machine()) != ('Linux', 'x86_64'):
        raise ValueError('requires Linux x86_64 perf ABI')
    args.output.mkdir(parents=True, exist_ok=False)
    command = ['taskset', '-c', '5', str(args.binary.resolve()), '--rows', str(args.rows),
               '--series', '16', '--kind', 'histogram', '--phase', 'append',
               '--fixed-last-price', '--samples', '2000']
    manifest = dict(diagnostic_only=True, clock_id=1, command=command,
                    binary_sha256=hashlib.sha256(args.binary.read_bytes()).hexdigest(),
                    script_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest())
    records = {name: [] for name in ['minor', 'major', 'switch']}
    with (args.output / 'process.log').open('x') as log, ExitStack() as resources:
        process = subprocess.Popen(command, stdout=log, stderr=subprocess.STDOUT)
        try:
            os.kill(process.pid, signal.SIGSTOP)
            stopped, status = os.waitpid(process.pid, os.WUNTRACED)
            if stopped != process.pid or not os.WIFSTOPPED(status):
                raise ValueError('could not stop owned child')
            rings = dict(minor=resources.enter_context(fault_events(process.pid)),
                         major=resources.enter_context(fault_events(process.pid, major=True)),
                         switch=resources.enter_context(switch_events(process.pid)))
            manifest['pid'] = process.pid
            os.kill(process.pid, signal.SIGCONT)
            deadline = time.monotonic() + 75
            while True:
                for name, ring in rings.items():
                    records[name].extend(drain_ring(ring))
                    if len(records[name]) > 1_000_000:
                        raise ValueError('fault trace overflow')
                if process.poll() is not None:
                    if all(struct.unpack_from('QQ', ring, 1024)[0] == struct.unpack_from('QQ', ring, 1024)[1] for ring in rings.values()):
                        break
                else:
                    try:
                        maps = Path(f'/proc/{process.pid}/maps').read_text()
                        (args.output / 'maps.txt').write_text(maps)
                    except FileNotFoundError:
                        pass
                if time.monotonic() >= deadline:
                    raise TimeoutError('update diagnostic exceeded 75 seconds')
                time.sleep(.01)
            manifest['exit_code'] = process.returncode
            for name in ['minor', 'major']:
                manifest[name] = [decode_fault(r, process.pid) for r in records[name]]
            manifest['intervals'], manifest['unmatched_final_out'] = off_cpu(records['switch'], process.pid)
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()
            (args.output / 'records.json').write_text(json.dumps(records))
            (args.output / 'manifest.json').write_text(json.dumps(manifest, indent=2))
    if process.returncode != 0:
        raise ValueError('update diagnostic failed')
    print(args.output / 'manifest.json')


if __name__ == '__main__':
    main()
