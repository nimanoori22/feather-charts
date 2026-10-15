import unittest

from profile_frame_wire import instrument_probe
from profile_public_chart import ROOT


class WirePatchTests(unittest.TestCase):
    def test_requests_are_bracketed_without_changing_foreign_window_ownership(self):
        original = (ROOT / 'examples/frame_latency_profile/presentation.rs').read_text()
        source = instrument_probe(original)
        self.assertIn('Vec::with_capacity(480)', source)
        self.assertIn('let begin = self.now_ns();', source)
        self.assertIn('let end = self.now_ns();', source)
        self.assertIn('requests.push((frame, begin, end));', source)
        self.assertIn('frame,begin_ns,end_ns,presentation', source)
        self.assertIn('.id().protocol_id()', source)
        self.assertIn('self.connection.flush().unwrap();', source)
        self.assertIn('_window: Arc<Window>', source)
        self.assertIn('impl Drop for Presentation', source)
        extended = instrument_probe(original, 2880)
        self.assertIn('Vec::with_capacity(2880)', extended)
        self.assertIn('requests.len() < 2880', extended)
        with self.assertRaises(ValueError):
            instrument_probe('unknown source')


if __name__ == '__main__':
    unittest.main()
