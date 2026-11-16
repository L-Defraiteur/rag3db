#!/usr/bin/env python3
"""Préparer un A/B figé puis jouer une phase explicitement allouée, sans budget reconduit."""
import argparse
from collections import Counter
import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess
import sys

TOOL_ROOT = Path(__file__).resolve().parents[4]
PREVIOUS = TOOL_ROOT / 'docs/codeparsers/16-11-2026/rust-red/pilot.py'
spec = importlib.util.spec_from_file_location('qualified_rust_pilot', PREVIOUS)
base = importlib.util.module_from_spec(spec)
spec.loader.exec_module(base)
PRODUCTS = {'baseline': 'd395cb1f03f84c6abdfa023a0e3c7f87a7c0dab6',
            'candidate': '118cc71eea57ebdfa4c79ae01d5d5a0caf181d4b'}
PARSERS = {'baseline': 'bd133850f89d6b12a3ca2b2f6ce238785ed37e3a',
           'candidate': '34c74e41b2e46570cdbe7d3e6d4218852a1abe1d'}
BASELINE_SOURCE = 'fec35198b64b7e9da0496de6b01a7232fa7a8390'
CORPUS_SHA = '0a6c9af21e3666ff95427ecb44ea253d10907cd5619f96b0a96e91e6c1297185'
PROBE = 'extension/rag3weaver/tests/sonde_marque_nom.rs'
LABELS = ['rw', 'engine']
TARGET = Path.home() / '.cache/rag3weaver-build/target-import-pointe-pile'


def check_corpus(pair):
    manifest = Path(pair['manifest'])
    assert base.digest(manifest) == CORPUS_SHA
    data = json.loads(manifest.read_text())
    roots = Path(data['frozen_roots'])
    lines = {}
    for label in LABELS:
        root = roots / label
        assert root.resolve(strict=True) == root and not root.is_symlink()
        selected = {row['relative_path']: row for row in data['files'] if row['corpus'] == label}
        actual = set()
        for path in root.rglob('*'):
            assert not path.is_symlink(), 'corpus symlink refused'
            if path.is_file():
                actual.add(path.relative_to(root).as_posix())
        assert actual == set(selected) and len(actual) == data['counts'][label]
        for relative, row in selected.items():
            path = root / relative
            assert path.stat().st_size == row['size_bytes'] <= 1_000_000
            assert base.digest(path) == row['sha256']
            raw = path.read_bytes()
            raw.decode('utf-8', errors='strict')
            lines[(label, relative)] = {'lines': raw.count(b'\n') + 1, 'bytes': len(raw)}
    return data, lines


def verify(pair, phase):
    assert base.git(TOOL_ROOT, 'rev-parse', 'HEAD') == pair['tool_source']
    assert not base.git(TOOL_ROOT, 'status', '--porcelain')
    assert base.digest(__file__) == pair['driver_sha256']
    assert base.digest(PREVIOUS) == pair['helper_sha256']
    assert base.digest(Path(__file__).with_name('compare.py')) == pair['comparison_sha256']
    assert base.digest(pair['historical_var_file']) == pair['historical_var_sha256']
    assert base.digest(pair['runtime_profile']) == pair['runtime_profile_sha256']
    config = pair['phases'][phase]
    base.verify(config)
    worktree = Path(config['worktree'])
    assert base.git(worktree / 'extension/rag3weaver/codeparsers', 'rev-parse', 'HEAD') == PARSERS[phase]
    assert not base.git(worktree / 'extension/rag3weaver/codeparsers', 'status', '--porcelain')
    subprocess.run(['git', '-C', str(worktree), 'merge-base', '--is-ancestor', PRODUCTS[phase], 'HEAD'], check=True)
    changed = base.git(worktree, 'diff', '--name-only', PRODUCTS[phase], 'HEAD').splitlines()
    assert all(name.startswith('docs/') or name == PROBE for name in changed), 'production drift'
    assert base.digest(worktree / PROBE) == pair['probe_sha256']
    return config


