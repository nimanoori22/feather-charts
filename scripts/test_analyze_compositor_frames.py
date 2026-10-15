import copy
import unittest

from analyze_compositor_frames import correlate


class CompositorJoinTests(unittest.TestCase):
    def fixture(self):
        events = []
        for probe, time, extra in [('schedule', 120, dict(owner_pid=42, wire_id=29)),
                                   ('commit', 125, dict(owner_pid=42, wire_id=29)),
                                   ('render_begin', 200, dict(monitor_id=1)),
                                   ('output_commit', 210, dict(monitor_id=1)),
                                   ('output_done', 220, dict(register_value=0xfeed0001)),
                                   ('render_end', 230, {})]:
            events.append(dict(probe=probe, time_ns=time, raw=dict(kind=9), **extra))
        observer = dict(complete=True, metadata=dict(client_pid=42), events=events)
        frames = [dict(frame=0, surface=29, phase='pan', warmup=False, work_ns=100,
                       presentation_ns=400, presented_ns=400, commit_monotonic_bounds_ns=[100, 110])]
        return observer, frames

    def test_verified_stage_join_and_bool_return(self):
        observer, frames = self.fixture()
        frame = correlate(observer, frames, 42, 1)['frames'][0]
        self.assertEqual(frame['client_to_receipt_bounds_ns'], [10, 20])
        self.assertEqual(frame['apply_to_render_ns'], 75)
        self.assertEqual(frame['render_to_submit_ns'], 10)
        self.assertEqual(frame['submit_call_ns'], 10)
        self.assertEqual(frame['submit_to_present_ns'], 180)
        self.assertTrue(frame['output']['ok'])

    def test_incomplete_identity_loss_and_missing_state_or_return_are_rejected(self):
        observer, frames = self.fixture()
        for bad in [dict(observer, complete=False),
                    dict(observer, metadata=dict(client_pid=43)),
                    dict(observer, events=observer['events'][1:]),
                    dict(observer, events=observer['events'][:-1])]:
            with self.assertRaises(ValueError):
                correlate(bad, frames, 42, 1)
        bad = copy.deepcopy(observer)
        bad['events'][0]['raw']['kind'] = 2
        with self.assertRaises(ValueError):
            correlate(bad, frames, 42, 1)


if __name__ == '__main__':
    unittest.main()
