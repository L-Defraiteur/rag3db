"""V2 PREPARATION : orchestration injectée, aucune entrée live/lecture credentials.

Un rapport de probe n'est pas un grant. Le transport réel/OS reste à intégrer,
relire et qualifier ; les faux ne remplacent pas cette qualification.
"""
from dataclasses import dataclass, field
import hashlib
import json
import math
import os
from pathlib import Path
import re
import stat
import time

MAX_FRAME=65536
MAX_FRAMES=16


class Refused(ValueError):
    def __init__(self,code):self.code=code;super().__init__(code)


def require(ok,code):
    if not ok:raise Refused(code)


@dataclass(frozen=True)
class SecretInput:
    dsn: str=field(repr=False)
    secret_fragments: tuple=field(repr=False)
    provider_id: str


def encode(value):return (json.dumps(value,sort_keys=True,allow_nan=False)+'\n').encode()


def write_new(path,value):
    raw=encode(value)
    fd=os.open(path,os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600)
    with os.fdopen(fd,'wb') as stream:stream.write(raw);stream.flush();os.fsync(stream.fileno())


def decode(raw):
    require(type(raw) is bytes and len(raw)<=MAX_FRAME,'frame_bound')
    def pairs(items):
        result={}
        for key,value in items:
            require(key not in result,'duplicate_key');result[key]=value
        return result
    def bad(_):raise Refused('nonfinite_number')
    try:return json.loads(raw,object_pairs_hook=pairs,parse_constant=bad)
    except (UnicodeError,json.JSONDecodeError):raise Refused('frame_json') from None


def exact(value,keys):require(type(value) is dict and set(value)==set(keys),'shape')


def positive(value):return type(value) is int and value>0


def check_allocation(a,now):
    exact(a,'schema_version kind host_id clock_domain nonce fixture_nonce scope source_sha256 client_sha256 target expected_identity reservation_reference role_readonly_attested private_test common_end_monotonic lease_end_monotonic'.split())
    require(type(a['schema_version']) is int and a['schema_version']==2 and a['kind']=='pg-probe-allocation-v2','not_v2_allocation')
    require(a['role_readonly_attested'] is True and a['private_test'] is True,'external_readonly_allocation_missing')
    for key in ['host_id','clock_domain','scope','reservation_reference']:
        require(type(a[key]) is str and bool(a[key]) and a[key].lower() not in ('unknown','pending'),'external_identity_missing')
    require(type(a['nonce']) is str and re.fullmatch('[0-9a-f]{32}',a['nonce']) is not None,'nonce')
    require(type(a['fixture_nonce']) is str and re.fullmatch('[0-9a-f]{32}',a['fixture_nonce']) is not None,'fixture_nonce')
    for key in ['source_sha256','client_sha256']:require(type(a[key]) is str and re.fullmatch('[0-9a-f]{64}',a[key]) is not None,'source_pin')
    t=a['target'];exact(t,['host','port','database','user'])
    require(t['host']=='127.0.0.1' and type(t['port']) is int and 1<=t['port']<=65535 and t['port']!=5433,'foreign_target')
    for key in ['database','user']:require(type(t[key]) is str and re.fullmatch('[a-z_][a-z0-9_]{0,62}',t[key]) is not None,'target_name')
    require(t['database']=='g_pg_probe_'+a['fixture_nonce'][:12] and t['user']=='g_pg_observer','allocated_fixture_target')
    identity=a['expected_identity']
    exact(identity,'database user session_user database_oid user_oid owner owner_oid server_address server_port server_version_num postmaster_start'.split())
    require(identity['database']==t['database'] and identity['user']==identity['session_user']==t['user'] and identity['owner']!=t['user'],'readonly_nonowner_identity')
    for key in ['database_oid','user_oid','owner_oid','server_version_num']:require(positive(identity[key]),'external_oid')
    require(identity['server_port']==5432 and type(identity['server_port']) is int and type(identity['postmaster_start']) is str,'server_identity')
    require(type(a['common_end_monotonic']) in (float,int) and type(a['lease_end_monotonic']) in (float,int) and math.isfinite(a['common_end_monotonic']) and math.isfinite(a['lease_end_monotonic']) and now<a['common_end_monotonic']<=a['lease_end_monotonic'],'lease_end')