def completed_scope(root):
    root = Path(root)
    scopes = list((root / 'scope').glob('validation-*/verdict.json'))
    assert len(scopes) == 1
    verdict = json.loads(scopes[0].read_text())
    authority = json.loads(scopes[0].with_name('autorite.json').read_text())
    assert verdict['verdict'] == 'VERT' and verdict['code'] == 0 and verdict['erreur'] is None
    assert verdict['restes'] == [] and verdict['nettoye'] is True and authority['etat'] == 'termine'
    assert not Path(verdict['prive']).exists()


def previous_end(pair, phase):
    if phase == 'baseline':
        root = Path.home() / '.cache/codeparsers-notes/rust-followup-118cc71'
        assert json.loads((root / 'classification.json').read_text())['followup_units_and_usages'] is True
        completed_scope(root)
        binaries = json.loads((root / 'executables.json').read_text())
        assert len(binaries) == 3
        assert all(base.digest(row['preserved_path']) == row['sha256'] for row in binaries)
    else:
        root = Path(pair['proofs']) / 'baseline'
        assert json.loads((root / 'classification.json').read_text())['phase_completed'] is True
        completed_scope(root)


def read_edges(path):
    with Path(path).open() as stream:
        return [json.loads(line) for line in stream if line.strip()]


def check_outputs(pair, phase):
    data, source_lines = check_corpus(pair)
    root = Path(pair['proofs']) / phase
    profile = json.loads(Path(pair['runtime_profile']).read_text())
    text = (root / 'probe.stdout').read_text()
    summaries = re.findall(r'^test result: (.+)$', text, re.M)
    assert summaries and summaries[-1].startswith('ok. 1 passed; 0 failed; 0 ignored;')
    assert not any('FAILED' in row for row in summaries)
    assert text.count('test aretes_par_marque_en_tsv ...') == 1
    assert json.loads((root / 'probe.result.json').read_text())['rc'] == 0
    counts = {}
    for n, label in enumerate(LABELS):
        meta = json.loads((root / f'edges-{n}.meta.json').read_text())
        expected = data['counts'][label]
        admission = json.loads((root / f'edges-{n}.admission.json').read_text())
        expected_paths = {row['relative_path'] for row in data['files'] if row['corpus'] == label}
        assert admission['rejected'] == [] and admission['discovered'] == expected
        assert admission['admitted'] == expected and set(admission['admitted_paths']) == expected_paths
        assert all(meta[name] == expected for name in ['discovered', 'admitted', 'analysis_files', 'stored_files'])
        assert meta['analysis_skipped'] == [] and meta['ingest_report']['failed'] == 0
        assert meta['orphan_scopes'] == 0 and meta['actual_build_id'] == base.LIB
        assert meta['strict_profile'] == profile
        rows = read_edges(root / f'edges-{n}.jsonl')
        marks = Counter()
        for row in rows:
            for side in ['caller', 'target']:
                path = row[side + '_path']
                position = source_lines[(label, path)]
                assert 1 <= row[side + '_start_line'] <= row[side + '_end_line'] <= position['lines']
                assert 0 <= row[side + '_start_byte'] <= row[side + '_end_byte'] <= position['bytes']
                assert row[side + '_key'] and row[side + '_uuid']
            assert 1 <= row['line'] <= source_lines[(label, row['caller_path'])]['lines']
            marks[row['resolution']] += 1
        assert dict(marks) == dict(meta['resolution_counts'])
        counts[label] = {'rows': len(rows), 'scopes': meta['scope_count'], 'resolution_counts': dict(marks),
            'jsonl_sha256': base.digest(root / f'edges-{n}.jsonl')}
    return counts


