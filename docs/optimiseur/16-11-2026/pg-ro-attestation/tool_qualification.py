"""Pilote PREPA : compile UNE fois + CLI/DSN validation-only, jamais une probe PG.

Aucune action à l'import. --run exige un GO externe piné et le vrai Common.
Les faux injectent le backend, aucun subprocess/proc ne leur est nécessaire.
"""
import argparse
import hashlib
import importlib.util
import json
import math
import re
import os
from pathlib import Path
import signal
import subprocess
import sys
import time

COMMON_SHA='6439fdd8b52c85fa4e74af0def2c667ef0bedae9b814cbbfd48e49946c4f74ea'
MAX_RAW=65536


def require(ok,message):
    if not ok:raise ValueError(message)


def digest(path):
    with Path(path).open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()


def read_json(path,sha=None):
    path=Path(path).absolute()
    require('.vault' not in path.parts and path.name!='.env','forbidden input path')
    require(path.resolve()==path and path.is_file() and path.stat().st_uid==os.getuid(),'linked/unowned input')
    require(path.stat().st_size<=4*1024*1024,'oversized input')
    if sha is not None:require(digest(path)==sha,'input SHA differs')
    return json.loads(path.read_text())


def write_new(path,value):
    raw=(json.dumps(value,sort_keys=True,allow_nan=False)+'\n').encode()
    fd=os.open(path,os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600)
    with os.fdopen(fd,'wb') as stream:stream.write(raw);stream.flush();os.fsync(stream.fileno())


def sync_dir(path):
    fd=os.open(path,os.O_DIRECTORY|os.O_RDONLY)
    try:os.fsync(fd)
    finally:os.close(fd)


def common_authority(packet_root,packet):
    path=packet_root/'common-confiner.py';require(digest(path)==COMMON_SHA,'Common source differs')
    spec=importlib.util.spec_from_file_location('qualified_common',path)
    module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
    receipt=module.autorite(module.processus())
    return {'pid':receipt['pid'],'start':receipt['debut'],'cgroup':receipt['cgroup'],'end':receipt['echeance']}


def validate_go(go,packet,packet_sha,now):
    require(type(go) is dict and set(go)=={'kind','run_authorized','packet_sha256','reference','nonce','inputs_manifest_sha256','deadline_monotonic','sql_authorized','docker_authorized','output'},'GO schema differs')
    require(type(go.get('reference')) is str and bool(go['reference']) and type(go.get('nonce')) is str and re.fullmatch('[0-9a-f]{32}',go['nonce']) is not None,'GO identity missing')
    require(go.get('kind')=='pg-rust-compile-and-parse-only' and go.get('run_authorized') is True,'no exact tool GO')
    require(go.get('packet_sha256')==packet_sha and go.get('reference') and go.get('nonce'),'unbound GO')
    require(go.get('sql_authorized') is False and go.get('docker_authorized') is False,'contacts not allowed in this tool')
    require(type(go.get('deadline_monotonic')) in (float,int) and math.isfinite(go['deadline_monotonic']) and now<go['deadline_monotonic'],'GO expired')
    require(go.get('inputs_manifest_sha256')==packet.get('inputs_manifest_sha256'),'GO inputs differ')
    return go


def verify_sources(packet_root,packet):
    for relative,sha in packet['source_files'].items():
        path=packet_root/relative
        require(path.resolve().is_relative_to(packet_root) and not path.is_symlink() and digest(path)==sha,'source pin differs')


def verify_inputs(packet,full):
    root=Path(packet['inputs_root'])
    manifest=read_json(root/'manifest.json',packet['inputs_manifest_sha256'])
    ready=read_json(root/'READY.json',packet['inputs_ready_sha256'])
    require(ready['manifest_sha256']==packet['inputs_manifest_sha256'],'archive not final')
    selected=list(manifest) if full else ['toolchain/bin/rustc']
    for relative in selected:
        path=root/relative;row=manifest[relative]
        require(path.resolve()==path and path.stat().st_size==row['size'] and digest(path)==row['sha256'],'archived input differs')
    for path,row in ready['host_linker_and_runtime_file_pins'].items():
        require(str(Path(path).resolve())==row['resolved'] and digest(path)==row['sha256'],'host linker/runtime pin differs')
    return manifest


def compile_argv(packet,out):
    root=Path(packet['inputs_root']);src=Path(packet['packet_root'])
    args=[str(root/'toolchain/bin/rustc'),'--sysroot',str(root/'toolchain'),'--edition=2021',
          '--crate-name','pg_ro_client',str(src/'pg_ro_client.rs'),'-L','dependency='+str(root/'deps')]
    for crate,relative in packet['externs'].items():args+=['--extern',crate+'='+str(root/relative)]
    args+=['-C','opt-level=1','-C','codegen-units=1','-C','linker=/usr/bin/cc','-o',str(out/'client')]
    return args


