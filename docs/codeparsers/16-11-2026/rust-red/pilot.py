#!/usr/bin/env python3
"""Prepare a pinned red pair; run only with a fresh allocation through the common supervisor."""
import argparse
import hashlib
import json
import os
import re
from pathlib import Path
import subprocess
import sys
import time

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


def cache_quiet(config, root):
    # Same OS attribution boundary as the reviewed ED pilot, never a user-manager query.
    roots = [Path(config['worktree']), Path(config['target']), Path(config['previous_owner'])]
    ancestors = set(); pid = os.getpid()
    while pid > 1 and pid not in ancestors:
        ancestors.add(pid)
        pid = int((Path('/proc') / str(pid) / 'stat').read_text().rsplit(') ', 1)[1].split()[1])
    found = []; unknown = []
    for proc in Path('/proc').iterdir():
        if not proc.name.isdigit():
            continue
        attributed = False; stamp = None
        try:
            fields = (proc / 'stat').read_text().rsplit(') ', 1)[1].split(); stamp = fields[19]
            if fields[0] == 'Z':
                continue
            for field in ['exe', 'cwd']:
                destination = os.readlink(proc / field)
                if any(destination == str(path) or destination.startswith(str(path) + '/') for path in roots):
                    attributed = True
                    found.append({'pid': proc.name, 'starttime': stamp, 'field': field, 'path': destination})
            for fd in (proc / 'fd').iterdir():
                try:
                    destination = os.readlink(fd)
                except (FileNotFoundError, ProcessLookupError):
                    continue
                if destination.endswith('/scripts/poste') and int(proc.name) in ancestors and proc.stat().st_uid == os.getuid():
                    assert digest(destination) == config['scripts']['extension/rag3weaver/scripts/poste']
                    continue
                if any(destination == str(path) or destination.startswith(str(path) + '/') for path in roots):
                    attributed = True
                    found.append({'pid': proc.name, 'starttime': stamp, 'field': 'fd', 'path': destination})
        except PermissionError:
            unknown.append({'pid': proc.name, 'starttime': stamp, 'attributed': attributed})
            if attributed:
                found.append(unknown[-1])
        except (FileNotFoundError, ProcessLookupError):
            continue
    (root / 'cache-quiet-after-poste.json').write_text(json.dumps({'observed': time.monotonic(),
        'roots': list(map(str, roots)), 'found': found, 'unknown': unknown}, indent=2))
    assert not found, 'attributed process still uses the source or target'


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
    cache_quiet(config, root)
    target = Path(config['target'])
    if config['target_started_absent']:
        assert not target.exists() and not target.is_symlink()
        target.mkdir(mode=0o700)
    else:
        assert target.is_dir() and not target.is_symlink()
        marker = target / '.arbre-du-target'
        assert marker.is_file() and not marker.is_symlink()
        assert marker.read_text().strip() == config['previous_owner']
        read_scope_verdict(Path(config['previous_red_proofs']), 1, 'red')
    verify(config)
    rc = record(root, 'red-pair', ['bash', './run_e2e.sh', '--test', 'e2e_usages', 'on_the_same_line'], crate, env)
    # The common supervisor retains the actual nonzero outcome. No expected red is promoted to green.
    verify(config)
    return rc


