#!/usr/bin/env python3
"""Boucles et gardes extraites de D45 ; premier déplacement à comportement constant.

Ce module ne livre pas encore le plan quatre publications ni WindowEvent.
Les politiques de ledger et les entrées de source restent dans les adaptateurs.
"""
import argparse
import hashlib
import importlib.util
import json
import math
import re
from collections.abc import Mapping
from dataclasses import dataclass
from types import MappingProxyType
import os
from pathlib import Path
import shlex
import signal
import socket
import subprocess
import sys
import tempfile
import time


def _json_plain(value):
    if isinstance(value, Mapping):
        return {key: _json_plain(item) for key, item in value.items()}
    if isinstance(value, tuple):
        return [_json_plain(item) for item in value]
    return value


def _json_frozen(value):
    if isinstance(value, Mapping):
        assert all(isinstance(key, str) for key in value)
        return MappingProxyType({key: _json_frozen(item) for key, item in value.items()})
    if isinstance(value, (list, tuple)):
        return tuple(_json_frozen(item) for item in value)
    assert value is None or isinstance(value, (str, bool, int, float)), 'non-JSON plan value'
    if isinstance(value, float):
        assert math.isfinite(value), 'non-finite plan value'
    return value


def canonical_sha(value):
    raw = json.dumps(_json_plain(value), sort_keys=True, separators=(',', ':'),
                     ensure_ascii=False, allow_nan=False).encode('utf-8')
    return hashlib.sha256(raw).hexdigest()


def safe_id(value):
    assert isinstance(value, str) and re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9._-]{0,95}', value)
    assert value not in ['.', '..']
    return value


def require_sha(value):
    assert isinstance(value, str) and re.fullmatch('[0-9a-f]{64}', value), 'invalid SHA256'
    return value


@dataclass(frozen=True)
class StageSpec:
    label: str
    ledger_file: str
    files: Mapping
    executables: Mapping
    source_contract: Mapping
    window_seconds: float | None

    @property
    def binding_sha256(self):
        # Identité du CONTRAT attendu, jamais SHA d'un patch ou fait observé.
        return canonical_sha(self.source_contract)


@dataclass(frozen=True)
class PublicationPlan:
    protocol_version: int
    execution_id: str
    phase_id: str
    stages: tuple
    sha256: str
    value: Mapping

    def stage(self, label):
        found = [spec for spec in self.stages if spec.label == label]
        assert len(found) == 1, 'undeclared publication label'
        return found[0]

    def position(self, label):
        return tuple(spec.label for spec in self.stages).index(label)


def validate_plan(value):
    if isinstance(value, PublicationPlan):
        return value
    value = _json_frozen(value)
    assert set(value) == {'protocol_version', 'execution_id', 'phase_id', 'stages'}
    assert value['protocol_version'] == 1
    safe_id(value['execution_id'])
    safe_id(value['phase_id'])
    assert value['stages'], 'empty publication plan'
    stages, previous = [], {}
    for raw in value['stages']:
        assert set(raw) == {'label', 'ledger_file', 'files', 'executables', 'source_contract', 'window_seconds'}
        label = safe_id(raw['label'])
        assert label not in previous, 'repeated publication label'
        files = raw['files']
        assert isinstance(files, Mapping) and files
        for name, role in files.items():
            safe_id(name)
            assert role in ['ledger', 'data', 'executable']
        ledger = safe_id(raw['ledger_file'])
        assert files.get(ledger) == 'ledger'
        assert sum(role == 'ledger' for role in files.values()) == 1
        assert isinstance(raw['source_contract'], Mapping) and raw['source_contract']
        window = raw['window_seconds']
        assert window is None or isinstance(window, (int, float)) and not isinstance(window, bool) and math.isfinite(window) and window > 0
        refs = raw['executables']
        assert isinstance(refs, Mapping)
        for artifact, ref in refs.items():
            safe_id(artifact)
            assert set(ref) == {'stage', 'file'}
            target_label, file_id = safe_id(ref['stage']), safe_id(ref['file'])
            target_files = files if target_label == label else previous.get(target_label)
            assert target_files is not None, 'reference to a future or unknown publication'
            assert target_files.get(file_id) == 'executable', 'reference to non-executable payload'
        declared_execs = {name for name, role in files.items() if role == 'executable'}
        used_execs = {ref['file'] for ref in refs.values() if ref['stage'] == label}
        assert declared_execs == used_execs, 'executable without a logical identity'
        stages.append(StageSpec(label, ledger, files, refs, raw['source_contract'], window))
        previous[label] = files
    return PublicationPlan(1, value['execution_id'], value['phase_id'], tuple(stages), canonical_sha(value), value)


