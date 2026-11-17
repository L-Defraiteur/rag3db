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
        value = value.value
    value = _json_frozen(value)
    assert set(value) == {'protocol_version', 'execution_id', 'phase_id', 'stages'}
    assert type(value['protocol_version']) is int and value['protocol_version'] == 1
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


def publication_limits(pair):
    limits = pair['publication_limits']
    expected = {'whole_soft', 'whole_kill', 'queue', 'phase_soft', 'phase_kill',
                'common_max', 'common_margin', 'operation_grace', 'stop_reserve', 'control_cap', 'copy_cap'}
    assert set(limits) == expected
    for value in limits.values():
        assert isinstance(value, (int, float)) and not isinstance(value, bool) and math.isfinite(value) and value > 0
    assert limits['control_cap'] <= limits['stop_reserve'] < limits['queue'] < limits['whole_soft']
    assert limits['operation_grace'] >= limits['stop_reserve']
    assert limits['common_max'] < limits['whole_soft'] + limits['whole_kill']
    return _json_frozen(limits)


def seconds_arg(seconds):
    return f'{seconds:g}s'


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
                 queue_budget=300, transfer_cap=30, control_cap=5, stop_reserve=5, hard_end=None, *, plan, prepared_sha256):
        self.transport, self.root, self.clock, self.sleep = transport, Path(root), clock, sleep
        self.queue_budget, self.transfer_cap, self.control_cap = queue_budget, transfer_cap, control_cap
        self.stop_reserve = stop_reserve
        assert 0 < control_cap <= stop_reserve < queue_budget
        self.deadline = None
        self.phase_deadline = None
        started = clock()
        self.queue_end = started + queue_budget
        self.hard_end = hard_end if hard_end is not None else started + queue_budget + 1798
        self.stage = None
        self.plan = validate_plan(plan)
        self.prepared_sha256 = require_sha(prepared_sha256)
        self.acknowledged = {}
        self.window_ends = {}

    def cap(self, cap):
        admission_end = self.deadline if self.deadline is not None else self.queue_end - self.stop_reserve
        end = min(admission_end, self.hard_end - self.stop_reserve)
        if self.stage in self.window_ends:
            end = min(end, self.window_ends[self.stage])
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
        for field in ['remaining', 'phase_remaining', 'queue_remaining']:
            if field in state:
                value = state[field]
                assert isinstance(value, (int, float)) and not isinstance(value, bool) and math.isfinite(value)
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
        label = state.get('stage')
        if label in {spec.label for spec in self.plan.stages}:
            spec = self.plan.stage(label)
            if spec.window_seconds is not None:
                window_remaining = state['publication_remaining']
                assert isinstance(window_remaining, (int, float)) and math.isfinite(window_remaining)
                window_end = before + window_remaining
                self.window_ends[label] = min(self.window_ends.get(label, window_end), window_end)
        self.cap(self.control_cap)
        if expected is not None:
            assert state['stage'] == expected, 'publication stage changed'
        return state

    def expected_header(self, label):
        return context_header(self.plan, self.prepared_sha256, label)

    def verify_publication(self, label, state):
        spec = self.plan.stage(label)
        header = dict(state['header'])
        assert type(header.get('protocol_version')) is int and type(header.get('sequence')) is int
        event_sha = header.pop('window_event_sha256', None)
        assert header == self.expected_header(label), 'publication identity or expected contract changed'
        assert set(state['files']) == set(spec.files), 'missing or extra publication payload'
        assert set(state['file_sizes']) == set(spec.files)
        for name, sha in state['files'].items():
            require_sha(sha)
            size = state['file_sizes'][name]
            assert isinstance(size, int) and not isinstance(size, bool) and size >= 0
        assert canonical_sha(state['observed_binding']) == require_sha(state['observed_binding_sha256'])
        # Le contrat statique et les faits observés sont distincts, pas des SHA à égaler.
        assert set(state['elf_map']) == set(spec.executables)
        for artifact, declared in spec.executables.items():
            actual = state['elf_map'][artifact]
            assert set(actual) == {'stage', 'file', 'sha256'}
            assert actual['stage'] == declared['stage'] and actual['file'] == declared['file']
            sha = require_sha(actual['sha256'])
            if actual['stage'] == label:
                assert sha == state['files'][actual['file']]
            else:
                assert actual['stage'] in self.acknowledged, 'inherited executable requires prior ACK'
                prior = self.acknowledged[actual['stage']]
                assert sha == prior['files'][actual['file']]
                copied = self.root / actual['stage'] / actual['file']
                assert not copied.is_symlink() and digest(copied) == sha
        if spec.window_seconds is None:
            assert event_sha is None and state.get('window') is None
        else:
            window = state['window']
            assert require_sha(event_sha) == window['event_sha256']
            assert window['ledger_sha256'] == state['files'][spec.ledger_file]
            assert state['publication_remaining'] > 0
        return spec

    def publish(self, kind, state):
        spec = self.plan.stage(kind)
        assert self.plan.position(kind) == len(self.acknowledged), 'publication out of order'
        self.verify_publication(kind, state)
        self.stage = kind
        folder = self.root / kind
        folder.mkdir(mode=0o700)
        for name in spec.files:
            current = self.refresh(kind)
            self.verify_publication(kind, current)
            assert current['header'] == state['header'] and current['files'] == state['files']
            assert current['ledger'] == state['ledger'] and current['elf_map'] == state['elf_map']
            assert current['observed_binding_sha256'] == state['observed_binding_sha256']
            pending = folder / (name + '.pending')
            actual = self.transport.fetch(kind, name, pending, self.cap(self.transfer_cap))
            with pending.open('rb') as stream:
                os.fsync(stream.fileno())
            assert actual == digest(pending) == current['files'][name], 'copied file hash differs'
            assert pending.stat().st_size == current['file_sizes'][name]
            if name == spec.ledger_file:
                assert json.loads(pending.read_text()) == current['ledger']
            fresh = self.refresh(kind)
            self.verify_publication(kind, fresh)
            assert fresh['files'] == current['files'] and fresh['header'] == current['header']
            self.cap(self.control_cap)
            if spec.files[name] == 'executable':
                pending.chmod(0o500)
            os.link(pending, folder / name)
            directory = os.open(folder, os.O_RDONLY | os.O_DIRECTORY)
            try:
                os.fsync(directory)
            finally:
                os.close(directory)
            pending.unlink()
        # Les références antérieures sont rehashées dans verify_publication avant l'ACK.
        fresh = self.refresh(kind)
        self.verify_publication(kind, fresh)
        assert fresh['files'] == state['files'] and fresh['ledger'] == state['ledger']
        assert fresh['header'] == state['header'] and fresh['elf_map'] == state['elf_map']
        ack = {'kind': kind, 'header': _json_plain(state['header']),
               'ledger_sha256': state['files'][spec.ledger_file], 'files': dict(state['files']),
               'file_sizes': dict(state['file_sizes']), 'elf_map': _json_plain(state['elf_map']),
               'observed_binding_sha256': state['observed_binding_sha256'], 'copied_and_rehashed': True}
        if len(state['elf_map']) == 1:
            ack['elf_sha256'] = next(iter(state['elf_map'].values()))['sha256']
        self.transport.publish_ack(kind, ack, self.cap(self.control_cap))
        atomic_json(folder / 'ack-accepted.json', ack, lambda: self.cap(self.control_cap))
        self.acknowledged[kind] = _json_frozen(state)
        self.stage = None

    def run(self):
        completed = set()
        stopped = False
        try:
            self.transport.ready()
            self.transport.start_phase()
            while len(completed) != len(self.plan.stages):
                if not self.transport.running():
                    raise RuntimeError('phase exited before both publications')
                state = self.refresh()
                stage = state.get('stage')
                if state['state'] == 'active' and stage is not None:
                    assert stage not in completed
                    assert stage == self.plan.stages[len(completed)].label, 'undeclared or out-of-order publication'
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
        self.plan = validate_plan(pair['publication_plans'][phase])
        self.prepared_sha256 = require_sha(pin)
        self.entry_file = Path(entry_file) if entry_file is not None else self.control / 'relay.py'
        spec = importlib.util.spec_from_file_location('relay_physical_guard', self.control / 'physical_guard.py')
        self.guard = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.guard)

    def command(self, action, *extra, cap=None):
        remote = [sys.executable, '-B', str(self.control / 'gate.py'), action,
                  '--proofs', self.pair['proofs'], '--phase', self.phase, '--pin', self.pin, *extra]
        if action == 'launch':
            remote = [self.pair['timeout'], '--signal=TERM', '--kill-after=' + seconds_arg(publication_limits(self.pair)['whole_kill']),
                seconds_arg(publication_limits(self.pair)['whole_soft']), *remote]
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
        names = self.plan.stage(kind).files
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



