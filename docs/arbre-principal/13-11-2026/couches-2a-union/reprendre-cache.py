#!/usr/bin/env python3
"""Two real common-guard calls: proven old owner, then mandatory invalidation."""
from pathlib import Path
import hashlib,json,os,re,stat,subprocess,sys
R=Path('/home/lucied/.cache/rag3weaver-build');OLD=R/'cx-validation-couches-2a-v1';NEW=R/'cx-wt-couches-union-r2/extension/rag3weaver';T=R/'cx-target-contenu-derive';proof=Path(sys.argv[1])
APP_OLD='69dc3fc25910f3e834652264edf6c71cca862ce0'
def git(root,*args):return subprocess.check_output(['git','-C',str(root),*args],env=dict(os.environ,GIT_OPTIONAL_LOCKS='0'),text=True).strip()
def digest(file):
 with Path(file).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
source=json.loads((OLD/'source.json').read_text());assert source['app']==APP_OLD and source['target']==str(T) and source['target_started_absent'] is True
oldw=Path(source['worktree']);oldc=oldw/'extension/rag3weaver'
assert NEW.resolve()==NEW and NEW!=oldc
assert oldc.resolve()==oldc and git(oldw,'rev-parse','HEAD')==APP_OLD and not git(oldw,'status','--porcelain')
profile=json.loads((OLD/'elf-main.json').read_text());assert profile['env']['CARGO_MANIFEST_DIR']==str(oldc)
assert (OLD/'VERDICT_COMPOSE').read_text().strip()=='VERT' and json.loads(Path(source['receipt']).read_text())['etat']=='termine'
ledger=json.loads((OLD/'inventaire-avant-runtime.json').read_text());assert digest(ledger['binary'])==digest(OLD/'binaire-cpu')==ledger['sha256']
assert T.resolve()==T and T.is_dir() and T.stat().st_uid==os.getuid()
marker=T/'.arbre-du-target';was_absent=not marker.exists()
assert not marker.is_symlink()
if marker.exists():
 st=marker.lstat();assert stat.S_ISREG(st.st_mode) and st.st_uid==os.getuid() and marker.read_text().strip()==str(oldc)
guard=NEW/'scripts/garde_target.sh';assert digest(guard)==digest(oldc/'scripts/garde_target.sh')
def fingerprints():return sorted(str(child.relative_to(T)) for folder in T.rglob('.fingerprint') if folder.is_dir() for child in folder.iterdir() if child.is_dir())
before=fingerprints();assert before
script='''set -euo pipefail
. "$1"
# Both mutations go through the existing primitive; old origin is proven above.
garder_le_target "$2" "$3"
RAG3WEAVER_TARGET_REPRENDRE=1 garder_le_target "$2" "$4"
'''
command=['bash','-c',script,'resume-proven-cache',str(guard),str(T),str(oldc),str(NEW)]
result=subprocess.run(command,cwd=NEW,env=os.environ.copy(),stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
(proof/'cache-common-guard.log').write_text(result.stdout);print(result.stdout,end='',flush=True)
assert result.returncode==0,('common target guard refused',result.returncode)
expected='target repris de '+str(oldc)+' : empreintes effacées pour '
assert expected in result.stdout,'mandatory common invalidation branch was not observed'
line=next(x for x in result.stdout.splitlines() if expected in x);packages=line.split(expected,1)[1].split();assert packages and 'rag3weaver' in packages
is_local=lambda path:any(re.fullmatch(re.escape(package)+r'-[0-9a-f]{16}',Path(path).name) for package in packages)
local_before=[p for p in before if is_local(p)];after=fingerprints();local_after=[p for p in after if is_local(p)]
assert local_before and not local_after
assert set(before)-set(local_before)<=set(after),'unrelated fingerprints removed'
assert marker.read_text().strip()==str(NEW) and not marker.is_symlink()
record=dict(old_app=APP_OLD,old_crate=str(oldc),new_crate=str(NEW),guard_sha256=digest(guard),marker_was_absent=was_absent,calls=2,argv=command,packages=packages,before=local_before,after=local_after,external_fingerprints_retained=sorted(set(before)-set(local_before)),new_marker=str(NEW),manual_fingerprint_deletion=False)
(proof/'cache-reprise.json').write_text(json.dumps(record,indent=2));print('CACHE_REPRISE_COMMUNE_EFFECTIVE',len(local_before),'->0',flush=True)
