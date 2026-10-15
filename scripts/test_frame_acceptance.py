"""Acceptance policy checks independent of the renderer and timing runners."""
from copy import deepcopy
import math
import unittest

from frame_acceptance import evaluate_frame_cohort, evaluate_update_growth


WORK_BUDGET = 16_666_667
PRESENTATION_BUDGET = 33_333_334


def screens(growth=0.0):
    return [dict(run=run, median_growth=growth, p95_growth=growth) for run in range(1, 7)]


def controls(growth=0.0):
    return [dict(rows=rows, reference_run=reference, run=run, start=0, count=2000,
                 median_growth=growth, p95_growth=growth)
            for rows in [1200, 100000] for reference, run in [(1, 2), (3, 4), (5, 6)]]


def frames(work=1_000_000, latency=2_000_000):
    return [dict(started_ns=(index + 1) * 1_000_000_000, work_ns=work,
                 presented_ns=(index + 1) * 1_000_000_000 + latency, discarded=False)
            for index in range(100)]


class UpdateAcceptanceTests(unittest.TestCase):
    def test_all_six_pass_at_boundary_with_complete_controls_without_mutating_inputs(self):
        history = screens(0.1)
        same_history = controls(0.8)
        original = deepcopy((history, same_history))
        self.assertEqual(evaluate_update_growth(history, same_history)['status'], 'PASS')
        self.assertEqual((history, same_history), original)

    def test_history_failure_is_never_accepted_by_controls(self):
        history = screens()
        history[4]['p95_growth'] = 0.100001
        original = deepcopy(history)
        self.assertEqual(evaluate_update_growth(history, controls())['status'], 'FAIL')
        noisy = controls()
        noisy[1]['median_growth'] = 0.100001
        self.assertEqual(evaluate_update_growth(history, noisy)['status'], 'INCONCLUSIVE')
        self.assertEqual(history, original, 'raw exceedance must remain recorded')

    def test_missing_duplicate_and_mismatched_history_or_controls_are_inconclusive(self):
        bad_histories = [screens()[:-1], screens() + [screens()[0]], screens()[:-1] + [screens()[0]]]
        for history in bad_histories:
            with self.subTest(history=history):
                self.assertEqual(evaluate_update_growth(history, controls())['status'], 'INCONCLUSIVE')
        bad_controls = [controls()[:-1], controls() + [controls()[0]], controls()[:-1] + [controls()[0]]]
        for field, value in [('rows', 20000), ('reference_run', 2), ('run', 7), ('count', 100), ('start', 100)]:
            broken = controls()
            broken[0][field] = value
            bad_controls.append(broken)
        for same_history in bad_controls:
            with self.subTest(controls=same_history):
                self.assertEqual(evaluate_update_growth(screens(), same_history)['status'], 'INCONCLUSIVE')

    def test_missing_nonfinite_or_impossible_growth_is_inconclusive(self):
        for value in [float('nan'), float('inf'), float('-inf'), -1.000001, None, 'bad']:
            for target in ['history', 'control']:
                history, same_history = screens(), controls()
                records = history if target == 'history' else same_history
                records[0]['p95_growth'] = value
                with self.subTest(value=value, target=target):
                    self.assertEqual(evaluate_update_growth(history, same_history)['status'], 'INCONCLUSIVE')
        for records, other, target in [(screens(), controls(), 'history'), (controls(), screens(), 'control')]:
            del records[0]['median_growth']
            result = evaluate_update_growth(records, other) if target == 'history' else evaluate_update_growth(other, records)
            self.assertEqual(result['status'], 'INCONCLUSIVE')
        self.assertEqual(evaluate_update_growth(screens(-1.0), controls())['status'], 'PASS')


class FrameAcceptanceTests(unittest.TestCase):
    def test_exact_budgets_pass_without_mutating_inputs(self):
        records = frames(work=WORK_BUDGET, latency=PRESENTATION_BUDGET)
        original = deepcopy(records)
        result = evaluate_frame_cohort(records)
        self.assertEqual(result['status'], 'PASS')
        self.assertEqual(result['work_p95_ns'], WORK_BUDGET)
        self.assertEqual(result['presentation_p95_ns'], PRESENTATION_BUDGET)
        self.assertEqual(result['discarded_count'], 0)
        self.assertEqual(records, original)
        self.assertEqual(evaluate_frame_cohort(frames(work=0, latency=0))['status'], 'PASS')

    def test_sorted_floor95_keeps_four_outliers_and_fails_on_five(self):
        for field in ['work', 'presentation']:
            for outliers, expected in [(4, 'PASS'), (5, 'FAIL')]:
                records = frames()
                # Put outliers first to require sorting, not positional indexing.
                for record in records[:outliers]:
                    if field == 'work':
                        record['work_ns'] = WORK_BUDGET + 1
                    else:
                        record['presented_ns'] = record['started_ns'] + PRESENTATION_BUDGET + 1
                with self.subTest(field=field, outliers=outliers):
                    result = evaluate_frame_cohort(records)
                    self.assertEqual(result['status'], expected)
                    key = 'work_p95_ns' if field == 'work' else 'presentation_p95_ns'
                    normal = 1_000_000 if field == 'work' else 2_000_000
                    exceeded = WORK_BUDGET + 1 if field == 'work' else PRESENTATION_BUDGET + 1
                    self.assertEqual(result[key], normal if outliers == 4 else exceeded)

    def test_discarded_frames_count_as_infinite_latency_and_are_not_filtered(self):
        for discarded, expected in [(4, 'PASS'), (5, 'FAIL')]:
            records = frames()
            for record in records[:discarded]:
                record.update(presented_ns=None, discarded=True)
            result = evaluate_frame_cohort(records)
            self.assertEqual(result['status'], expected)
            self.assertEqual(result['discarded_count'], discarded)
            if discarded == 4:
                self.assertEqual(result['presentation_p95_ns'], 2_000_000)
            else:
                self.assertTrue(result['presentation_p95_ns'] is None or
                                math.isinf(result['presentation_p95_ns']))

    def test_missing_feedback_or_incomplete_cohort_is_inconclusive(self):
        for records in [[], frames()[:-1], frames() + [frames()[0]]]:
            self.assertEqual(evaluate_frame_cohort(records)['status'], 'INCONCLUSIVE')
        records = frames()
        records[0].update(presented_ns=None, discarded=False)
        self.assertEqual(evaluate_frame_cohort(records)['status'], 'INCONCLUSIVE')
        del records[0]['presented_ns']
        self.assertEqual(evaluate_frame_cohort(records)['status'], 'INCONCLUSIVE')

    def test_malformed_timestamps_or_work_are_inconclusive(self):
        invalid = [('started_ns', 0), ('started_ns', -1), ('work_ns', -1),
                   ('presented_ns', 1), ('work_ns', None), ('discarded', 'yes')]
        invalid += [(field, value) for field in ['started_ns', 'work_ns', 'presented_ns']
                    for value in [float('nan'), float('inf'), float('-inf'), 'bad']]
        for field, value in invalid:
            records = frames()
            records[0][field] = value
            with self.subTest(field=field, value=value):
                self.assertEqual(evaluate_frame_cohort(records)['status'], 'INCONCLUSIVE')


if __name__ == '__main__':
    unittest.main()