@dataclass(frozen=True)
class ActivePhase:
    execution_id: str
    phase_id: str
    prepared_sha256: str
    plan_sha256: str
    authority_pid: int
    authority_start: str
    authority_cgroup: str
    phase_end_pc: float
    operation_end_pc: float


@dataclass(frozen=True)
class WindowEvidence:
    event_sha256: str
    owner: Mapping
    remaining: float


class GateBase:
    def __init__(self, pair, pin, phase, *, pilot, entry_file):
        self.pair, self.pin, self.phase = pair, pin, phase
        self.root = Path(pair['proofs']) / phase
        self.control = Path(pair['control'])
        self.pilot = pilot
        self.entry_file = Path(entry_file)
        self.provider = None
        self.plan = validate_plan(pair['publication_plans'][phase])
        self.guard = self.pilot.physical

    @property
    def limits(self):
        return publication_limits(self.pair)

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
        assert hard['argv'][:4] == [self.pair['timeout'], '--signal=TERM',
            '--kill-after=' + seconds_arg(self.limits['whole_kill']), seconds_arg(self.limits['whole_soft'])]
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
        remaining = int(timeout['start']) / os.sysconf('SC_CLK_TCK') + self.limits['phase_soft'] - time.monotonic()
        assert remaining > self.limits['operation_grace']
        return {'state': 'starting', 'remaining': remaining - self.limits['operation_grace'],
                'phase_remaining': remaining, 'stage': None}

    def active(self):
        self.verify()
        launch = self.launcher()
        # Le codec de politique lit des DESCRIPTEURS, il n'atteste jamais leur vivacité.
        info = self.pilot.publication_phase_info(self.pair, self.phase)
        receipt = Path(info['path'])
        assert receipt.name == 'autorite.json' and receipt.parent.parent == Path(info['scope_root'])
        assert receipt.resolve(strict=True) == receipt and receipt.stat().st_uid == os.getuid()
        authority = json.loads(receipt.read_text())  # Aucun jeton n'est exporté.
        assert authority['etat'] == 'actif' and not receipt.with_name('verdict.json').exists()
        common_entry = self.control / safe_id(info['common_entry'])
        assert authority['script_sha256'] == self.pair['controllers'][info['common_entry']]
        assert authority['script'] == str(common_entry)
        common = self.check_process(info['supervisor_identity'])
        assert common['pid'] == authority['pid'] and common['start'] == authority['debut']
        assert common['cgroup'].split(':', 2)[-1] == authority['cgroup']
        assert common['argv'] == info['common_argv']
        driver = self.check_process(info['driver_identity'])
        entry = self.control / safe_id(info['driver_entry'])
        assert info['driver_script_sha256'] == digest(entry) == self.pair['controllers'][info['driver_entry']]
        assert driver['argv'] == info['driver_argv']
        cursor, seen = driver, set()
        while cursor['pid'] != common['pid']:
            assert cursor['pid'] not in seen and cursor['parent'] > 1
            seen.add(cursor['pid'])
            cursor = self.guard.process_record(cursor['parent'])
        assert cursor['start'] == common['start']
        controls = json.loads(Path(info['controls_path']).read_text())
        timeout = next(row for row in controls if row['role'] == info['deadline_role'])
        self.check_process(timeout)
        assert digest(self.pair['timeout']) == self.pair['timeout_sha256']
        assert timeout['argv'] == info['deadline_argv']
        outer_end = int(timeout['start']) / os.sysconf('SC_CLK_TCK') + self.limits['phase_soft']
        end = min(authority['echeance'], outer_end)
        assert authority['echeance'] == info['original_common_end'], 'original Common END changed'
        recipe = json.loads(receipt.with_name('recette.json').read_text())
        assert 0 < recipe['delai'] <= self.limits['common_max']
        whole = self.bootstrap()['hard_timeout']
        hard_end = int(whole['start']) / os.sysconf('SC_CLK_TCK') + self.limits['whole_soft'] + self.limits['whole_kill']
        assert end <= hard_end - self.limits['common_margin']
        operation_end = end - self.limits['operation_grace']
        remaining = operation_end - time.monotonic()
        assert remaining > 0, 'PC publication deadline'
        self._active_epoch = {'pid': authority['pid'], 'start': authority['debut'],
            'cgroup': authority['cgroup'], 'phase_end': end, 'operation_end': operation_end}
        return {'state': 'active', 'remaining': remaining, 'phase_remaining': end - time.monotonic(),
            'coordinator': launch['coordinator']}

    def observe_active(self):
        self.active()  # Toutes les gardes natives du noyau, pas un booléen du fournisseur.
        assert self.pair['execution_id'] == self.plan.execution_id
        epoch = self._active_epoch
        return ActivePhase(self.plan.execution_id, self.phase, require_sha(self.pin), self.plan.sha256,
            epoch['pid'], epoch['start'], epoch['cgroup'], epoch['phase_end'], epoch['operation_end'])

    def revalidate_phase(self, phase):
        assert isinstance(phase, ActivePhase)
        current = self.observe_active()
        assert current == phase, 'phase identity or original END changed'
        assert time.monotonic() < current.operation_end_pc, 'operation END expired'
        return current

    def window_spec(self, spec):
        declared = self.plan.stage(spec.label)
        assert declared == spec and spec.window_seconds is not None
        windows = self.pair.get('window_events', {})
        assert spec.label in windows, 'publication window path not approved'
        approved = windows[spec.label]
        assert {'path', 'owner_role', 'event_kind'} <= set(approved)
        safe_id(approved['owner_role'])
        safe_id(approved['event_kind'])
        path = Path(approved['path'])
        assert path.is_absolute() and path.parent.resolve(strict=True) == path.parent
        assert path.parent == self.root or self.root in path.parent.parents
        safe_id(path.name)
        assert not path.is_symlink()
        if path.exists():
            assert path.is_file() and path.stat().st_uid == os.getuid()
        return approved

    def window_owner(self, approved):
        roles = self.pair.get('owner_roles', {})
        assert approved['owner_role'] in roles, 'window owner role not approved'
        spec = roles[approved['owner_role']]
        descriptor = spec['descriptor']
        safe_id(descriptor)
        # Le layout permet un fichier de rôle approuvé ; aucune résolution depuis le ledger.
        pointer = Path(spec.get('descriptor_path', str(self.root / 'authority-path.json')))
        assert pointer.resolve(strict=True) == pointer and self.root in pointer.parents
        observed = json.loads(pointer.read_text())
        expected = observed[descriptor]
        current = self.check_process(expected)
        entry = self.control / safe_id(spec['entry_file'])
        entry_sha = require_sha(spec['entry_sha256'])
        assert entry.resolve(strict=True) == entry and digest(entry) == entry_sha
        assert str(entry) in current['argv'], 'owner is not running the approved entry'
        components = spec['component_hashes']
        assert spec['entry_file'] in components and 'publication_relay.py' in components
        for name, sha in components.items():
            component = self.control / safe_id(name)
            assert require_sha(sha) == self.pair['controllers'][name]
            assert component.resolve(strict=True) == component and digest(component) == sha
        if spec.get('exe_sha256') is not None:
            assert digest(current['exe']) == require_sha(spec['exe_sha256'])
        return _json_frozen({'pid': current['pid'], 'start_ticks': current['start'],
            'uid': current['uid'], 'exe': current['exe'], 'argv': tuple(current['argv']),
            'cgroup': current['cgroup'], 'entry_sha256': entry_sha,
            'component_hashes': components})

    def window_header(self, spec, phase, ledger_sha256):
        header = context_header(self.plan, self.pin, spec.label)
        header.update(ledger_sha256=require_sha(ledger_sha256), authority_pid=phase.authority_pid,
            authority_start=phase.authority_start, authority_cgroup=phase.authority_cgroup)
        return header

    def record_window_start(self, spec, phase, ledger_sha256):
        self.revalidate_phase(phase)
        approved = self.window_spec(spec)
        for prior in self.plan.stages[:self.plan.position(spec.label)]:
            self.accepted(prior.label)
        owner = self.window_owner(approved)
        assert owner['pid'] == os.getpid(), 'only the actual owner may open its window'
        now = time.monotonic()
        end = min(phase.operation_end_pc, now + spec.window_seconds)
        assert now < end
        data = self.window_header(spec, phase, ledger_sha256)
        data.update(event_kind=approved['event_kind'], owner_role=approved['owner_role'],
            owner=_json_plain(owner), started_pc=now, operation_end_pc=phase.operation_end_pc)
        def before_link():
            self.revalidate_phase(phase)
            assert self.window_owner(approved) == owner
            assert time.monotonic() < end, 'window creation deadline'
        atomic_json(Path(approved['path']), data, before_link)
        self.revalidate_phase(phase)
        assert time.monotonic() < end, 'window fsync crossed END'
        return self.observe_window(spec, phase, ledger_sha256)

    def observe_window(self, spec, phase, ledger_sha256):
        self.revalidate_phase(phase)
        approved = self.window_spec(spec)
        path = Path(approved['path'])
        before = path.read_bytes()
        event_sha = hashlib.sha256(before).hexdigest()
        data = json.loads(before)
        header = self.window_header(spec, phase, ledger_sha256)
        assert set(data) == set(header) | {'event_kind', 'owner_role', 'owner', 'started_pc', 'operation_end_pc'}
        assert all(data[key] == value for key, value in header.items())
        assert data['event_kind'] == approved['event_kind'] and data['owner_role'] == approved['owner_role']
        assert data['operation_end_pc'] == phase.operation_end_pc
        owner = self.window_owner(approved)
        assert data['owner'] == _json_plain(owner), 'window owner identity or closure changed'
        started = data['started_pc']
        assert isinstance(started, (int, float)) and not isinstance(started, bool) and math.isfinite(started)
        now = time.monotonic()
        assert started <= now
        end = min(phase.operation_end_pc, started + spec.window_seconds)
        assert now < end, 'publication window expired'
        self.revalidate_phase(phase)
        assert path.read_bytes() == before, 'window changed during observation'
        assert self.window_owner(approved) == owner
        remaining = end - time.monotonic()
        assert remaining > 0, 'publication window expired after checks'
        return WindowEvidence(event_sha, owner, remaining)

    def publication_header(self, kind):
        return context_header(self.plan, self.pin, kind)

    def accepted(self, kind):
        path = self.root / (kind + '-peer-ack.json')
        assert path.is_file() and not path.is_symlink(), 'prior ACK is absent'
        ack = json.loads(path.read_text())
        header = dict(ack['header'])
        event_sha = header.pop('window_event_sha256', None)
        assert header == self.publication_header(kind), 'prior ACK belongs to another execution'
        spec = self.plan.stage(kind)
        assert set(ack['files']) == set(spec.files) and ack['copied_and_rehashed'] is True
        assert ack['prepared_sha256'] == self.pin and ack['phase'] == self.phase
        assert ack['host'] == self.pair['peer_host']
        assert ack['ledger_sha256'] == ack['files'][spec.ledger_file]
        if spec.window_seconds is not None:
            approved = self.window_spec(spec)
            assert digest(approved['path']) == require_sha(event_sha), 'prior window changed after ACK'
        else:
            assert event_sha is None
        return ack

    def stage(self, kind):
        state = self.active()
        spec = self.plan.stage(kind)
        assert not (self.root / (kind + '-peer-ack.json')).exists(), 'stage already acknowledged'
        for prior in self.plan.stages[:self.plan.position(kind)]:
            self.accepted(prior.label)
        ledger, files, observed_binding = self.provider.read(kind)
        assert set(files) == set(spec.files), 'provider returned missing or extra files'
        for path in files.values():
            path = Path(path)
            assert path.resolve(strict=True) == path and path.is_file() and not path.is_symlink()
            assert self.root in path.parents and path.stat().st_uid == os.getuid()
            assert not any(part in ['.env', '.vault', 'autorite.json'] for part in path.relative_to(self.root).parts)
        hashes = {name: digest(path) for name, path in files.items()}
        sizes = {name: Path(path).stat().st_size for name, path in files.items()}
        assert json.loads(Path(files[spec.ledger_file]).read_text()) == ledger
        elf_map = {}
        for artifact, ref in spec.executables.items():
            if ref['stage'] == kind:
                sha = hashes[ref['file']]
            else:
                prior = self.accepted(ref['stage'])
                sha = prior['files'][ref['file']]
                # Le résolveur hérité est déclaré par le fournisseur, jamais un chemin de ledger libre.
                inherited = self.provider.inherited(ref['stage'], ref['file'])
                assert Path(inherited).resolve(strict=True) == Path(inherited)
                assert self.root in Path(inherited).parents and digest(inherited) == sha
            elf_map[artifact] = {'stage': ref['stage'], 'file': ref['file'], 'sha256': sha}
        header = self.publication_header(kind)
        window = None
        publication_remaining = None
        if spec.window_seconds is not None:
            phase = self.observe_active()
            evidence = self.observe_window(spec, phase, hashes[spec.ledger_file])
            window = {'event_sha256': evidence.event_sha256, 'ledger_sha256': hashes[spec.ledger_file],
                'owner': _json_plain(evidence.owner)}
            publication_remaining = evidence.remaining
            header['window_event_sha256'] = evidence.event_sha256
        after = self.active()
        state.update(remaining=after['remaining'], phase_remaining=after['phase_remaining'],
            stage=kind, ledger=ledger, files=hashes, file_sizes=sizes, header=header,
            elf_map=elf_map, observed_binding=_json_plain(observed_binding),
            observed_binding_sha256=canonical_sha(observed_binding), window=window,
            publication_remaining=publication_remaining)
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
        for spec in self.plan.stages:
            kind = spec.label
            ledger = self.provider.ledger_path(kind)
            if not (self.root / (kind + '-peer-ack.json')).exists():
                if ledger.is_file():
                    if spec.window_seconds is not None and not Path(self.window_spec(spec)['path']).exists():
                        break  # Le propriétaire n'a pas encore scellé son événement, pas de reset.
                    return self.stage(kind)[0]
                break
        state = self.active()
        state['stage'] = None
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
        spec = self.plan.stage(kind)
        names = spec.files
        assert ack['header'] == state['header'] and ack['files'] == state['files']
        assert ack['file_sizes'] == state['file_sizes'] and ack['elf_map'] == state['elf_map']
        assert ack['observed_binding_sha256'] == state['observed_binding_sha256']
        assert ack['peer_files'] == {name: str(self.root / 'relay-local' / kind / name) for name in names}
        assert ack['ledger_sha256'] == state['files'][spec.ledger_file]
        assert ack['copied_and_rehashed'] is True
        if len(state['elf_map']) == 1:
            assert ack['elf_sha256'] == next(iter(state['elf_map'].values()))['sha256']
        def before_link():
            fresh, _ = self.stage(kind)
            assert fresh['files'] == state['files'] and fresh['coordinator'] == state['coordinator']
            assert fresh['header'] == state['header'] and fresh['elf_map'] == state['elf_map']
            assert fresh['observed_binding_sha256'] == state['observed_binding_sha256']
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
        assert hard['argv'][:4] == [self.pair['timeout'], '--signal=TERM',
            '--kill-after=' + seconds_arg(self.limits['whole_kill']), seconds_arg(self.limits['whole_soft'])]
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
        assert hard['argv'] == [self.pair['timeout'], '--signal=TERM',
            '--kill-after=' + seconds_arg(self.limits['whole_kill']), seconds_arg(self.limits['whole_soft']), *expected_gate]
        command = [sys.executable, '-B', str(self.control / 'pilot.py'), 'run', '--proofs',
                   self.pair['proofs'], '--phase', self.phase, '--allocation', allocation]
        now = time.monotonic()
        # Seule l'identité RÉELLE Gate est publiée ; run_argv est explicitement un projet.
        atomic_json(self.root / 'relay-bootstrap.json', {'identity': identity, 'run_argv': command,
            'coordinator': ready, 'prepared_sha256': self.pin, 'hard_timeout': hard,
            'gate_sha256': digest(self.control / 'gate.py'), 'transition_end': now + self.limits['control_cap'], 'queue_end': now + self.limits['queue']})
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
    limits = publication_limits(pair)
    assert pair['relay'] == {'queue_budget': limits['queue'], 'transfer_cap': limits['copy_cap'],
        'control_cap': limits['control_cap'], 'stop_reserve': limits['stop_reserve']}
    os.chdir(args.proofs)
    # Vrai timeout local : groupe privé hérité par tous nos SSH, kill2 physique.
    peer_timeout = pair['peer_timeout']
    assert digest(peer_timeout['path']) == peer_timeout['sha256']
    local_outer = [peer_timeout['path'], '--signal=TERM', '--kill-after=' + seconds_arg(limits['whole_kill']), seconds_arg(limits['whole_soft']),
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
    atomic_json(root / 'local-hard-supervision.json', {'identity': parent, 'hardcap': limits['whole_soft'] + limits['whole_kill'],
        'queue_budget': limits['queue'], 'timeout_sha256': peer_timeout['sha256']})
    transport = SSHTransport(pair, pin, args.phase, args.allocation, args.host, root, entry_file=entry_file)
    def stop(sig, frame):
        raise TimeoutError('relay received signal ' + str(sig))
    for sig in [signal.SIGTERM, signal.SIGINT, signal.SIGHUP]:
        signal.signal(sig, stop)
    hard_end = int(parent['start']) / os.sysconf('SC_CLK_TCK') + limits['whole_soft']
    return Relay(transport, root, hard_end=hard_end, plan=transport.plan,
        prepared_sha256=pin, **pair['relay']).run()


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

