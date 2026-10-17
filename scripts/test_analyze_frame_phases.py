"""Sample alignment, rather than independent percentiles, determines attribution."""
import json
import unittest

from analyze_frame_phases import parse_phases, summarize, read_trace
import tempfile
from pathlib import Path


def log(values):
    return '\n'.join(f'raw Histogram rows=1200 sources=16 phase=append/{k} ms={json.dumps(v)}'
                     for k, v in values.items())


class FramePhaseTests(unittest.TestCase):
    def test_tail_uses_same_sample_and_preserves_input_order(self):
        values = dict(update=[9., 0., 0., 0.], snapshot=[0., 8., 0., 0.], glyph=[0., 0., 7., 0.])
        parsed = parse_phases(log(values), 4)
        result = summarize(parsed)
        self.assertEqual(result['p95_sample'], 0)
        self.assertEqual(result['total']['p95'], 9.)
        self.assertEqual(result['tail_mean'], dict(update=9., snapshot=0., glyph=0.))
        self.assertEqual(parsed, values)
        self.assertNotEqual(sum(result[k]['p95'] for k in values), result['total']['p95'])

    def test_parse_rejects_missing_duplicate_nonfinite_and_mismatched_samples(self):
        valid = dict(update=[1., 2.], snapshot=[3., 4.], glyph=[5., 6.])
        for bad in [log({'update': [1., 2.]}), log(valid) + '\n' + log(valid),
                    log({**valid, 'glyph': [float('nan'), 2.]}),
                    log({**valid, 'snapshot': [1.]}),
                    log({**valid, 'update': [-1., 2.]})]:
            with self.subTest(bad=bad), self.assertRaises(ValueError):
                parse_phases(bad, 2)

    def test_tail_is_exactly_highest_five_percent_with_ties_resolved_by_index(self):
        result = summarize(dict(update=[1.] * 100, snapshot=[2.] * 100, glyph=[3.] * 100))
        self.assertEqual(result['tail_samples'], list(range(95, 100)))
        self.assertEqual(result['p95_sample'], 95)
        self.assertEqual(result['tail_share']['snapshot'], 1 / 3)

    def test_trace_requires_ordered_complete_phases_and_one_thread(self):
        header = 'sample,phase,pid,tid,clock_id,warmup,start_ns,end_ns,cpu_ns,minor,major,voluntary,involuntary\n'
        rows = ['0,update,10,10,1,false,100,200,90,0,0,0,0',
                '0,snapshot,10,10,1,false,200,300,90,0,0,0,0',
                '0,glyph,10,10,1,false,300,400,90,0,0,0,0']
        with tempfile.TemporaryDirectory() as tmp:
            p = Path(tmp) / 'trace.csv'
            p.write_text(header + '\n'.join(rows))
            self.assertEqual(len(read_trace(p, 1, warmups=0)), 3)
            p.write_text(header + '\n'.join(r.rsplit(',', 4)[0] + ',,,,' for r in rows))
            self.assertIsNone(read_trace(p, 1, warmups=0)[0]['minor'])
            for invalid in [rows[:-1], rows[::-1], [rows[0], rows[1].replace(',200,300,', ',199,300,'), rows[2]],
                            [rows[0], rows[1].replace(',10,10,', ',11,11,'), rows[2]]]:
                p.write_text(header + '\n'.join(invalid))
                with self.assertRaises(ValueError):
                    read_trace(p, 1, warmups=0)


if __name__ == '__main__':
    unittest.main()
