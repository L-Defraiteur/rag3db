#!/usr/bin/env python3
"""Adaptateur Rust du relais commun ; comportement D45 à deux publications."""
# Racine d'import minimale : le loader est épinglé LITTÉRALEMENT avant exec_module.
import hashlib as _bootstrap_hashlib
import importlib.util as _bootstrap_importlib
from pathlib import Path as _BootstrapPath
_bootstrap_entry = _BootstrapPath(__file__).absolute()
assert _bootstrap_entry.resolve(strict=True) == _bootstrap_entry
_bootstrap_source = (_bootstrap_entry.parents[4] / 'docs/codeparsers/17-11-2026/rust-probe-relay/relay.py'
    if tuple(_bootstrap_entry.parts[-5:]) == tuple(_BootstrapPath('docs/codeparsers/17-11-2026/rust-probe-relay/relay.py').parts) else None)
_bootstrap_loader = (_bootstrap_entry.parents[4] / 'extension/rag3weaver/scripts/publication_loader.py'
    if _bootstrap_source is not None else _bootstrap_entry.parent / 'publication_loader.py')
_bootstrap_loader_bytes = _bootstrap_loader.read_bytes()
assert _bootstrap_hashlib.sha256(_bootstrap_loader_bytes).hexdigest() == '5ee566f69c978f1444bd3b9d685145e2cfc68cc263f9ef48553780c54f593d4b'
_bootstrap_spec = _bootstrap_importlib.spec_from_file_location('verified_publication_loader', _bootstrap_loader)
_bootstrap_module = _bootstrap_importlib.module_from_spec(_bootstrap_spec)
exec(compile(_bootstrap_loader_bytes, str(_bootstrap_loader), 'exec'), _bootstrap_module.__dict__)
_bootstrap_context = _bootstrap_module.verify_entry(_bootstrap_entry, source_entry=_bootstrap_source)

import importlib.util
import os
from pathlib import Path
import signal
import socket
import sys
import time

HERE = Path(__file__).resolve().parent
COMMON = HERE / 'publication_relay.py'
if not COMMON.is_file():
    root = Path(__file__).resolve().parents[4]
    assert root / 'docs/codeparsers/17-11-2026/rust-probe-relay/relay.py' == Path(__file__).resolve()
    COMMON = root / 'extension/rag3weaver/scripts/publication_relay.py'
if 'publication_relay' in sys.modules:
    engine = sys.modules['publication_relay']
    assert Path(engine.__file__).resolve() == COMMON.resolve(strict=True)
    if _bootstrap_context is not None:
        assert engine.__verified_source_sha256__ == _bootstrap_context[0]['controllers']['publication_relay.py']
else:
    engine = _bootstrap_module.load_module('publication_relay', COMMON, context=_bootstrap_context)


def default_limits():
    return {'whole_soft': 2098, 'whole_kill': 2, 'queue': 300, 'phase_soft': 1798,
        'phase_kill': 2, 'common_max': 1750, 'common_margin': 30,
        'operation_grace': 15, 'stop_reserve': 5, 'control_cap': 5, 'copy_cap': 30}


def default_layout(pair):
    proofs = Path(pair['proofs'])
    names = {'bootstrap': 'relay-bootstrap.json', 'launch': 'relay-launch.json',
        'authority': 'authority-path.json', 'controls': 'controls.json',
        'outer_supervision': 'outer-supervision.json', 'common_command': 'common.command.json',
        'allocation': 'allocation.json', 'first_red': 'first-red.json',
        'abort': 'relay-abort.json', 'classification': 'classification.json',
        'peer_ready': 'relay-local/ready.json', 'peer_supervision': 'relay-local/local-hard-supervision.json',
        'whole_origin': 'whole-origin.json', 'peer_origin': 'peer-origin.json',
        'phase_origin': 'phase-origin.json', 'launch_input': 'launch-input.json'}
    return {'prepared': str(proofs / 'prepared.json'), 'execution_host': 'luciepc',
        'entries': {'coordinator': 'relay.py', 'gate': 'gate.py', 'launcher': 'pilot.py',
            'driver': 'pilot.py', 'common': 'confiner_validation.py', 'guard': 'physical_guard.py'},
        'phases': {phase: {'root': str(proofs / phase), 'peer': str(proofs / phase / 'relay-local'),
            'scope': str(proofs / phase / 'scope'), 'deadline_role': 'outer-timeout',
            'files': {key: str(proofs / phase / name) for key, name in names.items()}}
            for phase in pair['publication_plans']}}


def default_plan(pair, phase):
    config = pair['phases'][phase]
    binding = {'contract_id': 'rust-probe-ledger-v1', 'app': config['source'],
        'parser': config['parser'], 'sdk': pair['sdk'],
        'corpus_sha256': pair['corpus_manifest_sha256'], 'features': ['code', 'rag3db-native']}
    return {'protocol_version': 1, 'execution_id': pair['execution_id'], 'phase_id': phase,
        'stages': [{'label': 'build', 'ledger_file': 'ledger',
            'files': {'ledger': 'ledger', 'elf': 'executable'},
            'executables': {'probe': {'stage': 'build', 'file': 'elf'}},
            'source_contract': binding, 'window_seconds': None},
            {'label': 'runtime', 'ledger_file': 'ledger',
            'files': {'ledger': 'ledger', 'all': 'data', 'ignored': 'data'},
            'executables': {'probe': {'stage': 'build', 'file': 'elf'}},
            'source_contract': binding, 'window_seconds': None}]}


class Relay(engine.Relay):
    """Défaut Rust strict ; toutes les boucles restent dans le noyau commun."""
    def __init__(self, transport, root, clock=time.monotonic, sleep=time.sleep,
                 queue_budget=300, transfer_cap=30, control_cap=5, stop_reserve=5,
                 hard_end=None, *, plan=None):
        declared = transport.plan if plan is None else plan
        if hard_end is None:
            hard_end = clock() + default_limits()['whole_soft']
        super().__init__(transport, root, clock, sleep, queue_budget=queue_budget,
            transfer_cap=transfer_cap, control_cap=control_cap, stop_reserve=stop_reserve,
            hard_end=hard_end, plan=declared, prepared_sha256=transport.prepared_sha256)


SSHTransport = engine.SSHTransport
digest = engine.digest
atomic_json = engine.atomic_json
publish_result = engine.publish_result


def main():
    return engine.relay_main(__file__)


if __name__ == '__main__':
    sys.exit(main())
