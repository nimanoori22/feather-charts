#!/usr/bin/env python3
"""Synthetic demonstration results; no GUI dependencies."""
import json
import sys
from pathlib import Path

def write_new(path, document):
    # Only remove a partial file created by this call.
    output = Path(path)
    with output.open("x", encoding="utf-8") as stream:
        try:
            json.dump(document, stream, indent=2, allow_nan=False)
            stream.write("\n")
            stream.flush()
        except BaseException:
            stream.close()
            output.unlink()
            raise

if __name__ == "__main__":
    document = {"format": "feather-chart", "version": 1,
                "title": "Synthetic demonstration benchmark",
                "x_axis": {"label": "Input size", "spacing": "linear", "precision": 0},
                "y_axis": {"label": "Duration", "unit": "ms", "precision": 3},
                "series": [{"name": "Synthetic prepare", "kind": "line",
                            "points": [[1, 0.421], [2, 0.438], [100, 0.409]]}],
                "metadata": {"data": "synthetic demonstration; not measured"}}
    write_new(sys.argv[1], document)
