import struct
import unittest

from profile_update_faults import decode_fault


class FaultTraceTests(unittest.TestCase):
    def test_fault_timestamp_and_identity_and_loss_validation(self):
        record = dict(kind=9, misc=2, payload_hex=struct.pack('QIIQ', 0x1234, 42, 42, 1000).hex())
        self.assertEqual(decode_fault(record, 42), dict(ip=0x1234, time_ns=1000))
        for bad in [dict(record, kind=2), dict(record, misc=1), dict(record, payload_hex='')]:
            with self.assertRaises(ValueError):
                decode_fault(bad, 42)
        with self.assertRaises(ValueError):
            decode_fault(record, 43)


if __name__ == '__main__':
    unittest.main()
