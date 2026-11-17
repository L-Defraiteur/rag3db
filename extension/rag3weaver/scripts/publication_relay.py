#!/usr/bin/env python3
"""Boucles et gardes extraites de D45 ; premier déplacement à comportement constant.

Plan et WindowEvent préparés, qualification encore requise avant admission.
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
    assert limits['common_max'] <= limits['phase_soft'] <= limits['whole_soft']
    assert limits['common_margin'] >= limits['operation_grace']
    return _json_frozen(limits)


def publication_layout(pair):
    """Disposition approuvée, indépendante des noms et phases des fournisseurs."""
    value = pair['publication_layout']
    assert set(value) == {'prepared', 'execution_host', 'entries', 'phases'}
    assert isinstance(value['execution_host'], str) and value['execution_host']
    entries = value['entries']
    assert set(entries) == {'coordinator', 'gate', 'launcher', 'driver', 'common', 'guard'}
    for name in entries.values():
        safe_id(name)
    prepared = Path(value['prepared'])
    proofs = Path(pair['proofs'])
    assert prepared.is_absolute() and prepared.parent in {proofs, Path(pair['control'])}
    assert prepared.name not in {'.env', 'autorite.json'}
    assert value['phases'] and set(value['phases']) == set(pair['publication_plans'])
    for phase, paths in value['phases'].items():
        safe_id(phase)
        assert set(paths) == {'root', 'peer', 'scope', 'deadline_role', 'files'}
        safe_id(paths['deadline_role'])
        root = Path(paths['root'])
        assert root.is_absolute() and (root == proofs or proofs in root.parents)
        assert set(paths['files']) == {'bootstrap', 'launch', 'authority', 'controls',
            'outer_supervision', 'common_command', 'allocation', 'first_red', 'abort', 'classification',
            'peer_ready', 'peer_supervision', 'whole_origin', 'peer_origin', 'phase_origin', 'launch_input'}
        selected = [paths['peer'], paths['scope'], *paths['files'].values()]
        assert len(set(selected)) == len(selected)
        for item in selected:
            path = Path(item)
            assert path.is_absolute() and root in path.parents
            assert '..' not in path.parts and path.name not in {'.env', '.vault', 'autorite.json'}
    return _json_frozen(value)


def layout_entry(pair, role):
    return Path(pair['control']) / publication_layout(pair)['entries'][role]


def verify_components(pair):
    """Fermer les octets des contrôleurs avant leur import et chaque opération."""
    layout = publication_layout(pair)
    required = set(layout['entries'].values()) | {'publication_relay.py', 'publication_loader.py'}
    assert required <= set(pair['controllers'])
    control = Path(pair['control'])
    prepared = Path(layout['prepared'])
    assert prepared.resolve(strict=True) == prepared and prepared.is_file()
    assert prepared.stat().st_uid == os.getuid()
    assert control.is_absolute() and control.resolve(strict=True) == control
    for name, sha in pair['controllers'].items():
        path = control / safe_id(name)
        assert path.resolve(strict=True) == path and path.is_file()
        assert path.stat().st_uid == os.getuid() and digest(path) == require_sha(sha)


def verify_bound_context(pair, pin):
    approved = _read_verified_prepared(publication_layout(pair)['prepared'], pin)
    assert canonical_sha(pair) == canonical_sha(approved), 'prepared data differ from admitted snapshot'
    verify_components(pair)


def check_process(guard, expected):
    assert expected['uid'] == os.getuid()
    actual = guard.process_record(expected['pid'])
    assert actual['state'] != 'Z'
    assert all(actual[key] == expected[key] for key in ['pid', 'start', 'uid', 'exe', 'argv', 'cgroup'])
    return actual


def read_bootstrap(pair, pin, phase, guard):
    layout = publication_layout(pair)
    record = json.loads(Path(layout['phases'][phase]['files']['bootstrap']).read_text())
    assert record['prepared_sha256'] == pin
    gate = layout_entry(pair, 'gate')
    assert record['gate_sha256'] == digest(gate) == pair['controllers'][gate.name]
    hard = check_process(guard, record['hard_timeout'])
    assert hard['exe'] == pair['timeout'] and digest(hard['exe']) == pair['timeout_sha256']
    limits = publication_limits(pair)
    assert hard['argv'][:4] == [pair['timeout'], '--signal=TERM',
        '--kill-after=' + seconds_arg(limits['whole_kill']), seconds_arg(limits['whole_soft'])]
    origin_path = Path(layout['phases'][phase]['files']['whole_origin'])
    assert record['whole_origin_sha256'] == digest(origin_path)
    assert json.loads(origin_path.read_text()) == {key: value for key, value in record['whole_window'].items() if key != 'identity'}
    window_end(pair, pin, phase, 'whole', record['whole_window'], hard)
    return record


def make_window(pair, pin, phase, role, emitter, started):
    """Paquet d'origine AVANT exec, sans identité physique future inventée."""
    limits = publication_limits(pair)
    assert role in {'whole', 'peer', 'phase'}
    soft = limits['phase_soft'] if role == 'phase' else limits['whole_soft']
    kill = limits['phase_kill'] if role == 'phase' else limits['whole_kill']
    entry_role = 'launcher' if role == 'phase' else ('coordinator' if role == 'peer' else 'gate')
    return {'clock': 'CLOCK_MONOTONIC', 'host': pair['peer_host'] if role == 'peer' else publication_layout(pair)['execution_host'],
        'phase': phase, 'role': role, 'prepared_sha256': pin,
        'entry_sha256': pair['controllers'][layout_entry(pair, entry_role).name],
        'emitter': _json_plain(emitter), 'started': started, 'soft_end': started + soft,
        'kill_end': started + soft + kill}