def read_scope_verdict(root, common_rc, expected='red'):
    """Read one completed common invocation and the actual native red pair."""
    def require(condition, message):
        if not condition:
            raise ValueError(message)

    def load(path):
        require(path.resolve() == path.absolute() and not path.is_symlink(), 'linked proof artifact')
        try:
            return json.loads(path.read_text())
        except (OSError, ValueError) as error:
            raise ValueError('missing or invalid proof artifact: ' + path.name) from error

    require(expected in ('red', 'green'), 'unknown expected phase')
    matches = list((root / 'scope').glob('validation-*/verdict.json'))
    require(len(matches) == 1, 'expected exactly one common verdict')
    scope_path = matches[0]
    scope = load(scope_path)
    authority_path = scope_path.with_name('autorite.json')
    authority = load(authority_path)
    observed = load(root / 'authority-path.json')
    require(observed.get('path') == str(authority_path), 'authority does not belong to this invocation')
    require(authority.get('etat') == 'termine', 'common authority is not completed')
    require(observed.get('deadline') == authority.get('echeance') and 'deadline' in observed,
            'authority deadline mismatch')
    child_rc = 101 if expected == 'red' else 0
    expected_common_rc = 1 if expected == 'red' else 0
    expected_verdict = 'ROUGE' if expected == 'red' else 'VERT'
    require(common_rc == expected_common_rc and scope.get('code') == child_rc and scope.get('verdict') == expected_verdict,
            'unexpected common or child return code')
    require(scope.get('erreur', 'missing') is None and scope.get('restes') == [] and scope.get('nettoye') is True,
            'supervisor error, remaining descendants or incomplete cleanup')
    private = scope.get('prive')
    require(isinstance(private, str) and Path(private).is_absolute(), 'private scope path missing')
    require(not Path(private).exists() and not Path(private).is_symlink(), 'private scope still exists')
    require(load(root / 'red-pair.result.json').get('rc') == child_rc, 'unexpected native runner code')
    require(load(root / 'red-pair.command.json') ==
            ['bash', './run_e2e.sh', '--test', 'e2e_usages', 'on_the_same_line'], 'unexpected native command')
    stdout = (root / 'red-pair.stdout').read_text(errors='replace')
    stderr = (root / 'red-pair.stderr').read_text(errors='replace')
    text = stdout + stderr
    if expected == 'red':
        failed = [line.strip() for line in stdout.splitlines() if line.strip().startswith('e2e_usages::')]
        require(sorted(failed) == sorted('e2e_usages::' + name for name in NAMES), 'unexpected failed witness names')
        require(len(re.findall(r'test result: FAILED\. 0 passed; 2 failed; 0 ignored;', stdout)) == 1,
                'expected exactly two failed tests and no ignored test')
        require(text.count('the project call must remain visible') == 2 and
                text.count('left: 0') == 2 and text.count('right: 1') == 2,
                'failures do not establish the expected missing project calls')
    else:
        results = re.findall(r'^test result: (.+)$', stdout, re.M)
        require(bool(results) and results[-1].startswith('ok. 2 passed; 0 failed; 0 ignored;'),
                'expected exactly two passing native witnesses and no ignored test')
        require(all(stdout.count('test e2e_usages::' + name + ' ...') == 1 for name in NAMES),
                'green witnesses do not match the requested pair')
        require('test result: FAILED.' not in stdout, 'an earlier failure must not be hidden')
    return scope


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('action', choices=['prepare', 'run', 'inside'])
    p.add_argument('--worktree', type=Path)
    p.add_argument('--source')
    p.add_argument('--proofs', type=Path, required=True)
    p.add_argument('--allocation')
    p.add_argument('--expected', choices=['red', 'green'], default='red')
    p.add_argument('--previous-red-proofs', type=Path)
    args = p.parse_args(); root = args.proofs
    if args.action == 'prepare':
        assert os.uname().nodename.lower() == 'luciepc'
        worktree = args.worktree.resolve(strict=True)
        assert not root.exists() and not root.is_symlink()
        root.mkdir(mode=0o700, parents=True)
        config = {'source': args.source, 'base': BASE, 'worktree': str(worktree),
                  'target': str(Path.home() / '.cache/rag3weaver-build/target-import-pointe-pile'),
                  'store': str(Path.home() / '.cache/rag3db-libs'), 'budget': 1800 if args.expected == 'red' else 900,
                  'expected': args.expected,
                  'pilot_sha256': digest(__file__), 'scripts': {name: digest(worktree / name) for name in
                  ['extension/rag3weaver/scripts/confiner_validation.py', 'extension/rag3weaver/run_e2e.sh',
                   'extension/rag3weaver/scripts/lib_du_magasin.sh', 'extension/rag3weaver/scripts/cargo_garde.py',
                   'extension/rag3weaver/scripts/poste']}}
        target = Path(config['target'])
        assert target.parent.is_dir() and target.parent.resolve() == target.parent
        assert target.parent.stat().st_uid == os.getuid()
        if args.expected == 'red':
            assert not target.exists() and not target.is_symlink(), 'this allocation requires a fresh target'
            config['previous_owner'] = str(worktree / 'extension/rag3weaver')
            config['target_started_absent'] = True
        else:
            previous = args.previous_red_proofs.resolve(strict=True)
            read_scope_verdict(previous, 1, 'red')
            previous_config = json.loads((previous / 'prepared.json').read_text())
            assert str(target) == previous_config['target'] and previous_config['target_started_absent']
            assert previous_config['source'] == '056f552c122353a25c37f2feaf1d4e5006c712f3'
            assert target.is_dir() and not target.is_symlink() and target.stat().st_uid == os.getuid()
            marker = target / '.arbre-du-target'
            assert marker.is_file() and not marker.is_symlink()
            config['previous_owner'] = marker.read_text().strip()
            assert config['previous_owner'] == str(Path(previous_config['worktree']) / 'extension/rag3weaver')
            config['previous_red_proofs'] = str(previous)
            config['target_started_absent'] = False
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
    command = [sys.executable, str(common), 'lancer', '--delai', str(config['budget']), '--preuves', str(root / 'scope'),
               '--', sys.executable, str(Path(__file__).resolve()), 'inside', '--proofs', str(root)]
    os.chdir(root)
    rc = record(root, 'common', command, root, env)
    try:
        scope = read_scope_verdict(root, rc, config['expected'])
        expected, reason = True, None
    except (ValueError, OSError) as error:
        scope, expected, reason = {}, False, str(error)
    (root / 'classification.json').write_text(json.dumps({'expected_' + config['expected'] + '_pair': expected,
        'common_rc': rc, 'scope_verdict': scope.get('verdict', 'NOT_QUALIFIED'),
        'reason': reason, 'global_pass': False}))
    print(('EXPECTED ' + config['expected'].upper() + ' PAIR') if expected else 'UNEXPECTED RESULT; STOP AND DIAGNOSE')
    return 0 if expected and config['expected'] == 'green' else 1


if __name__ == '__main__':
    sys.exit(main())