def read_allocated_secret(reference,allocation,fixture_cache,*,scope_check):
    """Future GO C only: exact private fixture reference, no original credentials."""
    require(scope_check() is True,'scope_refused')
    cache=Path(fixture_cache);path=Path(reference)
    require(cache.is_absolute() and cache.resolve()==cache,'credential_cache')
    require(type(allocation.get('fixture_nonce')) is str and re.fullmatch('[0-9a-f]{32}',allocation['fixture_nonce']) is not None,'fixture_nonce')
    root=cache/('allocation-'+allocation['fixture_nonce'][:12]);expected=root/'credentials/observer-password'
    require(path==expected and path.resolve()==path and not any(x in ('.env','.vault') for x in path.parts),'credential_reference')
    for directory in (cache,root,root/'credentials'):
        st=directory.stat()
        require(directory.resolve()==directory and stat.S_ISDIR(st.st_mode) and st.st_uid==os.getuid() and stat.S_IMODE(st.st_mode)==0o700,'credential_directory')
    before=path.stat();require(stat.S_ISREG(before.st_mode) and before.st_uid==os.getuid() and before.st_nlink==1 and stat.S_IMODE(before.st_mode)==0o600,'credential_file')
    require(scope_check() is True,'scope_refused')
    fd=os.open(path,os.O_RDONLY|os.O_NOFOLLOW|os.O_CLOEXEC)
    try:
        current=os.fstat(fd);require((current.st_dev,current.st_ino,current.st_uid,current.st_mode,current.st_size)==(before.st_dev,before.st_ino,before.st_uid,before.st_mode,before.st_size),'credential_changed')
        raw=os.read(fd,66)
    finally:os.close(fd)
    require(len(raw)==65 and raw.endswith(b'\n') and all(c in b'0123456789abcdef' for c in raw[:-1]),'credential_format')
    require(scope_check() is True,'scope_refused')
    target=allocation['target']
    require(target['host']=='127.0.0.1' and type(target['port']) is int and 1<=target['port']<=65535 and target['port']!=5433,'foreign_target')
    require(target['database']=='g_pg_probe_'+allocation['fixture_nonce'][:12] and target['user']=='g_pg_observer','allocated_fixture_target')
    password=raw[:-1].decode('ascii')
    dsn=f"host=127.0.0.1 port={target['port']} user={target['user']} dbname={target['database']} password={password} sslmode=disable"
    return SecretInput(dsn=dsn,secret_fragments=(password,),provider_id='fixture-'+allocation['fixture_nonce'][:12])


def contains_secret(value,secret):
    fragments=(secret.dsn,*secret.secret_fragments) if isinstance(secret,SecretInput) else ()
    if isinstance(value,str):return any(isinstance(x,str) and x and x in value for x in fragments)
    if isinstance(value,dict):return any(contains_secret(k,secret) or contains_secret(v,secret) for k,v in value.items())
    if isinstance(value,(tuple,list)):return any(contains_secret(v,secret) for v in value)
    return False


