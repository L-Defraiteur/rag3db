#!/usr/bin/env python3
"""Exécute le vrai générateur et son RET, exclusivement sur fichiers privés."""
import hashlib
import json
import os
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
        else:
            result[key] = ['file', hashlib.sha256(path.read_bytes()).hexdigest()]
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


if __name__ == '__main__':
    unittest.main(verbosity=2)
