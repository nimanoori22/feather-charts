#!/usr/bin/env python3
"""One finite 24-process dense preparation study; retain every raw result."""
import hashlib
import json
import os
from pathlib import Path
import platform
import statistics
import subprocess
import sys
import time


def measure(command, prefix):
    with prefix.with_suffix(".stdout").open("w") as stdout, prefix.with_suffix(".stderr").open("w") as stderr:
        child = subprocess.Popen(command, stdout=stdout, stderr=stderr)
        _, status, usage = os.wait4(child.pid, 0)
        child.returncode = os.waitstatus_to_exitcode(status)
    return child.returncode, usage.ru_maxrss


def main():
    output = Path(sys.argv[1] if len(sys.argv) > 1 else "target/conflation/performance")
    output.mkdir(parents=True, exist_ok=True)
    protocol = output / "protocol.json"
    if protocol.exists():
        raise SystemExit("Use a new evidence directory; existing runs must be retained")
    binary = Path("target/release/examples/conflation_profile")
    runs = [(kind, rows, pair, enabled)
            for kind in ("line", "histogram") for rows in (8192, 100000)
            for pair in range(3) for enabled in (("off", "on") if pair % 2 == 0 else ("on", "off"))]
    assert len(runs) == 24
    protocol.write_text(json.dumps({
        "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        "platform": platform.platform(), "affinity": sorted(os.sched_getaffinity(0)),
        "rustc": subprocess.check_output(["rustc", "-Vv"], text=True),
        "head": subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(),
        "tracked_diff_sha256": hashlib.sha256(subprocess.check_output(["git", "diff"])).hexdigest(),
        "fixture_sha256": hashlib.sha256(Path("examples/conflation_profile.rs").read_bytes()).hexdigest(), "python": sys.version, "created_unix": time.time(),
        "warmup": 20, "samples": 100, "size": [1000, 500], "dpr": 1,
        "memory": "Linux wait4 ru_maxrss KiB, whole-process peak",
        "precompute": False, "renderer": "none; CPU preparation only",
        "policy": "24 serial executions, no retries, keep failures; no old timing gates changed",
        "runs": runs,
    }, indent=2) + "\n")
    results = []
    for index, (kind, rows, pair, enabled) in enumerate(runs):
        prefix = output / f"{index:02}-{kind}-{rows}-{pair}-{enabled}"
        command = [str(binary), str(rows), kind, enabled]
        load_before = os.getloadavg()
        status, rss = measure(command, prefix)
        record = {"index": index, "exit": status, "command": command, "load_before": load_before}
        if status == 0:
            record.update(json.loads(prefix.with_suffix(".stdout").read_text()))
            assert len(record["warm_ns"]) == 100
            assert record["rows"] == rows and record["enabled"] == (enabled == "on")
            record["warm_median_ns"] = statistics.median(record["warm_ns"])
            record["max_rss_kib"] = rss
        results.append(record)
        (output / "results.json").write_text(json.dumps(results, indent=2) + "\n")
        print(f"{index + 1}/24 {kind} {rows} {enabled}: exit {status}", flush=True)
    assert all(r["exit"] == 0 for r in results), "Failures retained; inspect results.json"
    for kind in ("line", "histogram"):
        for rows in (8192, 100000):
            cohort = [r for r in results if r["kind"] == kind and r["rows"] == rows]
            off = [r["warm_median_ns"] for r in cohort if not r["enabled"]]
            on = [r["warm_median_ns"] for r in cohort if r["enabled"]]
            print(f"{kind} {rows}: median warm speed ratio {statistics.median(off) / statistics.median(on):.2f}; "
                  f"separated ranges: {max(on) < min(off)}")


if __name__ == "__main__":
    if "--self-test" in sys.argv:
        import tempfile
        with tempfile.TemporaryDirectory() as directory:
            prefix = Path(directory) / "check"
            code, rss = measure([sys.executable, "-c", "print('retained'); raise SystemExit(7)"], prefix)
            assert code == 7 and rss > 0
            assert prefix.with_suffix(".stdout").read_text() == "retained\n"
        print("process status/output/memory self-check passed")
    else:
        main()
