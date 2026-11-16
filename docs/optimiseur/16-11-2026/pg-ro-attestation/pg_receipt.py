"""Contrat PG offline : aucun réseau, subprocess, Docker ou découverte de pins."""
import copy
import hashlib
import json
import math
import os
from pathlib import Path
import re
import stat

QUERY_SHA256 = hashlib.sha256(Path(__file__).with_name('identity.sql').read_bytes()).hexdigest()
POSTCHECK_SHA256 = hashlib.sha256(Path(__file__).with_name('backend-postcheck.sql').read_bytes()).hexdigest()
MAX_BYTES = 65536
TOP = set('schema_version kind app pilot scope c7_reference g_reference reservation_reference nonce ready_sha256 observed_monotonic valid_until_monotonic exclusive private_test active topology sql children_policy evidence'.split())
BINDINGS = 'app pilot scope c7_reference g_reference reservation_reference nonce ready_sha256 topology children_policy'.split()
EXPECTED = set(BINDINGS) | {'tool_qualified','allocation_active','sql_identity','observer_source_sha256','client_bundle_sha256'}
SQL = set('database user database_oid user_oid owner owner_oid session_user server_address server_port server_version_num postmaster_start backend_pid application_name'.split())
EVIDENCE = set('query_sha256 observer_source_sha256 transport client_bundle_sha256 started_monotonic deadline_monotonic ended_monotonic returncode partial truncated client_reaped quiet backend_absent read_only stdout stderr postcheck'.split())
POSTCHECK = set('query_sha256 started_monotonic ended_monotonic returncode partial truncated stdout stderr'.split())
CHECK_SQL = set('backend_pid application_name backend_absent observer_backend_pid observer_application_name'.split())
READY = set('issued_monotonic admission_deadline_monotonic common_end_monotonic active first_red'.split())


def require(ok, message):
    if not ok:
        raise ValueError(message)


def shape(value, keys, label):
    require(type(value) is dict and set(value) == keys, 'invalid '+label+' schema')


def number(value):
    require(type(value) in (int, float) and math.isfinite(value), 'invalid monotonic number')
    return value


def positive(value):
    return type(value) is int and value > 0


def hex_value(value, size):
    return type(value) is str and re.fullmatch('[0-9a-f]{'+str(size)+'}', value) is not None


def decode(raw):
    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result, 'duplicate JSON key')
            result[key] = value
        return result
    def invalid_constant(value):
        raise ValueError('nonfinite JSON number')
    try:
        return json.loads(raw, object_pairs_hook=pairs, parse_constant=invalid_constant)
    except (UnicodeError, json.JSONDecodeError) as error:
        raise ValueError('invalid JSON bytes') from error


def raw_bytes(reference, read_raw):
    shape(reference, {'path', 'sha256'}, 'raw reference')
    require(type(reference['path']) is str and reference['path'], 'missing raw path')
    require(hex_value(reference['sha256'],64), 'invalid raw SHA')
    try:
        value = read_raw(reference['path'])
    except Exception as error:
        raise ValueError('raw unavailable: '+type(error).__name__) from error
    require(type(value) is bytes and len(value) <= MAX_BYTES, 'raw missing or oversized')
    require(hashlib.sha256(value).hexdigest() == reference['sha256'], 'raw SHA differs')
    return value


def sql_shape(sql):
    shape(sql, SQL, 'sql')
    for field in ['database_oid','user_oid','owner_oid','server_version_num','backend_pid']:
        require(positive(sql[field]), 'invalid SQL integer: '+field)
    for field in ['database','user','owner','session_user','postmaster_start','application_name']:
        require(type(sql[field]) is str and sql[field], 'invalid SQL string: '+field)
    require(sql['server_address'] is None or type(sql['server_address']) is str, 'invalid server address')
    require(sql['server_port'] is None or type(sql['server_port']) is int and sql['server_port']==5432,
            'SQL server port is not the host binding5433')


