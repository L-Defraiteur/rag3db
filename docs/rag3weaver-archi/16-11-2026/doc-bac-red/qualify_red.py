#!/usr/bin/env python3
"""Proposed 1800-second, two-ELF, two-expected-red fixture qualification."""
from pathlib import Path
import hashlib, importlib.util, json, mmap, os, re, shutil, stat, subprocess, sys, time
import cache_state
import contracts as proof
import controller_policy as controller
import source_proof as source

R=source.R;W=source.W;C=W/'extension/rag3weaver';APP=source.APP
SESSION_TARGET=proof.SESSION;T=proof.T
P=R/'cx-archi-doc-bac-red-r1';Q=R/'cx-archi-doc-bac-red-controller-r1'
F=R/'bcr1'
OLD_W=Path.home()/'git_workspaces/rag3db-cx-archi-pas-va-au-bout-r2'
REL='docs/rag3weaver-archi/16-11-2026/doc-bac-red/qualify_red.py'
BUDGET=1800
FILES=['qualify_red.py','contracts.py','controller_policy.py','cache_state.py','source_proof.py','prepare_git.py']
LIB='rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF'
SDK={'librag3db.so':'b7e9fc75a3d2de04c72680b0309e8bb1df9eaadb5211ebbacf39716c6d0a2101',
     'libvector.rag3db_extension':'1669d6392ec8067fcf92ae2ccc972fd7cd3086629fb82d1f5d254e90d72d7561',
     'include/common/build_id.h':'1cf920608514b5f3c5e90952ecc580ca6170398f288dd554d00bb0bb32b03110'}
FEATURES=['code','daemon','rag3db-native']

def sha(path):return proof.sha(path)
def git(root,*args):return source.git(root,*args)
def save(name,value):proof.save(P/name,value)
def cfg():return json.loads((Q/'configuration.json').read_text())

def configure_controller():
    for name in ['R','W','C','T','OLD_W','P','Q','REL']:setattr(controller,name,globals()[name])

def check_transition():
    transition=json.loads((source.TRANSITION/'transition.json').read_text())
    assert transition['source']==APP and transition['old_source']==source.OLD_APP
    assert transition['owner']==str(C) and not transition['cargo_invoked'] and not transition['cache_guard_invoked']
    assert sha(source.TRANSITION/'frozen-source.json')==transition['archive_sha256']
    assert sha(source.TRANSITION/'new-source.json')==transition['new_source_sha256']
    frozen=json.loads((source.TRANSITION/'frozen-source.json').read_text())
    assert frozen['head']==source.OLD_APP
    source.verify_archive(frozen)
    return transition

def guard(config):
    for root,head in [(W,APP),(Path(config['control']),config['pilot'])]:
        assert git(root,'rev-parse','HEAD')==head and not git(root,'status','--porcelain')
        for name in ['rebase-merge','rebase-apply']:
            assert not Path(git(root,'rev-parse','--path-format=absolute','--git-path',name)).exists()
    assert git(C/'codeparsers','rev-parse','HEAD')==git(W,'ls-tree',APP,'extension/rag3weaver/codeparsers').split()[2]
    assert not git(C/'codeparsers','status','--porcelain')
    for name,digest in config['copies'].items():assert sha(Q/name)==digest
    for name,record in config['tools'].items():assert sha(Path(record['path']))==record['sha256'],name
    for name,digest in SDK.items():assert sha(R.parent/'rag3db-libs'/LIB/name)==digest
    assert sha(source.TRANSITION/'transition.json')==config['transition_sha256']
    check_transition()
    assert proof.archived_green(require_original=False)==config['old_green']
    assert sha(proof.RED/'frozen-old-cache.json')==config['cache']['frozen_old_sha256']
    cache_state.verify_frozen(SESSION_TARGET,T,json.loads((proof.RED/'frozen-old-cache.json').read_text()))
    cache_state.require_identity(T,config['cache']['identity'])

def launch_policy(config):
    launch=json.loads((Q/'launch.json').read_text())
    return controller.policy(Path(config['control']),config['pilot'],launch['identity'])

def fresh(config,old_cgroup,owned_cgroup=None):
    result=controller.scan(launch_policy(config),[T,W,Path(config['control']),source.ARCHIVE],
                           old_cgroup,owned_cgroup=owned_cgroup)
    assert not result['ancestor_errors'],result['ancestor_errors']
    assert not result['found'],result['found']
    assert not any(row.get('attributed') or row.get('reasons') for row in result['unknown']),result['unknown']
    assert len(result['controllers'])==3 and {r['role'] for r in result['controllers']}=={'launch','poste','watchdog'}
    return result

