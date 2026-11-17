"""One stopped-image extraction and sealed HOST CLI preflight; never SQL/server."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import shutil
import stat
import subprocess
import time
from elf_closure import image_file,metadata,require


def sha(path):
    with path.open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()


def write(path,value):
    fd=os.open(path,os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600)
    with os.fdopen(fd,'w') as f:json.dump(value,f,sort_keys=True,indent=2);f.write('\n');f.flush();os.fsync(f.fileno())


def sync(path):
    fd=os.open(path,os.O_RDONLY|os.O_DIRECTORY)
    try:os.fsync(fd)
    finally:os.close(fd)


def main():
    p=argparse.ArgumentParser();p.add_argument('--recipe',type=Path,required=True);p.add_argument('--recipe-sha',required=True);a=p.parse_args()
    require(sha(a.recipe)==a.recipe_sha,'recipe pin');r=json.loads(a.recipe.read_text());root=a.recipe.parent;src=Path(__file__).resolve().parent
    for n,h in r['source_files'].items():require(sha(src/n)==h,'source pin')
    common_path=src/'confiner_validation.py';require(sha(common_path)=='6439fdd8b52c85fa4e74af0def2c667ef0bedae9b814cbbfd48e49946c4f74ea','Common pin')
    spec=importlib.util.spec_from_file_location('psql_common',common_path);common=importlib.util.module_from_spec(spec);spec.loader.exec_module(common);authority=common.autorite(common.processus())
    started=time.monotonic();end=min(started+65,authority['echeance']-5);cid=None;steps=[];ledger=None
    docker_env=dict(PATH='/usr/bin:/bin',HOME=str(root/'home'),DOCKER_CONFIG=str(root/'docker-config'),DOCKER_HOST='unix:///var/run/docker.sock',LANG='C')
    cli_env=dict(PATH='/usr/bin:/bin',HOME=str(root/'home'),TMPDIR=str(root/'tmp'),LANG='C',LC_ALL='C',PGHOST=str(root/'no-socket'),PGUSER='bundle_no_contact',PGDATABASE='bundle_no_contact',PGCONNECT_TIMEOUT='1')
    def call(name,argv,env,cap=10,cleanup=False):
        common.autorite(common.processus());deadline=authority['echeance']-1 if cleanup else end;require(time.monotonic()<deadline,'original END');begin=time.monotonic()
        process=subprocess.Popen(argv,stdin=subprocess.DEVNULL,stdout=subprocess.PIPE,stderr=subprocess.PIPE,env=env)
        observed=dict(pid=process.pid,exe=None,start=None,maps=None,unknown='exited-or-raced-before-snapshot')
        try:
            proc=Path('/proc')/str(process.pid);st=(proc/'stat').read_text().rsplit(')',1)[1].split()
            observed.update(exe=os.readlink(proc/'exe'),start=st[19],maps=(proc/'maps').read_text()[:65536],unknown=None)
        except (OSError,IndexError):pass
        try:stdout,stderr=process.communicate(timeout=max(.001,min(cap,deadline-time.monotonic())))
        except subprocess.TimeoutExpired:
            process.kill();stdout,stderr=process.communicate();code=process.returncode
            (root/(name+'.stdout')).write_bytes(stdout[:1048576]);(root/(name+'.stderr')).write_bytes(stderr[:1048576])
            write(root/(name+'.json'),dict(argv=argv,code=code,timed_out=True,observed=observed,started_monotonic=begin,ended_monotonic=time.monotonic()));raise ValueError('command timeout')
        (root/(name+'.stdout')).write_bytes(stdout[:1048576]);(root/(name+'.stderr')).write_bytes(stderr[:1048576])
        record=dict(argv=argv,code=process.returncode,observed=observed,started_monotonic=begin,ended_monotonic=time.monotonic(),stdout_sha256=sha(root/(name+'.stdout')),stderr_sha256=sha(root/(name+'.stderr')),timed_out=False,truncated=len(stdout)>1048576 or len(stderr)>1048576)
        write(root/(name+'.json'),record);steps.append(record)
        require(process.returncode==0 and not record['truncated'] and time.monotonic()<deadline,'command refused/late')
        return stdout
    def remove_owned():
        if cid is None:return
        value=call('cleanup-identity',['/usr/bin/docker','inspect',cid,'--format','{{json .Id}} {{json .Image}} {{json .State.Running}} {{json .Config.Labels}}'],docker_env,cap=3,cleanup=True)
        values=[];text=value.decode();decoder=json.JSONDecoder()
        while text.strip():item,n=decoder.raw_decode(text.lstrip());values.append(item);text=text.lstrip()[n:]
        require(values[0]==cid and values[1]==r['image_id'] and values[2] is False and values[3].get('g.psql.bundle.nonce')==r['nonce'] and values[3].get('g.psql.bundle.owner')=='G','unowned/running cleanup target')
        call('cleanup-rm',['/usr/bin/docker','rm','-v',cid],docker_env,cap=3,cleanup=True)
    try:
        capture=root/'capture';capture.mkdir(mode=0o700);bundle=root/'bundle';bundle.mkdir(mode=0o700)
        for n in ['bin','lib','loader','share']:(bundle/n).mkdir(mode=0o700)
        argv=['/usr/bin/docker','create','--pull=never','--name',r['container_name'],'--network','none','--read-only','--tmpfs','/var/lib/postgresql/data:rw,nosuid,nodev,noexec,size=16m','--label','g.psql.bundle.owner=G','--label','g.psql.bundle.nonce='+r['nonce'],'--entrypoint','/bin/true',r['image_id']]
        cid=call('create',argv,docker_env).decode().strip();write(root/'created.json',dict(container_id=cid,nonce=r['nonce'],image_id=r['image_id'],started=False))
        state=call('stopped-identity',['/usr/bin/docker','inspect',cid,'--format','{{json .Id}} {{json .Image}} {{json .State}} {{json .Config.Labels}} {{json .Mounts}}'],docker_env)
        values=[];text=state.decode();decoder=json.JSONDecoder()
        while text.strip():item,n=decoder.raw_decode(text.lstrip());values.append(item);text=text.lstrip()[n:]
        require(values[0]==cid and values[1]==r['image_id'] and values[2]['Running'] is False and values[2]['Pid']==0,'image/container not stopped')
        require(all(m['Type']!='volume' for m in values[4]),'unexpected anonymous volume')
        (capture/'usr/local/bin').mkdir(parents=True);(capture/'usr/local/lib').parent.mkdir(parents=True,exist_ok=True);(capture/'usr/share').mkdir(parents=True)
        call('copy-psql',['/usr/bin/docker','cp','-L',cid+':/usr/local/bin/psql',str(capture/'usr/local/bin/psql')],docker_env)
        for i,(source,destination) in enumerate([('/lib',capture),('/usr/lib',capture/'usr'),('/usr/local/lib',capture/'usr/local'),('/usr/local/share/postgresql',capture/'usr/share')]):
            call('copy-tree-'+str(i),['/usr/bin/docker','cp',cid+':'+source,str(destination)],docker_env,cap=15)
        # Share is captured at /usr/share/postgresql; normalize only the capture directory.
        (capture/'usr/local/share').mkdir(exist_ok=True);shutil.move(str(capture/'usr/share/postgresql'),str(capture/'usr/local/share/postgresql'))
        program_source,chain=image_file(capture,'/usr/local/bin/psql');program=bundle/'bin/psql';shutil.copyfile(program_source,program);program.chmod(0o755)
        pm=metadata(program);interp=pm['interpreter'];require(interp=='/lib/ld-musl-x86_64.so.1','unapproved loader ABI')
        loader_source,loader_chain=image_file(capture,interp);loader=bundle/'loader/ld-musl-x86_64.so.1';shutil.copyfile(loader_source,loader);loader.chmod(0o755);metadata(loader)
        closure={};queue=list(pm['needed']);provenance={}
        while queue:
            name=queue.pop(0)
            if name in closure:continue
            candidates=[]
            for prefix in ['/lib','/usr/local/lib','/usr/lib']:
                candidate=capture/(prefix+'/'+name).lstrip('/')
                if not os.path.lexists(candidate):continue
                resolved,links=image_file(capture,prefix+'/'+name);candidates.append((resolved,links,prefix+'/'+name))
            require(candidates,'missing DT_NEEDED '+name);require(len({sha(x[0]) for x in candidates})==1,'ambiguous image library '+name)
            original,links,image_path=candidates[0];target=bundle/'lib'/name;shutil.copyfile(original,target);target.chmod(0o755)
            data=metadata(target);closure[name]=data;provenance[name]=dict(image_path=image_path,symlinks=links,sha256=sha(target));queue.extend(data['needed'])
        support_root=capture/'usr/local/share/postgresql';support_target=bundle/'share/postgresql';support_target.mkdir(mode=0o700)
        for item in support_root.rglob('*'):
            rel=item.relative_to(support_root);target=support_target/rel
            if item.is_symlink():original,_=image_file(capture,'/usr/local/share/postgresql/'+str(rel));target.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(original,target)
            elif item.is_dir():target.mkdir(exist_ok=True)
            elif item.is_file():target.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(item,target)
            else:raise ValueError('unsupported support file')
        remove_owned();cid=None
        files={str(p.relative_to(bundle)):dict(size=p.stat().st_size,sha256=sha(p),mode=stat.S_IMODE(p.stat().st_mode)) for p in bundle.rglob('*') if p.is_file()}
        require(all(not p.is_symlink() for p in bundle.rglob('*')),'bundle symlink')
        for path in bundle.rglob('*'):
            if path.is_file():
                with path.open('rb') as stream:os.fsync(stream.fileno())
        for directory in sorted((p for p in bundle.rglob('*') if p.is_dir()),key=lambda p:len(p.parts),reverse=True):sync(directory)
        ledger=dict(kind='psql-host-cli-bundle-before-execution',image_id=r['image_id'],nonce=r['nonce'],program_elf=dict(path=str(program),sha256=sha(program),elf=pm),kernel_entry_requested=dict(path=str(loader),sha256=sha(loader),role='explicit-musl-loader-not-psql-program'),container_interpreter_name=interp,closure=closure,provenance=provenance,files=files,environment=cli_env,native_executed=False,transport_qualified=False)
        write(root/'bundle-ledger.json',ledger);write(root/'bundle-ledger.sha256.json',dict(sha256=sha(root/'bundle-ledger.json')));sync(bundle);sync(root)
        write(root/'ledger-complete.json',dict(ledger_sha256=sha(root/'bundle-ledger.json'),completed_monotonic=time.monotonic()));sync(root)
        prefix=[str(loader),'--library-path',str(bundle/'lib')]
        resolution=call('loader-resolution',prefix+['--list',str(program)],cli_env,cap=5).decode()
        resolved_names=set();self_labels=[]
        for line in resolution.splitlines():
            if '=>' in line:
                name,value=line.split('=>',1);name=name.strip();value=value.strip().split()[0]
                if value==interp:
                    # musl --list deliberately displays PT_INTERP as its SELF name.
                    require(name in provenance and provenance[name]['sha256']==sha(loader),'unapproved interpreter label')
                    self_labels.append(dict(name=name,displayed=value,kind='PT_INTERP_SELF_LABEL_NOT_HOST_MAPPING'))
                else:
                    path=Path(value);require(path.is_absolute() and path.is_relative_to(bundle) and path.resolve()==path,'loader resolved outside bundle')
                    require(name in provenance and sha(path)==provenance[name]['sha256'],'resolved library SHA differs')
                resolved_names.add(name)
        require(all(name in resolved_names or row['sha256']==sha(loader) for name,row in provenance.items()),'incomplete loader resolution inventory')
        write(root/'resolution-interpretation.json',dict(resolved_names=sorted(resolved_names),self_labels=self_labels,kernel_entry_requested=str(loader),kernel_entry_sha256=sha(loader),program_requested=str(program),program_sha256=sha(program)))
        version=call('psql-version',prefix+[str(program),'--version'],cli_env,cap=5).decode();require(version.startswith('psql (PostgreSQL) 17.'),'unexpected psql version')
        help_text=call('psql-help',prefix+[str(program),'--help'],cli_env,cap=5).decode();require('Usage:' in help_text and 'psql' in help_text,'unexpected psql help')
        require(all(sha(bundle/name)==row['sha256'] for name,row in files.items()),'bundle changed after CLI')
        common.autorite(common.processus());require(time.monotonic()<end,'late final bundle')
        write(root/'result.json',dict(verdict='HOST_CLI_PREFLIGHT_PASS',ledger_sha256=sha(root/'bundle-ledger.json'),program_elf_sha256=sha(program),kernel_entry_sha256=sha(loader),steps=steps,transport_qualified=False,sql_executed=False,started_monotonic=started,ended_monotonic=time.monotonic()))
        sync(root);common.autorite(common.processus());require(time.monotonic()<end,'late final publication')
    except BaseException as error:
        write(root/'first-red.json',dict(reason=type(error).__name__+': '+str(error),container_id=cid,transport_qualified=False))
        if cid is not None:
            try:remove_owned();cid=None
            except BaseException as cleanup:write(root/'cleanup-refusal.json',dict(reason=type(cleanup).__name__+': '+str(cleanup),container_id=cid))
        raise


if __name__=='__main__':main()
