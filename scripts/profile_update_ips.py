"""Build a diagnostic-only copy of the actual update fixture with cache-miss IP sampling."""
import argparse
import hashlib
import json
from pathlib import Path
import platform
import shutil
import subprocess

from profile_public_chart import ROOT, replace_once, isolate_package


def instrument(source):
    source, boundary, remaining = source.partition('\n#[derive(Debug)]\nstruct ProbeBox;')
    if not boundary:
        raise ValueError('missing fixture boundary')
    source = replace_once(source, 'use feather_charts::{',
                          '#[path = "support/update_ip_samples.rs"]\nmod update_ip_samples;\nuse feather_charts::{')
    source = replace_once(source, 'fn main() {', 'fn main() {\n    println!("diagnostic_only=true");\n    println!("diagnostic_build={}", env!("CARGO_PKG_VERSION"));')
    source = replace_once(source, '    assert!(samples > 0);', '''    assert!(samples > 0);
    assert!(histories.len() == 1 && counts == [16] && kinds == [SeriesType::Histogram]
        && selected_phase == Some("append") && burst == 1 && !diagnostic && !auxiliary,
        "IP diagnostic requires one 16-source Histogram append fixture");''')
    source = replace_once(source, '\n                    for sample in 0..samples + 20 {',
                          '\n                    let mut ip_samples = update_ip_samples::Sampler::new(samples).expect("cache-miss sampling unavailable");\n'
                          '\n                    for sample in 0..samples + 20 {')
    source = replace_once(source, '\n                        let start = Instant::now();',
                          '\n                        ip_samples.before();\n                        let start = Instant::now();')
    source = replace_once(source, '\n                        let updated = Instant::now();',
                          '\n                        let updated = Instant::now();\n                        ip_samples.after(sample, updated - start);')
    return replace_once(source, '                    diagnostics.report();',
                         '                    diagnostics.report();\n                    ip_samples.report();') + boundary + remaining


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
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
    example.write_text(instrument(example.read_text()))
    command = ['cargo', 'build', '--offline', '--release', '--example', 'preparation_profile',
               '--manifest-path', str(project / 'Cargo.toml'), '--target-dir', str(ROOT / 'target'),
               '--config', 'profile.release.package.feather-charts.debug=1']
    snapshot = dict(diagnostic_only=True, package_version=version, command=command,
                    sources={str(p.relative_to(project)): hashlib.sha256(p.read_bytes()).hexdigest()
                             for p in project.rglob('*.rs')})
    (output / 'manifest.json').write_text(json.dumps(snapshot, indent=2))
    with (output / 'build.log').open('x') as log:
        subprocess.run(command, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, check=True)
    binary = output / 'update-ip-diagnostic'
    shutil.copy2(ROOT / 'target/release/examples/preparation_profile', binary)
    snapshot['binary_sha256'] = hashlib.sha256(binary.read_bytes()).hexdigest()
    (output / 'manifest.json').write_text(json.dumps(snapshot, indent=2))
    print(binary)


if __name__ == '__main__':
    main()
