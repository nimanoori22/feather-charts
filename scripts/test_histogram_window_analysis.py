"""Descriptive controls retain raw failures without attributing noise to history."""
from copy import deepcopy
import unittest

from histogram_window_analysis import analyze_windows


def run(small=1.0, large=1.0, count=200):
    return {1200: [small] * count, 100000: [large] * count}


class HistogramWindowAnalysisTests(unittest.TestCase):
    def test_same_history_variation_flags_control_failure_and_retains_raw_failure(self):
        result = analyze_windows({1: run(), 2: run(large=2.0)})
        self.assertEqual(result['attribution'], 'control_failed')
        self.assertEqual(result['history_exceeded'], 2)
        self.assertEqual(result['control_exceeded'], 2)
        failed = [record for record in result['history_windows'] if record['run'] == 2]
        self.assertEqual(len(failed), 2)
        self.assertTrue(all(record['exceeds_limit'] for record in failed))
        self.assertTrue(all(record['median_growth'] == 1.0 for record in failed))
        self.assertTrue(all(record['p95_growth'] == 1.0 for record in failed))

    def test_history_doubling_with_stable_controls_observes_history_growth(self):
        result = analyze_windows({2: run(large=2.0), 1: run(large=2.0)})
        self.assertEqual(result['attribution'], 'history_growth_observed')
        self.assertEqual(result['history_exceeded'], 4)
        self.assertEqual(result['control_exceeded'], 0)
        self.assertTrue(all(record['exceeds_limit'] for record in result['history_windows']))
        self.assertTrue(all(record['reference_run'] == 1 and record['run'] == 2
                            for record in result['control_windows']))

    def test_common_mode_variation_without_history_growth_is_not_history_growth(self):
        result = analyze_windows({1: run(), 2: run(small=2.0, large=2.0)})
        self.assertEqual(result['attribution'], 'no_growth_observed')
        self.assertEqual(result['history_exceeded'], 0)
        self.assertEqual(result['control_exceeded'], 4)

    def test_percentiles_use_profiler_sorted_floor_indices(self):
        large = [1.0] * 50 + [2.0] * 45 + [3.0] + [4.0] * 4
        data = {1200: [1.0] * 100, 100000: list(reversed(large))}
        result = analyze_windows({1: deepcopy(data), 2: deepcopy(data)})
        for record in result['history_windows']:
            # Sorted positions50 and95, rather than interpolation or averaging.
            self.assertEqual(record['median_growth'], 1.0)
            self.assertEqual(record['p95_growth'], 2.0)
            self.assertEqual(record['count'], 100)

    def test_every_window_and_sorted_nonoverlapping_control_pair_is_retained(self):
        runs = {number: run(small=float(number), large=float(number * 2), count=300)
                for number in [4, 1, 3, 2]}
        original = deepcopy(runs)
        result = analyze_windows(runs)
        self.assertEqual(runs, original, 'analysis must preserve raw samples')
        self.assertEqual(len(result['history_windows']), 12)
        self.assertEqual(len(result['control_windows']), 12)
        self.assertEqual(
            {(record['run'], record['start'], record['count'])
             for record in result['history_windows']},
            {(number, start, 100) for number in range(1, 5) for start in [0, 100, 200]},
        )
        self.assertEqual(
            {(record['rows'], record['reference_run'], record['run'], record['start'], record['count'])
             for record in result['control_windows']},
            {(rows, reference, number, start, 100)
             for rows in [1200, 100000] for reference, number in [(1, 2), (3, 4)]
             for start in [0, 100, 200]},
        )
        for record in result['control_windows']:
            expected = record['run'] / record['reference_run'] - 1.0
            self.assertAlmostEqual(record['median_growth'], expected)
            self.assertAlmostEqual(record['p95_growth'], expected)

    def test_custom_window_size_and_limit_do_not_discard_windows(self):
        result = analyze_windows({1: run(large=2.0, count=40), 2: run(large=2.0, count=40)},
                                 window_size=20, limit=1.01)
        self.assertEqual(len(result['history_windows']), 4)
        self.assertEqual(len(result['control_windows']), 4)
        self.assertEqual(result['history_exceeded'], 0)
        self.assertEqual(result['attribution'], 'no_growth_observed')
        self.assertTrue(all(record['count'] == 20 for record in result['history_windows']))

    def test_invalid_unbalanced_and_incomplete_data_raise_value_error(self):
        invalid = [
            {},
            {1: run()},
            {1: run(), 2: {1200: [1.0] * 200}},
            {1: run(), 2: {1200: [1.0] * 100, 100000: [1.0] * 200}},
            {1: run(count=100), 2: run(count=200)},
            {1: run(count=101), 2: run(count=101)},
            {1: run(count=99), 2: run(count=99)},
            {1: run(count=0), 2: run(count=0)},
        ]
        for value in [float('nan'), float('inf'), float('-inf'), 0.0, -1.0]:
            broken = {1: run(), 2: run()}
            broken[2][100000][100] = value
            invalid.append(broken)
        for data in invalid:
            with self.subTest(data=str(data)[:120]), self.assertRaises(ValueError):
                analyze_windows(data)

    def test_invalid_window_and_limit_raise_value_error(self):
        for window_size in [0, -1, 1.5]:
            with self.subTest(window_size=window_size), self.assertRaises(ValueError):
                analyze_windows({1: run(), 2: run()}, window_size=window_size)
        for limit in [-0.1, float('nan'), float('inf')]:
            with self.subTest(limit=limit), self.assertRaises(ValueError):
                analyze_windows({1: run(), 2: run()}, limit=limit)


if __name__ == '__main__':
    unittest.main()
