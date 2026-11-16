"""Policy-free fixed health transport. No READY, PG, embedding or service policy.

Caller supplies a pinned worker_gate(context)->None and persists trace in finally.
The only network action here is GET /sante on existing 127.0.0.1:7878 or :7879.
No proxy, redirects, environment dump, shell, sibling process or SDK discovery.
"""
import argparse
import hashlib
import http.client
import importlib.util
import json
import math
import os
from pathlib import Path
import re
import stat
import subprocess
import sys
import time

MAX_BODY = 65536
CLEANUP_RESERVE = .5
IDENTITY_FIELDS = {'service', 'modele', 'dim', 'factice', 'dual', 'sparse',
                   'executable', 'precision', 'max_chars_per_text'}


def decode(raw):
    def unique(pairs):
        value = {}
        for key, item in pairs:
            if key in value: raise ValueError('duplicate health JSON key')
            value[key] = item
        return value
    def nonfinite(value): raise ValueError('nonfinite health JSON scalar')
    def floating(value):
        number = float(value)
        if not math.isfinite(number): raise ValueError('overflowing health JSON number')
        return number
    if isinstance(raw, bytes): raw = raw.decode('utf-8', 'strict')
    return json.loads(raw, object_pairs_hook=unique, parse_constant=nonfinite, parse_float=floating)


def finite(value):
    if type(value) not in (int, float) or not math.isfinite(value) or value < 0:
        raise ValueError('finite monotonic deadline required')
    return value


def validate_health(body, identity9, *, status):
    if type(status) is not int or status != 200: raise ValueError('health HTTP status not 200')
    if not isinstance(body, bytes) or len(body) > MAX_BODY: raise ValueError('health body not bounded bytes')
    if not isinstance(identity9, dict) or set(identity9) != IDENTITY_FIELDS:
        raise ValueError('exact nine expected identity fields required')
    for key in ['service', 'modele', 'executable', 'precision']:
        if not isinstance(identity9[key], str) or not identity9[key]:
            raise ValueError('identity string pin absent')
    for key in ['dim', 'max_chars_per_text']:
        if type(identity9[key]) is not int or identity9[key] <= 0:
            raise ValueError('identity positive integer pin absent')
    if any(type(identity9[k]) is not bool for k in ['factice', 'dual', 'sparse']):
        raise ValueError('identity boolean pin invalid')
    value = decode(body)
    if not isinstance(value, dict) or any(type(value.get(k)) is not type(v) or value[k] != v
                                         for k, v in identity9.items()):
        raise ValueError('typed health identity differs')
    usage = value.get('usage', {})
    if not isinstance(usage, dict) or any(type(usage.get(k)) is not int or usage[k] != 0 for k in ['baux', 'en_cours']):
        raise ValueError('health usage active or unknown')
    return value


def context_check(context, now):
    if not isinstance(context, dict) or context.get('address') not in ['127.0.0.1:7878', '127.0.0.1:7879']:
        raise ValueError('fixed local health address required')
    if type(context.get('admission')) is not bool: raise ValueError('explicit admission mode required')
    due = finite(context.get('contact_deadline'))
    if not now < due <= now+5: raise ValueError('health deadline absent, expired or beyond whole5')
    expected = context.get('expected_process', {})
    if not isinstance(expected, dict) or not {'pid', 'uid', 'start', 'exe', 'exe_sha256'} <= set(expected):
        raise ValueError('complete expected process identity required')
    if type(expected['pid']) is not int or expected['pid'] <= 1 or type(expected['uid']) is not int or expected['uid'] < 0 or not isinstance(expected['start'], str) or not expected['start'].isdigit():
        raise ValueError('typed expected PID/UID/start required')
    if not isinstance(expected['exe'], str) or not expected['exe'].startswith('/') or not isinstance(expected['exe_sha256'], str) or not re.fullmatch('[a-f0-9]{64}', expected['exe_sha256']):
        raise ValueError('expected executable path/hash required')
    source = Path(context['gate_module_path'])
    if not source.is_absolute() or source.resolve() != source or '.vault' in source.parts or '.env' in source.parts:
        raise ValueError('policy source path linked/ambiguous/protected')
    info = source.lstat()
    if not stat.S_ISREG(info.st_mode) or info.st_uid != os.getuid():
        raise ValueError('policy source must be a regular owned file')
    if hashlib.sha256(source.read_bytes()).hexdigest() != context.get('gate_module_sha256'):
        raise ValueError('policy source differs from external pin')
    return due


def worker_gate(context):
    """Load only the caller's pinned policy; no interpretation of its other data."""
    source = Path(context['gate_module_path'])
    spec = importlib.util.spec_from_file_location('pinned_health_policy', source)
    module = importlib.util.module_from_spec(spec)
    before = list(sys.path)
    try:
        sys.path.insert(0, str(source.parent))
        spec.loader.exec_module(module)
        result = module.worker_gate(context)
        if result is not None: raise ValueError('policy gate must return None on acceptance')
    finally:
        sys.path[:] = before


