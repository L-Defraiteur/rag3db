"""Known prior CPU green provenance, read-only; no adoption, marker write or Cargo."""
from pathlib import Path
import hashlib,json,os,stat,subprocess
R=Path.home()/'.cache/rag3weaver-build'
OLD=Path.home()/'git_workspaces/rag3db-cx-ed-vertex-token-fix-r1'
OLD_APP='c38fc3e4ddd8c806de904c7d08eb43b4988fec99'
OLD_PILOT='441002d11bd07b7e8b77b0351d6e522ac5a9c6db'
OLD_P=R/'cx-ed-vertex-token-green-v1';OLD_Q=R/'cx-ed-vertex-token-green-controller-v3'
ARCHIVE=R/'cx-preuves-ed-vertex-token-green-v1'
MANIFEST='7a73c248158fe1587cb0045db06aca68f63589528b716416e37fbe2b38cba0a6'
ELF='8cbcefea72d6f584cec7e2c87f42eaeda32c9026ba6c737767a200ae65395406'
def sha(path):
 with Path(path).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def git(root,*args):return subprocess.check_output(['git','-C',str(root),*args],text=True).strip()
def identity(path):
 path=Path(path);s=path.lstat();assert path.resolve()==path and s.st_uid==os.getuid()
 return dict(path=str(path),dev=s.st_dev,ino=s.st_ino,mode=s.st_mode,uid=s.st_uid)
def capture(target):
 assert sha(ARCHIVE/'manifest.json')==MANIFEST
 rows=json.loads((ARCHIVE/'manifest.json').read_text());indexed={r['path']:r for r in rows}
 assert len(indexed)==len(rows)==59
 for row in rows:
  p=ARCHIVE/row['path'];assert p.stat().st_size==row['size'] and sha(p)==row['sha256']
 def matched(actual,name):
  value=sha(actual);assert value==indexed[name]['sha256'];return value
 assert git(OLD,'rev-parse','HEAD')==OLD_APP and not git(OLD,'status','--porcelain')
 sub=OLD/'extension/rag3weaver/codeparsers';assert git(sub,'rev-parse','HEAD')==git(OLD,'ls-tree',OLD_APP,'extension/rag3weaver/codeparsers').split()[2] and not git(sub,'status','--porcelain')
 oldcfg=json.loads((OLD_Q/'configuration.json').read_text());matched(OLD_Q/'configuration.json','controller/configuration.json')
 assert oldcfg['app']==OLD_APP and oldcfg['pilot']==OLD_PILOT
 control=Path(oldcfg['control']);assert git(control,'rev-parse','HEAD')==OLD_PILOT and not git(control,'status','--porcelain')
 for name,h in oldcfg['copies'].items():assert sha(OLD_Q/name)==h
 matched(OLD_P/'authority-path.json','client/authority-path.json')
 receipt=Path(json.loads((OLD_P/'authority-path.json').read_text())['path']);auth=json.loads(receipt.read_text());scope=json.loads((receipt.parent/'verdict.json').read_text())
 assert auth['etat']=='termine' and scope['verdict']=='VERT' and scope['code']==0 and not scope['restes'] and scope['nettoye'] and not Path(scope['prive']).exists()
 matched(receipt.parent/'verdict.json','supervisor/verdict.json');matched(receipt.parent/'enfants.json','supervisor/enfants.json')
 for name in ['ledger-before-tests.json','ledger-published.json','driver-result.json','composed-result.json','quiescence-independent.json']:matched(OLD_P/name,'client/'+name)
 ledger=json.loads((OLD_P/'ledger-before-tests.json').read_text());driver=json.loads((OLD_P/'driver-result.json').read_text());composition=json.loads((OLD_P/'composed-result.json').read_text());publication=json.loads((OLD_P/'ledger-published.json').read_text())
 assert ledger['source']==OLD_APP and ledger['pilot']==OLD_PILOT and ledger['features']==['daemon','openai-llm']
 assert driver['status']=='CPU_VERTEX_JUDGES_PASSED' and driver['exact_count']==7 and composition['status']=='CPU_VERTEX_CHECKED' and composition['returncode']==0 and composition['source']==OLD_APP and composition['pilot']==OLD_PILOT
 assert not (OLD_P/'first-red.json').exists() and auth['echeance']==1191197.356433272
 assert publication['source']==OLD_APP and publication['two_hosts_rehashed'] is True and publication['elf_sha256']==ELF and publication['ledger_sha256']==sha(OLD_P/'ledger-before-tests.json')
 original=Path(ledger['binary']);assert original.is_relative_to(target) and sha(original)==sha(ledger['copy'])==sha(ARCHIVE/'client/unit-tests')==ledger['sha256']==ELF
 t=identity(target);assert stat.S_ISDIR(t['mode'])
 marker=target/'.arbre-du-target';m=identity(marker);assert stat.S_ISREG(m['mode']) and marker.read_text().strip()==str(OLD/'extension/rag3weaver')
 fingerprints=[]
 for directory in sorted(target.rglob('.fingerprint')):
  for p in sorted(directory.rglob('*')):
   if p.is_file():fingerprints.append([str(p.relative_to(target)),sha(p)])
 assert fingerprints
 return dict(target=t,marker=m,owner=marker.read_text().strip(),old_source=OLD_APP,old_control=OLD_PILOT,archive_manifest=MANIFEST,authority=dict(path=str(receipt),sha256=sha(receipt),state=auth['etat']),verdict_sha256=sha(receipt.parent/'verdict.json'),children_sha256=sha(receipt.parent/'enfants.json'),ledger_sha256=sha(OLD_P/'ledger-before-tests.json'),elf=ELF,fingerprints=fingerprints)
