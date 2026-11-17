#!/usr/bin/env python3
"""Préparer un A/B figé puis jouer une phase explicitement allouée, sans budget reconduit."""
# Racine d'import minimale : le loader est épinglé LITTÉRALEMENT avant exec_module.
import hashlib as _bootstrap_hashlib
import importlib.util as _bootstrap_importlib
from pathlib import Path as _BootstrapPath
_bootstrap_entry = _BootstrapPath(__file__).absolute()
assert _bootstrap_entry.resolve(strict=True) == _bootstrap_entry
_bootstrap_source = (_bootstrap_entry.parents[4] / 'docs/codeparsers/16-11-2026/rust-probe/pilot.py'
    if tuple(_bootstrap_entry.parts[-5:]) == tuple(_BootstrapPath('docs/codeparsers/16-11-2026/rust-probe/pilot.py').parts) else None)
_bootstrap_loader = (_bootstrap_entry.parents[4] / 'extension/rag3weaver/scripts/publication_loader.py'
    if _bootstrap_source is not None else _bootstrap_entry.parent / 'publication_loader.py')
_bootstrap_loader_bytes = _bootstrap_loader.read_bytes()
assert _bootstrap_hashlib.sha256(_bootstrap_loader_bytes).hexdigest() == '5ee566f69c978f1444bd3b9d685145e2cfc68cc263f9ef48553780c54f593d4b'
_bootstrap_spec = _bootstrap_importlib.spec_from_file_location('verified_publication_loader', _bootstrap_loader)
_bootstrap_module = _bootstrap_importlib.module_from_spec(_bootstrap_spec)
exec(compile(_bootstrap_loader_bytes, str(_bootstrap_loader), 'exec'), _bootstrap_module.__dict__)
_bootstrap_context = _bootstrap_module.verify_entry(_bootstrap_entry, source_entry=_bootstrap_source)

import argparse
from collections import Counter
import importlib.util
import json
import os
from pathlib import Path
import re
import shutil
import signal
import socket
import stat
import subprocess
import sys
import time

TOOL_ROOT = Path(os.environ['RUST_PROBE_TOOL_ROOT']) if 'RUST_PROBE_TOOL_ROOT' in os.environ else Path(__file__).resolve().parents[4]
HERE = Path(__file__).resolve().parent
PREVIOUS = HERE / 'legacy.py' if (HERE / 'legacy.py').is_file() else TOOL_ROOT / 'docs/codeparsers/16-11-2026/rust-red/pilot.py'
base = _bootstrap_module.load_module('qualified_rust_pilot', PREVIOUS, context=_bootstrap_context)
physical = _bootstrap_module.load_module('rust_physical_guard', HERE / 'physical_guard.py', context=_bootstrap_context)

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
    assert str(TOOL_ROOT) == pair['tool_root']
    assert base.git(TOOL_ROOT, 'rev-parse', 'HEAD') == pair['tool_source']
    assert not base.git(TOOL_ROOT, 'status', '--porcelain')
    assert base.digest(__file__) == pair['driver_sha256']
    assert base.digest(PREVIOUS) == pair['helper_sha256']
    assert base.digest(HERE / 'compare.py') == pair['comparison_sha256']
    assert base.digest(HERE / 'physical_guard.py') == pair['physical_guard_sha256']
    for name, sha in pair.get('controllers', {}).items():
        assert base.digest(Path(pair['control']) / name) == sha
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


def publication_engine():
    path = (Path(_bootstrap_context[0]['control']) /
        _bootstrap_context[0]['publication_layout']['entries']['coordinator']
        if _bootstrap_context is not None else
        TOOL_ROOT / 'docs/codeparsers/17-11-2026/rust-probe-relay/relay.py')
    return _bootstrap_module.load_module('relay', path, context=_bootstrap_context)


def publication_control_roles(pair, phase):
    command = phase_timeout_command(pair, phase)
    return {'outer-timeout': command, 'composer': command[4:]}


def publication_deadline_info(pair, phase):
    return {'role': 'outer-timeout', 'argv': phase_timeout_command(pair, phase)}


def publication_launch_command(pair, phase, allocation):
    return [sys.executable, '-B', str(Path(pair['control']) / 'pilot.py'), 'run',
        '--proofs', pair['proofs'], '--phase', phase, '--pin',
        base.digest(Path(pair['proofs']) / 'prepared.json'), '--allocation', allocation]


def publication_launch_environment(pair, phase):
    return {'RUST_PROBE_TOOL_ROOT': pair['tool_root']}