def worker(context):
    due = context_check(context, time.monotonic())
    work_due = finite(context['work_deadline_monotonic'])
    if work_due > due-CLEANUP_RESERVE: raise ValueError('worker deadline consumes cleanup reserve')
    worker_gate(context)  # Caller policy checks phase/process immediately before GET.
    now = time.monotonic()
    if now >= work_due: raise ValueError('health deadline exhausted by policy/imports')
    connection = http.client.HTTPConnection('127.0.0.1', int(context['address'].rsplit(':', 1)[1]),
                                            timeout=work_due-now)
    def event(value):
        print(json.dumps(value, allow_nan=False), file=sys.stderr, flush=True)
    status = None
    body = bytearray()
    try:
        event(dict(event='health-request-start', address=context['address'], method='GET', path='/sante'))
        connection.request('GET', '/sante')
        response = connection.getresponse()
        status = response.status
        event(dict(event='health-status', status=status))
        while len(body) <= MAX_BODY:
            if time.monotonic() >= work_due: raise TimeoutError('health body deadline expired')
            chunk = response.read1(min(4096, MAX_BODY+1-len(body)))
            if not chunk:
                remaining = getattr(response, 'length', None)
                if type(remaining) is int and remaining > 0:
                    raise ValueError('health body closed before declared length')
                break
            chunk = chunk[:MAX_BODY+1-len(body)]
            body.extend(chunk)
            event(dict(event='health-body-chunk', raw_hex=chunk.hex(), received=len(body)))
            if len(body) > MAX_BODY: break
        oversized = len(body) > MAX_BODY
        print(json.dumps(dict(status=status, raw_hex=body.hex(),
                              oversize=oversized, prefix_only=oversized)), flush=True)
        if oversized: raise ValueError('health prefix exceeds body bound')
        if time.monotonic() >= work_due: raise ValueError('whole health worker deadline expired')
    except BaseException as error:
        partial = getattr(error, 'partial', None)
        if isinstance(partial, (bytes, bytearray)):
            prefix = bytes(partial[:MAX_BODY+1])
            event(dict(event='health-exception-partial', raw_hex=prefix.hex(),
                       reported_partial_bytes=len(partial), prefix_only=len(partial)>len(prefix),
                       deduplication='not_attempted', body_complete=False))
        event(dict(event='health-failure', error_type=type(error).__name__,
                   status=status, received=len(body)))
        try:
            connection.close()
        except BaseException as cleanup_error:
            event(dict(event='health-close-failure', error_type=type(cleanup_error).__name__))
        raise
    else:
        connection.close()


def run_health_worker(context, trace):
    started = time.monotonic()
    trace.update(started_monotonic=started, stdout_hex='', stderr_hex='', returncode=None,
                 timed_out=False, client_reaped=False, cleanup_verified=False, worker_created=False)
    due = context_check(context, started)
    trace['deadline_monotonic'] = due
    work_due = due-CLEANUP_RESERVE
    if time.monotonic() >= work_due:
        raise ValueError('health budget lacks cleanup reserve before spawn')
    child_context = dict(context, work_deadline_monotonic=work_due)
    trace['work_deadline_monotonic'] = work_due
    child = subprocess.Popen([sys.executable, '-B', str(Path(__file__).resolve()), '--worker'],
                             stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    trace.update(worker_pid=child.pid, worker_created=True)
    try:
        remaining = work_due-time.monotonic()
        if remaining <= 0: raise subprocess.TimeoutExpired(child.args, 0)
        out, err = child.communicate(json.dumps(child_context, allow_nan=False).encode(), timeout=remaining)
    except BaseException as error:
        trace['timed_out'] = isinstance(error, subprocess.TimeoutExpired)
        trace['first_error_type'] = type(error).__name__
        if trace['timed_out']:
            trace.update(timeout_stdout_hex=(error.output or b'').hex(),
                         timeout_stderr_hex=(error.stderr or b'').hex())
        trace['cleanup_errors'] = []
        try:
            child.kill()
        except BaseException as cleanup_error:
            trace['cleanup_errors'].append(type(cleanup_error).__name__)
        remaining = min(CLEANUP_RESERVE, max(0., due-time.monotonic()))
        if remaining > 0:
            try:
                out, err = child.communicate(timeout=remaining)
                trace.update(stdout_hex=out.hex(), stderr_hex=err.hex(), returncode=child.returncode,
                             client_reaped=type(child.returncode) is int,
                             cleanup_verified=type(child.returncode) is int)
            except BaseException as cleanup_error:
                trace['cleanup_errors'].append(type(cleanup_error).__name__)
                if isinstance(cleanup_error, subprocess.TimeoutExpired):
                    trace.update(cleanup_stdout_hex=(cleanup_error.output or b'').hex(),
                                 cleanup_stderr_hex=(cleanup_error.stderr or b'').hex())
        else:
            trace['cleanup_errors'].append('DeadlineExhausted')
        if not trace['client_reaped']:
            trace['returncode'] = child.returncode
            error.add_note('worker reap unknown; original refusal preserved; inspect worker_pid in trace')
        trace['finished_monotonic'] = time.monotonic()
        raise
    trace.update(stdout_hex=out.hex(), stderr_hex=err.hex(), returncode=child.returncode,
                 client_reaped=True, cleanup_verified=True, finished_monotonic=time.monotonic())
    if child.returncode or time.monotonic() >= due:
        raise ValueError('health worker failed or exceeded whole budget')
    response = decode(out)
    if type(response.get('status')) is not int or not isinstance(response.get('raw_hex'), str):
        raise ValueError('health worker response invalid')
    body = bytes.fromhex(response['raw_hex'])
    if len(body) > MAX_BODY or response.get('oversize') is not False or response.get('prefix_only') is not False:
        raise ValueError('health worker response incomplete or oversized')
    trace['returned_monotonic'] = time.monotonic()
    if trace['returned_monotonic'] >= due:
        raise ValueError('whole health budget expired during response validation')
    return dict(status=response['status'], body=body)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--worker', action='store_true', required=True)
    parser.parse_args()
    worker(decode(sys.stdin.buffer.read(524289)))