def context_header(plan, prepared_sha256, label):
    spec = plan.stage(label)
    return {'protocol_version': plan.protocol_version, 'execution_id': plan.execution_id,
        'phase_id': plan.phase_id, 'sequence': plan.position(label), 'stage': label,
        'prepared_sha256': require_sha(prepared_sha256), 'plan_sha256': plan.sha256,
        'binding_sha256': spec.binding_sha256}


def digest(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def atomic_json(path, value, before_link=lambda: None):
    path = Path(path)
    fd, pending = tempfile.mkstemp(prefix='.' + path.name + '-', dir=path.parent)
    try:
        with os.fdopen(fd, 'w') as stream:
            json.dump(value, stream, indent=2)
            stream.flush()
            os.fsync(stream.fileno())
        before_link()
        os.link(pending, path)
        directory = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY)
        try:
            os.fsync(directory)
        finally:
            os.close(directory)
    finally:
        os.unlink(pending)


def publish_result(path, record, deadline, clock=time.monotonic):
    # Primitive TTY256 : horloge fraîche après open puis fsync, résultat du RELAIS seul.
    with Path(path).open('x') as stream:
        if clock() >= deadline:
            record.update(verdict='ROUGE', error='relay publication already late')
        json.dump(record, stream, indent=2)
        stream.flush()
        os.fsync(stream.fileno())
        if clock() >= deadline:
            record.update(verdict='ROUGE', error='relay publication fsync crossed deadline')
            stream.seek(0)
            stream.truncate()
            json.dump(record, stream, indent=2)
            stream.flush()
            os.fsync(stream.fileno())
    return record


