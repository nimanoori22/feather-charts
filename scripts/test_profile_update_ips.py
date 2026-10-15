import unittest

from profile_update_ips import instrument, instrument_update
from profile_public_chart import ROOT


class UpdateIpPatchTests(unittest.TestCase):
    def test_phase_route_is_derived_from_and_preserves_the_original_method(self):
        original = (ROOT / 'src/api/chart.rs').read_text()
        source = instrument_update(original)
        self.assertEqual(source.count('pub fn update_data('), 1)
        self.assertEqual(source.count('pub fn update_data_profiled('), 1)
        self.assertIn('mark("begin");', source)
        self.assertIn('mark("validate");', source)
        self.assertIn('mark("data-layer");', source)
        self.assertIn('mark("model");', source)
        instrumented = instrument((ROOT / 'examples/preparation_profile.rs').read_text(), fast=True, phases=True)
        self.assertIn('update_data_profiled(ids[0], item, false, |stage| ip_samples.mark(stage))', instrumented)
        with self.assertRaises(ValueError):
            instrument_update('unknown engine version')

    def test_rdpmc_uses_the_same_brackets_without_sampling_interrupts(self):
        source = instrument((ROOT / 'examples/preparation_profile.rs').read_text(), fast=True)
        self.assertIn('update_counters::FastDiagnostics::new(samples)', source)
        self.assertNotIn('mod update_ip_samples', source)
        self.assertEqual(source.count('ip_samples.before();'), 1)
        self.assertEqual(source.count('ip_samples.after(sample, updated - start);'), 1)
        self.assertIn('diagnostic_only=true', source)

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