def unexpected(reason):
    row=dict(expected=False,reason=reason,observed=time.monotonic())
    for name in ['first-red.json','unexpected-red.json']:
        try:save(name,row)
        except FileExistsError:pass

def storage():
    sizes=[]
    for root in [SESSION_TARGET,F]:
        sizes.append(int(subprocess.check_output(['du','-sx','--block-size=1',str(root)],text=True).split()[0]))
    assert sum(sizes)<=32*1024**3,'session cache plus private runtime exceeds 32 GiB'
    return dict(session=sizes[0],runtime=sizes[1],combined=sum(sizes))

def compile_refusal(text):
    for line in text.splitlines():
        try:row=json.loads(line)
        except ValueError:continue
        if row.get('reason')=='compiler-message' and row.get('message',{}).get('level') in ['error','failure-note']:
            return row
    return None

def run(label,args,env,end,cap):
    limit=min(end-15,time.monotonic()+cap);assert time.monotonic()<limit
    save(label+'.command.json',dict(argv=args,deadline=limit,observed=time.monotonic()))
    with (P/(label+'.log')).open('x') as output:
        child=subprocess.Popen(args,cwd=C,env=env,stdout=output,stderr=subprocess.STDOUT)
        last_storage=0.
        while child.poll() is None:
            assert time.monotonic()<limit,label+' timeout'
            if label.startswith('compile-'):
                error=compile_refusal((P/(label+'.log')).read_text(errors='replace'))
                if error is not None:
                    save('compiler-first-refusal.json',dict(phase=label,diagnostic=error,observed=time.monotonic()))
                    raise RuntimeError('compiler error before any judge: '+label)
            if time.monotonic()-last_storage>=1:
                storage();last_storage=time.monotonic()
            time.sleep(.05)
        rc=child.wait()
    assert time.monotonic()<limit,label+' deadline after wait'
    text=(P/(label+'.log')).read_text(errors='replace')
    if label.startswith('compile-'):assert compile_refusal(text) is None,'compiler refusal at exit'
    save(label+'.result.json',dict(exit_code=rc,observed=time.monotonic()))
    return rc,text

def runtime(binary,args):
    return ['bash','-c','. "$1"; exporter_la_lib || exit 2; shift; exec "$@"','fixture-runtime',
            str(C/'scripts/lib_du_magasin.sh'),str(binary),*args]

def artifact(label,text):
    rows=[]
    for line in text.splitlines():
        try:row=json.loads(line)
        except ValueError:continue
        name='rag3weaver' if label=='lib' else 'e2e_equipier'
        if row.get('reason')=='compiler-artifact' and row.get('target',{}).get('name')==name and row.get('profile',{}).get('test') and row.get('executable'):
            rows.append(row)
    assert len(rows)==1 and sorted(rows[0]['features'])==FEATURES,'wrong test artifact or features'
    binary=Path(rows[0]['executable'])
    assert binary.resolve()==binary and binary.is_file() and binary.is_relative_to(T)
    copy=P/(label+'-elf');shutil.copy2(binary,copy)
    assert sha(binary)==sha(copy)
    with copy.open('rb') as stream,mmap.mmap(stream.fileno(),0,access=mmap.ACCESS_READ) as data:
        assert data[:4]==b'\x7fELF';offset=data.find(APP[:9].encode());assert offset>=0
    return dict(binary=str(binary),copy=str(copy),sha256=sha(copy),size=copy.stat().st_size,
                source_offset=offset,features=rows[0]['features'],cargo_artifact=rows[0])

def check_ledger(ledger):
    assert ledger['source']==APP and set(ledger['artifacts'])==set(proof.NAMES)
    for item in ledger['artifacts'].values():
        assert sha(Path(item['binary']))==sha(Path(item['copy']))==item['sha256']

