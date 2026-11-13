#!/usr/bin/env python3
"""Prepare a reviewed controller outside both frozen worktrees, then actual poste."""
from pathlib import Path
import hashlib,json,os,shutil,subprocess,sys,stat
R=Path('/home/lucied/.cache/rag3weaver-build');W=R/'cx-wt-couches-epinglees';CONTROL=Path(__file__).resolve().parents[4]
APP='69dc3fc25910f3e834652264edf6c71cca862ce0';PILOT=sys.argv[1]
P=R/'cx-validation-couches-2a-v1';Q=R/'cx-pilote-couches-2a-v1';S=R/'cx-confinement-couches-2a-v1';T=R/'cx-target-contenu-derive'
assert os.uname().nodename.lower()=='luciepc'
for root,sha in [(W,APP),(CONTROL,PILOT)]:
 assert subprocess.check_output(['git','-C',str(root),'rev-parse','HEAD'],text=True).strip()==sha
 assert not subprocess.check_output(['git','-C',str(root),'status','--porcelain'],text=True).strip()
cp=W/'extension/rag3weaver/codeparsers'
assert (cp/'Cargo.toml').is_file(),'initialize the exact tracked submodule before freezing app W'
expected=subprocess.check_output(['git','-C',str(W),'ls-tree',APP,'extension/rag3weaver/codeparsers'],text=True).split()[2]
assert subprocess.check_output(['git','-C',str(cp),'rev-parse','HEAD'],text=True).strip()==expected
assert shutil.which('gdb'),'gdb needed for deterministic maps at first exact test main'
assert not any(p.exists() for p in [P,Q,S,T]),'destinations/target must be new; no reset'
Q.mkdir();records={}
for name in ['driver.py','finaliser.py','native-check.py']:
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

command=[str(W/'extension/rag3weaver/scripts/poste'),'lourd','python3',str(Q/'confiner_validation.py'),'lancer','--delai','3000','--preuves',str(S),'--','python3',str(Q/'driver.py')]
result=subprocess.run(command,env=env)
# Outside the retired bus/cgroup and outside W, even after a refused admission.
final=subprocess.run(['python3',str(Q/'finaliser.py'),str(result.returncode)],env=env)
sys.exit(final.returncode)