def validate_receipt(receipt, *, expected, ready, now, read_raw, admission=True, admitted_at=None):
    """Valide une pièce depuis des attentes externes ; read_raw ne fait que lire des bytes."""
    shape(receipt, TOP, 'receipt')
    shape(expected, EXPECTED, 'external allocation')
    shape(expected['sql_identity'], SQL-{'backend_pid','application_name'}, 'external SQL identity')
    sql_shape(dict(expected['sql_identity'],backend_pid=1,application_name='external-pin-shape'))
    shape(ready, READY, 'ready timing')
    require(type(admission) is bool, 'invalid admission mode')
    require(expected.get('tool_qualified') is True and expected.get('allocation_active') is True,
            'tool qualification or external allocation unavailable')
    require(ready['active'] is True and ready['first_red'] is False, 'phase inactive or first red')
    require(type(receipt['schema_version']) is int and receipt['schema_version']==1 and
            receipt['kind']=='postgres-read-only-admission', 'unsupported contract')
    for key in BINDINGS:
        require(key in expected and receipt[key]==expected[key], 'external binding differs: '+key)
    for key in ['app','pilot']:
        require(hex_value(receipt[key],40), 'invalid source SHA')
    require(hex_value(receipt['ready_sha256'],64) and hex_value(receipt['nonce'],32), 'invalid READY identity')
    for key in ['scope','c7_reference','g_reference','reservation_reference']:
        require(type(receipt[key]) is str and receipt[key] and receipt[key].lower() not in
                ('unknown','pending','missing'), 'missing external reference')
    for key in ['exclusive','private_test','active']:
        require(receipt[key] is True, 'inactive or nonexclusive test reservation')
    require(receipt['children_policy']=='per-suite-process-databases', 'invalid child database policy')
    issued=number(ready['issued_monotonic']); limit=number(ready['admission_deadline_monotonic'])
    end=number(ready['common_end_monotonic']); now=number(now)
    observed=number(receipt['observed_monotonic']); lease=number(receipt['valid_until_monotonic'])
    require(0<=issued<=observed<=now<end and issued<limit<=end and lease>=end and now<lease,
            'observation or lease outside original job')
    accepted=now if admission else number(admitted_at)
    require(observed<=accepted<=now and accepted<limit and accepted-observed<=120,
            'not fresh at the original admission')
    topology=receipt['topology'];shape(topology,{'container_id','image_id','binding'},'topology')
    require(hex_value(topology['container_id'],64) and type(topology['image_id']) is str and
            topology['image_id'].startswith('sha256:') and hex_value(topology['image_id'][7:],64),
            'missing immutable container/image identity')
    binding=topology['binding'];shape(binding,{'host','host_port','container_port','protocol'},'binding')
    require(binding==dict(host='127.0.0.1',host_port=5433,container_port=5432,protocol='tcp') and
            type(binding['host_port']) is int and type(binding['container_port']) is int,
            'unexpected host/container binding')
    sql=receipt['sql'];sql_shape(sql)
    identity={k:v for k,v in sql.items() if k not in ('backend_pid','application_name')}
    require(identity==expected.get('sql_identity') and sql['database']=='rag3weaver_test' and
            sql['user']=='rag3weaver' and sql['owner']=='rag3weaver', 'SQL identity differs from external pins')
    require(sql['application_name']=='g-pg-'+receipt['nonce'], 'backend is not attributed to nonce')
    evidence=receipt['evidence'];shape(evidence,EVIDENCE,'evidence')
    require(evidence['transport']=='private-host-psql' and evidence['query_sha256']==QUERY_SHA256,
            'unqualified transport or arbitrary query')
    for key in ['observer_source_sha256','client_bundle_sha256']:
        require(hex_value(evidence[key],64) and evidence[key]==expected.get(key), 'unknown observer/client pin')
    for key in ['client_reaped','quiet','backend_absent','read_only']:
        require(evidence[key] is True, 'execution not quiet, reaped and read-only')
    require(evidence['partial'] is False and evidence['truncated'] is False and
            type(evidence['returncode']) is int and evidence['returncode']==0, 'incomplete or failed observation')
    started=number(evidence['started_monotonic']); deadline=number(evidence['deadline_monotonic'])
    ended=number(evidence['ended_monotonic'])
    require(issued<=started<=ended<=deadline<=started+5 and deadline<limit and ended<=observed,
            'worker exceeds unique whole5 or READY admission')
    decoded=decode(raw_bytes(evidence['stdout'],read_raw));sql_shape(decoded)
    require(decoded==sql, 'typed SQL differs from raw stdout')
    require(raw_bytes(evidence['stderr'],read_raw)==b'', 'SQL stderr is not empty')
    check=evidence['postcheck'];shape(check,POSTCHECK,'postcheck')
    require(check['query_sha256']==POSTCHECK_SHA256 and type(check['returncode']) is int and
            check['returncode']==0 and check['partial'] is False and check['truncated'] is False,
            'postcheck failed or partial')
    require(started<=number(check['started_monotonic'])<=number(check['ended_monotonic'])<=ended,
            'postcheck outside original whole5')
    checked=decode(raw_bytes(check['stdout'],read_raw));shape(checked,CHECK_SQL,'postcheck SQL')
    require(type(checked['backend_pid']) is int and checked['backend_pid']==sql['backend_pid'] and
            checked['application_name']==sql['application_name'] and checked['backend_absent'] is True and
            positive(checked['observer_backend_pid']) and checked['observer_backend_pid']!=sql['backend_pid'] and
            checked['observer_application_name']==sql['application_name']+':check', 'backend postcheck differs')
    require(raw_bytes(check['stderr'],read_raw)==b'', 'postcheck stderr is not empty')
    return receipt