class Relay:
    """Le même moteur pour SSH réel et témoins : pas de branche fake/skip."""
    def __init__(self, transport, root, clock=time.monotonic, sleep=time.sleep,
                 queue_budget=300, transfer_cap=30, control_cap=5, stop_reserve=5, hard_end=None):
        self.transport, self.root, self.clock, self.sleep = transport, Path(root), clock, sleep
        self.queue_budget, self.transfer_cap, self.control_cap = queue_budget, transfer_cap, control_cap
        self.stop_reserve = stop_reserve
        assert 0 < stop_reserve <= control_cap < queue_budget
        self.deadline = None
        self.phase_deadline = None
        started = clock()
        self.queue_end = started + queue_budget
        self.hard_end = hard_end if hard_end is not None else started + queue_budget + 1798
        self.stage = None

    def cap(self, cap):
        admission_end = self.deadline if self.deadline is not None else self.queue_end - self.stop_reserve
        end = min(admission_end, self.hard_end - self.stop_reserve)
        remaining = end - self.clock()
        if remaining <= 0:
            raise TimeoutError('relay deadline')
        return min(cap, remaining)

    def cleanup_cap(self):
        # Réserve DANS la borne connue, jamais cap() de l'opération déjà expirée.
        stop_end = self.phase_deadline if self.phase_deadline is not None else self.queue_end
        remaining = min(stop_end, self.hard_end) - self.clock()
        if remaining <= 0:
            raise TimeoutError('cleanup reserve exhausted; remote quiescence UNKNOWN')
        return min(self.control_cap, remaining)

    def refresh(self, expected=None):
        before = self.clock()
        state = self.transport.snapshot(self.cap(self.control_cap))
        if state['state'] == 'waiting':
            assert expected is None, 'active phase disappeared'
            assert self.deadline is None, 'active phase regressed to queued'
            if 'queue_remaining' in state:
                self.queue_end = min(self.queue_end, before + state['queue_remaining'])
            self.cap(self.control_cap)
            return state
        assert state['state'] in ['starting', 'active'], 'phase is no longer active'
        # Durée PC ancrée AVANT le RPC : sa latence ne rallonge jamais le délai.
        end = before + state['remaining']
        self.deadline = min(self.deadline, end) if self.deadline is not None else end
        phase_end = before + state.get('phase_remaining', state['remaining'])
        self.phase_deadline = min(self.phase_deadline, phase_end) if self.phase_deadline is not None else phase_end
        self.cap(self.control_cap)
        if expected is not None:
            assert state['stage'] == expected, 'publication stage changed'
        return state

    def publish(self, kind, state):
        files = ['ledger', 'elf'] if kind == 'build' else ['ledger', 'all', 'ignored']
        folder = self.root / kind
        folder.mkdir(mode=0o700)
        for name in files:
            current = self.refresh(kind)
            assert current['files'] == state['files'] and current['ledger'] == state['ledger']
            pending = folder / (name + '.pending')
            actual = self.transport.fetch(kind, name, pending, self.cap(self.transfer_cap))
            with pending.open('rb') as stream:
                os.fsync(stream.fileno())
            assert actual == digest(pending) == current['files'][name], 'copied file hash differs'
            if name == 'ledger':
                assert json.loads(pending.read_text()) == current['ledger']
            elif name == 'elf':
                assert actual == current['ledger']['elf_sha256']
            fresh = self.refresh(kind)
            assert fresh['files'] == current['files'], 'payload changed before local publication'
            self.cap(self.control_cap)
            if name == 'elf':
                pending.chmod(0o500)
            os.link(pending, folder / name)
            directory = os.open(folder, os.O_RDONLY | os.O_DIRECTORY)
            try:
                os.fsync(directory)
            finally:
                os.close(directory)
            pending.unlink()
        if kind == 'runtime':
            assert digest(folder / 'all') == state['ledger']['inventory_sha256']
            assert digest(folder / 'ignored') == state['ledger']['ignored_inventory_sha256']
            assert digest(self.root / 'build/elf') == state['ledger']['elf_sha256']
        fresh = self.refresh(kind)
        assert fresh['files'] == state['files'] and fresh['ledger'] == state['ledger']
        ack = {'kind': kind, 'ledger_sha256': state['files']['ledger'],
               'elf_sha256': state['ledger']['elf_sha256'], 'copied_and_rehashed': True}
        self.transport.publish_ack(kind, ack, self.cap(self.control_cap))
        atomic_json(folder / 'ack-accepted.json', ack, lambda: self.cap(self.control_cap))

    def run(self):
        completed = set()
        stopped = False
        try:
            self.transport.ready()
            self.transport.start_phase()
            while len(completed) != 2:
                if not self.transport.running():
                    raise RuntimeError('phase exited before both publications')
                state = self.refresh()
                stage = state.get('stage')
                if state['state'] == 'active' and stage in ['build', 'runtime']:
                    assert stage not in completed
                    assert stage == 'build' or completed == {'build'}, 'runtime before build'
                    self.publish(stage, state)
                    completed.add(stage)
                else:
                    self.sleep(min(.1, self.cap(.1)))
            remaining = min(self.phase_deadline, self.hard_end) - self.stop_reserve - self.clock()
            if remaining <= 0:
                raise TimeoutError('phase completion deadline')
            rc = self.transport.finish(remaining)
            if rc:
                raise RuntimeError('phase returned ' + str(rc))
            # La récolte/fermeture des enfants et des logs précède le verdict final.
            self.transport.stop()
            stopped = True
            if self.clock() >= self.phase_deadline:
                raise TimeoutError('phase deadline crossed during finish or cleanup')
            result = publish_result(self.root / 'relay-result.json', {'verdict': 'VERT',
                'phase_ssh_rc': rc, 'native_verdict_unchanged': True}, self.phase_deadline, self.clock)
            if result['verdict'] != 'VERT' or self.clock() >= self.phase_deadline:
                # Un close tardif ne laisse pas un fichier vert comme dernier état du relais.
                result.update(verdict='ROUGE', error='relay final publication deadline')
                with (self.root / 'relay-result.json').open('r+') as stream:
                    json.dump(result, stream, indent=2)
                    stream.truncate()
                    stream.flush()
                    os.fsync(stream.fileno())
                raise TimeoutError('relay final publication deadline')
            return rc
        except BaseException as error:
            try:
                atomic_json(self.root / 'first-red.json', {'reason': repr(error), 'observed': self.clock()})
            except FileExistsError:
                pass
            # Le refus distant est borné par le temps restant déjà reçu, pas un nouveau budget.
            try:
                if stopped:
                    # finish rc0 puis récolte déjà achevés : ne pas rouvrir la phase close.
                    atomic_json(self.root / 'cleanup-not-required.json', {'phase_ssh_rc': 0,
                        'reason': 'phase and local children already collected before relay refusal',
                        'native_verdict_unchanged': True, 'observed': self.clock()})
                else:
                    self.transport.abort(repr(error), self.cleanup_cap())
            except BaseException as stop_error:
                atomic_json(self.root / 'cleanup-refused.json', {'remote_quiet': 'UNKNOWN',
                    'reason': repr(stop_error), 'original_error': repr(error),
                    'native_verdict_unchanged': True, 'observed': self.clock()})
            raise
        finally:
            if not stopped:
                self.transport.stop()


