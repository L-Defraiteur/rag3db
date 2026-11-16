#!/usr/bin/env python3
"""Authorized Git-only transition; never invokes Cargo or the cache guard."""
from pathlib import Path
import json, os, subprocess, time
import source_proof as source
import cache_state

R=source.R;W=source.W;ARCHIVE=source.ARCHIVE;P=source.TRANSITION
SESSION=R/'target-cx-archi-pas-va-au-bout-r2';T=SESSION/'native-doc-ea670-v1'
OLD=R/'cx-archi-doc-supervisor-empty-r1'

def main():
    assert os.uname().nodename.lower()=='luciepc'
    assert not P.exists() and not ARCHIVE.exists()
    assert source.git(W,'rev-parse','HEAD')==source.OLD_APP and not source.git(W,'status','--porcelain')
    scope=next((OLD/'supervision').glob('validation-*'))
    authority=json.loads((scope/'autorite.json').read_text());verdict=json.loads((scope/'verdict.json').read_text())
    assert json.loads((OLD/'result.json').read_text())['status']=='EXPECTED_RED'
    assert authority['etat']=='termine' and verdict['code']==101 and verdict['restes']==[] and verdict['nettoye']
    assert not Path(verdict['prive']).exists()
    alive=[]
    for row in json.loads((scope/'enfants.json').read_text()):
        try:
            fields=(Path('/proc')/str(row['pid'])/'stat').read_text().rsplit(') ',1)[1].split()
            if fields[19]==row['debut'] and fields[0]!='Z':alive.append(row)
        except (FileNotFoundError,ProcessLookupError):continue
    assert not alive
    ledger=json.loads((OLD/'ledger-before-witness.json').read_text())
    assert ledger['source']==source.OLD_APP and ledger['sha256']==source.sha(Path(ledger['binary']))==source.sha(OLD/'native-fixture-elf')
    identity=json.loads((OLD/'new-cache-identity.json').read_text());st=T.lstat()
    assert dict(device=st.st_dev,inode=st.st_ino,uid=st.st_uid,path=str(T))==identity
    assert (T/'.arbre-du-target').read_text().strip()==str(W/'extension/rag3weaver')
    frozen_cache=json.loads((OLD/'frozen-old-cache.json').read_text())
    cache_state.verify_frozen(SESSION,T,frozen_cache)
    P.mkdir(mode=0o700)
    child_before=cache_state.inventory(T,None)
    (P/'cache-before-git.json').write_text(json.dumps(child_before,indent=2)+'\n')
    before=source.tracked(W)
    old_ref=source.git(W,'rev-parse','refs/remotes/origin/cx-archi-doc-supervisor-r1')
    subprocess.run(['git','-C',str(W),'worktree','add','--detach',str(ARCHIVE),source.OLD_APP],check=True)
    subprocess.run(['git','-C',str(ARCHIVE),'submodule','update','--init','extension/rag3weaver/codeparsers'],check=True)
    frozen=source.physical_archive(ARCHIVE)
    assert frozen['gitdir']!=source.git(W,'rev-parse','--absolute-git-dir')
    assert frozen['tracked']==before
    original_admins={row['gitdir'] for row in source.gitlink_admins(W)}
    assert not original_admins.intersection(row['gitdir'] for row in frozen['gitlink_admins'])
    (P/'frozen-source.json').write_text(json.dumps(frozen,indent=2)+'\n')
    # Preserve the original local reference before changing this owned checkout.
    subprocess.run(['git','-C',str(W),'branch','cx-archi-doc-ea670-preserved-r1',source.OLD_APP],check=True)
    subprocess.run(['git','-C',str(W),'switch','--no-track','-c','cx-archi-doc-supervisor-fixed-pc-r1',source.APP],check=True)
    assert source.git(W,'rev-parse','HEAD')==source.APP and not source.git(W,'status','--porcelain')
    source.verify_archive(frozen)
    assert source.git(W,'rev-parse','refs/remotes/origin/cx-archi-doc-supervisor-r1')==old_ref
    assert source.sha(Path(ledger['binary']))==ledger['sha256']
    cache_state.verify_frozen(SESSION,T,frozen_cache)
    child_after=cache_state.inventory(T,None)
    (P/'cache-after-git.json').write_text(json.dumps(child_after,indent=2)+'\n');assert child_after==child_before
    (P/'new-source.json').write_text(json.dumps(source.tracked(W),indent=2)+'\n')
    record=dict(old_authority_sha256=source.sha(scope/'autorite.json'),cache_creation_record_sha256=source.sha(OLD/'new-cache-identity.json'),new_source_sha256=source.sha(P/'new-source.json'),mode='GIT_ONLY_SAME_OWNER',source=source.APP,old_source=source.OLD_APP,
        frozen_source_sha256=source.sha(P/'frozen-source.json'),old_ledger_sha256=source.sha(OLD/'ledger-before-witness.json'),
        old_elf_sha256=ledger['sha256'],cache_identity=identity,owner=str(W/'extension/rag3weaver'),
        completed=time.monotonic(),old_authority_terminated=True,old_observed_alive=alive,
        old_remote_ref=old_ref,cache_guard_invoked=False,cargo_invoked=False)
    (P/'transition.json').write_text(json.dumps(record,indent=2)+'\n')
    print(json.dumps(record))

if __name__=='__main__':main()
