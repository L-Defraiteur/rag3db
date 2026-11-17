#!/usr/bin/env python3
"""Porte PC : aucun export ou ACK sans phase active vérifiée sur l'horloge PC."""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import socket
import signal
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

    def verify_contract(self):
        assert socket.gethostname().lower() == 'luciepc'
        assert digest(Path(self.pair['proofs']) / 'prepared.json') == self.pin
        assert Path(__file__).resolve() == self.control / 'gate.py'
        assert digest(__file__) == self.pair['controllers']['gate.py']
        self.pilot.verify(self.pair, self.phase)

    def verify(self):
        self.verify_contract()
        for name in ['first-red.json', 'relay-abort.json', 'classification.json']:
            assert not (self.root / name).exists(), 'phase has already refused or ended'

    def check_process(self, expected):
        assert expected['uid'] == os.getuid()
        actual = self.guard.process_record(expected['pid'])
        assert actual['state'] != 'Z'
        assert all(actual[k] == expected[k] for k in ['pid', 'start', 'uid', 'exe', 'argv', 'cgroup'])
        return actual

    def bootstrap(self):
        record = json.loads((self.root / 'relay-bootstrap.json').read_text())
        assert record['prepared_sha256'] == self.pin
        assert record['gate_sha256'] == digest(self.control / 'gate.py') == self.pair['controllers']['gate.py']
        hard = self.check_process(record['hard_timeout'])
        assert hard['exe'] == self.pair['timeout']
        assert digest(hard['exe']) == self.pair['timeout_sha256']
        assert hard['argv'][:4] == [self.pair['timeout'], '--signal=TERM', '--kill-after=2s', '2098s']
        return record

    def launcher(self):
        bootstrap = self.bootstrap()
        record = json.loads((self.root / 'relay-launch.json').read_text())
        self.check_process(record['identity'])
        assert record['launcher_script_sha256'] == digest(self.control / 'pilot.py') == self.pair['driver_sha256']
        assert record['identity']['argv'] == bootstrap['run_argv']
        assert record['identity']['pid'] == bootstrap['identity']['pid']
        assert record['identity']['start'] == bootstrap['identity']['start']
        assert record['prepared_sha256'] == self.pin and record['coordinator'] == bootstrap['coordinator']
        return record

    def waiting_transition(self):
        bootstrap = self.bootstrap()
        observed = self.guard.process_record(bootstrap['identity']['pid'])
        expected = bootstrap['identity']
        assert all(observed[k] == expected[k] for k in ['pid', 'start', 'uid', 'exe', 'cgroup'])
        assert observed['argv'] in [expected['argv'], bootstrap['run_argv']]
        assert observed['state'] != 'Z' and time.monotonic() < bootstrap['transition_end']
        return {'state': 'waiting', 'queue_remaining': bootstrap['queue_end'] - time.monotonic()}

    def starting(self):
        controls = json.loads((self.root / 'controls.json').read_text())
        timeout = next(row for row in controls if row['role'] == 'outer-timeout')
        self.check_process(timeout)
        assert timeout['argv'] == self.pilot.outer_command(self.pair, self.phase)[2:]
        assert int(timeout['start']) / os.sysconf('SC_CLK_TCK') <= self.bootstrap()['queue_end']
        assert timeout['exe'] == self.pair['timeout'] and digest(timeout['exe']) == self.pair['timeout_sha256']
        remaining = int(timeout['start']) / os.sysconf('SC_CLK_TCK') + 1798 - time.monotonic()
        assert remaining > 15
        return {'state': 'starting', 'remaining': remaining - 15,
                'phase_remaining': remaining, 'stage': 'waiting-authority'}

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
            if (self.root / 'relay-bootstrap.json').exists():
                return self.waiting_transition()
            return {'state': 'waiting'}
        self.launcher()
        if not (self.root / 'authority-path.json').exists():
            if (self.root / 'outer-supervision.json').exists():
                return self.starting()
            return {'state': 'waiting', 'queue_remaining': self.bootstrap()['queue_end'] - time.monotonic()}
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

    def role_alive(self, expected, allowed_argv=None):
        try:
            actual = self.guard.process_record(expected['pid'])
        except (FileNotFoundError, ProcessLookupError):
            return False
        if actual['start'] != expected['start'] or actual['state'] == 'Z':
            return False
        assert expected['uid'] == actual['uid'] == os.getuid()
        assert all(actual[k] == expected[k] for k in ['exe', 'cgroup'])
        assert actual['argv'] in (allowed_argv or [expected['argv']])
        return True

    def signal_role(self, expected, sig, allowed_argv=None):
        if not self.role_alive(expected, allowed_argv):
            return
        try:
            fd = os.pidfd_open(expected['pid'])
            try:
                if self.role_alive(expected, allowed_argv):
                    signal.pidfd_send_signal(fd, sig)
            finally:
                os.close(fd)
        except ProcessLookupError:
            pass

    def final_roles(self, roles):
        # Aucun garde temporel autour du constat final : non observé != absent.
        alive, unknown, observations = [], [], []
        for role, identity, argv in roles:
            row = {'role': role, 'pid': identity['pid'], 'start': identity['start'], 'uid': identity['uid']}
            try:
                present = self.role_alive(identity, argv)
                row['status'] = 'alive' if present else 'absent'
                if present:
                    alive.append(role)
            except (OSError, AssertionError) as error:
                row.update(status='UNKNOWN', error=repr(error))
                unknown.append(role + ': ' + repr(error))
            row['observed'] = time.monotonic()
            observations.append(row)
        return alive, unknown, observations

    def cancel_launcher(self, reason):
        # Annulation attribuée, y compris en file : aucun appel à active().
        self.verify_contract()
        bootstrap = json.loads((self.root / 'relay-bootstrap.json').read_text())
        assert bootstrap['prepared_sha256'] == self.pin
        assert bootstrap['gate_sha256'] == digest(self.control / 'gate.py') == self.pair['controllers']['gate.py']
        hard = bootstrap['hard_timeout']
        assert hard['exe'] == self.pair['timeout'] and digest(hard['exe']) == self.pair['timeout_sha256']
        assert hard['argv'][:4] == [self.pair['timeout'], '--signal=TERM', '--kill-after=2s', '2098s']
        allowed = [bootstrap['identity']['argv'], bootstrap['run_argv']]
        roles = [('remote-hard-timeout', hard, None), ('launcher', bootstrap['identity'], allowed)]
        info_path = self.root / 'authority-path.json'
        if info_path.exists():
            info = json.loads(info_path.read_text())
            assert info['driver_script_sha256'] == digest(self.control / 'pilot.py') == self.pair['driver_sha256']
            assert info['supervisor_identity']['argv'] == json.loads((self.root / 'common.command.json').read_text())
            roles.append(('common', info['supervisor_identity'], None))
        elif (self.root / 'scope').exists():
            # Fenêtre Common lancé mais pilote pas encore au rendez-vous authority-path.
            receipts = list((self.root / 'scope').glob('validation-*/autorite.json'))
            assert len(receipts) <= 1, 'ambiguous Common cancellation'
            if receipts:
                authority = json.loads(receipts[0].read_text())
                assert authority['script_sha256'] == self.pair['controllers']['confiner_validation.py']
                try:
                    common = self.guard.process_record(authority['pid'])
                except (FileNotFoundError, ProcessLookupError):
                    common = None
                if common and common['start'] == authority['debut']:
                    assert common['uid'] == os.getuid()
                    assert common['argv'] == json.loads((self.root / 'common.command.json').read_text())
                    roles.append(('common', common, None))
        controls_path = self.root / 'controls.json'
        if controls_path.exists():
            phase_outer = next(row for row in json.loads(controls_path.read_text()) if row['role'] == 'outer-timeout')
            assert phase_outer['argv'] == self.pilot.outer_command(self.pair, self.phase)[2:]
            roles.append(('phase-timeout', phase_outer, None))
        closed = (self.root / 'classification.json').exists()
        if not closed:
            self.pilot.first_red(self.root, RuntimeError('relay: ' + reason))
        end = time.monotonic() + 3.4
        stop_errors = []
        try:
            # Laisser au Common réellement observé sa fermeture canonique avant le hard launcher.
            common_roles = [row for row in roles if row[0] == 'common']
            for _, identity, argv in common_roles:
                self.signal_role(identity, signal.SIGTERM, argv)
            common_end = min(end, time.monotonic() + 1.7)
            while any(self.role_alive(identity, argv) for _, identity, argv in common_roles) and time.monotonic() < common_end:
                time.sleep(.025)
            for role, identity, argv in roles:
                if role in ['phase-timeout', 'remote-hard-timeout']:
                    self.signal_role(identity, signal.SIGTERM, argv)
            while time.monotonic() < end:
                if not any(self.role_alive(identity, argv) for _, identity, argv in roles):
                    break
                time.sleep(.025)
        except (OSError, AssertionError) as error:
            stop_errors.append('signaling not attested: ' + repr(error))
        # Relevé frais obligatoire même si aucune boucle n'a pu faire un tour.
        alive, unknown, observations = self.final_roles(roles)
        unknown.extend(stop_errors)
        scope_quiet = None
        scopes = list((self.root / 'scope').glob('validation-*'))
        if len(scopes) > 1:
            unknown.append('ambiguous Common final scopes')
        for scope in scopes:
            if (scope / 'verdict.json').exists():
                scope_quiet = self.guard.owned_quiet(scope)
                verdict = json.loads((scope / 'verdict.json').read_text())
                authority = json.loads((scope / 'autorite.json').read_text())
                if authority['etat'] != 'termine' or scope_quiet['alive'] or scope_quiet['unknown'] or verdict['restes'] or not verdict['nettoye'] or Path(verdict['prive']).exists():
                    unknown.append('Common owned or private scope not quiet')
            else:
                unknown.append('Common final verdict unavailable')
        result = {'launcher_absent': not alive and not unknown, 'alive_roles': alive, 'unknown': unknown,
            'role_observations': observations,
            'scope_quiet': scope_quiet, 'native_verdict_unchanged': True, 'observed': time.monotonic()}
        atomic_json(self.root / 'relay-cancel.json', result)
        assert result['launcher_absent'] and not unknown, 'remote cancellation not attested quiet'
        return result

    def abort(self, reason):
        return self.cancel_launcher(reason)

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
        identity = self.guard.process_record(os.getpid())
        hard = self.guard.process_record(os.getppid())
        expected_gate = [sys.executable, '-B', str(self.control / 'gate.py'), 'launch',
            '--proofs', self.pair['proofs'], '--phase', self.phase, '--pin', self.pin, '--allocation', allocation]
        assert hard['uid'] == os.getuid() and hard['exe'] == self.pair['timeout']
        assert digest(hard['exe']) == self.pair['timeout_sha256']
        assert hard['argv'] == [self.pair['timeout'], '--signal=TERM', '--kill-after=2s', '2098s', *expected_gate]
        command = [sys.executable, '-B', str(self.control / 'pilot.py'), 'run', '--proofs',
                   self.pair['proofs'], '--phase', self.phase, '--allocation', allocation]
        now = time.monotonic()
        # Seule l'identité RÉELLE Gate est publiée ; run_argv est explicitement un projet.
        atomic_json(self.root / 'relay-bootstrap.json', {'identity': identity, 'run_argv': command,
            'coordinator': ready, 'prepared_sha256': self.pin, 'hard_timeout': hard,
            'gate_sha256': digest(self.control / 'gate.py'), 'transition_end': now + 5, 'queue_end': now + 300})
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