def driver():
    config=cfg();guard(config)
    subprocess.run([sys.executable,str(Q/'confiner_validation.py'),'verifier'],check=True)
    receipt=Path(os.environ['RAG3WEAVER_VALIDATION_RECU']);auth=json.loads(receipt.read_text());end=auth['echeance']
    assert auth['etat']=='actif' and json.loads((receipt.parent/'recette.json').read_text())['delai']==BUDGET
    cg=(Path('/proc/self/cgroup').read_text().strip().split('::',1)[1]);assert cg==auth['cgroup']
    limits={k:(Path('/sys/fs/cgroup'+cg)/k).read_text().strip() for k in ['memory.max','memory.high','cpu.max']}
    assert limits['memory.max']=='34359738368' and limits['memory.high']=='max'
    quota,period=limits['cpu.max'].split();assert quota!='max' and 0<int(quota)<=8*int(period)
    save('authority-path.json',dict(path=str(receipt),initial=end-BUDGET,deadline=end,limits=limits))
    scan=fresh(config,config['old_green']['old_cgroup'],owned_cgroup=cg);save('cache-after-poste-scan.json',scan)
    proof.owned_gone(config['old_green']['authority_path'])
    assert proof.archived_green()==config['old_green']
    assert proof.cache_contract()==config['cache'],'cache changed after preparation'
    assert not F.exists() and not F.is_symlink()
    F.mkdir(mode=0o700);(F/'tmp').mkdir(mode=0o700)
    assert F.resolve()==F and F.stat().st_uid==os.getuid() and stat.S_IMODE(F.stat().st_mode)==0o700
    assert subprocess.check_output(['findmnt','-n','-o','FSTYPE','-T',str(F)],text=True).strip() not in ['tmpfs','ramfs','']
    assert len(os.fsencode(F/'tmp/documentation-fixture-123456/pont-12345678/ecoute/pont.sock'))<=107
    for name in ['config','data','cache','xdg-state','state-fixture','daemon','journals']:(P/name).mkdir(mode=0o700)
    env=proof.private_environment(os.environ,P,F/'tmp',os.environ['POSTE_FILS'])
    env.update(RAG3DB_LIB=LIB,RAG3DB_LIBS_DIR=str(R.parent/'rag3db-libs'),RAG3DB_BUFFER_POOL_SIZE='1073741824')
    save('selected-environment.json',{k:v for k,v in env.items() if k!='RAG3WEAVER_VALIDATION_JETON'})
    before=cache_state.inventory(T,None);save('same-owner-before.json',before)
    rc,submodules=run('preflight',['bash','-c','. "$1"; if build_rs_d_un_autre_arbre "$2" "$3"; then exit 42; fi; sous_modules_de "$3"',
        'same-owner',str(C/'scripts/garde_target.sh'),str(T),str(C)],env,end,30)
    assert rc==0 and (T/'.sous-modules-du-target').read_text().strip()==submodules.strip()
    rc,text=run('same-owner-guard',['bash','-c','. "$1"; garder_le_target "$2" "$3"','same-owner',
        str(C/'scripts/garde_target.sh'),str(T),str(C)],env,end,30)
    after=cache_state.inventory(T,None);save('same-owner-after.json',after)
    assert rc==0 and not text.strip() and before==after,'guard mutated or invalidated known target'
    artifacts={}
    for label in ['lib','e2e']:
        guard(config)
        selection=['--lib'] if label=='lib' else ['--test','e2e_equipier']
        command=['bash','scripts/cargo_magasin.sh','test',*selection,'--no-default-features',
                 '--features',','.join(FEATURES),'--locked','--offline','--no-run','--message-format=json-render-diagnostics']
        rc,text=run('compile-'+label,command,env,end,600)
        assert rc==0,'compilation failed before a judge'
        artifacts[label]=artifact(label,text)
    # Both builds finish BEFORE either witness, so the ledger covers both final ELFs.
    for label,item in artifacts.items():
        for kind,options in [('inventory',[]),('ignored',['--ignored'])]:
            rc,text=run(kind+'-'+label,runtime(Path(item['binary']),['--list','--format','terse',*options]),env,end,15)
            assert rc==0
            found=bool(re.search(r'^'+re.escape(proof.NAMES[label])+r': test$',text,re.M))
            assert found==(kind=='inventory'),'witness absent or ignored'
    ledger=dict(source=APP,pilot=config['pilot'],artifacts=artifacts,names=proof.NAMES,
                created=time.monotonic(),distinct_judges=2,planned_raw_test_invocations=4,
                expected='two explicit missing private sandbox connections; not a green qualification')
    check_ledger(ledger);save('ledger-before-witnesses.json',ledger)
    pause_end=min(end-30,time.monotonic()+120)
    while not (P/'ledger-published.json').exists():
        assert time.monotonic()<pause_end,'ledger not replicated/published in bounded pause'
        time.sleep(.1)
    publication=json.loads((P/'ledger-published.json').read_text())
    hashes={k:v['sha256'] for k,v in artifacts.items()}
    assert publication['source']==APP and publication['pilot']==config['pilot']
    assert publication['ledger_sha256']==sha(P/'ledger-before-witnesses.json')
    assert publication['elf_sha256']==hashes and publication['copies']==dict(pc=hashes,local=hashes)
    assert ledger['created']<=publication['observed']<=time.monotonic()<pause_end
    guard(config);check_ledger(ledger)
    save('runtime-permitted.json',dict(observed=time.monotonic(),ledger_sha256=sha(P/'ledger-before-witnesses.json')))
    observations=[]
    for label in ['lib','e2e']:
        guard(config);check_ledger(ledger)
        name=proof.NAMES[label]
        rc,text=run('expected-red-'+label,runtime(Path(artifacts[label]['binary']),
                    [name,'--exact','--nocapture','--test-threads=1']),env,end,45)
        result=proof.expected_red(name,rc,text)
        fixture=Path(result['fixture']);assert fixture.resolve()==fixture and fixture.is_relative_to(F/'tmp')
        assert (fixture/'setup-child.stdout').is_file() and (fixture/'setup-child.stderr').is_file()
        assert proof.PARENT_MARKER in json.loads((fixture/'fixture-failure.json').read_text())['reason']
        result.update(observed=time.monotonic(),label=label);observations.append(result)
        save('expected-red-'+label+'.json',result)
        try:save('first-red.json',dict(expected=True,observation=result))
        except FileExistsError:pass
    guard(config);check_ledger(ledger)
    save('driver-result.json',dict(status='EXPECTED_SETUP_REDS_OBSERVED',source=APP,pilot=config['pilot'],
         deadline=end,observations=observations,distinct_judges=2,raw_test_invocations=4,
         production_fixed=False,not_a_full_qualification=True,finished=time.monotonic(),storage=storage()))
    return 101

