"""Pre/post checks inside the live Common scope, before any target adoption."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
from relocate_target import scan, digest

worktree, target, evidence, phase = map(Path, sys.argv[1:])
phase = str(phase)
subprocess.run([sys.executable, str(worktree / 'extension/rag3weaver/scripts/confiner_validation.py'), 'verifier'], check=True)
base = Path.home() / '.cache/rag3weaver-build'
record = dict(phase=phase)
try:
    old = base / 'validations/validation-6tdyny5z'
    authority = json.loads((old / 'autorite.json').read_text())
    verdict = json.loads((old / 'verdict.json').read_text())
    assert authority['etat'] == 'termine' and verdict['restes'] == [] and verdict['nettoye']
    record['previous_authority'] = dict(pid=authority['pid'], start=authority['debut'], state=authority['etat'])
    proc = Path('/proc') / str(authority['pid']) / 'stat'
    if proc.exists():
        assert proc.read_text().rsplit(') ', 1)[1].split()[19] != authority['debut']
    record['scan'] = scan(target, worktree)
    assert not record['scan']['attributed']
    assert target.resolve() == target and target.stat().st_uid == os.getuid()
    owner = (target / '.arbre-du-target').read_text().strip()
    assert owner == str(worktree / 'extension/rag3weaver/tty')
    record['owner'] = owner
    record['identity'] = dict(uid=target.stat().st_uid, dev=target.stat().st_dev, ino=target.stat().st_ino)
    # Les artefacts précédents restent traçables avant la garde réelle.
    record['elf'] = {str(p.relative_to(target)): digest(p) for p in sorted((target / 'debug/deps').glob('rag3weaver_tty-*')) if p.is_file() and not p.suffix and os.access(p, os.X_OK)}
    record['archive_sha'] = digest(base / 'archives/cx-tty-aliases-red-20261116/target-and-proof.tar')
    assert record['archive_sha'] == '56aca2b2488b99a302f0e806724bf58a2677805a9413f8ca9152ac2caf9d09f3'
    record['status'] = 'READY' if phase == 'before' else 'QUIET'
finally:
    (evidence / ('profile-' + phase + '.json')).write_text(json.dumps(record, indent=2))