def orchestrate(config,backend):
    require(config['sql_authorized'] is False and config['docker_authorized'] is False,'contacts forbidden')
    require(config['compile_cap']==60 and config['worker_cap']==5,'whole caps changed')
    cases=config['cases'];require(len(cases)==8 and len({x['name'] for x in cases})==8,'eight cases required')
    require(all(x['argv'] in ([],['--validate-request']) for x in cases),'capture mode forbidden')
    requests=[dict(name='compile',kind='compile',argv=config['compile_argv'],stdin='',cap=60,
                   expected_code=0,expected_stderr='')]
    requests += [dict(name=x['name'],kind='parse-only',argv=[config['client_path'],*x['argv']],
                      stdin=x['stdin'],cap=5,expected_code=x['returncode'],expected_stderr=x['stderr_contains']) for x in cases]
    results=[]
    for request in requests:
        result=backend.run(request);results.append(result)
        require(result['returncode']==request['expected_code'] and result['timed_out'] is False,'native result refused')
        for key in ['quiet','raw_published','within_whole','stdin_started_inside_whole']:
            require(result[key] is True,'whole OS/cleanup/publication unknown: '+key)
        if request['expected_stderr']:require(request['expected_stderr'].encode() in result['stderr'],'wrong CLI refusal')
        if request['kind']=='parse-only':
            if request['expected_code']==0:
                value=json.loads(result['stdout']);require(set(value)=={'request_valid','pg_contact'} and
                    value['request_valid'] is True and value['pg_contact'] is False,'not a pure parsing result')
            else:require(result['stdout']==b'','unexpected stdout on refusal')
    return results


def own_cgroup():
    return Path('/proc/self/cgroup').read_text().strip()


def group_members(group):
    members=[];unknown=[]
    for proc in Path('/proc').iterdir():
        if not proc.name.isdigit():continue
        try:
            fields=(proc/'stat').read_text().rsplit(')',1)[1].split()
            if int(fields[2])!=group:continue
            members.append({'pid':int(proc.name),'start':int(fields[19]),'uid':proc.stat().st_uid,
                            'cgroup':(proc/'cgroup').read_text().strip()})
        except FileNotFoundError:continue
        except (PermissionError,ProcessLookupError):
            try:
                if proc.stat().st_uid==os.getuid():unknown.append(int(proc.name))
            except FileNotFoundError:pass
    return members,unknown


def stop_group(group,expected_cgroup):
    members,unknown=group_members(group)
    require(not unknown,'unknown own processes while cleaning group')
    require(all(x['uid']==os.getuid() and x['cgroup']==expected_cgroup for x in members),'foreign process group member')
    if members:
        try:os.killpg(group,signal.SIGKILL)
        except ProcessLookupError:pass


