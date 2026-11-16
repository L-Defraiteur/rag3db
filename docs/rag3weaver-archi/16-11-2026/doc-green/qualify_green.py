#!/usr/bin/env python3
"""Five policy witnesses and one native completion; no Vertex or command grant."""
from pathlib import Path
import hashlib, json, mmap, os, re, shutil, signal, stat, subprocess, sys, time

R = Path.home() / '.cache/rag3weaver-build'
W = Path.home() / 'git_workspaces/rag3db-cx-archi-doc-supervisor-r1'
C = W / 'extension/rag3weaver'
SESSION_TARGET = R / 'target-cx-archi-pas-va-au-bout-r2'
T = SESSION_TARGET / 'native-doc-ea670-v1'
P = R / 'cx-archi-doc-supervisor-green-r1'
Q = R / 'cx-archi-doc-supervisor-controller-green-r1'
OLD = R / 'cx-archi-pas-va-au-bout-r2-59ac-pass1'
OLD_W = Path.home() / 'git_workspaces/rag3db-cx-archi-pas-va-au-bout-r2'
APP = '1bd32dcd9b0cba098f64728971091c5c7b55c19e'
RED = R/'cx-archi-doc-supervisor-empty-r1'
TRANSITION = R/'cx-doc-green-transition-r1'
OLD_APP = '713433bd602a6d35266fe5cd528925ba664924c9'
REL = 'docs/rag3weaver-archi/16-11-2026/doc-green/qualify_green.py'
ORIGINAL_END = 1177548.693791571
import controller_policy as controller
import cache_state as empty_cache
import source_proof
for key in ['R','W','C','T','OLD_W','P','Q','REL']:
    setattr(controller, key, globals()[key])
NAME = 'daemon_host::teammate_tests::a_documentation_supervisor_denies_lock_deletion_and_the_teammate_finishes_the_original_request'
POLICY_PREFIX='daemon_host::teammate_tests::documentation_fixture::tests::'
POLICY=[POLICY_PREFIX+name for name in [
    'only_the_exact_lock_deletion_gets_a_linked_refusal',
    'other_commands_and_shell_variants_stop_the_fixture',
    'unknown_questions_and_missing_ids_stop_the_fixture',
    'an_answered_approval_is_not_answered_again',
    'ambiguous_or_mixed_pending_approvals_stop_without_an_answer']]
NAMES=[*POLICY,NAME]
LIB = 'rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF'
LIB_HASHES = {'librag3db.so': 'b7e9fc75a3d2de04c72680b0309e8bb1df9eaadb5211ebbacf39716c6d0a2101', 'libvector.rag3db_extension': '1669d6392ec8067fcf92ae2ccc972fd7cd3086629fb82d1f5d254e90d72d7561'}
FEATURES = 'rag3db-native,code,daemon'


def sha(path):
    with Path(path).open('rb') as f: return hashlib.file_digest(f, 'sha256').hexdigest()

def save(name, value):
    (P / name).write_text(json.dumps(value, indent=2) + '\n')

def git(root, *args):
    return subprocess.check_output(['git', '-C', str(root), *args], text=True).strip()

def config(): return json.loads((Q / 'configuration.json').read_text())

def guard(cfg):
    for root, head in [(W, APP), (Path(cfg['control']), cfg['pilot']), (OLD_W, OLD_APP)]:
        assert git(root, 'rev-parse', 'HEAD') == head and not git(root, 'status', '--porcelain')
    assert sha(Q / 'qualify_green.py') == cfg['script_sha256']
    assert sha(Q / 'controller_policy.py') == cfg['policy_sha256']
    assert sha(Q / 'cache_state.py') == cfg['empty_sha256']
    assert sha(Q/'source_proof.py')==cfg['source_proof_sha256']
    assert sha(TRANSITION/'transition.json')==cfg['transition_sha256']
    source_proof.verify_archive(json.loads((TRANSITION/'frozen-source.json').read_text()))
    assert source_proof.tracked(W)==json.loads((TRANSITION/'new-source.json').read_text())
    empty_cache.require_size(SESSION_TARGET)
    empty_cache.require_identity(T,cfg['cache_identity'])
    assert sha(Q / 'confiner_validation.py') == cfg['supervisor_sha256']
    assert git(W, 'ls-tree', APP, 'extension/rag3weaver/codeparsers').split()[2] == git(C / 'codeparsers', 'rev-parse', 'HEAD')
    assert not git(C / 'codeparsers', 'status', '--porcelain')
    for name, expected in LIB_HASHES.items(): assert sha(R.parent / 'rag3db-libs' / LIB / name) == expected