def window_end(pair, pin, phase, role, window, actual):
    limits = publication_limits(pair)
    assert window['clock'] == 'CLOCK_MONOTONIC' and window['role'] == role
    assert window['phase'] == phase and window['prepared_sha256'] == pin
    host = pair['peer_host'] if role == 'peer' else publication_layout(pair)['execution_host']
    assert window['host'] == host
    entry_role = 'launcher' if role == 'phase' else ('coordinator' if role == 'peer' else 'gate')
    entry = layout_entry(pair, entry_role)
    assert window['entry_sha256'] == pair['controllers'][entry.name] == digest(entry)
    issuer = window['emitter']
    assert str(entry) in issuer['argv'], 'origin emitter was not the approved entry'
    assert all(issuer[key] == actual[key] for key in ['pid', 'start', 'uid']), 'origin/timeout exec identity changed'
    assert actual['uid'] == os.getuid()
    if 'identity' in window:
        assert all(window['identity'][key] == actual[key] for key in ['pid', 'start', 'uid', 'exe', 'argv', 'cgroup'])
    for key in ['started', 'soft_end', 'kill_end']:
        assert isinstance(window[key], (int, float)) and not isinstance(window[key], bool) and math.isfinite(window[key])
    soft = limits['phase_soft'] if role == 'phase' else limits['whole_soft']
    kill = limits['phase_kill'] if role == 'phase' else limits['whole_kill']
    assert window['soft_end'] == window['started'] + soft
    assert window['kill_end'] == window['soft_end'] + kill
    assert window['started'] <= time.monotonic(), 'origin clock is in the future'
    return window['soft_end']


def phase_window(pair, pin, phase, guard):
    paths = publication_layout(pair)['phases'][phase]
    rows = json.loads(Path(paths['files']['controls']).read_text())
    matches = [row for row in rows if row['role'] == paths['deadline_role']]
    assert len(matches) == 1
    actual = check_process(guard, matches[0])
    limits = publication_limits(pair)
    assert actual['exe'] == pair['timeout'] and digest(actual['exe']) == pair['timeout_sha256']
    assert actual['argv'][:4] == [pair['timeout'], '--signal=TERM',
        '--kill-after=' + seconds_arg(limits['phase_kill']), seconds_arg(limits['phase_soft'])]
    record = json.loads(Path(paths['files']['outer_supervision']).read_text())
    window = record['window']
    role = window['role']
    assert role in {'phase', 'whole'}
    origin_path = Path(paths['files']['phase_origin' if role == 'phase' else 'whole_origin'])
    assert record['phase_origin_sha256'] == digest(origin_path)
    assert json.loads(origin_path.read_text()) == {key: value for key, value in window.items() if key != 'identity'}
    if role == 'whole':
        # Une seule enveloppe physique peut porter les deux bornes logiques,
        # seulement si identité ET limites préautorisées désignent le même objet.
        whole = read_bootstrap(pair, pin, phase, guard)
        assert all(actual[key] == whole['hard_timeout'][key] for key in ['pid', 'start', 'uid', 'exe', 'argv', 'cgroup'])
        assert limits['phase_soft'] == limits['whole_soft'] and limits['phase_kill'] == limits['whole_kill']
    window_end(pair, pin, phase, role, window, actual)
    return window


def common_request_budget(pair, pin, phase, guard):
    """Les DEUX enveloppes homologuées bornent la demande avant Common."""
    verify_bound_context(pair, pin)
    whole = read_bootstrap(pair, pin, phase, guard)['whole_window']
    phase_budget = phase_window(pair, pin, phase, guard)
    limits = publication_limits(pair)
    end = min(whole['soft_end'], phase_budget['soft_end'])
    remaining = end - time.monotonic() - limits['common_margin']
    requested = math.floor(min(limits['common_max'], remaining))
    assert requested > limits['operation_grace'] + limits['stop_reserve'], 'Common admission budget exhausted'
    return requested


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


def capture_failure(owner, error, *, stage):
    """Premier objet/record capturés AVANT tout constat faillible."""
    first = getattr(owner, '_first_error', None)
    if first is None:
        owner._first_error = error
        owner.failure_record = MappingProxyType({'first_error': repr(error),
            'secondary_errors': [], 'remote_quiet': 'UNKNOWN'})
        return error
    if error is not first:
        record_secondary(owner, stage, error)
    return first