def publication_phase_info(pair, phase):
    """Codec de descripteurs Rust : lecture seulement, pas de décision de liveness."""
    root = Path(pair['proofs']) / phase
    info = json.loads((root / 'authority-path.json').read_text())
    control = Path(pair['control'])
    return {**info, 'scope_root': str(root / 'scope'),
        'driver_entry': 'pilot.py', 'common_entry': 'confiner_validation.py',
        'driver_argv': [sys.executable, '-B', str(control / 'pilot.py'), 'inside',
            '--proofs', pair['proofs'], '--phase', phase, '--pin',
            base.digest(Path(pair['proofs']) / 'prepared.json')],
        'common_argv': json.loads((root / 'common.command.json').read_text()),
        'controls_path': str(root / 'controls.json'), 'deadline_role': 'outer-timeout',
        'deadline_argv': phase_timeout_command(pair, phase), 'original_common_end': info['deadline']}


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
    if phase == 'baseline' and pair.get('prior_end'):
        prior = pair['prior_end']
        root = Path(prior['root'])
        assert base.digest(root / 'classification.json') == prior['classification_sha256']
        assert base.digest(root / 'build-ledger.json') == prior['ledger_sha256']
        scopes = list((root / 'scope').glob('validation-*/verdict.json'))
        assert len(scopes) == 1
        verdict = json.loads(scopes[0].read_text())
        authority = json.loads(scopes[0].with_name('autorite.json').read_text())
        assert verdict['code'] == prior['code'] and verdict['verdict'] == prior['verdict']
        assert not verdict['restes'] and verdict['nettoye'] and authority['etat'] == 'termine'
        assert authority['cgroup'] == prior['cgroup'] and not Path(verdict['prive']).exists()
        assert base.digest(root / 'bin/probe-executable') == prior['elf_sha256']
    elif phase == 'baseline':
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


def save(path, value):
    # Les ledgers ne deviennent visibles qu'après fsync, par lien exclusif.
    import tempfile
    path = Path(path)
    fd, pending = tempfile.mkstemp(prefix='.' + path.name + '-', dir=path.parent)
    try:
        with os.fdopen(fd, 'w') as stream:
            json.dump(value, stream, indent=2)
            stream.flush()
            os.fsync(stream.fileno())
        os.link(pending, path)
        directory = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY)
        try:
            os.fsync(directory)
        finally:
            os.close(directory)
    finally:
        os.unlink(pending)


def first_red(root, error):
    try:
        save(Path(root) / 'first-red.json', {'reason': repr(error), 'observed': time.monotonic()})
    except FileExistsError:
        pass


def run_step(root, label, argv, cwd, env, end, cap):
    deadline = min(end, time.monotonic() + cap)
    assert deadline > time.monotonic()
    assert not (root / 'first-red.json').exists(), 'external first red'
    save(root / (label + '.command.json'), {'argv': argv, 'cwd': str(cwd), 'deadline': deadline})
    with (root / (label + '.stdout')).open('xb') as out, (root / (label + '.stderr')).open('xb') as err:
        child = subprocess.Popen(argv, cwd=cwd, env=env, stdout=out, stderr=err)
        try:
            while child.poll() is None:
                assert not (root / 'first-red.json').exists(), 'external first red'
                if time.monotonic() >= deadline:
                    raise TimeoutError(label)
                if label == 'probe':
                    raw = (root / (label + '.stdout')).read_bytes() + (root / (label + '.stderr')).read_bytes()
                    if re.search(rb'^test result: FAILED|^thread .*panicked at|\.\.\. FAILED', raw, re.M):
                        raise RuntimeError('first native failure: ' + label)
                time.sleep(.05)
            save(root / (label + '.result.json'), {'rc': child.returncode, 'finished': time.monotonic()})
            assert child.returncode == 0, (label, child.returncode)
        except BaseException as error:
            first_red(root, error)
            if child.poll() is None:
                child.terminate()
                try:
                    child.wait(timeout=1)
                except subprocess.TimeoutExpired:
                    child.kill()
                    child.wait(timeout=1)
            raise


def choose_artifact(raw, source):
    rows = [json.loads(line) for line in raw.splitlines() if line.startswith('{')]
    found = [row for row in rows if row.get('reason') == 'compiler-artifact'
        and row.get('target', {}).get('name') == 'sonde_marque_nom'
        and row['target'].get('kind') == ['test'] and row['target'].get('src_path') == source
        and row.get('profile', {}).get('test') and row.get('executable')]
    assert len(found) == 1, 'expected exactly one source-bound compiler artifact'
    assert set(found[0]['features']) == {'code', 'rag3db-native'}
    return found[0]


