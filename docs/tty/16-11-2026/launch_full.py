"""Prepare immutable provenance; launch only with a fresh allocation; finalize after Common."""
import argparse
import json
import math
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time
from strict_target_guard import digest, snapshot, scan, finish

BASE = Path.home() / '.cache/rag3weaver-build'
W = Path.home() / 'git_workspaces/cx-tty-english-aliases'
T = BASE / 'target-cx-tty-english-aliases-2'
Q = BASE / 'cx-tty-aliases-controller-v3'
P = BASE / 'cx-tty-english-aliases-full-v3'
S = BASE / 'cx-tty-aliases-common-v3'


def git(*args):
    return subprocess.check_output(['git', '-C', str(W), *args], text=True).strip()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--source', required=True)
    parser.add_argument('--prepare-only', action='store_true')
    parser.add_argument('--allocation')
    args = parser.parse_args()
    assert git('rev-parse', 'HEAD') == args.source and not git('status', '--porcelain')
    assert not any(p.exists() or p.is_symlink() for p in [P, S])
    if args.prepare_only:
        assert not Q.exists() and not Q.is_symlink()
        Q.mkdir(mode=0o700)
        old = BASE / 'validations/validation-6tdyny5z'
        authority = json.loads((old / 'autorite.json').read_text())
        verdict = json.loads((old / 'verdict.json').read_text())
        assert authority['etat'] == 'termine' and verdict['verdict'] == 'VERT' and verdict['code'] == 0
        assert verdict['restes'] == [] and verdict['nettoye']
        assert (BASE / 'cx-tty-english-aliases-green-v1/source.txt').read_text().strip() == '0c4ab173d4a3e418b2c5df1253b5090a4d97911e'
        creation = (BASE / 'cx-tty-english-aliases-go2-v1/target-initial.txt').read_text().split()
        st = T.stat()
        assert [str(st.st_uid), str(st.st_dev), str(st.st_ino), oct(st.st_mode & 0o777)[2:]] == creation
        quiet = scan('/proc', [W, T])
        (Q / 'quiet-prepare.json').write_text(json.dumps(quiet, indent=2))
        assert quiet['found'] == []
        ledger = snapshot(T)
        assert snapshot(T) == ledger
        (Q / 'target-ledger.json').write_text(json.dumps(ledger, indent=2))
        (Q / 'elf').mkdir()
        for p in (T / 'debug/deps').glob('rag3weaver_tty-*'):
            if p.is_file() and not p.suffix and os.access(p, os.X_OK):
                shutil.copy2(p, Q / 'elf' / p.name)
                assert digest(p) == digest(Q / 'elf' / p.name)
        for name in ['autorite.json', 'verdict.json', 'recette.json', 'enfants.json']:
            shutil.copy2(old / name, Q / ('previous-' + name))
        sources = {name: W / 'docs/tty/16-11-2026' / name for name in [
            'check-english-aliases-full.sh', 'check_target_profile.py', 'strict_target_guard.py',
            'run_private_harness.py', 'admit_full.py']}
        sources['confiner_validation.py'] = W / 'extension/rag3weaver/scripts/confiner_validation.py'
        hashes = {}
        for name, path in sources.items():
            shutil.copy2(path, Q / name)
            hashes[name] = digest(Q / name)
            assert hashes[name] == digest(path)
        config = dict(source=args.source, worktree=str(W), target=str(T), proofs=str(P), scope=str(S),
                      budget=3600, controllers=hashes, ledger_sha=digest(Q / 'target-ledger.json'),
                      creation_identity=creation, capture='late stable capture anchored to recorded GO2 root identity',
                      previous_source='0c4ab173d4a3e418b2c5df1253b5090a4d97911e')
        (Q / 'prepared.json').write_text(json.dumps(config, indent=2))
        print('PREPARED', Q)
        return
    assert args.allocation and (Q / 'prepared.json').is_file()
    config = json.loads((Q / 'prepared.json').read_text())
    assert config['source'] == args.source
    for name, sha in config['controllers'].items():
        assert digest(Q / name) == sha
    assert digest(Q / 'target-ledger.json') == config['ledger_sha']
    start = time.monotonic()
    (Q / 'allocation.json').open('x').write(json.dumps(dict(reference=args.allocation, start=start, end=start + 3600)))
    env = dict(os.environ, POSTE_MEM_MAX='32G', RAG3WEAVER_BUILD_MEMORY_HIGH='0', TTY_CONTROLLER_DIR=str(Q))
    os.chdir(BASE)
    rc = 2
    result = dict(verdict='ROUGE')
    try:
        command = [str(W / 'extension/rag3weaver/scripts/poste'), 'lourd', sys.executable, str(Q / 'admit_full.py')]
        (Q / 'outer-command.json').write_text(json.dumps(command))
        rc = subprocess.run(command, env=env, timeout=3600).returncode
        receipt = json.loads((P / 'authority-path.json').read_text())
        authority = json.loads(Path(receipt['path']).read_text())
        scope = json.loads((Path(receipt['path']).parent / 'verdict.json').read_text())
        assert rc == 0 and not Path(scope['prive']).exists()
        assert snapshot(T) == json.loads((P / 'target-final.json').read_text())
        for file in ['sdk-sha', 'tty-elf-sha']:
            assert (P / (file + '.txt')).read_text() == (P / (file + '-final.txt')).read_text()
        quiet = scan('/proc', [W, T])
        (P / 'quiet-after-common.json').write_text(json.dumps(quiet, indent=2))
        assert quiet['found'] == []
        assert git('rev-parse', 'HEAD') == args.source and not git('status', '--porcelain')
        driver = 'VERT' if (P / 'verdict.txt').read_text().strip() == 'STAGES_PASSED F8 8/8 et harnais tty privé entier' else 'ROUGE'
        result['verdict'] = finish(authority, scope, driver, time.monotonic(), start + 3600, True)
        result['scope'] = scope
    except Exception as error:
        result['error'] = repr(error)
    result.update(rc=rc, start=start, end=start + 3600, finished=time.monotonic())
    (Q / 'composed-verdict.json').write_text(json.dumps(result, indent=2))
    print(result['verdict'], Q / 'composed-verdict.json')
    if result['verdict'] != 'VERT':
        raise SystemExit(1)


if __name__ == '__main__':
    main()
