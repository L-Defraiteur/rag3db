#!/usr/bin/env python3
"""One deterministic native red witness; no Vertex and no permission granted."""
from pathlib import Path
import hashlib, json, os, re, shutil, signal, stat, subprocess, sys, time

R = Path.home() / '.cache/rag3weaver-build'
W = Path.home() / 'git_workspaces/rag3db-cx-archi-doc-supervisor-r1'
C = W / 'extension/rag3weaver'
T = R / 'target-cx-archi-pas-va-au-bout-r2'
P = R / 'cx-archi-doc-supervisor-red1'
Q = R / 'cx-archi-doc-supervisor-controller-red1'
OLD = R / 'cx-archi-pas-va-au-bout-r2-59ac-pass1'
OLD_W = Path.home() / 'git_workspaces/rag3db-cx-archi-pas-va-au-bout-r2'
APP = 'ea67054a11d73e06cf44c38d9ce941a754c0190d'
OLD_APP = '713433bd602a6d35266fe5cd528925ba664924c9'
REL = 'docs/rag3weaver-archi/16-11-2026/doc-supervisor/qualify_red.py'
NAME = 'daemon_host::teammate_tests::a_documentation_supervisor_denies_lock_deletion_and_the_teammate_finishes_the_original_request'
EXPECTED = 'documentation fixture has no supervisor answer for the pending lock deletion'
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
    assert sha(Q / 'qualify_red.py') == cfg['script_sha256']
    assert sha(Q / 'confiner_validation.py') == cfg['supervisor_sha256']
    assert git(W, 'ls-tree', APP, 'extension/rag3weaver/codeparsers').split()[2] == git(C / 'codeparsers', 'rev-parse', 'HEAD')
    assert not git(C / 'codeparsers', 'status', '--porcelain')
    for name, expected in LIB_HASHES.items(): assert sha(R.parent / 'rag3db-libs' / LIB / name) == expected


def in_roots(value, roots):
    return any(value == str(root) or value.startswith(str(root) + '/') for root in roots)


def fresh_cache(cfg):
    subprocess.run(['python3', str(Q / 'confiner_validation.py'), 'verifier'], check=True)
    st = T.lstat()
    assert T.resolve() == T and stat.S_ISDIR(st.st_mode) and st.st_uid == os.getuid()
    assert [st.st_dev, st.st_ino] == cfg['target_identity']
    marker = T / '.arbre-du-target'
    assert marker.is_file() and not marker.is_symlink() and marker.read_text().strip() == str(OLD_W / 'extension/rag3weaver')
    receipt = Path(cfg['old_receipt']); authority = json.loads(receipt.read_text())
    assert sha(OLD / 'manifest-final.json') == cfg['old_manifest_sha256']
    assert authority['etat'] == 'termine' and sha(receipt) == cfg['old_receipt_sha256']
    assert json.loads((receipt.parent / 'verdict.json').read_text()) == cfg['old_verdict']
    assert not Path(cfg['old_verdict']['prive']).exists()
    roots = [T, W, OLD_W, Path(cfg['control'])]
    # Exempt only the exact launch/poste ancestors, never target/old-source FDs.
    ancestors = {}; cursor = os.getppid()
    while cursor > 1 and cursor not in ancestors:
        proc = Path('/proc') / str(cursor)
        s = (proc / 'stat').read_text().rsplit(') ', 1)[1].split()
        ancestors[cursor] = (s[19], int(s[1]), (proc / 'cgroup').read_text())
        cursor = int(s[1])
    launcher = str(Path(cfg['control']) / REL); poste = str(C / 'scripts/poste')
    roles = {'launch': [launcher, cfg['pilot']], 'poste': [poste, 'lourd', sys.executable, str(Q / 'qualify_red.py'), '--admit', cfg['pilot']]}
    expected_exes = {'launch': str(Path(sys.executable).resolve()), 'poste': str(Path('/bin/sh').resolve())}
    found = []; unknown = []; controllers = []
    for proc in Path('/proc').iterdir():
        if not proc.name.isdigit() or int(proc.name) == os.getpid(): continue
        attributed = False; role = None
        try:
            if proc.stat().st_uid != os.getuid(): continue
            s = (proc / 'stat').read_text().rsplit(') ', 1)[1].split()
            if s[0] == 'Z': continue
            cg = (proc / 'cgroup').read_text(); argv = [os.fsdecode(v) for v in (proc / 'cmdline').read_bytes().split(b'\0') if v]
            exe = os.readlink(proc / 'exe'); cwd = os.readlink(proc / 'cwd')
            attributed = cg == authority['cgroup'] + '\n' or any(in_roots(v, roots) for v in argv)
            if int(proc.name) in ancestors and ancestors[int(proc.name)] == (s[19], int(s[1]), cg):
                matches = [key for key, tail in roles.items() if argv[1:] == tail and exe == expected_exes[key] and sha(proc / 'exe') == sha(expected_exes[key]) and cwd == '/var/tmp']
                if len(matches) == 1:
                    role = matches[0]; attributed = False
                    controllers.append(dict(pid=int(proc.name), start=s[19], role=role, argv=argv, exe=exe, exe_sha256=sha(proc / 'exe'), cgroup=cg))
            for value in [exe, cwd]:
                if in_roots(value, roots): attributed = True
            for fd in (proc / 'fd').iterdir():
                try: value = os.readlink(fd)
                except FileNotFoundError: continue
                if in_roots(value, roots):
                    if role == 'poste' and value == poste and not in_roots(value, [T, OLD_W]):
                        assert sha(value) == sha(C / 'scripts/poste'); continue
                    attributed = True
            assert (proc / 'stat').read_text().rsplit(') ', 1)[1].split()[19] == s[19]
            if attributed: found.append(dict(pid=proc.name, start=s[19], role=role))
        except PermissionError:
            unknown.append(dict(pid=proc.name, attributed=attributed or role is not None))
            if attributed or role is not None: found.append(dict(pid=proc.name, reason='attributed UNKNOWN'))
        except (FileNotFoundError, ProcessLookupError): continue
    save('cache-after-poste.json', dict(observed=time.monotonic(), found=found, unknown=unknown, controllers=controllers, device=st.st_dev, inode=st.st_ino))
    assert not found and len(controllers) == 2 and {r['role'] for r in controllers} == {'launch', 'poste'}