def safe_frame(raw,secret):
    require(type(raw) is bytes and len(raw)<=MAX_FRAME,'frame_bound')
    for text in (secret.dsn,*secret.secret_fragments):
        require(type(text) is str and bool(text),'secret_provider_incomplete')
        require(text.encode() not in raw,'credential_material_in_frame')
    f=decode(raw)
    require(not contains_secret(f,secret),'credential_material_in_frame')
    require(type(f) is dict and f.get('event') in ('connected','readonly','identity','closed','postcheck','complete','failure'),'unknown_frame')
    keys={'connected':['event','session','cancel_token_acquired'],'readonly':['event','session','state'],
          'identity':['event','row'],'closed':['event','session','driver_joined'],
          'postcheck':['event','row'],'complete':['event','transport_qualified'],'failure':['event','code','cleanup']}[f['event']]
    exact(f,keys)
    if 'session' in f:require(f['session'] in ('identity','postcheck'),'session')
    if f['event']=='failure':
        require(f['code'] in ('connect_before_attribution','query','readonly','cancel','driver_join','postcheck','timeout','protocol'),'unknown_failure_code')
        exact(f['cleanup'],'cancel_token_acquired cancel_attempted cancel_complete client_dropped driver_joined backend_pid backend_absent'.split())
        require(all(type(f['cleanup'][k]) is bool for k in ['cancel_token_acquired','cancel_attempted','cancel_complete','client_dropped','driver_joined']),'cleanup_shape')
        require(f['cleanup']['backend_pid'] is None or positive(f['cleanup']['backend_pid']),'cleanup_pid')
        require(f['cleanup']['backend_absent'] is None,'failure_cannot_attest_backend_absence')
        require(not f['cleanup']['cancel_attempted'] or f['cleanup']['cancel_token_acquired'],'cancel_without_token')
        require(not f['cleanup']['cancel_complete'] or f['cleanup']['cancel_attempted'],'cancel_completion_without_attempt')
        if f['code']=='connect_before_attribution':require(f['cleanup']['backend_pid'] is None and f['cleanup']['cancel_token_acquired'] is False,'connect_attribution_invented')
    # Jamais d'objet opaque/Debug/SQL/env autorisé dans la trace.
    if f['event']=='readonly':exact(f['state'],'transaction_read_only default_transaction_read_only superuser createdb createrole replication bypassrls database_owner user database backend_pid'.split())
    if f['event']=='identity':exact(f['row'],'database user session_user database_oid user_oid owner owner_oid server_address server_port server_version_num postmaster_start backend_pid application_name'.split())
    if f['event']=='postcheck':exact(f['row'],'backend_pid application_name backend_absent observer_backend_pid observer_application_name'.split())
    return f


class FrameState:
    expected=[('connected','identity'),('readonly','identity'),('identity',None),('closed','identity'),
              ('connected','postcheck'),('readonly','postcheck'),('postcheck',None),('closed','postcheck'),('complete',None)]
    def __init__(self,allocation):self.a=allocation;self.position=0;self.identity=None;self.check=None;self.pid=None;self.check_pid=None
    def accept(self,f):
        if f['event']=='failure':
            if f['cleanup']['backend_pid'] is not None:self.pid=f['cleanup']['backend_pid']
            raise Refused('native_'+f['code'])
        require(self.position<len(self.expected) and (f['event'],f.get('session'))==self.expected[self.position],'frame_order')
        event=f['event'];app='g-pg-'+self.a['nonce']
        if event=='connected':require(f['cancel_token_acquired'] is True,'missing_cancel_token')
        if event=='readonly':
            s=f['state'];require(s['transaction_read_only'] is True and s['default_transaction_read_only'] is True,'readonly_not_observed')
            require(all(s[k] is False for k in ['superuser','createdb','createrole','replication','bypassrls','database_owner']),'role_not_readonly')
            require(s['user']==self.a['target']['user'] and s['database']==self.a['target']['database'] and positive(s['backend_pid']),'session_identity')
            if f['session']=='identity':self.pid=s['backend_pid']
            else:self.check_pid=s['backend_pid'];require(self.check_pid!=self.pid,'observer_pid_reused')
        if event=='identity':
            row=f['row']
            for key in ['database_oid','user_oid','owner_oid','server_version_num','backend_pid']:require(positive(row[key]),'sql_integer')
            for key in ['database','user','session_user','owner','postmaster_start','application_name']:require(type(row[key]) is str and bool(row[key]),'sql_string')
            require(type(row['server_port']) is int and row['server_port']==5432,'server_port')
            require(row['server_address'] is None or type(row['server_address']) is str,'server_address')
            require(row==dict(self.a['expected_identity'],backend_pid=self.pid,application_name=app),'external_identity_differs');self.identity=row
        if event=='closed':require(f['driver_joined'] is True,'driver_not_joined')
        if event=='postcheck':
            require(positive(f['row']['backend_pid']) and positive(f['row']['observer_backend_pid']) and f['row']['backend_absent'] is True,'postcheck_typed')
            require(f['row']==dict(backend_pid=self.pid,application_name=app,backend_absent=True,observer_backend_pid=self.check_pid,observer_application_name=app+':check'),'backend_not_absent');self.check=f['row']
        if event=='complete':require(f['transport_qualified'] is False,'native_cannot_grant_qualification')
        self.position+=1
    def complete(self):return self.position==len(self.expected)


