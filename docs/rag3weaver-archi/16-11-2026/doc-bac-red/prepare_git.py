#!/usr/bin/env python3
"""Git-only preparation to be reviewed before execution; never invokes Cargo/guard."""
import json, os, subprocess, time, sys
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

def reprise():
    assert os.uname().nodename.lower()=='luciepc'
    assert not proof.REPRISE.exists() and not proof.ARCHIVE049.exists()
    assert source.git(source.W,'rev-parse','HEAD')==proof.PREVIOUS_APP and not source.git(source.W,'status','--porcelain')
    old=proof.previous_refusal();gone=proof.owned_gone(old['authority_path'])
    before=proof.cache_contract();tracked=source.tracked(source.W)
    proof.REPRISE.mkdir(mode=0o700);proof.save(proof.REPRISE/'cache-before-git.json',before)
    subprocess.run(['git','-C',str(source.W),'worktree','add','--detach',str(proof.ARCHIVE049),proof.PREVIOUS_APP],check=True)
    subprocess.run(['git','-C',str(proof.ARCHIVE049),'submodule','update','--init','extension/rag3weaver/codeparsers'],check=True)
    archive=source.physical_archive(proof.ARCHIVE049,proof.PREVIOUS_APP);assert archive['tracked']==tracked
    assert archive['gitdir']!=source.git(source.W,'rev-parse','--absolute-git-dir')
    assert not ({v['gitdir'] for v in archive['gitlink_admins']} & {v['gitdir'] for v in source.gitlink_admins(source.W)})
    proof.save(proof.REPRISE/'frozen-source.json',archive)
    subprocess.run(['git','-C',str(source.W),'merge','--ff-only',source.APP],check=True)
    assert source.git(source.W,'rev-parse','HEAD')==source.APP and not source.git(source.W,'status','--porcelain')
    source.verify_archive(archive);assert proof.previous_refusal()==old
    after=proof.cache_contract();proof.save(proof.REPRISE/'cache-after-git.json',after);assert before==after
    proof.save(proof.REPRISE/'new-source.json',source.tracked(source.W))
    proof.save(proof.REPRISE/'transition.json',dict(source=source.APP,previous_source=proof.PREVIOUS_APP,
        previous_refusal=old,old_owned=gone,source_archive_sha256=proof.sha(proof.REPRISE/'frozen-source.json'),
        new_source_sha256=proof.sha(proof.REPRISE/'new-source.json'),cache_identity=before['identity'],
        cargo_invoked=False,cache_guard_invoked=False,completed=time.monotonic()))

if __name__=='__main__':
    if sys.argv[1:]==['--reprise']:reprise()
    elif not sys.argv[1:]:main()
    else:raise RuntimeError('explicit known Git preparation only')
