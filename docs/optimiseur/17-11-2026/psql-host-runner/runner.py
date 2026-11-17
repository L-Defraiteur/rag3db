"""Source runner psql HOST fixture. Aucun CLI ni grant implicite.

Import : constantes et primitives v1 figées seulement. La factory native ne
s'utilise qu'après admission externe Common/lease et GO contact distinct.
"""
import hashlib
import importlib.util
import json
import math
import os
from pathlib import Path
import re
import selectors
import signal
import stat
import subprocess
import time


class RunnerUnavailable(RuntimeError):
    """Conservé pour le témoin de capacité initial, pas un verdict positif."""


class CaptureRefused(ValueError):
    def __init__(self, record):
        self.record = record
        super().__init__('bounded child capture refused')


def require(ok, message):
    if not ok:
        raise ValueError(message)


def finite(value):
    return type(value) in (int, float) and math.isfinite(value)


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


V1_SHA = 'de69c498b1ac38d80d4976144fea79ddbfa7d3a34ea8ce7dae1d9e4a457aeed6'
base = Path(__file__).resolve().parent
legacy = base/'frozen-v1'
if not legacy.exists():
    legacy = base.parents[1]/'16-11-2026/pg-ro-attestation'
require(sha(legacy/'pg_receipt.py') == V1_SHA, 'frozen v1 source differs')
spec = importlib.util.spec_from_file_location('psql_frozen_v1', legacy/'pg_receipt.py')
v1 = importlib.util.module_from_spec(spec)
spec.loader.exec_module(v1)
IDENTITY = (legacy/'identity.sql').read_bytes()
POSTCHECK = (legacy/'backend-postcheck.sql').read_bytes()
require(hashlib.sha256(IDENTITY).hexdigest() ==
        'f9496688a506085562a071ce25827a2a683bccd457c9edf6e944e228b7c29328', 'identity SQL pin')
require(hashlib.sha256(POSTCHECK).hexdigest() ==
        '2907a86d971770275b9ff6291e9bffebdf4467d23c7c59ff2015ae6a3fb9c864', 'postcheck SQL pin')
READONLY = (legacy/'readonly-session-v2.sql').read_bytes()
require(hashlib.sha256(READONLY).hexdigest() == 'f40eb6ebc867839303aa51229497e7d533c6708854bea9b63bcfe67f2012b8b9', 'RO SQL pin')
# Réemploi du SQL existant : mesures GUC/rôles dans la session de la requête.
# La source et son SHA doivent figurer au paquet avant toute future entrée.
sync_directory = v1._sync_directory
MAX_BYTES = v1.MAX_BYTES


def validate_profile(p):
    v1.shape(p, set('kind nonce fixture_nonce host port database user expected_identity'.split()), 'fixture profile')
    require(p['kind'] == 'psql-private-fixture-profile-1', 'not a fixture profile')
    require(v1.hex_value(p['nonce'], 32) and v1.hex_value(p['fixture_nonce'], 32), 'fixture nonce')
    require(p['host'] == '127.0.0.1' and type(p['port']) is int and p['port'] == 15433, 'fixture binding')
    require(p['database'] == 'g_pg_probe_'+p['fixture_nonce'][:12] and p['user'] == 'g_pg_observer', 'fixture target')
    i = p['expected_identity']
    v1.shape(i, v1.SQL-{'backend_pid', 'application_name'}, 'external SQL fixture identity')
    v1.sql_shape(dict(i, backend_pid=1, application_name='external-fixture'))
    require(i['database'] == p['database'] and i['user'] == i['session_user'] == p['user'], 'identity target')
    require(i['owner'] != p['user'] and i['owner_oid'] != i['user_oid'], 'fixture must be nonowner')
    require(type(i['server_port']) is int and i['server_port'] == 5432, 'server port')
    return p