def record_secondary(owner, stage, error):
    first = owner._first_error
    notes = list(getattr(error, '__notes__', ())) if error is not first else []
    owner.failure_record['secondary_errors'].append({'stage': stage, 'error': repr(error), 'notes': notes})
    # Ces notes survivent sur la vraie exception quand le disque refuse tout.
    first.add_note(stage + ' UNKNOWN: ' + repr(error))
    for note in notes:
        first.add_note(stage + ': ' + note)


def persist_failure(owner, path, build_value, *, stage):
    """Un constat raté n'interrompt jamais l'annulation de la cause initiale."""
    try:
        value = build_value()
        value['failure_record'] = _json_plain(owner.failure_record)
        atomic_json(path, value)
        return True
    except FileExistsError:
        return False  # Le premier fichier n'est jamais réécrit.
    except BaseException as error:
        record_secondary(owner, stage, error)
        return False


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
        assert isinstance(hard_end, (int, float)) and not isinstance(hard_end, bool) and math.isfinite(hard_end)
        self.hard_end = hard_end
        assert started < self.hard_end
        self.stage = None
        self.plan = validate_plan(plan)
        self.prepared_sha256 = require_sha(prepared_sha256)
        self.acknowledged = {}
        self.window_ends = {}
        self._first_error, self.failure_record = None, None

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
        return min(self.stop_reserve, remaining)

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
        failure = None
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
            final_end = min(self.phase_deadline, self.hard_end)
            self.transport.stop(deadline=final_end)
            stopped = True
            if self.clock() >= final_end:
                raise TimeoutError('phase deadline crossed during finish or cleanup')
            result = publish_result(self.root / 'relay-result.json', {'verdict': 'VERT',
                'phase_ssh_rc': rc, 'native_verdict_unchanged': True}, final_end, self.clock)
            if result['verdict'] != 'VERT' or self.clock() >= final_end:
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
            failure = capture_failure(self, error, stage='run')
            persist_failure(self, self.root / 'first-red.json', lambda: {
                'reason': repr(failure), 'observed': self.clock()}, stage='write-first-red')
            # Le journal absent n'empêche pas cette tentative de fermeture.
            try:
                if stopped:
                    persist_failure(self, self.root / 'cleanup-not-required.json', lambda: {
                        'phase_ssh_rc': 0,
                        'reason': 'phase and local children already collected before relay refusal',
                        'native_verdict_unchanged': True, 'observed': self.clock()}, stage='write-cleanup-not-required')
                else:
                    self.transport.abort(repr(failure), self.cleanup_cap())
            except BaseException as stop_error:
                record_secondary(self, 'abort', stop_error)
                persist_failure(self, self.root / 'cleanup-refused.json', lambda: {
                    'remote_quiet': 'UNKNOWN', 'reason': repr(stop_error),
                    'original_error': repr(failure), 'native_verdict_unchanged': True,
                    'observed': self.clock()}, stage='write-cleanup-refused')
            raise failure
        finally:
            if not stopped:
                end = min(self.hard_end, self.phase_deadline if self.phase_deadline is not None else self.hard_end)
                try:
                    self.transport.stop(deadline=end)
                except BaseException as stop_error:
                    stop_was_first = failure is None
                    if failure is None:
                        failure = capture_failure(self, stop_error, stage='stop')
                    else:
                        record_secondary(self, 'stop', stop_error)
                    persist_failure(self, self.root / 'local-stop-refused.json', lambda: {
                        'remote_quiet': 'UNKNOWN', 'original_error': repr(failure),
                        'error': repr(stop_error), 'native_verdict_unchanged': True,
                        'observed': self.clock()}, stage='write-local-stop-refused')
                    # Même objet d'origine, malgré l'échec éventuel du constat.
                    if stop_was_first:
                        raise failure



