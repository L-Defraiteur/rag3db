#!/usr/bin/env python3
"""Independent final scope checks, outside W and outside the validation bus."""
from pathlib import Path
import hashlib,json,os,sys,time
R=Path('/home/lucied/.cache/rag3weaver-build');P=R/'cx-validation-couches-2b-a-v1';Q=Path(__file__).parent;W=R/'cx-wt-couches-2b-admission';T=R/'cx-target-contenu-derive';rc=int(sys.argv[1]);record=dict(verdict='ROUGE',client_rc=rc)
P.mkdir(exist_ok=True)
try:
 assert rc==0 and (P/'VERDICT').read_text().strip()=='VERT'
 source=json.loads((P/'source.json').read_text());receipt=Path(source['receipt']);scope=json.loads((receipt.parent/'verdict.json').read_text())
 assert receipt.parent.is_relative_to(R/'cx-confinement-couches-2b-a-v1')
 assert scope['verdict']=='VERT' and scope['code']==0 and scope['restes']==[] and scope['nettoye']
 assert not Path(scope['prive']).exists()
 assert time.monotonic()<source['deadline'] and time.monotonic()-source['initial']<1800
 quiet=json.loads((P/'quiescence.json').read_text());assert not quiet['found']
 owned=json.loads((P/'owned.json').read_text());control=Path(source['control']);found=[];unknown=[]
 for q in Path('/proc').iterdir():
  if not q.name.isdigit() or int(q.name)==os.getpid():continue
  stamp=None
  try:
   s=(q/'stat').read_text().rsplit(') ',1)[1].split();stamp=s[19]
   if s[0]=='Z' or (q.stat().st_uid!=os.getuid() and owned.get(q.name)!=s[19]):continue
   exe=os.readlink(q/'exe');cwd=os.readlink(q/'cwd')
   if exe.startswith(str(T)+'/') or any(cwd==str(root) or cwd.startswith(str(root)+'/') for root in [W,control]):found.append(dict(pid=q.name,starttime=s[19],exe=exe,cwd=cwd))
   if owned.get(q.name)==s[19]:found.append(dict(pid=q.name,starttime=s[19],observation='owned-live'))
  except PermissionError:
   unknown.append(q.name)
   if q.name in owned and (stamp is None or owned[q.name]==stamp):found.append(dict(pid=q.name,observation='UNKNOWN-owned-live'))
  except (FileNotFoundError,ProcessLookupError):pass
 assert not found,found
 inventory=json.loads((P/'inventaire-avant-runtime.json').read_text());assert inventory['features']==[] and len(inventory['admission'])==15 and len(inventory['publication'])==25 and not inventory['ignored'] and not inventory['publication_ignored']
 assert not (P/'first-red.json').exists()
 assert (T/'.arbre-du-target').read_text().strip()==str(W/'extension/rag3weaver')
 for file in [Path(inventory['binary']),P/'binaire-cpu']:
  with file.open('rb') as f:assert hashlib.file_digest(f,'sha256').hexdigest()==inventory['sha256']
 qualification=json.loads((P/'qualification.json').read_text());assert qualification==dict(cpu_exacts='VERT_15_EXACTS',cpu_lib='VERT_CPU_LIB')
 finished=time.monotonic()
 assert finished<source['deadline'] and finished-source['initial']<1800,'deadline after scan and ELF hashes'
 record.update(verdict='VERT',scope=scope,quiescence=found,unknown_baseline=unknown,qualification=qualification,elapsed=finished-source['initial'],deadline=source['deadline'],native_runtime_qualified=False)
except BaseException as error:record['error']=repr(error)
finally:
 # Recheck at publication too; expensive scan/hashes must never outlive END.
 if record['verdict']=='VERT':
  finished=time.monotonic()
  if finished>=source['deadline'] or finished-source['initial']>=1800:
   record.update(verdict='ROUGE',error='deadline before composed publication')
  record['elapsed']=finished-source['initial']
 (P/'verdict-compose.json').write_text(json.dumps(record,indent=2)+'\n');(P/'VERDICT_COMPOSE').write_text(record['verdict']+'\n');print(json.dumps(record),flush=True)
sys.exit(0 if record['verdict']=='VERT' else 1)
