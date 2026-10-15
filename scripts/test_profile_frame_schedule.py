import struct
import unittest

from profile_frame_schedule import decode, off_cpu, drain_ring


class SchedulingTraceTests(unittest.TestCase):
    def test_ring_wrap_bounds_and_tail_publication(self):
        ring = bytearray(4096 + 32)
        record = struct.pack('IHHIIQ', 14, 1 << 13, 24, 42, 42, 1234)
        for i, value in enumerate(record):
            ring[4096 + (20 + i) % 32] = value
        struct.pack_into('QQQQ', ring, 1024, 44, 20, 4096, 32)
        self.assertEqual(drain_ring(ring), decode(record))
        self.assertEqual(struct.unpack_from('Q', ring, 1032)[0], 44)
        self.assertEqual(drain_ring(ring), [])
        struct.pack_into('Q', ring, 1024, 100)
        with self.assertRaises(ValueError):
            drain_ring(ring)
        struct.pack_into('QQQQ', ring, 1024, 0, 0, 4096, 33)
        with self.assertRaises(ValueError):
            drain_ring(ring)

    def test_switches_preserve_preemption_and_reject_lost_or_malformed_data(self):
        def event(misc, timestamp):
            return struct.pack('IHHIIQ', 14, misc, 24, 42, 42, timestamp)
        records = decode(event(0, 10) + event((1 << 13) | (1 << 14), 20) + event(0, 30))
        self.assertEqual(off_cpu(records, 42), ([dict(start_ns=20, end_ns=30, preempt=True)], None))
        for bad in [b'x', event(0, 1)[:-1], bytes(8), struct.pack('IHH', 14, 0, 8)]:
            with self.assertRaises(ValueError):
                decode(bad)
        duplicate_in = decode(event(0, 40))
        for bad in [records + [records[0]], [dict(kind=2)], records[:2] * 2, records + duplicate_in]:
            with self.assertRaises(ValueError):
                off_cpu(bad, 42)
        with self.assertRaises(ValueError):
            off_cpu(records, 43)
        self.assertEqual(off_cpu(records[:2], 42)[1], records[1])


if __name__ == '__main__':
    unittest.main()
