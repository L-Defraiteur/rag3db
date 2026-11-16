#!/usr/bin/env python3
"""Exécute le vrai générateur et son RET, exclusivement sur fichiers privés."""
import hashlib
import json
import os
import shutil
import socket
import stat
from pathlib import Path
import subprocess
import tempfile
import unittest

GENERATOR = Path(__file__).with_name('instantanes.sh').resolve()


def shape(root):
    result = {}
    for path in sorted(root.rglob('*')):
        key = str(path.relative_to(root))
        if path.is_symlink():
            result[key] = ['link', os.readlink(path)]
        elif path.is_dir():
            result[key] = ['directory']
        elif path.is_file():
            result[key] = ['file', hashlib.sha256(path.read_bytes()).hexdigest()]
        else:
            result[key] = ['special', stat.S_IFMT(path.lstat().st_mode)]
    return result


class GeneratedReturn(unittest.TestCase):
    def test_generated_return_preserves_future_wal_and_all_neighbours(self):
        # Pas de nettoyage : anciennes données, état après et journaux restent auditables.
        root = Path(tempfile.mkdtemp(prefix='generated-return-', dir=Path.home()/'.cache'))
        home = root/'home'
        live = root/'private-files'
        live.mkdir(); home.mkdir()
        base = live/'sample.rag3db'
        base.write_bytes(b'old-base')
        (live/'sample.rag3db.extensions').write_bytes(b'old-extensions')
        (live/'sample.rag3db.fts').mkdir()
        (live/'sample.rag3db.fts'/'segment').write_bytes(b'old-fts')
        (live/'sample.checkpoints').mkdir()
        (live/'sample.checkpoints'/'checkpoint').write_bytes(b'old-checkpoint')
        old = shape(live)
        fake = root/'bin'; fake.mkdir()
        (fake/'systemctl').write_text('#!/bin/sh\nprintf "inactive\\n"\n')
        (fake/'systemctl').chmod(0o755)
        env = dict(os.environ, HOME=str(home), PATH=str(fake)+':'+os.environ['PATH'])
        p = subprocess.run(['bash',str(GENERATOR),'fixture',str(base)], env=env,
                           text=True,capture_output=True,timeout=120)
        (root/'generation.log').write_text(p.stdout+p.stderr)
        self.assertEqual(p.returncode,0,p.stdout+p.stderr)
        snapshot_root = home/'.local/share/rag3weaver/instantanes/fixture'
        snapshot = snapshot_root/'sample'
        self.assertEqual(shape(snapshot),old)
        base.write_bytes(b'new-base')
        (live/'sample.rag3db.wal').write_bytes(b'new-wal-never-replay')
        (live/'sample.rag3db.new-directory').mkdir()
        (live/'sample.rag3db.new-directory'/'content').write_bytes(b'after-only')
        (live/'sample.rag3db.new-link').symlink_to('external-evidence')
        (live/'sample.rag3db.readers').write_bytes(b'')
        after = shape(live)
        p = subprocess.run(['bash',str(snapshot_root/'RETOUR.sh')],env=env,
                           text=True,capture_output=True,timeout=60)
        (root/'return.log').write_text(p.stdout+p.stderr)
        (root/'identities.json').write_text(json.dumps({'root':str(root),'old':old,'after':after},indent=2))
        print('fixture='+str(root),flush=True)
        self.assertEqual(p.returncode,0,p.stdout+p.stderr)
        self.assertFalse((live/'sample.rag3db.wal').exists(), 'le RET généré laisse le nouveau WAL dans le vivant')
        saved = [p for p in live.iterdir() if p.is_dir() and p.name.startswith(('after-return-','apres-'))]
        self.assertEqual(len(saved),1)
        self.assertEqual(shape(saved[0]),after)
        current = shape(live)
        current = {k:v for k,v in current.items() if k!=saved[0].name and not k.startswith(saved[0].name+'/')}
        self.assertEqual(current,old)
        self.assertEqual(shape(snapshot),old)