def result_file(value):
    with (P/'result.json').open('x') as stream:
        if value['status']=='EXPECTED_SETUP_REDS_OBSERVED' and time.monotonic()>=value['deadline']:
            value=dict(value,status='UNEXPECTED_FAILURE',error='deadline after result open')
        json.dump(value,stream,indent=2);stream.write('\n');stream.flush();os.fsync(stream.fileno())
    return value

def compose(pilot):
    config=cfg();assert config['pilot']==pilot
    spec=importlib.util.spec_from_file_location('common',Q/'confiner_validation.py')
    common=importlib.util.module_from_spec(spec);spec.loader.exec_module(common)
    assert common.poste_tenu(common.processus())
    outcome=dict(status='UNEXPECTED_FAILURE',not_a_full_qualification=True)
    try:
        guard(config)
        # The outer timeout wraps poste, not its immediate child: verify the real ancestry.
        scan=fresh(config,config['old_green']['old_cgroup']);save('before-scope-scan.json',scan)
        command=[sys.executable,str(Q/'confiner_validation.py'),'lancer','--delai',str(BUDGET),
                 '--preuves',str(P/'supervision'),'--',sys.executable,str(Q/'qualify_red.py'),'--driver',pilot]
        code=subprocess.run(command,cwd='/var/tmp').returncode
        receipt=Path(json.loads((P/'authority-path.json').read_text())['path'])
        auth=json.loads(receipt.read_text());scope=json.loads((receipt.parent/'verdict.json').read_text())
        assert code==1 and auth['etat']=='termine' and scope['verdict']=='ROUGE' and scope['code']==101
        assert not scope['erreur'] and not scope['restes'] and scope['nettoye'] and not Path(scope['prive']).exists()
        result=json.loads((P/'driver-result.json').read_text());assert result['status']=='EXPECTED_SETUP_REDS_OBSERVED'
        assert result['distinct_judges']==2 and result['raw_test_invocations']==4 and not (P/'unexpected-red.json').exists()
        assert json.loads((P/'first-red.json').read_text())['expected'] is True
        gone=proof.owned_gone(receipt);scan=fresh(config,auth['cgroup']);save('quiescence-final.json',dict(scan=scan,owned=gone))
        guard(config);check_ledger(json.loads((P/'ledger-before-witnesses.json').read_text()))
        assert time.monotonic()<auth['echeance']
        outcome.update(status='EXPECTED_SETUP_REDS_OBSERVED',scope=scope,deadline=auth['echeance'],
            elapsed=time.monotonic()-(auth['echeance']-BUDGET),authority_terminated=True,bus_absent=True,
            source=APP,pilot=pilot,distinct_judges=2,raw_test_invocations=4,production_fixed=False)
    except Exception as error:
        unexpected(repr(error));outcome['error']=repr(error)
    outcome=result_file(outcome);print(json.dumps(outcome));return 1

