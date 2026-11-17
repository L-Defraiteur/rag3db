"""Sonde de préparation RO : capturer avant de juger, sans compiler ni contact.

Entrée live uniquement sous GO externe neuf et vrai Common/poste. Le cap physique
est celui du Common/timeout extérieur, pas une promesse d'I/O Python interruptible.
"""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import stat
import sys
import time
sys.dont_write_bytecode=True
MAX_FDINFO=16384
COMMON_SHA='6439fdd8b52c85fa4e74af0def2c667ef0bedae9b814cbbfd48e49946c4f74ea'


def require(ok,message):
    if not ok:raise ValueError(message)


def digest(raw):return hashlib.sha256(raw).hexdigest()


def stat_view(st):
    return dict(device=st.st_dev,major=os.major(st.st_dev),minor=os.minor(st.st_dev),inode=st.st_ino,
                uid=st.st_uid,mode=st.st_mode,size=st.st_size,mtime_ns=st.st_mtime_ns,ctime_ns=st.st_ctime_ns)


def opened_snapshot(path,*,directory=False,proc_magic=False):
    flags=os.O_RDONLY|os.O_CLOEXEC
    if not proc_magic:flags|=os.O_NOFOLLOW
    if directory:flags|=os.O_DIRECTORY
    observed=path.stat();fd=os.open(path,flags)
    try:return dict(path=str(path),stat=stat_view(observed),fstat=stat_view(os.fstat(fd)))
    finally:os.close(fd)


def read_prefix(path,limit):
    with path.open('rb') as stream:raw=stream.read(limit+1)
    return raw[:limit],len(raw)>limit


def collect_snapshot(common,table,expected_root,proc=Path('/proc')):
    started=time.monotonic();root=Path(expected_root)
    require(root.is_absolute() and root.resolve()==root,'noncanonical expected root')
    actual=Path(os.environ.get('POSTE_LOCK_DIR',str(Path.home()/'.cache/rag3weaver-build'))).absolute()
    require(actual==root,'Common lock root differs from expected')
    pid=common.poste_tenu(table);require(pid is not None and pid in table,'no genuine poste ancestor')
    row=dict(table[pid]);fiche=root/'fiches'/str(pid);lock=root/'poste.lock'
    raw,truncated=read_prefix(proc/str(pid)/'fdinfo/9',MAX_FDINFO)
    fiche_raw,fiche_truncated=read_prefix(fiche,4096)
    capture=dict(started_monotonic=started,observer_pid=os.getpid(),observer_uid=os.getuid(),
                 poste=dict(pid=pid,**row),expected_root=str(root),
                 root=opened_snapshot(root,directory=True),fiche=opened_snapshot(fiche),
                 lock=opened_snapshot(lock),poste_fd9=opened_snapshot(proc/str(pid)/'fd/9',proc_magic=True),
                 fdinfo_truncated=truncated,fdinfo_sha256=digest(raw),
                 fiche_truncated=fiche_truncated,fiche_sha256=digest(fiche_raw),
                 fiche_mode='READ' if fiche_raw.startswith(b'tient lourd ') else 'WRITE' if fiche_raw.startswith(b'tient mesure ') else None,
                 poste_after=common.processus().get(pid),transport_qualified=False)
    capture['expected_stat_tuple']=[capture['lock']['stat'][key] for key in ('major','minor','inode')]
    lines=[line for line in raw.decode('utf-8',errors='replace').splitlines() if line.startswith('lock:')]
    capture['kernel_lock_lines']=lines
    capture['observed_kernel_tuple']=None;capture['kernel_parse_error']=None
    try:
        require(len(lines)==1,'missing/ambiguous kernel lock row')
        fields=lines[0].split();require(len(fields)==9,'kernel lock row shape differs')
        device=fields[6].split(':');require(len(device)==3,'kernel device tuple shape differs')
        capture['observed_kernel_tuple']=[int(device[0],16),int(device[1],16),int(device[2])]
        capture['kernel_kind']=fields[2:5];capture['kernel_range']=fields[7:]
    except (ValueError,IndexError) as error:capture['kernel_parse_error']=str(error)
    capture['ended_monotonic']=time.monotonic()
    return capture,raw


def compare_snapshot(capture,raw):
    require(not capture['fdinfo_truncated'] and not capture['fiche_truncated'],'raw evidence truncated')
    require(digest(raw)==capture['fdinfo_sha256'],'fdinfo SHA differs')
    require(capture['poste_after']=={k:v for k,v in capture['poste'].items() if k!='pid'},'poste identity changed')
    for key in ('root','fiche','lock','poste_fd9'):
        item=capture[key]
        require(item['stat']==item['fstat'],'stat/fstat differs: '+key)
        require(item['stat']['uid']==capture['observer_uid'],'evidence UID differs: '+key)
        require(stat.S_ISDIR(item['stat']['mode']) if key=='root' else stat.S_ISREG(item['stat']['mode']),'evidence file kind differs: '+key)
    require(capture['fiche_mode'] in ('READ','WRITE'),'poste fiche not holding')
    require(not capture['kernel_parse_error'],capture['kernel_parse_error'] or 'kernel parse refused')
    require(capture['kernel_kind']==['FLOCK','ADVISORY',capture['fiche_mode']] and capture['kernel_range']==['0','EOF'],'kernel lock mode/range differs')
    require(all(capture['poste_fd9']['stat'][k]==capture['lock']['stat'][k] for k in ('device','inode')),'FD9 inode differs from lock path')
    require(capture['observed_kernel_tuple']==capture['expected_stat_tuple'],'kernel lock inode differs')


