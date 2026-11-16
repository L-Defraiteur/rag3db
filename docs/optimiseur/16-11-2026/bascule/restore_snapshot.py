#!/usr/bin/env python3
"""Retour opérateur à froid : vérifie le snapshot, conserve les voisins actuels, restaure l'ancien ensemble.

Ne lance ni moteur, ni migration, ni service. L'opérateur coordonne les détenteurs
externes avant l'appel ; l'API PC et sa socket doivent déjà être inactives.
La recette conserve les preuves et l'état après, même sur erreur ; aucun repli,
effacement automatique ou réveil. À exécuter sous le budget borné de l'opérateur.
"""
import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import shutil
import stat
import subprocess
import sys
import uuid


class Refusal(RuntimeError):
    pass


def require(condition, message):
    if not condition:
        raise Refusal(message)


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def nodes(root):
    result = []
    for folder, directories, files in os.walk(root, followlinks=False):
        parent = Path(folder)
        for name in directories + files:
            result.append(parent/name)
    return sorted(result)


def verify_snapshot(root, rows, original_root):
    expected = {}
    for row in rows:
        original = Path(row['path'])
        relative = original.relative_to(original_root)
        require(relative.parts and '..' not in relative.parts, 'invalid snapshot entry')
        expected[relative] = row
    actual = {p.relative_to(root): p for p in nodes(root)}
    require(set(expected) == set(actual), 'snapshot paths differ from the inventory')
    for relative, row in expected.items():
        path = actual[relative]
        info = path.lstat()
        require(not stat.S_ISLNK(info.st_mode), f'linked snapshot entry: {relative}')
        fields = [('mode',info.st_mode),('uid',info.st_uid),('gid',info.st_gid),('mtime_ns',info.st_mtime_ns)]
        if root == original_root:
            fields += [(k,v) for k,v in [('device',info.st_dev),('inode',info.st_ino),('ctime_ns',info.st_ctime_ns)] if k in row]
        for field, observed in fields:
            require(observed == row[field], f'snapshot {field} changed: {relative}')
        if stat.S_ISREG(info.st_mode):
            require(info.st_size == row['size'], f'snapshot size changed: {relative}')
            require(digest(path) == row['sha256'], f'snapshot SHA changed: {relative}')
        else:
            require(stat.S_ISDIR(info.st_mode), f'unsupported snapshot type: {relative}')
    return expected


def live_roots(base):
    return sorted(p for p in base.parent.iterdir()
                  if p.name == base.name or p.name.startswith(base.name+'.')
                  or p.name == base.with_suffix('.checkpoints').name)


def describe(roots, parent):
    result = {}
    for root in roots:
        entries = [root] + (nodes(root) if root.is_dir() and not root.is_symlink() else [])
        for path in entries:
            info = path.lstat()
            row = {'mode':info.st_mode,'uid':info.st_uid,'gid':info.st_gid,
                   'mtime_ns':info.st_mtime_ns}
            if stat.S_ISLNK(info.st_mode):
                row.update(type='link',target=os.readlink(path))
            elif stat.S_ISDIR(info.st_mode):
                row['type'] = 'directory'
            elif stat.S_ISREG(info.st_mode):
                row.update(type='file',size=info.st_size,sha256=digest(path))
            else:
                raise Refusal(f'unsupported live neighbour: {path.name}')
            result[str(path.relative_to(parent))] = row
    return result


def require_cold_api():
    states = {}
    for unit in ['rag3weaver-daemon.service','rag3weaver-daemon.socket']:
        state = subprocess.check_output(['systemctl','--user','show',unit,
                                         '-p','ActiveState','--value'],text=True,timeout=15).strip()
        require(state == 'inactive', f'API unit is not inactive: {unit} ({state})')
        states[unit] = state
    return states


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for flag in ['snapshot','base','inventory','evidence']:
        parser.add_argument('--'+flag,required=True,type=Path)
    args = parser.parse_args()
    evidence = args.evidence.absolute()
    snapshot = args.snapshot.absolute()
    base = args.base.absolute()
    try:
        require(not evidence.exists(), 'evidence already exists')
        require(evidence.resolve() == evidence and snapshot.resolve() == snapshot
                and base.parent.resolve() == base.parent, 'linked root or parent')
        require(not evidence.is_relative_to(snapshot), 'evidence inside the snapshot')
        if evidence.is_relative_to(base.parent):
            first = evidence.relative_to(base.parent).parts[0] if evidence != base.parent else ''
            require(first != base.name and not first.startswith(base.name+'.')
                    and first != base.with_suffix('.checkpoints').name, 'evidence intersects live data')
    except Refusal as error:
        print(str(error),file=sys.stderr)
        return 2
    evidence.mkdir(parents=True)
    report = {'verdict':'RED','live_restore_executed':False,'moved':[],
              'date':datetime.datetime.now().astimezone().isoformat()}
    try:
        snapshot = args.snapshot.absolute()
        base = args.base.absolute()
        require(snapshot.resolve() == snapshot and base.parent.resolve() == base.parent,
                'linked snapshot root or live parent')
        require(snapshot.is_dir() and base.name.endswith('.rag3db'), 'invalid snapshot or base')
        require(not snapshot.is_relative_to(base.parent) or snapshot != base.parent,
                'snapshot is the live parent')
        require(not evidence.is_relative_to(snapshot), 'evidence inside the snapshot')
        rows = json.loads(args.inventory.read_text())['files']
        require(rows and (snapshot/base.name).is_file(), 'snapshot base is missing')
        report['units'] = require_cold_api()
        verify_snapshot(snapshot,rows,snapshot)
        stage = evidence/'staged'
        stage.mkdir()
        subprocess.run(['cp','-a','--reflink=always',str(snapshot)+'/.',str(stage)],
                       check=True,timeout=600)
        verify_snapshot(stage,rows,snapshot)
        verify_snapshot(snapshot,rows,snapshot)
        roots = live_roots(base)
        require(roots, 'no current live state to preserve')
        before = describe(roots,base.parent)
        expected_old = describe(list(snapshot.iterdir()),snapshot)
        report['units'] = require_cold_api()
        saved = base.parent/('after-return-'+snapshot.parent.name+'-'+uuid.uuid4().hex)
        saved.mkdir()
        report.update(snapshot=str(snapshot),base=str(base),after_state=str(saved),
                      before=before,expected_restored=expected_old)
        # Tous les voisins ACTUELS sont déplacés, jamais une liste figée à la prise.
        for path in roots:
            shutil.move(str(path),str(saved/path.name))
            report['moved'].append(path.name)
        require(describe(list(saved.iterdir()),saved) == before, 'saved after-state differs')
        report['live_restore_executed'] = True
        for path in sorted(stage.iterdir()):
            require(not (base.parent/path.name).exists(), 'unexpected destination exists')
            shutil.move(str(path),str(base.parent/path.name))
        restored = describe(live_roots(base),base.parent)
        require(restored == expected_old, 'restored types, SHA or metadata differ')
        verify_snapshot(snapshot,rows,snapshot)
        report.update(verdict='VERT',restored=restored,after_state_preserved=True,
                      snapshot_data_unchanged=True)
        print(json.dumps({k:report[k] for k in ['verdict','after_state','base']}))
        return 0
    except Exception as error:
        report['error'] = f'{type(error).__name__}: {error}'
        print(report['error'],file=sys.stderr)
        return 1
    finally:
        (evidence/'verdict.json').write_text(json.dumps(report,indent=2))


if __name__ == '__main__':
    raise SystemExit(main())
