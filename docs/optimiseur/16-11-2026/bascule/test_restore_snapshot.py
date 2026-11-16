#!/usr/bin/env python3
"""Fixtures de retour, sans moteur ni unités réelles. --legacy rend le rouge historique."""
import hashlib
import json
import os
from pathlib import Path
import shlex
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).with_name('restore_snapshot.py')
LEGACY = '--legacy' in sys.argv
if LEGACY:
    sys.argv.remove('--legacy')


def shape(root):
    result = {}
    for path in sorted(root.rglob('*')):
        key = str(path.relative_to(root))
        if path.is_symlink():
            result[key] = ['link', os.readlink(path)]
        elif path.is_dir():
            result[key] = ['directory']
        else:
            result[key] = ['file', hashlib.sha256(path.read_bytes()).hexdigest()]
    return result


class RestoreTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='restore-fixture-', dir=Path.home()/'.cache')
        self.root = Path(self.temp.name)
        self.live = self.root/'live'
        self.snapshot = self.root/'snapshot'/'sample'
        self.live.mkdir()
        self.snapshot.mkdir(parents=True)
        self.name = 'sample.rag3db'
        for folder in [self.live, self.snapshot]:
            (folder/self.name).write_bytes(b'old-base')
            (folder/(self.name+'.extensions')).write_bytes(b'old-extension')
            (folder/(self.name+'.fts')).mkdir()
            (folder/(self.name+'.fts')/'segment').write_bytes(b'old-fts')
            (folder/'sample.checkpoints').mkdir()
        self.old = shape(self.snapshot)
        rows = []
        for path in sorted(self.snapshot.rglob('*')):
            st = path.lstat()
            row = dict(path=str(path), mode=st.st_mode, uid=st.st_uid, gid=st.st_gid,
                       size=st.st_size, mtime_ns=st.st_mtime_ns)
            if path.is_file():
                row['sha256'] = hashlib.sha256(path.read_bytes()).hexdigest()
            rows.append(row)
        self.inventory = self.root/'inventory.json'
        self.inventory.write_text(json.dumps(dict(files=rows)))
        # Ces noms n'existaient pas à la prise, ou portent désormais un contenu différent.
        (self.live/self.name).write_bytes(b'new-base')
        (self.live/(self.name+'.wal')).write_bytes(b'new-wal-never-replay')
        (self.live/(self.name+'.readers')).write_bytes(b'')
        (self.live/(self.name+'.new-directory')).mkdir()
        (self.live/(self.name+'.new-directory')/'evidence').write_bytes(b'after-only')
        (self.live/(self.name+'.new-link')).symlink_to('external-evidence')
        self.after = shape(self.live)
        fake = self.root/'bin'
        fake.mkdir()
        unit = fake/'systemctl'
        unit.write_text('#!/bin/sh\nprintf "%s\\n" "${FIXTURE_UNIT_STATE:-inactive}"\n')
        unit.chmod(0o755)
        self.env = dict(os.environ, PATH=str(fake)+':'+os.environ['PATH'])
        self.evidence = self.root/'proof'

    def tearDown(self):
        self.temp.cleanup()

    def run_recipe(self):
        if LEGACY:
            # Forme exacte du générateur historique : les noms sont figés AVANT le WAL nouveau.
            listed = [self.live/path.name for path in self.snapshot.iterdir()]
            saved = self.live/'after-return-legacy'
            recipe = ('mkdir '+shlex.quote(str(saved))+'\nmv '+
                      ' '.join(shlex.quote(str(p)) for p in listed)+' '+shlex.quote(str(saved))+'\n'+
                      'cp -a --reflink=always '+shlex.quote(str(self.snapshot))+'/. '+shlex.quote(str(self.live))+'\n')
            result = subprocess.run(['bash','-e','-c',recipe], env=self.env, capture_output=True, text=True)
            return result
        return subprocess.run([sys.executable,str(SCRIPT),'--snapshot',str(self.snapshot),
                               '--base',str(self.live/self.name),'--inventory',str(self.inventory),
                               '--evidence',str(self.evidence)],env=self.env,capture_output=True,text=True)

    def test_future_wal_and_all_new_neighbours_leave_the_live_set(self):
        result = self.run_recipe()
        self.assertEqual(result.returncode,0,result.stderr)
        saved = list(self.live.glob('after-return-*'))
        self.assertEqual(len(saved),1)
        self.assertEqual(shape(saved[0]),self.after)
        restored = shape(self.live)
        restored = {k:v for k,v in restored.items() if k!=saved[0].name and not k.startswith(saved[0].name+'/')}
        self.assertEqual(restored,self.old)
        self.assertFalse((self.live/(self.name+'.wal')).exists())
        self.assertEqual((saved[0]/(self.name+'.wal')).read_bytes(),b'new-wal-never-replay')
        self.assertEqual(shape(self.snapshot),self.old)

    def test_corrupt_snapshot_refuses_before_moving_live_files(self):
        if LEGACY:
            return
        (self.snapshot/self.name).write_bytes(b'corrupt-snapshot')
        result = self.run_recipe()
        self.assertNotEqual(result.returncode,0)
        self.assertEqual(shape(self.live),self.after)
        self.assertEqual(json.loads((self.evidence/'verdict.json').read_text())['verdict'],'RED')

    def test_extra_snapshot_file_refuses_before_moving_live_files(self):
        if LEGACY:
            return
        (self.snapshot/'unexpected').write_bytes(b'extra')
        result = self.run_recipe()
        self.assertNotEqual(result.returncode,0)
        self.assertEqual(shape(self.live),self.after)

    def test_evidence_inside_snapshot_is_rejected_without_writing_it(self):
        if LEGACY:
            return
        self.evidence = self.snapshot/'unsafe-evidence'
        result = self.run_recipe()
        self.assertNotEqual(result.returncode,0)
        self.assertFalse(self.evidence.exists())
        self.assertEqual(shape(self.snapshot),self.old)
        self.assertEqual(shape(self.live),self.after)

    def test_active_api_refuses_before_moving_live_files(self):
        if LEGACY:
            return
        self.env['FIXTURE_UNIT_STATE']='active'
        result = self.run_recipe()
        self.assertNotEqual(result.returncode,0)
        self.assertEqual(shape(self.live),self.after)


if __name__=='__main__':
    if LEGACY:
        suite = unittest.TestSuite([RestoreTests('test_future_wal_and_all_new_neighbours_leave_the_live_set')])
        raise SystemExit(not unittest.TextTestRunner(verbosity=2).run(suite).wasSuccessful())
    unittest.main(verbosity=2)