class SSHTransport:
    def __init__(self, pair, pin, phase, allocation, host, root, *, entry_file=None):
        self.pair, self.pin, self.phase, self.allocation = pair, pin, phase, allocation
        self.root, self.host, self.child, self.counter = Path(root), host, None, 0
        assert self.root == Path(publication_layout(pair)['phases'][phase]['peer'])
        assert host == publication_layout(pair)['execution_host']
        self.active_call = None
        self.handles = []
        self.ready_record = None
        self.remote_cancelled = False
        self.control = Path(pair['control'])
        verify_bound_context(pair, pin)
        self.plan = validate_plan(pair['publication_plans'][phase])
        self.prepared_sha256 = require_sha(pin)
        self.entry_file = Path(entry_file) if entry_file is not None else layout_entry(self.pair, 'coordinator')
        verify_components(pair)
        path = layout_entry(self.pair, 'guard')
        self.guard = _load_verified_module('relay_physical_guard', path,
            expected_sha=pair['controllers'][path.name])

    def command(self, action, *extra, cap=None):
        remote = [sys.executable, '-B', str(layout_entry(self.pair, 'gate')), action,
                  '--proofs', self.pair['proofs'], '--prepared', publication_layout(self.pair)['prepared'],
                  '--phase', self.phase, '--pin', self.pin, *extra]
        if action == 'launch':
            # L'entrée arm scelle l'origine AVANT exec du vrai timeout.
            remote[3] = 'arm'
        elif cap is not None:
            assert 0 < cap <= publication_limits(self.pair)['copy_cap']
            remote = [self.pair['timeout'], '--signal=TERM',
                '--kill-after=' + seconds_arg(cap * .1), seconds_arg(cap * .8), *remote]
        # Un seul argv distant quoté ; pas d'interpolation de shell ni de parent argv W/T.
        return ['ssh', '-o', 'BatchMode=yes', '-o', 'ConnectTimeout=5', '-o', 'ControlMaster=no',
                '-o', 'ControlPath=none', self.host, 'exec ' + shlex.join(remote)]

    def paths(self, label, argv):
        self.counter += 1
        stem = self.root / (str(self.counter).zfill(3) + '-' + label)
        atomic_json(stem.with_suffix('.command.json'), {'argv': argv, 'started': time.monotonic()})
        return stem

    def call(self, action, timeout, extra=(), payload=None, destination=None):
        assert math.isfinite(timeout) and timeout > 0
        end = time.monotonic() + timeout
        self.check_coordinator()
        remaining = end - time.monotonic()
        assert remaining > 0, 'RPC admission deadline'
        reserve = min(1.0, remaining / 3)
        argv = self.command(action, *extra, cap=remaining - reserve)
        stem = self.paths(action, argv)
        output = Path(destination) if destination else stem.with_suffix('.stdout')
        primary, cleanup_error, local_quiet = None, None, False
        cleanup_errors = []
        child = None
        try:
            with output.open('xb') as out, stem.with_suffix('.stderr').open('xb') as err:
                try:
                    assert time.monotonic() < end - reserve, 'RPC startup deadline'
                    child = subprocess.Popen(argv, stdin=subprocess.PIPE if payload else subprocess.DEVNULL,
                        stdout=out, stderr=err, start_new_session=False)
                    self.active_call = child
                    duration = end - reserve - time.monotonic()
                    assert duration > 0, 'RPC communicate admission deadline'
                    child.communicate(json.dumps(payload).encode() if payload else None, timeout=duration)
                    if child.returncode:
                        raise RuntimeError('SSH gate ' + action + ' returned ' + str(child.returncode))
                except BaseException as error:
                    primary = error
                finally:
                    if child is not None:
                        try:
                            self.reap(child, deadline=end)
                            local_quiet = child.poll() is not None
                        except BaseException as error:
                            cleanup_error = error
                            cleanup_errors.append(repr(error))
                        if local_quiet:
                            self.active_call = None
        except BaseException as close_error:
            cleanup_errors.append(repr(close_error))
            if cleanup_error is None:
                cleanup_error = close_error
            if primary is None:
                primary = close_error
        if primary is None and (cleanup_error is not None or time.monotonic() >= end):
            primary = cleanup_error if cleanup_error is not None else TimeoutError('RPC final close deadline')
        try:
            atomic_json(stem.with_suffix('.result.json'), {
                'rc': child.returncode if child is not None else None, 'finished': time.monotonic(),
                'interrupted': primary is not None, 'original_error': repr(primary) if primary is not None else None,
                'cleanup': {'local_quiet': local_quiet, 'remote_quiet': 'UNKNOWN',
                    'error': '; '.join(cleanup_errors) if cleanup_errors else None}})
        except BaseException as metadata_error:
            if primary is not None:
                raise primary from metadata_error
            raise
        if primary is not None:
            # Réémettre LE même objet, pas l'erreur secondaire de récolte.
            if cleanup_error is not None and cleanup_error is not primary:
                raise primary from cleanup_error
            raise primary
        if time.monotonic() >= end:
            raise TimeoutError('RPC result fsync deadline')
        return None if destination else json.loads(output.read_text())

    @staticmethod
    def reap(child, *, deadline):
        # L'identité appartient au Popen non récolté ; toute grâce tient dans END.
        if child.poll() is not None:
            return
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise TimeoutError('local child quiescence UNKNOWN at original deadline')
        child.terminate()
        try:
            child.wait(timeout=min(.5, remaining / 2))
        except subprocess.TimeoutExpired:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise TimeoutError('local child quiescence UNKNOWN before KILL')
            child.kill()
            child.wait(timeout=min(.5, remaining))
        if time.monotonic() > deadline:
            raise TimeoutError('local child reap crossed original deadline')

    def ready(self):
        identity = self.guard.process_record(os.getpid())
        self.ready_record = {'host': socket.gethostname(), 'identity': identity,
            'script_sha256': digest(self.entry_file), 'prepared_sha256': self.pin, 'phase': self.phase}
        atomic_json(Path(publication_layout(self.pair)['phases'][self.phase]['files']['peer_ready']), self.ready_record)

    def check_coordinator(self):
        verify_bound_context(self.pair, self.pin)
        assert digest(self.entry_file) == self.pair['controllers'][layout_entry(self.pair, 'coordinator').name]
        assert Path(__file__).resolve() == self.control / 'publication_relay.py'
        assert digest(__file__) == self.pair['controllers']['publication_relay.py']
        assert digest(publication_layout(self.pair)['prepared']) == self.pin
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

    def stop(self, *, deadline):
        failure = None
        for child in (self.active_call, self.child):
            if child is None:
                continue
            try:
                self.reap(child, deadline=deadline)
            except BaseException as error:
                failure = capture_failure(self, error, stage='reap')
        for handle in self.handles:
            try:
                handle.close()
            except BaseException as error:
                failure = capture_failure(self, error, stage='close-log')
        try:
            if time.monotonic() > deadline:
                raise TimeoutError('local child/log close crossed original deadline')
        except BaseException as error:
            failure = capture_failure(self, error, stage='stop-deadline')
        if failure is not None:
            raise failure


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
        self.root = Path(publication_layout(pair)['phases'][phase]['root'])
        self.control = Path(pair['control'])
        self.pilot = pilot
        self.entry_file = Path(entry_file)
        self.provider = None
        self.plan = validate_plan(pair['publication_plans'][phase])
        self.guard = self.pilot.physical

    def path(self, name):
        paths = publication_layout(self.pair)['phases'][self.phase]
        return Path(paths[name] if name in {'scope', 'peer'} else paths['files'][name])

    @property
    def limits(self):
        return publication_limits(self.pair)

    def verify_contract(self):
        verify_bound_context(self.pair, self.pin)
        assert socket.gethostname().lower() == publication_layout(self.pair)['execution_host'].lower()
        assert digest(publication_layout(self.pair)['prepared']) == self.pin
        assert self.entry_file.resolve() == layout_entry(self.pair, 'gate')
        assert Path(__file__).resolve() == self.control / 'publication_relay.py'
        assert digest(__file__) == self.pair['controllers']['publication_relay.py']
        assert digest(self.entry_file) == self.pair['controllers'][layout_entry(self.pair, 'gate').name]
        self.pilot.verify(self.pair, self.phase)

    def verify(self):
        self.verify_contract()
        for name in ['first_red', 'abort', 'classification']:
            assert not self.path(name).exists(), 'phase has already refused or ended'

    def check_process(self, expected):
        return check_process(self.guard, expected)

    def bootstrap(self):
        return read_bootstrap(self.pair, self.pin, self.phase, self.guard)

    def launcher(self):
        bootstrap = self.bootstrap()
        record = json.loads((self.path('launch')).read_text())
        self.check_process(record['identity'])
        assert record['launcher_script_sha256'] == digest(layout_entry(self.pair, 'launcher')) == self.pair['controllers'][layout_entry(self.pair, 'launcher').name]
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

    def control_descriptors(self):
        expected = self.pilot.publication_control_roles(self.pair, self.phase)
        rows = json.loads(self.path('controls').read_text())
        assert len(rows) == len(expected) and {row['role'] for row in rows} == set(expected)
        for row in rows:
            safe_id(row['role'])
            assert row['uid'] == os.getuid() and row['argv'] == expected[row['role']]
        return rows

    def deadline_spec(self):
        spec = self.pilot.publication_deadline_info(self.pair, self.phase)
        safe_id(spec['role'])
        assert spec['role'] == publication_layout(self.pair)['phases'][self.phase]['deadline_role']
        assert spec['argv'][:4] == [self.pair['timeout'], '--signal=TERM',
            '--kill-after=' + seconds_arg(self.limits['phase_kill']), seconds_arg(self.limits['phase_soft'])]
        return spec

    def starting(self):
        controls = self.control_descriptors()
        deadline = self.deadline_spec()
        timeout = next(row for row in controls if row['role'] == deadline['role'])
        self.check_process(timeout)
        assert timeout['argv'] == deadline['argv']
        phase_origin = phase_window(self.pair, self.pin, self.phase, self.guard)
        assert phase_origin['started'] <= self.bootstrap()['queue_end']
        assert timeout['exe'] == self.pair['timeout'] and digest(timeout['exe']) == self.pair['timeout_sha256']
        remaining = min(phase_origin['soft_end'], self.bootstrap()['whole_window']['soft_end']) - time.monotonic()
        assert remaining > self.limits['operation_grace']
        return {'state': 'starting', 'remaining': remaining - self.limits['operation_grace'],
                'phase_remaining': remaining, 'stage': None}

    def active(self):
        self.verify()
        launch = self.launcher()
        # Le codec de politique lit des DESCRIPTEURS, il n'atteste jamais leur vivacité.
        info = self.pilot.publication_phase_info(self.pair, self.phase)
        receipt = Path(info['path'])
        assert Path(info['scope_root']) == self.path('scope')
        assert receipt.name == 'autorite.json' and receipt.parent.parent == self.path('scope')
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
        assert entry == layout_entry(self.pair, 'driver')
        assert common_entry == layout_entry(self.pair, 'common')
        assert info['driver_script_sha256'] == digest(entry) == self.pair['controllers'][info['driver_entry']]
        assert driver['argv'] == info['driver_argv']
        cursor, seen = driver, set()
        while cursor['pid'] != common['pid']:
            assert cursor['pid'] not in seen and cursor['parent'] > 1
            seen.add(cursor['pid'])
            cursor = self.guard.process_record(cursor['parent'])
        assert cursor['start'] == common['start']
        assert Path(info['controls_path']) == self.path('controls')
        deadline = self.deadline_spec()
        assert info['deadline_role'] == deadline['role'] and info['deadline_argv'] == deadline['argv']
        controls = self.control_descriptors()
        timeout = next(row for row in controls if row['role'] == info['deadline_role'])
        self.check_process(timeout)
        assert digest(self.pair['timeout']) == self.pair['timeout_sha256']
        assert timeout['argv'] == info['deadline_argv']
        outer_end = phase_window(self.pair, self.pin, self.phase, self.guard)['soft_end']
        end = authority['echeance']
        assert math.isfinite(end) and end <= outer_end
        assert authority['echeance'] == info['original_common_end'], 'original Common END changed'
        recipe = json.loads(receipt.with_name('recette.json').read_text())
        assert 0 < recipe['delai'] <= self.limits['common_max']
        hard_end = self.bootstrap()['whole_window']['soft_end']
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
        pointer = Path(spec.get('descriptor_path', str(self.path('authority'))))
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
        assert 'publication_loader.py' in components
        assert dict(components) == dict(self.pair['controllers']), 'owner source closure is incomplete'
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
        if not (self.path('launch')).exists():
            if (self.path('bootstrap')).exists():
                return self.waiting_transition()
            return {'state': 'waiting'}
        self.launcher()
        if not (self.path('authority')).exists():
            if (self.path('outer_supervision')).exists():
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
                assert time.monotonic() - before < min(self.limits['copy_cap'], state['remaining'])
                assert not any(self.path(name).exists() for name in ['first_red', 'abort', 'classification'])
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
        assert ack['peer_files'] == {name: str(self.path('peer') / kind / name) for name in names}
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
        bootstrap = json.loads((self.path('bootstrap')).read_text())
        assert bootstrap['prepared_sha256'] == self.pin
        assert bootstrap['gate_sha256'] == digest(layout_entry(self.pair, 'gate')) == self.pair['controllers'][layout_entry(self.pair, 'gate').name]
        hard = bootstrap['hard_timeout']
        assert hard['exe'] == self.pair['timeout'] and digest(hard['exe']) == self.pair['timeout_sha256']
        assert hard['argv'][:4] == [self.pair['timeout'], '--signal=TERM',
            '--kill-after=' + seconds_arg(self.limits['whole_kill']), seconds_arg(self.limits['whole_soft'])]
        allowed = [bootstrap['identity']['argv'], bootstrap['run_argv']]
        roles = [('remote-hard-timeout', hard, None), ('launcher', bootstrap['identity'], allowed)]
        common_end_original = None
        info_path = self.path('authority')
        if info_path.exists():
            info = self.pilot.publication_phase_info(self.pair, self.phase)
            assert info['driver_entry'] == layout_entry(self.pair, 'driver').name
            assert info['driver_script_sha256'] == digest(layout_entry(self.pair, 'driver')) == self.pair['controllers'][info['driver_entry']]
            assert info['supervisor_identity']['argv'] == info['common_argv']
            receipt = Path(info['path'])
            assert receipt.resolve(strict=True) == receipt and receipt.parent.parent == self.path('scope')
            common_end_original = json.loads(receipt.read_text())['echeance']
            roles.append(('common', info['supervisor_identity'], None))
        elif (self.path('scope')).exists():
            # Fenêtre Common lancé mais pilote pas encore au rendez-vous authority-path.
            receipts = list((self.path('scope')).glob('validation-*/autorite.json'))
            assert len(receipts) <= 1, 'ambiguous Common cancellation'
            if receipts:
                authority = json.loads(receipts[0].read_text())
                assert authority['script_sha256'] == self.pair['controllers'][layout_entry(self.pair, 'common').name]
                try:
                    common = self.guard.process_record(authority['pid'])
                except (FileNotFoundError, ProcessLookupError):
                    common = None
                if common and common['start'] == authority['debut']:
                    assert common['uid'] == os.getuid()
                    assert common['argv'] == json.loads((self.path('common_command')).read_text())
                    roles.append(('common', common, None))
        controls_path = self.path('controls')
        if controls_path.exists():
            deadline = self.deadline_spec()
            descriptors = self.control_descriptors()
            phase_outer = next(row for row in descriptors if row['role'] == deadline['role'])
            assert phase_outer['argv'] == deadline['argv']
            roles.extend(('phase-timeout' if row['role'] == deadline['role'] else row['role'], row, None)
                for row in descriptors)
        closed = (self.path('classification')).exists()
        if not closed:
            try:
                atomic_json(self.path('first_red'), {'reason': repr(RuntimeError('relay: ' + reason)),
                    'observed': time.monotonic()})
            except FileExistsError:
                pass
        whole_end = window_end(self.pair, self.pin, self.phase, 'whole', bootstrap['whole_window'], hard)
        end = min(time.monotonic() + max(.001, self.limits['stop_reserve'] - 1.6), whole_end)
        if common_end_original is not None:
            assert math.isfinite(common_end_original)
            end = min(end, common_end_original)
        stop_errors = []
        try:
            # Laisser au Common réellement observé sa fermeture canonique avant le hard launcher.
            common_roles = [row for row in roles if row[0] == 'common']
            for _, identity, argv in common_roles:
                if time.monotonic() >= end:
                    break
                self.signal_role(identity, signal.SIGTERM, argv)
            common_end = min(end, time.monotonic() + 1.7)
            while any(self.role_alive(identity, argv) for _, identity, argv in common_roles) and time.monotonic() < common_end:
                time.sleep(.025)
            for role, identity, argv in roles:
                if role in ['phase-timeout', 'remote-hard-timeout'] and time.monotonic() < end:
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
        if time.monotonic() >= end:
            unknown.append('cancellation observation crossed original deadline')
        scope_quiet = None
        scopes = list((self.path('scope')).glob('validation-*'))
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

    def arm(self, allocation, ready):
        # Pas de corpus/preflight/acteur avant l'installation du watchdog.
        verify_bound_context(self.pair, self.pin)
        assert digest(publication_layout(self.pair)['prepared']) == self.pin
        assert socket.gethostname().lower() == publication_layout(self.pair)['execution_host'].lower()
        assert allocation and ready['prepared_sha256'] == self.pin and ready['phase'] == self.phase
        assert self.entry_file.resolve() == layout_entry(self.pair, 'gate')
        assert Path(__file__).resolve() == self.control / 'publication_relay.py'
        emitter = self.guard.process_record(os.getpid())
        assert str(self.entry_file) in emitter['argv']
        now = time.monotonic()
        origin = make_window(self.pair, self.pin, self.phase, 'whole', emitter, now)
        atomic_json(self.path('whole_origin'), origin)
        assert digest(self.pair['timeout']) == self.pair['timeout_sha256']
        command = [self.pair['timeout'], '--signal=TERM',
            '--kill-after=' + seconds_arg(self.limits['whole_kill']), seconds_arg(self.limits['whole_soft']),
            sys.executable, '-B', str(layout_entry(self.pair, 'gate')), 'launch',
            '--proofs', self.pair['proofs'], '--prepared', publication_layout(self.pair)['prepared'],
            '--phase', self.phase, '--pin', self.pin, '--allocation', allocation]
        # ready reste sur stdin du même processus, pas un nouveau rendez-vous libre.
        atomic_json(self.path('launch_input'), ready)
        with self.path('launch_input').open('rb') as stream:
            os.dup2(stream.fileno(), 0)
        os.execve(self.pair['timeout'], command, os.environ.copy())

    def launch(self, allocation, ready):
        self.verify()
        assert allocation and not (self.path('allocation')).exists()
        assert not (self.path('scope')).exists() and not (self.path('launch')).exists()
        assert ready['host'] == self.pair['peer_host'] and ready['identity']['uid'] == os.getuid()
        assert ready['prepared_sha256'] == self.pin and ready['phase'] == self.phase
        assert ready['script_sha256'] == self.pair['controllers'][layout_entry(self.pair, 'coordinator').name]
        assert ready['identity']['pid'] > 1 and ready['identity']['start'].isdigit()
        assert ready['identity']['argv'][:3] == [sys.executable, '-B', str(layout_entry(self.pair, 'coordinator'))]
        assert digest(layout_entry(self.pair, 'coordinator')) == ready['script_sha256']
        identity = self.guard.process_record(os.getpid())
        hard = self.guard.process_record(os.getppid())
        expected_gate = [sys.executable, '-B', str(layout_entry(self.pair, 'gate')), 'launch',
            '--proofs', self.pair['proofs'], '--prepared', publication_layout(self.pair)['prepared'],
            '--phase', self.phase, '--pin', self.pin, '--allocation', allocation]
        assert hard['uid'] == os.getuid() and hard['exe'] == self.pair['timeout']
        assert digest(hard['exe']) == self.pair['timeout_sha256']
        assert hard['argv'] == [self.pair['timeout'], '--signal=TERM',
            '--kill-after=' + seconds_arg(self.limits['whole_kill']), seconds_arg(self.limits['whole_soft']), *expected_gate]
        command = self.pilot.publication_launch_command(self.pair, self.phase, allocation)
        assert isinstance(command, list) and all(isinstance(item, str) for item in command)
        assert str(layout_entry(self.pair, 'launcher')) in command
        now = time.monotonic()
        origin = json.loads(self.path('whole_origin').read_text())
        window_end(self.pair, self.pin, self.phase, 'whole', origin, hard)
        queue_end = origin['started'] + self.limits['queue']
        assert now < queue_end, 'queue budget already exhausted before ARM'
        # Seule l'identité RÉELLE Gate est publiée ; run_argv est explicitement un projet.
        atomic_json(self.path('bootstrap'), {'identity': identity, 'run_argv': command,
            'coordinator': ready, 'prepared_sha256': self.pin, 'hard_timeout': hard,
            'whole_window': dict(origin, identity=hard), 'whole_origin_sha256': digest(self.path('whole_origin')),
            'gate_sha256': digest(layout_entry(self.pair, 'gate')), 'transition_end': now + self.limits['control_cap'], 'queue_end': queue_end})
        os.chdir(self.pair['proofs'])
        os.execve(sys.executable, command, dict(os.environ, **self.pilot.publication_launch_environment(self.pair, self.phase)))