def in_roots(value, roots):
    return any(value == str(root) or value.startswith(str(root) + '/') for root in roots)


def fresh_cache(cfg):
    subprocess.run(['python3', str(Q / 'confiner_validation.py'), 'verifier'], check=True)
    st = SESSION_TARGET.lstat()
    assert SESSION_TARGET.resolve() == SESSION_TARGET and stat.S_ISDIR(st.st_mode) and st.st_uid == os.getuid()
    assert [st.st_dev, st.st_ino] == cfg['target_identity']
    marker = SESSION_TARGET / '.arbre-du-target'
    assert marker.is_file() and not marker.is_symlink() and marker.read_text().strip() == str(OLD_W / 'extension/rag3weaver')
    assert (T/'.arbre-du-target').read_text().strip()==str(C)
    receipt=Path(cfg['red_receipt']);authority=json.loads(receipt.read_text())
    assert authority['etat']=='termine' and sha(receipt)==cfg['red_receipt_sha256']
    scope=json.loads((receipt.parent/'verdict.json').read_text())
    assert scope['code']==101 and not scope['restes'] and scope['nettoye'] and not Path(scope['prive']).exists()
    assert sha(RED/'first-red.json')==cfg['red_first_sha256']
    assert sha(RED/'native-fixture-elf')==cfg['red_elf_sha256']
    assert sha(RED/'ledger-before-witness.json')==cfg['red_ledger_sha256']
    expected = controller.policy(Path(cfg['control']), cfg['pilot'],cfg['launch_identity'])
    current_auth=json.loads(Path(os.environ['RAG3WEAVER_VALIDATION_RECU']).read_text())
    observed = controller.scan(expected, [T, W, Path(cfg['control'])], authority['cgroup'], owned_cgroup=current_auth['cgroup'])
    save('cache-after-poste.json', observed)
    assert not observed['ancestor_errors'], 'incomplete ancestor capture'
    assert not any(row.get('attributed') or row.get('reasons') for row in observed['unknown']), 'attributed or owned UNKNOWN'
    empty_cache.require_identity(T,cfg['cache_identity'])
    assert not observed['found'] and len(observed['controllers']) == 3 and {r['role'] for r in observed['controllers']} == {'launch','poste','watchdog'}



