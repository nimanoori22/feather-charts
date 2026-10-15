"""Build an isolated, diagnostic-only copy of the real demo and Iced timing hooks."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def replace_once(source, old, new):
    if source.count(old) != 1:
        raise ValueError(f'expected one source anchor, found {source.count(old)}: {old[:100]}')
    return source.replace(old, new, 1)


def isolate_package(project):
    # Cargo shares artifact names for equal package IDs across copied manifests.
    version = '0.1.0+probe.' + hashlib.sha256(str(project.resolve()).encode()).hexdigest()[:16]
    manifest = project / 'Cargo.toml'
    manifest.write_text(replace_once(manifest.read_text(), 'version = "0.1.0"', f'version = "{version}"'))
    return version


def isolate_example(project, name):
    diagnostic = f'feather_diagnostic_{name}'
    (project / f'examples/{name}.rs').rename(project / f'examples/{diagnostic}.rs')
    return diagnostic


def patch_debug(source, span_path):
    anchor = '#[cfg(any(not(feature = "enable"), target_arch = "wasm32"))]'
    before, after = source.split(anchor, 1) if source.count(anchor) == 1 else ('', '')
    if not after:
        raise ValueError('unsupported iced_debug source')
    after, count = re.subn(
        r'(pub fn (\w+)\([^)]*\) -> Span \{\n)        Span\n',
        lambda m: m[1] + f'        Span::new("{m[2]}")\n', after)
    if not count:
        raise ValueError('no disabled span hooks found')
    after = replace_once(after, 'Span::new("time")',
                         'Span::new(match _name.into().as_str() { "chart_prepare" => "chart_prepare", "chart_geometry" => "chart_geometry", "update_total" => "update_total", "update_validate" => "update_validate", "update_layer" => "update_layer", "update_model" => "update_model", _ => "time" })') if 'Span::new("time")' in after else after
    after = replace_once(after, '''    #[derive(Debug)]
    pub struct Span;

    impl Span {
        pub fn finish(self) {}
    }''', '    pub use crate::interaction_trace::Span;')
    return (before + anchor + after + '\n#[path = ' + json.dumps(str(span_path)) +
            ']\nmod interaction_trace;\npub use interaction_trace::{diagnostic_record, diagnostic_flush};\n')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--build-only', action='store_true')
    parser.add_argument('--example', choices=['public_chart', 'preparation_profile'], default='public_chart')
    parser.add_argument('--backend', choices=['tiny-skia', 'wgpu'], default='tiny-skia')
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    metadata = json.loads(subprocess.check_output(
        ['cargo', 'metadata', '--offline', '--filter-platform', 'x86_64-unknown-linux-gnu', '--format-version', '1'], cwd=ROOT))
    packages = {p['name']: p for p in metadata['packages']}
    project = output / 'project'
    project.mkdir()
    for name in ['Cargo.toml', 'Cargo.lock']:
        shutil.copy2(ROOT / name, project / name)
    for name in ['src', 'examples']:
        shutil.copytree(ROOT / name, project / name)
    version = isolate_package(project)
    manifest = project / 'Cargo.toml'
    manifest.write_text(replace_once(manifest.read_text(), '[dependencies]',
                                    '[dependencies]\niced_debug = "0.14.0"'))
    patches, original_hashes = {}, {}
    for name in ['iced_debug', 'iced_winit']:
        package = packages[name]
        if package['version'] != {'iced_debug': '0.14.0', 'iced_winit': '0.14.1'}[name]:
            raise ValueError(f'unsupported {name} version: {package["version"]}')
        source = Path(package['manifest_path']).parent
        destination = output / name
        shutil.copytree(source, destination)
        patches[name] = destination
        original_hashes[name] = hashlib.sha256((source / 'src/lib.rs').read_bytes()).hexdigest()
    debug = patches['iced_debug'] / 'src/lib.rs'
    debug.write_text(patch_debug(debug.read_text(), project / 'examples/support/runtime_profile_spans.rs'))
    with (patches['iced_debug'] / 'Cargo.toml').open('a') as f:
        f.write('\n[target.\'cfg(target_os = "linux")\'.dependencies.libc]\nversion = "0.2"\n')
    native = patches['iced_winit'] / 'src/lib.rs'
    source = native.read_text()
    source = replace_once(source, '                        let present_span = debug::present(id);',
                          '                        interaction_native::drain();\n'
                          '                        let present_span = debug::present(id);')
    source = replace_once(source, '                            || window.raw.pre_present_notify(),',
                          '                            || {\n'
                          '                                interaction_native::request(window.raw.clone());\n'
                          '                                window.raw.pre_present_notify();\n'
                          '                            },')
    source = replace_once(source, '                        match window_event {',
                          '                        interaction_native::input(&window_event);\n'
                          '                        match window_event {')
    source += '\nuse feather_runtime_profile as interaction_native;\n'
    helper = output / 'native_probe'
    helper.mkdir()
    (helper / 'lib.rs').write_text('#[path = ' + json.dumps(str(project / 'examples/support/runtime_profile_native.rs')) + ']\nmod implementation;\npub use implementation::{drain, request, input};\n')
    (helper / 'Cargo.toml').write_text('''[package]
name = "feather_runtime_profile"
version = "0.0.0"
edition = "2024"
[lib]
path = "lib.rs"
[dependencies]
iced_debug = "0.14.0"
winit = "0.30"
wayland-client = { version = "0.31", features = ["system"] }
wayland-protocols = { version = "0.32", features = ["client"] }
libc = "0.2"
''')
    native.write_text(source)
    with (patches['iced_winit'] / 'Cargo.toml').open('a') as f:
        f.write('\n[dependencies.feather_runtime_profile]\npath = ' + json.dumps(str(helper)) + '\n')
    chart = project / 'src/api/chart.rs'
    chart.write_text(replace_once(chart.read_text(),
        '        if self.size != size || self.ratio != ratio {',
        '        let _span = iced_debug::time("chart_prepare");\n'
        '        iced_debug::diagnostic_record("chart_bounds", 0, size.width.to_bits() as u64, size.height.to_bits() as u64);\n'
        '        if self.size != size || self.ratio != ratio {'))
    widget = project / 'src/api/widget.rs'
    widget_source = widget.read_text()
    draw_anchor = ('        _state.geometry.clear();' if '        _state.geometry.clear();' in widget_source
                   else '        let mut frame = Frame::new(renderer, bounds.size());')
    widget.write_text(replace_once(widget_source, draw_anchor,
        '        let _span = iced_debug::time("chart_geometry");\n'
        '        iced_debug::diagnostic_record("canvas_origin", 0, bounds.x.to_bits() as u64, bounds.y.to_bits() as u64);\n'
        + draw_anchor))
    chart_source = chart.read_text()
    chart.write_text(replace_once(chart_source, '                self.frame = frame;',
        '                for (index, pane) in frame.panes.iter().enumerate() {\n'
        '                    let plot = pane.layout.plot;\n'
        '                    iced_debug::diagnostic_record("plot_origin", index as u64, plot.origin.x.to_bits(), plot.origin.y.to_bits());\n'
        '                    iced_debug::diagnostic_record("plot_size", index as u64, plot.size.width.to_bits(), plot.size.height.to_bits());\n'
        '                }\n'
        '                self.frame = frame;'))
    if args.example == 'preparation_profile':
        source = chart.read_text()
        source = replace_once(source,
            '        validate_data(self.kind(id)?, std::slice::from_ref(&item), false)?;\n'
            '        let update = self.layer.update_series_data(id, item, historical)?;\n'
            '        self.model.apply_data_update(update)?;',
            '        let _total = iced_debug::time("update_total");\n'
            '        { let _s = iced_debug::time("update_validate");\n'
            '          validate_data(self.kind(id)?, std::slice::from_ref(&item), false)?; }\n'
            '        let update = { let _s = iced_debug::time("update_layer");\n'
            '          self.layer.update_series_data(id, item, historical)? };\n'
            '        { let _s = iced_debug::time("update_model");\n'
            '          self.model.apply_data_update(update)?; }')
        chart.write_text(source)
        example = project / 'examples/preparation_profile.rs'
        example.write_text(replace_once(example.read_text(), 'fn main() {',
            'fn main() {\n    println!("diagnostic_only=true");'))
    target = isolate_example(project, args.example)
    command = ['cargo', 'build', '--offline', '--release', '--example', target,
               '--manifest-path', str(manifest), '--target-dir', str(ROOT / 'target')]
    for name, destination in patches.items():
        command += ['--config', f'patch.crates-io.{name}.path={json.dumps(str(destination))}']
    snapshot = dict(diagnostic_only=True, package_version=version, backend=args.backend, build_command=command,
                    original_dependency_hashes=original_hashes,
                    sources={str(p.relative_to(project)): hashlib.sha256(p.read_bytes()).hexdigest()
                             for p in project.rglob('*.rs')})
    (output / 'manifest.json').write_text(json.dumps(snapshot, indent=2) + '\n')
    with (output / 'build.log').open('w') as log:
        subprocess.run(command, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, check=True)
    binary = output / (args.example.replace('_', '-') + '-diagnostic')
    shutil.copy2(ROOT / f'target/release/examples/{target}', binary)
    snapshot['binary_sha256'] = hashlib.sha256(binary.read_bytes()).hexdigest()
    (output / 'manifest.json').write_text(json.dumps(snapshot, indent=2) + '\n')
    if not args.build_only:
        if args.example != 'public_chart':
            raise ValueError('use --build-only and run the recorded preparation binary explicitly')
        env = dict(os.environ, ICED_BACKEND=args.backend, FEATHER_RUNTIME_TRACE=str(output / 'trace.csv'))
        subprocess.run([str(binary), '--histogram', '--primitives', '--live-feed'], env=env, check=True)


if __name__ == '__main__':
    main()