def inside(pair, phase):
    config = verify(pair, phase)
    previous_end(pair, phase)
    check_corpus(pair)
    root = Path(pair['proofs']) / phase
    worktree = Path(config['worktree'])
    crate = worktree / 'extension/rag3weaver'
    env = {key: os.environ[key] for key in ['HOME', 'USER', 'LOGNAME', 'PATH', 'SHELL', 'LANG', 'LC_ALL',
        'TZ', 'CARGO_HOME', 'RUSTUP_HOME', 'DBUS_SESSION_BUS_ADDRESS', 'RAG3WEAVER_VALIDATION_RECU',
        'RAG3WEAVER_VALIDATION_JETON'] if key in os.environ}
    for name in ['tmp', 'config', 'data', 'cache', 'xdg-state', 'state-fixture']:
        (root / name).mkdir(mode=0o700)
    env.update(base.library_environment(config))
    env.update(CARGO_TARGET_DIR=config['target'], CARGO_BUILD_JOBS='8', CARGO_INCREMENTAL='0', RUST_TEST_THREADS='1',
        RAG3WEAVER_TARGET_REPRENDRE='1', RAG3WEAVER_TARGET_MAX_GO='32', CARGO_TERM_COLOR='never',
        RAG3WEAVER_BUILD_MEMORY_HIGH='0', RAG3WEAVER_E2E_FEATURES='rag3db-native,code', RAG3WEAVER_BURN_DEVICE='cpu', RAG3DB_ROOT=str(worktree),
        TMPDIR=str(root / 'tmp'), XDG_CONFIG_HOME=str(root / 'config'), XDG_DATA_HOME=str(root / 'data'),
        XDG_CACHE_HOME=str(root / 'cache'), XDG_STATE_HOME=str(root / 'xdg-state'),
        RAG3WEAVER_STATE_DIR=str(root / 'state-fixture'), SONDE_DIRS=','.join(pair['roots']),
        SONDE_TSV=str(root / 'edges'), RUST_PROBE_PROFILE=pair['runtime_profile'])
    assert base.record(root, 'authority-check', [sys.executable, str(crate / 'scripts/confiner_validation.py'), 'verifier'], crate, env) == 0
    base.cache_quiet(config, root)
    assert (TARGET / '.arbre-du-target').read_text().strip() == config['previous_owner']
    # Même resolver que l'unité native qualifiée ; un seul export, Cargo explicitement verrouillé.
    command = ['bash', '-c', '. "$1"; exporter_la_lib || exit $?; shift; exec "$@"',
        'real-library-resolver', str(crate / 'scripts/lib_du_magasin.sh'), sys.executable,
        str(crate / 'scripts/cargo_garde.py'), 'test', '--locked', '--manifest-path', str(crate / 'Cargo.toml'),
        '--features', 'rag3db-native,code', '--test', 'sonde_marque_nom', '-j8', '--', '--ignored', '--nocapture']
    rc = base.record(root, 'probe', command, crate, env)
    verify(pair, phase)
    check_corpus(pair)
    if rc == 0:
        (root / 'output-check.json').write_text(json.dumps(check_outputs(pair, phase), indent=2))
    return rc


