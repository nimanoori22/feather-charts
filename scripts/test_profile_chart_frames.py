"""The native reader must reject incomplete or mismatched evidence."""
import csv
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from profile_chart_frames import PHASES, read_execution


class NativeEvidenceTests(unittest.TestCase):
    def fixture(self, directory):
        (directory / 'metadata.txt').write_text(
            'requested_backend=wgpu\nrows=1200\nsources=16\nframes_per_phase=120\n'
            'warmups_per_phase=20\nsize_changed=false\nscale=1\nphysical_width=917\n'
            'physical_height=871\nmonitor_refresh_millihertz=Some(60000)\n')
        records = [dict(frame=index, phase=PHASES[index // 120],
                        warmup=str(index % 120 < 20).lower(), started_ns=1_000_000_000 + index,
                        work_ns=1_000_000, presented_ns=1_002_000_000 + index,
                        discarded='false', resolved='true') for index in range(480)]
        return records

    def save(self, directory, records):
        with (directory / 'frames.csv').open('w') as file:
            writer = csv.DictWriter(file, fieldnames=records[0])
            writer.writeheader()
            writer.writerows(records)

    def test_complete_fixture_passes_and_budget_failure_stays_failed(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp)
            records = self.fixture(path)
            self.save(path, records)
            self.assertEqual(read_execution(path, 'wgpu', 1200, False)['status'], 'PASS')
            for record in records[20:120]:
                record['work_ns'] = 20_000_000
            self.save(path, records)
            self.assertEqual(read_execution(path, 'wgpu', 1200, False)['status'], 'FAIL')

    def test_truncated_duplicate_and_missing_warmup_feedback_are_inconclusive(self):
        for fault in ['truncated', 'duplicate', 'warmup', 'phase']:
            with self.subTest(fault=fault), tempfile.TemporaryDirectory() as temp:
                path = Path(temp)
                records = self.fixture(path)
                if fault == 'truncated':
                    records.pop()
                elif fault == 'duplicate':
                    records[1]['frame'] = 0
                elif fault == 'warmup':
                    records[0]['resolved'] = 'false'
                else:
                    records[120]['phase'] = 'append'
                self.save(path, records)
                self.assertEqual(read_execution(path, 'wgpu', 1200, False)['status'], 'INCONCLUSIVE')

    def test_wrong_backend_history_and_dimensions_are_inconclusive(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp)
            self.save(path, self.fixture(path))
            for backend, rows, narrow in [('tiny-skia', 1200, False), ('wgpu', 100000, False), ('wgpu', 1200, True)]:
                self.assertEqual(read_execution(path, backend, rows, narrow)['status'], 'INCONCLUSIVE')

    def test_diagnostic_draw_cannot_pass_acceptance(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp)
            self.save(path, self.fixture(path))
            original = (path / 'metadata.txt').read_text()
            for mode in ['no-rectangles', 'schedule']:
                (path / 'metadata.txt').write_text(original + f'diagnostic_draw={mode}\n')
                self.assertEqual(read_execution(path, 'wgpu', 1200, False)['status'], 'INCONCLUSIVE')

    def test_runner_retains_failed_set_stops_and_refuses_overwrite(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            template = root / 'template'
            template.mkdir()
            records = self.fixture(template)
            for record in records:
                record['work_ns'] = 20_000_000
            self.save(template, records)
            binary = root / 'fake-native'
            binary.write_text(f'#!{sys.executable}\n' + '''
from pathlib import Path
import shutil
import sys
args = sys.argv
value = lambda key: args[args.index(key) + 1]
output = Path(value('--output'))
output.mkdir()
template = Path(__file__).parent / 'template'
shutil.copyfile(template / 'frames.csv', output / 'frames.csv')
metadata = (template / 'metadata.txt').read_text()
metadata = metadata.replace('requested_backend=wgpu', 'requested_backend=' + value('--backend'))
metadata = metadata.replace('rows=1200', 'rows=' + value('--rows'))
if '--narrow' in args:
    metadata = metadata.replace('physical_width=917', 'physical_width=420').replace('physical_height=871', 'physical_height=320')
(output / 'metadata.txt').write_text(metadata)
''')
            binary.chmod(0o755)
            output = root / 'results'
            command = [sys.executable, str(Path(__file__).with_name('profile_chart_frames.py')),
                       '--binary', str(binary), '--output', str(output)]
            failed = subprocess.run(command, capture_output=True, text=True)
            self.assertEqual(failed.returncode, 1, failed.stderr)
            manifest = (output / 'manifest.json').read_text()
            result = json.loads(manifest)
            self.assertEqual(result['status'], 'FAIL')
            self.assertEqual(len(result['runs']), 8)
            self.assertTrue(all(run['set'] == 1 for run in result['runs']))
            self.assertEqual(len(list(output.rglob('frames.csv'))), 8)
            repeated = subprocess.run(command, capture_output=True, text=True)
            self.assertNotEqual(repeated.returncode, 0)
            self.assertEqual((output / 'manifest.json').read_text(), manifest)


if __name__ == '__main__':
    unittest.main()
