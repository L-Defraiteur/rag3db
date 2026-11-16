#!/usr/bin/env python3
"""Préparer puis lancer un banc autonome, sous la portée canonique et un GO frais."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time

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


def verify(config):
    worktree = Path(config['worktree'])
    assert git(worktree, 'rev-parse', 'HEAD') == config['source']
    assert not git(worktree, 'status', '--porcelain')
    parser = worktree / 'extension/rag3weaver/codeparsers'
    assert git(parser, 'rev-parse', 'HEAD') == config['parser']
    assert not git(parser, 'status', '--porcelain')
    assert all(digest(worktree / name) == sha for name, sha in config['scripts'].items())
    assert digest(config['manifest']) == config['manifest_sha256']
    assert digest(config['roots']) == config['roots_sha256']
    assert digest(parser / 'Cargo.lock') == config['lock_sha256']


def inside(config, proofs):
    verify(config)
    worktree = Path(config['worktree'])
    crate = worktree / 'extension/rag3weaver'
    env = {key: os.environ[key] for key in ['HOME', 'USER', 'LOGNAME', 'PATH', 'SHELL',
        'LANG', 'LC_ALL', 'TZ', 'CARGO_HOME', 'RUSTUP_HOME', 'DBUS_SESSION_BUS_ADDRESS',
        'RAG3WEAVER_VALIDATION_RECU', 'RAG3WEAVER_VALIDATION_JETON'] if key in os.environ}
    for name in ['tmp', 'config', 'data', 'cache', 'xdg-state', 'state-fixture']:
        (proofs / name).mkdir(mode=0o700)
    env.update(CARGO_TARGET_DIR=config['target'], CARGO_BUILD_JOBS='8', CARGO_INCREMENTAL='0',
        RAG3WEAVER_TARGET_REPRENDRE='1', RAG3WEAVER_TARGET_MAX_GO='32', CARGO_TERM_COLOR='never',
        TMPDIR=str(proofs / 'tmp'), XDG_CONFIG_HOME=str(proofs / 'config'),
        XDG_DATA_HOME=str(proofs / 'data'), XDG_CACHE_HOME=str(proofs / 'cache'),
        XDG_STATE_HOME=str(proofs / 'xdg-state'), RAG3WEAVER_STATE_DIR=str(proofs / 'state-fixture'))
    assert record(proofs, 'authority-check', [sys.executable, str(crate / 'scripts/confiner_validation.py'), 'verifier'], worktree, env) == 0
    cache_quiet(config, proofs)
    target = Path(config['target'])
    assert target.is_dir() and not target.is_symlink()
    assert (target / '.arbre-du-target').read_text().strip() == config['previous_owner']
    test_command = [sys.executable, str(crate / 'scripts/cargo_garde.py'), 'test', '--locked',
        '--manifest-path', str(crate / 'codeparsers/Cargo.toml'), '--example', 'language_corpus_probe',
        '-j8', 'tests::specialized_parser_errors_remain_observations', '--', '--exact']
    rc = record(proofs, 'conversion-test', test_command, worktree, env)
    if rc != 0:
        return rc
    assert '1 passed; 0 failed; 0 ignored;' in (proofs / 'conversion-test.stdout').read_text()
    command = [sys.executable, str(crate / 'scripts/cargo_garde.py'), 'build', '--locked',
        '--manifest-path', str(crate / 'codeparsers/Cargo.toml'), '--example', 'language_corpus_probe', '-j8']
    rc = record(proofs, 'compile', command, worktree, env)
    if rc != 0:
        return rc
    verify(config)
    binary = target / 'debug/examples/language_corpus_probe'
    (proofs / 'binary.json').write_text(json.dumps({'path': str(binary), 'sha256': digest(binary)}))
    command = [sys.executable, '-B', str(Path(__file__).with_name('control_probe.py')), '--binary', str(binary),
        '--manifest', config['manifest'], '--roots', config['roots'], '--proofs', str(proofs / 'campaign'), '--file-seconds', '10']
    rc = record(proofs, 'campaign', command, worktree, env)
    verify(config)
    return rc


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['prepare', 'run', 'inside'])
    parser.add_argument('--proofs', type=Path, required=True)
    parser.add_argument('--worktree', type=Path)
    parser.add_argument('--source')
    parser.add_argument('--manifest', type=Path)
    parser.add_argument('--roots', type=Path)
    parser.add_argument('--allocation')
    args = parser.parse_args()
    proofs = args.proofs.absolute()
    if args.action == 'prepare':
        assert os.uname().nodename.lower() == 'luciepc'
        worktree = args.worktree.resolve(strict=True)
        target = Path.home() / '.cache/rag3weaver-build/target-import-pointe-pile'
        owner = '/home/lucied/git_workspaces/cx-codeparsers-rust-fix/extension/rag3weaver'
        assert target.is_dir() and not target.is_symlink()
        assert (target / '.arbre-du-target').read_text().strip() == owner
        files = ['extension/rag3weaver/scripts/' + name for name in
            ['confiner_validation.py', 'poste', 'cargo_garde.py', 'garde_target.sh']]
        files += ['docs/codeparsers/16-11-2026/language-corpus/' + name for name in ['pilot.py', 'control_probe.py']]
        config = {'worktree': str(worktree), 'source': args.source,
            'parser': '1b35cfb66ae160954981d7dd3a90f5ef88e95b01', 'target': str(target), 'previous_owner': owner,
            'manifest': str(args.manifest.resolve(strict=True)), 'roots': str(args.roots.resolve(strict=True)),
            'budget': 1800, 'scripts': {name: digest(worktree / name) for name in files}}
        config['parser'] = git(worktree / 'extension/rag3weaver/codeparsers', 'rev-parse', '1b35cfb66ae160954981d7dd3a90f5ef88e95b01^{commit}')
        config['manifest_sha256'] = digest(config['manifest'])
        config['roots_sha256'] = digest(config['roots'])
        config['lock_sha256'] = digest(worktree / 'extension/rag3weaver/codeparsers/Cargo.lock')
        verify(config)
        proofs.mkdir(mode=0o700, parents=True, exist_ok=False)
        (proofs / 'prepared.json').write_text(json.dumps(config, indent=2))
        print('PREPARED; no Cargo or authority started')
        return 0
    config = json.loads((proofs / 'prepared.json').read_text())
    verify(config)
    if args.action == 'inside':
        return inside(config, proofs)
    assert args.allocation and 'RAG3WEAVER_VALIDATION_RECU' not in os.environ
    (proofs / 'allocation.json').open('x').write(json.dumps({'reference': args.allocation}))
    common = Path(config['worktree']) / 'extension/rag3weaver/scripts/confiner_validation.py'
    env = os.environ.copy()
    env.update(POSTE_MEM_MAX='32G', RAG3WEAVER_BUILD_MEMORY_HIGH='0')
    command = [sys.executable, str(common), 'lancer', '--delai', str(config['budget']),
        '--preuves', str(proofs / 'scope'), '--', sys.executable, '-B', str(Path(__file__).resolve()),
        'inside', '--proofs', str(proofs)]
    rc = record(proofs, 'common', command, proofs, env)
    matches = list((proofs / 'scope').glob('validation-*/verdict.json'))
    assert len(matches) == 1
    verdict = json.loads(matches[0].read_text())
    authority = json.loads(matches[0].with_name('autorite.json').read_text())
    accepted = (rc == 0 and verdict.get('code') == 0 and verdict.get('verdict') == 'VERT'
        and verdict.get('erreur', 'missing') is None and verdict.get('restes') == []
        and verdict.get('nettoye') is True and authority.get('etat') == 'termine'
        and not Path(verdict['prive']).exists())
    (proofs / 'classification.json').write_text(json.dumps({'campaign_completed': accepted,
        'common_rc': rc, 'coverage_status': 'unjudged', 'global_pass': False}, indent=2))
    return 0 if accepted else 1


if __name__ == '__main__':
    sys.exit(main())
