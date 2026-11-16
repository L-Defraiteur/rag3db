#!/usr/bin/env python3
"""Git-only preparation to be reviewed before execution; never invokes Cargo/guard."""
import json, os, subprocess, time, sys
from pathlib import Path
import cache_state
import contracts as proof
import source_proof as source

def main():
    assert os.uname().nodename.lower()=='luciepc'
    assert not proof.REPRISE.exists() and not proof.ARCHIVE223.exists()
    assert source.git(source.W,'rev-parse','HEAD')==proof.PREVIOUS_APP and not source.git(source.W,'status','--porcelain')
    old=proof.previous_expected_red();gone=proof.owned_gone(old['authority_path'])
    before=proof.cache_contract();tracked=source.tracked(source.W)
    proof.REPRISE.mkdir(mode=0o700);proof.save(proof.REPRISE/'cache-before-git.json',before)
    subprocess.run(['git','-C',str(source.W),'worktree','add','--detach',str(proof.ARCHIVE223),proof.PREVIOUS_APP],check=True)
    subprocess.run(['git','-C',str(proof.ARCHIVE223),'submodule','update','--init','extension/rag3weaver/codeparsers'],check=True)
    archive=source.physical_archive(proof.ARCHIVE223,proof.PREVIOUS_APP);assert archive['tracked']==tracked
    assert archive['gitdir']!=source.git(source.W,'rev-parse','--absolute-git-dir')
    assert not ({v['gitdir'] for v in archive['gitlink_admins']} & {v['gitdir'] for v in source.gitlink_admins(source.W)})
    proof.save(proof.REPRISE/'frozen-source.json',archive)
    subprocess.run(['git','-C',str(source.W),'merge','--ff-only',source.APP],check=True)
    assert source.git(source.W,'rev-parse','HEAD')==source.APP and not source.git(source.W,'status','--porcelain')
    source.verify_archive(archive);assert proof.previous_expected_red()==old
    after=proof.cache_contract();proof.save(proof.REPRISE/'cache-after-git.json',after);assert before==after
    proof.save(proof.REPRISE/'new-source.json',source.tracked(source.W))
    proof.save(proof.REPRISE/'transition.json',dict(source=source.APP,previous_source=proof.PREVIOUS_APP,
        previous_expected_red=old,old_owned=gone,source_archive_sha256=proof.sha(proof.REPRISE/'frozen-source.json'),
        new_source_sha256=proof.sha(proof.REPRISE/'new-source.json'),cache_identity=before['identity'],
        cargo_invoked=False,cache_guard_invoked=False,completed=time.monotonic()))

if __name__=='__main__':
    assert not sys.argv[1:]
    main()