def prepare(pilot):
    assert os.uname().nodename.lower()=='luciepc' and re.fullmatch('[0-9a-f]{40}',pilot)
    control=Path(__file__).resolve().parents[4]
    assert git(control,'rev-parse','HEAD')==pilot and not git(control,'status','--porcelain')
    assert git(W,'rev-parse','HEAD')==APP and not git(W,'status','--porcelain')
    assert not F.exists() and not F.is_symlink()
    for path in [P,Q]:
        assert not path.is_symlink() and (not path.exists() or (path.resolve()==path and path.stat().st_uid==os.getuid()))
    if P.exists():assert not list(P.iterdir()),'proof root must remain empty before launch'
    old=proof.archived_green();proof.owned_gone(old['authority_path']);cache=proof.cache_contract()
    transition=check_transition();assert transition['old_proof']==old
    P.mkdir(mode=0o700,exist_ok=True);Q.mkdir(mode=0o700,exist_ok=True)
    copies={}
    for name in [*FILES,'confiner_validation.py']:
        origin=Path(__file__).with_name(name) if name in FILES else C/'scripts/confiner_validation.py'
        dest=Q/name
        if dest.exists():assert not dest.is_symlink() and sha(dest)==sha(origin)
        else:shutil.copy2(origin,dest)
        copies[name]=sha(dest)
    tools={}
    for name in ['python3','bash','timeout','cargo']:
        file=Path(shutil.which(name)).resolve();tools[name]=dict(path=str(file),sha256=sha(file))
    config=dict(app=APP,pilot=pilot,control=str(control),copies=copies,tools=tools,
                transition_sha256=sha(source.TRANSITION/'transition.json'),old_green=old,cache=cache)
    if (Q/'configuration.json').exists():assert cfg()==config
    else:proof.save(Q/'configuration.json',config)
    guard(config)
    print(json.dumps(dict(prepared=True,source=APP,pilot=pilot,cargo=False,authority=False,proof_root_empty=True)))

def launch(pilot):
    assert os.getcwd()=='/var/tmp' and 'RAG3WEAVER_VALIDATION_RECU' not in os.environ
    control=Path(__file__).resolve().parents[4]
    assert sys.orig_argv==['python3',str(control/REL),'--launch',pilot]
    config=cfg();assert config['pilot']==pilot;guard(config)
    allocation=os.environ['RAG3WEAVER_C7_ALLOCATION'];assert allocation.strip()
    assert not list(P.iterdir()) and not (Q/'launch.json').exists()
    assert proof.archived_green()==config['old_green'] and proof.cache_contract()==config['cache']
    identity=controller.snapshot(Path('/proc')/str(os.getpid()));assert identity['stable']
    proof.save(Q/'launch.json',dict(identity=identity,allocation=allocation,expected_reds=list(proof.NAMES.values())))
    runtime_root=Path('/run/user')/str(os.getuid());bus=runtime_root/'bus'
    assert runtime_root.resolve()==runtime_root and bus.resolve()==bus and bus.is_socket() and bus.stat().st_uid==os.getuid()
    env={k:os.environ[k] for k in ['HOME','USER','LOGNAME','PATH','LANG','LC_ALL','CARGO_HOME','RUSTUP_HOME'] if k in os.environ}
    env.update(POSTE_MEM_MAX='32G',RAG3WEAVER_BUILD_MEMORY_HIGH='0',DBUS_SESSION_BUS_ADDRESS='unix:path='+str(bus),XDG_RUNTIME_DIR=str(runtime_root))
    command=controller.commands(control,pilot)['outer']
    with (P/'supervision.log').open('x') as output:
        return subprocess.run(command,cwd='/var/tmp',env=env,stdout=output,stderr=subprocess.STDOUT).returncode

if __name__=='__main__':
    mode,pilot=sys.argv[1:];configure_controller()
    if mode=='--prepare':prepare(pilot)
    elif mode=='--launch':sys.exit(launch(pilot))
    elif mode=='--admit':sys.exit(compose(pilot))
    elif mode=='--driver':
        try:sys.exit(driver())
        except Exception as error:
            unexpected(repr(error));raise
    else:raise RuntimeError('explicit preparation or exact authorized mode required')
