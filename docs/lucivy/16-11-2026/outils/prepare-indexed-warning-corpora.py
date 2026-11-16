"""Prepare admitted public corpus copies only; never invoke a test or compiler."""
import hashlib,json,os,secrets,stat,subprocess,tarfile,time
from pathlib import Path
END=time.monotonic()+180
H=Path.home(); NONCE=secrets.token_hex(16)
Q=H/'.cache/rag3weaver-build/cx-lucivy-indexed-warning-core-16nov-v1'
ARCHIVE=H/'.cache/lucivy-corpora/linux-linux-2.6.0.tar.gz'
CLONE=H/'.cache/lucivy-tests/rag3db-clone'
COMMIT='1b22409b31a79543a9fe63e516c5b311bc43f1da'
A_SHA='4d0299b3f9a5149635173998fc104c88c533523a2b43098a95b4ceba72dc6f73'
KERNEL=Path('/tmp/lucivy-cmp')
def budget():
    assert time.monotonic()<END,'preparation deadline exceeded'
def sha(p):
    budget();h=hashlib.sha256()
    with p.open('rb') as f:
        for b in iter(lambda:f.read(1024*1024),b''):h.update(b)
    return h.hexdigest()
def write(p,data):
    budget();p.parent.mkdir(parents=True,exist_ok=True,mode=0o700)
    with p.open('xb') as f:f.write(data)
    p.chmod(0o444)
def save(p,data):write(p,(json.dumps(data,indent=2)+'\n').encode())
def forbidden(path):return any(p.startswith('.env') or '.vault' in p or p=='.git' for p in path.parts)
def git(*args):
    budget();return subprocess.check_output(['git','-C',str(CLONE),*args])
assert sha(ARCHIVE)==A_SHA
assert git('rev-parse','HEAD').decode().strip()==COMMIT
for p in [Q,ARCHIVE,CLONE]:
    cur=Path('/')
    for part in p.parts[1:]:
        cur/=part
        if cur.exists():assert not cur.is_symlink(),str(cur)
assert ARCHIVE.stat().st_uid==os.getuid() and CLONE.stat().st_uid==os.getuid()
assert not Q.exists() and not Q.is_symlink()
assert not KERNEL.exists() and not KERNEL.is_symlink()
Q.mkdir(mode=0o700);KERNEL.mkdir(mode=0o700)
C=Q/'corpora';C.mkdir(mode=0o700)
R=C/'rag3db';R.mkdir(mode=0o700)
archive_stat=ARCHIVE.stat();files={};excluded=[]
with tarfile.open(ARCHIVE,'r:gz') as tf:
    for m in tf:
        budget();p=Path(m.name)
        assert not p.is_absolute() and '..' not in p.parts and p.parts[0]=='linux-2.6.0'
        rel=Path(*p.parts[1:])
        if str(rel)=='.':continue
        if forbidden(rel):excluded.append({'path':str(rel),'reason':'forbidden name'});continue
        if m.isdir():(KERNEL/rel).mkdir(parents=True,exist_ok=True,mode=0o700);continue
        if not m.isfile():excluded.append({'path':str(rel),'reason':'non-regular member','type':repr(m.type)});continue
        data=tf.extractfile(m).read();write(KERNEL/rel,data);files[str(rel)]=hashlib.sha256(data).hexdigest()
assert 'COPYING' in files
linux=dict(root=str(KERNEL),uid=os.getuid(),nonce=NONCE,archive=str(ARCHIVE),archive_sha256=A_SHA,
 origin='https://mirrors.edge.kernel.org/pub/linux/kernel/v2.6/linux-2.6.0.tar.gz',version='2.6.0',
 qualification='warning core regression only; never Linux 7.2 ULP',licence_file='COPYING',
 licence_sha256=files['COPYING'],files=files,excluded=excluded)
save(C/'linux-manifest.json',linux)
entries=[];excluded=[]
for record in git('ls-tree','-r','-z',COMMIT).split(b'\0'):
    if not record:continue
    meta,name=record.split(b'\t',1);mode,kind,oid=meta.decode().split();rel=Path(os.fsdecode(name))
    assert not rel.is_absolute() and '..' not in rel.parts
    if forbidden(rel) or mode not in ['100644','100755'] or kind!='blob':
        excluded.append({'path':str(rel),'mode':mode,'reason':'forbidden name or non-regular git entry'});continue
    entries.append((rel,oid))
proc=subprocess.Popen(['git','-C',str(CLONE),'cat-file','--batch'],stdin=subprocess.PIPE,stdout=subprocess.PIPE)
rfiles={}
for rel,oid in entries:
    budget();proc.stdin.write((oid+'\n').encode());proc.stdin.flush()
    header=proc.stdout.readline().decode().split();assert header[0]==oid and header[1]=='blob'
    size=int(header[2]);data=proc.stdout.read(size);assert len(data)==size and proc.stdout.read(1)==b'\n'
    write(R/rel,data);rfiles[str(rel)]=hashlib.sha256(data).hexdigest()
proc.stdin.close();assert proc.wait(timeout=10)==0
licences={name:digest for name,digest in rfiles.items() if '/' not in name and ('LICENSE' in name.upper() or 'COPYING' in name.upper())}
assert licences
save(C/'rag3db-manifest.json',dict(root=str(R),uid=os.getuid(),nonce=NONCE,source_cache=str(CLONE),
 commit=COMMIT,origin='https://github.com/L-Defraiteur/rag3db.git',files=rfiles,licences=licences,excluded=excluded,
 selection='all regular tracked blobs at commit, except forbidden names; no working-tree content read'))
selected=[]
def walk(d):
    for p in sorted(d.iterdir()):
        if p.is_dir():walk(p)
        elif p.suffix in ['.c','.h','.rst','.txt'] and 0<p.stat().st_size<=60000:
            try:p.read_text(encoding='utf-8')
            except UnicodeError:continue
            selected.append(str(p.relative_to(KERNEL)))
walk(KERNEL);assert len(selected)>=400
save(C/'linux-selections.json',dict(order='sorted depth-first; extensions c/h/rst/txt, size 1..60000, valid UTF-8',
 eligible_count=len(selected),first300=selected[:300],first400=selected[:400],
 tests={'test_derived_in_ram':300,'test_dictionary_index':[300,400],'test_positions_off':300}))
assert sha(ARCHIVE)==A_SHA and ARCHIVE.stat()==archive_stat
assert git('rev-parse','HEAD').decode().strip()==COMMIT
save(Q/'corpus-preparation.json',dict(nonce=NONCE,uid=os.getuid(),deadline=END,finished=time.monotonic(),
 kernel_files=len(files),rag3db_files=len(rfiles),licences=licences,
 linux_manifest_sha256=sha(C/'linux-manifest.json'),rag3db_manifest_sha256=sha(C/'rag3db-manifest.json'),
 kernel_root=str(KERNEL),rag3db_root=str(R),tests_executed=0,cargo_invoked=False))
print(json.dumps({'package':str(Q),'nonce':NONCE,'kernel_files':len(files),'rag3db_files':len(rfiles),
 'eligible_kernel':len(selected),'linux_manifest_sha256':sha(C/'linux-manifest.json'),
 'rag3db_manifest_sha256':sha(C/'rag3db-manifest.json'),'remaining':END-time.monotonic()}))
