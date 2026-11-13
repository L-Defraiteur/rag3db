"""Fixtures uniquement : le faux cargo ne compile et ne résout aucun paquet."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

SCRIPTS = Path(__file__).resolve().parent


class GuardTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix="cx-garde-target-")
        self.root = Path(self.tmp.name)
        self.addCleanup(self.tmp.cleanup)
        self.crate = self.root / "crate"
        self.crate.mkdir()
        (self.crate / "Cargo.toml").write_text('[package]\nname="local"\nversion="0.1.0"\n')
        self.target = self.root / "target"
        self.bin = self.root / "bin"
        self.bin.mkdir()
        cargo = self.bin / "cargo"
        cargo.write_text('''#!/usr/bin/env python3
import json,os,sys
from pathlib import Path
with open(os.environ['CALLS'],'a') as f:f.write(json.dumps(sys.argv[1:])+'\\n')
if sys.argv[1]=='metadata':
 print(json.dumps({'workspace_root':os.environ['CRATE'],'target_directory':os.environ['TARGET'],'packages':[{'name':'local','source':None}]}))
else:
 Path(os.environ['COMPILED']).write_text(os.environ.get('CARGO_TARGET_DIR',''))
''')
        cargo.chmod(0o755)
        self.env = dict(os.environ, PATH=str(self.bin) + ":" + os.environ["PATH"],
                        CRATE=str(self.crate), TARGET=str(self.target),
                        CALLS=str(self.root / "calls"), COMPILED=str(self.root / "compiled"))
        self.env.pop("RAG3WEAVER_TARGET_REPRENDRE", None)
        self.env.pop("CARGO_TARGET_DIR", None)

    def guard(self, resume=False):
        env = dict(self.env, RAG3WEAVER_TARGET_REPRENDRE="1" if resume else "0")
        return subprocess.run(['bash', '-c', '. "$1"; garder_le_target "$2" "$3"',
                               'fixture', str(SCRIPTS / 'garde_target.sh'), str(self.target), str(self.crate)],
                              env=env, capture_output=True, text=True)

    def test_empty_target_can_be_claimed(self):
        self.assertEqual(self.guard().returncode, 0)
        self.assertEqual((self.target / '.arbre-du-target').read_text().strip(), str(self.crate))

    def test_unknown_nonempty_target_refused_even_with_resume(self):
        self.target.mkdir()
        artifact = self.target / 'foreign-output'
        artifact.write_bytes(b'unknown provenance')
        for resume in (False, True):
            with self.subTest(resume=resume):
                self.assertEqual(self.guard(resume).returncode, 2)
                self.assertEqual(artifact.read_bytes(), b'unknown provenance')
                self.assertFalse((self.target / '.arbre-du-target').exists())

    def test_symlink_marker_refused_without_writing_its_target(self):
        self.target.mkdir()
        external = self.root / 'external'
        external.write_text('untouched')
        (self.target / '.arbre-du-target').symlink_to(external)
        self.assertEqual(self.guard(True).returncode, 2)
        self.assertEqual(external.read_text(), 'untouched')

    def test_known_retake_invalidates_only_local_fingerprints(self):
        self.target.mkdir()
        (self.target / '.arbre-du-target').write_text('/old/crate\n')
        for profile in ('debug', 'triplet/release'):
            for package in ('local', 'external'):
                fp = self.target / profile / '.fingerprint' / (package + '-0123456789abcdef')
                fp.mkdir(parents=True)
                (fp / 'output').write_text(package)
        self.assertEqual(self.guard().returncode, 2)
        result = self.guard(True)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertFalse(list(self.target.rglob('local-0123456789abcdef')))
        self.assertEqual(len(list(self.target.rglob('external-0123456789abcdef'))), 2)

    def test_invalid_marker_is_not_a_proven_origin(self):
        self.target.mkdir()
        marker = self.target / '.arbre-du-target'
        for value in ('', 'relative/path'):
            marker.write_text(value)
            self.assertEqual(self.guard(True).returncode, 2)
            self.assertEqual(marker.read_text(), value)

    def wrapper(self, *args):
        scripts = self.crate / 'scripts'
        scripts.mkdir(exist_ok=True)
        for name in ('cargo_magasin.sh', 'garde_target.sh', 'cargo_garde.py'):
            if (SCRIPTS / name).exists():
                shutil.copy2(SCRIPTS / name, scripts / name)
        (scripts / 'lib_du_magasin.sh').write_text('exporter_la_lib() { return 0; }\n')
        return subprocess.run(['bash', str(scripts / 'cargo_magasin.sh'), *args], cwd=self.crate,
                              env=self.env, capture_output=True, text=True)

    def test_wrapper_refuses_unknown_target_before_build(self):
        self.target.mkdir()
        (self.target / 'foreign').write_text('persist')
        self.assertEqual(self.wrapper('test', '--lib').returncode, 2)
        self.assertFalse((self.root / 'compiled').exists())

    def test_wrapper_uses_metadata_target_not_an_assumed_default(self):
        result = self.wrapper('check', '--lib')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.target / '.arbre-du-target').read_text().strip(), str(self.crate))
        self.assertEqual((self.root / 'compiled').read_text(), str(self.target))

    def test_wrapper_honors_explicit_target_and_manifest(self):
        explicit = self.root / 'explicit'
        result = self.wrapper('test', '--manifest-path', str(self.crate / 'Cargo.toml'),
                              '--target-dir=' + str(explicit), '--', '--target-dir=not-cargo')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((explicit / '.arbre-du-target').read_text().strip(), str(self.crate))
        self.assertFalse(self.target.exists())
        calls = [json.loads(x) for x in (self.root / 'calls').read_text().splitlines()]
        self.assertIn('--manifest-path', calls[0])
        self.assertIn('--target-dir=not-cargo', calls[-1])

    def test_wrapper_refuses_unqualified_config_override(self):
        self.assertEqual(self.wrapper('check', '--config', 'build.target-dir="other"').returncode, 2)
        self.assertFalse((self.root / 'compiled').exists())
        self.assertFalse(self.target.exists())


if __name__ == '__main__':
    unittest.main()
