#!/usr/bin/env python3
"""Measure bounded public surface coverage and recorded source conformance."""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
INVENTORY = 'docs/porting/public-parity-inventory.json'
EVIDENCE = 'docs/porting/runtime-primary-axis-evidence.json'
OUTPUT = 'docs/porting/public-parity-score.json'
COVERED = {'native', 'mapped', 'represented'}
STATUSES = COVERED | {'partial', 'platform', 'gap', 'review'}


def fraction(passed, total):
    return {'numerator': passed, 'denominator': total,
            'percent': round(100 * passed / total, 2) if total else None}


def coverage(rows):
    if len({row['id'] for row in rows}) != len(rows):
        raise ValueError('Duplicate inventory member')
    counts = Counter(row['status'] for row in rows)
    if counts.keys() - STATUSES:
        raise ValueError('Unknown inventory classification')
    covered = sum(counts[status] for status in COVERED)
    return {'total': len(rows), 'classifications': dict(sorted(counts.items())),
            'native_scope': fraction(covered, len(rows) - counts['platform']),
            'including_platform_differences': fraction(covered, len(rows))}


def digest(path):
    return hashlib.sha256((ROOT / path).read_bytes()).hexdigest()


def read(path):
    return json.loads((ROOT / path).read_text())


def measure():
    inventory, evidence = read(INVENTORY), read(EVIDENCE)
    # Verify the recorded inputs before treating their results as measurement data.
    for record in evidence['artifacts'] + evidence['logs']:
        if digest(record['path']) != record['sha256']:
            raise ValueError('Evidence drift: ' + record['path'])
    references = read(evidence['source_comparisons']['results']['path'])
    for row in references:
        if digest(row['script']) != row['sha256']:
            raise ValueError('Source-check script drift: ' + row['script'])
    snapshot = evidence['source_snapshot']
    if digest(snapshot['path']) != snapshot['sha256']:
        raise ValueError('Validated snapshot drift: ' + snapshot['path'])
    source_snapshot = read(snapshot['path'])
    for group in ['sources', 'fixtures']:
        for path, expected in source_snapshot[group].items():
            if digest(path) != expected:
                raise ValueError('Validated source drift: ' + path)
    rows = inventory['rows']
    observations = evidence['original_observations']
    families = evidence['combined_captures']['families']
    total_images = sum(row['images'] for row in families)
    return {
        'schema': 1,
        'source_version': evidence['source_version'],
        'source_head': evidence['source_head'],
        'scope': inventory['scope'],
        'rubric': {
            'unit': 'One declared inventory member; inherited declarations counted once.',
            'covered': sorted(COVERED),
            'partial_credit': 0,
            'native_exclusions': 'platform rows only; listed explicitly below',
            'meaning': 'API/field presence and reviewed Rust adaptation; not per-member behavioral proof.',
            'behavior': 'Recorded source-comparison suite pass rate; selected cases, not exhaustive member coverage.',
            'regression': 'Image comparisons use accepted native captures, not browser/native visual equivalence.',
            'aggregation': 'No weighted overall score; overlapping suites, observations and images stay separate.',
        },
        'surface': coverage(rows),
        'methods': coverage([row for row in rows if row['kind'] == 'method']),
        'properties': coverage([row for row in rows if row['kind'] == 'property']),
        'source_conformance_suites': fraction(sum(row['exit'] == 0 for row in references), len(references)),
        'original_source_observations': fraction(sum(row['equal'] for row in observations), len(observations)),
        'native_image_regressions': fraction(evidence['combined_captures']['accepted_images_unchanged'], total_images),
        'native_tests': evidence['tests'],
        'platform_exclusions': [row for row in rows if row['status'] == 'platform'],
        'partial_members': [row for row in rows if row['status'] == 'partial'],
        'whole_port_behavioral_parity_percent': None,
        'member_behavioral_coverage_percent': None,
        'unmeasured': [
            'No per-member map from all 374 inventory rows to source assertions.',
            'Inventory excludes some exported aliases, plugin contracts and behavior combinations.',
            'Passing native regression images do not measure browser/native pixel parity.',
            'Timing and dedicated device acceptance remain separate open gates.',
        ],
        'independent_exceptions': evidence['independent_exceptions'],
        'inputs': {path: digest(path) for path in [INVENTORY, EVIDENCE, 'scripts/measure_public_parity.py']},
    }


class CoverageTests(unittest.TestCase):
    def test_partial_has_no_credit_and_platform_is_explicitly_excluded(self):
        rows = [{'id': str(i), 'status': status} for i, status in
                enumerate(['native', 'mapped', 'represented', 'partial', 'platform'])]
        result = coverage(rows)
        self.assertEqual(result['native_scope'], {'numerator': 3, 'denominator': 4, 'percent': 75.0})
        self.assertEqual(result['including_platform_differences'], {'numerator': 3, 'denominator': 5, 'percent': 60.0})

    def test_duplicate_and_unknown_members_are_rejected(self):
        with self.assertRaises(ValueError):
            coverage([{'id': 'x', 'status': 'native'}] * 2)
        with self.assertRaises(ValueError):
            coverage([{'id': 'x', 'status': 'invented'}])

    def test_real_inventory_fraction_rounds_to_two_decimal_places(self):
        self.assertEqual(fraction(365, 370),
                         {'numerator': 365, 'denominator': 370, 'percent': 98.65})

    def test_empty_native_scope_is_unmeasured(self):
        self.assertIsNone(coverage([])['native_scope']['percent'])
        self.assertIsNone(coverage([{'id': 'x', 'status': 'platform'}])['native_scope']['percent'])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true', help='Verify the saved scorecard without changing it')
    parser.add_argument('--self-test', action='store_true', help='Test scoring arithmetic and validation')
    args = parser.parse_args()
    if args.self_test:
        result = unittest.TextTestRunner().run(unittest.defaultTestLoader.loadTestsFromTestCase(CoverageTests))
        raise SystemExit(not result.wasSuccessful())
    result = measure()
    if args.check:
        if read(OUTPUT) != result:
            raise ValueError('Scorecard drift; regenerate after reviewing the changed evidence')
    else:
        (ROOT / OUTPUT).write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({key: result[key] for key in ['surface', 'methods', 'properties', 'source_conformance_suites']}, indent=2))


if __name__ == '__main__':
    main()