def validate_session(raw, p, *, identity=None):
    validate_profile(p)
    require(type(raw) is bytes and len(raw) <= MAX_BYTES, 'session output bound')
    lines = raw.splitlines()
    require(len(lines) == 2 and all(lines), 'expected exactly RO and SQL rows')
    ro, row = [v1.decode(line) for line in lines]
    v1.shape(ro, set('transaction_read_only default_transaction_read_only superuser createdb createrole replication bypassrls database_owner user database backend_pid'.split()), 'readonly row')
    require(ro['transaction_read_only'] is True and ro['default_transaction_read_only'] is True, 'RO not measured')
    require(all(ro[k] is False for k in ['superuser', 'createdb', 'createrole', 'replication', 'bypassrls', 'database_owner']), 'privileged fixture role')
    require(v1.positive(ro['backend_pid']) and ro['user'] == p['user'] and ro['database'] == p['database'], 'RO target')
    app = 'g-pg-'+p['nonce']
    if identity is None:
        v1.sql_shape(row)
        require(row == dict(p['expected_identity'], backend_pid=ro['backend_pid'], application_name=app), 'SQL identity differs')
    else:
        v1.sql_shape(identity)
        v1.shape(row, v1.CHECK_SQL, 'postcheck SQL')
        require(v1.positive(row['backend_pid']) and v1.positive(row['observer_backend_pid']), 'postcheck PID types')
        require(row['backend_absent'] is True and row['observer_backend_pid'] != identity['backend_pid'], 'backend absence')
        require(row == dict(backend_pid=identity['backend_pid'], application_name=app, backend_absent=True,
                observer_backend_pid=ro['backend_pid'], observer_application_name=app+':check'), 'postcheck attribution')
    return row


def capture(argv, *, env, stdin, deadline, clock=time.monotonic):
    """Nonblocking pipes, owned session, no retry, cleanup reserve INSIDE END.

    A retained EOF after direct child exit is partial, never rc0 success. No
    communicate()/wait() without bound. Unknown descendants keep failure red;
    the outer Common is still required to attest scope-wide quiescence.
    """
    started = clock()
    require(finite(deadline) and started < deadline-.15, 'no child cleanup reserve')
    require(type(stdin) is bytes and len(stdin) <= MAX_BYTES, 'stdin bound')
    record = dict(stdout=b'', stderr=b'', returncode=None, pid=None, start=None,
        exe=None, maps=None, observed_unknown=True, reaped=False, partial=False,
        truncated=False, timed_out=False, started_monotonic=started, ended_monotonic=None)
    child = None
    pidfd = None
    selector = None
    failure = None
    record.update(first_error=None, cleanup_errors=[])
    pending = memoryview(stdin)
    closing = False
    killed = False
    term_at = None
    work_end = deadline-.12
    try:
        child = subprocess.Popen(argv, env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            stderr=subprocess.PIPE, start_new_session=True, close_fds=True)
        record['pid'] = child.pid
        selector = selectors.DefaultSelector()
        pidfd = os.pidfd_open(child.pid)
        try:
            proc = Path('/proc')/str(child.pid)
            record.update(start=(proc/'stat').read_text().rsplit(')', 1)[1].split()[19],
                exe=os.readlink(proc/'exe'), maps=(proc/'maps').read_text()[:16384], maps_capture='prefix-at-most-16384-not-complete-inventory', observed_unknown=False)
        except (OSError, IndexError):
            pass  # Explicit UNKNOWN, never fabricated process identity.
        for stream, name in [(child.stdout, 'stdout'), (child.stderr, 'stderr')]:
            os.set_blocking(stream.fileno(), False)
            selector.register(stream, selectors.EVENT_READ, name)
        os.set_blocking(child.stdin.fileno(), False)
        if pending:
            selector.register(child.stdin, selectors.EVENT_WRITE, 'stdin')
        else:
            child.stdin.close()
        while True:
            now = clock()
            code = child.poll()
            open_outputs = any(k.data != 'stdin' for k in selector.get_map().values())
            if code is not None and not open_outputs:
                record['returncode'] = code
                record['reaped'] = True
                break
            if code is not None and open_outputs:
                # Drain ready bytes first, then require immediate EOF below.
                closing = True
            if now >= work_end and not closing:
                record['timed_out'] = True
                closing = True
            if closing and term_at is None:
                term_at = now
                if code is None:
                    signal.pidfd_send_signal(pidfd, signal.SIGTERM)
            if closing and code is None and not killed and now >= term_at+.03:
                signal.pidfd_send_signal(pidfd, signal.SIGKILL)
                killed = True
            if now >= deadline:
                record['timed_out'] = True
                break
            events = selector.select(min(.01, max(0, deadline-now)))
            for key, _ in events:
                stream, name = key.fileobj, key.data
                if name == 'stdin':
                    try:
                        written = os.write(stream.fileno(), pending)
                        pending = pending[written:]
                    except BrokenPipeError:
                        pending = memoryview(b'')
                        record['partial'] = True
                    if not pending:
                        selector.unregister(stream)
                        stream.close()
                    continue
                while True:
                    try:
                        chunk = os.read(stream.fileno(), 8192)
                    except BlockingIOError:
                        break
                    if not chunk:
                        selector.unregister(stream)
                        stream.close()
                        break
                    room = MAX_BYTES-len(record[name])
                    record[name] += chunk[:room]
                    if len(chunk) > room:
                        record['truncated'] = True
                        closing = True
                        break

            if code is not None and any(k.data != 'stdin' for k in selector.get_map().values()):
                record['partial'] = True
        if pending:
            record['partial'] = True
    except BaseException as error:
        failure = error
        record['first_error'] = type(error).__name__
        record['partial'] = True
    finally:
        # Every cleanup step is best-effort and recorded separately. Never
        # replace the first cause; never claim reap merely because cleanup ran.
        def cleanup_error(error):
            record['cleanup_errors'].append(type(error).__name__)
        if child is not None:
            alive = True
            try:
                alive = child.poll() is None
            except BaseException as error:
                cleanup_error(error)
            if alive:
                try:
                    if pidfd is not None:
                        signal.pidfd_send_signal(pidfd, signal.SIGKILL)
                    else:
                        child.kill()  # Exactly owned direct child, not yet waited.
                except BaseException as error:
                    cleanup_error(error)
                remaining = deadline-clock()
                if remaining > 0:
                    try:
                        child.wait(timeout=remaining)
                    except BaseException as error:
                        cleanup_error(error)
            record['returncode'] = child.returncode
            record['reaped'] = child.returncode is not None
            for stream in [child.stdin, child.stdout, child.stderr]:
                try:
                    if stream is not None and not stream.closed:
                        stream.close()
                except BaseException as error:
                    cleanup_error(error)
        if selector is not None:
            try:
                selector.close()
            except BaseException as error:
                cleanup_error(error)
        if pidfd is not None:
            try:
                os.close(pidfd)
            except BaseException as error:
                cleanup_error(error)
        record['ended_monotonic'] = clock()
    if (failure is not None or record['cleanup_errors'] or not record['reaped'] or
            record['partial'] or record['truncated'] or record['timed_out'] or
            record['ended_monotonic'] > deadline):
        raise CaptureRefused(record) from failure
    return record


