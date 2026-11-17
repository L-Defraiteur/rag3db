#!/usr/bin/env python3
"""Fournisseur de ledger Rust pour la porte commune ; aucune seconde boucle."""
# Racine d'import minimale : le loader est épinglé LITTÉRALEMENT avant exec_module.
import hashlib as _bootstrap_hashlib
import importlib.util as _bootstrap_importlib
from pathlib import Path as _BootstrapPath
_bootstrap_entry = _BootstrapPath(__file__).absolute()
assert _bootstrap_entry.resolve(strict=True) == _bootstrap_entry
_bootstrap_source = (_bootstrap_entry.parents[4] / 'docs/codeparsers/17-11-2026/rust-probe-relay/gate.py'
    if tuple(_bootstrap_entry.parts[-5:]) == tuple(_BootstrapPath('docs/codeparsers/17-11-2026/rust-probe-relay/gate.py').parts) else None)
_bootstrap_loader = (_bootstrap_entry.parents[4] / 'extension/rag3weaver/scripts/publication_loader.py'
    if _bootstrap_source is not None else _bootstrap_entry.parent / 'publication_loader.py')
_bootstrap_loader_bytes = _bootstrap_loader.read_bytes()
assert _bootstrap_hashlib.sha256(_bootstrap_loader_bytes).hexdigest() == 'bfdc8ab3bcaac209d644a8e25e71c79765d36a736eac68141219f4eec491093e'
_bootstrap_spec = _bootstrap_importlib.spec_from_file_location('verified_publication_loader', _bootstrap_loader)
_bootstrap_module = _bootstrap_importlib.module_from_spec(_bootstrap_spec)
exec(compile(_bootstrap_loader_bytes, str(_bootstrap_loader), 'exec'), _bootstrap_module.__dict__)
_bootstrap_context = _bootstrap_module.verify_entry(_bootstrap_entry, source_entry=_bootstrap_source)

import importlib.util
import json
import os
from pathlib import Path
import signal
import socket
import sys
import time

_relay_path = (_BootstrapPath(_bootstrap_context[0]['control']) / _bootstrap_context[0]['publication_layout']['entries']['coordinator']
    if _bootstrap_context is not None else _BootstrapPath(__file__).with_name('relay.py'))
_relay = _bootstrap_module.load_module('relay', _relay_path, context=_bootstrap_context)
engine, atomic_json, digest = _relay.engine, _relay.atomic_json, _relay.digest


def load_pilot(pair):
    engine.verify_components(pair)
    os.environ['RUST_PROBE_TOOL_ROOT'] = pair['tool_root']
    entry = engine.layout_entry(pair, 'launcher')
    return _bootstrap_module.load_module('relay_gate_pilot', entry, context=_bootstrap_context,
        expected_sha=pair['controllers'][entry.name])


class RustPublicationProvider:
    """Seules les lectures/validations Rust ; aucun export, ACK ou test natif."""
    def __init__(self, gate):
        self.root, self.pair, self.phase, self.pilot = gate.root, gate.pair, gate.phase, gate.pilot

    def ledger_path(self, kind):
        return self.root / (kind + '-ledger.json')

    def inherited(self, stage, file_id):
        assert stage == 'build' and file_id == 'elf'
        return self.root / 'bin/probe-executable'

    def read(self, kind):
        ledger_path = self.root / (kind + '-ledger.json')
        ledger = json.loads(ledger_path.read_text())
        config = self.pair['phases'][self.phase]
        assert ledger['app'] == config['source'] and ledger['parser'] == self.pilot.PARSERS[self.phase]
        assert ledger['sdk'] == self.pilot.base.LIB and ledger['sdk_hashes'] == self.pilot.base.LIB_HASHES
        assert ledger['corpus_sha256'] == self.pilot.CORPUS_SHA
        assert ledger['features'] == ['code', 'rag3db-native']
        assert ledger['copied_executable'] == str(self.root / 'bin/probe-executable')
        source = Path(ledger['source_executable'])
        assert Path(config['target']) in source.parents and source.resolve(strict=True) == source
        assert digest(source) == digest(ledger['copied_executable']) == ledger['elf_sha256']
        files = {'ledger': ledger_path}
        if kind == 'build':
            files['elf'] = Path(ledger['copied_executable'])
        else:
            files.update(all=self.root / 'inventory-all.stdout', ignored=self.root / 'inventory-ignored.stdout')
            assert digest(files['all']) == ledger['inventory_sha256']
            assert digest(files['ignored']) == ledger['ignored_inventory_sha256']
        observed = {key: ledger[key] for key in ['app', 'parser', 'sdk', 'sdk_hashes',
            'features', 'corpus_sha256', 'profile', 'artifact']}
        return ledger, files, observed


class Gate(engine.GateBase):
    def __init__(self, pair, pin, phase):
        engine.verify_bound_context(pair, pin)
        super().__init__(pair, pin, phase, pilot=load_pilot(pair), entry_file=__file__)
        self.provider = RustPublicationProvider(self)


def main():
    return engine.gate_main(Gate)


if __name__ == '__main__':
    main()
