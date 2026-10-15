import struct
import unittest
import os
from pathlib import Path
from tempfile import TemporaryDirectory

from trace_hyprland_surface import file_offset, sample, identify, read_client_pid, chart_surfaces, monitor_identity


class CompositorProbeTests(unittest.TestCase):
    def test_monitor_identity_uses_the_pinned_scheduler_state_and_register_layouts(self):
        values = {0x1000+0x18: struct.pack('<Q', 0x2000), 0x2000+0x68: struct.pack('<Q', 1)}
        def read(address, length):
            self.assertEqual(len(values[address]), length)
            return values[address]
        for name, value in [('scheduler_frame', 0x1000), ('gpu_sync', 0x1000),
                            ('render_begin', 0x2000), ('output_commit', 0x21c8)]:
            self.assertEqual(monitor_identity(read, name, value), 1)
        with self.assertRaises(ValueError):
            monitor_identity(read, 'render_end', 0)
    def test_only_verified_chart_surface_identities_are_accepted(self):
        events = [dict(surface_pointer=10), dict(owner_pid=42, wire_id=29),
                  dict(owner_pid=42, wire_id=29), dict(owner_pid=43, wire_id=30)]
        self.assertEqual(chart_surfaces(events, 42), [29])
        self.assertEqual(chart_surfaces(events, 44), [])

    def test_missing_invalid_or_dead_client_fails_before_recording(self):
        with TemporaryDirectory() as directory:
            path = Path(directory) / 'client.pid'
            with self.assertRaises(FileNotFoundError):
                read_client_pid(path)
            for value in ['', '0', '-1', 'not a pid', '999999999']:
                path.write_text(value)
                with self.assertRaises((ValueError, FileNotFoundError)):
                    read_client_pid(path)
            path.write_text(str(os.getpid()))
            self.assertEqual(read_client_pid(path), os.getpid())

    def test_elf_offsets_and_numeric_surface_identity(self):
        elf = bytearray(120)
        elf[:6] = b'\x7fELF\x02\x01'
        struct.pack_into('<Q', elf, 32, 64)
        struct.pack_into('<HH', elf, 54, 56, 1)
        struct.pack_into('<IIQQQQQQ', elf, 64, 1, 5, 0x1000, 0x2000, 0, 0x500, 0x500, 4096)
        self.assertEqual(file_offset(elf, 0x2123), 0x1123)
        with self.assertRaises(ValueError):
            file_offset(elf, 0x9999)
        values = {0x1000+0x648:struct.pack('<QQQ', 0, 0x2000, 0x3000),
                  0x3000+0xa8:struct.pack('<I', 123), 0x2000+0x1a0:struct.pack('<Q', 0x4000),
                  0x4000+16:struct.pack('<I', 29)}
        def read(address, length):
            result = values[address]
            self.assertEqual(len(result), length)
            return result
        self.assertEqual(identify(read, 0x1000), (123, 29))

    def test_sample_register_abi_and_lost_record_are_explicit(self):
        record = dict(kind=9, payload_hex=struct.pack('<IIQQQ', 42, 42, 1234, 2, 0x1000).hex())
        self.assertEqual(sample(record, 42), dict(time_ns=1234, register_value=0x1000))
        self.assertIsNone(sample(dict(kind=2), 42))
        with self.assertRaises(ValueError):
            sample(record, 43)
        with self.assertRaises(ValueError):
            sample(dict(kind=9,payload_hex=''), 42)


if __name__ == '__main__':
    unittest.main()
