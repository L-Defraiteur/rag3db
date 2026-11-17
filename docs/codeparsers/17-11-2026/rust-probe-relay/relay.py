#!/usr/bin/env python3
"""Adaptateur Rust du relais commun ; comportement D45 à deux publications."""
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
else:
    spec = importlib.util.spec_from_file_location('publication_relay', COMMON)
    engine = importlib.util.module_from_spec(spec)
    sys.modules['publication_relay'] = engine
    spec.loader.exec_module(engine)

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