def driver():
    cfg = config(); guard(cfg)
    receipt = Path(os.environ['RAG3WEAVER_VALIDATION_RECU']); auth = json.loads(receipt.read_text())
    end = auth['echeance']; assert json.loads((receipt.parent / 'recette.json').read_text())['delai'] == 600
    save('authority-path.json', dict(path=str(receipt), deadline=end))
    env = {k: os.environ[k] for k in ['HOME', 'USER', 'LOGNAME', 'PATH', 'LANG', 'LC_ALL', 'CARGO_HOME', 'RUSTUP_HOME', 'POSTE_FILS', 'DBUS_SESSION_BUS_ADDRESS', 'RAG3WEAVER_VALIDATION_RECU', 'RAG3WEAVER_VALIDATION_JETON'] if k in os.environ}
    for label in ['tmp', 'cache', 'data', 'config', 'xdg-state', 'state-fixture', 'old-proof']: (P / label).mkdir()
    jobs=min(8,int(os.environ['POSTE_FILS']));assert jobs>0, 'invalid poste allocation'
    env.update(CARGO_TARGET_DIR=str(T), CARGO_MANIFEST_DIR=str(C), CARGO_BUILD_JOBS=str(jobs), RAYON_NUM_THREADS=str(jobs), RUST_TEST_THREADS='1', CARGO_INCREMENTAL='0',
        XDG_CACHE_HOME=str(P/'cache'), XDG_CONFIG_HOME=str(P/'config'), XDG_DATA_HOME=str(P/'data'), XDG_STATE_HOME=str(P/'xdg-state'),
        RAG3WEAVER_STATE_DIR=str(P/'state-fixture'), TMPDIR=str(P/'tmp'), RAG3DB_LIB=LIB,
        RAG3DB_LIBS_DIR=str(R.parent/'rag3db-libs'), RAG3DB_BUFFER_POOL_SIZE='1073741824')
    for suffix in ['', '_EMBEDDER', '_RERANKER', '_OCR']: env['RAG3WEAVER_BURN_DEVICE'+suffix] = 'cpu'
    assert not any(k.startswith('GOOGLE_') or k.endswith('_API_KEY') for k in env)
    save('selected-environment.json', {k:v for k,v in env.items() if k not in ['RAG3WEAVER_VALIDATION_JETON']})
    def run(label, args, cap):
        guard(cfg); deadline = min(end-15, time.monotonic()+cap)
        assert time.monotonic() < deadline
        with (P/(label+'.log')).open('w') as output:
            child = subprocess.Popen(args, cwd=C, env=env, stdout=output, stderr=subprocess.STDOUT)
            last_storage_check=0.0
            while child.poll() is None:
                if time.monotonic()-last_storage_check>=1:
                    empty_cache.require_size(SESSION_TARGET);last_storage_check=time.monotonic()
                assert time.monotonic() < deadline, label+' timeout'
                time.sleep(.05)
            rc = child.wait()
        assert time.monotonic() < deadline
        text=(P/(label+'.log')).read_text(errors='replace')
        if rc != 0:
            if not (P/'first-red.json').exists():save('first-red.json',dict(observed=time.monotonic(),phase=label,rc=rc))
            raise RuntimeError(label+' failed; no next invocation')
        return rc,text
    try:
        fresh_cache(cfg)
        baseline=json.loads((RED/'frozen-old-cache.json').read_text())
        empty_cache.verify_frozen(SESSION_TARGET,T,baseline)
        save('same-owner-cache-before.json',empty_cache.inventory(T,None))
        # Read-only preflight: refuse a foreign build.rs or changed gitlink before invoking the real guard.
        command=['bash','-c','. "$1"; if build_rs_d_un_autre_arbre "$2" "$3"; then exit 42; fi; sous_modules_de "$3"',
                 'same-owner-preflight',str(C/'scripts/garde_target.sh'),str(T),str(C)]
        _,submodules=run('same-owner-preflight',command,30)
        assert (T/'.sous-modules-du-target').read_text().strip()==submodules.strip(), 'changed gitlink requires a different admission'
        _,log=run('same-owner-guard',['bash','-c','. "$1"; garder_le_target "$2" "$3"','same-owner',str(C/'scripts/garde_target.sh'),str(T),str(C)],30)
        after=empty_cache.inventory(T,None)
        save('same-owner-cache-after.json',after)
        assert after==json.loads((P/'same-owner-cache-before.json').read_text()), 'guard mutated the known cache'
        assert 'empreintes effacées' not in log and (T/'.arbre-du-target').read_text().strip()==str(C)
        rc, text = run('compile', ['bash','scripts/cargo_magasin.sh','test','--lib','--no-default-features','--features',FEATURES,'--locked','--no-run','--message-format=json-render-diagnostics'],360)
        assert rc == 0, 'compilation failed before the green witnesses'
        artifacts = []
        for line in text.splitlines():
            try: row = json.loads(line)
            except ValueError: continue
            if row.get('reason')=='compiler-artifact' and row.get('target',{}).get('name')=='rag3weaver' and row.get('profile',{}).get('test') and row.get('executable'): artifacts.append(row)
        assert len(artifacts)==1
        artifact=artifacts[0]; binary=Path(artifact['executable']); assert binary.is_relative_to(T)
        assert set(artifact['features'])=={'rag3db-native','code','daemon'}
        copied=P/'native-fixture-elf'; shutil.copy2(binary,copied); assert sha(binary)==sha(copied)
        with copied.open('rb') as stream, mmap.mmap(stream.fileno(), 0, access=mmap.ACCESS_READ) as data:
            source_offset=data.find(APP[:9].encode()); assert source_offset >= 0
        save('ledger-before-witness.json',dict(source=APP,pilot=cfg['pilot'],artifact=artifact,binary=str(binary),sha256=sha(copied),source_offset=source_offset,features=artifact['features'],witnesses=NAMES))
        def runtime(name):
            return ['bash','-c','. "$1"; exporter_la_lib || exit 2; shift; exec "$@"','fixture-runtime',str(C/'scripts/lib_du_magasin.sh'),str(binary),*name]
        _,listed=run('inventory',runtime(['--list','--format','terse']),10)
        assert all(re.search(r'^'+re.escape(name)+r': test$',listed,re.M) for name in NAMES)
        save('inventory-named.json',dict(names=NAMES,distinct=6))
        for index,name in enumerate(NAMES):
            _,text=run('policy-'+str(index) if index<5 else 'native',runtime(['--exact',name,'--nocapture','--test-threads=1']),10 if index<5 else 120)
            assert re.search(r'test result: ok\. 1 passed; 0 failed; 0 ignored;',text), 'not one fully judged witness'
        empty_cache.verify_frozen(SESSION_TARGET,T,baseline)
        empty_cache.require_identity(T,cfg['cache_identity'])
        guard(cfg);assert time.monotonic()<end-3
        save('driver-result.json',dict(status='GREEN_TARGETED_ONLY',source=APP,pilot=cfg['pilot'],distinct=6,invocations=6,deadline=end,finished=time.monotonic(),not_a_full_qualification=True))
    except Exception as error:
        if not (P/'first-red.json').exists(): save('first-red.json',dict(observed=time.monotonic(),expected_red=False,error=repr(error)))
        raise


