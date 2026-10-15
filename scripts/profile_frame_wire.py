"""Build an isolated native fixture that brackets Wayland feedback requests."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

from profile_public_chart import ROOT, replace_once, isolate_package, isolate_example


def instrument_probe(source, capacity=480):
    source = replace_once(source, '    connection: Connection,',
        '    connection: Connection,\n    requests: std::cell::RefCell<Vec<(u64, u64, u64)>>,')
    source = replace_once(source, '            connection,',
        '            connection,\n            requests: std::cell::RefCell::new(Vec::with_capacity(480)),')
    source = replace_once(source, '    pub fn request(&self, frame: u64) {',
        '    pub fn request(&self, frame: u64) {\n        let begin = self.now_ns();')
    source = replace_once(source, '        self.connection.flush().unwrap();\n    }\n    pub fn drain',
        '''        self.connection.flush().unwrap();
        let end = self.now_ns();
        let mut requests = self.requests.borrow_mut();
        assert!(requests.len() < 480, "wire trace overflow");
        requests.push((frame, begin, end));
    }
    pub fn save_requests(&self, directory: &std::path::Path) {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new().write(true).create_new(true)
            .open(directory.join("requests.csv")).unwrap();
        let presentation = self.state.presentation.as_ref().unwrap().id().protocol_id();
        writeln!(file, "frame,begin_ns,end_ns,presentation").unwrap();
        for (frame, begin, end) in self.requests.borrow().iter() {
            writeln!(file, "{frame},{begin},{end},{presentation}").unwrap();
        }
    }
    pub fn drain''')
    return source.replace('Vec::with_capacity(480)', f'Vec::with_capacity({capacity})').replace('requests.len() < 480', f'requests.len() < {capacity}')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--long-capture', action='store_true', help='720 frames per phase for a bounded roughly 48-second diagnostic')
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    project = output / 'project'
    project.mkdir()
    for name in ['Cargo.toml', 'Cargo.lock']:
        shutil.copy2(ROOT / name, project / name)
    for name in ['src', 'examples']:
        shutil.copytree(ROOT / name, project / name)
    version = isolate_package(project)
    probe = project / 'examples/frame_latency_profile/presentation.rs'
    probe.write_text(instrument_probe(probe.read_text(), 2880 if args.long_capture else 480))
    native = project / 'examples/frame_latency_profile/native.rs'
    source = replace_once(native.read_text(), '        fs::create_dir_all(output).unwrap();',
                          '        fs::create_dir_all(output).unwrap();\n        self.probe.save_requests(output);')
    source = replace_once(source, 'writeln!(metadata, "diagnostic_draw={}", self.diagnostic_draw).unwrap();',
                          'writeln!(metadata, "diagnostic_draw=wire\\ndiagnostic_build={}", env!("CARGO_PKG_VERSION")).unwrap();')
    if args.long_capture:
        source = replace_once(source, 'const PER_PHASE: usize = 120;', 'const PER_PHASE: usize = 720;')
        source = replace_once(source, 'frames_per_phase=120', 'frames_per_phase=720')
    native.write_text(source)
    target = isolate_example(project, 'frame_latency_profile')
    command = ['cargo', 'build', '--offline', '--release', '--example', target,
               '--manifest-path', str(project / 'Cargo.toml'), '--target-dir', str(ROOT / 'target')]
    snapshot = dict(diagnostic_only=True, frames_per_phase=720 if args.long_capture else 120, package_version=version, command=command,
                    sources={str(p.relative_to(project)): hashlib.sha256(p.read_bytes()).hexdigest()
                             for p in project.rglob('*.rs')})
    (output / 'manifest.json').write_text(json.dumps(snapshot, indent=2))
    with (output / 'build.log').open('x') as log:
        subprocess.run(command, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, check=True)
    binary = output / 'frame-wire-diagnostic'
    shutil.copy2(ROOT / f'target/release/examples/{target}', binary)
    snapshot['binary_sha256'] = hashlib.sha256(binary.read_bytes()).hexdigest()
    (output / 'manifest.json').write_text(json.dumps(snapshot, indent=2))
    print(binary)


if __name__ == '__main__':
    main()