def prepare(args):
    assert os.uname().nodename.lower() == 'luciepc'
    proofs = args.proofs.absolute()
    manifest = TOOL_ROOT / 'docs/codeparsers/16-11-2026/rust-followup/frozen-corpus.json'
    pair = {'proofs': str(proofs), 'manifest': str(manifest), 'tool_source': args.candidate_source,
        'driver_sha256': base.digest(__file__), 'helper_sha256': base.digest(PREVIOUS), 'budget_per_phase': 1800,
        'sdk': base.LIB, 'corpus_manifest_sha256': CORPUS_SHA,
        'comparison_sha256': base.digest(Path(__file__).with_name('compare.py')),
        'historical_var_file': str(Path(__file__).with_name('historical-var-pairs.json')),
        'historical_var_sha256': base.digest(Path(__file__).with_name('historical-var-pairs.json'))}
    data, _ = check_corpus(pair)
    pair['roots'] = [str(Path(data['frozen_roots']) / label) for label in LABELS]
    profile = {'roots': pair['roots'], 'expected_counts': [data['counts'][label] for label in LABELS],
        'expected_build_id': base.LIB, 'corpus_manifest_sha256': CORPUS_SHA}
    pair['runtime_profile'] = str(proofs / 'runtime-profile.json')
    pair['probe_sha256'] = base.digest(args.candidate_worktree / PROBE)
    scripts = ['extension/rag3weaver/scripts/' + name for name in
        ['confiner_validation.py', 'poste', 'cargo_garde.py', 'garde_target.sh', 'lib_du_magasin.sh']]
    scripts += [PROBE, 'extension/rag3weaver/Cargo.lock', 'extension/rag3weaver/Cargo.toml']
    pair['phases'] = {}
    for phase, worktree, source in [('baseline', args.baseline_worktree, BASELINE_SOURCE),
            ('candidate', args.candidate_worktree, args.candidate_source)]:
        worktree = worktree.resolve(strict=True)
        previous_owner = str(args.candidate_worktree.resolve() / 'extension/rag3weaver') if phase == 'baseline' else str(args.baseline_worktree.resolve() / 'extension/rag3weaver')
        pair['phases'][phase] = {'worktree': str(worktree), 'source': source, 'target': str(TARGET),
            'store': str(Path.home() / '.cache/rag3db-libs'), 'budget': 1800, 'previous_owner': previous_owner,
            'pilot_sha256': base.digest(PREVIOUS), 'scripts': {name: base.digest(worktree / name) for name in scripts}}
    for name in scripts:
        if name != PROBE:
            assert pair['phases']['baseline']['scripts'][name] == pair['phases']['candidate']['scripts'][name], 'configuration differs'
    assert (TARGET / '.arbre-du-target').read_text().strip() == pair['phases']['baseline']['previous_owner']
    previous_end(pair, 'baseline')
    proofs.mkdir(mode=0o700, parents=True, exist_ok=False)
    Path(pair['runtime_profile']).write_text(json.dumps(profile, indent=2))
    pair['runtime_profile_sha256'] = base.digest(pair['runtime_profile'])
    for phase in PRODUCTS:
        verify(pair, phase)
        (proofs / phase).mkdir(mode=0o700)
    (proofs / 'prepared.json').write_text(json.dumps(pair, indent=2))
    print('PREPARED; neither Cargo, test nor authority started; two separate future allocations required')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['prepare', 'run', 'inside', 'compare'])
    parser.add_argument('--proofs', type=Path, required=True)
    parser.add_argument('--phase', choices=list(PRODUCTS))
    parser.add_argument('--baseline-worktree', type=Path)
    parser.add_argument('--candidate-worktree', type=Path)
    parser.add_argument('--candidate-source')
    parser.add_argument('--allocation')
    args = parser.parse_args()
    if args.action == 'prepare':
        prepare(args)
        return 0
    pair = json.loads((args.proofs / 'prepared.json').read_text())
    if args.action == 'compare':
        for phase in PRODUCTS:
            verify(pair, phase)
            completed_scope(Path(pair['proofs']) / phase)
        comparison_spec = importlib.util.spec_from_file_location('probe_comparison', Path(__file__).with_name('compare.py'))
        comparison = importlib.util.module_from_spec(comparison_spec)
        comparison_spec.loader.exec_module(comparison)
        print(comparison.compare_pair(pair, read_edges, check_outputs))
        return 0
    if args.phase is None:
        parser.error('--phase is required for run and inside')
    config = verify(pair, args.phase)
    previous_end(pair, args.phase)
    if args.action == 'inside':
        return inside(pair, args.phase)
    assert args.allocation and 'RAG3WEAVER_VALIDATION_RECU' not in os.environ
    root = Path(pair['proofs']) / args.phase
    with (root / 'allocation.json').open('x') as stream:
        stream.write(json.dumps({'reference': args.allocation}))
    os.chdir(root)
    env = os.environ.copy()
    env.update(POSTE_MEM_MAX='32G', POSTE_CPU_QUOTA='500%', RAG3WEAVER_BUILD_MEMORY_HIGH='0')
    command = [sys.executable, str(Path(config['worktree']) / 'extension/rag3weaver/scripts/confiner_validation.py'),
        'lancer', '--delai', str(pair['budget_per_phase']), '--preuves', str(root / 'scope'), '--',
        sys.executable, '-B', str(Path(__file__).resolve()), 'inside', '--proofs', pair['proofs'], '--phase', args.phase]
    rc = base.record(root, 'common', command, root, env)
    accepted, reason = False, None
    try:
        assert rc == 0
        completed_scope(root)
        counts = check_outputs(pair, args.phase)
        accepted = True
    except (AssertionError, OSError, ValueError, KeyError) as error:
        counts, reason = {}, repr(error)
    (root / 'classification.json').write_text(json.dumps({'phase_completed': accepted, 'common_rc': rc,
        'phase': args.phase, 'counts': counts, 'reason': reason, 'global_pass': False}, indent=2))
    return 0 if accepted else 1


if __name__ == '__main__':
    sys.exit(main())
