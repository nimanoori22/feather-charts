import io
import runpy
from unittest import mock
import tempfile
import unittest
from pathlib import Path
from export_profile_fchart import convert

class ExportTests(unittest.TestCase):
    def test_conversion_and_phases(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "frames.csv"
            source.write_text("frame,phase,warmup,prepare_ns,work_ns\n1,idle,true,1000000,2000000\n2,idle,false,1500000,2500000\n100,pan,false,2000000,3000000\n")
            result = convert(source)
            self.assertEqual(result["series"][0]["points"], [[2, 1.5]])
            self.assertEqual(result["series"][2]["name"], "pan / prepare_ns")
            self.assertEqual(result["series"][2]["points"], [[100, 2.0]])
            self.assertEqual(len(convert(source, include_warmups=True)["series"][0]["points"]), 2)
    def test_python_producer_cleans_up_a_failed_flush(self):
        write_new = runpy.run_path(str(Path(__file__).resolve().parents[1] / "examples/fchart/write_benchmark.py"))["write_new"]
        class BrokenFlush(io.StringIO):
            def flush(self):
                raise OSError("flush failed")
        with mock.patch.object(Path, "open", return_value=BrokenFlush()) as opened, mock.patch.object(Path, "unlink") as removed:
            with self.assertRaisesRegex(OSError, "flush failed"):
                write_new("new-result.fchart", {"example": 1})
            opened.assert_called_once_with("x", encoding="utf-8")
            removed.assert_called_once_with()

    def test_invalid_rows(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "frames.csv"
            for data in ["frame\n1\n", "frame,phase,warmup,prepare_ns,work_ns\n1,p,false,nan,1\n", "frame,phase,warmup,prepare_ns,work_ns\n2,p,false,1,1\n1,p,false,1,1\n", "frame,phase,warmup,prepare_ns,work_ns\n1,p,maybe,1,1\n"]:
                source.write_text(data)
                with self.assertRaises(ValueError):
                    convert(source)
if __name__ == "__main__":
    unittest.main()
