import struct
import unittest

from trace_hyprland_surface import file_offset, sample, identify


class CompositorProbeTests(unittest.TestCase):
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
        self.assertEqual(sample(record, 42), dict(time_ns=1234, surface_pointer=0x1000))
        self.assertIsNone(sample(dict(kind=2), 42))
        with self.assertRaises(ValueError):
            sample(record, 43)
        with self.assertRaises(ValueError):
            sample(dict(kind=9,payload_hex=''), 42)


if __name__ == '__main__':
    unittest.main()
