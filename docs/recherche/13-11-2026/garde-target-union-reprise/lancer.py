#!/usr/bin/env python3
"""Prepare a reviewed controller outside both frozen worktrees, then actual poste."""
from pathlib import Path
import hashlib,json,os,shutil,subprocess,sys,stat,time
R=Path('/home/lucied/.cache/rag3weaver-build');W=R/'cx-wt-recherche-garde-target-union-r2';CONTROL=Path(__file__).resolve().parents[4]
APP='9ec421069320b2f0cca4099fa27f83059e944de9';PILOT=sys.argv[1]
P=R/'cx-validation-garde-target-union-v2';Q=R/'cx-pilote-garde-target-union-v2';S=R/'cx-confinement-garde-target-union-v2';T=R/'cx-target-garde-target-qualification-v1'
assert os.uname().nodename.lower()=='luciepc'
assert sys.argv==[str(Path(__file__).resolve()),PILOT] and sys.orig_argv[1:]==sys.argv, 'exact launcher argv required, no Python flags'
assert time.monotonic()<919393.915525602-6, 'original deadline exhausted'
for root,sha in [(W,APP),(CONTROL,PILOT)]:
 assert subprocess.check_output(['git','-C',str(root),'rev-parse','HEAD'],text=True).strip()==sha
 assert not subprocess.check_output(['git','-C',str(root),'status','--porcelain'],text=True).strip()
cp=W/'extension/rag3weaver/codeparsers'
assert (cp/'Cargo.toml').is_file(),'initialize the exact tracked submodule before freezing app W'
expected=subprocess.check_output(['git','-C',str(W),'ls-tree',APP,'extension/rag3weaver/codeparsers'],text=True).split()[2]
assert subprocess.check_output(['git','-C',str(cp),'rev-parse','HEAD'],text=True).strip()==expected
assert shutil.which('gdb'),'gdb needed for deterministic maps at first exact test main'
assert not any(p.exists() for p in [P,Q,S]),'proof/controller/scope must be new; no reset'
assert T.is_dir() and T.resolve()==T and T.stat().st_uid==os.getuid()
old=R/'cx-validation-garde-target-v1';resume=R/'cx-validation-garde-target-v2';previous=json.loads((old/'source.json').read_text())
assert previous['app']=='698b952ab89fd48ac112645650000ebae34e6742'
assert (resume/'verdict-compose.json').is_file() and json.loads((resume/'verdict-compose.json').read_text())['verdict']=='VERT'
for source in [previous,json.loads((resume/'source.json').read_text())]:assert json.loads(Path(source['receipt']).read_text())['etat']=='termine'
ledger=json.loads((old/'ledger.json').read_text());assert Path(ledger['binary']).is_relative_to(T)
for file in [Path(ledger['binary']),old/'binaire-cpu',resume/'binaire-cpu']:
 with file.open('rb') as f:assert hashlib.file_digest(f,'sha256').hexdigest()==ledger['sha256']
oldc=R/'cx-wt-recherche-garde-target/extension/rag3weaver'
assert oldc==R/'cx-wt-recherche-garde-target/extension/rag3weaver'
assert subprocess.check_output(['git','-C',str(oldc),'rev-parse','HEAD'],text=True).strip()==previous['app']
assert not subprocess.check_output(['git','-C',str(oldc),'status','--porcelain'],text=True).strip()
assert (T/'.arbre-du-target').is_file() and not (T/'.arbre-du-target').is_symlink() and (T/'.arbre-du-target').read_text().strip()==str(oldc)
Q.mkdir();records={}
# Fresh scan of this private cache / source context, never other targets.
found=[];unknown=[]
for proc in Path('/proc').iterdir():
 if not proc.name.isdigit() or int(proc.name)==os.getpid():continue
 try:
  if proc.stat().st_uid!=os.getuid():continue
  st=(proc/'stat').read_text().rsplit(') ',1)[1].split()
  if st[0]=='Z':continue
  for field in [proc/'exe',proc/'cwd',*list((proc/'fd').iterdir())]:
   try:
    value=os.readlink(field)
    if any(value==str(root) or value.startswith(str(root)+'/') for root in [T,W]):found.append(dict(pid=proc.name,starttime=st[19],field=str(field),path=value))
   except (FileNotFoundError,ProcessLookupError):pass
 except PermissionError:unknown.append(proc.name)
 except (FileNotFoundError,ProcessLookupError):pass
(Q/'cache-before.json').write_text(json.dumps(dict(previous_app=previous['app'],previous_receipt=previous['receipt'],old_crate=str(oldc),previous_ledger_sha256=hashlib.sha256((old/'ledger.json').read_bytes()).hexdigest(),target=str(T),inode=T.stat().st_ino,device=T.stat().st_dev,mode=T.stat().st_mode,uid=T.stat().st_uid,observed=time.monotonic(),quiescence=found,unknown_baseline=unknown),indent=2));assert not found

for name in ['driver.py','finaliser.py','sdk-exec.py','attester.py','capture-stable.py','superviser-elf.py','provenance-primitives.json','admettre.py']:
 src=Path(__file__).with_name(name);shutil.copy2(src,Q/name);records[name]=hashlib.sha256(src.read_bytes()).hexdigest()
common=W/'extension/rag3weaver/scripts/confiner_validation.py';shutil.copy2(common,Q/'confiner_validation.py');records[common.name]=hashlib.sha256(common.read_bytes()).hexdigest()
(Q/'provenance.json').write_text(json.dumps(dict(app=APP,pilot=PILOT,control=str(CONTROL),files=records),indent=2))
os.chdir('/var/tmp');env=os.environ.copy()
# Establish the user manager bus environment BEFORE poste and driver whitelist.
# No restart or replacement of an existing manager/service is performed.
runtime=Path('/run/user')/str(os.getuid());userbus=runtime/'bus'
assert runtime.resolve()==runtime and userbus.resolve()==userbus
runtime_before=runtime.lstat();bus_before=userbus.lstat()
assert stat.S_ISDIR(runtime_before.st_mode) and runtime_before.st_uid==os.getuid()
assert stat.S_ISSOCK(bus_before.st_mode) and bus_before.st_uid==os.getuid()

env.update(XDG_RUNTIME_DIR=str(runtime),DBUS_SESSION_BUS_ADDRESS='unix:path='+str(userbus),POSTE_MEM_MAX='32G',RAG3WEAVER_BUILD_MEMORY_HIGH='0')
health=subprocess.run(['busctl','--user','--timeout=5','call','org.freedesktop.DBus','/org/freedesktop/DBus','org.freedesktop.DBus.Peer','Ping'],env=env,text=True,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
for path,before in [(runtime,runtime_before),(userbus,bus_before)]:
 after=path.lstat()
 assert (before.st_dev,before.st_ino,before.st_mode,before.st_uid)==(after.st_dev,after.st_ino,after.st_mode,after.st_uid),'user manager bus changed during Ping'
(Q/'user-bus-before-poste.json').write_text(json.dumps(dict(runtime=str(runtime),address=env['DBUS_SESSION_BUS_ADDRESS'],code=health.returncode,error=health.stderr),indent=2));assert health.returncode==0,'user manager bus unavailable; refuse before poste'

command=[str(W/'extension/rag3weaver/scripts/poste'),'lourd','python3',str(Q/'admettre.py')]
result=subprocess.run(command,env=env)
# Outside the retired bus/cgroup and outside W, even after a refused admission.
final=subprocess.run(['python3',str(Q/'finaliser.py'),str(result.returncode)],env=env)
sys.exit(final.returncode)
