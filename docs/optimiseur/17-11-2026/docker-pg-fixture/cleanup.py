"""Owned fixture cleanup only. No credential reads, no Env inspection."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time


def main():
    a=argparse.ArgumentParser();a.add_argument('--seed',type=Path,required=True);a.add_argument('--seed-sha',required=True);args=a.parse_args()
    raw=args.seed.read_bytes();assert hashlib.sha256(raw).hexdigest()==args.seed_sha
    seed=json.loads(raw);root=args.seed.parent
    assert hashlib.sha256(Path(__file__).read_bytes()).hexdigest()==seed['source_files']['cleanup.py']
    env=dict(PATH='/usr/bin:/bin',HOME=str(root/'home'),DOCKER_CONFIG=str(root/'docker-config'),DOCKER_HOST='unix:///var/run/docker.sock',LANG='C.UTF-8')
    end=time.monotonic()+20;records=[]
    def call(argv):
        r=subprocess.run(argv,stdout=subprocess.PIPE,stderr=subprocess.PIPE,env=env,timeout=max(.1,min(5,end-time.monotonic())))
        records.append(dict(argv=argv,code=r.returncode,stdout=r.stdout.decode(errors='replace')[:4096],stderr=r.stderr.decode(errors='replace')[:4096]))
        return r
    def owned(kind,name):
        fmt='{{json .Id}} {{json .Config.Labels}} {{json .Image}}' if kind=='container' else '{{json .Id}} {{json .Labels}}'
        r=call(['/usr/bin/docker',kind,'inspect',name,'--format',fmt])
        if r.returncode:
            if b'No such' in r.stderr or b'not found' in r.stderr:return None
            raise RuntimeError('cleanup identity unavailable')
        decoder=json.JSONDecoder();text=r.stdout.decode();values=[]
        while text.strip():
            v,n=decoder.raw_decode(text.lstrip());values.append(v);text=text.lstrip()[n:]
        labels=values[1]
        assert labels.get('g.pg.fixture.nonce')==seed['nonce'] and labels.get('g.pg.fixture.owner')=='G'
        if kind=='container':assert values[2]==seed['image_id']
        return values[0]
    try:
        cid=owned('container',seed['container_name'])
        if cid:
            r=call(['/usr/bin/docker','rm','-f',cid]);assert r.returncode==0 or b'No such' in r.stderr
        nid=owned('network',seed['network_name'])
        if nid:
            r=call(['/usr/bin/docker','network','rm',nid]);assert r.returncode==0 or b'No such' in r.stderr
        for name in ['admin-password','observer-password']:
            path=root/'credentials'/name
            if path.exists():path.unlink()
        verdict='REMOVED_OR_ALREADY_ABSENT'
    except BaseException as error:verdict='REFUSED_'+type(error).__name__
    result=dict(verdict=verdict,nonce=seed['nonce'],ended_monotonic=time.monotonic(),operations=records)
    path=root/('cleanup-'+str(time.time_ns())+'.json')
    fd=os.open(path,os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600)
    with os.fdopen(fd,'w') as f:json.dump(result,f,sort_keys=True,indent=2);f.write('\n')
    if verdict.startswith('REFUSED'):raise SystemExit(1)


if __name__=='__main__':main()
