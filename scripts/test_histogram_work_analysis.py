import csv
from pathlib import Path
import tempfile
import unittest

from histogram_work_analysis import read_counts, compare_work


def frame(source1=(1, 1, 83), source2=(0, 0, 0)):
    return [source1, source2, *[(0, 0, 0)] * 14]


class HistogramWorkTests(unittest.TestCase):
    def test_constant_counts_are_joined_to_the_same_slow_sample(self):
        values = dict(update=[0., 0., 0., 0.], snapshot=[1., 8., 2., 3.], glyph=[0.] * 4)
        result = compare_work(values, [frame()] * 4, warmups=0)
        self.assertTrue(result['work_constant'])
        self.assertEqual(result['patterns'][0]['tail_count'], 1)
        self.assertEqual(result['patterns'][0]['normal_count'], 3)
        self.assertEqual(result['p95_sample'], 1)

    def test_unchanged_totals_do_not_hide_different_per_series_work(self):
        values = dict(update=[0.] * 4, snapshot=[1., 8., 2., 3.], glyph=[0.] * 4)
        result = compare_work(values, [frame(), frame((0, 0, 0), (1, 1, 83)), frame(), frame()], warmups=0)
        self.assertFalse(result['work_constant'])
        self.assertEqual(len(result['patterns']), 2)

    def test_incomplete_duplicate_out_of_order_or_negative_counts_are_rejected(self):
        rows = [[0, i, 1, 1, 83] for i in range(1, 17)]
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / 'counts.csv'
            def write(data):
                with path.open('w') as f:
                    w = csv.writer(f); w.writerow(['sample', 'series', 'refreshes', 'preparations', 'points']); w.writerows(data)
            write(rows)
            self.assertEqual(read_counts(path, 1, warmups=0), [[(1, 1, 83)] * 16])
            for invalid in [rows[:-1], [rows[0]] * 16, rows[::-1], [[0, 1, 1, 1, -1], *rows[1:]]]:
                write(invalid)
                with self.assertRaises(ValueError):read_counts(path, 1, warmups=0)


if __name__ == '__main__':
    unittest.main()
