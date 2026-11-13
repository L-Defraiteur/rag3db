#!/usr/bin/env python3
"""Independent final scope checks, outside W and outside the validation bus."""
from pathlib import Path
import hashlib,json,os,sys,time
R=Path('/home/lucied/.cache/rag3weaver-build');P=R/'cx-validation-couches-2b-union-v1';Q=Path(__file__).parent;W=R/'cx-wt-couches-2b-union';T=R/'cx-target-contenu-derive';rc=int(sys.argv[1]);record=dict(verdict='ROUGE',client_rc=rc)
P.mkdir(exist_ok=True)
try:
 assert rc==0 and (P/'VERDICT').read_text().strip()=='VERT'
 source=json.loads((P/'source.json').read_text());assert source['deadline']-source['initial']==4800;receipt=Path(source['receipt']);scope=json.loads((receipt.parent/'verdict.json').read_text())
 assert receipt.parent.is_relative_to(R/'cx-confinement-couches-2b-union-v1')
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
 inventory=json.loads((P/'inventaire-avant-runtime.json').read_text());assert set(inventory['artifacts'])=={'large','navigateur','cpu'}
 for item in inventory['artifacts'].values():
  for file in [Path(item['binary']),Path(item['copy'])]:
   with file.open('rb') as f:assert hashlib.file_digest(f,'sha256').hexdigest()==item['sha256']
 main=json.loads((P/'large-main-process.json').read_text());att=json.loads((P/'large-main-attestation.json').read_text())
 assert att['capture']=='gdb-main-same-inferior'
 real=json.loads((P/'elf-exit-real.json').read_text());detached=json.loads((P/'elf-detached.json').read_text());parent=json.loads((P/'elf-wait-parent.json').read_text())
 assert real['pid']==detached['pid']==parent['pid']==main['pid'] and real['starttime']==detached['starttime']==parent['starttime']==main['starttime']
 assert real['exit_code']==0 and real['wait_status']==0 and detached['tracer_pid']==0 and detached['observed']<real['observed']
 assert real['parent_pid']==parent['parent_pid']
 first=json.loads((P/'sdk-attempt-exec.json').read_text());second=json.loads((P/'sdk-attempt-main.json').read_text())
 assert first['pid']==second['pid']==main['pid'] and first['starttime']==second['starttime']==main['starttime']
 assert not first['diff'] and not second['diff'] and first['rawlen']>0 and second['rawlen']>0
 assert all(first['present'].values()) and all(second['present'].values())
 assert att['pid']==main['pid'] and att['starttime']==main['starttime']
 assert att['exe']==main['expected_exe']==inventory['artifacts']['large']['binary'] and att['exe_sha256']==main['expected_sha256']==inventory['artifacts']['large']['sha256']
 assert att['cgroup']==main['cgroup']==source['cgroup'] and att['phase']=='lib-large' and att['principal']
 rows=[line for line in (P/att['maps']).read_text().splitlines() if 'librag3db.so' in line]
 assert rows==att['native'] and rows and all('/'+source['native']+'/librag3db.so' in line for line in rows)
 sdk=json.loads((P/'sdk-env-lib-large.json').read_text());profile=json.loads((P/'large-real-sdk-env.json').read_text())
 assert sdk['pid']==profile['pid']==main['pid'] and sdk['starttime']==profile['starttime']==main['starttime']
 assert sdk['env']==profile['env']==att['env'] and sdk['cgroup'].strip().split('::',1)[1]==source['cgroup']
 cache=json.loads((Q/'cache-before.json').read_text());fresh=json.loads((P/'cache-after-poste.json').read_text());assert not cache['quiescence'] and not fresh['found'] and fresh['observed']>=fresh['before_observed']==cache['observed']
 assert fresh['inode']==cache['inode'] and fresh['device']==cache['device'] and fresh['marker']==cache['old_crate']
 assert (P/'cache-metadata-profile.json').exists() and not (P/'first-red.json').exists()
 assert (T/'.arbre-du-target').read_text().strip()==str(W/'extension/rag3weaver')
 assert (P/'features-real-sdk-env.json').exists() and (P/'nav-real-profile.json').exists()
 assets=json.loads((P/'assets-http-contre-git.json').read_text());assert len(assets)==2 and {v['asset'] for v in assets}=={'app.js','app.css'}
 qualification=json.loads((P/'qualification.json').read_text());assert qualification==dict(features='VERT_8_SUR_8',large='VERT',nav='VERT_40_SUR_40',assets='DEUX_CORPS_HTTP_IDENTIQUES_AUX_BLOBS_GIT',cpu='VERT_15_EXACTS',publication='VERT_25_SUR_25')
 with (R.parent/'rag3db-libs'/source['native']/'librag3db.so').open('rb') as stream:assert hashlib.file_digest(stream,'sha256').hexdigest()=='b014febe51417ca535218d1c0b18308e7d301157abf45027a5cbba70dfc54929'
 assert inventory['artifacts']['cpu']['features']==[] and len(inventory['cpu'])==15 and not inventory['cpu_ignored'] and len(inventory['publication'])==25 and not inventory['publication_ignored']
 assert set(inventory['cpu'])==set(json.loads((Q/'expected.json').read_text())['exacts'])
 assert hashlib.sha256((P/'inventaire-avant-runtime.json').read_bytes()).hexdigest()==(P/'inventaire-publie').read_text().strip()
 cpu=json.loads((P/'elf-main.json').read_text());assert cpu['exe']==inventory['artifacts']['cpu']['binary'] and cpu['cgroup']==source['cgroup']
 runtime=json.loads((P/'large-main-runtime-profile.json').read_text());assert runtime['pid']==main['pid'] and runtime['starttime']==main['starttime'] and runtime['env']==runtime['expected']
 assert runtime['env']['XDG_STATE_HOME']==source['state_roots']['xdg'] and runtime['env']['RAG3WEAVER_STATE_DIR']==source['state_roots']['rag']
 assert source['state_roots']==dict(xdg=str(P/'xdg-state'),rag=str(P/'state-fixture'))
 assert cpu['env']['XDG_STATE_HOME']==source['state_roots']['xdg'] and cpu['env']['RAG3WEAVER_STATE_DIR']==source['state_roots']['rag']
 assert not Path(source['state_roots']['xdg']).is_relative_to(Path(source['state_roots']['rag'])) and not Path(source['state_roots']['rag']).is_relative_to(Path(source['state_roots']['xdg']))
 assert not json.loads((P/'elf-main-native.json').read_text())['rows']
 finished=time.monotonic()
 assert finished<source['deadline'] and finished-source['initial']<4800,'deadline after scan and ELF hashes'
 record.update(verdict='VERT',scope=scope,quiescence=found,unknown_baseline=unknown,qualification=qualification,elapsed=finished-source['initial'],deadline=source['deadline'],native_unit_runtime_observed=True,assets_http_against_git=2,no_pg_bge_or_old_physical_lot=True)
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
