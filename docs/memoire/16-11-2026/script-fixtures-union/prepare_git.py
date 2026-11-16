#!/usr/bin/env python3
"""Git-only owned-source transition; full cache inventory never invokes Cargo."""
from pathlib import Path
import json,os,subprocess,time
import source_proof as source
import cache_state

R=source.R;W=source.W;P=source.TRANSITION;ARCHIVE=source.ARCHIVE
T=R/'target-cx-memoire-script-fixtures';OLD=R/'cx-memoire-script-fixtures-qualification-v3'

def require(condition,message):
    if not condition:raise RuntimeError(message)

def save(name,value):
    with (P/name).open('x') as stream:json.dump(value,stream,indent=2);stream.write('\n')

def no_secret_tracked(root):
    entries=subprocess.check_output(['git','-C',str(root),'ls-files','-z']).split(b'\0')
    require(not any(set(Path(os.fsdecode(p)).parts)&{'.env','.vault'} for p in entries if p),'secret path must not be read or copied')
    for entry in subprocess.check_output(['git','-C',str(root),'ls-files','--stage','-z']).split(b'\0'):
        if entry and entry.split(b' ',1)[0]==b'160000':no_secret_tracked(root/os.fsdecode(entry.split(b'\t',1)[1]))

def live_owned(rows):
    alive=[];unknown=[]
    for row in rows:
        try:
            fields=(Path('/proc')/str(row['pid'])/'stat').read_text().rsplit(') ',1)[1].split()
            if fields[19]==row['debut'] and fields[0]!='Z':alive.append(row)
        except (FileNotFoundError,ProcessLookupError):continue
        except OSError as error:unknown.append(dict(pid=row['pid'],error=repr(error)))
    return dict(alive=alive,unknown=unknown)

def main():
    require(os.uname().nodename.lower()=='luciepc','PC only')
    require(not P.exists() and not ARCHIVE.exists(),'fresh transition/archive only')
    require(source.git(W,'rev-parse','HEAD')==source.OLD_APP and not source.git(W,'status','--porcelain'),'prior source must be clean')
    require(source.git(W,'merge-base',source.APP,'bef0b27cb33b9143dabee019fccdf7453da553a3')=='bef0b27cb33b9143dabee019fccdf7453da553a3','candidate not based on masterbef')
    scope=Path(json.loads((OLD/'authority-path.json').read_text())['path']).parent
    auth=json.loads((scope/'autorite.json').read_text());verdict=json.loads((scope/'verdict.json').read_text())
    require(auth['etat']=='termine' and verdict['verdict']=='VERT' and verdict['code']==0 and not verdict['restes'] and verdict['nettoye'] and not Path(verdict['prive']).exists(),'prior scope not terminated green')
    require(json.loads((OLD/'composed-result.json').read_text())['status']=='GREEN','prior composition not green')
    owned=live_owned(json.loads((scope/'enfants.json').read_text()));require(not owned['alive'] and not owned['unknown'],'prior owned processes remain/unknown')
    ledger=json.loads((OLD/'ledger-before-second-lib.json').read_text());require(ledger['source']==source.OLD_APP and source.sha(Path(ledger['binary']))==source.sha(OLD/'large-unit-tests')==ledger['sha256'],'prior real ELF changed')
    require((T/'.arbre-du-target').read_text().strip()==str(W/'extension/rag3weaver'),'wrong target owner')
    cache_state.require_root(T);cache_state.require_size(T);no_secret_tracked(W)
    P.mkdir(mode=0o700)
    before=cache_state.inventory(T,None);save('cache-before-git.json',before)
    tracked_before=source.tracked(W)
    old_ref=source.git(W,'rev-parse','refs/remotes/origin/cx-memoire-script-fixtures-r4')
    subprocess.run(['git','-C',str(W),'worktree','add','--detach',str(ARCHIVE),source.OLD_APP],check=True)
    subprocess.run(['git','-C',str(ARCHIVE),'submodule','update','--init','extension/rag3weaver/codeparsers'],check=True)
    no_secret_tracked(ARCHIVE);frozen=source.physical_archive(ARCHIVE)
    require(frozen['tracked']==tracked_before and frozen['gitdir']!=source.git(W,'rev-parse','--absolute-git-dir'),'physical source copy differs or shares admin')
    require(not {v['gitdir'] for v in source.gitlink_admins(W)}&{v['gitdir'] for v in frozen['gitlink_admins']},'gitlink admin reused')
    save('frozen-source.json',frozen)
    subprocess.run(['git','-C',str(W),'switch','--detach',source.APP],check=True)
    require(source.git(W,'rev-parse','HEAD')==source.APP and not source.git(W,'status','--porcelain'),'union source differs')
    no_secret_tracked(W);source.verify_archive(frozen)
    require(source.git(W,'rev-parse','refs/remotes/origin/cx-memoire-script-fixtures-r4')==old_ref,'old remote ref changed')
    require(source.sha(Path(ledger['binary']))==ledger['sha256'],'old ELF changed')
    after=cache_state.inventory(T,None);save('cache-after-git.json',after);require(after==before,'cache changed during Git-only transition')
    save('new-source.json',source.tracked(W))
    save('transition.json',dict(mode='GIT_ONLY_SAME_OWNER',source=source.APP,old_source=source.OLD_APP,owner=str(W/'extension/rag3weaver'),cache_identity=dict(device=T.lstat().st_dev,inode=T.lstat().st_ino,uid=T.lstat().st_uid),frozen_source_sha256=source.sha(P/'frozen-source.json'),new_source_sha256=source.sha(P/'new-source.json'),before_cache_sha256=source.sha(P/'cache-before-git.json'),after_cache_sha256=source.sha(P/'cache-after-git.json'),old_elf_sha256=ledger['sha256'],old_ledger_sha256=source.sha(OLD/'ledger-before-second-lib.json'),old_authority_sha256=source.sha(scope/'autorite.json'),old_remote_ref=old_ref,old_observed_owned=owned,cache_guard_invoked=False,cargo_invoked=False,completed=time.monotonic()))
    print(json.dumps(json.loads((P/'transition.json').read_text())))

if __name__=='__main__':main()
