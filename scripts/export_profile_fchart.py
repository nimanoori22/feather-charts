#!/usr/bin/env python3
"""Convert retained profiler frames.csv after measurement. Standard library only."""
import argparse
import csv
import json
import math
from pathlib import Path

COLUMNS = ("update_ns", "prepare_ns", "geometry_ns", "present_call_ns", "present_cpu_ns", "work_cpu_ns", "work_ns")

def convert(source, columns=("prepare_ns", "work_ns"), include_warmups=False):
    source = Path(source)
    if not columns or len(set(columns)) != len(columns) or any(c not in COLUMNS for c in columns):
        raise ValueError("select distinct measured duration columns: " + ", ".join(COLUMNS))
    groups = {}
    previous = None
    with source.open(newline="", encoding="utf-8") as stream:
        reader = csv.DictReader(stream)
        required = {"frame", "phase", "warmup", *columns}
        if not required.issubset(reader.fieldnames or []):
            raise ValueError("missing CSV fields: " + ", ".join(sorted(required - set(reader.fieldnames or []))))
        for number, row in enumerate(reader, 2):
            try:
                frame = int(row["frame"])
                if frame < 0 or frame > 2**53 or (previous is not None and frame <= previous):
                    raise ValueError("frame keys must increase exactly as f64")
                previous = frame
                phase = row["phase"]
                if not phase or row["warmup"] not in ("true", "false"):
                    raise ValueError("require phase and boolean warmup")
                values = [float(row[c]) for c in columns]
                if any(not math.isfinite(v) or v < 0 for v in values):
                    raise ValueError("durations must be finite nonnegative ns")
                if row["warmup"] == "true" and not include_warmups:
                    continue
                for column, value in zip(columns, values):
                    groups.setdefault((phase, column), []).append([frame, value / 1_000_000])
            except (ValueError, TypeError, KeyError) as error:
                raise ValueError(f"{source}:{number}: {error}") from error
    if not groups:
        raise ValueError("no measured rows after warmup policy")
    if len(groups) > 32:
        raise ValueError("phase/column selection exceeds 32 series")
    metadata = {"source": str(source), "source_columns": ",".join(columns),
                "warmup_policy": "included" if include_warmups else "excluded",
                "measurement": "CPU/work durations; not GPU or presentation latency",
                "series_identity": "phase / original source column"}
    context = source.with_name("metadata.txt")
    if context.exists():
        raw = context.read_text(encoding="utf-8")
        if len(raw.encode()) > 4096:
            raise ValueError("metadata.txt exceeds metadata value budget")
        metadata["run_context"] = raw
    return {"format": "feather-chart", "version": 1, "title": "Frame profiler durations",
            "x_axis": {"label": "Original frame", "spacing": "linear", "precision": 0},
            "y_axis": {"label": "Duration", "unit": "ms", "precision": 3},
            "series": [{"name": f"{phase} / {column}", "kind": "line", "points": points}
                       for (phase, column), points in groups.items()], "metadata": metadata}

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--columns", nargs="+", default=["prepare_ns", "work_ns"])
    parser.add_argument("--include-warmups", action="store_true")
    args = parser.parse_args()
    try:
        document = convert(args.source, args.columns, args.include_warmups)
        encoded = json.dumps(document, indent=2, allow_nan=False) + "\n"
        if len(encoded.encode()) > 32 * 1024 * 1024 or sum(len(s["points"]) for s in document["series"]) > 250000:
            raise ValueError("document exceeds v1 budgets")
        with args.output.open("x", encoding="utf-8") as stream:
            try:
                stream.write(encoded)
                stream.flush()
            except BaseException:
                stream.close()
                args.output.unlink()
                raise
    except (OSError, ValueError) as error:
        parser.exit(1, f"{error}\n")

if __name__ == "__main__":
    main()