def _encode(value):
    return (json.dumps(value, sort_keys=True, allow_nan=False)+'\n').encode()


def observe(profile, out, *, session_factory, clock, phase_check):
    """Fixture publication only. No v1 grant, no authority discovered implicitly.

    The external launcher MUST supply a qualified Common/lease checker and the
    separately admitted native factory. Injection in CPU tests is not admission.
    """
    start = clock()
    deadline = start+5
    out = Path(out).absolute()
    require(out.resolve() == out and not any(x in ('.env', '.vault') for x in out.parts), 'artifact path')
    out.mkdir(mode=0o700)
    phase = None
    captures = []
    first_error = None
    cleanup_error = None
    def gate():
        nonlocal phase, deadline
        current = phase_check()
        v1.shape(current, {'active', 'first_red', 'common_end'}, 'phase')
        require(current['active'] is True and current['first_red'] is False and finite(current['common_end']), 'inactive phase')
        if phase is None:
            phase = current.copy()
            deadline = min(deadline, current['common_end'])
        require(current == phase and clock() < deadline, 'original phase/whole5 expired')
    try:
        gate()
        validate_profile(profile)
        session = session_factory(deadline)
        identity = None
        try:
            gate()
            for name in ['identity', 'postcheck']:
                call_start = clock()
                gate()
                try:
                    rec = session(identity=identity)
                except CaptureRefused as error:
                    first_error = error
                    rec = error.record
                    v1._write_exclusive(out/(name+'.capture.json'), _encode({k:v for k,v in rec.items() if k not in ('stdout','stderr')}))
                    for channel in ['stdout', 'stderr']:
                        data = rec.get(channel, b'')
                        if type(data) is bytes:
                            v1._write_exclusive(out/(name+'.'+channel), data[:MAX_BYTES])
                    raise
                captures.append(rec)
                v1._write_exclusive(out/(name+'.capture.json'), _encode({k:v for k,v in rec.items() if k not in ('stdout','stderr')}))
                for channel in ['stdout', 'stderr']:
                    require(type(rec.get(channel)) is bytes, 'missing capture bytes')
                    v1._write_exclusive(out/(name+'.'+channel), rec[channel][:MAX_BYTES])
                require(type(rec['returncode']) is int and rec['returncode'] == 0 and
                    rec['reaped'] is True and rec['partial'] is False and
                    rec['truncated'] is False and rec['timed_out'] is False, 'capture failure')
                require(finite(rec['started_monotonic']) and finite(rec['ended_monotonic']) and
                    call_start <= rec['started_monotonic'] <= rec['ended_monotonic'] <= clock() < deadline, 'capture outside whole5')
                require(rec['stderr'] == b'', 'SQL stderr not empty')
                row = validate_session(rec['stdout'], profile, identity=identity)
                if identity is None:
                    identity = row
                gate()
        except BaseException as error:
            if first_error is None:
                first_error = error
            raise
        finally:
            closer = getattr(session, 'close', None)
            if closer is not None:
                try:
                    closer()  # Passfile removal/fsync inside original whole5.
                except BaseException as error:
                    cleanup_error = error
                    if first_error is None:
                        raise
        gate()
        files = {p.name: dict(path=str(p), sha256=sha(p), size=p.stat().st_size)
                 for p in out.iterdir() if p.is_file() and p.suffix in ('.stdout','.stderr')}
        body = dict(kind='psql-private-fixture-observation-1', profile=profile,
            identity=identity, postcheck=row, started_monotonic=start,
            deadline_monotonic=deadline, ended_monotonic=clock(),
            query_sha256=v1.QUERY_SHA256, postcheck_sha256=v1.POSTCHECK_SHA256,
            readonly_sql_sha256=hashlib.sha256(READONLY).hexdigest(),
            captures=[{k: v for k, v in c.items() if k not in ('stdout', 'stderr')} for c in captures],
            raw_files=files, transport_qualified=False, grant_authorized=False)
        gate()
        sync_directory(out)
        gate()
        payload = _encode(body)
        require(len(payload) <= MAX_BYTES, 'observation frame bound')
        pending = out/'observation.pending.json'
        v1._write_exclusive(pending, payload)
        sync_directory(out)
        gate()
        target = out/'observation.json'
        os.link(pending, target)
        digest = hashlib.sha256(payload).hexdigest()
        v1._write_exclusive(out/'observation.sha256', (digest+'\n').encode())
        sync_directory(out)
        gate()  # Final fsync included. A refusal marker invalidates published bytes.
        completed = dict(kind='psql-fixture-publication-complete-1',
            artifact_sha256=digest, completed_monotonic=clock(), deadline_monotonic=deadline)
        v1._write_exclusive(out/'publication-complete.json', _encode(completed))
        sync_directory(out)
        gate()  # A terminal external receipt is still mandatory after this return.
        return target, digest
    except BaseException as error:
        refusal = dict(verdict='RED', reason=type(first_error or error).__name__,
            first_error=type(first_error or error).__name__,
            cleanup_error=type(cleanup_error).__name__ if cleanup_error is not None else None,
            observed_monotonic=clock(), artifact_published=(out/'observation.json').exists(),
            grant_authorized=False)
        v1._write_exclusive(out/'refusal.json', _encode(refusal))
        sync_directory(out)
        raise ValueError('fixture observation refused') from error


