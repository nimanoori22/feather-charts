import csv
from pathlib import Path
import tempfile
import unittest

from histogram_work_analysis import read_counts, compare_work, split_snapshot, read_histogram_times


def frame(source1=(1, 1, 83), source2=(0, 0, 0)):
    return [source1, source2, *[(0, 0, 0)] * 14]


class HistogramWorkTests(unittest.TestCase):
    def test_timer_rows_are_complete_positive_and_exclude_warmups(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / 'times.csv'
            rows = [[sample, source, 1, 1, 83, (sample + 1) * 100]
                    for sample in range(2) for source in range(1, 17)]
            def write(data):
                with path.open('w') as f:
                    w = csv.writer(f)
                    w.writerow(['sample', 'series', 'refreshes', 'preparations', 'points', 'histogram_ns'])
                    w.writerows(data)
            write(rows)
            self.assertEqual(read_histogram_times(path, 1, warmups=1), [3200])
            for invalid in [rows[:-1], [[0, 1, 1, 1, 83, 0], *rows[1:]],
                            [[0, 1, 1, 1, 83, -1], *rows[1:]]]:
                write(invalid)
                with self.assertRaises(ValueError):
                    read_histogram_times(path, 1, warmups=1)

    def test_snapshot_split_uses_the_same_total_tail_and_rejects_bad_timings(self):
        values = dict(update=[0.] * 4, snapshot=[1., 8., 2., 3.], glyph=[0.] * 4)
        result = split_snapshot(values, [100_000, 200_000, 100_000, 100_000])
        self.assertAlmostEqual(result['tail']['histogram'], .2)
        self.assertAlmostEqual(result['tail']['other_snapshot'], 7.8)
        self.assertAlmostEqual(result['tail_minus_normal']['histogram'], .1)
        for invalid in [[100_000], [-1] * 4, [9_000_000] * 4]:
            with self.assertRaises(ValueError):
                split_snapshot(values, invalid)

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
