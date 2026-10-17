"""Freeze symbolized baseline/candidate fixtures with optional phase observations."""
from pathlib import Path
import argparse
import hashlib
import io
import json
import subprocess
import tarfile


def replace_once(text, old, new):
    assert text.count(old) == 1, old
    return text.replace(old, new)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--frame-pointers', action='store_true')
    args = parser.parse_args()
    root = Path.cwd()
    out = root / ('target/frame-phase-investigation/build-fp' if args.frame_pointers
                  else 'target/frame-phase-investigation/build-v3')
    out.mkdir(exist_ok=False)
    for version, revision in [('baseline', 'e148fde'), ('candidate', 'ae067e2')]:
        project = out / version / 'project'
        project.mkdir(parents=True)
        archive = subprocess.check_output(['git', 'archive', revision])
        with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
            tar.extractall(project, filter='data')
        path = project / 'examples/preparation_profile.rs'
        s = path.read_text()
        s, rest = s.split('\nfn auxiliary_probes(', 1)
        s = 'mod phase_profile { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/support/phase_profile_body.rs")); }\n' + s
        # Keep the original inner module documentation first.
        s = s.replace('//! Phase timings for fixed-visible-range preparation and incremental data updates.\n', '')
        s = replace_once(s, '                    for sample in 0..samples + 20 {',
                        '                    let mut trace = phase_profile::Recorder::new(samples);\n                    for sample in 0..samples + 20 {')
        s = replace_once(s, '                        let start = Instant::now();',
                        '                        let t0 = trace.as_ref().map(|r| r.stamp());\n                        let start = Instant::now();')
        s = replace_once(s, '                        let updated = Instant::now();',
                        '                        let updated = Instant::now();\n                        let t1 = trace.as_ref().map(|r| r.stamp());')
        s = replace_once(s, '                        let prepared = Instant::now();',
                        '                        let prepared = Instant::now();\n                        let t2 = trace.as_ref().map(|r| r.stamp());')
        s = replace_once(s, '                        let converted = Instant::now();',
                        '                        let converted = Instant::now();\n                        if let Some(r) = &mut trace {\n                            let t3 = r.stamp();\n                            r.push(sample, [t0.unwrap(), t1.unwrap(), t2.unwrap(), t3]);\n                        }')
        s = replace_once(s, '                    diagnostics.report();',
                        '                    diagnostics.report();\n                    if let Some(r) = trace { r.finish(); }')
        path.write_text(s + '\nfn auxiliary_probes(' + rest)
        helper = (root / 'examples/support/phase_profile.rs').read_text()
        (project / 'examples/support/phase_profile_body.rs').write_text(helper.replace('//!', '//', 1))
        with (project / 'Cargo.toml').open('a') as f:
            f.write('\n[profile.release.package.feather-charts]\ndebug = 1\n')
        import shutil
        import os
        target = out / version / 'target'
        # Reuse dependencies, never another source tree's chart/library/example artifacts.
        def exclude_chart(directory, names):
            return [n for n in names if n.startswith(('feather-charts-', 'libfeather_charts',
                                                      'feather_charts-', 'preparation_profile'))
                    or n == 'examples']
        dependency_cache = (out / 'baseline/target/release' if version == 'candidate'
                            else root / 'target/frame-phase-investigation/fresh-target/release')
        if dependency_cache.exists():
            shutil.copytree(dependency_cache,
                            target / 'release', copy_function=os.link, ignore=exclude_chart)
        command = ['cargo', 'build', '--offline', '--release', '--example', 'preparation_profile',
                   '--manifest-path', str(project / 'Cargo.toml'), '--target-dir', str(target)]
        with (out / version / 'build.log').open('x') as f:
            env = dict(os.environ)
            if args.frame_pointers:
                env['RUSTFLAGS'] = '-C force-frame-pointers=yes'
            subprocess.run(command, env=env, stdout=f, stderr=subprocess.STDOUT, check=True)
        binary = out / version / 'preparation_profile'
        shutil.copy2(target / 'release/examples/preparation_profile', binary)
        (out / version / 'manifest.json').write_text(json.dumps(dict(revision=revision,
            binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(), command=command,
            frame_pointers=args.frame_pointers,
            sources={str(p.relative_to(project)): hashlib.sha256(p.read_bytes()).hexdigest()
                     for folder in ['src', 'examples'] for p in (project / folder).rglob('*.rs')}), indent=2))
        print(version, 'build frozen', flush=True)


if __name__ == '__main__':
    main()