def observe(allocation,secret,transport,*,out,clock=time.monotonic,scope_check):
    """Source-only harness, transport injecté. Aucun loader secret ou accès réseau ici.

    transport.capture fait le natif futurement autorisé ; finish arrête/reap dans
    le même END. Une primitive OS externe doit entourer toute cette fonction.
    """
    started=clock();deadline=None;attempted=False;finished=False;tracker=FrameState(allocation)
    def publish(path,value):
        require(not contains_secret(value,secret),'credential_material_in_trace')
        write_new(path,value)
    def gate():require(scope_check() is True,'scope_refused');require(clock()<deadline,'original_deadline')
    try:
        check_allocation(allocation,started)
        require(isinstance(secret,SecretInput) and secret.secret_fragments and secret.dsn and re.fullmatch('[A-Za-z0-9_.-]+',secret.provider_id) is not None,'secret_provider_incomplete')
        deadline=min(started+10.,allocation['common_end_monotonic']);gate()
        publish(out/'invocation.json',dict(kind='pg-tool-probe-v2',schema_version=2,scope=allocation['scope'],nonce=allocation['nonce'],
                  source_sha256=allocation['source_sha256'],client_sha256=allocation['client_sha256'],provider_id=secret.provider_id,
                  started_monotonic=started,deadline_monotonic=deadline,transport_qualified=False))
        payload=dict(dsn=secret.dsn,nonce=allocation['nonce'],target=allocation['target'],budget_ms=min(8000,max(1,int((deadline-clock())*1000)-500)))
        attempted=True
        for number,raw in enumerate(transport.capture(payload,deadline)):
            gate();require(number<MAX_FRAMES,'too_many_frames')
            f=safe_frame(raw,secret)
            # Pièces canoniques encodées, pas une revendication de raw v1.
            publish(out/(f'frame-{number:02d}.json'),f)
            tracker.accept(f)
        gate();require(tracker.complete(),'incomplete_capture')
        finished=True;status=transport.finish(deadline,stop=False)
        require(status==dict(reaped=True,quiet=True,returncode=0),'native_not_quiet')
        gate()
        result=dict(schema_version=2,kind='pg-tool-probe-v2',source_sha256=allocation['source_sha256'],client_sha256=allocation['client_sha256'],
                    identity=tracker.identity,postcheck=tracker.check,transport_qualified=False,lease_granted=False,
                    started_monotonic=started,ended_monotonic=clock(),deadline_monotonic=deadline,evidence_encoding='validated-canonical-frames-not-v1-raw')
        publish(out/'probe.json',result);gate()
        return result
    except BaseException as error:
        # Aucun message d'exception externe : il peut contenir le DSN/canari.
        code=error.code if isinstance(error,Refused) and type(error.code) is str and re.fullmatch('[a-z_]+',error.code) and not contains_secret(error.code,secret) else 'transport_error'
        marker=dict(reason=code,error_class=type(error).__name__ if type(error) in (Refused,ValueError,TimeoutError,OSError) else 'Exception',server_attributed=tracker.pid is not None,
                    backend_absent=None,transport_qualified=False,lease_granted=False)
        if not (out/'first-red.json').exists():publish(out/'first-red.json',marker)
        if attempted and not finished:
            try:
                status=transport.finish(deadline,stop=True)
                publish(out/'cleanup.json',dict(reaped=status.get('reaped') is True,quiet=status.get('quiet') is True,
                          original_deadline_monotonic=deadline,server_attributed=tracker.pid is not None,backend_absent=None))
            except BaseException:pass
        raise Refused(code) from None


if __name__=='__main__':
    raise SystemExit('PREPARATION_ONLY: no live entry, credential loader, admission or PGv1 adapter')