def _read_regular(path, root):
    path=Path(path).absolute();root=Path(root).absolute()
    require(path.resolve()==path and path.is_relative_to(root) and path!=root,
            'raw path outside artifact or linked')
    require('.vault' not in path.parts and path.name!='.env', 'forbidden raw path')
    try:
        fd=os.open(path,os.O_RDONLY|os.O_NOFOLLOW)
        with os.fdopen(fd,'rb') as stream:
            info=os.fstat(stream.fileno())
            require(stat.S_ISREG(info.st_mode) and info.st_uid==os.getuid() and info.st_size<=MAX_BYTES,
                    'raw is not a bounded regular file owned by observer')
            return stream.read(MAX_BYTES+1)
    except OSError as error:
        raise ValueError('artifact unavailable: '+type(error).__name__) from error


def validate_attestation(path, expected_sha, expected_allocation, ready, now, *, admission=True, admitted_at=None):
    """Charge uniquement l'artefact et ses bruts voisins, jamais une connexion PG."""
    path=Path(path).absolute()
    require(hex_value(expected_sha,64), 'missing external artifact SHA')
    require(not os.path.lexists(path.parent/'refusal.json'), 'attestation directory was refused')
    raw=_read_regular(path,path.parent)
    require(hashlib.sha256(raw).hexdigest()==expected_sha, 'attestation SHA differs')
    receipt=decode(raw)
    validated=validate_receipt(receipt,expected=expected_allocation,ready=ready,now=now,
                               admission=admission,admitted_at=admitted_at,
                               read_raw=lambda p:_read_regular(p,path.parent))
    require(not os.path.lexists(path.parent/'refusal.json'), 'attestation refused during raw read')
    return validated


def prepare_observation():
    """Plan HOLD explicite : ce module ne contient PAS de runner SQL/Docker."""
    return dict(status='HOLD',run_authorized=False,transport_qualified=False,
                reason='private client/runtime pins/observer must be separately authorized and qualified',
                query_sha256=QUERY_SHA256,postcheck_sha256=POSTCHECK_SHA256,whole_seconds=5)


def _write_exclusive(path, raw):
    fd=os.open(path,os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600)
    with os.fdopen(fd,'wb') as stream:
        stream.write(raw)
        stream.flush()
        os.fsync(stream.fileno())


