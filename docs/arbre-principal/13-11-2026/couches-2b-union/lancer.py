#!/usr/bin/env python3
"""Prepare a reviewed controller outside both frozen worktrees, then actual poste."""
from pathlib import Path
import hashlib,json,os,shutil,subprocess,sys,stat,time
R=Path('/home/lucied/.cache/rag3weaver-build');W=R/'cx-wt-couches-2b-union';CONTROL=Path(__file__).resolve().parents[4]
APP='193d77cd71fa4cc1608e961ea4c8a8d3d445412e';PILOT=sys.argv[1]
P=R/'cx-validation-couches-2b-union-v1';Q=R/'cx-pilote-couches-2b-union-v1';S=R/'cx-confinement-couches-2b-union-v1';T=R/'cx-target-contenu-derive'
assert os.uname().nodename.lower()=='luciepc'
assert sys.argv==[str(Path(__file__).resolve()),PILOT] and sys.orig_argv[1:]==sys.argv, 'exact launcher argv required, no Python flags'
for root,sha in [(W,APP),(CONTROL,PILOT)]:
 assert subprocess.check_output(['git','-C',str(root),'rev-parse','HEAD'],text=True).strip()==sha
 assert not subprocess.check_output(['git','-C',str(root),'status','--porcelain'],text=True).strip()
cp=W/'extension/rag3weaver/codeparsers'
assert (cp/'Cargo.toml').is_file(),'initialize the exact tracked submodule before freezing app W'
expected=subprocess.check_output(['git','-C',str(W),'ls-tree',APP,'extension/rag3weaver/codeparsers'],text=True).split()[2]
assert subprocess.check_output(['git','-C',str(cp),'rev-parse','HEAD'],text=True).strip()==expected
assert not subprocess.check_output(['git','-C',str(cp),'status','--porcelain'],text=True).strip()
assert shutil.which('gdb'),'gdb needed for deterministic maps at first exact test main'
assert not any(p.exists() for p in [P,Q,S]),'proof/controller/scope must be new; no reset'
assert T.is_dir() and T.resolve()==T and T.stat().st_uid==os.getuid()
old=R/'cx-validation-couches-2b-a-v1';previous=json.loads((old/'source.json').read_text())
assert previous['app']=='affa6d03b5a57dd99810b6853783dc1a267f76b9'
assert json.loads((old/'verdict-compose.json').read_text())['verdict']=='VERT'
assert json.loads(Path(previous['receipt']).read_text())['etat']=='termine'
ledger=json.loads((old/'inventaire-avant-runtime.json').read_text());assert Path(ledger['binary']).is_relative_to(T)
assert hashlib.sha256((old/'inventaire-avant-runtime.json').read_bytes()).hexdigest()=='22e3ccb9af4c9711db3491d545486a41b231bff0cbb96ae23b78dfda909b1be2'
for file in [Path(ledger['binary']),old/'binaire-cpu']:
 with file.open('rb') as f:assert hashlib.file_digest(f,'sha256').hexdigest()==ledger['sha256']
oldc=R/'cx-wt-couches-2b-admission/extension/rag3weaver'
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
    if any(value==str(root) or value.startswith(str(root)+'/') for root in [T,oldc.parent.parent,W,CONTROL]):found.append(dict(pid=proc.name,starttime=st[19],field=str(field),path=value))
   except (FileNotFoundError,ProcessLookupError):pass
 except PermissionError:unknown.append(proc.name)
 except (FileNotFoundError,ProcessLookupError):pass
(Q/'cache-before.json').write_text(json.dumps(dict(previous_app=previous['app'],previous_receipt=previous['receipt'],old_crate=str(oldc),previous_ledger_sha256=hashlib.sha256((old/'inventaire-avant-runtime.json').read_bytes()).hexdigest(),target=str(T),inode=T.stat().st_ino,device=T.stat().st_dev,mode=T.stat().st_mode,uid=T.stat().st_uid,observed=time.monotonic(),quiescence=found,unknown_baseline=unknown),indent=2));assert not found

for name in ['driver.py','finaliser.py','sdk-exec.py','attester.py','capture-stable.py','superviser-elf.py','provenance-primitives.json','admettre.py']:
 src=Path(__file__).with_name(name);shutil.copy2(src,Q/name);records[name]=hashlib.sha256(src.read_bytes()).hexdigest()
expected_source=W/'docs/arbre-principal/13-11-2026/2b-a-juges-attendus.json';shutil.copy2(expected_source,Q/'expected.json');records['expected.json']=hashlib.sha256(expected_source.read_bytes()).hexdigest()
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
