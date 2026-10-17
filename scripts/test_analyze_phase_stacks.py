import unittest

from analyze_phase_stacks import parse_samples, matching_phase


class PhaseStackTests(unittest.TestCase):
    def test_nanosecond_timestamps_and_inline_frames_are_preserved(self):
        text = 'chart 12/13 42.000000123: cycles:u: \n\t abc inner (inlined)\n\t def outer (/bin/chart)\n'
        result = parse_samples(text)
        self.assertEqual(result[0]['time_ns'], 42000000123)
        self.assertEqual(result[0]['symbols'], ['inner (inlined)', 'outer (/bin/chart)'])
        self.assertEqual(result[0]['tid'], 13)

    def test_lost_or_unrecognized_events_cannot_silently_disappear(self):
        for text in ['PERF_RECORD_LOST 5 events', 'unknown format']:
            with self.assertRaises(ValueError):
                parse_samples(text)

    def test_instruction_pointer_survives_when_callchain_unwinding_is_missing(self):
        line = 'chart 12/13 42.000000123: cycles:u: abc font::shape (/bin/chart)\n'
        samples = parse_samples(line + line.replace('123:', '124:'))
        self.assertEqual(samples[0]['symbols'], ['font::shape (/bin/chart)'])
        self.assertEqual(len(samples), 2)

    def test_assignment_uses_half_open_windows_and_checks_thread_identity(self):
        rows = [dict(start_ns=100, end_ns=200, pid=1, tid=2),
                dict(start_ns=200, end_ns=300, pid=1, tid=2)]
        self.assertEqual(matching_phase(rows, [100, 200], dict(time_ns=200, pid=1, tid=2)), rows[1])
        self.assertIsNone(matching_phase(rows, [100, 200], dict(time_ns=300, pid=1, tid=2)))
        self.assertIsNone(matching_phase(rows, [100, 200], dict(time_ns=200, pid=1, tid=3)))


if __name__ == '__main__':
    unittest.main()
