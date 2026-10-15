import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


class UpdateEvidenceTests(unittest.TestCase):
    def test_complete_raw_evidence_and_truncation(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            artifact = root / 'current-profile'
            artifact.write_bytes(b'frozen test artifact')
            protocol = dict(runs=6, samples=2000, warmups=20, growth_limit=.1, histogram_tail=True,
                            artifacts=dict(current=hashlib.sha256(artifact.read_bytes()).hexdigest()))
            (root / 'protocol.json').write_text(json.dumps(protocol))
            for run in range(1, 7):
                for rows in [1200, 100000]:
                    (root / f'run-{run}-current-histogram-{rows}-16-append.log').write_text(
                        f'raw Histogram rows={rows} sources=16 phase=append/update ms={([.01] * 2000)!r}\n')
            command = [sys.executable, str(Path(__file__).with_name('frame_acceptance.py')), '--updates', str(root)]
            passing = subprocess.run(command, text=True, capture_output=True)
            self.assertEqual(passing.returncode, 0, passing.stderr)
            self.assertEqual(json.loads(passing.stdout)['status'], 'PASS')
            marked = root / 'run-1-current-histogram-1200-16-append.log'
            original = marked.read_text()
            for marker in ['diagnostic_counters=enabled', 'diagnostic_only=true']:
                marked.write_text(marker + '\n' + original)
                diagnostic = subprocess.run(command, text=True, capture_output=True)
                self.assertEqual(json.loads(diagnostic.stdout)['status'], 'INCONCLUSIVE')
            marked.write_text(original)
            (root / 'run-6-current-histogram-100000-16-append.log').write_text(
                'raw Histogram rows=100000 sources=16 phase=append/update ms=[.01]\n')
            incomplete = subprocess.run(command, text=True, capture_output=True)
            self.assertEqual(incomplete.returncode, 1)
            self.assertEqual(json.loads(incomplete.stdout)['status'], 'INCONCLUSIVE')
            artifact.write_bytes(b'changed artifact')
            mismatch = subprocess.run(command, text=True, capture_output=True)
            self.assertEqual(json.loads(mismatch.stdout)['reason'], 'artifact mismatch')


if __name__ == '__main__':
    unittest.main()
