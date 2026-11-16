#!/usr/bin/env python3
"""Prepare a pinned red pair; run only with a fresh allocation through the common supervisor."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

BASE = 'd395cb1f03f84c6abdfa023a0e3c7f87a7c0dab6'
LIB = 'rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF'
LIB_HASHES = {'librag3db.so': 'b7e9fc75a3d2de04c72680b0309e8bb1df9eaadb5211ebbacf39716c6d0a2101',
              'libvector.rag3db_extension': '1669d6392ec8067fcf92ae2ccc972fd7cd3086629fb82d1f5d254e90d72d7561'}
NAMES = ['a_standard_library_path_does_not_hide_a_bare_project_call_on_the_same_line',
         'a_bare_project_call_before_a_library_path_on_the_same_line_remains_visible']


def digest(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def record(root, name, command, cwd, env):
    (root / (name + '.command.json')).write_text(json.dumps(command))
    with (root / (name + '.stdout')).open('x') as out, (root / (name + '.stderr')).open('x') as err:
        result = subprocess.run(command, cwd=cwd, env=env, stdout=out, stderr=err)
    (root / (name + '.result.json')).write_text(json.dumps({'rc': result.returncode}))
    return result.returncode


def git(worktree, *args):
    return subprocess.check_output(['git', '-C', str(worktree), *args], text=True).strip()


def library_environment(config):
    return {'RAG3DB_LIB': LIB, 'RAG3DB_LIBS_DIR': config['store'],
            'RAG3DB_BUFFER_POOL_SIZE': '1073741824'}


def verify(config):
    worktree = Path(config['worktree'])
    assert git(worktree, 'rev-parse', 'HEAD') == config['source']
    assert not git(worktree, 'status', '--porcelain')
    subprocess.run(['git', '-C', str(worktree), 'merge-base', '--is-ancestor', BASE, 'HEAD'], check=True)
    assert all(digest(Path(config['store']) / LIB / name) == sha for name, sha in LIB_HASHES.items())
    assert digest(Path(__file__)) == config['pilot_sha256']
    assert all(digest(worktree / name) == sha for name, sha in config['scripts'].items())


def inside(config, root):
    worktree = Path(config['worktree']); crate = worktree / 'extension/rag3weaver'
    receipt = Path(os.environ['RAG3WEAVER_VALIDATION_RECU'])
    authority = json.loads(receipt.read_text())
    assert authority['etat'] == 'actif'
    env = {key: os.environ[key] for key in ['HOME', 'USER', 'LOGNAME', 'PATH', 'SHELL', 'LANG', 'LC_ALL',
           'TZ', 'CARGO_HOME', 'RUSTUP_HOME', 'DBUS_SESSION_BUS_ADDRESS', 'RAG3WEAVER_VALIDATION_RECU',
           'RAG3WEAVER_VALIDATION_JETON'] if key in os.environ}
    for name in ['tmp', 'config', 'data', 'cache', 'xdg-state', 'state-fixture']:
        (root / name).mkdir()
    env.update(library_environment(config))
    env.update(CARGO_TARGET_DIR=config['target'], CARGO_BUILD_JOBS='8', CARGO_INCREMENTAL='0',
               RUST_TEST_THREADS='1', CARGO_TERM_COLOR='never', POSTE_MEM_MAX='32G',
               RAG3WEAVER_TARGET_REPRENDRE='1', RAG3WEAVER_TARGET_MAX_GO='32',
               RAG3WEAVER_BUILD_MEMORY_HIGH='0', RAG3WEAVER_E2E_FEATURES='rag3db-native,code',
               TMPDIR=str(root / 'tmp'), XDG_CONFIG_HOME=str(root / 'config'),
               XDG_DATA_HOME=str(root / 'data'), XDG_CACHE_HOME=str(root / 'cache'),
               XDG_STATE_HOME=str(root / 'xdg-state'), RAG3WEAVER_STATE_DIR=str(root / 'state-fixture'),
               RAG3DB_ROOT=str(worktree), RAG3WEAVER_BURN_DEVICE='cpu')
    assert not any(key in env for key in ['RAG3WEAVER_PG', 'RAG3WEAVER_EMBED_SERVICE', 'RAG3WEAVER_MOTEUR_ANCIEN'])
    command = [sys.executable, str(crate / 'scripts/confiner_validation.py'), 'verifier']
    assert record(root, 'authority-check', command, crate, env) == 0
    (root / 'authority-path.json').write_text(json.dumps({'path': str(receipt), 'deadline': authority['echeance']}))
    (root / 'profile.json').write_text(json.dumps({'keys': sorted(env), 'library': library_environment(config),
        'features': env['RAG3WEAVER_E2E_FEATURES'], 'witnesses': NAMES, 'worktree': str(worktree)}))
    verify(config)
    rc = record(root, 'red-pair', ['bash', './run_e2e.sh', '--test', 'e2e_usages', 'on_the_same_line'], crate, env)
    # The common supervisor retains the actual nonzero outcome. No expected red is promoted to green.
    return rc


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('action', choices=['prepare', 'run', 'inside'])
    p.add_argument('--worktree', type=Path)
    p.add_argument('--source')
    p.add_argument('--proofs', type=Path, required=True)
    p.add_argument('--allocation')
    args = p.parse_args(); root = args.proofs
    if args.action == 'prepare':
        assert os.uname().nodename.lower() == 'luciepc'
        worktree = args.worktree.resolve(strict=True)
        assert not root.exists() and not root.is_symlink()
        root.mkdir(mode=0o700, parents=True)
        config = {'source': args.source, 'base': BASE, 'worktree': str(worktree),
                  'target': str(Path.home() / '.cache/rag3weaver-build/target-import-pointe-pile'),
                  'store': str(Path.home() / '.cache/rag3db-libs'), 'budget': 1800,
                  'pilot_sha256': digest(__file__), 'scripts': {name: digest(worktree / name) for name in
                  ['extension/rag3weaver/scripts/confiner_validation.py', 'extension/rag3weaver/run_e2e.sh',
                   'extension/rag3weaver/scripts/lib_du_magasin.sh', 'extension/rag3weaver/scripts/cargo_garde.py']}}
        (root / 'prepared.json').write_text(json.dumps(config, indent=2))
        verify(config)
        crate = worktree / 'extension/rag3weaver'
        env = {key: os.environ[key] for key in ['HOME', 'USER', 'PATH', 'LANG'] if key in os.environ}
        (root / 'resolver-cache').mkdir()
        env.update(library_environment(config), XDG_CACHE_HOME=str(root / 'resolver-cache'))
        command = ['bash', '-c', '. "$1"; exporter_la_lib || exit $?; printf "%s\\n" "$RAG3DB_LIBRARY_DIR" "$RAG3DB_INCLUDE_DIR" "$RAG3DB_SHARED" "$LD_LIBRARY_PATH"',
                   'real-library-resolver', str(crate / 'scripts/lib_du_magasin.sh')]
        assert record(root, 'library-resolver', command, crate, env) == 0
        expected = str(Path(config['store']) / LIB)
        assert (root / 'library-resolver.stdout').read_text().splitlines() == [expected, expected, '1', expected]
        assert not list((root / 'resolver-cache').iterdir())
        required = ['bash', 'tools/poste/required_suites.sh', BASE, 'HEAD']
        assert record(root, 'required-suites', required, worktree, env) == 0
        assert 'e2e_code_sync' in (root / 'required-suites.stdout').read_text()
        print('PREPARED; no Cargo, authority, service or database opened')
        return 0
    config = json.loads((root / 'prepared.json').read_text())
    verify(config)
    if args.action == 'inside':
        return inside(config, root)
    assert args.allocation, 'fresh allocation required'
    assert 'RAG3WEAVER_VALIDATION_RECU' not in os.environ
    (root / 'allocation.json').open('x').write(json.dumps({'reference': args.allocation}))
    worktree = Path(config['worktree']); common = worktree / 'extension/rag3weaver/scripts/confiner_validation.py'
    env = os.environ.copy(); env.update(POSTE_MEM_MAX='32G', RAG3WEAVER_BUILD_MEMORY_HIGH='0')
    command = [sys.executable, str(common), 'lancer', '--delai', '1800', '--preuves', str(root / 'scope'),
               '--', sys.executable, str(Path(__file__).resolve()), 'inside', '--proofs', str(root)]
    rc = record(root, 'common', command, worktree, env)
    stdout = (root / 'red-pair.stdout').read_text(errors='replace') if (root / 'red-pair.stdout').exists() else ''
    stderr = (root / 'red-pair.stderr').read_text(errors='replace') if (root / 'red-pair.stderr').exists() else ''
    text = stdout + stderr
    expected = (rc != 0 and all(('e2e_usages::' + name + ' ... FAILED') in text for name in NAMES)
                and '0 passed; 2 failed; 0 ignored' in text
                and text.count('the project call must remain visible') == 2)
    scope = json.loads((root / 'scope/verdict.json').read_text())
    expected = expected and scope['nettoye'] and not scope['restes']
    (root / 'classification.json').write_text(json.dumps({'expected_red_pair': expected,
        'common_rc': rc, 'scope_verdict': scope['verdict'], 'global_pass': False}))
    print('EXPECTED RED PAIR' if expected else 'UNEXPECTED RED; STOP AND DIAGNOSE')
    return 1


if __name__ == '__main__':
    sys.exit(main())
