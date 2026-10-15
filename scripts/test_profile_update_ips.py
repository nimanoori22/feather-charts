import unittest

from profile_update_ips import instrument
from profile_public_chart import ROOT


class UpdateIpPatchTests(unittest.TestCase):
    def test_sampling_brackets_the_existing_timed_update_and_marks_diagnostics(self):
        source = instrument((ROOT / 'examples/preparation_profile.rs').read_text())
        self.assertIn('println!("diagnostic_only=true")', source)
        self.assertIn('env!("CARGO_PKG_VERSION")', source)
        self.assertEqual(source.count('ip_samples.before()'), 1)
        self.assertEqual(source.count('ip_samples.after('), 1)
        self.assertLess(source.index('ip_samples.before()'), source.index('let start = Instant::now();'))
        self.assertIn('let updated = Instant::now();\n                        ip_samples.after(sample, updated - start);', source)
        self.assertIn('ip_samples.report();', source)
        with self.assertRaises(ValueError):
            instrument('unknown fixture')


if __name__ == '__main__':
    unittest.main()
