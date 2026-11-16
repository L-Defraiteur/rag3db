#!/usr/bin/env python3
"""Préparer les unités et usages Rust ; aucune autorité ne naît de prepare."""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess
import sys

PREVIOUS = Path(__file__).resolve().parents[1] / 'rust-red/pilot.py'
spec = importlib.util.spec_from_file_location('rust_pair_pilot', PREVIOUS)
base = importlib.util.module_from_spec(spec)
spec.loader.exec_module(base)
PARSER = '34c74e41b2e46570cdbe7d3e6d4218852a1abe1d'
OWNER = '/home/lucied/git_workspaces/cx-codeparsers-language-corpus/extension/rag3weaver/codeparsers'
ELF_SHA = 'bff1cafb72e8c6662d2d595457b0ca6da4718686ae64b91ae1bde7a4e95565e2'
REQUIRED = base.NAMES + ['a_path_into_the_standard_library_is_not_a_project_homonym',
    'a_rust_call_named_like_a_javascript_keyword_is_a_reference']


def verify(config):
    base.verify(config)
    parser = Path(config['worktree']) / 'extension/rag3weaver/codeparsers'
    assert base.git(parser, 'rev-parse', 'HEAD') == PARSER
    assert not base.git(parser, 'status', '--porcelain')
    assert base.digest(parser / 'Cargo.lock') == config['parser_lock_sha256']
    assert base.digest(Path(__file__)) == config['followup_sha256']


def check_previous(config):
    root = Path(config['previous_corpus_proofs'])
    assert json.loads((root / 'classification.json').read_text())['campaign_completed'] is True
    paths = list((root / 'scope').glob('validation-*/verdict.json'))
    assert len(paths) == 1
    scope = json.loads(paths[0].read_text())
    authority = json.loads(paths[0].with_name('autorite.json').read_text())
    assert scope['verdict'] == 'VERT' and scope['code'] == 0 and scope['erreur'] is None
    assert scope['restes'] == [] and scope['nettoye'] is True and authority['etat'] == 'termine'
    assert not Path(scope['prive']).exists()
    assert base.digest(root / 'bin/language_corpus_probe') == ELF_SHA


