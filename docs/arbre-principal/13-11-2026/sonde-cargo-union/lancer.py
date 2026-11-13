#!/usr/bin/env python3
"""Reviewed observation only: cargo --version, no compile or target mutation."""
from pathlib import Path
import hashlib,json,os,shutil,stat,subprocess,sys
R=Path('/home/lucied/.cache/rag3weaver-build');W=R/'cx-wt-couches-union-r2';CONTROL=Path(__file__).resolve().parents[4];APP='236e1e9f46f7e73eda21ac8cfa765f45a29f4595';PILOT=sys.argv[1]
P=R/'cx-sonde-cargo-union-v1';Q=R/'cx-controle-sonde-cargo-union-v1';S=R/'cx-confinement-sonde-cargo-union-v1';T=R/'cx-target-contenu-derive';OLD=R/'cx-validation-couches-2a-union-v1'
assert os.uname().nodename.lower()=='luciepc'
for root,sha in [(W,APP),(CONTROL,PILOT)]:
 assert subprocess.check_output(['git','-C',str(root),'rev-parse','HEAD'],text=True).strip()==sha
 assert not subprocess.check_output(['git','-C',str(root),'status','--porcelain'],text=True).strip()
original=json.loads((OLD/'source.json').read_text());assert original['app']==APP and original['initial']==906224.174319861 and original['deadline']==911024.174319861
assert json.loads(Path(original['receipt']).read_text())['etat']=='termine'
reprise=json.loads((OLD/'cache-reprise.json').read_text());assert reprise['calls']==2 and reprise['before'] and not reprise['after']
assert T.resolve()==T and T.is_dir() and (T/'.arbre-du-target').read_text().strip()==str(W/'extension/rag3weaver')
assert not any(p.exists() for p in [P,Q,S]);P.mkdir();Q.mkdir();files={}
for name in ['apres-poste.py','sonde.py','finaliser.py']:
 src=Path(__file__).with_name(name);shutil.copy2(src,Q/name);files[name]=hashlib.sha256(src.read_bytes()).hexdigest()
src=W/'extension/rag3weaver/scripts/confiner_validation.py';shutil.copy2(src,Q/src.name);files[src.name]=hashlib.sha256(src.read_bytes()).hexdigest()
(Q/'provenance.json').write_text(json.dumps(dict(app=APP,pilot=PILOT,control=str(CONTROL),original=original,files=files),indent=2))
os.chdir('/var/tmp');env=os.environ.copy();runtime=Path('/run/user')/str(os.getuid());bus=runtime/'bus';assert runtime.resolve()==runtime and bus.resolve()==bus
a,b=runtime.lstat(),bus.lstat();assert stat.S_ISDIR(a.st_mode) and stat.S_ISSOCK(b.st_mode) and a.st_uid==b.st_uid==os.getuid()
env.update(XDG_RUNTIME_DIR=str(runtime),DBUS_SESSION_BUS_ADDRESS='unix:path='+str(bus),POSTE_MEM_MAX='32G',RAG3WEAVER_BUILD_MEMORY_HIGH='0')
h=subprocess.run(['busctl','--user','--timeout=5','call','org.freedesktop.DBus','/org/freedesktop/DBus','org.freedesktop.DBus.Peer','Ping'],env=env,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
for p,old in [(runtime,a),(bus,b)]:
 new=p.lstat();assert (old.st_dev,old.st_ino,old.st_mode,old.st_uid)==(new.st_dev,new.st_ino,new.st_mode,new.st_uid)
(P/'bus-manager.json').write_text(json.dumps(dict(code=h.returncode,error=h.stderr)));assert h.returncode==0
result=subprocess.run([str(W/'extension/rag3weaver/scripts/poste'),'lourd','python3',str(Q/'apres-poste.py')],env=env)
final=subprocess.run(['python3',str(Q/'finaliser.py'),str(result.returncode)],env=env);sys.exit(final.returncode)
