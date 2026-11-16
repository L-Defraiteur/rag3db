#!/usr/bin/env python3
"""Prepare without Cargo; run only after a fresh C7 allocation."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import stat
import subprocess
import sys
from cache_contract import capture_cache_contract, verify_cache_contract, publish_composed

APP = '9576079d62c3f46bdf241cd19c9ab30e9218333e'
BASE = 'd395cb1f03f84c6abdfa023a0e3c7f87a7c0dab6'
R = Path.home() / '.cache/rag3weaver-build'
W = Path.home() / 'git_workspaces/rag3db-cx-recherche-fts-existing-union-r1'
C = W / 'extension/rag3weaver'
Q = R / 'cx-control-recherche-fts-existing-canon-v2'
P = R / 'cx/recherche-fts-existing-canon-v1'
S = R / 'cx-scope-recherche-fts-existing-canon-v1'
T = R / 'cx-target-recherche-fts-counts-freshness'
BUDGET = 6000
PREVIOUS_W = R / 'cx-wt-recherche-fts-existing-green-source'
PIN = '2150fcd847b1df7de04908d357a0c78abc05e50ce599e2793ff08ab48108825d'
LIB = 'rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF'
LIB_HASHES = {
    'librag3db.so': 'b7e9fc75a3d2de04c72680b0309e8bb1df9eaadb5211ebbacf39716c6d0a2101',
    'libvector.rag3db_extension': '1669d6392ec8067fcf92ae2ccc972fd7cd3086629fb82d1f5d254e90d72d7561',
}

def digest(path):
    with Path(path).open('rb') as f:
        return hashlib.file_digest(f, 'sha256').hexdigest()

def git(root, *args):
    return subprocess.check_output(['git', '-C', str(root), *args], text=True).strip()

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--pilot', required=True)
    parser.add_argument('--prepare-only', action='store_true')
    parser.add_argument('--allocation', help='Fresh C7 allocation reference; never an old GO')
    args = parser.parse_args()
    assert os.uname().nodename.lower() == 'luciepc'
    control = Path(__file__).resolve().parents[4]
    for root, sha in [(W, APP), (control, args.pilot)]:
        assert git(root, 'rev-parse', 'HEAD') == sha and not git(root, 'status', '--porcelain')
        for name in ['rebase-merge', 'rebase-apply']:
            assert not Path(git(root, 'rev-parse', '--path-format=absolute', '--git-path', name)).exists()
    cp = C / 'codeparsers'
    expected = git(W, 'ls-tree', APP, 'extension/rag3weaver/codeparsers').split()[2]
    assert (cp / 'Cargo.toml').is_file() and git(cp, 'rev-parse', 'HEAD') == expected
    assert not git(cp, 'status', '--porcelain')
    assert not any(x.exists() or x.is_symlink() for x in [P, S]), 'fresh proof/scope only'
    assert T.is_dir() and T.resolve() == T and T.stat().st_uid == os.getuid()
    assert (T / '.arbre-du-target').is_file() and not (T / '.arbre-du-target').is_symlink()
    assert (T / '.arbre-du-target').read_text().strip() == str(PREVIOUS_W / 'extension/rag3weaver')
    previous = R / 'cx-archive-recherche-fts-existing-green-v1'
    assert git(PREVIOUS_W, 'rev-parse', 'HEAD') == '04634cfed667711192efd313ff1418ef8dfd5c26'
    assert not git(PREVIOUS_W, 'status', '--porcelain')
    assert json.loads((previous / 'supervisor/autorite.json').read_text())['etat'] == 'termine'
    old_scope = json.loads((previous / 'supervisor/verdict.json').read_text())
    assert old_scope['verdict'] == 'VERT' and not old_scope['restes'] and old_scope['nettoye']
    ledger = json.loads((previous / 'driver/ledger.json').read_text())
    assert digest(ledger['artifact']['executable']) == ledger['elf_sha256']
    assert digest(previous / 'manifest.json') == 'df97717f2c944ccadca3246492a076db98a81d08886387d132d6bf526a38d2c9'
    previous_authority = R / 'cx-proofs-recherche-fts-existing-green-v1-supervisor/validation-akvadari/autorite.json'
    contract_config=dict(target=str(T),previous_worktree=str(PREVIOUS_W),previous_archive=str(previous),previous_authority=str(previous_authority))
    previous_contract=capture_cache_contract(contract_config,git,digest)
    verify_cache_contract(previous_contract,previous_contract)
    previous_contract.pop('observed',None)
    cache = R / 'passe_epinglee.sh'
    assert digest(cache) == PIN and stat.S_IMODE(cache.stat().st_mode) == 0o755
    source_pin = subprocess.check_output(['git', '-C', str(W), 'show', BASE + ':tools/poste/passe_epinglee.sh'])
    assert hashlib.sha256(source_pin).hexdigest() == PIN
    store = R.parent / 'rag3db-libs'
    assert store.resolve() == store and store.stat().st_uid == os.getuid()
    libroot = store / LIB
    for name, sha in LIB_HASHES.items():
        assert digest(libroot / name) == sha
    Q.mkdir(mode=0o700, exist_ok=True)
    assert Q.resolve() == Q and Q.stat().st_uid == os.getuid()
    files = {}
    for name, source in [
        ('driver.py', Path(__file__).with_name('driver.py')),
        ('admit.py', Path(__file__).with_name('admit.py')),
        ('cache_contract.py', Path(__file__).with_name('cache_contract.py')),
        ('confiner_validation.py', C / 'scripts/confiner_validation.py'),
    ]:
        dest = Q / name
        if dest.exists():
            assert not dest.is_symlink() and digest(dest) == digest(source)
        else:
            shutil.copy2(source, dest)
        files[name] = digest(dest)
    config = dict(app=APP, pilot=args.pilot, base=BASE, control=str(control), worktree=str(W),
                  target=str(T), proofs=str(P), scope=str(S), lib=LIB, lib_hashes=LIB_HASHES,
                  pass_sha256=PIN, controllers=files, budget=BUDGET, library_store=str(store),
                  previous_worktree=str(PREVIOUS_W), previous_archive=str(previous), previous_contract=previous_contract, previous_authority=str(previous_authority),
                  target_started_absent=False, target_same_owner=False, poste_sha256=digest(C / 'scripts/poste'),
                  previous_manifest_sha256=digest(previous / 'manifest.json'))
    path = Q / 'prepared.json'
    if path.exists():
        assert json.loads(path.read_text()) == config
    else:
        path.write_text(json.dumps(config, indent=2) + '\n')
    if args.prepare_only:
        print(json.dumps(dict(prepared=str(path), cargo_executed=False, authority_created=False)))
        return
    assert args.allocation, 'fresh C7 allocation required before entering poste'
    assert 'RAG3WEAVER_VALIDATION_RECU' not in os.environ
    env = os.environ.copy()
    env.pop('RAG3WEAVER_MOTEUR_ANCIEN', None)
    env.pop('POSTE_CPU_QUOTA', None)
    env.update(POSTE_MEM_MAX='32G', POSTE_SWAP_MAX='0', CARGO_BUILD_JOBS='8', RUST_TEST_THREADS='8', RAG3WEAVER_BUILD_MEMORY_HIGH='0')
    runtime = Path('/run/user') / str(os.getuid())
    bus = runtime / 'bus'
    assert runtime.resolve() == runtime and bus.resolve() == bus
    before = [(p, p.lstat()) for p in [runtime, bus]]
    assert stat.S_ISDIR(before[0][1].st_mode) and stat.S_ISSOCK(before[1][1].st_mode)
    assert all(s.st_uid == os.getuid() for _, s in before)
    env.update(XDG_RUNTIME_DIR=str(runtime), DBUS_SESSION_BUS_ADDRESS='unix:path=' + str(bus))
    subprocess.run(['busctl', '--user', '--timeout=5', 'call', 'org.freedesktop.DBus',
                    '/org/freedesktop/DBus', 'org.freedesktop.DBus.Peer', 'Ping'], env=env, check=True)
    for path, old in before:
        new = path.lstat()
        assert (old.st_dev, old.st_ino, old.st_mode, old.st_uid) == (new.st_dev, new.st_ino, new.st_mode, new.st_uid)
    os.chdir('/var/tmp')
    (Q / 'allocation.json').write_text(json.dumps(dict(reference=args.allocation)))
    command = [str(C / 'scripts/poste'), 'lourd', sys.executable, str(Q / 'admit.py')]
    result = subprocess.run(command, env=env)
    record = dict(verdict='ROUGE', rc=result.returncode)
    try:
        receipt = json.loads((P / 'authority-path.json').read_text())
        scope = json.loads((Path(receipt['path']).parent / 'verdict.json').read_text())
        final_authority = json.loads(Path(receipt['path']).read_text())
        assert final_authority['etat'] == 'termine' and final_authority['echeance'] == receipt['deadline']
        assert result.returncode == 0 and scope['verdict'] == 'VERT' and scope['code'] == 0
        assert not scope['restes'] and scope['nettoye'] and not Path(scope['prive']).exists()
        assert json.loads((P / 'driver-verdict.json').read_text())['verdict'] == 'VERT'
        import time
        for name, sha in LIB_HASHES.items():
            assert digest(store / LIB / name) == sha
        remaining = json.loads((Q / 'remaining-after-poste.json').read_text())
        assert time.monotonic() < receipt['deadline'] <= remaining['original_end']
        assert time.monotonic() - remaining['original_start'] < BUDGET
        record.update(verdict='VERT', scope=scope, deadline=receipt['deadline'])
    except BaseException as error:
        record['error'] = repr(error)
    if P.exists():
        record = publish_composed(P / 'composed-verdict.json', record, record.get('deadline', 0))
    print(json.dumps(record))
    sys.exit(0 if record['verdict'] == 'VERT' else 1)

if __name__ == '__main__':
    main()
