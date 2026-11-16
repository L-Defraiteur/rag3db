#!/usr/bin/env python3
"""Politique LIVE bornée par base/receipt ; mécanisme de lecture repris du provider copie épinglé.

Préparation seulement tant que copie, retour V2 et gates C7 ne sont pas acquis.
Ne lance aucune CLI, n'autorise aucune CLI dans sa réponse et ne touche aucune unité.
"""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import re
import sys
import time

CORE_SHA = '84168019beaa26a588fb475ed8dcd95ca2c94395e4752d5343c304f5bee281bd'
BASES = ('code-principal','code-lourd')


def load_pinned(path, name, sha):
    import hashlib
    if hashlib.sha256(path.read_bytes()).hexdigest()!=sha:
        raise RuntimeError('pinned source changed: '+str(path))
    spec=importlib.util.spec_from_file_location(name,path)
    module=importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def validate_live(request, base, cold, guard, phase, now, core):
    require=core.require
    require(base in BASES and request['schema_version']==1, 'invalid LIVE version/base')
    require(request['stage']=='live-'+base+'-01' and request['base']==base
            and request['target_kind']=='live', 'wrong LIVE stage/base/target_kind')
    require(request['phase']==phase and re.fullmatch('[0-9a-f]{32}',request['nonce']), 'invalid phase/nonce')
    require(0<request['entered_monotonic']<=request['requested_monotonic']<=now
            and now-request['requested_monotonic']<=30, 'invalid LIVE clock order/freshness')
    canonical=Path.home()/'.local/share/rag3weaver'/(base+'.rag3db')
    require(request['canonical_base']==str(canonical) and request['target_base']==str(canonical),
            'LIVE canonical and target must both be the exact base')
    require(cold['qualification']=='FRESH_DIAGNOSTIC_SNAPSHOT_READY' and cold['base']==base
            and cold['canonical_base']==str(canonical), 'wrong cold snapshot receipt')
    require(cold['source']==core.SOURCE_SHA and cold['host_sha256']==core.HOST_SHA,
            'cold receipt source/host mismatch')
    require(request['source_snapshot']==cold['snapshot_root'], 'wrong corresponding fresh snapshot')
    for field in ['canonical_base','target_base','source_snapshot']:
        path=Path(request[field])
        require(path.is_absolute() and path.resolve()==path
                and not any(p in ('.env','.vault') for p in path.parts), 'linked/protected LIVE path')
    host=request['expected_host']
    require(host['pid']==3761669 and host['exe_path']==str(Path.home()/'.local/bin/rag3weaver-backend')
            and host['exe_sha256']==core.HOST_SHA and host['source_sha']==core.SOURCE_SHA,
            'wrong expected host')
    for field,name in [('reply_path',phase+'-response.json'),('reply_sha_path',phase+'-response.sha256')]:
        require(request[field]==str(guard/name), 'reply path outside LIVE phase')


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--base',choices=BASES,required=True)
    for flag in ['core-script','common-script','cold-receipt','evidence']:
        parser.add_argument('--'+flag,type=Path,required=True)
    parser.add_argument('--cold-receipt-sha',required=True)
    parser.add_argument('--seconds',type=int,default=3900)
    args=parser.parse_args()
    os.umask(0o077)
    core=load_pinned(args.core_script,'cold_service_core_18ff',CORE_SHA)
    core.require(os.getuid()==1000 and 0<args.seconds<=3900, 'wrong UID/provider budget')
    common=load_pinned(args.common_script,'live_common_927',core.COMMON_SHA)
    core.require(core.digest(args.cold_receipt)==args.cold_receipt_sha, 'cold receipt SHA mismatch')
    cold=json.loads(args.cold_receipt.read_text())
    guard=Path.home()/'.cache/rag3weaver-build/bascule-live-20261116/migration-offline-01'/args.base/'guard'
    core.require(guard.resolve()==guard, 'linked LIVE guard')
    args.evidence.mkdir(parents=True,exist_ok=False)
    core.write_json(args.evidence/'observer.json',dict(observer=core.identity(os.getpid()),
                    base=args.base,stage='live-'+args.base+'-01',guard=str(guard),
                    budget_seconds=args.seconds,core_sha256=CORE_SHA,common_sha256=core.COMMON_SHA,
                    cold_receipt=str(args.cold_receipt),cold_receipt_sha256=args.cold_receipt_sha))
    end=time.monotonic()+args.seconds
    seen=set();responded=[];authority_path=None
    try:
        for phase in ['versions','generations']:
            path=guard/(phase+'-request.json')
            while not path.exists():
                core.require(time.monotonic()<end, 'LIVE provider deadline before phase')
                if authority_path:
                    state=json.loads(authority_path.read_text())
                    if state['etat']!='actif' or time.monotonic()>=state['echeance']:
                        core.write_json(args.evidence/'finished.json',dict(reason='Common ended',responded=responded))
                        return 0
                time.sleep(.05)
            core.require(not path.is_symlink() and path.stat().st_uid==os.getuid(), 'invalid LIVE request owner/type')
            raw=path.read_bytes();request=json.loads(raw)
            core.require(core.digest(args.cold_receipt)==args.cold_receipt_sha, 'cold receipt changed')
            validate_live(request,args.base,cold,guard,phase,time.monotonic(),core)
            core.require(request['nonce'] not in seen, 'reused LIVE nonce');seen.add(request['nonce'])
            core.verify_common(request,common)
            authority_path=Path(request['common_receipt']['path'])
            # Lecture seule du même mécanisme ; aucune mutation des politiques du provider copie.
            response=core.collect(request,phase,guard,common)
            core.require(path.read_bytes()==raw and core.digest(args.cold_receipt)==args.cold_receipt_sha,
                         'LIVE request or cold receipt changed')
            import hashlib
            response.update(request_sha256=hashlib.sha256(raw).hexdigest(),base=args.base,target_kind='live',
                            cold_receipt_sha256=args.cold_receipt_sha,cli_authorized=False)
            reply=Path(request['reply_path']);core.write_json(reply,response)
            with Path(request['reply_sha_path']).open('x') as stream:
                stream.write(core.digest(reply)+'\n');stream.flush();os.fsync(stream.fileno())
            responded.append(phase)
            print(json.dumps(dict(base=args.base,phase=phase,reply=str(reply),sha256=core.digest(reply),
                                  observer_pid=os.getpid())),flush=True)
        while time.monotonic()<end:
            state=json.loads(authority_path.read_text())
            if state['etat']!='actif' or time.monotonic()>=state['echeance']:break
            time.sleep(.1)
        core.write_json(args.evidence/'finished.json',dict(reason='Common ended or provider deadline',responded=responded))
        return 0
    except Exception as error:
        core.write_json(args.evidence/'refusal.json',dict(verdict='ROUGE',error=f'{type(error).__name__}: {error}',responded=responded))
        print(f'ROUGE LIVE provider: {type(error).__name__}: {error}',file=sys.stderr)
        return 1


if __name__=='__main__':raise SystemExit(main())
