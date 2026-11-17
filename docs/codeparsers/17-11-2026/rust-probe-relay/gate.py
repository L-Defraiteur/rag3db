#!/usr/bin/env python3
"""Fournisseur de ledger Rust pour la porte commune ; aucune seconde boucle."""
import importlib.util
import json
import os
from pathlib import Path
import signal
import socket
import sys
import time

from relay import engine, atomic_json, digest


def load_pilot(pair):
    os.environ['RUST_PROBE_TOOL_ROOT'] = pair['tool_root']
    spec = importlib.util.spec_from_file_location('relay_gate_pilot', Path(pair['control']) / 'pilot.py')
    pilot = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(pilot)
    return pilot


class RustPublicationProvider:
    """Seules les lectures/validations Rust ; aucun export, ACK ou test natif."""
    def __init__(self, gate):
        self.root, self.pair, self.phase, self.pilot = gate.root, gate.pair, gate.phase, gate.pilot

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
        return ledger, files


class Gate(engine.GateBase):
    def __init__(self, pair, pin, phase):
        super().__init__(pair, pin, phase, pilot=load_pilot(pair), entry_file=__file__)
        self.provider = RustPublicationProvider(self)


def main():
    return engine.gate_main(Gate)


if __name__ == '__main__':
    main()
