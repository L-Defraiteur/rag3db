#!/usr/bin/env python3
"""CPU fixture of the real common guard; Cargo metadata is a shell stub only.

No Cargo/rustc executable, real target, source worktree or SDK is invoked/mutated.
This tests guard branching, not compiled code freshness or real metadata output.
"""
from pathlib import Path
import ast,json,os,subprocess,sys,tempfile
proof=Path(sys.argv[1]);proof.mkdir(exist_ok=False)
base=Path(__file__).parent;guard=Path(sys.argv[2]).resolve(strict=True)
a=ast.parse((base/'reprendre-cache.py').read_text());bridge=next(ast.literal_eval(n.value) for n in ast.walk(a) if isinstance(n,ast.Assign) and any(isinstance(t,ast.Name) and t.id=='script' for t in n.targets))
stub='''cargo() {
 [ "$*" = "metadata --format-version 1 --all-features" ] || return 97
 printf '%s\\n' "$*" >> "$FIXTURE_METADATA_CALLS"
 printf '%s\\n' '{"packages":[{"name":"rag3weaver","source":null},{"name":"codeparsers","source":null},{"name":"external","source":"registry"}]}'
}
'''
with tempfile.TemporaryDirectory(prefix='cx-fixture-cache-',dir='/var/tmp') as tmp:
 root=Path(tmp);old=root/'old';new=root/'new';old.mkdir();new.mkdir();calls=root/'metadata-calls';env=os.environ.copy();env['FIXTURE_METADATA_CALLS']=str(calls);env.pop('CARGO_TARGET_DIR',None);env['RAG3WEAVER_TARGET_REPRENDRE']='1'
 local=['debug/.fingerprint/rag3weaver-aaaaaaaaaaaaaaaa','triplet/debug/.fingerprint/codeparsers-bbbbbbbbbbbbbbbb'];external='debug/.fingerprint/external-cccccccccccccccc'
 def target(name):
  t=root/name
  for path in [*local,external]:(t/path).mkdir(parents=True)
  return t
 bad=target('bad');r=subprocess.run(['bash','-c',stub+'\nset -euo pipefail\n. "$1"\ngarder_le_target "$2" "$3"','fixture-direct',str(guard),str(bad),str(new)],env=env,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
 assert r.returncode==0 and all((bad/p).exists() for p in local) and not calls.exists()
 (proof/'contre-exemple.log').write_text(r.stdout+'NO_MARKER_DIRECT_ADOPTION_LOCAL_FINGERPRINTS_REMAIN\n')
 good=target('good');r=subprocess.run(['bash','-c',stub+'\n'+bridge,'fixture-bridge',str(guard),str(good),str(old),str(new)],env=env,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
 assert r.returncode==0 and all(not (good/p).exists() for p in local) and (good/external).is_dir()
 assert (good/'.arbre-du-target').read_text().strip()==str(new)
 assert calls.read_text().splitlines()==['metadata --format-version 1 --all-features']
 assert 'target repris de '+str(old) in r.stdout
 (proof/'reprise.log').write_text(r.stdout)
 record=dict(verdict='VERT_FIXTURE_GARDE',real_guard=str(guard),direct_adoption_leaves_local_fingerprints=True,two_real_guard_calls_invalidate_local=True,local_before=2,local_after=0,external_retained=True,metadata_stub_calls=1,real_cargo_or_rustc_calls=0,real_target_mutated=False,scope='guard branch fixture only, no Rust qualification')
 (proof/'verdict.json').write_text(json.dumps(record,indent=2));print(json.dumps(record))
