"""Trusted T0, preserved positive provenance, Git-only source transition."""
from pathlib import Path
import json,os,subprocess
import cache_state,evidence,policy,source742
from settings import *

def git(root,*args):return subprocess.check_output(['git','-C',str(root),*args],text=True).strip()

def previous():
 policy.require(evidence.sha(OLD_ARCHIVE/'manifest.json')==policy.OLD_MANIFEST,'72 proof manifest changed')
 rows=json.loads((OLD_ARCHIVE/'manifest.json').read_text());policy.require(len(rows)==72,'wrong proof count')
 for row in rows:
  p=OLD_ARCHIVE/row['path'];policy.require(p.resolve().is_relative_to(OLD_ARCHIVE) and p.stat().st_size==row['size'] and evidence.sha(p)==row['sha256'],'preserved positive proof changed')
 index={x['path']:x for x in rows}
 for name in ['result.json','driver-result.json','ledger-before-witnesses.json','ledger-published.json','runtime-permitted.json','positive-lib.json','positive-e2e.json']:
  policy.require(evidence.sha(OLD_P/name)==index['client/'+name]['sha256'],'live positive carrier changed')
 result=json.loads((OLD_P/'result.json').read_text());policy.require(result['status']=='GREEN_SETUP_TARGETED_ONLY' and result['source']==policy.OLD and result['pilot']==policy.OLD_PILOT,'old positive provenance differs')
 receipt=Path(json.loads((OLD_P/'authority-path.json').read_text())['path']);auth=json.loads(receipt.read_text());scope=json.loads((receipt.parent/'verdict.json').read_text())
 policy.require(auth['etat']=='termine' and scope['verdict']=='VERT' and scope['code']==0 and not scope['restes'] and scope['nettoye'] and not Path(scope['prive']).exists(),'old qualification still active/dirty')
 ledger=json.loads((OLD_P/'ledger-before-witnesses.json').read_text())
 for label,item in ledger['artifacts'].items():policy.require(evidence.sha(Path(item['copy']))==item['sha256'],'positive ELF copy changed')
 for name in ['verdict.json','enfants.json']:policy.require(evidence.sha(receipt.parent/name)==index['supervisor/'+name]['sha256'],'old scope evidence changed')
 cfg=json.loads((OLD_Q/'configuration.json').read_text());policy.require(git(Path(cfg['control']),'rev-parse','HEAD')==policy.OLD_PILOT and not git(Path(cfg['control']),'status','--porcelain'),'old controller changed')
 return dict(source=policy.OLD,pilot=policy.OLD_PILOT,authority=str(receipt),authority_sha256=evidence.sha(receipt),cgroup=auth['cgroup'],ledger_sha256=evidence.sha(OLD_P/'ledger-before-witnesses.json'),manifest_sha256=policy.OLD_MANIFEST)

def owned_gone(receipt):
 rows=json.loads((Path(receipt).parent/'enfants.json').read_text());alive=[];unknown=[]
 for row in rows:
  try:
   fields=(Path('/proc')/str(row['pid'])/'stat').read_text().rsplit(') ',1)[1].split()
   if fields[19]==str(row['debut']) and fields[0]!='Z':alive.append(row)
  except (FileNotFoundError,ProcessLookupError):continue
  except OSError as error:unknown.append(dict(pid=row['pid'],error=repr(error)))
 policy.require(not alive and not unknown,'owned process remains or cannot be attributed')
 return dict(observations=len(rows),alive=alive,unknown_owned=unknown)

def capture():
 st=cache_state.require_root(T);identity=dict(device=st.st_dev,inode=st.st_ino,uid=st.st_uid,path=str(T));policy.require((T/'.arbre-du-target').read_text().strip()==str(C),'T owner differs')
 policy.require(not TASKS.exists() and not TASKS.is_symlink(),'task-target root not fresh')
 return dict(identity=identity,whole=cache_state.inventory(SESSION,None),active=cache_state.inventory(T,None),source_owner=str(C))

def check_t0(record):
 cache_state.require_identity(T,record['identity']);policy.require(capture()==record,'trusted T0 changed before guard')

def outside_active():
 # Exclude the active native child and the ONE explicitly declared new task subtree.
 rows=cache_state.inventory(SESSION,T)
 return [row for row in rows if row['path']!='task-targets' and not row['path'].startswith('task-targets/')]

def source_clean(root,head):
 policy.require(git(root,'rev-parse','HEAD')==head and not git(root,'status','--porcelain'),'source/control dirty or wrong pin')
 for name in ['rebase-merge','rebase-apply']:policy.require(not Path(git(root,'rev-parse','--path-format=absolute','--git-path',name)).exists(),'rebase active')
 path=root/'extension/rag3weaver/codeparsers';oid=git(root,'ls-tree',head,'extension/rag3weaver/codeparsers').split()[2]
 policy.require(git(path,'rev-parse','HEAD')==oid and not git(path,'status','--porcelain'),'gitlink mismatch/dirty')

def freeze_and_transition():
 policy.require(os.uname().nodename.lower()=='luciepc' and not PREP.exists() and not ARCHIVE742.exists(),'preparation must be fresh on PC')
 source_clean(W,policy.OLD);prior=previous();gone=owned_gone(prior['authority']);before=capture();tracked=source742.tracked(W)
 PREP.mkdir(mode=0o700);evidence.save(PREP/'T0-before-git.json',before)
 subprocess.run(['git','-C',str(W),'worktree','add','--detach',str(ARCHIVE742),policy.OLD],check=True)
 subprocess.run(['git','-C',str(ARCHIVE742),'submodule','update','--init','extension/rag3weaver/codeparsers'],check=True)
 archive=source742.physical_archive(ARCHIVE742,policy.OLD);policy.require(archive['tracked']==tracked,'physical742 archive differs')
 policy.require(archive['gitdir']!=git(W,'rev-parse','--absolute-git-dir'),'archive administration shared')
 policy.require(not ({x['gitdir'] for x in archive['gitlink_admins']} & {x['gitdir'] for x in source742.gitlink_admins(W)}),'gitlink administration shared')
 evidence.save(PREP/'source742.json',archive)
 subprocess.run(['git','-C',str(W),'switch','--no-track','-c','cx-archi-global-source-pc-r2',policy.APP],check=True)
 source_clean(W,policy.APP);source742.verify_archive(archive);policy.require(previous()==prior and capture()==before,'Git-only transition changed cache/proof')
 evidence.save(PREP/'T0-after-git.json',capture());evidence.save(PREP/'outside-active.json',outside_active())
 evidence.save(PREP/'transition.json',dict(old=policy.OLD,new=policy.APP,archive_sha256=evidence.sha(PREP/'source742.json'),T0_sha256=evidence.sha(PREP/'T0-before-git.json'),old_proof=prior,owned=gone,cargo=False,guard=False))
