#!/usr/bin/env python3
"""Porte PC : aucun export ou ACK sans phase active vérifiée sur l'horloge PC."""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import socket
import sys
import time

from relay import atomic_json, digest


def load_pilot(pair):
    os.environ['RUST_PROBE_TOOL_ROOT'] = pair['tool_root']
    spec = importlib.util.spec_from_file_location('relay_gate_pilot', Path(pair['control']) / 'pilot.py')
    pilot = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(pilot)
    return pilot


class Gate:
    def __init__(self, pair, pin, phase):
        self.pair, self.pin, self.phase = pair, pin, phase
        self.root = Path(pair['proofs']) / phase
        self.control = Path(pair['control'])
        self.pilot = load_pilot(pair)
        self.guard = self.pilot.physical

    def verify(self):
        assert socket.gethostname().lower() == 'luciepc'
        assert digest(Path(self.pair['proofs']) / 'prepared.json') == self.pin
        assert Path(__file__).resolve() == self.control / 'gate.py'
        assert digest(__file__) == self.pair['controllers']['gate.py']
        self.pilot.verify(self.pair, self.phase)
        for name in ['first-red.json', 'relay-abort.json', 'classification.json']:
            assert not (self.root / name).exists(), 'phase has already refused or ended'

    def check_process(self, expected):
        assert expected['uid'] == os.getuid()
        actual = self.guard.process_record(expected['pid'])
        assert actual['state'] != 'Z'
        assert all(actual[k] == expected[k] for k in ['pid', 'start', 'uid', 'exe', 'argv', 'cgroup'])
        return actual

    def launcher(self):
        record = json.loads((self.root / 'relay-launch.json').read_text())
        self.check_process(record['identity'])
        assert record['prepared_sha256'] == self.pin
        return record

    def active(self):
        self.verify()
        launch = self.launcher()
        info = json.loads((self.root / 'authority-path.json').read_text())
        receipt = Path(info['path'])
        assert receipt.name == 'autorite.json' and receipt.parent.parent == self.root / 'scope'
        assert receipt.resolve(strict=True) == receipt
        authority = json.loads(receipt.read_text())  # Le jeton ne sort jamais de ce processus.
        assert authority['etat'] == 'actif' and not receipt.with_name('verdict.json').exists()
        assert authority['script_sha256'] == self.pair['controllers']['confiner_validation.py']
        assert authority['script'] == str(self.control / 'confiner_validation.py')
        common = self.check_process(info['supervisor_identity'])
        assert common['pid'] == authority['pid'] and common['start'] == authority['debut']
        assert common['cgroup'].split(':', 2)[-1] == authority['cgroup']
        assert common['argv'] == json.loads((self.root / 'common.command.json').read_text())
        driver = self.check_process(info['driver_identity'])
        assert info['driver_script_sha256'] == digest(self.control / 'pilot.py') == self.pair['driver_sha256']
        expected_driver = [sys.executable, '-B', str(self.control / 'pilot.py'), 'inside',
            '--proofs', self.pair['proofs'], '--phase', self.phase]
        assert driver['argv'] == expected_driver
        cursor, seen = driver, set()
        while cursor['pid'] != common['pid']:
            assert cursor['pid'] not in seen and cursor['parent'] > 1
            seen.add(cursor['pid'])
            cursor = self.guard.process_record(cursor['parent'])
        assert cursor['start'] == common['start']
        controls = json.loads((self.root / 'controls.json').read_text())
        timeout = next(row for row in controls if row['role'] == 'outer-timeout')
        self.check_process(timeout)
        assert digest(self.pair['timeout']) == self.pair['timeout_sha256']
        assert timeout['argv'] == self.pilot.outer_command(self.pair, self.phase)[2:]
        outer_end = int(timeout['start']) / os.sysconf('SC_CLK_TCK') + 1798
        end = min(authority['echeance'], outer_end)
        remaining = end - 15 - time.monotonic()
        assert remaining > 0, 'PC publication deadline'
        return {'state': 'active', 'remaining': remaining, 'phase_remaining': end - time.monotonic(),
            'coordinator': launch['coordinator']}

    def stage(self, kind):
        state = self.active()
        assert kind in ['build', 'runtime']
        assert not (self.root / (kind + '-peer-ack.json')).exists(), 'stage already acknowledged'
        if kind == 'runtime':
            assert (self.root / 'build-peer-ack.json').exists()
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
        hashes = {name: digest(path) for name, path in files.items()}
        after = self.active()
        state.update(remaining=after['remaining'], phase_remaining=after['phase_remaining'],
            stage=kind, ledger=ledger, files=hashes)
        return state, files

    def snapshot(self):
        self.verify()
        if not (self.root / 'relay-launch.json').exists():
            return {'state': 'waiting'}
        self.launcher()
        if not (self.root / 'authority-path.json').exists():
            return {'state': 'waiting'}
        for kind in ['build', 'runtime']:
            if (self.root / (kind + '-ledger.json')).exists() and not (self.root / (kind + '-peer-ack.json')).exists():
                return self.stage(kind)[0]
        state = self.active()
        state['stage'] = 'running-test' if (self.root / 'runtime-peer-ack.json').exists() else 'waiting-ledger'
        return state

    def export(self, kind, name, out):
        state, files = self.stage(kind)
        assert name in files
        path = files[name]
        before = time.monotonic()
        with path.open('rb') as stream:
            while chunk := stream.read(1 << 20):
                assert time.monotonic() - before < min(30, state['remaining'])
                assert not any((self.root / name).exists() for name in ['first-red.json', 'relay-abort.json', 'classification.json'])
                out.write(chunk)
            out.flush()
        fresh, _ = self.stage(kind)
        assert fresh['files'] == state['files'], 'exported file changed'

    def acknowledge(self, kind, ack):
        state, _ = self.stage(kind)
        assert ack['kind'] == kind and ack['phase'] == self.phase and ack['host'] == self.pair['peer_host']
        assert ack['prepared_sha256'] == self.pin and ack['coordinator'] == state['coordinator']
        names = ['ledger', 'elf'] if kind == 'build' else ['ledger', 'all', 'ignored']
        assert ack['peer_files'] == {name: str(self.root / 'relay-local' / kind / name) for name in names}
        assert ack['ledger_sha256'] == state['files']['ledger']
        assert ack['elf_sha256'] == state['ledger']['elf_sha256'] and ack['copied_and_rehashed'] is True
        def before_link():
            fresh, _ = self.stage(kind)
            assert fresh['files'] == state['files'] and fresh['coordinator'] == state['coordinator']
        atomic_json(self.root / (kind + '-peer-ack.json'), ack, before_link)
        # Le pilote vérifie aussi son END après observation du fichier ACK.
        self.active()
        return {'accepted': True}

    def abort(self, reason):
        self.active()
        self.pilot.first_red(self.root, RuntimeError('relay: ' + reason))
        return {'refused': True}

    def launch(self, allocation, ready):
        self.verify()
        assert allocation and not (self.root / 'allocation.json').exists()
        assert not (self.root / 'scope').exists() and not (self.root / 'relay-launch.json').exists()
        assert ready['host'] == self.pair['peer_host'] and ready['identity']['uid'] == os.getuid()
        assert ready['prepared_sha256'] == self.pin and ready['phase'] == self.phase
        assert ready['script_sha256'] == self.pair['controllers']['relay.py']
        assert ready['identity']['pid'] > 1 and ready['identity']['start'].isdigit()
        assert ready['identity']['argv'][:3] == [sys.executable, '-B', str(self.control / 'relay.py')]
        assert digest(self.control / 'relay.py') == ready['script_sha256']
        expected = self.guard.process_record(os.getpid())
        command = [sys.executable, '-B', str(self.control / 'pilot.py'), 'run', '--proofs',
                   self.pair['proofs'], '--phase', self.phase, '--allocation', allocation]
        expected['argv'] = command
        atomic_json(self.root / 'relay-launch.json', {'identity': expected,
            'coordinator': ready, 'prepared_sha256': self.pin})
        os.chdir(self.pair['proofs'])
        os.execve(sys.executable, command, dict(os.environ, RUST_PROBE_TOOL_ROOT=self.pair['tool_root']))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['launch', 'snapshot', 'export', 'ack', 'abort'])
    parser.add_argument('--proofs', type=Path, required=True)
    parser.add_argument('--pin', required=True)
    parser.add_argument('--phase', choices=['baseline', 'candidate'], required=True)
    parser.add_argument('--allocation')
    parser.add_argument('--kind', choices=['build', 'runtime'])
    parser.add_argument('--file', choices=['ledger', 'elf', 'all', 'ignored'])
    args = parser.parse_args()
    pair_path = args.proofs / 'prepared.json'
    assert digest(pair_path) == args.pin
    pair = json.loads(pair_path.read_text())
    os.chdir(args.proofs)
    gate = Gate(pair, args.pin, args.phase)
    if args.action == 'launch':
        gate.launch(args.allocation, json.load(sys.stdin))
    elif args.action == 'snapshot':
        print(json.dumps(gate.snapshot()))
    elif args.action == 'export':
        gate.export(args.kind, args.file, sys.stdout.buffer)
    elif args.action == 'ack':
        print(json.dumps(gate.acknowledge(args.kind, json.load(sys.stdin))))
    else:
        print(json.dumps(gate.abort(json.load(sys.stdin)['reason'])))


if __name__ == '__main__':
    main()