class SSHTransport:
    def __init__(self, pair, pin, phase, allocation, host, root, *, entry_file=None):
        self.pair, self.pin, self.phase, self.allocation = pair, pin, phase, allocation
        self.root, self.host, self.child, self.counter = Path(root), host, None, 0
        self.active_call = None
        self.handles = []
        self.ready_record = None
        self.remote_cancelled = False
        self.control = Path(pair['control'])
        self.entry_file = Path(entry_file) if entry_file is not None else self.control / 'relay.py'
        spec = importlib.util.spec_from_file_location('relay_physical_guard', self.control / 'physical_guard.py')
        self.guard = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.guard)

    def command(self, action, *extra, cap=None):
        remote = [sys.executable, '-B', str(self.control / 'gate.py'), action,
                  '--proofs', self.pair['proofs'], '--phase', self.phase, '--pin', self.pin, *extra]
        if action == 'launch':
            remote = [self.pair['timeout'], '--signal=TERM', '--kill-after=2s', '2098s', *remote]
        elif cap is not None:
            assert 0 < cap <= 30
            remote = [self.pair['timeout'], '--signal=TERM', '--kill-after=1s', str(cap) + 's', *remote]
        # Un seul argv distant quoté ; pas d'interpolation de shell ni de parent argv W/T.
        return ['ssh', '-o', 'BatchMode=yes', '-o', 'ConnectTimeout=5', '-o', 'ControlMaster=no',
                '-o', 'ControlPath=none', self.host, 'exec ' + shlex.join(remote)]

    def paths(self, label, argv):
        self.counter += 1
        stem = self.root / (str(self.counter).zfill(3) + '-' + label)
        atomic_json(stem.with_suffix('.command.json'), {'argv': argv, 'started': time.monotonic()})
        return stem

    def call(self, action, timeout, extra=(), payload=None, destination=None):
        self.check_coordinator()
        argv = self.command(action, *extra, cap=max(.001, timeout - .1))
        stem = self.paths(action, argv)
        output = Path(destination) if destination else stem.with_suffix('.stdout')
        with output.open('xb') as out, stem.with_suffix('.stderr').open('xb') as err:
            child = subprocess.Popen(argv, stdin=subprocess.PIPE if payload else subprocess.DEVNULL,
                stdout=out, stderr=err, start_new_session=False)
            self.active_call = child
            try:
                child.communicate(json.dumps(payload).encode() if payload else None, timeout=timeout)
                atomic_json(stem.with_suffix('.result.json'), {'rc': child.returncode, 'finished': time.monotonic()})
                if child.returncode:
                    raise RuntimeError('SSH gate ' + action + ' returned ' + str(child.returncode))
            finally:
                self.reap(child)
                if not stem.with_suffix('.result.json').exists():
                    atomic_json(stem.with_suffix('.result.json'), {'rc': child.returncode, 'finished': time.monotonic(),
                        'interrupted': True})
                self.active_call = None
        return None if destination else json.loads(output.read_text())

    @staticmethod
    def reap(child):
        # L'objet Popen non récolté identifie cet enfant ; aucun PID découvert par nom.
        if child.poll() is None:
            child.terminate()
            try:
                child.wait(timeout=.5)
            except subprocess.TimeoutExpired:
                child.kill()
                child.wait(timeout=.5)

    def ready(self):
        identity = self.guard.process_record(os.getpid())
        self.ready_record = {'host': socket.gethostname(), 'identity': identity,
            'script_sha256': digest(self.entry_file), 'prepared_sha256': self.pin, 'phase': self.phase}
        atomic_json(self.root / 'ready.json', self.ready_record)

    def check_coordinator(self):
        assert digest(self.entry_file) == self.pair['controllers']['relay.py']
        assert Path(__file__).resolve() == self.control / 'publication_relay.py'
        assert digest(__file__) == self.pair['controllers']['publication_relay.py']
        assert digest(Path(self.pair['proofs']) / 'prepared.json') == self.pin
        actual = self.guard.process_record(os.getpid())
        expected = self.ready_record['identity']
        assert actual['state'] != 'Z' and all(actual[k] == expected[k] for k in
            ['pid', 'start', 'uid', 'exe', 'argv', 'cgroup'])

    def start_phase(self):
        self.check_coordinator()
        argv = self.command('launch', '--allocation', self.allocation)
        stem = self.paths('launch', argv)
        out, err = stem.with_suffix('.stdout').open('xb'), stem.with_suffix('.stderr').open('xb')
        self.handles.extend([out, err])
        self.child = subprocess.Popen(argv, stdin=subprocess.PIPE, stdout=out, stderr=err, start_new_session=False)
        self.child.stdin.write(json.dumps(self.ready_record).encode())
        self.child.stdin.close()

    def snapshot(self, timeout):
        return self.call('snapshot', timeout)

    def fetch(self, kind, name, destination, timeout):
        self.call('export', timeout, ('--kind', kind, '--file', name), destination=destination)
        return digest(destination)

    def publish_ack(self, kind, ack, timeout):
        names = ['ledger', 'elf'] if kind == 'build' else ['ledger', 'all', 'ignored']
        ack['peer_files'] = {name: str(self.root / kind / name) for name in names}
        ack.update(host=socket.gethostname(), phase=self.phase, prepared_sha256=self.pin,
            coordinator=self.ready_record)
        result = self.call('ack', timeout, ('--kind', kind), ack)
        assert result['accepted'] is True

    def running(self):
        return self.child is not None and self.child.poll() is None

    def finish(self, timeout):
        rc = self.child.wait(timeout=timeout)
        atomic_json(self.root / 'phase-ssh-result.json', {'rc': rc, 'finished': time.monotonic()})
        return rc

    def abort(self, reason, timeout):
        if self.child is not None:
            result = self.call('abort', timeout, payload={'reason': reason})
            assert result['launcher_absent'] is True and not result['unknown']
            self.remote_cancelled = True
            atomic_json(self.root / 'remote-cancel-attested.json', result)

    def stop(self):
        if self.active_call is not None:
            self.reap(self.active_call)
        if self.child is not None:
            self.reap(self.child)
        for handle in self.handles:
            handle.close()



