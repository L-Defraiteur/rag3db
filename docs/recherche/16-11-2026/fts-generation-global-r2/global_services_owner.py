"""Global canonical coordination: two existing providers and private PG bootstrap.

No lifecycle no-embedding policy is reused. No contact on import. Production pins
bind a fresh C7/G SQL+topology attestation and exclusive reservation.
No PostgreSQL contact, client installation or docker exec is performed here.
Bootstrap identity is not the identity of future per-process test databases.
"""
import argparse
import hashlib
import http.client
import importlib.util
import json
import os
from pathlib import Path
import re
import stat
import subprocess
import sys
import tempfile
import time
import lifecycle_protocol as phase_protocol
import lifecycle_provider_observation as process_observation

PROVIDERS = {'127.0.0.1:7878': ('granite-278m', 768, False),
             '127.0.0.1:7879': ('bge-m3', 1024, True)}
IDENTITY = {'service', 'modele', 'dim', 'factice', 'dual', 'sparse',
            'precision', 'executable', 'max_chars_per_text'}


def sha(path): return hashlib.sha256(Path(path).read_bytes()).hexdigest()

def pins_digest(pins):
    return hashlib.sha256(json.dumps(pins, sort_keys=True, separators=(',', ':'),
                                     allow_nan=False).encode()).hexdigest()


def owned_json(path, expected_sha=None):
    path = Path(path)
    info = path.lstat()
    if not stat.S_ISREG(info.st_mode) or info.st_uid != os.getuid() or info.st_size > 65536:
        raise ValueError('coordination input not regular/owned/bounded')
    raw = path.read_bytes()
    if expected_sha is not None and hashlib.sha256(raw).hexdigest() != expected_sha:
        raise ValueError('coordination input bytes changed')
    return phase_protocol.decode(raw)


def strict(actual, expected):
    if not isinstance(actual, dict) or any(type(actual.get(k)) is not type(v) or actual[k] != v
                                          for k, v in expected.items()):
        raise ValueError('typed pinned identity differs')


def pins_check(pins):
    for key, path in [('protocol_sha256', phase_protocol.__file__),
                      ('process_module_sha256', process_observation.__file__),
                      ('owner_sha256', __file__)]:
        if sha(path) != pins.get(key): raise ValueError('admitted source pin changed')
    for key in ['app', 'pilot']:
        if not isinstance(pins.get(key), str) or not re.fullmatch('[a-f0-9]{40}', pins[key]):
            raise ValueError('exact source APP/pilot required')
    for key in ['scope', 'c7_reference', 'embar_reference', 'g_reference', 'reservation_reference']:
        if not isinstance(pins.get(key), str) or not pins[key].strip():
            raise ValueError('admission/reservation provenance absent')
    if not pins['scope'].startswith('/'): raise ValueError('scope path must be absolute')
    providers = pins.get('providers', [])
    if len(providers) != 2 or {p['address'] for p in providers} != set(PROVIDERS):
        raise ValueError('exact two existing providers required')
    for provider in providers:
        process = provider['process']
        if set(process) != {'pid', 'uid', 'start', 'exe', 'exe_sha256'}:
            raise ValueError('provider process pins incomplete')
        if type(process['pid']) is not int or process['pid'] <= 1 or type(process['uid']) is not int or process['uid'] != os.getuid() or not isinstance(process['start'], str) or not process['start'].isdigit():
            raise ValueError('provider PID/UID/start pin invalid')
        if not isinstance(process['exe'], str) or not process['exe'].startswith('/') or not isinstance(process['exe_sha256'], str) or not re.fullmatch('[a-f0-9]{64}', process['exe_sha256']):
            raise ValueError('provider executable pin invalid')
        identity = provider['identity']
        if set(identity) != IDENTITY: raise ValueError('exact nine identity fields required')
        model, dimension, dual = PROVIDERS[provider['address']]
        strict(identity, dict(service='rag3weaver-embeddings', modele=model, dim=dimension,
                              dual=dual, sparse=dual, factice=False, precision='Flex32'))
        if type(identity['max_chars_per_text']) is not int or identity['max_chars_per_text'] <= 0 or not isinstance(identity['executable'], str) or not identity['executable'].startswith('/'):
            raise ValueError('provider cap/executable pin invalid')
    transport_module(pins)
    pg = pins['postgres']
    strict(pg, dict(host='127.0.0.1', port=5433, database='rag3weaver_test', user='rag3weaver'))
    expected = pg.get('allocation')
    if not isinstance(expected, dict): raise ValueError('external G allocation pins required')
    strict(expected, {k:pins[k] for k in ['app', 'pilot', 'scope', 'c7_reference', 'g_reference', 'reservation_reference']})
    strict(expected, dict(tool_qualified=True, allocation_active=True))
    pg_module(pins)