def relay_main(entry_file):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--proofs', type=Path, required=True)
    parser.add_argument('--prepared', type=Path)
    parser.add_argument('--pin', required=True)
    parser.add_argument('--phase', required=True)
    parser.add_argument('--allocation', required=True)
    parser.add_argument('--host', default='luciepc')
    args = parser.parse_args()
    path = args.prepared if args.prepared is not None else args.proofs / 'prepared.json'
    pin = args.pin
    pair = _read_verified_prepared(path, pin)
    assert args.host == publication_layout(pair)['execution_host']
    assert str(path) == publication_layout(pair)['prepared']
    assert str(args.proofs) == pair['proofs']
    verify_components(pair)
    assert socket.gethostname() == pair['peer_host']
    assert Path(entry_file).resolve() == layout_entry(pair, 'coordinator')
    assert Path(__file__).resolve() == Path(pair['control']) / 'publication_relay.py'
    assert digest(__file__) == pair['controllers']['publication_relay.py']
    assert digest(entry_file) == pair['controllers'][layout_entry(pair, 'coordinator').name]
    limits = publication_limits(pair)
    assert pair['relay'] == {'queue_budget': limits['queue'], 'transfer_cap': limits['copy_cap'],
        'control_cap': limits['control_cap'], 'stop_reserve': limits['stop_reserve']}
    os.chdir(args.proofs)
    # Vrai timeout local : groupe privé hérité par tous nos SSH, kill2 physique.
    peer_timeout = pair['peer_timeout']
    assert digest(peer_timeout['path']) == peer_timeout['sha256']
    local_outer = [peer_timeout['path'], '--signal=TERM', '--kill-after=' + seconds_arg(limits['whole_kill']), seconds_arg(limits['whole_soft']),
        sys.executable, '-B', str(Path(entry_file).resolve()), *sys.argv[1:]]
    path_guard = layout_entry(pair, 'guard')
    guard = _load_verified_module('local_outer_guard', path_guard,
        expected_sha=pair['controllers'][path_guard.name])
    parent = guard.process_record(os.getppid())
    if parent['exe'] != peer_timeout['path'] or parent['argv'] != local_outer:
        identity = guard.process_record(os.getpid())
        origin = make_window(pair, pin, args.phase, 'peer', identity, time.monotonic())
        atomic_json(Path(publication_layout(pair)['phases'][args.phase]['files']['peer_origin']), origin)
        os.execve(peer_timeout['path'], local_outer, os.environ.copy())
    assert parent['uid'] == os.getuid()
    root = Path(publication_layout(pair)['phases'][args.phase]['peer'])
    root.mkdir(mode=0o700)
    atomic_json(Path(publication_layout(pair)['phases'][args.phase]['files']['peer_supervision']), {'identity': parent, 'hardcap': limits['whole_soft'] + limits['whole_kill'],
        'queue_budget': limits['queue'], 'timeout_sha256': peer_timeout['sha256']})
    transport = SSHTransport(pair, pin, args.phase, args.allocation, args.host, root, entry_file=entry_file)
    def stop(sig, frame):
        raise TimeoutError('relay received signal ' + str(sig))
    for sig in [signal.SIGTERM, signal.SIGINT, signal.SIGHUP]:
        signal.signal(sig, stop)
    origin = json.loads(Path(publication_layout(pair)['phases'][args.phase]['files']['peer_origin']).read_text())
    # Le timeout local est distinct du timeout PC, seule l'horloge locale est comparée.
    hard_end = window_end(pair, pin, args.phase, 'peer', origin, parent)
    return Relay(transport, root, hard_end=hard_end, plan=transport.plan,
        prepared_sha256=pin, **pair['relay']).run()


