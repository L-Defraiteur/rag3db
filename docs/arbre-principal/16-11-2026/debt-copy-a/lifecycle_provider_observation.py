"""Pinned owner raccord: only GET /sante on existing 127.0.0.1:7878.

No embedding, restart, proxy, redirect, SDK lookup or model probe. The worker is
killed/reaped on its absolute deadline; delayed observations never grant access.
Evidence is exclusive and includes the raw health body even on refusal.
"""
import argparse
import hashlib
import http.client
import importlib.util
import json
import os
from pathlib import Path
import stat
import subprocess
import sys
import time
import lifecycle_protocol as protocol
import lifecycle_grant_producer as producer

IDENTITY_FIELDS = ('service', 'modele', 'dim', 'factice', 'dual', 'sparse',
                   'executable', 'precision', 'max_chars_per_text')


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def process_snapshot(pid):
    root = Path('/proc')/str(pid)
    def sample():
        fields = (root/'stat').read_text().rsplit(')', 1)[1].split()
        return dict(pid=pid, uid=root.stat().st_uid, start=fields[19],
                    state=fields[0], exe=os.readlink(root/'exe'))
    before = sample()
    sha = digest(root/'exe')
    after = sample()
    identity_fields = ('pid', 'uid', 'start', 'exe')
    if any(before[k] != after[k] for k in identity_fields) or any(
            sample['state'] not in ('R', 'S', 'D', 'I') for sample in (before, after)):
        raise ValueError('provider process unstable/unknown')
    return dict(after, exe_sha256=sha)


