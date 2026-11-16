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

APP = 'bd76f557928b5078f98a235f1cb0b5f78e7360d8'
BASE = '68590270e21d00285b29800bd9cdecf0cb921461'
R = Path.home() / '.cache/rag3weaver-build'
W = Path.home() / 'git_workspaces/rag3db-cx-ed-stable-tie-identity-r2'
C = W / 'extension/rag3weaver'
Q = R / 'cx-ed-ties-controller-20261116-v3'
P = R / 'cx/ed-ties-59ac-r2-v3'
S = R / 'cx-ed-ties-confinement-20261116-v3'
T = R / 'target-cx-ed-stable-tie-identity'
ORIGINAL_END = 1180927.964331921
ORIGINAL_START = ORIGINAL_END - 7200
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
    assert (T / '.arbre-du-target').read_text().strip() == str(C)
    previous = R / 'cx/ed-ties-59ac-r2-v2'
    old_authority = json.loads((previous / 'authority-path.json').read_text())
    assert old_authority['deadline'] <= ORIGINAL_END
    assert json.loads(Path(old_authority['path']).read_text())['etat'] == 'termine'
    old_scope = json.loads((Path(old_authority['path']).parent / 'verdict.json').read_text())
    assert old_scope['verdict'] == 'ROUGE' and not old_scope['restes'] and old_scope['nettoye']
    assert json.loads((previous / 'configuration.json').read_text())['app'] == APP
    assert 'builtin-templates' in json.loads((previous / 'first-red.json').read_text())['reason']
    assert '== lib' in (previous / 'official/resume.log').read_text()
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
                  pass_sha256=PIN, controllers=files, budget=7200, library_store=str(store),
                  original_start=ORIGINAL_START, original_end=ORIGINAL_END,
                  target_started_absent=False, target_same_owner=True, poste_sha256=digest(C / 'scripts/poste'),
                  previous_red_sha256=digest(previous / 'first-red.json'),
                  previous_configuration_sha256=digest(previous / 'configuration.json'))
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
    env.update(POSTE_MEM_MAX='32G', RAG3WEAVER_BUILD_MEMORY_HIGH='0')
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
        assert result.returncode == 0 and scope['verdict'] == 'VERT' and scope['code'] == 0
        assert not scope['restes'] and scope['nettoye'] and not Path(scope['prive']).exists()
        assert json.loads((P / 'driver-verdict.json').read_text())['verdict'] == 'VERT'
        import time
        for name, sha in LIB_HASHES.items():
            assert digest(store / LIB / name) == sha
        assert time.monotonic() < receipt['deadline'] <= ORIGINAL_END
        assert time.monotonic() - ORIGINAL_START < 7200
        record.update(verdict='VERT', scope=scope, deadline=receipt['deadline'])
    except BaseException as error:
        record['error'] = repr(error)
    if P.exists():
        (P / 'composed-verdict.json').write_text(json.dumps(record, indent=2))
    print(json.dumps(record))
    sys.exit(0 if record['verdict'] == 'VERT' else 1)

if __name__ == '__main__':
    main()
