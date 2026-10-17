"""Add opt-in per-series Histogram counters to a verified isolated phase fixture."""
from pathlib import Path
import argparse
import hashlib
import json
import os
import shutil
import subprocess

from build_phase_profile import replace_once


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--timing', action='store_true', help='also time Histogram preparation in an isolated build')
    args = parser.parse_args()
    root = Path.cwd()
    reference = root / 'target/frame-phase-investigation/build-v3/candidate'
    manifest = json.loads((reference / 'manifest.json').read_text())
    for name, expected in manifest['sources'].items():
        assert hashlib.sha256((reference / 'project' / name).read_bytes()).hexdigest() == expected, name
    assert hashlib.sha256((reference / 'preparation_profile').read_bytes()).hexdigest() == manifest['binary_sha256']
    out = root / ('target/histogram-timing-load/build' if args.timing else 'target/histogram-work-study/build')
    out.mkdir(exist_ok=False)
    project = out / 'project'
    shutil.copytree(reference / 'project', project, ignore=shutil.ignore_patterns('target', '.git'))
    helper = (root / 'examples/support/histogram_work_counts.rs').read_text()
    (project / 'src/diagnostic_histogram_counts.rs').write_text(helper)
    with (project / 'src/lib.rs').open('a') as f:
        f.write('\npub mod diagnostic_histogram_counts;\n')
    path = project / 'src/model/series/histogram_pane_view.rs'
    s = path.read_text()
    s = replace_once(s, '        self.clear();',
                     '        crate::diagnostic_histogram_counts::refresh(id.value());\n        self.clear();')
    s = replace_once(s, '    let SeriesOptionsMap::Histogram(options)',
                     '    crate::diagnostic_histogram_counts::preparation(series.id().value());\n    let SeriesOptionsMap::Histogram(options)')
    if args.timing:
        s = replace_once(s, '    let SeriesOptionsMap::Histogram(options)',
                         '    let _timer = crate::diagnostic_histogram_counts::preparation_timer(series.id().value());\n    let SeriesOptionsMap::Histogram(options)')
    s = replace_once(s, '    let visible_range = crate::model::time_data::SeriesItemsIndexesRange {',
                     '    crate::diagnostic_histogram_counts::projected(series.id().value(), items.len());\n    let visible_range = crate::model::time_data::SeriesItemsIndexesRange {')
    path.write_text(s)
    path = project / 'examples/preparation_profile.rs'
    s = path.read_text()
    s = replace_once(s, '                    let mut trace = phase_profile::Recorder::new(samples);',
                     '''                    let mut trace = phase_profile::Recorder::new(samples);
                    let work_path = std::env::var_os("FEATHER_WORK_COUNTS");
                    let mut work = Vec::with_capacity(if work_path.is_some() { samples + 20 } else { 0 });''')
    s = replace_once(s, '                        let snapshot = chart.prepare_snapshot(size, 1.25, updated).unwrap();',
                     '''                        if work_path.is_some() { feather_charts::diagnostic_histogram_counts::begin(); }
                        let snapshot = chart.prepare_snapshot(size, 1.25, updated).unwrap();''')
    s = replace_once(s, '                        let prepared = Instant::now();',
                     '''                        let prepared = Instant::now();
                        let work_counts = work_path.as_ref().map(|_| feather_charts::diagnostic_histogram_counts::end());''')
    s = replace_once(s, '                        let converted = Instant::now();',
                     '''                        if let Some(counts) = work_counts { work.push((sample, counts)); }
                        let converted = Instant::now();''')
    s = replace_once(s, '                    if let Some(r) = trace { r.finish(); }',
                     '''                    if let Some(r) = trace { r.finish(); }
                    if let Some(path) = work_path {
                        use std::io::Write;
                        assert_eq!(work.len(), samples + 20);
                        let mut output = std::io::BufWriter::new(std::fs::File::create_new(path).unwrap());
                        writeln!(output, "sample,series,refreshes,preparations,points").unwrap();
                        for (sample, counts) in work {
                            for (index, c) in counts.into_iter().enumerate() {
                                writeln!(output, "{sample},{},{},{},{}", index + 1, c.refreshes, c.preparations, c.points).unwrap();
                            }
                        }
                    }''')
    if args.timing:
        s = replace_once(s, 'feather_charts::diagnostic_histogram_counts::begin();',
                         'feather_charts::diagnostic_histogram_counts::begin_timed();')
        s = replace_once(s, 'sample,series,refreshes,preparations,points',
                         'sample,series,refreshes,preparations,points,histogram_ns')
        s = replace_once(s, '"{sample},{},{},{},{}", index + 1, c.refreshes, c.preparations, c.points',
                         '"{sample},{},{},{},{},{}", index + 1, c.refreshes, c.preparations, c.points, c.preparation_ns')
    path.write_text(s)
    def exclude_chart(directory, names):
        return [n for n in names if n.startswith(('feather-charts-', 'libfeather_charts', 'feather_charts-', 'preparation_profile')) or n == 'examples']
    target = out / 'target'
    shutil.copytree(reference / 'target/release', target / 'release', copy_function=os.link, ignore=exclude_chart)
    command = ['cargo', 'build', '--offline', '--release', '--example', 'preparation_profile',
               '--manifest-path', str(project / 'Cargo.toml'), '--target-dir', str(target)]
    with (out / 'build.log').open('x') as f:
        subprocess.run(command, stdout=f, stderr=subprocess.STDOUT, check=True)
    binary = out / 'preparation_profile'
    shutil.copy2(target / 'release/examples/preparation_profile', binary)
    (out / 'manifest.json').write_text(json.dumps(dict(source='ae067e2', command=command, timing=args.timing,
        binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
        reference_binary_sha256=manifest['binary_sha256'],
        sources={str(p.relative_to(project)): hashlib.sha256(p.read_bytes()).hexdigest()
                 for folder in ['src', 'examples'] for p in (project / folder).rglob('*.rs')}), indent=2))


if __name__ == '__main__':
    main()