def _sync_directory(path):
    fd=os.open(path,os.O_RDONLY|os.O_DIRECTORY)
    try:
        os.fsync(fd)
    finally:
        os.close(fd)


def publish_observation(target, receipt, raw, *, expected, ready, clock, phase_check):
    """Archive les captures d'un observateur EXTERNE ; ne lance aucune probe.

    Le caller doit être qualifié séparément. Cette fonction peut valider des
    faux CPU, ce qui n'atteste jamais une ressource réelle. Tout refus conserve
    les captures et ne fournit ni artifactpath/SHA admissible ni grant.
    """
    target=Path(target).absolute()
    require(target.resolve()==target and '.vault' not in target.parts and target.name!='.env',
            'invalid observation directory')
    target.mkdir(mode=0o700)  # exclusif : ne réécrit JAMAIS une ancienne observation
    body=copy.deepcopy(receipt)
    try:
        require(type(raw) is dict, 'captured raw must be a dictionary')
        names={'sql-out':'identity.stdout','sql-err':'identity.stderr',
               'check-out':'postcheck.stdout','check-err':'postcheck.stderr'}
        captured={}
        for key,name in names.items():
            if key not in raw or type(raw[key]) is not bytes:
                continue
            prefix=raw[key][:MAX_BYTES]
            path=target/name
            _write_exclusive(path,prefix)  # AVANT tout jugement des métadonnées
            captured[key]=dict(path=str(path),sha256=hashlib.sha256(prefix).hexdigest())
        require(set(raw)==set(names) and set(captured)==set(names), 'captured raw incomplete')
        slots=[('sql-out',body['evidence'],'stdout'),('sql-err',body['evidence'],'stderr'),
               ('check-out',body['evidence']['postcheck'],'stdout'),
               ('check-err',body['evidence']['postcheck'],'stderr')]
        mismatches=[]
        for key,owner,field in slots:
            if len(raw[key])>MAX_BYTES:
                owner['truncated']=True
            if owner[field]['sha256']!=captured[key]['sha256']:
                mismatches.append(key)
            owner[field]=captured[key]
        require(not mismatches,'captured raw differs from claimed SHA')

        def current_phase():
            phase=phase_check()
            shape(phase,READY,'current ready timing')
            for key in ['issued_monotonic','admission_deadline_monotonic','common_end_monotonic']:
                require(phase[key]==ready[key], 'original phase timing changed')
            require(phase['active'] is True and phase['first_red'] is False, 'phase inactive or first red')
            return phase

        def validate_current():
            phase=current_phase()
            now=number(clock())
            require(now<=number(body['evidence']['deadline_monotonic']), 'publication exceeds original whole5')
            return validate_receipt(body,expected=expected,ready=phase,now=now,
                                    read_raw=lambda p:_read_regular(p,target))

        validate_current()
        payload=(json.dumps(body,sort_keys=True,allow_nan=False)+'\n').encode()
        pending=target/'attestation.pending.json'
        _write_exclusive(pending,payload)
        _sync_directory(target)
        validate_current()  # phase/hash/budget APRÈS fsync, juste avant publication
        final=target/'attestation.json'
        os.link(pending,final)  # lien exclusif ; aucune ancienne pièce remplacée
        sha=hashlib.sha256(payload).hexdigest()
        _write_exclusive(target/'attestation.sha256',(sha+'\n').encode())
        _sync_directory(target)
        validate_current()  # dernier fsync inclus : aucune publication tardive admise
        current_phase()
        require(number(clock())<=number(body['evidence']['deadline_monotonic']),
                'final publication exceeds original whole5')
        return final,sha
    except Exception as error:
        refusal=dict(verdict='RED',first_red_monotonic=clock(),reason=type(error).__name__+': '+str(error),
                     artifact_published=(target/'attestation.json').exists(),grant_authorized=False)
        _write_exclusive(target/'refusal.json',(json.dumps(refusal,sort_keys=True)+'\n').encode())
        _sync_directory(target)
        raise ValueError('observation refused: '+type(error).__name__) from error
