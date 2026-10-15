"""CLI regression checks for the controlled Histogram timing follow-up."""
from pathlib import Path
import ast
import json
import os
import subprocess
import tempfile
import unittest

RUNNER = Path(__file__).with_name('profile_incremental_append.py').resolve()
FAKE = '''#!/usr/bin/env python3
import os, sys
args = sys.argv[1:]
def arg(name, default):
    return args[args.index(name) + 1] if name in args else default
rows = int(arg('--rows', '10'))
count = int(arg('--series', '1'))
samples = int(arg('--samples', '1'))
recorded_samples = samples - 1 if os.environ.get('FAKE_SHORT_RAW') else samples
value = 2.0 if os.environ.get('FAKE_SLOW') and rows == 100000 else 1.0
print('engine_sentinel tail_response_rows=1')
for stage in ['update', 'snapshot', 'glyph']:
    print(f'raw Histogram rows={rows} sources={count} phase=append/{stage} ms={[value] * recorded_samples}')
    print(f'Histogram rows={rows} sources={count} phase=append/{stage} median_ms={value:.6f} p95_ms={value:.6f}')
'''


class HistogramTailRunnerTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        artifact = self.root / 'current-profile'
        artifact.write_text(FAKE)
        artifact.chmod(0o755)

    def run_profile(self, slow=False, short_raw=False):
        env = os.environ.copy()
        env.pop('FAKE_SLOW', None)
        env.pop('FAKE_SHORT_RAW', None)
        if slow:
            env['FAKE_SLOW'] = '1'
        if short_raw:
            env['FAKE_SHORT_RAW'] = '1'
        return subprocess.run(
            ['python3', str(RUNNER), '--histogram-tail', '--directory', str(self.root)],
            env=env, text=True, capture_output=True,
        )

    def test_runs_complete_fixed_budget_without_baseline_and_refuses_overwrite(self):
        result = self.run_profile()
        self.assertEqual(result.returncode, 0, result.stderr)
        protocol = json.loads((self.root / 'protocol.json').read_text())
        self.assertEqual((protocol['runs'], protocol['samples'], protocol['warmups']), (6, 2000, 20))
        summaries = json.loads((self.root / 'measurements.json').read_text())
        self.assertEqual(len(summaries), 36)
        self.assertEqual({x['kind'] for x in summaries}, {'histogram'})
        self.assertEqual({x['sources'] for x in summaries}, {16})
        raw = next(line for line in (self.root / 'run-1-current-histogram-1200-16-append.log')
                   .read_text().splitlines() if line.startswith('raw '))
        self.assertEqual(len(ast.literal_eval(raw.split(' ms=', 1)[1])), 2000)
        self.assertIn('6/6', result.stdout)
        analysis = json.loads((self.root / 'window-analysis.json').read_text())
        self.assertEqual(analysis['attribution'], 'no_growth_observed')
        self.assertEqual(len(analysis['history_windows']), 120)
        self.assertEqual(len(analysis['control_windows']), 120)
        self.assertLess(result.stdout.index('run-2-current-histogram-100000-'),
                        result.stdout.index('run-2-current-histogram-1200-'))
        saved = (self.root / 'measurements.json').read_bytes()
        self.assertNotEqual(self.run_profile().returncode, 0)
        self.assertEqual((self.root / 'measurements.json').read_bytes(), saved)

    def test_failure_returns_nonzero_after_retaining_every_pair(self):
        result = self.run_profile(slow=True)
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertIn('0/6', result.stdout)
        self.assertEqual(len(list(self.root.glob('run-*.log'))), 12)
        screens = json.loads((self.root / 'growth-screens.json').read_text())
        self.assertEqual(len(screens), 6)
        self.assertTrue(all(x['p95_growth'] == 1.0 for x in screens))
        analysis = json.loads((self.root / 'window-analysis.json').read_text())
        self.assertEqual(analysis['attribution'], 'history_growth_observed')
        self.assertEqual(analysis['history_exceeded'], 120)

    def test_truncated_raw_samples_cannot_satisfy_the_budget(self):
        result = self.run_profile(short_raw=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(len(list(self.root.glob('run-*.log'))), 1)
        self.assertFalse((self.root / 'growth-screens.json').exists())

    def test_original_full_matrix_keeps_its_budget(self):
        for name in ['baseline-profile', 'baseline-custom-profile', 'current-custom-profile']:
            artifact = self.root / name
            artifact.write_text(FAKE)
            artifact.chmod(0o755)
        env = os.environ.copy()
        env.pop('FAKE_SLOW', None)
        env.pop('FAKE_SHORT_RAW', None)
        result = subprocess.run(
            ['python3', str(RUNNER), '--directory', str(self.root), '--baseline-tail-rows', '1'],
            env=env, text=True, capture_output=True,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        protocol = json.loads((self.root / 'protocol.json').read_text())
        self.assertEqual((protocol['runs'], protocol['samples']), (3, 100))
        self.assertEqual(len(list(self.root.glob('run-*.log'))), 204)
        self.assertEqual(len(json.loads((self.root / 'measurements.json').read_text())), 612)
        self.assertIn('24/24', result.stdout)


if __name__ == '__main__':
    unittest.main()
