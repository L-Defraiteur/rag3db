"""Prepare a distinct green coupon after the real collector red; never qualify the full lot."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import time
from strict_target_guard import digest, snapshot, scan, require_source, require_hashes
from launch_full import publish_result
from harness_only import outer_watchdog_command

R=Path.home()/'.cache/rag3weaver-build'
W=Path.home()/'git_workspaces/cx-tty-english-aliases-union-99'
OLD_W=Path.home()/'git_workspaces/cx-tty-english-aliases'
T=R/'target-cx-tty-english-aliases-2'
Q=R/'cx-tty-tab-green-controller-v1'
P=R/'cx-tty-tab-green-v1'
S=R/'cx-tty-tab-green-common-v1'
BASE='99fef92ce6ee9a0007eaf99c7c3a6a3789061c57'
FILTER='tui::tests::partial_english_and_legacy_commands_collect_catalogue_before_completion'
BUDGET=600


def audit():
    subprocess.run(['git','-C',str(W),'merge-base','--is-ancestor',BASE,'HEAD'],check=True)
    names=subprocess.check_output(['git','-C',str(W),'diff','--name-only',BASE,'HEAD','--','extension/rag3weaver'],text=True).splitlines()
    allowed={'extension/rag3weaver/tty/'+name for name in ['README.md','src/main.rs','src/commandes.rs','src/tui.rs',
             'mesures/mesure.py','mesures/harness_observation.py','mesures/test_harness_observation.py']}
    assert set(names)<=allowed
    return names


def worker(args):
    if not args.prepare_only:
        assert args.allocation and args.started is not None
        assert (Path('/proc')/str(os.getppid())/'exe').resolve()==Path('/usr/bin/timeout').resolve()
        assert args.started<=time.monotonic()<args.started+BUDGET
    require_source(W,args.source)
    changed=audit()
    assert not any(p.exists() or p.is_symlink() for p in [P,S])
    if args.prepare_only:
        Q.mkdir(mode=0o700)
        old=R/'cx-tty-tab-red-common-v2/validation-p8xp1kn3'
        authority=json.loads((old/'autorite.json').read_text());verdict=json.loads((old/'verdict.json').read_text())
        assert authority['etat']=='termine' and verdict['verdict']=='ROUGE' and verdict['code']==1 and verdict['erreur'] is None and verdict['restes']==[] and verdict['nettoye']
        assert (T/'.arbre-du-target').read_text().strip()==str(W/'extension/rag3weaver/tty')
        quiet=scan('/proc',[W,OLD_W,T]);(Q/'quiet-prepare.json').write_text(json.dumps(quiet,indent=2));assert quiet['found']==[]
        red_stage=json.loads((R/'cx-tty-tab-red-v2/test-result.json').read_text())
        assert red_stage['verdict']=='ROUGE_ASSERTION_ATTENDUE' and red_stage['compiled_from_new_worktree']
        red_elf=Path(red_stage['test_elf']);assert digest(red_elf)==red_stage['test_elf_sha256']
        shutil.copy2(red_elf,Q/'red-test-elf')
        assert snapshot(T)==json.loads((R/'cx-tty-tab-red-v2/target-after.json').read_text())
        ledger=snapshot(T);assert snapshot(T)==ledger
        (Q/'target-ledger.json').write_text(json.dumps(ledger,indent=2))
        old_elf=T/'debug/rag3weaver-tty';old_sha=digest(old_elf)
        assert old_sha=='cb24ce0425bcb2487c83b9e0416a274cbf40585e65743d7327cef430108f031a'
        shutil.copy2(old_elf,Q/'historical-tty-elf')
        with tarfile.open(Q/'old-local-fingerprints.tar','w') as output:
            for path in sorted((T/'debug/.fingerprint').glob('rag3weaver-tty-*')):
                output.add(path,arcname=str(path.relative_to(T)),recursive=True)
        for name in ['autorite.json','verdict.json']:
            shutil.copy2(old/name,Q/('previous-'+name))
        folder=W/'docs/tty/16-11-2026'
        sources={name:folder/name for name in ['tab_green.py','tab_green_admit.py','tab_green_driver.py','strict_target_guard.py',
                  'launch_full.py','harness_only.py']}
        sources.update({name:W/'extension/rag3weaver/scripts'/name for name in ['poste','confiner_validation.py','cargo_garde.py','garde_target.sh']})
        hashes={}
        for name,path in sources.items():
            shutil.copy2(path,Q/name);hashes[str(Q/name)]=digest(path);assert digest(Q/name)==hashes[str(Q/name)]
        config=dict(source=args.source,base=BASE,worktree=str(W),old_worktree=str(OLD_W),target=str(T),proofs=str(P),scope=str(S),
                    host=os.uname().nodename,budget=BUDGET,filter=FILTER,changed=changed,controllers=hashes,
                    ledger_sha=digest(Q/'target-ledger.json'),old_elf_sha=old_sha,old_fingerprints_sha=digest(Q/'old-local-fingerprints.tar'),
                    timeout={'/usr/bin/timeout':digest('/usr/bin/timeout')},historical_f8='6d 8/8 historical only, NOT union99 qualification',
                    red_source='bc8ab075813cb882c9876dffbc2f3bb8128da560',red_test_elf_sha=red_stage['test_elf_sha256'],role='ONE GREEN COLLECTOR TEST ONLY; not full union99 qualification; no HTTP server/provider/SDK/7970')
        (Q/'prepared.json').write_text(json.dumps(config,indent=2));print('PREPARED_ONLY',Q);return
    c=json.loads((Q/'prepared.json').read_text());assert c['source']==args.source and c['host']==os.uname().nodename
    require_hashes(c['controllers']);require_hashes(c['timeout']);assert digest(Q/'target-ledger.json')==c['ledger_sha']
    end=args.started+BUDGET
    with (Q/'allocation.json').open('x') as out:json.dump(dict(reference=args.allocation,start=args.started,end=end),out)
    env=dict(os.environ,POSTE_MEM_MAX='32G',RAG3WEAVER_BUILD_MEMORY_HIGH='0',TTY_CONTROLLER_DIR=str(Q))
    os.chdir(Q);result=dict(verdict='ROUGE_NON_QUALIFIE',source=args.source,start=args.started,end=end);rc=2
    try:
        rc=subprocess.run([str(Q/'poste'),'lourd',sys.executable,str(Q/'tab_green_admit.py')],env=env).returncode
        ref=json.loads((P/'authority-path.json').read_text());authority=json.loads(Path(ref['path']).read_text())
        scope=json.loads((Path(ref['path']).parent/'verdict.json').read_text())
        assert rc==0 and authority['etat']=='termine' and scope['verdict']=='VERT' and scope['code']==0 and scope['erreur'] is None
        assert scope['restes']==[] and scope['nettoye'] and not Path(scope['prive']).exists()
        stage=json.loads((P/'test-result.json').read_text());assert stage['verdict']=='VERT_CIBLE_ATTENDU'
        assert snapshot(T)==json.loads((P/'target-after.json').read_text())
        require_hashes(c['controllers']);require_source(W,args.source)
        quiet=scan('/proc',[W,OLD_W,T]);(P/'quiet-after-common.json').write_text(json.dumps(quiet,indent=2));assert quiet['found']==[]
        assert time.monotonic()<end
        result.update(verdict='VERT_CIBLE',scope=scope,limited_to=FILTER)
    except Exception as error:result['error']=repr(error)
    result['rc']=rc;result=publish_result(Q/'composed-verdict.json',result,end)
    print(result['verdict'],Q/'composed-verdict.json');raise SystemExit(0 if result['verdict']=='VERT_CIBLE' else 1)


def main():
    p=argparse.ArgumentParser();p.add_argument('--source',required=True);p.add_argument('--prepare-only',action='store_true')
    p.add_argument('--allocation');p.add_argument('--worker',action='store_true',help=argparse.SUPPRESS);p.add_argument('--started',type=float,help=argparse.SUPPRESS)
    a=p.parse_args()
    if a.prepare_only or a.worker:return worker(a)
    assert a.allocation,'fresh exact C7 allocation required'
    started=time.monotonic();command=outer_watchdog_command(Q/'tab_green.py',a.source,a.allocation,started,BUDGET,5)
    os.execv(command[0],command)


if __name__=='__main__':main()
