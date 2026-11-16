"""Known old red provenance, read-only; no adoption, marker write or Cargo."""
from pathlib import Path
import hashlib,json,os,stat,subprocess
R=Path.home()/'.cache/rag3weaver-build'
OLD=Path.home()/'git_workspaces/rag3db-cx-ed-vertex-token-refresh-r3'
OLD_APP='fe40b640eb0cc54645cbd552aa13b065ed1d2fa7'
OLD_PILOT='eb026f0a3c7c4143b511175b2bc92bd4f65c2db4'
OLD_P=R/'cx-ed-vertex-token-red-v1';OLD_Q=R/'cx-ed-vertex-token-controller-v1'
ARCHIVE=R/'cx-preuves-ed-vertex-token-red-v1'
MANIFEST='141a63c6d1c0172d01a17077de5deb341ff303d107aaf0e0c7f04e644097524b'
ELF='53b5a47ab371ba29e7301ba0bc3fbafe307a4ced4941e39b838eecc997dc8526'
def sha(path):
 with Path(path).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def git(root,*args):return subprocess.check_output(['git','-C',str(root),*args],text=True).strip()
def identity(path):
 path=Path(path);s=path.lstat();assert path.resolve()==path and s.st_uid==os.getuid()
 return dict(path=str(path),dev=s.st_dev,ino=s.st_ino,mode=s.st_mode,uid=s.st_uid)
def capture(target):
 assert sha(ARCHIVE/'manifest.json')==MANIFEST
 rows=json.loads((ARCHIVE/'manifest.json').read_text());indexed={r['path']:r for r in rows}
 assert len(indexed)==len(rows)==39
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
 assert auth['etat']=='termine' and scope['verdict']=='ROUGE' and scope['code']==1 and not scope['restes'] and scope['nettoye'] and not Path(scope['prive']).exists()
 matched(receipt.parent/'verdict.json','supervisor/verdict.json');matched(receipt.parent/'enfants.json','supervisor/enfants.json')
 matched(OLD_P/'ledger-before-tests.json','client/ledger-before-tests.json');matched(OLD_P/'ledger-published.json','client/ledger-published.json');matched(OLD_P/'first-red.json','client/first-red.json');matched(OLD_P/'red-observation.json','client/red-observation.json')
 ledger=json.loads((OLD_P/'ledger-before-tests.json').read_text());red=json.loads((OLD_P/'red-observation.json').read_text());publication=json.loads((OLD_P/'ledger-published.json').read_text())
 assert ledger['source']==OLD_APP and ledger['pilot']==OLD_PILOT and ledger['features']==['daemon','openai-llm']
 assert red['status']=='EXPECTED_TRANSPORT_RED_OBSERVED' and red['rc']==101 and not red['production_fixed'] and not red['persistent_401_judge_played']
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
