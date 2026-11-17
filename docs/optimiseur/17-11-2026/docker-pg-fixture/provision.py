"""One private allocation, not the PG candidate transport. Common180 required."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import secrets
import socket
import subprocess
import time


def main():
    parser=argparse.ArgumentParser();parser.add_argument('--seed',type=Path,required=True);parser.add_argument('--seed-sha',required=True);args=parser.parse_args()
    root=args.seed.parent;source=Path(__file__).resolve().parent;raw=args.seed.read_bytes();assert hashlib.sha256(raw).hexdigest()==args.seed_sha;seed=json.loads(raw)
    for n,h in seed['source_files'].items():assert hashlib.sha256((source/n).read_bytes()).hexdigest()==h
    common_path=source/'confiner_validation.py';assert hashlib.sha256(common_path.read_bytes()).hexdigest()=='6439fdd8b52c85fa4e74af0def2c667ef0bedae9b814cbbfd48e49946c4f74ea'
    spec=importlib.util.spec_from_file_location('fixture_common',common_path);common=importlib.util.module_from_spec(spec);spec.loader.exec_module(common);authority=common.autorite(common.processus())
    env=dict(PATH='/usr/bin:/bin',HOME=str(root/'home'),DOCKER_CONFIG=str(root/'docker-config'),DOCKER_HOST='unix:///var/run/docker.sock',LANG='C.UTF-8')
    end=min(authority['echeance']-10,time.monotonic()+150);passwords=[];steps=[];cid=None;nid=None
    def publish(name,value):
        path=root/name;fd=os.open(path,os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600)
        with os.fdopen(fd,'w') as f:json.dump(value,f,sort_keys=True,indent=2);f.write('\n');f.flush();os.fsync(f.fileno())
    def call(argv,*,data=None,cap=15,expected=(0,)):
        common.autorite(common.processus());assert time.monotonic()<end
        r=subprocess.run(argv,input=data,stdout=subprocess.PIPE,stderr=subprocess.PIPE,env=env,timeout=max(.1,min(cap,end-time.monotonic())))
        safe=[]
        for raw in [r.stdout,r.stderr]:
            for password in passwords:raw=raw.replace(password.encode(),b'[REDACTED]')
            safe.append(raw[:65536].decode(errors='replace'))
        steps.append(dict(argv=argv,code=r.returncode,stdout=safe[0],stderr=safe[1],truncated=len(r.stdout)>65536 or len(r.stderr)>65536))
        assert r.returncode in expected,'fixture command refused'
        return r
    try:
        # The expiry timer has already been installed OUTSIDE Common, never via its private bus.
        timer=json.loads((root/'expiry-armed.json').read_text());assert timer['nonce']==seed['nonce'] and timer['code']==0
        sock=socket.socket()
        try:sock.bind(('127.0.0.1',seed['port']))
        finally:sock.close()
        private=root/'credentials';private.mkdir(mode=0o700);lifecycle=root/'lifecycle';lifecycle.mkdir(mode=0o700);(lifecycle/'ttl.jsonl').touch(mode=0o600)
        for name in ['admin-password','observer-password']:
            password=secrets.token_hex(32);passwords.append(password)
            fd=os.open(private/name,os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600)
            with os.fdopen(fd,'w') as f:f.write(password+'\n')
        # No secret in argv, input logs or docker Config.Env; only _FILE references.
        nid=call(['/usr/bin/docker','network','create','--internal','--label','g.pg.fixture.owner=G','--label','g.pg.fixture.nonce='+seed['nonce'],seed['network_name']]).stdout.decode().strip()
        argv=['/usr/bin/docker','create','--name',seed['container_name'],'--rm','--network',nid,
              '--label','g.pg.fixture.owner=G','--label','g.pg.fixture.nonce='+seed['nonce'],
              '--memory','2g','--memory-swap','2g','--cpus','2','--pids-limit','128','--shm-size','64m','--read-only',
              '--security-opt','no-new-privileges','--log-driver','json-file','--log-opt','max-size=1m','--log-opt','max-file=2',
              '--tmpfs','/var/lib/postgresql/data:rw,nosuid,nodev,noexec,size=512m',
              '--tmpfs','/var/run/postgresql:rw,nosuid,nodev,size=16m','--tmpfs','/tmp:rw,nosuid,nodev,noexec,size=32m',
              '--mount','type=bind,src='+str(private)+',dst=/run/secrets,readonly',
              '--mount','type=bind,src='+str(source)+',dst=/fixture,readonly',
              '--mount','type=bind,src='+str(lifecycle)+',dst=/fixture-lifecycle',
              '-e','POSTGRES_PASSWORD_FILE=/run/secrets/admin-password','-e','POSTGRES_USER=g_pg_fixture_admin',
              '-e','POSTGRES_DB='+seed['database'],'-e','POSTGRES_INITDB_ARGS=--auth-host=scram-sha-256 --auth-local=trust',
              '-e','G_PG_FIXTURE_NONCE='+seed['nonce'],'-p','127.0.0.1:'+str(seed['port'])+':5432',
              '--entrypoint','/bin/sh',seed['image_id'],'/fixture/entrypoint.sh','postgres',
              '-c','log_statement=none','-c','log_min_error_statement=panic','-c','log_min_messages=fatal',
              '-c','max_connections=20','-c','shared_buffers=64MB']
        cid=call(argv).stdout.decode().strip();publish('created.json',dict(container_id=cid,network_id=nid,nonce=seed['nonce'],image_id=seed['image_id']))
        call(['/usr/bin/docker','start',cid])
        ready_end=min(end,time.monotonic()+60)
        while True:
            r=call(['/usr/bin/docker','exec',cid,'pg_isready','-q','-U','g_pg_fixture_admin','-d',seed['database']],cap=5,expected=(0,1,2))
            if r.returncode==0:break
            assert time.monotonic()<ready_end,'fixture readiness deadline';time.sleep(.5)
        base=['/usr/bin/docker','exec','-i','--user','0',cid,'psql','-X','-q','-v','ON_ERROR_STOP=1','-U','g_pg_fixture_admin','-d',seed['database']]
        call([*base,'-v','fixture_db='+seed['database']],data=(source/'provision.sql').read_bytes())
        verification=call([*base,'-A','-t'],data=(source/'verify.sql').read_bytes());value=json.loads(verification.stdout)
        assert value['database']==seed['database'] and value['owner']=='g_pg_fixture_admin' and value['observer']=='g_pg_observer'
        for key in ['superuser','createdb','createrole','replication','bypassrls','inherit','owner_is_observer','can_create','can_temp','schema_create','public_create','table_insert','table_update','table_delete']:assert value[key] is False,key
        assert all(value[k] is True for k in ['can_connect','table_select']);assert value['count']==3 and value['weight_sum']==10
        assert value['rows']==[dict(id=1,label='red',weight=2),dict(id=2,label='blue',weight=3),dict(id=3,label='green',weight=5)]
        fmt='{{json .Id}} {{json .Image}} {{json .State}} {{json .HostConfig.Memory}} {{json .HostConfig.MemorySwap}} {{json .HostConfig.NanoCpus}} {{json .HostConfig.PidsLimit}} {{json .NetworkSettings.Ports}} {{json .Config.Labels}} {{json .Mounts}}'
        identity=call(['/usr/bin/docker','inspect',cid,'--format',fmt]).stdout.decode();publish('docker-identity.json',dict(formatted=identity))
        decoder=json.JSONDecoder();text=identity;parts=[]
        while text.strip():
            item,n=decoder.raw_decode(text.lstrip());parts.append(item);text=text.lstrip()[n:]
        assert parts[0]==cid and parts[1]==seed['image_id']
        assert parts[3:7]==[2147483648,2147483648,2000000000,128]
        assert parts[7]['5432/tcp']==[dict(HostIp='127.0.0.1',HostPort=str(seed['port']))]
        assert parts[8]['g.pg.fixture.nonce']==seed['nonce']
        assert value['memberships']==[] and 'default_transaction_read_only=on' in value['role_settings']
        observer_query=b"SELECT json_build_object('user',current_user,'transaction_read_only',current_setting('transaction_read_only'),'rows',(SELECT count(*) FROM fixture.samples),'weight_sum',(SELECT sum(weight) FROM fixture.samples));\n"
        observer=call(['/usr/bin/docker','exec','-i','--user','0',cid,'psql','-X','-q','-A','-t','-v','ON_ERROR_STOP=1','-U','g_pg_observer','-d',seed['database']],data=observer_query)
        assert json.loads(observer.stdout)==dict(user='g_pg_observer',transaction_read_only='on',rows=3,weight_sum=10)
        publish('observer-fixture-judge.json',json.loads(observer.stdout))
        logs=call(['/usr/bin/docker','logs','--tail','120',cid],expected=(0,));publish('server-log.json',dict(stdout=steps[-1]['stdout'],stderr=steps[-1]['stderr']))
        common.autorite(common.processus());assert time.monotonic()<end
        publish('allocation.json',dict(kind='private-docker-pg-allocation-not-a-transport-grant',status='READY',nonce=seed['nonce'],container_id=cid,network_id=nid,image_id=seed['image_id'],host='127.0.0.1',port=seed['port'],sql_expectations=value,credential_reference=str(private/'observer-password'),credential_mode='0600',readonly_role='g_pg_observer',ttl_max_seconds=1800,expires_no_later_monotonic=seed['timer_armed_monotonic']+1800,transport_qualified=False,lease_granted=False))
    except BaseException as error:
        reason=str(error)
        for password in passwords:reason=reason.replace(password,'[REDACTED]')
        publish('first-red.json',dict(reason=type(error).__name__+': '+reason,container_id=cid,network_id=nid,transport_qualified=False))
        # Attributed cleanup outside the failed job; no second initialization.
        subprocess.run(['/usr/bin/python3',str(source/'cleanup.py'),'--seed',str(args.seed),'--seed-sha',args.seed_sha],timeout=25,check=False)
        raise SystemExit(1)
    finally:publish('operations.json',steps)


if __name__=='__main__':main()