def read_fixture_observation(path, digest, profile, *, terminal_receipt=None, terminal_sha256=None):
    """Requires externally pinned TERMINAL admission, never writer-only evidence.

    Only the separate, reviewed supervisor may produce/admit this terminal
    receipt after successful command AND Common termination. CPU fake receipts
    are not real admission. No producer of such a runtime receipt is qualified
    by this source; no inference from a mutable caller boolean.
    """
    path = Path(path).absolute()
    require(terminal_receipt is not None and v1.hex_value(terminal_sha256, 64), 'missing terminal admission')
    terminal_path = Path(terminal_receipt).absolute()
    terminal_raw = v1._read_regular(terminal_path, terminal_path.parent)
    require(hashlib.sha256(terminal_raw).hexdigest() == terminal_sha256, 'terminal SHA differs')
    terminal = v1.decode(terminal_raw)
    v1.shape(terminal, set('kind artifact_path artifact_sha256 common_state common_verdict common_code common_remaining common_cleaned publication_complete_sha256 observed_monotonic'.split()), 'terminal receipt')
    require(terminal['kind'] == 'psql-fixture-terminal-admission-1' and
        terminal['artifact_path'] == str(path) and terminal['artifact_sha256'] == digest and
        terminal['common_state'] == 'termine' and terminal['common_verdict'] == 'VERT' and
        type(terminal['common_code']) is int and terminal['common_code'] == 0 and
        terminal['common_remaining'] == [] and terminal['common_cleaned'] is True and
        finite(terminal['observed_monotonic']), 'terminal scope not successfully completed')
    require(not (path.parent/'refusal.json').exists(), 'refused observation')
    sidecar = v1._read_regular(path.parent/'observation.sha256', path.parent)
    require(sidecar == (digest+'\n').encode(), 'missing or different artifact sidecar')
    complete_raw = v1._read_regular(path.parent/'publication-complete.json', path.parent)
    require(hashlib.sha256(complete_raw).hexdigest() == terminal['publication_complete_sha256'], 'completion SHA differs')
    complete = v1.decode(complete_raw)
    v1.shape(complete, {'kind','artifact_sha256','completed_monotonic','deadline_monotonic'}, 'completion')
    require(complete['kind'] == 'psql-fixture-publication-complete-1' and complete['artifact_sha256'] == digest and
        finite(complete['completed_monotonic']) and finite(complete['deadline_monotonic']) and
        complete['completed_monotonic'] < complete['deadline_monotonic'] and
        complete['completed_monotonic'] <= terminal['observed_monotonic'], 'invalid publication completion')
    raw = v1._read_regular(path, path.parent)
    require(hashlib.sha256(raw).hexdigest() == digest, 'observation SHA')
    body = v1.decode(raw)
    require(body['kind'] == 'psql-private-fixture-observation-1' and body['profile'] == profile and
        body['grant_authorized'] is False and body['transport_qualified'] is False, 'fixture evidence only')
    validate_profile(profile)
    require(body['deadline_monotonic'] == complete['deadline_monotonic'] and
        body['started_monotonic'] <= body['ended_monotonic'] <= complete['completed_monotonic'], 'completion outside observation')
    identity = validate_session(v1._read_regular(path.parent/'identity.stdout', path.parent), profile)
    check = validate_session(v1._read_regular(path.parent/'postcheck.stdout', path.parent), profile, identity=identity)
    require(identity == body['identity'] and check == body['postcheck'], 'raw identity differs')
    require(set(body['raw_files']) == {'identity.stdout','identity.stderr','postcheck.stdout','postcheck.stderr'}, 'raw inventory')
    for name, item in body['raw_files'].items():
        require(item['path'] == str(path.parent/name), 'raw path differs')
        raw_file = v1._read_regular(path.parent/name, path.parent)
        require(hashlib.sha256(raw_file).hexdigest() == item['sha256'] and len(raw_file) == item['size'], 'raw changed')
        if name.endswith('.stderr'):
            require(raw_file == b'', 'stderr differs')
    require(not (path.parent/'refusal.json').exists() and
        v1._read_regular(terminal_path, terminal_path.parent) == terminal_raw and
        v1._read_regular(path.parent/'publication-complete.json', path.parent) == complete_raw, 'changed/refused during read')
    return body


