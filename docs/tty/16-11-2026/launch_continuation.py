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
from strict_target_guard import digest, snapshot, scan, finish, require_source, require_hashes

BASE = Path.home() / '.cache/rag3weaver-build'
W = Path.home() / 'git_workspaces/cx-tty-english-aliases'
T = BASE / 'target-cx-tty-english-aliases-2'
Q = BASE / 'cx-tty-aliases-continuation-controller-v1'
P = BASE / 'cx-tty-aliases-continuation-v1'
S = BASE / 'cx-tty-aliases-continuation-common-v1'
ORIGINAL_START = 848482.791694931
ORIGINAL_END = 852082.791694931
PRODUCT_SOURCE = '6d886375ea4627c88d28aef918b948a23f628ab1'


def git(*args):
    return subprocess.check_output(['git', '-C', str(W), *args], text=True).strip()


def publish_result(path, result, deadline):
    # Opening the actual destination is part of the deadline, not a later promise.
    with Path(path).open('x') as stream:
        now = time.monotonic()  # fresh AFTER open, immediately before encoding
        result['finished'] = now
        if now >= deadline:
            result.update(verdict='ROUGE', error='deadline exceeded before publication')
        json.dump(result, stream, indent=2)
        stream.flush()
        os.fsync(stream.fileno())
        after = time.monotonic()
        if after >= deadline:
            result.update(verdict='ROUGE', error='deadline exceeded during publication', finished=after)
            stream.seek(0)
            stream.truncate()
            json.dump(result, stream, indent=2)
            stream.flush()
            os.fsync(stream.fileno())
    return result


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--source', required=True)
    parser.add_argument('--prepare-only', action='store_true')
    parser.add_argument('--allocation')
    args = parser.parse_args()
    require_source(W, args.source)
    assert time.monotonic() < ORIGINAL_END
    subprocess.run(['git', '-C', str(W), 'diff', '--exit-code', PRODUCT_SOURCE, '--', 'extension/rag3weaver'], check=True)
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
        latest = BASE / 'cx-tty-aliases-common-v5/validation-93l3ksmi'
        latest_authority = json.loads((latest / 'autorite.json').read_text())
        latest_verdict = json.loads((latest / 'verdict.json').read_text())
        assert latest_authority['etat'] == 'termine'
        assert latest_verdict['verdict'] == 'ROUGE' and latest_verdict['restes'] == [] and latest_verdict['nettoye']
        assert (T / '.arbre-du-target').read_text().strip() == str(W / 'extension/rag3weaver')
        carried = BASE / 'cx-tty-english-aliases-full-v5'
        assert (carried / 'source.txt').read_text().strip() == PRODUCT_SOURCE
        assert (carried / 'f8.rc').read_text().strip() == '0'
        assert (carried / 'f8.log').read_text().splitlines()[-1] == 'CHECK_FEATURES VERT 8/8'
        logs = sorted((carried / 'features').glob('*.log'))
        assert len(logs) == 8
        carried_hashes = {str(p): digest(p) for p in [carried / 'source.txt', carried / 'f8.log', carried / 'f8.rc', *logs]}
        require_hashes(carried_hashes)
        tty_payloads = json.loads((BASE / 'cx-tty-lock-payload-preparation/report.json').read_text())
        assert tty_payloads['status'] == 'TTY_LOCK_PAYLOADS_READY' and tty_payloads['missing_after'] == []
        crates = json.loads((BASE / 'cx-tty-five-crates-preparation/report.json').read_text())
        assert crates['status'] == 'FIVE_VERIFIED' and crates['missing_locked_payloads'] == []
        crate_cache = Path.home() / '.cargo/registry/cache/index.crates.io-1949cf8c6b5b557f'
        crate_hashes = {str(crate_cache / (package['name'] + '-4.3.9.crate')): package['sha256'] for package in crates['packages']}
        assert len(crate_hashes) == 5
        crate_hashes.update({str(crate_cache / (p['name'] + '-' + p['version'] + '.crate')): p['sha256'] for p in tty_payloads['added_or_verified']})
        require_hashes(crate_hashes)
        for name in ['autorite.json', 'verdict.json']:
            shutil.copy2(latest / name, Q / ('latest-red-' + name))
        shutil.copy2(BASE / 'cx-tty-five-crates-preparation/report.json', Q / 'five-crates.json')
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
            'check-english-aliases-continuation.sh', 'check_target_profile.py', 'strict_target_guard.py',
            'run_private_harness.py', 'admit_full.py', 'launch_continuation.py']}
        sources['poste'] = W / 'extension/rag3weaver/scripts/poste'
        sources['confiner_validation.py'] = W / 'extension/rag3weaver/scripts/confiner_validation.py'
        hashes = {}
        for name, path in sources.items():
            shutil.copy2(path, Q / name)
            hashes[name] = digest(Q / name)
            assert hashes[name] == digest(path)
        sdk = Path.home() / '.cache/rag3db-libs/rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF'
        sdk_hashes = {str(sdk / 'librag3db.so'): 'b7e9fc75a3d2de04c72680b0309e8bb1df9eaadb5211ebbacf39716c6d0a2101', str(sdk / 'libvector.rag3db_extension'): '1669d6392ec8067fcf92ae2ccc972fd7cd3086629fb82d1f5d254e90d72d7561'}
        require_hashes(sdk_hashes)
        config = dict(source=args.source, worktree=str(W), target=str(T), proofs=str(P), scope=str(S),
                      budget=3600, original_start=ORIGINAL_START, original_end=ORIGINAL_END, product_source=PRODUCT_SOURCE, carried_f8=carried_hashes, driver='check-english-aliases-continuation.sh', controllers=hashes, sdk_hashes=sdk_hashes, crate_hashes=crate_hashes, ledger_sha=digest(Q / 'target-ledger.json'),
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
    start = ORIGINAL_START
    assert ORIGINAL_END - time.monotonic() > 60
    (Q / 'allocation.json').open('x').write(json.dumps(dict(reference=args.allocation, start=start, end=start + 3600)))
    env = dict(os.environ, POSTE_MEM_MAX='32G', RAG3WEAVER_BUILD_MEMORY_HIGH='0', TTY_CONTROLLER_DIR=str(Q))
    os.chdir(BASE)
    rc = 2
    result = dict(verdict='ROUGE')
    try:
        command = [str(Q / 'poste'), 'lourd', sys.executable, str(Q / 'admit_full.py')]
        (Q / 'outer-command.json').write_text(json.dumps(command))
        rc = subprocess.run(command, env=env, timeout=max(1, math.floor(ORIGINAL_END - time.monotonic()))).returncode
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
        require_source(W, args.source)
        result['physical_sdk_hashes'] = require_hashes(config['sdk_hashes'])
        result['physical_controller_hashes'] = require_hashes({str(Q / name): sha for name, sha in config['controllers'].items()})
        result['carried_f8'] = require_hashes(config['carried_f8'])
        subprocess.run(['git', '-C', str(W), 'diff', '--exit-code', PRODUCT_SOURCE, '--', 'extension/rag3weaver'], check=True)
        driver = 'VERT' if (P / 'verdict.txt').read_text().strip() == 'STAGES_PASSED build et harnais tty privé entier (F8 porté)' else 'ROUGE'
        result['verdict'] = finish(authority, scope, driver, time.monotonic(), start + 3600, True)
        result['scope'] = scope
    except Exception as error:
        result['error'] = repr(error)
    result.update(rc=rc, start=start, end=start + 3600)
    result = publish_result(Q / 'composed-verdict.json', result, start + 3600)
    print(result['verdict'], Q / 'composed-verdict.json')
    if result['verdict'] != 'VERT':
        raise SystemExit(1)


if __name__ == '__main__':
    main()
