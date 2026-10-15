"""Build a diagnostic-only copy of the actual update fixture with cache-miss IP sampling."""
import argparse
import hashlib
import json
from pathlib import Path
import platform
import shutil
import subprocess

from profile_public_chart import ROOT, replace_once, isolate_package, isolate_example


def instrument(source, fast=False, phases=False):
    source, boundary, remaining = source.partition('\n#[derive(Debug)]\nstruct ProbeBox;')
    if not boundary:
        raise ValueError('missing fixture boundary')
    if not fast:
        source = replace_once(source, 'use feather_charts::{',
                              '#[path = "support/update_ip_samples.rs"]\nmod update_ip_samples;\nuse feather_charts::{')
    source = replace_once(source, 'fn main() {', 'fn main() {\n    println!("diagnostic_only=true");\n    println!("diagnostic_build={}", env!("CARGO_PKG_VERSION"));')
    source = replace_once(source, '    assert!(samples > 0);', '''    assert!(samples > 0);
    assert!(histories.len() == 1 && counts == [16] && kinds == [SeriesType::Histogram]
        && selected_phase == Some("append") && burst == 1 && !diagnostic && !auxiliary,
        "IP diagnostic requires one 16-source Histogram append fixture");''')
    probe = 'update_counters::FastDiagnostics' if fast else 'update_ip_samples::Sampler'
    source = replace_once(source, '\n                    for sample in 0..samples + 20 {',
                          f'\n                    let mut ip_samples = {probe}::new(samples).expect("diagnostic counters unavailable");\n'
                          '\n                    for sample in 0..samples + 20 {')
    if phases:
        source = replace_once(source, '\n                    for sample in 0..samples + 20 {',
                              '\n                    ip_samples.enable_phases(samples);\n                    for sample in 0..samples + 20 {')
        source = replace_once(source, 'chart.update_data(ids[0], item, false).unwrap();',
                              'chart.update_data_profiled(ids[0], item, false, |stage| ip_samples.mark(stage)).unwrap();')
    source = replace_once(source, '\n                        let start = Instant::now();',
                          '\n                        ip_samples.before();\n                        let start = Instant::now();')
    source = replace_once(source, '\n                        let updated = Instant::now();',
                          '\n                        let updated = Instant::now();\n                        ip_samples.after(sample, updated - start);')
    return replace_once(source, '                    diagnostics.report();',
                         '                    diagnostics.report();\n                    ip_samples.report();') + boundary + remaining


def instrument_update(source):
    original = '''    pub fn update_data(
        &mut self,
        id: SeriesId,
        item: BuiltInSeriesDataItem<Time>,
        historical: bool,
    ) -> Result<(), ChartError> {
        validate_data(self.kind(id)?, std::slice::from_ref(&item), false)?;
        let update = self.layer.update_series_data(id, item, historical)?;
        self.model.apply_data_update(update)?;
        Ok(())
    }'''
    profiled = original.replace('pub fn update_data(', 'pub fn update_data_profiled(')
    profiled = profiled.replace('        historical: bool,', "        historical: bool,\n        mut mark: impl FnMut(&'static str),")
    profiled = profiled.replace('        validate_data(', '        mark("begin");\n        validate_data(')
    profiled = profiled.replace('        let update =', '        mark("validate");\n        let update =')
    profiled = profiled.replace('        self.model.apply_data_update', '        mark("data-layer");\n        self.model.apply_data_update')
    profiled = profiled.replace('        Ok(())', '        mark("model");\n        Ok(())')
    return replace_once(source, original, original + '\n' + profiled)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--rdpmc', action='store_true')
    parser.add_argument('--phases', action='store_true')
    args = parser.parse_args()
    if args.phases and not args.rdpmc:
        parser.error('--phases requires --rdpmc')
    if (platform.system(), platform.machine()) != ('Linux', 'x86_64'):
        raise ValueError('requires Linux x86_64 perf ABI')
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    abi = output / 'abi.c'
    abi.write_text('''#include <linux/perf_event.h>
#include <stddef.h>
_Static_assert(offsetof(struct perf_event_mmap_page, data_head)==1024, "head");
_Static_assert(offsetof(struct perf_event_mmap_page, data_tail)==1032, "tail");
_Static_assert(offsetof(struct perf_event_mmap_page, data_offset)==1040, "offset");
_Static_assert(offsetof(struct perf_event_mmap_page, data_size)==1048, "size");
_Static_assert(PERF_EVENT_IOC_ENABLE==0x2400 && PERF_EVENT_IOC_DISABLE==0x2401, "ioctl");
_Static_assert(offsetof(struct perf_event_mmap_page, lock)==8, "lock");
_Static_assert(offsetof(struct perf_event_mmap_page, index)==12, "index");
_Static_assert(offsetof(struct perf_event_mmap_page, offset)==16, "offset");
_Static_assert(offsetof(struct perf_event_mmap_page, time_enabled)==24, "enabled");
_Static_assert(offsetof(struct perf_event_mmap_page, time_running)==32, "running");
_Static_assert(offsetof(struct perf_event_mmap_page, capabilities)==40, "capabilities");
_Static_assert(offsetof(struct perf_event_mmap_page, pmc_width)==48, "width");
int main(void) { return 0; }
''')
    subprocess.run(['cc', str(abi), '-o', str(output / 'abi')], check=True)
    subprocess.run([str(output / 'abi')], check=True)
    project = output / 'project'
    project.mkdir()
    for name in ['Cargo.toml', 'Cargo.lock']:
        shutil.copy2(ROOT / name, project / name)
    for name in ['src', 'examples']:
        shutil.copytree(ROOT / name, project / name)
    version = isolate_package(project)
    example = project / 'examples/preparation_profile.rs'
    example.write_text(instrument(example.read_text(), fast=args.rdpmc, phases=args.phases))
    if args.phases:
        chart = project / 'src/api/chart.rs'
        chart.write_text(instrument_update(chart.read_text()))
    if args.rdpmc:
        counters = project / 'examples/support/update_counters.rs'
        source = replace_once(counters.read_text(), 'attr[5] = (1 << 5) | (1 << 6);',
                              'attr[5] = (1 << 2) | (1 << 5) | (1 << 6);')
        source += '\n#[path = ' + json.dumps(str(project / 'examples/support/update_fast_counters.rs')) + ']\nmod fast;\npub use fast::FastDiagnostics;\n'
        counters.write_text(source)
    target = isolate_example(project, 'preparation_profile')
    command = ['cargo', 'build', '--offline', '--release', '--example', target,
               '--manifest-path', str(project / 'Cargo.toml'), '--target-dir', str(ROOT / 'target'),
               '--config', 'profile.release.package.feather-charts.debug=1']
    snapshot = dict(diagnostic_only=True, counter_mode='rdpmc' if args.rdpmc else 'ip', phases=args.phases, package_version=version, command=command,
                    sources={str(p.relative_to(project)): hashlib.sha256(p.read_bytes()).hexdigest()
                             for p in project.rglob('*.rs')})
    (output / 'manifest.json').write_text(json.dumps(snapshot, indent=2))
    with (output / 'build.log').open('x') as log:
        subprocess.run(command, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, check=True)
    binary = output / ('update-rdpmc-diagnostic' if args.rdpmc else 'update-ip-diagnostic')
    shutil.copy2(ROOT / f'target/release/examples/{target}', binary)
    snapshot['binary_sha256'] = hashlib.sha256(binary.read_bytes()).hexdigest()
    (output / 'manifest.json').write_text(json.dumps(snapshot, indent=2))
    print(binary)


if __name__ == '__main__':
    main()