def validate_native_attestation(record, expected, *, now):
    """Pure semantic validation of a PARENT-supplied held-process observation.

    This function cannot attest the OS itself. A separately qualified collector
    must obtain the values while its own child is held after the readonly row.
    Synthetic CPU records do not qualify that collector or a PG transport.
    """
    proof=record.get('native_attestation')
    require(type(proof) is dict, 'native evidence missing')
    v1.shape(proof,set('schema_version stage pid start cgroup kernel_entry maps map_files environment readonly_stdout_sha256 readonly_backend_pid observed_monotonic alive_before alive_after'.split()),'native schema')
    require(type(proof['schema_version']) is int and proof['schema_version']==1,'native schema')
    require(record.get('observed_unknown') is False and v1.positive(record.get('pid')) and
        type(record.get('start')) is str and record['start'].isdigit() and int(record['start'])>0 and
        type(proof['pid']) is int and proof['pid']==record['pid'] and
        type(proof['start']) is str and proof['start']==record['start'] and
        record.get('exe')==expected['kernel_path'],'native identity')
    require(proof['stage']=='held-after-readonly' and proof['alive_before'] is True and
        proof['alive_after'] is True,'native stage')
    require(proof['cgroup']==expected['cgroup'],'native cgroup')
    require(type(proof['environment']) is dict and proof['environment']==expected['environment'],'native environment')
    require(all(finite(x) for x in [record['started_monotonic'],proof['observed_monotonic'],
        record['ended_monotonic'],now,expected['call_started'],expected['deadline']]) and
        expected['call_started']<=record['started_monotonic']<=proof['observed_monotonic']<=
        record['ended_monotonic']<=now<expected['deadline'],'native timing')
    require(type(record.get('stdout')) is bytes and len(record['stdout'])<=MAX_BYTES,'native readonly binding')
    lines=record['stdout'].splitlines()
    require(len(lines)==2 and hashlib.sha256(lines[0]).hexdigest()==proof['readonly_stdout_sha256'] and
        v1.positive(proof['readonly_backend_pid']) and
        v1.decode(lines[0]).get('backend_pid')==proof['readonly_backend_pid'],'native readonly binding')
    files=expected['allowed_files']
    require(type(files) is dict and expected['kernel_path'] in files and expected['program_path'] in files,
        'native expected files missing')
    meta_keys={'sha256','device_major','device_minor','inode'}
    def file_identity(meta):
        return (type(meta.get('device_major')) is int and meta['device_major']>=0 and
            type(meta.get('device_minor')) is int and meta['device_minor']>=0 and
            v1.positive(meta.get('inode')) and v1.hex_value(meta.get('sha256'),64))
    v1.shape(proof['kernel_entry'],meta_keys|{'path'},'native kernel')
    require(proof['kernel_entry']==dict(path=expected['kernel_path'],**files[expected['kernel_path']]) and
        file_identity(proof['kernel_entry']),'native kernel')
    raw_maps=proof['maps']
    require(type(raw_maps) is str and 0<len(raw_maps.encode())<=MAX_BYTES and raw_maps.endswith('\n'),
        'native maps frame')
    mapped={};previous_end=0
    for line in raw_maps.splitlines():
        fields=line.split(None,5)
        try:
            require(len(fields)>=5,'native maps syntax')
            begin,end=[int(x,16) for x in fields[0].split('-')]
            offset=int(fields[2],16)
            major,minor=[int(x,16) for x in fields[3].split(':')]
            inode=int(fields[4])
            require(begin>=previous_end and end>begin and offset>=0 and major>=0 and minor>=0 and inode>=0 and
                re.fullmatch('[r-][w-][x-][ps]',fields[1]) is not None,'native maps syntax')
            previous_end=end
        except (ValueError,IndexError) as error:
            raise ValueError('native maps syntax') from error
        path=fields[5] if len(fields)==6 else ''
        if path.startswith('/'):
            require(path in files and not path.endswith(' (deleted)'),'native map outside pinned bundle')
            require(inode>0,'native map identity')
            identity=(major,minor,inode)
            require(path not in mapped or mapped[path]==identity,'native map identity')
            mapped[path]=identity
        else:
            require(inode==0 and (not path or path.startswith('[') and path.endswith(']')),'native map outside pinned bundle')
    require(type(proof['map_files']) is list and len(proof['map_files'])<=len(files),'native map identity')
    listed={}
    for item in proof['map_files']:
        v1.shape(item,meta_keys|{'path'},'native map identity')
        path=item['path']
        require(type(path) is str and path not in listed and path in mapped and path in files,
            'native map identity')
        require(file_identity(item) and {k:item[k] for k in meta_keys}==files[path] and
            (item['device_major'],item['device_minor'],item['inode'])==mapped[path],'native map identity')
        listed[path]=item['sha256']
    require(set(listed)==set(mapped),'native map identity')
    require(expected['kernel_path'] in listed and expected['program_path'] in listed and
        expected['required_shas'].issubset(set(listed.values())),'native required maps')
    return proof