def wait_publication(pair, phase, kind, ledger, end):
    root = Path(pair['proofs']) / phase
    path = root / (kind + '-peer-ack.json')
    while not path.exists():
        assert not (root / 'first-red.json').exists(), 'external first red'
        assert time.monotonic() < end, 'peer publication deadline'
        time.sleep(.1)
    ack = json.loads(path.read_text())
    assert ack['prepared_sha256'] == base.digest(Path(pair['proofs']) / 'prepared.json')
    assert ack['coordinator'] == json.loads((root / 'relay-launch.json').read_text())['coordinator']
    assert not (root / 'first-red.json').exists(), 'external first red'
    assert ack['host'] == pair['peer_host'] and ack['phase'] == phase and ack['kind'] == kind
    assert ack['ledger_sha256'] == base.digest(root / (kind + '-ledger.json'))
    assert ack['elf_sha256'] == ledger['elf_sha256'] and ack['copied_and_rehashed'] is True
    assert base.digest(ledger['source_executable']) == base.digest(ledger['copied_executable']) == ledger['elf_sha256']
    assert time.monotonic() < end


def fresh_quiet(pair, phase, path, final=False, authority=None, private=None):
    roots = [TARGET, *[config['worktree'] for config in pair['phases'].values()]]
    old = Path.home() / '.cache/codeparsers-notes/rust-followup-118cc71/authority-status.json'
    groups = [json.loads(old.read_text())['cgroup']]
    if pair.get('prior_end'):
        groups.append(pair['prior_end']['cgroup'])
    controls = []
    if final:
        controls = json.loads((Path(pair['proofs']) / phase / 'controls.json').read_text())
        groups.append(authority['cgroup'])
        roots.append(private)
    record = physical.scan('/proc', roots, groups, controls)
    save(path, record)
    assert not record['found'] and not any(row['attributed'] for row in record['unknown'])
    return record