def write_new(path,raw):
    fd=os.open(path,os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600)
    with os.fdopen(fd,'wb') as stream:stream.write(raw);stream.flush();os.fsync(stream.fileno())


def encode(value):return (json.dumps(value,sort_keys=True,allow_nan=False,indent=2)+'\n').encode()


def sync_dir(path):
    fd=os.open(path,os.O_RDONLY|os.O_DIRECTORY)
    try:os.fsync(fd)
    finally:os.close(fd)


def publish_and_judge(out,capture,raw,*,deadline):
    try:
        write_new(out/'fdinfo.raw',raw)
        frozen=encode(capture);write_new(out/'capture.json',frozen)
        write_new(out/'capture.sha256',(digest(frozen)+'\n').encode());sync_dir(out)
        require(time.monotonic()<deadline,'capture publication exceeded original deadline')
        compare_snapshot(capture,raw)
        write_new(out/'comparison.json',encode(dict(verdict='MATCH',transport_qualified=False,compile_qualified=False,capture_sha256=digest(frozen))))
        sync_dir(out);require(time.monotonic()<deadline,'late comparison publication')
    except BaseException as error:
        if not (out/'first-red.json').exists():
            write_new(out/'first-red.json',encode(dict(reason=type(error).__name__+': '+str(error),observed_monotonic=time.monotonic())))
        raise


def load_json(path,expected_sha):
    require(path.is_absolute() and path.resolve()==path and path.stat().st_uid==os.getuid() and stat.S_ISREG(path.stat().st_mode),'invalid JSON file')
    raw,truncated=read_prefix(path,1024*1024);require(not truncated and digest(raw)==expected_sha,'JSON pin/bound differs')
    def pairs(items):
        result={}
        for key,value in items:
            require(key not in result,'duplicate JSON key');result[key]=value
        return result
    return json.loads(raw,object_pairs_hook=pairs)


def main():
    parser=argparse.ArgumentParser();parser.add_argument('--packet-sha',required=True)
    parser.add_argument('--go',type=Path,required=True);parser.add_argument('--go-sha',required=True)
    args=parser.parse_args();root=Path(__file__).resolve().parent
    packet=load_json(root/'diagnostic-packet.json',args.packet_sha)
    require({p.name for p in root.iterdir()}==set(packet['source_files'])|{'diagnostic-packet.json','diagnostic-packet.sha256'},'unexpected packet entry')
    for name,sha in packet['source_files'].items():
        p=root/name;require(p.parent==root and not p.is_symlink() and digest(p.read_bytes())==sha,'diagnostic source differs')
    path=root/'confiner_validation.py';require(digest(path.read_bytes())==COMMON_SHA,'Common pin differs')
    spec=importlib.util.spec_from_file_location('diagnostic_common',path);common=importlib.util.module_from_spec(spec);spec.loader.exec_module(common)
    table=common.processus();authority=common.autorite(table)
    go=load_json(args.go,args.go_sha)
    require(set(go)=={'kind','run_authorized','reference','nonce','packet_sha256','output','deadline_monotonic','compile_authorized','sql_authorized','network_authorized'},'diagnostic GO shape differs')
    require(go['kind']=='poste-lock-readonly-diagnostic' and go['run_authorized'] is True and go['reference'],'no exact diagnostic GO')
    require(go['compile_authorized'] is False and go['sql_authorized'] is False and go['network_authorized'] is False,'forbidden permission')
    require(isinstance(go['nonce'],str) and len(go['nonce'])==32 and all(c in '0123456789abcdef' for c in go['nonce']),'invalid nonce')
    require(go['packet_sha256']==args.packet_sha,'unbound diagnostic GO')
    deadline=go['deadline_monotonic'];require(type(deadline) in (int,float) and time.monotonic()<deadline<=authority['echeance'],'expired/excess diagnostic deadline')
    out=Path(go['output']);require(out.is_absolute() and out.resolve()==out and out.parent==Path(packet['output_parent']) and not out.exists(),'output not exclusive')
    out.mkdir(mode=0o700)
    try:
        write_new(out/'invocation.json',encode(dict(packet_sha256=args.packet_sha,go_sha256=args.go_sha,reference=go['reference'],nonce=go['nonce'],
                  supervisor=dict(pid=authority['pid'],start=authority['debut'],cgroup=authority['cgroup'],end=authority['echeance']),deadline_monotonic=deadline)))
        capture,raw=collect_snapshot(common,table,packet['poste_lock_root'])
        publish_and_judge(out,capture,raw,deadline=deadline)
        common.autorite(common.processus());require(time.monotonic()<deadline,'late diagnostic final return')
    except BaseException as error:
        if not (out/'first-red.json').exists():write_new(out/'first-red.json',encode(dict(reason=type(error).__name__+': '+str(error),observed_monotonic=time.monotonic())))
        raise


if __name__=='__main__':
    try:main()
    except Exception as error:
        print(type(error).__name__+': '+str(error),file=sys.stderr);raise SystemExit(1)