class NativeBackend:
    def __init__(self,root,packet,packet_sha,go,out):
        self.root=root;self.packet=packet;self.packet_sha=packet_sha;self.go=go;self.out=out
        self.compiled=False;self.client_sha=None
    def run(self,request):
        started=time.monotonic();deadline=min(started+request['cap'],self.go['deadline_monotonic'])
        phase=self.out/request['name'];phase.mkdir(mode=0o700)
        process=None;timed=False;quiet=False
        try:
            authority=common_authority(self.root,self.packet)
            deadline=min(deadline,authority['end']-.5)
            require(deadline>started+.6,'insufficient original budget')
            validate_go(self.go,self.packet,self.packet_sha,time.monotonic())
            require(not (self.out/'first-red.json').exists(),'first red already present')
            verify_sources(self.root,self.packet)
            verify_inputs(self.packet,request['kind']=='compile')
            if request['kind']=='compile':
                require(not self.compiled,'compile already consumed');self.compiled=True
                write_new(self.out/'compile-used.json',{'used':True,'time':time.monotonic()})
                require(not (self.out/'client').exists(),'exclusive output required')
                require(request['argv']==compile_argv(self.packet,self.out),'compiler argv differs')
            else:
                require(self.compiled and self.client_sha==digest(self.out/'client'),'compiled ELF changed')
                require(request['argv'][0]==str(self.out/'client') and request['argv'][1:] in ([],['--validate-request']),'non-parser argv')
            require(time.monotonic()<deadline-.5,'preflight exhausted whole cap')
            from tool_phase import identity
            body=dict(parent=identity(os.getpid()),request=request,started=started,deadline=deadline,phase=str(phase),packet_sha=self.packet_sha,
                      packet_root=str(self.root),go=self.go,expected_cgroup=own_cgroup())
            req=phase/'request.json';write_new(req,body)
            env={k:os.environ[k] for k in ['RAG3WEAVER_VALIDATION_RECU','RAG3WEAVER_VALIDATION_JETON'] if k in os.environ}
            env.update(PATH='/usr/bin:/bin',HOME=str(self.out/'home'),TMPDIR=str(self.out/'tmp'),LANG='C.UTF-8')
            process=subprocess.Popen([sys.executable,str(self.root/'tool_phase.py'),str(req),digest(req)],
                stdin=subprocess.DEVNULL,stdout=subprocess.PIPE,stderr=subprocess.PIPE,env=env,start_new_session=True)
            try:controller_out,controller_err=process.communicate(timeout=max(.001,deadline-time.monotonic()-.35))
            except subprocess.TimeoutExpired:
                timed=True;stop_group(process.pid,body['expected_cgroup'])
                controller_out,controller_err=process.communicate(timeout=max(.001,deadline-time.monotonic()-.1))
            members,unknown=group_members(process.pid)
            if members:stop_group(process.pid,body['expected_cgroup'])
            while time.monotonic()<deadline-.05:
                members,unknown=group_members(process.pid)
                if not members:break
                time.sleep(.005)
            quiet=not members and not unknown and process.returncode is not None
            write_new(phase/'controller.json',{'code':process.returncode,'stdout_hex':controller_out[:MAX_RAW].hex(),
                      'stderr_hex':controller_err[:MAX_RAW].hex(),'timed_out':timed,'group_quiet':quiet})
            result_path=phase/'phase-result.json'
            record=read_json(result_path) if result_path.exists() else {}
            stdout=(phase/'stdout.raw').read_bytes()[:MAX_RAW] if (phase/'stdout.raw').exists() else b''
            stderr=(phase/'stderr.raw').read_bytes()[:MAX_RAW] if (phase/'stderr.raw').exists() else b''
            result=dict(name=request['name'],returncode=record.get('returncode'),timed_out=timed or process.returncode!=0 or (phase/'refusal.json').exists(),
                        quiet=quiet and record.get('native_reaped') is True,raw_published=record.get('raw_published') is True,
                        within_whole=time.monotonic()<=deadline and record.get('within_whole') is True,
                        stdin_started_inside_whole=record.get('stdin_started_inside_whole') is True,
                        native_invoked=record.get('native_invoked') is True)
            write_new(phase/'result.json',result);sync_dir(phase)
            # Dernières opérations fsync/phase/horloge également dans la borne unique.
            common_authority(self.root,self.packet)
            if request['kind']=='compile' and result['returncode']==0 and (self.out/'client').is_file():
                self.client_sha=digest(self.out/'client')
                verify_inputs(self.packet,True)
            common_authority(self.root,self.packet)
            result['within_whole']=result['within_whole'] and time.monotonic()<=deadline
            result.update(stdout=stdout,stderr=stderr)
            return result
        except Exception:
            if process is not None:
                stop_group(process.pid,own_cgroup())
                try:process.wait(timeout=max(.001,deadline-time.monotonic()))
                except subprocess.TimeoutExpired:pass
            raise


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--run',action='store_true');parser.add_argument('--packet-sha',required=True)
    parser.add_argument('--go');parser.add_argument('--go-sha')
    args=parser.parse_args();require(args.run and args.go and args.go_sha,'no tool GO; PREPARATION ONLY')
    root=Path(__file__).resolve().parent;packet=read_json(root/'packet.json',args.packet_sha)
    require(packet['packet_root']==str(root),'packet location differs')
    authority=common_authority(root,packet);go=read_json(args.go,args.go_sha)
    validate_go(go,packet,args.packet_sha,time.monotonic());verify_sources(root,packet)
    out=Path(go['output']).absolute();parent=Path(packet['output_parent']).absolute()
    require(out.resolve()==out and out.parent==parent and not out.exists(),'output must be exclusive under prepared parent')
    out.mkdir(mode=0o700);(out/'home').mkdir();(out/'tmp').mkdir()
    require(go['deadline_monotonic']<=authority['end'],'GO beyond original common END')
    cases=read_json(root/'tool-qualification-cases.json')['cases']
    config=dict(compile_argv=compile_argv(packet,out),client_path=str(out/'client'),compile_cap=60,worker_cap=5,
                sql_authorized=False,docker_authorized=False,cases=cases)
    try:
        results=orchestrate(config,NativeBackend(root,packet,args.packet_sha,go,out))
        summary=[{k:v for k,v in x.items() if k not in ('stdout','stderr')} for x in results]
        write_new(out/'qualification.json',{'verdict':'VERT','kind':'compile-and-parse-only','phases':summary,
                  'sql_executed':False,'transport_qualified':False,'client_sha256':digest(out/'client')})
        common_authority(root,packet);require(time.monotonic()<min(go['deadline_monotonic'],authority['end']),'late tool completion')
    except Exception as error:
        marker=out/'first-red.json'
        if not marker.exists():write_new(marker,{'first_red_monotonic':time.monotonic(),'reason':type(error).__name__+': '+str(error)})
        raise


if __name__=='__main__':
    try:main()
    except Exception as error:
        print(type(error).__name__+': '+str(error),file=sys.stderr);raise SystemExit(1)
