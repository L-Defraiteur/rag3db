#!/usr/bin/env python3
"""Prepare a reviewed controller outside both frozen worktrees, then actual poste."""
from pathlib import Path
import hashlib,json,os,shutil,subprocess,sys,stat
R=Path('/home/lucied/.cache/rag3weaver-build');W=R/'cx-wt-couches-union-r2';CONTROL=Path(__file__).resolve().parents[4]
APP='236e1e9f46f7e73eda21ac8cfa765f45a29f4595';PILOT=sys.argv[1]
P=R/'cx-validation-couches-2a-union-v2';Q=R/'cx-pilote-couches-2a-union-v2';S=R/'cx-confinement-couches-2a-union-v2';T=R/'cx-target-contenu-derive'
assert os.uname().nodename.lower()=='luciepc'
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
old=R/'cx-validation-couches-2a-union-v1';previous=json.loads((old/'source.json').read_text())
assert previous['app']==APP and previous['target']==str(T)
assert previous['initial']==906224.174319861 and previous['deadline']==911024.174319861
authority=json.loads(Path(previous['receipt']).read_text());assert authority['etat']=='termine'
assert hashlib.sha256((old/'cache-reprise.json').read_bytes()).hexdigest()=='f6e2cb53026b8af0a0e0cf60363f737bc0a72314b0240287a4a44c451e1a60ee'
carried=json.loads((old/'cache-reprise.json').read_text());assert carried['calls']==2 and carried['before'] and not carried['after']
assert (T/'.arbre-du-target').read_text().strip()==str(W/'extension/rag3weaver')

Q.mkdir();records={}
shutil.copy2(old/'cache-reprise.json',Q/'cache-reprise-carried.json');records['cache-reprise-carried.json']=hashlib.sha256((Q/'cache-reprise-carried.json').read_bytes()).hexdigest()
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
(Q/'cache-before.json').write_text(json.dumps(dict(previous_app=previous['app'],previous_receipt=previous['receipt'],target=str(T),inode=T.stat().st_ino,quiescence=found,unknown_baseline=unknown),indent=2));assert not found

for name in ['driver.py','finaliser.py','sdk-exec.py','apres-poste.py']:
 src=Path(__file__).with_name(name);shutil.copy2(src,Q/name);records[name]=hashlib.sha256(src.read_bytes()).hexdigest()
common=W/'extension/rag3weaver/scripts/confiner_validation.py';shutil.copy2(common,Q/'confiner_validation.py');records[common.name]=hashlib.sha256(common.read_bytes()).hexdigest()
(Q/'provenance.json').write_text(json.dumps(dict(app=APP,pilot=PILOT,control=str(CONTROL),files=records,original=dict(initial=906224.174319861,deadline=911024.174319861),cache_carried_from=str(old)),indent=2))
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

command=[str(W/'extension/rag3weaver/scripts/poste'),'lourd','python3',str(Q/'apres-poste.py')]
result=subprocess.run(command,env=env)
# Outside the retired bus/cgroup and outside W, even after a refused admission.
final=subprocess.run(['python3',str(Q/'finaliser.py'),str(result.returncode)],env=env)
sys.exit(final.returncode)