def prepare(pilot):
    assert os.uname().nodename.lower()=='luciepc'
    control=Path(__file__).resolve().parents[4]
    assert sys.orig_argv[1:]==[str(control/REL),'--green',pilot]
    assert re.fullmatch('[0-9a-f]{40}',pilot)
    assert not P.exists() and not Q.exists()
    for root,head in [(W,APP),(control,pilot),(OLD_W,OLD_APP)]:
        assert git(root,'rev-parse','HEAD')==head and not git(root,'status','--porcelain')
    transition=json.loads((TRANSITION/'transition.json').read_text())
    assert transition['source']==APP and transition['old_source']==source_proof.OLD_APP
    assert transition['owner']==str(C) and not transition['cache_guard_invoked'] and not transition['cargo_invoked']
    assert sha(TRANSITION/'frozen-source.json')==transition['frozen_source_sha256']
    assert sha(TRANSITION/'new-source.json')==transition['new_source_sha256']
    source_proof.verify_archive(json.loads((TRANSITION/'frozen-source.json').read_text()))
    receipts=list((RED/'supervision').glob('validation-*/autorite.json'));assert len(receipts)==1
    ledger=json.loads((RED/'ledger-before-witness.json').read_text())
    assert sha(RED/'native-fixture-elf')==ledger['sha256']==transition['old_elf_sha256']
    assert sha(RED/'ledger-before-witness.json')==transition['old_ledger_sha256']
    assert sha(receipts[0])==transition['old_authority_sha256']
    assert sha(RED/'new-cache-identity.json')==transition['cache_creation_record_sha256']
    assert ledger['source']==source_proof.OLD_APP
    P.mkdir(mode=0o700);Q.mkdir(mode=0o700)
    for name in ['qualify_green.py','controller_policy.py','cache_state.py','source_proof.py']:
        shutil.copy2(control/REL.replace('qualify_green.py',name),Q/name)
    shutil.copy2(C/'scripts/confiner_validation.py',Q/'confiner_validation.py')
    st=SESSION_TARGET.lstat()
    cfg=dict(mode='SAME_OWNER_GREEN_PROPOSED',source_proof_sha256=sha(Q/'source_proof.py'),
        transition_sha256=sha(TRANSITION/'transition.json'),cache_identity=transition['cache_identity'],
        red_receipt=str(receipts[0]),red_receipt_sha256=sha(receipts[0]),red_first_sha256=sha(RED/'first-red.json'),
        red_ledger_sha256=sha(RED/'ledger-before-witness.json'),red_elf_sha256=ledger['sha256'],
        launch_identity=controller.snapshot(Path('/proc')/str(os.getpid())),
        app=APP,pilot=pilot,control=str(control),target_identity=[st.st_dev,st.st_ino],
        script_sha256=sha(__file__),policy_sha256=sha(Q/'controller_policy.py'),
        empty_sha256=sha(Q/'cache_state.py'),supervisor_sha256=sha(Q/'confiner_validation.py'))
    (Q/'configuration.json').write_text(json.dumps(cfg,indent=2)+'\n');guard(cfg)
    runtime=Path('/run/user')/str(os.getuid());bus=runtime/'bus'
    assert runtime.resolve()==runtime and bus.resolve()==bus and bus.is_socket() and bus.stat().st_uid==os.getuid()
    env={k:os.environ[k] for k in ['HOME','USER','LOGNAME','PATH','LANG','LC_ALL','CARGO_HOME','RUSTUP_HOME'] if k in os.environ}
    env.update(POSTE_MEM_MAX='32G',RAG3WEAVER_BUILD_MEMORY_HIGH='0',XDG_RUNTIME_DIR=str(runtime),DBUS_SESSION_BUS_ADDRESS='unix:path='+str(bus))
    command=controller.commands(control,pilot)['outer']
    with (P/'supervision.log').open('w') as output: result=subprocess.run(command,cwd='/var/tmp',env=env,stdout=output,stderr=subprocess.STDOUT)
    verdicts=list((P/'supervision').glob('validation-*/verdict.json'))
    outcome=dict(status='UNEXPECTED_FAILURE',scope_rc=result.returncode,not_a_full_qualification=True)
    if len(verdicts)==1:
        scope=json.loads(verdicts[0].read_text());outcome['scope']=scope
        try:
            assert result.returncode==0 and scope['verdict']=='VERT' and scope['code']==0 and not scope['restes'] and scope['nettoye']
            authority=json.loads((verdicts[0].parent/'autorite.json').read_text())
            assert authority['etat']=='termine' and not Path(scope['prive']).exists()
            driver_result=json.loads((P/'driver-result.json').read_text());assert driver_result['status']=='GREEN_TARGETED_ONLY'
            guard(cfg)
            ledger=json.loads((P/'ledger-before-witness.json').read_text())
            assert sha(ledger['binary'])==sha(P/'native-fixture-elf')==ledger['sha256']
            empty_cache.verify_frozen(SESSION_TARGET,T,json.loads((RED/'frozen-old-cache.json').read_text()))
            empty_cache.require_identity(T,cfg['cache_identity'])
            finished=time.monotonic();assert finished<authority['echeance']
            outcome.update(status='GREEN_TARGETED_ONLY',deadline=authority['echeance'],finished=finished,authority_terminated=True,bus_absent=True)
        except Exception as error:outcome['error']=repr(error)
    if outcome['status']=='GREEN_TARGETED_ONLY' and time.monotonic()>=outcome['deadline']:
        outcome.update(status='UNEXPECTED_FAILURE',error='deadline before publication')
    save('result.json',outcome);print(json.dumps(outcome));sys.exit(0 if outcome['status']=='GREEN_TARGETED_ONLY' else 1)


if __name__=='__main__':
    mode=sys.argv[1]
    if mode=='--admit':
        os.execv(sys.executable,[sys.executable,str(Q/'confiner_validation.py'),'lancer','--delai','600','--preuves',str(P/'supervision'),'--','python3',str(Q/'qualify_green.py'),'--driver',sys.argv[-1]])
    elif mode=='--driver': driver()
    elif mode=='--green': prepare(sys.argv[2])
    else: raise RuntimeError('explicit green mode required')