def driver():
    cfg = config(); guard(cfg)
    receipt = Path(os.environ['RAG3WEAVER_VALIDATION_RECU']); auth = json.loads(receipt.read_text())
    end = auth['echeance']; assert json.loads((receipt.parent / 'recette.json').read_text())['delai'] == 1200
    save('authority-path.json', dict(path=str(receipt), deadline=end))
    env = {k: os.environ[k] for k in ['HOME', 'USER', 'LOGNAME', 'PATH', 'LANG', 'LC_ALL', 'CARGO_HOME', 'RUSTUP_HOME', 'DBUS_SESSION_BUS_ADDRESS', 'RAG3WEAVER_VALIDATION_RECU', 'RAG3WEAVER_VALIDATION_JETON'] if k in os.environ}
    for label in ['tmp', 'cache', 'data', 'config', 'xdg-state', 'state-fixture', 'old-elfs']: (P / label).mkdir()
    env.update(CARGO_TARGET_DIR=str(T), CARGO_MANIFEST_DIR=str(C), CARGO_BUILD_JOBS='8', CARGO_INCREMENTAL='0',
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
            while child.poll() is None:
                assert time.monotonic() < deadline, label+' timeout'
                time.sleep(.05)
            rc = child.wait()
        assert time.monotonic() < deadline
        return rc, (P/(label+'.log')).read_text(errors='replace')
    try:
        # Preserve the previous real ELF evidence before common invalidation.
        old_binaries = []
        for f in sorted((T/'debug/deps').iterdir()):
            if f.name.startswith(('rag3weaver-', 'e2e-')) and f.is_file():
                with f.open('rb') as stream: is_elf = stream.read(4) == b'\x7fELF'
                if is_elf:
                    copied = P/'old-elfs'/f.name; shutil.copy2(f, copied)
                    assert sha(f) == sha(copied)
                    old_binaries.append(dict(path=str(f), copy=str(copied), sha256=sha(copied)))
        assert old_binaries; save('old-elfs.json', old_binaries)
        fresh_cache(cfg)
        rc, log = run('target-guard', ['bash','-c','. "$1"; garder_le_target "$2" "$3"','fixture-cache',str(C/'scripts/garde_target.sh'),str(T),str(C)], 60)
        assert rc == 0 and 'target repris de '+str(OLD_W/'extension/rag3weaver')+' : empreintes effacées pour ' in log
        rc, text = run('compile', ['bash','scripts/cargo_magasin.sh','test','--lib','--no-default-features','--features',FEATURES,'--no-run','--message-format=json-render-diagnostics'],900)
        assert rc == 0, 'compilation failed before the expected red witness'
        artifacts = []
        for line in text.splitlines():
            try: row = json.loads(line)
            except ValueError: continue
            if row.get('reason')=='compiler-artifact' and row.get('target',{}).get('name')=='rag3weaver' and row.get('profile',{}).get('test') and row.get('executable'): artifacts.append(row)
        assert len(artifacts)==1
        artifact=artifacts[0]; binary=Path(artifact['executable']); assert binary.is_relative_to(T)
        assert set(artifact['features'])=={'rag3db-native','code','daemon'}
        copied=P/'native-fixture-elf'; shutil.copy2(binary,copied); assert sha(binary)==sha(copied)
        save('ledger-before-witness.json',dict(source=APP,pilot=cfg['pilot'],artifact=artifact,binary=str(binary),sha256=sha(copied),features=artifact['features'],witness=NAME))
        rc, text = run('witness',['bash','-c','. "$1"; exporter_la_lib || exit 2; shift; exec "$@"','fixture-runtime',str(C/'scripts/lib_du_magasin.sh'),str(binary),'--exact',NAME,'--nocapture','--test-threads=1'],120)
        expected_red = rc != 0 and EXPECTED in text and ('FAILED' in text) and 'DOCUMENTATION_FAILED_FIXTURE ' in text
        save('first-red.json',dict(observed=time.monotonic(),phase='witness',rc=rc,expected_red=expected_red,expected=EXPECTED,counts=re.findall(r'test result:.*',text)))
        assert expected_red,'witness did not reach the expected missing-supervisor failure'
        save('driver-result.json',dict(status='EXPECTED_RED',not_a_green_qualification=True,rc=rc))
        sys.exit(101)
    except Exception as error:
        if not (P/'first-red.json').exists(): save('first-red.json',dict(observed=time.monotonic(),expected_red=False,error=repr(error)))
        raise


def prepare(pilot):
    assert os.uname().nodename.lower()=='luciepc'
    control=Path(__file__).resolve().parents[4]
    assert sys.orig_argv[1:]==[str(control/REL),pilot]
    assert re.fullmatch('[0-9a-f]{40}',pilot)
    assert not P.exists() and not Q.exists()
    old_source=json.loads((OLD/'source.json').read_text()); assert old_source['source']==OLD_APP
    receipts=list((OLD/'supervision').glob('validation-*/autorite.json')); assert len(receipts)==1
    authority=json.loads(receipts[0].read_text()); assert authority['etat']=='termine'
    verdict=json.loads((receipts[0].parent/'verdict.json').read_text()); assert verdict['verdict']=='ROUGE' and verdict['restes']==[] and verdict['nettoye']
    assert (T/'.arbre-du-target').read_text().strip()==str(OLD_W/'extension/rag3weaver')
    P.mkdir(mode=0o700); Q.mkdir(mode=0o700)
    shutil.copy2(__file__,Q/'qualify_red.py'); shutil.copy2(C/'scripts/confiner_validation.py',Q/'confiner_validation.py')
    st=T.lstat()
    cfg=dict(app=APP,pilot=pilot,control=str(control),script_sha256=sha(__file__),supervisor_sha256=sha(Q/'confiner_validation.py'),target_identity=[st.st_dev,st.st_ino],old_receipt=str(receipts[0]),old_receipt_sha256=sha(receipts[0]),old_verdict=verdict,old_manifest_sha256=sha(OLD/'manifest-final.json'))
    (Q/'configuration.json').write_text(json.dumps(cfg,indent=2)+'\n'); guard(cfg)
    runtime=Path('/run/user')/str(os.getuid());bus=runtime/'bus'
    assert runtime.resolve()==runtime and bus.resolve()==bus and bus.is_socket() and bus.stat().st_uid==os.getuid()
    env={k:os.environ[k] for k in ['HOME','USER','LOGNAME','PATH','LANG','LC_ALL','CARGO_HOME','RUSTUP_HOME'] if k in os.environ}
    env.update(POSTE_MEM_MAX='32G',RAG3WEAVER_BUILD_MEMORY_HIGH='0',XDG_RUNTIME_DIR=str(runtime),DBUS_SESSION_BUS_ADDRESS='unix:path='+str(bus))
    command=['timeout','--signal=TERM','--kill-after=15s','1320s',str(C/'scripts/poste'),'lourd',sys.executable,str(Q/'qualify_red.py'),'--admit',pilot]
    with (P/'supervision.log').open('w') as output: result=subprocess.run(command,cwd='/var/tmp',env=env,stdout=output,stderr=subprocess.STDOUT)
    verdicts=list((P/'supervision').glob('validation-*/verdict.json'))
    outcome=dict(status='UNEXPECTED_FAILURE',scope_rc=result.returncode,not_a_green_qualification=True)
    if len(verdicts)==1:
        scope=json.loads(verdicts[0].read_text());outcome['scope']=scope
        red=P/'driver-result.json'
        if red.is_file() and json.loads(red.read_text())['status']=='EXPECTED_RED' and scope['verdict']=='ROUGE' and not scope['restes'] and scope['nettoye']:
            outcome['status']='EXPECTED_RED';outcome['authority_terminated']=json.loads((verdicts[0].parent/'autorite.json').read_text())['etat']=='termine'
    save('result.json',outcome); print(json.dumps(outcome));sys.exit(1)


if __name__=='__main__':
    mode=sys.argv[1]
    if mode=='--admit':
        os.execv(sys.executable,[sys.executable,str(Q/'confiner_validation.py'),'lancer','--delai','1200','--preuves',str(P/'supervision'),'--','python3',str(Q/'qualify_red.py'),'--driver',sys.argv[-1]])
    elif mode=='--driver': driver()
    else: prepare(mode)
