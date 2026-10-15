"""Patch only the expected dependency source; diagnostic evidence is not acceptance."""
import unittest

from profile_public_chart import replace_once, patch_debug, isolate_package, isolate_example


class DiagnosticPatchTests(unittest.TestCase):
    def test_diagnostic_build_preserves_the_normal_example_executable(self):
        from pathlib import Path
        import shutil
        import subprocess
        import tempfile
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            original = root / 'original'
            (original / 'src').mkdir(parents=True)
            (original / 'src/lib.rs').write_text('')
            (original / 'examples').mkdir()
            (original / 'examples/scene.rs').write_text('fn main() { println!("normal"); }')
            (original / 'Cargo.toml').write_text('[package]\nname="probe"\nversion = "0.1.0"\nedition="2024"\n')
            def build(project, target):
                subprocess.run(['cargo', 'build', '--offline', '--quiet', '--manifest-path',
                                str(project / 'Cargo.toml'), '--example', target,
                                '--target-dir', str(root / 'target')], check=True)
            build(original, 'scene')
            copy = root / 'copy'
            shutil.copytree(original, copy)
            isolate_package(copy)
            target = isolate_example(copy, 'scene')
            (copy / f'examples/{target}.rs').write_text('fn main() { println!("diagnostic"); }')
            build(copy, target)
            output = root / 'target/debug/examples'
            self.assertEqual(subprocess.check_output([str(output / 'scene')], text=True).strip(), 'normal')
            self.assertEqual(subprocess.check_output([str(output / target)], text=True).strip(), 'diagnostic')

    def test_copied_source_changes_reach_executables_in_a_shared_target(self):
        import os
        from pathlib import Path
        import shutil
        import subprocess
        import tempfile
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            original = root / 'original.rs'
            original.write_text('fn main() { println!("1"); }')
            for name in ['before', 'after']:
                project = root / name
                project.mkdir()
                (project / 'src').mkdir()
                (project / 'Cargo.toml').write_text('[package]\nname="probe"\nversion = "0.1.0"\nedition="2024"\n')
                shutil.copy2(original, project / 'src/main.rs')
                if name == 'after':
                    (project / 'src/main.rs').write_text('fn main() { println!("2"); }')
                    stamp = original.stat()
                    os.utime(project / 'src/main.rs', ns=(stamp.st_atime_ns, stamp.st_mtime_ns))
                isolate_package(project)
                subprocess.run(['cargo', 'build', '--offline', '--quiet', '--manifest-path',
                                str(project / 'Cargo.toml'), '--target-dir', str(root / 'target')], check=True)
                actual = subprocess.check_output([str(root / 'target/debug/probe')], text=True).strip()
                self.assertEqual(actual, '1' if name == 'before' else '2')

    def test_copied_projects_have_distinct_package_ids(self):
        import tempfile
        from pathlib import Path
        with tempfile.TemporaryDirectory() as directory:
            versions = []
            for name in ['before', 'after']:
                project = Path(directory) / name
                project.mkdir()
                (project / 'Cargo.toml').write_text('[package]\nversion = "0.1.0"\n')
                versions.append(isolate_package(project))
                self.assertIn(versions[-1], (project / 'Cargo.toml').read_text())
            self.assertNotEqual(*versions)

    def test_patch_refuses_missing_or_ambiguous_source(self):
        for source in ['unrelated', 'old old']:
            with self.assertRaises(ValueError):
                replace_once(source, 'old', 'new')
        self.assertEqual(replace_once('prefix old suffix', 'old', 'new'), 'prefix new suffix')

    def test_only_disabled_debug_hooks_are_patched(self):
        source = '''enabled sentinel
#[cfg(any(not(feature = "enable"), target_arch = "wasm32"))]
mod internal {
    pub fn draw(_window: window::Id) -> Span {
        Span
    }
    #[derive(Debug)]
    pub struct Span;

    impl Span {
        pub fn finish(self) {}
    }
}
'''
        result = patch_debug(source, '/tmp/spans.rs')
        self.assertIn('enabled sentinel', result)
        self.assertIn('Span::new("draw")', result)
        self.assertNotIn('pub struct Span;', result)
        self.assertIn('diagnostic_record', result)
        with self.assertRaises(ValueError):
            patch_debug('unknown version', '/tmp/spans.rs')

class RuntimeTraceTests(unittest.TestCase):
    def test_trace_keeps_discarded_and_unresolved_frames(self):
        from analyze_public_chart import summarize
        rows = [dict(stage='diagnostic_only', id=0, start_ns=0, end_ns=0, cpu_ns=0, a=1, b=0),
                dict(stage='overflow', id=0, start_ns=0, end_ns=0, cpu_ns=0, a=0, b=0),
                dict(stage='clock_id', id=0, start_ns=1, end_ns=1, cpu_ns=0, a=1, b=0)]
        for frame in range(3):
            rows.append(dict(stage='submit', id=frame, start_ns=100+frame*100,
                             end_ns=100+frame*100, cpu_ns=0, a=0, b=0))
        rows += [dict(stage='presented', id=0, start_ns=160, end_ns=160, cpu_ns=0, a=150, b=16),
                 dict(stage='discarded', id=1, start_ns=260, end_ns=260, cpu_ns=0, a=0, b=0)]
        result = summarize(rows)
        self.assertTrue(result['diagnostic_only'])
        self.assertEqual((result['submitted'], result['discarded'], result['unresolved']), (3, 1, 1))
        self.assertIsNone(result['submission_to_presentation_p95_ns'])
        with self.assertRaises(ValueError):
            summarize(rows + [rows[-2]])
        rows[1]['a'] = 1
        with self.assertRaises(ValueError):
            summarize(rows)


if __name__ == '__main__':
    unittest.main()