class PsqlSession:
    """Native fixture session factory product; SOURCE only until separate GO.

    No discovery of runtime/credentials/PG. authority must validate the actual
    original Common and fresh external allocation; a dictionary alone cannot
    qualify that launcher. Neither this class nor its evidence grants admission.
    """
    def __init__(self, profile, *, bundle_root, ledger_path, ledger_sha256,
                 fixture_cache, credential_reference, private_root, deadline,
                 authority, clock=time.monotonic):
        import resource
        self.clock = clock
        self.deadline = deadline
        self.authority = authority
        self.profile = validate_profile(profile)
        self.profile_sha256 = hashlib.sha256(_encode(profile)).hexdigest()
        self.check_authority()
        require(resource.getrlimit(resource.RLIMIT_CORE) == (0, 0), 'core dumps not disabled by launcher')
        self.bundle = Path(bundle_root).absolute()
        ledger_path = Path(ledger_path).absolute()
        require(ledger_path.resolve() == ledger_path and sha(ledger_path) == ledger_sha256,
                'ledger missing or differs')
        # The only currently qualified tool, not an arbitrary supplied binary.
        require(ledger_sha256 == '824b5913054898e735d88980cd9eb9505dbb0364d8384bfdcadf49c2d076c4a6', 'unqualified CLI ledger')
        ledger = v1.decode(ledger_path.read_bytes())
        require(self.bundle.resolve() == self.bundle and self.bundle.name == 'bundle', 'bundle location')
        self.native_map_files={}
        for name, rec in ledger['files'].items():
            q = self.bundle/name
            require(q.is_relative_to(self.bundle) and q.resolve() == q and q.is_file() and
                    q.stat().st_uid == os.getuid() and q.stat().st_size == rec['size'] and
                    sha(q) == rec['sha256'], 'bundle file pin')
            info=q.stat()
            self.native_map_files[str(q)]=dict(sha256=rec['sha256'],device_major=os.major(info.st_dev),
                device_minor=os.minor(info.st_dev),inode=info.st_ino)
            self.check_authority()
        self.program = self.bundle/'bin/psql'
        self.loader = self.bundle/'loader/ld-musl-x86_64.so.1'
        require(str(self.program) == ledger['program_elf']['path'] and
                str(self.loader) == ledger['kernel_entry_requested']['path'], 'bundle relocation not qualified')
        self.program_sha256 = sha(self.program)
        self.loader_sha256 = sha(self.loader)
        self.native_required_map_shas={x['sha256'] for x in ledger['provenance'].values()}|{self.program_sha256,self.loader_sha256}
        self.check_authority()
        cache = Path(fixture_cache).absolute()
        credential = Path(credential_reference).absolute()
        allocated = cache/('allocation-'+profile['fixture_nonce'][:12])
        require(credential == allocated/'credentials/observer-password', 'credential not allocated fixture')
        for directory in [cache, allocated, allocated/'credentials']:
            info = directory.stat()
            require(directory.resolve() == directory and stat.S_ISDIR(info.st_mode) and
                    info.st_uid == os.getuid() and stat.S_IMODE(info.st_mode) == 0o700, 'credential directory')
        require(not any(x in ('.env', '.vault') for x in credential.parts), 'forbidden credential')
        fd = os.open(credential, os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC)
        try:
            info = os.fstat(fd)
            require(stat.S_ISREG(info.st_mode) and info.st_uid == os.getuid() and
                    stat.S_IMODE(info.st_mode) == 0o600 and info.st_nlink == 1 and info.st_size == 65, 'credential permissions')
            raw = os.read(fd, 66)
        finally:
            os.close(fd)
        require(re.fullmatch(b'[0-9a-f]{64}\n', raw) is not None, 'credential format')
        self.secret = raw[:-1]
        self.check_authority()
        self.private = Path(private_root).absolute()
        require(self.private.resolve() == self.private and self.private.is_relative_to(cache.parent) and
                not any(x in ('.env', '.vault') for x in self.private.parts), 'private native directory')
        self.private.mkdir(mode=0o700)
        self.pgpass = self.private/'pgpass'
        line = f"127.0.0.1:15433:{profile['database']}:{profile['user']}:".encode()+self.secret+b'\n'
        self.home = self.private/'home'
        self.tmp = self.private/'tmp'
        self.home.mkdir(mode=0o700)
        self.tmp.mkdir(mode=0o700)
        self.check_authority()
        try:
            v1._write_exclusive(self.pgpass, line)
            self.check_authority()
        except BaseException:
            if self.pgpass.exists():
                self.close()
            raise

    def check_authority(self):
        a = self.authority()
        v1.shape(a, set('active first_red common_end lease_end fixture_nonce profile_sha256 common_cgroup'.split()), 'native authority')
        require(a['active'] is True and a['first_red'] is False and
                finite(a['common_end']) and finite(a['lease_end']) and
                self.clock() < self.deadline <= a['common_end'] <= a['lease_end'] and
                a['fixture_nonce'] == self.profile['fixture_nonce'] and
                a['profile_sha256'] == self.profile_sha256, 'native authority/lease refused')
        group=a['common_cgroup']
        require(type(group) is str and group.startswith('/') and '\n' not in group and '\r' not in group and
            '..' not in Path(group).parts,'native cgroup authority')
        require(not hasattr(self,'native_cgroup') or self.native_cgroup==group,'native cgroup authority changed')
        self.native_cgroup=group

    def __call__(self, *, identity=None):
        self.check_authority()
        p = self.profile
        app = 'g-pg-'+p['nonce']
        if identity is not None:
            v1.sql_shape(identity)
            require(identity == dict(p['expected_identity'],backend_pid=identity['backend_pid'],application_name=app), 'postcheck target')
        remaining = self.deadline-self.clock()
        require(remaining > .4, 'no postcheck/cleanup budget')
        # Same original whole5, per-call caps consume rather than reset it.
        millis = max(1, min(1000, int((remaining-.25)*1000)))
        options = ('-c default_transaction_read_only=on -c statement_timeout='+str(millis)+
                   ' -c lock_timeout='+str(millis)+' -c idle_in_transaction_session_timeout='+str(millis))
        if identity is not None:
            options += (' -c g_pg.expected_backend_pid='+str(identity['backend_pid'])+
                        ' -c g_pg.expected_application_name='+app)
        env = dict(PATH='/usr/bin:/bin', HOME=str(self.home), TMPDIR=str(self.tmp), LANG='C', LC_ALL='C',
            PGPASSFILE=str(self.pgpass), PGCONNECT_TIMEOUT='1', PGSSLMODE='disable',
            PGAPPNAME=app+(':check' if identity is not None else ''), PGOPTIONS=options)
        argv = [str(self.loader), '--library-path', str(self.bundle/'lib'), str(self.program),
            '-X', '--no-password', '-qAt', '-v', 'ON_ERROR_STOP=1',
            '-h', '127.0.0.1', '-p', '15433', '-U', p['user'], '-d', p['database'], '-f', '-']
        sql = b'BEGIN READ ONLY;\n'+READONLY+(POSTCHECK if identity is not None else IDENTITY)+b'ROLLBACK;\n'
        call_started=self.clock()
        call_deadline=min(self.deadline,call_started+1.5)
        try:
            rec = capture(argv, env=env, stdin=sql, deadline=call_deadline, clock=self.clock)
        except CaptureRefused as error:
            self.scrub(error.record)
            raise
        self.scrub(rec)
        try:
            require(type(rec.get('native_attestation')) is dict,'native evidence missing')
            require(type(getattr(self,'native_map_files',None)) is dict and
                type(getattr(self,'native_required_map_shas',None)) is set and
                type(getattr(self,'native_cgroup',None)) is str,'native expected configuration missing')
            require(type(rec['returncode']) is int and rec['returncode']==0 and rec['reaped'] is True and
                rec['partial'] is False and rec['truncated'] is False and rec['timed_out'] is False,
                'native capture failed')
            expected=dict(kernel_path=str(self.loader),program_path=str(self.program),
                allowed_files=self.native_map_files,required_shas=self.native_required_map_shas,
                environment=env,cgroup=self.native_cgroup,call_started=call_started,deadline=call_deadline)
            validate_native_attestation(rec,expected,now=self.clock())
            validate_session(rec['stdout'],p,identity=identity)
            require(self.clock()<call_deadline,'native timing')
        except ValueError as error:
            rec['native_refusal']=str(error)
            rec['first_error']=rec.get('first_error') or type(error).__name__
            raise CaptureRefused(rec) from error
        rec['native_attestation_checked']=True  # Semantic contract only, not OS qualification.
        rec['transport_qualified']=False
        rec.update(argv=argv, kernel_entry_requested=str(self.loader), kernel_entry_sha256=self.loader_sha256,
                   program_requested=str(self.program), program_sha256=self.program_sha256,
                   executed_sql_sha256=hashlib.sha256(sql).hexdigest())
        self.check_authority()
        return rec

    def scrub(self, rec):
        # New native metadata must be scrubbed too, before any error record can
        # be archived. Whole unsafe fields are removed/redacted, never copied
        # as a purported complete OS proof. No secret is printed by this path.
        def unsafe(value,depth=0):
            if depth>16:return True
            if type(value) is bytes:return len(value)>MAX_BYTES or self.secret in value
            if type(value) is str:return len(value.encode())>MAX_BYTES or self.secret.decode() in value
            if type(value) is dict:
                return any(unsafe(k,depth+1) or unsafe(v,depth+1) for k,v in value.items())
            if type(value) in (list,tuple):return any(unsafe(v,depth+1) for v in value)
            return False
        bad=[key for key,value in rec.items() if unsafe(key) or unsafe(value)]
        if bad:
            for key in bad:
                if unsafe(key):del rec[key]
                else:rec[key]='[REDACTED unsafe capture metadata]'
            rec['stdout']=b'[REDACTED unsafe capture]\n'
            rec['stderr']=b'[REDACTED unsafe capture]\n'
            rec['partial']=True
            rec['native_refusal']='native secret-bearing or unbounded capture'
            raise CaptureRefused(rec)

    def close(self):
        # Only our newly created passfile, never the provider's original secret.
        self.pgpass.unlink()
        sync_directory(self.private)
        self.secret = b''


if __name__ == '__main__':
    raise SystemExit('SOURCE ONLY: no live entrypoint; separate reviewed launcher/GO required')
