"""Prepare an immutable full-lot controller; execute only on a distinct exact grant."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import time

sys.path.append(str(Path(__file__).resolve().parents[1] / '16-11-2026'))
from strict_target_guard import digest, snapshot, scan, require_source, require_hashes
from launch_full import publish_result
from harness_only import outer_watchdog_command

R = Path.home()/'.cache/rag3weaver-build'
W = Path.home()/'git_workspaces/cx-tty-english-aliases-union-99'
OLD = Path.home()/'git_workspaces/cx-tty-english-aliases'
T = R/'target-cx-tty-english-aliases-2'
Q = R/'cx-tty-union99-controller-v1'
P = R/'cx-tty-union99-stages-v1'
S = R/'cx-tty-union99-common-v1'
CANON = R/'cx-tty-union99-canonical-v1'
BASE = '99fef92ce6ee9a0007eaf99c7c3a6a3789061c57'
SDK = 'rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF'
BUDGET = 3600


def prepare(source):
    Q.mkdir(mode=0o700)
    assert subprocess.check_output(['git','-C',str(W),'rev-parse','origin/master'],text=True).strip() == BASE
    subprocess.run(['git','-C',str(W),'merge-base','--is-ancestor',BASE,source],check=True)
    required = subprocess.check_output(['bash','tools/poste/required_suites.sh',BASE,source],cwd=W,text=True)
    assert not required.strip()
    quiet = scan('/proc',[W,OLD,T]); (Q/'quiet-prepare.json').write_text(json.dumps(quiet,indent=2))
    assert not quiet['found']
    assert (T/'.arbre-du-target').read_text().strip() == str(W/'extension/rag3weaver/tty')
    ledger = snapshot(T)
    assert ledger == json.loads((R/'cx-tty-tab-green-v1/target-after.json').read_text())
    assert snapshot(T) == ledger
    (Q/'target-ledger.json').write_text(json.dumps(ledger,indent=2))
    previous = R/'cx-tty-tab-green-common-v1/validation-91mmtf2u'
    authority=json.loads((previous/'autorite.json').read_text()); verdict=json.loads((previous/'verdict.json').read_text())
    assert authority['etat']=='termine' and verdict['verdict']=='VERT' and verdict['code']==0
    assert verdict['erreur'] is None and verdict['restes']==[] and verdict['nettoye']
    for src,dest in [('autorite.json','previous-authority.json'),('verdict.json','previous-verdict.json')]:
        shutil.copy2(previous/src,Q/dest)
    green = json.loads((R/'cx-tty-tab-green-v1/test-result.json').read_text())
    assert green['verdict']=='VERT_CIBLE_ATTENDU' and green['compiled_from_new_worktree']
    assert digest(green['test_elf']) == green['test_elf_sha256']
    shutil.copy2(green['test_elf'],Q/'preserved-green-test-elf')
    old_elf=T/'debug/rag3weaver-tty'
    assert digest(old_elf)=='cb24ce0425bcb2487c83b9e0416a274cbf40585e65743d7327cef430108f031a'
    shutil.copy2(old_elf,Q/'historical-tty-elf')
    with tarfile.open(Q/'fingerprints-before.tar','w') as output:
        for path in sorted((T/'debug/.fingerprint').glob('rag3weaver*')):
            output.add(path,arcname=str(path.relative_to(T)))
    here=W/'docs/tty/17-11-2026'; old=W/'docs/tty/16-11-2026'
    sources={name:here/name for name in ['qualification.py','qualification_admit.py','qualification_driver.py',
        'qualification_contract.py','qualification_harness.py','qualification-contract-inactive.json']}
    sources.update({name:old/name for name in ['strict_target_guard.py','launch_full.py','harness_only.py','run_private_harness.py']})
    sources.update({name:W/'extension/rag3weaver/scripts'/name for name in ['poste','confiner_validation.py','cargo_garde.py','garde_target.sh']})
    sources.update({name:W/'extension/rag3weaver/tty/mesures'/name for name in ['mesure.py','harness_observation.py','faux_hote.py']})
    canonical=R/'passe_epinglee.sh'
    assert digest(canonical)==digest(W/'tools/poste/passe_epinglee.sh')=='2150fcd847b1df7de04908d357a0c78abc05e50ce599e2793ff08ab48108825d'
    sources['passe_epinglee.sh']=canonical
    controllers={}
    for name,path in sources.items():
        shutil.copy2(path,Q/name); controllers[str(Q/name)]=digest(path)
    external = [
        ('ec5e675cc2870e5987baee226e17ff43e4175b7d', 'docs/orchestration/17-11-2026/coordination/tty-provider-contract-preparation.md', 'provider-contract.md'),
        ('85c96a59bed8af4369f8b1c6be736c7955149a3a', 'docs/optimiseur/17-11-2026/01-tty-local-resource-contract.md', 'local-resource-contract.md'),
        ('85c96a59bed8af4369f8b1c6be736c7955149a3a', 'docs/optimiseur/17-11-2026/tty-local-resources.inactive.json', 'local-resources.inactive.json'),
        ('4f54eef8af693c17e5401d1adc9d4c9062b315d3', 'docs/orchestration/17-11-2026/coordination/grant_watch_actor.py', 'grant_watch_actor.py'),
        ('443b5a039cc2a3ff11705b0ebaa5bfa6577d6b0e', 'docs/recherche/16-11-2026/fts-generation-global-r2/global_services_owner.py', 'global_services_owner.py'),
        ('443b5a039cc2a3ff11705b0ebaa5bfa6577d6b0e', 'docs/orchestration/16-11-2026/coordination/provider_health_transport.py', 'provider_health_transport.py'),
    ]
    external_pins = {}
    for commit,path,name in external:
        raw = subprocess.check_output(['git','-C',str(W),'show',commit+':'+path])
        with (Q/name).open('xb') as out:
            out.write(raw)
        controllers[str(Q/name)] = digest(Q/name)
        external_pins[name] = dict(commit=commit,path=path,sha256=digest(Q/name))
    assert external_pins['provider-contract.md']['sha256']=='3f3342ae49695484b68cdc79619580f9c863e152a43e6c31a88915f27cec5fc3'
    require_hashes(controllers)
    sdkdir=Path.home()/'.cache/rag3db-libs'/SDK
    sdk={str(path):digest(path) for path in sorted(sdkdir.rglob('*')) if path.is_file()}
    assert sdk[str(sdkdir/'librag3db.so')]=='b7e9fc75a3d2de04c72680b0309e8bb1df9eaadb5211ebbacf39716c6d0a2101'
    assert sdk[str(sdkdir/'libvector.rag3db_extension')]=='1669d6392ec8067fcf92ae2ccc972fd7cd3086629fb82d1f5d254e90d72d7561'
    browser=shutil.which('google-chrome-stable'); assert browser
    react=R/'paquets-react'
    assert (react/'react-refresh/package.json').is_file()
    runtime={str(path):digest(path) for path in sorted(react.rglob('*')) if path.is_file()}
    runtime[browser]=digest(browser)
    for path in sorted(Path(browser).resolve().parent.rglob('*')):
        if path.is_file():
            runtime[str(path)] = digest(path)
    for path in [W/'tools/poste/required_suites.txt',W/'tools/poste/required_suites.sh',canonical]:
        runtime[str(path)]=digest(path)
    tmux=shutil.which('tmux'); assert tmux
    c=dict(source=source,base=BASE,worktree=str(W),old_worktree=str(OLD),target=str(T),proofs=str(P),scope=str(S),
        canonical_proofs=str(CANON),budget=BUDGET,host=os.uname().nodename,controllers=controllers,
        sdk=sdk,sdk_id=SDK,runtime=runtime,browser=browser,react=str(react),tmux={tmux:digest(tmux)},
        ledger_sha=digest(Q/'target-ledger.json'),historical_elf_sha=digest(Q/'historical-tty-elf'),
        preserved={str(Q/name):digest(Q/name) for name in ['preserved-green-test-elf','historical-tty-elf','fingerprints-before.tar']},
        timeout={'/usr/bin/timeout':digest('/usr/bin/timeout')},external_pins=external_pins,
        execution_ready=False, role='PREPARED INACTIVE; no service lease or execution allocation',
        historical='934 harness / old6d F8 / cb24 ELF are not qualification of union99')
    (Q/'prepared.json').write_text(json.dumps(c,indent=2))
    print('PREPARED_INACTIVE',Q,'SHA256',digest(Q/'prepared.json'))


def worker(args):
    if not args.prepare_only:
        assert args.allocation and args.started is not None and args.grant_sha
        assert (Path('/proc')/str(os.getppid())/'exe').resolve()==Path('/usr/bin/timeout').resolve()
        assert args.started<=time.monotonic()<args.started+BUDGET
    require_source(W,args.source)
    assert not any(path.exists() or path.is_symlink() for path in [P,S,CANON])
    if args.prepare_only:
        prepare(args.source); return
    c=json.loads((Q/'prepared.json').read_text())
    assert c['source']==args.source and c['host']==os.uname().nodename
    assert c['execution_ready'], 'HOLD before slot: common PC actor/TTY policy and cross-host admission not integrated'
    for field in ['controllers','sdk','runtime','tmux','timeout','preserved']:
        require_hashes(c[field])
    assert digest(Q/'target-ledger.json')==c['ledger_sha']
    assert digest(Q/'grants.json')==args.grant_sha
    end=args.started+BUDGET
    with (Q/'allocation.json').open('x') as out:
        json.dump(dict(reference=args.allocation,start=args.started,end=end,grant_sha256=args.grant_sha),out)
    os.chdir(Q)
    env=dict(os.environ,POSTE_MEM_MAX='32G',RAG3WEAVER_BUILD_MEMORY_HIGH='0',TTY_CONTROLLER_DIR=str(Q))
    result=dict(verdict='ROUGE',source=args.source,start=args.started,end=end); rc=2
    try:
        rc=subprocess.run([str(Q/'poste'),'lourd',sys.executable,'-B',str(Q/'qualification_admit.py')],env=env).returncode
        ref=json.loads((P/'authority-path.json').read_text())
        authority=json.loads(Path(ref['path']).read_text()); scope=json.loads((Path(ref['path']).parent/'verdict.json').read_text())
        assert rc==0 and authority['etat']=='termine' and scope['verdict']=='VERT' and scope['code']==0
        assert scope['erreur'] is None and scope['restes']==[] and scope['nettoye'] and not Path(scope['prive']).exists()
        assert snapshot(T)==json.loads((P/'target-after.json').read_text())
        for field in ['controllers','sdk','runtime','tmux','timeout','preserved']:
            require_hashes(c[field])
        require_source(W,args.source)
        quiet=scan('/proc',[W,OLD,T]);(P/'quiet-after-common.json').write_text(json.dumps(quiet,indent=2))
        assert not quiet['found'] and time.monotonic()<end
        assert json.loads((P/'driver-result.json').read_text())['verdict']=='VERT'
        result.update(verdict='VERT',scope=scope,release_required=True)
    except Exception as error:
        result['error']=repr(error)
    result['rc']=rc
    result=publish_result(Q/'composed-verdict.json',result,end)
    print(result['verdict'],Q/'composed-verdict.json')
    raise SystemExit(0 if result['verdict']=='VERT' else 1)


def main():
    p=argparse.ArgumentParser();p.add_argument('--source',required=True);p.add_argument('--prepare-only',action='store_true')
    p.add_argument('--allocation');p.add_argument('--grant-sha');p.add_argument('--worker',action='store_true',help=argparse.SUPPRESS)
    p.add_argument('--started',type=float,help=argparse.SUPPRESS)
    a=p.parse_args()
    if a.prepare_only or a.worker:
        return worker(a)
    assert a.allocation and a.grant_sha, 'distinct exact execution GO plus fresh owner leases required'
    started=time.monotonic()
    command=outer_watchdog_command(Q/'qualification.py',a.source,a.allocation,started,BUDGET,5)
    command.extend(['--grant-sha',a.grant_sha])
    os.execv(command[0],command)


if __name__=='__main__':
    main()