def gate_main(gate_factory):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['arm', 'launch', 'snapshot', 'export', 'ack', 'abort'])
    parser.add_argument('--proofs', type=Path, required=True)
    parser.add_argument('--prepared', type=Path)
    parser.add_argument('--pin', required=True)
    parser.add_argument('--phase', required=True)
    parser.add_argument('--allocation')
    parser.add_argument('--kind')
    parser.add_argument('--file')
    args = parser.parse_args()
    pair_path = args.prepared if args.prepared is not None else args.proofs / 'prepared.json'
    pair = _read_verified_prepared(pair_path, args.pin)
    assert str(pair_path) == publication_layout(pair)['prepared']
    assert str(args.proofs) == pair['proofs']
    verify_components(pair)
    os.chdir(args.proofs)
    gate = gate_factory(pair, args.pin, args.phase)
    if args.action == 'arm':
        gate.arm(args.allocation, json.load(sys.stdin))
    elif args.action == 'launch':
        gate.launch(args.allocation, json.load(sys.stdin))
    elif args.action == 'snapshot':
        print(json.dumps(gate.snapshot()))
    elif args.action == 'export':
        gate.export(args.kind, args.file, sys.stdout.buffer)
    elif args.action == 'ack':
        print(json.dumps(gate.acknowledge(args.kind, json.load(sys.stdin))))
    else:
        print(json.dumps(gate.abort(json.load(sys.stdin)['reason'])))

