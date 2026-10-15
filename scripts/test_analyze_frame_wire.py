import unittest

from analyze_frame_wire import parse_wire, correlate


class WireJoinTests(unittest.TestCase):
    def test_independent_native_and_mesa_requests_share_one_commit(self):
        lines, frame, request = self.fixture()
        lines.insert(1, '[00:00:01.000100] -> wp_presentation#9.feedback(wl_surface#2, new id wp_presentation_feedback#8)')
        lines.append('[00:00:01.040001] wp_presentation_feedback#8.presented(0, 1, 40000000, 16666666, 0, 100, 7)')
        wire = parse_wire(lines)
        self.assertEqual([r['shared_commit_requests'] for r in wire], [1, 1])
        native = [r for r in wire if r['presentation'] == 1]
        correlate(native, [frame], [request], dict(intervals=[]))
        with self.assertRaises(ValueError):
            correlate(native, [frame], [dict(request, presentation=9)], dict(intervals=[]))
        with self.assertRaises(ValueError):
            correlate(wire, [frame], [request], dict(intervals=[]))

    def fixture(self):
        lines = ['[00:00:01.000000] -> wp_presentation#1.feedback(wl_surface#2, new id wp_presentation_feedback#3)',
                 '[00:00:01.000900] -> wl_surface#2.attach(wl_buffer#4, 0, 0)',
                 '[00:00:01.001000] -> wl_surface#2.commit()',
                 '[00:00:01.040000] wp_presentation_feedback#3.presented(0, 1, 40000000, 16666666, 0, 100, 7)']
        frame = dict(frame=0, phase='pan', warmup=False, started_ns=999000000, work_ns=3000000,
                     work_cpu_ns=2500000, discarded=False, resolved=True, presented_ns=1040000000)
        request = dict(frame=0, begin_ns=1000000000, end_ns=1000001000)
        return lines, frame, request

    def test_commit_join_reused_ids_and_invalid_clock_or_feedback(self):
        lines, frame, request = self.fixture()
        result = correlate(parse_wire(lines), [frame], [request], dict(intervals=[]))
        self.assertEqual(result['frames'][0]['request_to_commit_ns'], 1000000)
        self.assertEqual(result['frames'][0]['commit_to_presentation_bounds_ns'], [38998000, 39001000])
        self.assertEqual(len(parse_wire(lines + lines)), 2)
        for bad in [lines + [lines[-1]], lines[:1] + lines]:
            with self.assertRaises(ValueError):
                parse_wire(bad)
        for bad_wire in [parse_wire(lines[:-1]), parse_wire(lines[:1] + lines[-1:])]:
            with self.assertRaises(ValueError):
                correlate(bad_wire, [frame], [request], dict(intervals=[]))
        with self.assertRaises(ValueError):
            correlate(parse_wire(lines), [dict(frame, presented_ns=2)], [request], dict(intervals=[]))
        with self.assertRaises(ValueError):
            correlate(parse_wire(lines), [frame], [dict(request, end_ns=1)], dict(intervals=[]))


if __name__ == '__main__':
    unittest.main()