def inside(config, proofs):
    verify(config)
    check_previous(config)
    worktree = Path(config['worktree'])
    crate = worktree / 'extension/rag3weaver'
    env = {key: os.environ[key] for key in ['HOME', 'USER', 'LOGNAME', 'PATH', 'SHELL',
        'LANG', 'LC_ALL', 'TZ', 'CARGO_HOME', 'RUSTUP_HOME', 'DBUS_SESSION_BUS_ADDRESS',
        'RAG3WEAVER_VALIDATION_RECU', 'RAG3WEAVER_VALIDATION_JETON'] if key in os.environ}
    for name in ['tmp', 'config', 'data', 'cache', 'xdg-state', 'state-fixture']:
        (proofs / name).mkdir(mode=0o700)
    env.update(base.library_environment(config))
    env.update(CARGO_TARGET_DIR=config['target'], CARGO_BUILD_JOBS='8', CARGO_INCREMENTAL='0',
        RAG3WEAVER_TARGET_REPRENDRE='1', RAG3WEAVER_TARGET_MAX_GO='32', RUST_TEST_THREADS='1',
        CARGO_TERM_COLOR='never', RAG3WEAVER_E2E_FEATURES='rag3db-native,code',
        TMPDIR=str(proofs / 'tmp'), XDG_CONFIG_HOME=str(proofs / 'config'),
        XDG_DATA_HOME=str(proofs / 'data'), XDG_CACHE_HOME=str(proofs / 'cache'),
        XDG_STATE_HOME=str(proofs / 'xdg-state'), RAG3WEAVER_STATE_DIR=str(proofs / 'state-fixture'),
        RAG3DB_ROOT=str(worktree), RAG3WEAVER_BURN_DEVICE='cpu')
    assert base.record(proofs, 'authority-check', [sys.executable, str(crate / 'scripts/confiner_validation.py'), 'verifier'], crate, env) == 0
    base.cache_quiet(config, proofs)
    marker = Path(config['target']) / '.arbre-du-target'
    assert marker.read_text().strip() == OWNER
    # Le SDK réel est exporté par la primitive existante, jamais reconstitué.
    resolver = ['bash', '-c', '. "$1"; exporter_la_lib || exit $?; shift; exec "$@"',
        'real-library-resolver', str(crate / 'scripts/lib_du_magasin.sh')]
    commands = [
        ('stop-words', [sys.executable, str(crate / 'scripts/cargo_garde.py'), 'test', '--locked',
            '--manifest-path', str(crate / 'codeparsers/Cargo.toml'), '--test', 'rust_stop_words', '-j8']),
        ('reference-path', resolver + [sys.executable, str(crate / 'scripts/cargo_garde.py'), 'test', '--locked',
            '--manifest-path', str(crate / 'Cargo.toml'), '--features', 'rag3db-native,code', '--lib',
            '-j8', 'code::reference_path_tests', '--', '--nocapture']),
        ('usages', ['bash', './run_e2e.sh', '--test', 'e2e_usages']),
    ]
    for name, command in commands:
        rc = base.record(proofs, name, command, crate, env)
        if rc != 0:
            return rc
        text = (proofs / (name + '.stdout')).read_text()
        summaries = re.findall(r'^test result: (.+)$', text, re.M)
        assert summaries and all('FAILED' not in row for row in summaries)
        last = summaries[-1]
        assert re.match(r'ok\. [1-9][0-9]* passed; 0 failed; 0 ignored;', last)
        if name in ('stop-words', 'reference-path'):
            assert last.startswith('ok. 1 passed;')
        if name == 'usages':
            assert all(text.count('test e2e_usages::' + test + ' ...') == 1 for test in REQUIRED)
        verify(config)
    return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['prepare', 'run', 'inside'])
    parser.add_argument('--proofs', type=Path, required=True)
    parser.add_argument('--worktree', type=Path)
    parser.add_argument('--source')
    parser.add_argument('--allocation')
    args = parser.parse_args()
    proofs = args.proofs.absolute()
    if args.action == 'prepare':
        assert os.uname().nodename.lower() == 'luciepc'
        worktree = args.worktree.resolve(strict=True)
        files = ['extension/rag3weaver/scripts/' + name for name in
            ['confiner_validation.py', 'poste', 'cargo_garde.py', 'garde_target.sh', 'lib_du_magasin.sh']]
        files += ['extension/rag3weaver/run_e2e.sh',
            'docs/codeparsers/16-11-2026/rust-red/pilot.py',
            'docs/codeparsers/16-11-2026/rust-followup/pilot.py']
        config = {'worktree': str(worktree), 'source': args.source, 'base': base.BASE,
            'target': str(Path.home() / '.cache/rag3weaver-build/target-import-pointe-pile'),
            'store': str(Path.home() / '.cache/rag3db-libs'), 'budget': 900,
            'previous_owner': OWNER,
            'previous_corpus_proofs': str(Path.home() / '.cache/codeparsers-notes/language-corpus-02b'),
            'pilot_sha256': base.digest(PREVIOUS), 'followup_sha256': base.digest(__file__),
            'parser_lock_sha256': base.digest(worktree / 'extension/rag3weaver/codeparsers/Cargo.lock'),
            'scripts': {name: base.digest(worktree / name) for name in files}}
        verify(config)
        check_previous(config)
        target = Path(config['target'])
        assert target.is_dir() and not target.is_symlink()
        assert (target / '.arbre-du-target').read_text().strip() == OWNER
        proofs.mkdir(mode=0o700, parents=True, exist_ok=False)
        (proofs / 'prepared.json').write_text(json.dumps(config, indent=2))
        print('PREPARED; no Cargo, service, database or authority started')
        return 0
    config = json.loads((proofs / 'prepared.json').read_text())
    verify(config)
    if args.action == 'inside':
        return inside(config, proofs)
    assert args.allocation and 'RAG3WEAVER_VALIDATION_RECU' not in os.environ
    with (proofs / 'allocation.json').open('x') as stream:
        stream.write(json.dumps({'reference': args.allocation}))
    os.chdir(proofs)
    common = Path(config['worktree']) / 'extension/rag3weaver/scripts/confiner_validation.py'
    env = os.environ.copy()
    env.update(POSTE_MEM_MAX='32G', RAG3WEAVER_BUILD_MEMORY_HIGH='0')
    command = [sys.executable, str(common), 'lancer', '--delai', str(config['budget']),
        '--preuves', str(proofs / 'scope'), '--', sys.executable, '-B', str(Path(__file__).resolve()),
        'inside', '--proofs', str(proofs)]
    rc = base.record(proofs, 'common', command, proofs, env)
    paths = list((proofs / 'scope').glob('validation-*/verdict.json'))
    assert len(paths) == 1
    verdict = json.loads(paths[0].read_text())
    authority = json.loads(paths[0].with_name('autorite.json').read_text())
    accepted = (rc == 0 and verdict['code'] == 0 and verdict['verdict'] == 'VERT'
        and verdict['erreur'] is None and verdict['restes'] == [] and verdict['nettoye'] is True
        and authority['etat'] == 'termine' and not Path(verdict['prive']).exists())
    (proofs / 'classification.json').write_text(json.dumps({'followup_units_and_usages': accepted,
        'common_rc': rc, 'historical_probe_run': False, 'global_pass': False}, indent=2))
    return 0 if accepted else 1


if __name__ == '__main__':
    sys.exit(main())