def inside(pair, phase):
    assert Path(__file__).resolve() == Path(pair['control']) / 'pilot.py'
    os.chdir(Path(pair['proofs']) / phase)
    config = verify(pair, phase)
    previous_end(pair, phase)
    check_corpus(pair)
    root = Path(pair['proofs']) / phase
    worktree = Path(config['worktree'])
    crate = worktree / 'extension/rag3weaver'
    env = {key: os.environ[key] for key in ['HOME', 'USER', 'LOGNAME', 'PATH', 'SHELL', 'LANG', 'LC_ALL',
        'TZ', 'RUST_PROBE_TOOL_ROOT', 'CARGO_HOME', 'RUSTUP_HOME', 'DBUS_SESSION_BUS_ADDRESS', 'RAG3WEAVER_VALIDATION_RECU',
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
    run_step(root, 'authority-check', [sys.executable, str(Path(pair['control']) / 'confiner_validation.py'), 'verifier'], root, env,
        json.loads(Path(os.environ['RAG3WEAVER_VALIDATION_RECU']).read_text())['echeance'] - 15, 10)
    fresh_quiet(pair, phase, root / 'cache-quiet-after-poste.json')
    assert (TARGET / '.arbre-du-target').read_text().strip() == config['previous_owner']
    # SDK réel, compilation seule et identité du nouvel ELF avant toute invocation.
    receipt = Path(os.environ['RAG3WEAVER_VALIDATION_RECU'])
    authority = json.loads(receipt.read_text())
    bus_path = os.environ['DBUS_SESSION_BUS_ADDRESS'].split(',')[0].removeprefix('unix:path=')
    assert bus_path.startswith('/tmp/r3v-') and stat.S_ISSOCK(Path(bus_path).stat().st_mode)
    save(root / 'authority-path.json', {'path': str(receipt), 'deadline': authority['echeance'],
        'pid': authority['pid'], 'start': authority['debut'], 'cgroup': authority['cgroup'], 'bus_path': bus_path,
        'driver_identity': physical.process_record(os.getpid()),
        'supervisor_identity': physical.process_record(authority['pid']),
        'driver_script_sha256': base.digest(__file__)})
    assert authority['etat'] == 'actif'
    group = Path('/sys/fs/cgroup') / authority['cgroup'].lstrip('/')
    limits = {name: (group / name).read_text().strip() for name in
        ['memory.max', 'memory.high', 'memory.swap.max', 'cpu.max']}
    assert limits['memory.max'] == '34359738368' and limits['memory.high'] == 'max' and limits['memory.swap.max'] == '0'
    quota, period = limits['cpu.max'].split()
    assert quota != 'max' and int(quota) == 5 * int(period)
    save(root / 'envelope-observed.json', limits)
    deadline = authority['echeance'] - 15
    sdk_keys = ['RAG3DB_SHARED', 'RAG3DB_LIBRARY_DIR', 'RAG3DB_INCLUDE_DIR', 'LD_LIBRARY_PATH']
    resolver = ['bash', '-c', '. "$1"; exporter_la_lib || exit $?; shift; exec "$@"', 'real-library-resolver',
        str(crate / 'scripts/lib_du_magasin.sh'), sys.executable, '-c',
        'import os,json;print(json.dumps({k:os.environ[k] for k in '+repr(sdk_keys)+'}))']
    run_step(root, 'sdk-export', resolver, crate, env, deadline, 30)
    sdk = json.loads((root / 'sdk-export.stdout').read_text())
    expected_sdk = str(Path(config['store']) / base.LIB)
    assert sdk['RAG3DB_LIBRARY_DIR'] == sdk['RAG3DB_INCLUDE_DIR'] == expected_sdk and sdk['RAG3DB_SHARED'] == '1'
    env.update(sdk, CARGO_MANIFEST_DIR=str(crate))
    build = [sys.executable, str(Path(pair['control']) / 'cargo_garde.py'), 'test', '--locked', '--no-default-features',
        '--manifest-path', str(crate / 'Cargo.toml'), '--features', 'rag3db-native,code', '--test',
        'sonde_marque_nom', '--no-run', '-j8', '--message-format=json-render-diagnostics']
    run_step(root, 'compile', build, crate, env, deadline, 600)
    artifact = choose_artifact((root / 'compile.stdout').read_text(), str(crate / 'tests/sonde_marque_nom.rs'))
    original = Path(artifact['executable'])
    assert original.resolve() == original and TARGET in original.parents and original.is_file()
    (root / 'bin').mkdir(mode=0o700)
    copied = root / 'bin/probe-executable'
    shutil.copy2(original, copied)
    copied.chmod(0o500)
    elf_sha = base.digest(original)
    assert base.digest(copied) == elf_sha
    ledger = {'app': config['source'], 'parser': PARSERS[phase], 'artifact': artifact,
        'source_executable': str(original), 'copied_executable': str(copied), 'elf_sha256': elf_sha,
        'sdk': base.LIB, 'sdk_hashes': base.LIB_HASHES, 'corpus_sha256': CORPUS_SHA,
        'features': sorted(artifact['features']), 'profile': artifact['profile'],
        'expected_names': ['aretes_par_marque_en_tsv'], 'published_before_any_elf_invocation': time.monotonic()}
    if phase == 'candidate':
        prior = json.loads((Path(pair['proofs']) / 'baseline/runtime-ledger.json').read_text())
        assert ledger['features'] == prior['features'] and ledger['profile'] == prior['profile']
    save(root / 'build-ledger.json', ledger)
    wait_publication(pair, phase, 'build', ledger, deadline)
    run_step(root, 'inventory-all', [str(copied), '--list'], crate, env, deadline, 15)
    run_step(root, 'inventory-ignored', [str(copied), '--list', '--ignored'], crate, env, deadline, 15)
    for label in ['inventory-all', 'inventory-ignored']:
        names = [line[:-6] for line in (root / (label + '.stdout')).read_text().splitlines() if line.endswith(': test')]
        assert names == ['aretes_par_marque_en_tsv']
    ledger.update(inventory_sha256=base.digest(root / 'inventory-all.stdout'),
        ignored_inventory_sha256=base.digest(root / 'inventory-ignored.stdout'),
        ignored_inventory_names=['aretes_par_marque_en_tsv'], published_before_test=time.monotonic())
    assert base.digest(original) == base.digest(copied) == elf_sha
    save(root / 'runtime-ledger.json', ledger)
    wait_publication(pair, phase, 'runtime', ledger, deadline)
    save(root / 'runtime-start.json', {'ledger_sha256': base.digest(root / 'runtime-ledger.json'),
        'elf_sha256': elf_sha, 'observed': time.monotonic()})
    run_step(root, 'probe', [str(copied), '--exact', 'aretes_par_marque_en_tsv', '--ignored', '--nocapture'],
        crate, env, deadline, max(1, deadline - time.monotonic()))
    verify(pair, phase)
    check_corpus(pair)
    assert base.digest(original) == base.digest(copied) == elf_sha
    save(root / 'output-check.json', check_outputs(pair, phase))
    assert time.monotonic() < deadline
    save(root / 'driver-result.json', {'status': 'PROBE_COMPLETED', 'elf_sha256': elf_sha,
        'end': authority['echeance'], 'finished': time.monotonic()})
    return 0


def phase_timeout_command(pair, phase):
    return [pair['timeout'], '--signal=TERM', '--kill-after=2s', '1798s',
        sys.executable, '-B', str(Path(pair['control']) / 'pilot.py'), 'compose',
        '--proofs', pair['proofs'], '--phase', phase, '--pin',
        base.digest(Path(pair['proofs']) / 'prepared.json')]


def outer_command(pair, phase):
    # Poste est acquis AVANT de produire l'origine du watchdog de phase.
    return [str(Path(pair['control']) / 'poste'), 'lourd', sys.executable, '-B',
        str(Path(pair['control']) / 'pilot.py'), 'phase-arm', '--proofs', pair['proofs'],
        '--phase', phase, '--pin', base.digest(Path(pair['proofs']) / 'prepared.json')]


def phase_arm(pair, phase):
    relay = publication_engine()
    relay.engine.verify_components(pair)
    assert socket.gethostname().lower() == relay.engine.publication_layout(pair)['execution_host'].lower()
    assert relay.engine.layout_entry(pair, 'launcher') == Path(__file__).resolve()
    pin = base.digest(Path(pair['proofs']) / 'prepared.json')
    emitter = physical.process_record(os.getpid())
    window = relay.engine.make_window(pair, pin, phase, 'phase', emitter, time.monotonic())
    path = Path(relay.engine.publication_layout(pair)['phases'][phase]['files']['phase_origin'])
    save(path, window)
    assert base.digest(pair['timeout']) == pair['timeout_sha256']
    command = phase_timeout_command(pair, phase)
    os.execve(pair['timeout'], command, os.environ.copy())


def compose(pair, phase):
    assert Path(__file__).resolve() == Path(pair['control']) / 'pilot.py'
    root = Path(pair['proofs']) / phase
    started = time.monotonic()
    relay = publication_engine()
    pin = base.digest(Path(pair['proofs']) / 'prepared.json')
    origin_path = Path(relay.engine.publication_layout(pair)['phases'][phase]['files']['phase_origin'])
    origin = json.loads(origin_path.read_text())
    deadline = origin['soft_end']
    common = _bootstrap_module.load_module('probe_common', Path(pair['control']) / 'confiner_validation.py',
        context=_bootstrap_context, expected_sha=pair['controllers']['confiner_validation.py'])
    parent = physical.process_record(os.getppid())
    assert parent['exe'] == pair['timeout'] and parent['argv'] == phase_timeout_command(pair, phase)
    relay.engine.window_end(pair, pin, phase, 'phase', origin, parent)
    assert base.digest(pair['timeout']) == pair['timeout_sha256']
    assert common.poste_tenu(common.processus())
    # Deux rôles prouvés ; aucune exemption d'ancêtre, de suffixe FD ou de chemin.
    controls = [dict(physical.process_record(os.getpid()), role='composer'), dict(parent, role='outer-timeout')]
    save(root / 'controls.json', controls)
    save(root / 'outer-supervision.json', {'soft_timeout': 1798, 'kill_after': 2,
        'common_budget': 1750, 'hardcap_per_phase': 1800, 'max_two_phases': 3600,
        'started': started, 'timeout_identity': parent, 'window': dict(origin, identity=parent),
        'phase_origin_sha256': base.digest(origin_path)})
    def stop(sig, frame):
        raise TimeoutError('outer supervision signal')
    signal.signal(signal.SIGTERM, stop)
    record = {'verdict': 'ROUGE', 'phase_completed': False, 'phase': phase, 'global_pass': False}
    try:
        assert not (root / 'first-red.json').exists(), 'launcher already refused'
        verify(pair, phase)
        previous_end(pair, phase)
        fresh_quiet(pair, phase, root / 'cache-quiet-before-common.json')
        # Le même noyau vérifie le watchdog et borne la demande avant Common.
        relay = publication_engine()
        common_budget = relay.engine.common_request_budget(pair,
            base.digest(Path(pair['proofs']) / 'prepared.json'), phase, physical)
        save(root / 'common-request.json', {'requested': common_budget,
            'observed': time.monotonic(), 'common_max': pair['publication_limits']['common_max']})
        command = [sys.executable, str(Path(pair['control']) / 'confiner_validation.py'),
            'lancer', '--delai', str(common_budget), '--preuves', str(root / 'scope'), '--', sys.executable,
            '-B', str(Path(pair['control']) / 'pilot.py'), 'inside', '--proofs', pair['proofs'], '--phase', phase, '--pin',
        base.digest(Path(pair['proofs']) / 'prepared.json')]
        rc = base.record(root, 'common', command, root, os.environ.copy())
        info = json.loads((root / 'authority-path.json').read_text())
        authority_path = Path(info['path'])
        authority = json.loads(authority_path.read_text())
        deadline = min(deadline, authority['echeance'])
        assert rc == 0
        completed_scope(root)
        verdict = json.loads(authority_path.with_name('verdict.json').read_text())
        assert not Path(info['bus_path']).exists() and not Path(verdict['prive']).exists()
        verify(pair, phase)
        counts = check_outputs(pair, phase)
        ledger = json.loads((root / 'runtime-ledger.json').read_text())
        assert base.digest(ledger['source_executable']) == base.digest(ledger['copied_executable']) == ledger['elf_sha256']
        assert json.loads((root / 'driver-result.json').read_text())['status'] == 'PROBE_COMPLETED'
        assert not (root / 'first-red.json').exists()
        quiet = physical.owned_quiet(authority_path.parent)
        save(root / 'owned-after-scope.json', quiet)
        assert not quiet['alive'] and not quiet['unknown']
        fresh_quiet(pair, phase, root / 'cache-quiet-final.json', True, authority, verdict['prive'])
        record.update(verdict='VERT', phase_completed=True, common_rc=rc, counts=counts, deadline=deadline)
    except BaseException as error:
        first_red(root, error)
        record.update(error=repr(error))
    published = physical.publish_result(root / 'classification.json', record, deadline)
    return 0 if published['verdict'] == 'VERT' else 1


def prepare(args):
    assert os.uname().nodename.lower() == 'luciepc' and args.peer_host
    proofs = args.proofs.absolute()
    manifest = TOOL_ROOT / 'docs/codeparsers/16-11-2026/rust-followup/frozen-corpus.json'
    pair = {'execution_id': proofs.name, 'tool_root': str(TOOL_ROOT), 'peer_host': args.peer_host,
        'physical_guard_sha256': base.digest(HERE / 'physical_guard.py'), 'proofs': str(proofs), 'manifest': str(manifest), 'tool_source': args.tool_source or base.git(TOOL_ROOT, 'rev-parse', 'HEAD'),
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
    assert args.peer_timeout_sha256 and re.fullmatch('[0-9a-f]{64}', args.peer_timeout_sha256)
    pair['peer_timeout'] = {'path': args.peer_timeout, 'sha256': args.peer_timeout_sha256}
    pair['relay'] = {'queue_budget': 300, 'transfer_cap': 30, 'control_cap': 5, 'stop_reserve': 5}
    pair['phases'] = {}
    if args.prior_root:
        prior_root = args.prior_root.resolve(strict=True)
        verdict_paths = list((prior_root / 'scope').glob('validation-*/verdict.json'))
        assert len(verdict_paths) == 1
        verdict = json.loads(verdict_paths[0].read_text())
        authority = json.loads(verdict_paths[0].with_name('autorite.json').read_text())
        old_ledger = json.loads((prior_root / 'build-ledger.json').read_text())
        assert old_ledger['app'] == BASELINE_SOURCE and old_ledger['parser'] == PARSERS['baseline']
        pair['prior_end'] = {'root': str(prior_root), 'verdict': verdict['verdict'], 'code': verdict['code'],
            'cgroup': authority['cgroup'], 'classification_sha256': base.digest(prior_root / 'classification.json'),
            'ledger_sha256': base.digest(prior_root / 'build-ledger.json'), 'elf_sha256': old_ledger['elf_sha256']}
    for phase, worktree, source in [('baseline', args.baseline_worktree, BASELINE_SOURCE),
            ('candidate', args.candidate_worktree, args.candidate_source)]:
        worktree = worktree.resolve(strict=True)
        previous_owner = str(args.candidate_worktree.resolve() / 'extension/rag3weaver') if phase == 'baseline' else str(args.baseline_worktree.resolve() / 'extension/rag3weaver')
        if phase == 'baseline' and args.prior_root:
            previous_owner = str(args.baseline_worktree.resolve() / 'extension/rag3weaver')
        pair['phases'][phase] = {'parser': PARSERS[phase], 'worktree': str(worktree), 'source': source, 'target': str(TARGET),
            'store': str(Path.home() / '.cache/rag3db-libs'), 'budget': 1800, 'previous_owner': previous_owner,
            'pilot_sha256': base.digest(PREVIOUS), 'scripts': {name: base.digest(worktree / name) for name in scripts}}
    adapter = _bootstrap_module.load_module('rust_plan_adapter',
        TOOL_ROOT / 'docs/codeparsers/17-11-2026/rust-probe-relay/relay.py')
    pair['publication_limits'] = adapter.default_limits()
    pair['publication_plans'] = {phase: adapter.default_plan(pair, phase) for phase in PRODUCTS}
    pair['publication_layout'] = adapter.default_layout(pair)
    for name in scripts:
        if name != PROBE:
            assert pair['phases']['baseline']['scripts'][name] == pair['phases']['candidate']['scripts'][name], 'configuration differs'
    assert (TARGET / '.arbre-du-target').read_text().strip() == pair['phases']['baseline']['previous_owner']
    previous_end(pair, 'baseline')
    proofs.mkdir(mode=0o700, parents=True, exist_ok=False)
    Path(pair['runtime_profile']).write_text(json.dumps(profile, indent=2))
    pair['runtime_profile_sha256'] = base.digest(pair['runtime_profile'])
    control = proofs / 'control'
    control.mkdir(mode=0o700)
    assert not any(root == control or root in control.parents for root in [TARGET, args.baseline_worktree, args.candidate_worktree])
    pair['control'] = str(control)
    sources = {name: HERE / name for name in ['pilot.py', 'physical_guard.py', 'compare.py']}
    relay_sources = TOOL_ROOT / 'docs/codeparsers/17-11-2026/rust-probe-relay'
    sources.update({name: relay_sources / name for name in ['relay.py', 'gate.py']})
    sources['publication_relay.py'] = TOOL_ROOT / 'extension/rag3weaver/scripts/publication_relay.py'
    sources['publication_loader.py'] = TOOL_ROOT / 'extension/rag3weaver/scripts/publication_loader.py'
    sources['legacy.py'] = PREVIOUS
    sources.update({name: args.candidate_worktree / 'extension/rag3weaver/scripts' / name
        for name in ['poste', 'confiner_validation.py', 'cargo_garde.py', 'garde_target.sh']})
    pair['controllers'] = {}
    for name, source in sources.items():
        shutil.copy2(source, control / name)
        pair['controllers'][name] = base.digest(control / name)
    pair['timeout'] = str(Path(shutil.which('timeout')).resolve(strict=True))
    pair['timeout_sha256'] = base.digest(pair['timeout'])
    for phase in PRODUCTS:
        verify(pair, phase)
        (proofs / phase).mkdir(mode=0o700)
    (proofs / 'prepared.json').write_text(json.dumps(pair, indent=2))
    print('PREPARED; neither Cargo, test nor authority started; two separate future allocations required')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['prepare', 'run', 'phase-arm', 'compose', 'inside', 'compare'])
    parser.add_argument('--proofs', type=Path, required=True)
    parser.add_argument('--phase', choices=list(PRODUCTS))
    parser.add_argument('--prepared', type=Path)
    parser.add_argument('--pin')
    parser.add_argument('--baseline-worktree', type=Path)
    parser.add_argument('--candidate-worktree', type=Path)
    parser.add_argument('--candidate-source')
    parser.add_argument('--tool-source')
    parser.add_argument('--prior-root', type=Path)
    parser.add_argument('--peer-timeout', default='/usr/bin/timeout')
    parser.add_argument('--peer-timeout-sha256')
    parser.add_argument('--allocation')
    parser.add_argument('--peer-host')
    args = parser.parse_args()
    if args.action == 'prepare':
        prepare(args)
        return 0
    pair = json.loads((args.proofs / 'prepared.json').read_text())
    if args.action == 'compare':
        # Rapport CPU distinct : aucune prétention de l'inclure dans les deux phases natives.
        assert args.allocation, 'separate explicit offline reporting allocation required'
        for phase in PRODUCTS:
            verify(pair, phase)
            completed_scope(Path(pair['proofs']) / phase)
        comparison = _bootstrap_module.load_module('probe_comparison', HERE / 'compare.py',
            context=_bootstrap_context, expected_sha=pair['comparison_sha256'])
        print(comparison.compare_pair(pair, read_edges, check_outputs))
        return 0
    if args.phase is None:
        parser.error('--phase is required')
    if args.action == 'phase-arm':
        return phase_arm(pair, args.phase)
    if args.action == 'compose':
        return compose(pair, args.phase)
    if args.action == 'inside':
        try:
            return inside(pair, args.phase)
        except BaseException as error:
            first_red(Path(pair['proofs']) / args.phase, error)
            raise
    # Sortir du worktree AVANT de créer un parent persistant ou d'entrer dans poste.
    control = Path(pair['control']) / 'pilot.py'
    if Path(__file__).resolve() != control:
        os.chdir(pair['proofs'])
        env = dict(os.environ, RUST_PROBE_TOOL_ROOT=pair['tool_root'])
        launch_args = sys.argv[1:]
        if '--pin' not in launch_args:
            launch_args = [*launch_args, '--pin', base.digest(Path(pair['proofs']) / 'prepared.json')]
        os.execve(sys.executable, [sys.executable, '-B', str(control), *launch_args], env)
    root = Path(pair['proofs']) / args.phase
    bootstrap = json.loads((root / 'relay-bootstrap.json').read_text())
    assert bootstrap['prepared_sha256'] == base.digest(Path(pair['proofs']) / 'prepared.json')
    assert bootstrap['gate_sha256'] == base.digest(Path(pair['control']) / 'gate.py') == pair['controllers']['gate.py']
    actual = physical.process_record(os.getpid())
    assert actual['argv'] == bootstrap['run_argv']
    assert all(actual[key] == bootstrap['identity'][key] for key in ['pid', 'start', 'uid', 'exe', 'cgroup'])
    assert time.monotonic() < bootstrap['transition_end']
    # Rendez-vous APRÈS le véritable exec, avant toute vérification susceptible de durer.
    save(root / 'relay-launch.json', {'identity': actual, 'prepared_sha256': bootstrap['prepared_sha256'],
        'coordinator': bootstrap['coordinator'], 'launcher_script_sha256': base.digest(__file__)})
    verify(pair, args.phase)
    previous_end(pair, args.phase)
    assert args.allocation and 'RAG3WEAVER_VALIDATION_RECU' not in os.environ
    root = Path(pair['proofs']) / args.phase
    save(root / 'allocation.json', {'reference': args.allocation})
    env = os.environ.copy()
    env.update(POSTE_MEM_MAX='32G', POSTE_SWAP_MAX='0', POSTE_CPU_QUOTA='500%',
        RAG3WEAVER_BUILD_MEMORY_HIGH='0', POSTE_LOURD_ATTENTE_MAX='300', RUST_PROBE_TOOL_ROOT=pair['tool_root'])
    # Comme af11 : aucun scan, hash ou verdict après le retour de l'enfant borné.
    def interrupted(sig, frame):
        raise TimeoutError('remote launcher interrupted ' + str(sig))
    for sig in [signal.SIGTERM, signal.SIGINT, signal.SIGHUP]:
        signal.signal(sig, interrupted)
    with (root / 'outer.log').open('x') as out:
        child = subprocess.Popen(outer_command(pair, args.phase), cwd=pair['proofs'], env=env,
            stdout=out, stderr=subprocess.STDOUT)
        try:
            identity = physical.process_record(child.pid)
            save(root / 'poste-child.json', {'identity': identity, 'queue_end': bootstrap['queue_end']})
            acquired = False
            while child.poll() is None:
                marker = root / 'outer-supervision.json'
                if not acquired and marker.exists():
                    began = json.loads(marker.read_text())['timeout_identity']
                    phase_origin = json.loads((root / 'phase-origin.json').read_text())
                    relay = publication_engine()
                    relay.engine.window_end(pair, args.pin, args.phase, 'phase', phase_origin, began)
                    assert phase_origin['started'] <= bootstrap['queue_end']
                    acquired = True
                if not acquired and time.monotonic() >= bootstrap['queue_end']:
                    raise TimeoutError('PC poste queue deadline')
                if not acquired and (root / 'first-red.json').exists():
                    raise RuntimeError('queued launcher refused')
                time.sleep(.05)
            return child.returncode
        except BaseException as error:
            first_red(root, error)
            # Popen conserve l'identité de cet enfant non récolté ; poste gère son attente.
            if child.poll() is None:
                child.terminate()
                try:
                    child.wait(timeout=.7)
                except subprocess.TimeoutExpired:
                    child.kill()
                    child.wait(timeout=.7)
            raise


if __name__ == '__main__':
    sys.exit(main())