def pg_module(pins):
    """Import the exact G verifier/SQL constants; never infer executable/tool pins."""
    root = Path(__file__).resolve().parent
    for key, name in [('pg_receipt_sha256','pg_receipt.py'),
                      ('pg_identity_sql_sha256','identity.sql'),
                      ('pg_postcheck_sql_sha256','backend-postcheck.sql')]:
        path = root/name
        if not path.is_file() or path.is_symlink() or sha(path) != pins.get(key):
            raise ValueError('exact G verifier/SQL source unavailable')
    spec = importlib.util.spec_from_file_location('global_admitted_pg_receipt', root/'pg_receipt.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def ready_read(path, expected_sha, nonce, pins):
    ready = owned_json(path, expected_sha)
    strict(ready, dict(app=pins['app'], pilot=pins['pilot'], scope=pins['scope'],
                       c7_reference=pins['c7_reference'], nonce=nonce,
                       service_operations_authorized=False,
                       embedding_calls_authorized_only_by_canonical=True))
    strict(ready['phase'], dict(job='lifecycle-provider', purpose='fts-global-models-and-pg',
                                pilot=pins['pilot'], allocation=pins['c7_reference']))
    if 'no_embedding_calls_authorized' in ready:
        raise ValueError('lifecycle no-embedding flag forbidden in global policy')
    if set(ready['requested']) != {*PROVIDERS, '127.0.0.1:5433/rag3weaver_test'}:
        raise ValueError('READY service set differs')
    return ready


def gate(ready, clock, phase_reader):
    snapshot = phase_reader(ready)
    now = clock()
    phase_protocol.validate_ready_phase(ready, now, snapshot)
    return now


def reservation_read(path, expected_sha, ready, ready_sha, pins, now, *, admission=True):
    expected = pins['postgres']['allocation']
    strict(expected, dict(nonce=ready['nonce'], ready_sha256=ready_sha))
    normal = dict(issued_monotonic=ready['ready_observed_monotonic'],
                  admission_deadline_monotonic=ready['grant_deadline_monotonic'],
                  common_end_monotonic=ready['deadline'], active=True, first_red=False)
    admitted = None if admission else pins.get('admitted_monotonic')
    return pg_module(pins).validate_attestation(path, expected_sha, expected, normal, now,
                       admission=admission, admitted_at=admitted)


def process_check(provider, process_reader):
    expected = provider['process'] if 'process' in provider else {k:provider[k] for k in ['pid','uid','start','exe','exe_sha256']}
    actual = process_reader(expected['pid'])
    strict(actual, expected)
    if actual.get('state') not in ['R', 'S', 'D', 'I']:
        raise ValueError('provider process not live/known')
    return actual


def transport_module(pins):
    path = Path(__file__).resolve().with_name('provider_health_transport.py')
    if not path.is_file() or path.is_symlink() or sha(path) != pins.get('transport_sha256'):
        raise ValueError('exact shared health transport unavailable')
    spec = importlib.util.spec_from_file_location('global_admitted_health_transport', path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def worker_gate(context):
    """Global policy only; called by the common worker immediately before HTTP."""
    pins = context['pins']
    pins_check(pins)
    ready = ready_read(context['ready_path'], context['ready_sha256'], context['nonce'], pins)
    provider = next((p for p in pins['providers'] if p['address'] == context.get('address')), None)
    if context.get('kind') != 'provider' or provider is None:
        raise ValueError('only fixed provider health contacts allowed')
    strict(context['expected_process'], provider['process'])
    process_check(provider, process_observation.process_snapshot)
    now = contact_gate(ready, pins, context['admission'], context['lease_end'],
                       time.monotonic, phase_protocol.read_phase_state)
    due = phase_protocol._clock(context['contact_deadline'], 'contact deadline')
    upper = min(now+5, context['lease_end']-15)
    if context['admission']: upper = min(upper, ready['grant_deadline_monotonic']-30)
    if due <= now or due > upper:
        raise ValueError('contact budget expired or exceeds admission/lease margin')
    return None


def worker_context(context):
    pins = context['pins']
    provider = next((p for p in pins['providers'] if p['address'] == context.get('address')), None)
    if provider is None: raise ValueError('provider outside admitted set')
    return dict(context, gate_module_path=str(Path(__file__).resolve()),
                gate_module_sha256=pins['owner_sha256'], expected_process=provider['process'])


def worker(context):
    # Compatibility entry; there is only one network/cleanup implementation.
    shared = transport_module(context['pins'])
    context = worker_context(context)
    context.setdefault('work_deadline_monotonic', context['contact_deadline']-.5)
    return shared.worker(context)


def transport(context, trace):
    return transport_module(context['pins']).run_health_worker(worker_context(context), trace)


def contact_gate(ready, pins, admission, lease_end, clock, phase_reader):
    if type(admission) is not bool:
        raise ValueError('explicit admission/lease policy required')
    lease_end = phase_protocol._clock(lease_end, 'contact lease end')
    if lease_end > ready['deadline']:
        raise ValueError('contact lease cannot extend common END')
    current = ready_read(pins['ready_path'], pins['ready_sha256'], pins['nonce'], pins)
    if current != ready: raise ValueError('caller READY differs from immutable published bytes')
    snapshot = phase_reader(current)
    now = clock()
    if admission: phase_protocol.validate_ready_phase(ready, now, snapshot)
    else: phase_protocol.validate_phase_authority(ready, now, snapshot)
    if now >= lease_end: raise ValueError('contact lease expired')
    reservation_read(pins['reservation_path'], pins['reservation_sha256'], ready,
                     pins['ready_sha256'], pins, now, admission=admission)
    return now


def health_observation(ready, pins, provider, evidence, *, admission, lease_end,
                       clock=time.monotonic, phase_reader=phase_protocol.read_phase_state,
                       process_reader=process_observation.process_snapshot, contact=transport):
    """Reusable owner/consumer contact; caller persists evidence even on refusal.

    Consumer uses grant.owner_pins (immutable source/provenance/process context).
    Admission before canon, scope+lease after canon: no late re-admission.
    """
    pins_check(pins)
    pinned = next((p for p in pins['providers'] if p['address'] == provider['address']), None)
    if pinned is None: raise ValueError('provider outside admitted two-provider set')
    strict(provider['identity'], pinned['identity'])
    expected_process = provider['process'] if 'process' in provider else {k:provider[k] for k in ['pid','uid','start','exe','exe_sha256']}
    strict(expected_process, pinned['process'])
    evidence.update(address=provider['address'], trace={})
    evidence['process_before'] = process_check(provider, process_reader)
    now = contact_gate(ready, pins, admission, lease_end, clock, phase_reader)
    due = min(now+5, lease_end-15)
    if admission: due = min(due, ready['grant_deadline_monotonic']-30)
    if due <= now: raise ValueError('contact margin exhausted')
    context = dict(kind='provider', pins=pins, ready_path=pins['ready_path'],
                   ready_sha256=pins['ready_sha256'], nonce=ready['nonce'],
                   admission=admission, lease_end=lease_end,
                   address=provider['address'], contact_deadline=due)
    response = contact(context, evidence['trace'])
    body = response['body']
    evidence.update(status=response['status'], raw_hex=body.hex(),
                    raw_sha256=hashlib.sha256(body).hexdigest())
    value = transport_module(pins).validate_health(body, provider['identity'], status=response['status'])
    now = contact_gate(ready, pins, admission, lease_end, clock, phase_reader)
    if now >= due: raise ValueError('total health budget exceeded')
    evidence['process_after'] = process_check(provider, process_reader)
    pins_check(pins)
    now = contact_gate(ready, pins, admission, lease_end, clock, phase_reader)
    if now >= due: raise ValueError('total observation budget exceeded')
    evidence.update(observed=value, finished_monotonic=now)
    return value


def stage(destination, value):
    fd, name = tempfile.mkstemp(prefix='.services-', dir=destination.parent)
    path = Path(name)
    try:
        with os.fdopen(fd, 'wb') as out:
            out.write(json.dumps(value, sort_keys=True, allow_nan=False).encode())
            out.flush()
            os.fsync(out.fileno())
    except BaseException:
        path.unlink()
        raise
    return path


def publish_services(ready_path, destination, *, expected_ready_sha256, expected_nonce,
                     pins, reservation_path, reservation_sha256, evidence_path,
                     clock=time.monotonic, phase_reader=phase_protocol.read_phase_state,
                     process_reader=process_observation.process_snapshot, contact=transport):
    pins_check(pins)
    pins = phase_protocol.decode(json.dumps(pins, allow_nan=False))
    ready_path, destination, evidence_path = map(Path, [ready_path, destination, evidence_path])
    pins.update(ready_path=str(ready_path), ready_sha256=expected_ready_sha256, nonce=expected_nonce,
                reservation_path=str(reservation_path), reservation_sha256=reservation_sha256)
    if any(p.exists() or p.is_symlink() for p in [destination, evidence_path]):
        raise FileExistsError('fresh exclusive grant/evidence required')
    ready = ready_read(ready_path, expected_ready_sha256, expected_nonce, pins)
    record = dict(kind='GLOBAL_CANONICAL_OWNER_OBSERVATIONS', contacts=[],
                  rpc_total='unknown; usage0 baseline is not a total RPC counter',
                  pg_scope='bootstrap identity; canonical creates per-process databases')
    persisted = False
    def persist():
        nonlocal persisted
        with evidence_path.open('x') as out:
            json.dump(record, out, indent=2, allow_nan=False)
            out.flush()
            os.fsync(out.fileno())
        persisted = True
    def check():
        current = ready_read(ready_path, expected_ready_sha256, expected_nonce, pins)
        now = gate(current, clock, phase_reader)
        reservation_read(reservation_path, reservation_sha256, current,
                         expected_ready_sha256, pins, now)
        return now
    try:
        for provider in pins['providers']:
            row = {}
            record['contacts'].append(row)
            health_observation(ready, pins, provider, row, admission=True,
                               lease_end=ready['deadline'], clock=clock,
                               phase_reader=phase_reader, process_reader=process_reader, contact=contact)
        now = check()
        pins['admitted_monotonic'] = now
        record['pg_attestation'] = reservation_read(reservation_path, reservation_sha256,
                            ready, expected_ready_sha256, pins, now)
        grant = dict(app=pins['app'], pilot=pins['pilot'], scope=pins['scope'],
                     ready_sha256=expected_ready_sha256, nonce=expected_nonce,
                     c7_reference=pins['c7_reference'], embar_reference=pins['embar_reference'],
                     observed_monotonic=now, valid_until_monotonic=ready['deadline'],
                     restart_authorized=False, service_operations_authorized=False,
                     embedding_calls_authorized_only_by_canonical=True,
                     owner_pins=pins, owner_pins_sha256=pins_digest(pins),
                     providers=[dict(address=p['address'], identity=p['identity'], **p['process'])
                                for p in pins['providers']],
                     postgres=dict(host='127.0.0.1', port=5433, database='rag3weaver_test', ready=True,
                         private_test_only=True, exclusive_reservation=True,
                         reservation_reference=pins['reservation_reference'],
                         identity=record['pg_attestation']['sql'],
                         topology=record['pg_attestation']['topology']))
        record.update(result='OBSERVED_NOT_YET_GRANTED', grant=grant)
        persist()
        staged = stage(destination, grant)
        try:
            pins_check(pins)
            record['process_before_grant'] = [process_check(p, process_reader) for p in pins['providers']]
            now = check()
            if not ready['ready_observed_monotonic'] <= grant['observed_monotonic'] <= now or now >= grant['valid_until_monotonic']:
                raise ValueError('grant observation/lease invalid before publication')
            os.link(staged, destination)
        finally:
            staged.unlink()
        return grant
    except BaseException as error:
        record.update(result='REFUSED', error_type=type(error).__name__)
        raise
    finally:
        if not persisted: persist()


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--worker', action='store_true')
    for name in ['ready', 'destination', 'pins', 'ready-sha256', 'nonce',
                 'reservation', 'reservation-sha256', 'evidence']:
        parser.add_argument('--'+name)
    args = parser.parse_args()
    if args.worker:
        worker(phase_protocol.decode(sys.stdin.buffer.read(262145)))
    else:
        if any(getattr(args, key) is None for key in ['ready', 'destination', 'pins',
                'ready_sha256', 'nonce', 'reservation', 'reservation_sha256', 'evidence']):
            parser.error('all immutable admission and reservation inputs required')
        publish_services(args.ready, args.destination, expected_ready_sha256=args.ready_sha256,
            expected_nonce=args.nonce, pins=owned_json(args.pins), reservation_path=args.reservation,
            reservation_sha256=args.reservation_sha256, evidence_path=args.evidence)