def transport_module(pins):
    path = Path(__file__).resolve().with_name('provider_health_transport.py')
    if not path.is_file() or path.is_symlink() or digest(path) != pins.get('transport_sha256'):
        raise ValueError('exact shared health transport unavailable')
    spec = importlib.util.spec_from_file_location('cycle_admitted_health_transport', path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def worker_gate(context):
    """Owner's pre-grant lifecycle policy, never the global embedding policy."""
    pins = context['pins']
    _pins(pins)
    if context.get('address') != '127.0.0.1:7878' or context.get('admission') is not True:
        raise ValueError('owner observation only admits lifecycle 7878 health')
    ready = producer._ready(Path(context['ready_path']), context['ready_sha256'], context['nonce'])
    if ready['phase']['pilot'] != pins['pilot'] or ready['phase']['allocation'] != pins['allocation']:
        raise ValueError('worker READY admission differs')
    _validate_process(context['expected_process'], pins['expected_process']) if 'state' in context['expected_process'] else None
    if context['expected_process'] != pins['expected_process']:
        raise ValueError('worker expected process pins differ')
    actual = process_snapshot(pins['expected_process']['pid'])
    _validate_process(actual, pins['expected_process'])
    phase = protocol.read_phase_state(ready)
    now = time.monotonic()
    protocol.validate_ready_phase(ready, now, phase)
    due = protocol._clock(context['contact_deadline'], 'health deadline')
    if not now < due <= min(now+5., ready['grant_deadline_monotonic']-30.):
        raise ValueError('owner health deadline expired or consumes READY margin')
    return None


def worker_context(deadline, context):
    pins = context['pins']
    return dict(context, address='127.0.0.1:7878', admission=True,
                contact_deadline=deadline, expected_process=dict(pins['expected_process']),
                gate_module_path=str(Path(__file__).resolve()),
                gate_module_sha256=pins['observation_sha256'])


def _health_worker(deadline, context):
    # Compatibility entry delegates the network action to the exact shared blob.
    shared = transport_module(context['pins'])
    return shared.worker(dict(worker_context(deadline, context), work_deadline_monotonic=deadline-.5))


def health_get(deadline, *, ready_path, ready_sha256, nonce, pins, trace=None):
    if trace is None: trace = {}
    context = worker_context(deadline, dict(ready_path=str(ready_path),
                             ready_sha256=ready_sha256, nonce=nonce, pins=pins))
    worker_gate(context)
    return transport_module(pins).run_health_worker(context, trace)


def _validate_process(actual, expected):
    if not isinstance(actual, dict) or any(type(actual.get(k)) is not type(v) or actual[k] != v
                                          for k, v in expected.items()):
        raise ValueError('provider PID/start/UID/executable changed')
    if actual.get('state') not in ('R', 'S', 'D', 'I'):
        raise ValueError('provider process state unknown')


def _pins(pins):
    for name, module in [('protocol_sha256', protocol), ('producer_sha256', producer)]:
        if digest(module.__file__) != pins.get(name):
            raise ValueError('shared admitted source hash changed')
    if digest(__file__) != pins.get('observation_sha256'):
        raise ValueError('owner observation source hash changed')
    transport_module(pins)
    expected = pins.get('expected_process', {})
    if set(expected) != {'pid', 'uid', 'start', 'exe', 'exe_sha256'}:
        raise ValueError('complete expected provider process pin required')
    if type(expected['pid']) is not int or expected['pid'] <= 1 or type(expected['uid']) is not int or expected['uid'] != os.getuid():
        raise ValueError('provider process pin UID/PID invalid')
    if not isinstance(expected['start'], str) or not expected['start'].isdigit():
        raise ValueError('provider start pin invalid')
    if not isinstance(expected['exe'], str) or not expected['exe'].startswith('/') or not isinstance(expected['exe_sha256'], str) or len(expected['exe_sha256']) != 64 or any(x not in '0123456789abcdef' for x in expected['exe_sha256']):
        raise ValueError('provider executable pin invalid')
    identity = pins.get('identity', {})
    if set(identity) != set(IDENTITY_FIELDS): raise ValueError('nine identity pins required')
    fixed = dict(service='rag3weaver-embeddings', modele='granite-278m', dim=768,
                 factice=False, dual=False, sparse=False, precision='Flex32')
    if any(type(identity[k]) is not type(v) or identity[k] != v for k, v in fixed.items()):
        raise ValueError('expected real granite dense identity required')
    if type(identity['max_chars_per_text']) is not int or identity['max_chars_per_text'] <= 0 or not isinstance(identity['executable'], str) or not identity['executable'].startswith('/'):
        raise ValueError('provider cap/executable pin invalid')
    for name in ['pilot', 'allocation', 'c7_reference', 'embar_reference']:
        if not isinstance(pins.get(name), str) or not pins[name].strip():
            raise ValueError('admission provenance pin absent')
    if pins['allocation'] != pins['c7_reference']:
        raise ValueError('C7 reference must be the admitted allocation')


def publish_observed_grant(ready_path, destination, *, expected_ready_sha256,
                           expected_nonce, pins, evidence_path,
                           clock=time.monotonic, phase_reader=protocol.read_phase_state,
                           process_reader=process_snapshot, get_health=None):
    """Owner supplies immutable admission pins; producer margin is always 30 s.

    Admission expiry, HTTP's <=5 s budget and the receipt lease are distinct.
    Returned usage0 is an observed baseline, never evidence of zero total RPC.
    """
    _pins(pins)
    evidence_path = Path(evidence_path)
    if evidence_path.exists() or evidence_path.is_symlink():
        raise FileExistsError('fresh owner evidence required')
    pins = protocol.decode(json.dumps(pins, allow_nan=False))
    if get_health is None:
        def get_health(deadline):
            record['transport'] = {}
            return health_get(deadline, ready_path=ready_path, ready_sha256=expected_ready_sha256,
                              nonce=expected_nonce, pins=pins, trace=record['transport'])
    record = dict(kind='LIFECYCLE_NO_EMBEDDING_OWNER_OBSERVATION',
                  no_embedding_calls_authorized=True, health_margin=30,
                  total_rpc_count='unknown; usage0 is not a total RPC counter')

    def persist():
        with evidence_path.open('x') as out:
            json.dump(record, out, indent=2, allow_nan=False)
            out.flush()
            os.fsync(out.fileno())

    def observe(ready):
        if ready['phase']['pilot'] != pins['pilot'] or ready['phase']['allocation'] != pins['allocation']:
            raise ValueError('READY differs from admitted job provenance')
        before = process_reader(pins['expected_process']['pid'])
        record['process_before'] = before
        _validate_process(before, pins['expected_process'])
        phase = phase_reader(ready)
        now = clock()
        protocol.validate_ready_phase(ready, now, phase)
        due = min(now+5., ready['grant_deadline_monotonic']-30.)
        if due <= now: raise ValueError('READY margin exhausted before health')
        record.update(health_started_monotonic=now, health_deadline_monotonic=due)
        response = get_health(due)
        raw = response['body']
        record.update(health_status=response['status'], health_raw_hex=raw.hex(),
                      health_raw_sha256=hashlib.sha256(raw).hexdigest())
        health = transport_module(pins).validate_health(raw, pins['identity'], status=response['status'])
        record['health'] = health
        phase = phase_reader(ready)
        observed = clock()
        protocol.validate_ready_phase(ready, observed, phase)
        if observed >= due: raise ValueError('total health deadline exceeded')
        after = process_reader(pins['expected_process']['pid'])
        record['process_after'] = after
        _validate_process(after, pins['expected_process'])
        _pins(pins)
        phase = phase_reader(ready)
        observed = clock()
        protocol.validate_ready_phase(ready, observed, phase)
        if observed >= due: raise ValueError('total observation deadline exceeded')
        record['health_finished_monotonic'] = observed
        until = min(ready['deadline']-480., observed+230.)
        if until <= observed+20: raise ValueError('receipt lease lacks post-health reserve')
        grant = dict(provider='127.0.0.1:7878', model='granite-278m', dimension=768,
                     c7_reference=pins['c7_reference'], embar_reference=pins['embar_reference'],
                     restart_authorized=False, observed_monotonic=observed,
                     valid_until_monotonic=until, pid=before['pid'], start=before['start'],
                     exe_sha256=before['exe_sha256'], identity=dict(pins['identity']),
                     nonce=ready['nonce'], ready_sha256=expected_ready_sha256,
                     no_embedding_calls_authorized=True, owner_pins=pins,
                     owner_pins_sha256=hashlib.sha256(json.dumps(pins, sort_keys=True,
                         separators=(',', ':'), allow_nan=False).encode()).hexdigest())
        record.update(grant=grant, result='OBSERVED_NOT_YET_GRANTED')
        persist()  # Raw observations must exist BEFORE the producer exposes the grant.
        return grant
    try:
        grant = producer.publish_grant(
            ready_path, destination, expected_sha256=expected_ready_sha256,
            expected_nonce=expected_nonce, expected_protocol_sha256=pins['protocol_sha256'],
            observe_provider=observe, clock=clock, phase_reader=phase_reader,
            health_margin=30.)
        record['result'] = 'PUBLISHED'
        return grant
    except BaseException as error:
        record.update(result='REFUSED', error_type=type(error).__name__)
        raise
    finally:
        if not evidence_path.exists(): persist()


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--health-worker', type=float)
    parser.add_argument('--ready')
    parser.add_argument('--destination')
    parser.add_argument('--pins')
    parser.add_argument('--ready-sha256')
    parser.add_argument('--nonce')
    parser.add_argument('--evidence')
    args = parser.parse_args()
    if args.health_worker is not None:
        _health_worker(args.health_worker, protocol.decode(sys.stdin.buffer.read(131073)))
    else:
        if not all([args.ready, args.destination, args.pins, args.ready_sha256, args.nonce, args.evidence]):
            parser.error('all immutable admission arguments are required')
        publish_observed_grant(args.ready, args.destination,
            expected_ready_sha256=args.ready_sha256, expected_nonce=args.nonce,
            pins=protocol.decode(Path(args.pins).read_bytes()), evidence_path=args.evidence)
