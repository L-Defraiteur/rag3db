#!/usr/bin/env python3
"""Independent final scope checks, outside W and outside the validation bus."""
from pathlib import Path
import hashlib,json,os,sys,time
R=Path('/home/lucied/.cache/rag3weaver-build');P=R/'cx-validation-couches-2a-union-v2';Q=Path(__file__).parent;W=R/'cx-wt-couches-union-r2';T=R/'cx-target-contenu-derive';rc=int(sys.argv[1]);record=dict(verdict='ROUGE',client_rc=rc)
P.mkdir(exist_ok=True)
try:
 assert rc==0 and (P/'VERDICT').read_text().strip()=='VERT'
 source=json.loads((P/'source.json').read_text());assert source['initial']==906224.174319861 and source['deadline']<=911024.174319861; receipt=Path(source['receipt']);scope=json.loads((receipt.parent/'verdict.json').read_text())
 assert receipt.parent.is_relative_to(R/'cx-confinement-couches-2a-union-v2')
 assert scope['verdict']=='VERT' and scope['code']==0 and scope['restes']==[] and scope['nettoye']
 assert not Path(scope['prive']).exists()
 assert time.monotonic()<source['deadline'] and time.monotonic()-source['initial']<4800
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
 inventory=json.loads((P/'inventaire-avant-runtime.json').read_text());assert set(inventory['artifacts'])=={'large','cpu'}
 for item in inventory['artifacts'].values():
  for file in [Path(item['binary']),Path(item['copy'])]:
   with file.open('rb') as f:assert hashlib.file_digest(f,'sha256').hexdigest()==item['sha256']
 main=json.loads((P/'large-main-process.json').read_text());att=json.loads((P/'large-main-attestation.json').read_text())
 assert att['pid']==main['pid'] and att['starttime']==main['starttime']
 assert att['exe']==main['expected_exe']==inventory['artifacts']['large']['binary'] and att['exe_sha256']==main['expected_sha256']==inventory['artifacts']['large']['sha256']
 assert att['cgroup']==main['cgroup']==source['cgroup'] and att['phase']=='lib-large' and att['principal']
 rows=[line for line in (P/att['maps']).read_text().splitlines() if 'librag3db.so' in line]
 assert rows==att['native'] and rows and all('/'+source['native']+'/librag3db.so' in line for line in rows)
 sdk=json.loads((P/'sdk-env-lib-large.json').read_text());profile=json.loads((P/'large-real-sdk-env.json').read_text())
 assert sdk['pid']==profile['pid']==main['pid'] and sdk['starttime']==profile['starttime']==main['starttime']
 assert sdk['env']==profile['env']==att['env'] and sdk['cgroup'].strip().split('::',1)[1]==source['cgroup']
 reprise=json.loads((P/'cache-reprise-carried.json').read_text());assert reprise['calls']==2 and reprise['before'] and not reprise['after']
 qualification=json.loads((P/'qualification.json').read_text());assert qualification==dict(features='VERT_8_SUR_8',large='VERT',cpu='VERT',publication='VERT',minimal_native_check='COMPILATION_F8_JEU8_CHECK_TESTS_SANS_RUNTIME')
 finished=time.monotonic()
 assert finished<source['deadline'] and finished-source['initial']<4800,'deadline after scan and ELF hashes'
 record.update(verdict='VERT',scope=scope,quiescence=found,unknown_baseline=unknown,qualification=qualification,elapsed=finished-source['initial'],deadline=source['deadline'],native_unit_runtime_observed=True,layer_storage_runtime_qualified=False)
except BaseException as error:record['error']=repr(error)
finally:
 # Recheck at publication too; expensive scan/hashes must never outlive END.
 if record['verdict']=='VERT':
  finished=time.monotonic()
  if finished>=source['deadline'] or finished-source['initial']>=4800:
   record.update(verdict='ROUGE',error='deadline before composed publication')
  record['elapsed']=finished-source['initial']
 (P/'verdict-compose.json').write_text(json.dumps(record,indent=2)+'\n');(P/'VERDICT_COMPOSE').write_text(record['verdict']+'\n');print(json.dumps(record),flush=True)
sys.exit(0 if record['verdict']=='VERT' else 1)