class GateBase:
    def __init__(self, pair, pin, phase, *, pilot, entry_file):
        self.pair, self.pin, self.phase = pair, pin, phase
        self.root = Path(pair['proofs']) / phase
        self.control = Path(pair['control'])
        self.pilot = pilot
        self.entry_file = Path(entry_file)
        self.provider = None
        self.guard = self.pilot.physical

    def verify_contract(self):
        assert socket.gethostname().lower() == 'luciepc'
        assert digest(Path(self.pair['proofs']) / 'prepared.json') == self.pin
        assert self.entry_file.resolve() == self.control / 'gate.py'
        assert Path(__file__).resolve() == self.control / 'publication_relay.py'
        assert digest(__file__) == self.pair['controllers']['publication_relay.py']
        assert digest(self.entry_file) == self.pair['controllers']['gate.py']
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
        ledger, files = self.provider.read(kind)
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



def relay_main(entry_file):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--proofs', type=Path, required=True)
    parser.add_argument('--phase', choices=['baseline', 'candidate'], required=True)
    parser.add_argument('--allocation', required=True)
    parser.add_argument('--host', default='luciepc')
    args = parser.parse_args()
    path = args.proofs / 'prepared.json'
    pair, pin = json.loads(path.read_text()), digest(path)
    assert args.host == 'luciepc'
    assert socket.gethostname() == pair['peer_host']
    assert Path(entry_file).resolve() == Path(pair['control']) / 'relay.py'
    assert Path(__file__).resolve() == Path(pair['control']) / 'publication_relay.py'
    assert digest(__file__) == pair['controllers']['publication_relay.py']
    assert digest(entry_file) == pair['controllers']['relay.py']
    assert pair['relay'] == {'queue_budget': 300, 'transfer_cap': 30, 'control_cap': 5, 'stop_reserve': 5}
    os.chdir(args.proofs)
    # Vrai timeout local : groupe privé hérité par tous nos SSH, kill2 physique.
    peer_timeout = pair['peer_timeout']
    assert digest(peer_timeout['path']) == peer_timeout['sha256']
    local_outer = [peer_timeout['path'], '--signal=TERM', '--kill-after=2s', '2098s',
        sys.executable, '-B', str(Path(entry_file).resolve()), *sys.argv[1:]]
    spec = importlib.util.spec_from_file_location('local_outer_guard', Path(pair['control']) / 'physical_guard.py')
    guard = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(guard)
    parent = guard.process_record(os.getppid())
    if parent['exe'] != peer_timeout['path'] or parent['argv'] != local_outer:
        os.execve(peer_timeout['path'], local_outer, os.environ.copy())
    assert parent['uid'] == os.getuid()
    root = args.proofs / args.phase / 'relay-local'
    root.mkdir(mode=0o700)
    atomic_json(root / 'local-hard-supervision.json', {'identity': parent, 'hardcap': 2100,
        'queue_budget': 300, 'phase_budget': 1800, 'timeout_sha256': peer_timeout['sha256']})
    transport = SSHTransport(pair, pin, args.phase, args.allocation, args.host, root, entry_file=entry_file)
    def stop(sig, frame):
        raise TimeoutError('relay received signal ' + str(sig))
    for sig in [signal.SIGTERM, signal.SIGINT, signal.SIGHUP]:
        signal.signal(sig, stop)
    hard_end = int(parent['start']) / os.sysconf('SC_CLK_TCK') + 2098
    return Relay(transport, root, hard_end=hard_end, **pair['relay']).run()


def gate_main(gate_factory):
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
    gate = gate_factory(pair, args.pin, args.phase)
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

