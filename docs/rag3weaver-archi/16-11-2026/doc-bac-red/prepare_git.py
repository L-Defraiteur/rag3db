#!/usr/bin/env python3
"""Git-only preparation to be reviewed before execution; never invokes Cargo/guard."""
import json, os, subprocess, time
from pathlib import Path
import cache_state
import contracts as proof
import source_proof as source

def main():
    assert os.uname().nodename.lower()=='luciepc'
    assert not source.TRANSITION.exists() and not source.ARCHIVE.exists()
    assert source.git(source.W,'rev-parse','HEAD')==source.OLD_APP
    assert not source.git(source.W,'status','--porcelain')
    old=proof.archived_green();gone=proof.owned_gone(old['authority_path'])
    before=proof.cache_contract();tracked=source.tracked(source.W)
    refs={ref:source.git(source.W,'rev-parse',ref) for ref in [
        'refs/remotes/origin/cx-archi-doc-supervisor-fix-r1',
        'refs/remotes/origin/cx-archi-doc-supervisor-r1']}
    source.TRANSITION.mkdir(mode=0o700)
    proof.save(source.TRANSITION/'cache-before-git.json',before)
    subprocess.run(['git','-C',str(source.W),'worktree','add','--detach',str(source.ARCHIVE),source.OLD_APP],check=True)
    subprocess.run(['git','-C',str(source.ARCHIVE),'submodule','update','--init','extension/rag3weaver/codeparsers'],check=True)
    archive=source.physical_archive(source.ARCHIVE)
    assert archive['tracked']==tracked
    assert archive['gitdir']!=source.git(source.W,'rev-parse','--absolute-git-dir')
    assert not ({r['gitdir'] for r in source.gitlink_admins(source.W)} & {r['gitdir'] for r in archive['gitlink_admins']})
    proof.save(source.TRANSITION/'frozen-source.json',archive)
    subprocess.run(['git','-C',str(source.W),'branch','cx-archi-doc-1bd-preserved-r1',source.OLD_APP],check=True)
    subprocess.run(['git','-C',str(source.W),'switch','--no-track','-c','cx-archi-doc-bac-witness-pc-r1',source.APP],check=True)
    assert source.git(source.W,'rev-parse','HEAD')==source.APP and not source.git(source.W,'status','--porcelain')
    source.verify_archive(archive)
    assert all(source.git(source.W,'rev-parse',ref)==value for ref,value in refs.items())
    assert proof.archived_green()==old
    after=proof.cache_contract();proof.save(source.TRANSITION/'cache-after-git.json',after);assert after==before
    proof.save(source.TRANSITION/'new-source.json',source.tracked(source.W))
    proof.save(source.TRANSITION/'transition.json',dict(source=source.APP,old_source=source.OLD_APP,
        owner=str(proof.C),archive_sha256=proof.sha(source.TRANSITION/'frozen-source.json'),
        new_source_sha256=proof.sha(source.TRANSITION/'new-source.json'),
        old_proof=old,old_owned=gone,cache_identity=before['identity'],refs=refs,
        cargo_invoked=False,cache_guard_invoked=False,completed=time.monotonic()))

if __name__=='__main__':main()