class GeneratedReturnIntegrity(unittest.TestCase):
    def setUp(self):
        self.root = Path(tempfile.mkdtemp(prefix='generated-return-integrity-',dir=Path.home()/'.cache'))
        self.live = self.root/'private-files'; self.live.mkdir()
        self.snapshot = self.root/'snapshot'; self.snapshot.mkdir()
        self.bases = []
        for name in ["quoted's base", 'second']:
            base = self.live/(name+'.rag3db'); base.write_bytes(b'old-base')
            folder = self.snapshot/name; folder.mkdir()
            shutil.copy2(base,folder/base.name)
            self.bases.append(base)
        fake = self.root/'bin';fake.mkdir()
        (fake/'systemctl').write_text('#!/bin/sh\nprintf "%s\\n" "${FIXTURE_UNIT_STATE:-inactive}"\n')
        (fake/'systemctl').chmod(0o755)
        self.env = dict(os.environ,PATH=str(fake)+':'+os.environ['PATH'])
        result = subprocess.run(['python3',str(GENERATOR.with_name('generate_snapshot_return.py')),
                                 str(self.snapshot),*[str(b) for b in self.bases]],
                                capture_output=True,text=True,timeout=10)
        self.assertEqual(result.returncode,0,result.stderr)
        for base in self.bases:
            base.write_bytes(b'new-base')
            base.with_name(base.name+'.wal').write_bytes(b'new-wal')
        self.after = shape(self.live)

    def run_return(self):
        result = subprocess.run(['bash',str(self.snapshot/'RETOUR.sh')],env=self.env,
                                capture_output=True,text=True,timeout=20)
        (self.root/'return.log').write_text(result.stdout+result.stderr)
        print('fixture='+str(self.root),flush=True)
        return result

    def assert_refusal_unchanged(self):
        result = self.run_return()
        self.assertNotEqual(result.returncode,0,result.stdout+result.stderr)
        self.assertEqual(shape(self.live),self.after)

    def test_two_bases_and_quoted_path(self):
        result = self.run_return()
        self.assertEqual(result.returncode,0,result.stdout+result.stderr)
        saved = list(self.live.glob('after-return-*'))
        self.assertEqual(len(saved),2)
        for base in self.bases:
            self.assertEqual(base.read_bytes(),b'old-base')
            self.assertFalse(base.with_name(base.name+'.wal').exists())
            self.assertEqual(sum((d/(base.name+'.wal')).exists() for d in saved),1)

    def test_changed_helper_refuses_before_any_move(self):
        with (self.snapshot/'return-tools/restore_snapshot.py').open('a') as stream:
            stream.write('\n# unexpected change\n')
        self.assert_refusal_unchanged()

    def test_changed_inventory_refuses_before_any_move(self):
        (self.snapshot/'return-tools/second.json').write_text('{}')
        self.assert_refusal_unchanged()

    def test_corrupt_second_snapshot_refuses_before_first_move(self):
        (self.snapshot/'second/second.rag3db').write_bytes(b'corrupt')
        self.assert_refusal_unchanged()

    def test_absent_second_live_set_refuses_before_first_move(self):
        second = self.bases[1]
        second.unlink()
        second.with_name(second.name+'.wal').unlink()
        self.after = shape(self.live)
        self.assert_refusal_unchanged()

    def test_second_live_socket_refuses_before_first_move(self):
        endpoint = self.live/(self.bases[1].name+'.unsupported-socket')
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as listener:
            listener.bind(str(endpoint))
            self.after = shape(self.live)
            self.assert_refusal_unchanged()

    def test_nested_live_sets_refuse_before_any_move(self):
        first, second = self.bases
        nested = self.live/(first.name+'.nested')
        nested.mkdir()
        for path in [second, second.with_name(second.name+'.wal')]:
            path.rename(nested/path.name)
        self.bases[1] = nested/second.name
        target = self.root/'nested-snapshot'; target.mkdir()
        first_copy = target/first.stem; first_copy.mkdir()
        for path in self.live.iterdir():
            if path.name == first.name or path.name.startswith(first.name+'.'):
                if path.is_dir():
                    shutil.copytree(path,first_copy/path.name)
                else:
                    shutil.copy2(path,first_copy/path.name)
        second_copy = target/second.stem;second_copy.mkdir()
        for path in nested.iterdir():
            shutil.copy2(path,second_copy/path.name)
        result = subprocess.run(['python3',str(GENERATOR.with_name('generate_snapshot_return.py')),
                                 str(target),*[str(b) for b in self.bases]],
                                capture_output=True,text=True,timeout=10)
        self.assertEqual(result.returncode,0,result.stderr)
        self.snapshot = target
        self.after = shape(self.live)
        self.assert_refusal_unchanged()

    def test_snapshot_inside_live_neighbour_refuses_before_any_move(self):
        target = self.live/(self.bases[0].name+'.snapshot')
        target.mkdir()
        for base in self.bases:
            folder = target/base.stem;folder.mkdir()
            shutil.copy2(base,folder/base.name)
        result = subprocess.run(['python3',str(GENERATOR.with_name('generate_snapshot_return.py')),
                                 str(target),*[str(b) for b in self.bases]],
                                capture_output=True,text=True,timeout=10)
        self.assertEqual(result.returncode,0,result.stderr)
        self.snapshot = target
        self.after = shape(self.live)
        self.assert_refusal_unchanged()

    def test_active_api_refuses_before_any_move(self):
        self.env['FIXTURE_UNIT_STATE']='active'
        self.assert_refusal_unchanged()


if __name__ == '__main__':
    unittest.main(verbosity=2)
