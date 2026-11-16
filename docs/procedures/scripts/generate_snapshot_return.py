#!/usr/bin/env python3
"""Émet un RET autonome à partir du helper V2 épinglé, sans ouvrir de base."""
import hashlib
import json
from pathlib import Path
import shutil
import stat
import sys

HELPER_SHA256 = 'e1fd96b25c1cc61b70507c59f7f203acb21936e5b676f68c74b88d216f923854'


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def generate(destination, bases):
    destination = destination.absolute()
    helper = Path(__file__).with_name('restore_snapshot.py')
    if digest(helper) != HELPER_SHA256:
        raise RuntimeError('return helper does not match its qualified SHA')
    tools = destination/'return-tools'
    tools.mkdir()
    copied_helper = tools/'restore_snapshot.py'
    shutil.copy2(helper, copied_helper)
    records = []
    for base in bases:
        base = base.absolute()
        snapshot = destination/base.stem
        rows = []
        for path in sorted(snapshot.rglob('*')):
            info = path.lstat()
            if not (stat.S_ISDIR(info.st_mode) or stat.S_ISREG(info.st_mode)):
                raise RuntimeError('snapshot has a link or unsupported entry: '+str(path))
            row = dict(path=str(path), mode=info.st_mode, uid=info.st_uid,
                       gid=info.st_gid, size=info.st_size, mtime_ns=info.st_mtime_ns,
                       device=info.st_dev, inode=info.st_ino, ctime_ns=info.st_ctime_ns)
            if stat.S_ISREG(info.st_mode):
                row['sha256'] = digest(path)
            rows.append(row)
        if not rows or not (snapshot/base.name).is_file():
            raise RuntimeError('snapshot base is missing: '+str(base))
        inventory = tools/(base.stem+'.json')
        with inventory.open('x') as stream:
            json.dump({'files':rows},stream,indent=2)
        records.append(dict(base=str(base),snapshot=str(snapshot),
                            inventory=str(inventory),inventory_sha256=digest(inventory)))
    configuration = dict(helper=str(copied_helper),helper_sha256=HELPER_SHA256,
                         evidence_parent=str(destination.parent),bases=records)
    # La recette ne dépend plus du worktree ni du cache du générateur.
    runtime = '''import hashlib, importlib.util, json, os, subprocess, sys, uuid
from pathlib import Path
sys.dont_write_bytecode = True
CONFIGURATION

def digest(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()

if set(config) != {'helper', 'helper_sha256', 'evidence_parent', 'bases'} or not config['bases']:
    raise SystemExit('invalid return configuration schema')
helper = Path(config['helper'])
if helper.resolve() != helper or helper.is_symlink() or digest(helper) != config['helper_sha256']:
    raise SystemExit('return helper SHA changed')
spec = importlib.util.spec_from_file_location('snapshot_return', helper)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
# Préflight GLOBAL : snapshots ET ensembles vivants, sans écrire ni déplacer.
evidence = Path(config['evidence_parent'])/('return-evidence-'+uuid.uuid4().hex)
if evidence.parent.resolve() != evidence.parent or evidence.exists():
    raise SystemExit('invalid evidence parent')
protected = [helper.parent, evidence]
live_sets = []
for item in config['bases']:
    if set(item) != {'base', 'snapshot', 'inventory', 'inventory_sha256'}:
        raise SystemExit('invalid base record schema')
    inventory = Path(item['inventory'])
    snapshot = Path(item['snapshot'])
    base = Path(item['base'])
    if inventory.resolve() != inventory or inventory.is_symlink() or digest(inventory) != item['inventory_sha256']:
        raise SystemExit('return inventory SHA changed')
    if snapshot.resolve() != snapshot or base.parent.resolve() != base.parent:
        raise SystemExit('linked snapshot or destination parent')
    rows = json.loads(inventory.read_text())['files']
    module.require(snapshot.is_dir() and base.name.endswith('.rag3db')
                   and (snapshot/base.name).is_file(), 'invalid snapshot or base')
    module.verify_snapshot(snapshot, rows, snapshot)
    roots = module.live_roots(base)
    module.require(roots, 'no current live state to preserve: '+str(base))
    module.describe(roots, base.parent)  # refuse les types spéciaux AVANT base1
    live_sets.append(roots)
    protected.extend([snapshot, inventory.parent])

def intersects(left, right):
    return left.is_relative_to(right) or right.is_relative_to(left)

seen = []
for roots in live_sets:
    for root in roots:
        module.require(not any(intersects(root, other) for other in seen),
                       'overlapping live sets')
        module.require(not any(intersects(root, path) for path in protected),
                       'live set intersects protected snapshot, tool or evidence')
    seen.extend(roots)
module.require_cold_api()
for item in config['bases']:
    subprocess.run([sys.executable, str(helper), '--snapshot', item['snapshot'],
                    '--base', item['base'], '--inventory', item['inventory'],
                    '--evidence', str(evidence/Path(item['base']).stem)],
                   check=True, timeout=660)
'''.replace('CONFIGURATION', 'config = '+repr(configuration))
    recipe = destination/'RETOUR.sh'
    with recipe.open('x') as stream:
        stream.write("#!/bin/sh\n# Retour à froid, helper et inventaires épinglés ; conservation dynamique de l'état après.\nset -eu\nexec python3 - <<'SNAPSHOT_RETURN_PY'\n"+runtime+'SNAPSHOT_RETURN_PY\n')
    recipe.chmod(0o755)
    (tools/'SHA256SUMS').write_text(''.join(digest(p)+'  '+p.name+'\n' for p in sorted(tools.iterdir()))+digest(recipe)+'  ../RETOUR.sh\n')


if __name__ == '__main__':
    if len(sys.argv) < 3:
        raise SystemExit('usage: generate_snapshot_return.py SNAPSHOT_ROOT BASE...')
    generate(Path(sys.argv[1]), [Path(p) for p in sys.argv[2:]])
