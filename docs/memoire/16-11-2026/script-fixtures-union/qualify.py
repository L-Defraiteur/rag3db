#!/usr/bin/env python3
"""One fresh allocation: canonical pass, then the identical large unit-test ELF."""
from pathlib import Path
from collections import Counter
import argparse, hashlib, json, os, re, shutil, stat, subprocess, sys, time
import profile_contract as contract
import source_proof
import cache_state
import mmap

APP = 'e2722d521192b3c25f9dd13e0e9aed47a7d7487f'
BASE = 'bef0b27cb33b9143dabee019fccdf7453da553a3'
REL = 'docs/memoire/16-11-2026/script-fixtures-union/qualify.py'
PIN = 'rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF'
SO = {'librag3db.so':'b7e9fc75a3d2de04c72680b0309e8bb1df9eaadb5211ebbacf39716c6d0a2101',
      'libvector.rag3db_extension':'1669d6392ec8067fcf92ae2ccc972fd7cd3086629fb82d1f5d254e90d72d7561'}
PASS_SHA = '2150fcd847b1df7de04908d357a0c78abc05e50ce599e2793ff08ab48108825d'
FEATURES = 'rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres,openai-llm,anthropic-llm'
R = Path.home()/'.cache/rag3weaver-build'
W = Path.home()/'git_workspaces/cx-memoire-script-fixtures-r4'
C = W/'extension/rag3weaver'
T = R/'target-cx-memoire-script-fixtures'
P = R/'cx-memoire-script-fixtures-union-qualification-v1'
Q = R/'cx-memoire-script-fixtures-union-controller-v1'
S = R/'cx-memoire-script-fixtures-union-scope-v1'


def sha(path):
    with Path(path).open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()

def git(root,*args):return subprocess.check_output(['git','-C',str(root),*args],text=True).strip()
def save(root,name,value,publication_deadline=None):
    with (root/name).open('x') as stream:
        if publication_deadline is not None and value.get('status')=='GREEN' and time.monotonic()>=publication_deadline:
            value=dict(status='RED',returncode=1,error='deadline reached after open immediately before publication')
        json.dump(value,stream,indent=2);stream.write('\n');stream.flush();os.fsync(stream.fileno())
    return value

def require(condition,message):
    if not condition:raise RuntimeError(message)

def deadline(end,now=None):
    require((time.monotonic() if now is None else now)<end,'allocation deadline reached')

def summary(text):
    rows=re.findall(r'^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;.*? (\d+) filtered out;',text,re.M)
    require(len(rows)==1,'expected exactly one successful test summary')
    passed,failed,ignored,filtered=map(int,rows[0]);require(passed>0 and failed==0,'no passing tests or failed tests')
    return dict(passed=passed,failed=failed,ignored=ignored,filtered=filtered)

def compare_judge_names(first,second,expected):
    def collect(text):
        result={'ok':Counter(),'ignored':Counter()}
        for name,status in re.findall(r'^test (.*?) \.\.\. (ok|ignored)(?:, [^\n]*)?$',text,re.M):
            result[status][name.removesuffix(' - should panic')]+=1
        return result
    left,right=collect(first),collect(second)
    only_first={status:dict(left[status]-right[status]) for status in left}
    only_second={status:dict(right[status]-left[status]) for status in left}
    duplicates={label:sorted(name for name,count in (row['ok']+row['ignored']).items() if count!=1) for label,row in [('first',left),('second',right)]}
    totals={label:{status:sum(names.values()) for status,names in row.items()} for label,row in [('first',left),('second',right)]}
    cardinalities=all(row['ok']==expected['passed'] and row['ignored']==expected['ignored'] for row in totals.values())
    counts_match=summary(first)==summary(second)==expected and expected['filtered']==0
    qualified=counts_match and cardinalities and not any(duplicates.values()) and not any(only_first.values()) and not any(only_second.values())
    return dict(qualified=qualified,counts_match=counts_match,cardinalities_match=cardinalities,first=left,second=right,totals=totals,duplicates=duplicates,only_first=only_first,only_second=only_second,meaning='multisets of actual Rust harness names; ignored remain unplayed; should-panic display suffix normalized')

def large_elf(text,target):
    rows=re.findall(r'Running unittests src/lib\.rs \(([^\n]+)\)',text)
    require(len(rows)==1,'expected one large unit-test ELF')
    binary=Path(rows[0]);require(binary.is_absolute() and binary.is_relative_to(target),'ELF outside private target')
    require(binary.resolve()==binary and binary.is_file(),'ELF path not canonical')
    with binary.open('rb') as stream:require(stream.read(4)==b'\x7fELF','not an ELF')
    return binary

def failure_match(text):
    return re.search(r'^test [^\n]* \.\.\. FAILED\b|^\s*FAILED\s*$|^test result: FAILED\.',text,re.M)

def guard(cfg):
    for root,head in [(W,APP),(Path(cfg['control']),cfg['pilot'])]:
        require(git(root,'rev-parse','HEAD')==head and not git(root,'status','--porcelain'),'source/control changed')
    require(git(W,'merge-base',APP,'origin/master')==BASE,'master base changed since source freeze')
    sub=C/'codeparsers'
    require(git(sub,'rev-parse','HEAD')==git(W,'ls-tree',APP,'extension/rag3weaver/codeparsers').split()[2] and not git(sub,'status','--porcelain'),'gitlink mismatch/dirty')
    for name,value in cfg['copies'].items():require(sha(Q/name)==value,'controller copy changed: '+name)
    require(sha(R/'passe_epinglee.sh')==PASS_SHA and sha(W/'tools/poste/passe_epinglee.sh')==PASS_SHA,'canonical pass differs')
    source_proof.verify_archive(json.loads((source_proof.TRANSITION/'frozen-source.json').read_text()))
    require(source_proof.tracked(W)==json.loads((source_proof.TRANSITION/'new-source.json').read_text()),'new physical source changed')
    require(sha(source_proof.TRANSITION/'transition.json')==cfg['transition_sha256'],'Git-only transition changed')
    for name,value in SO.items():require(sha(R.parent/'rag3db-libs'/PIN/name)==value,'SDK changed')

def scan(owned=()):
    # Unknown global observations are reported; known owned identities fail closed.
    roots=[W,T]
    if (P/'short-tmp.json').is_file():roots.append(Path(json.loads((P/'short-tmp.json').read_text())['path']))
    found=[];unknown=[];alive=[];owned_unknown=[]
    for row in owned:
        proc=Path('/proc')/str(row['pid'])
        try:
            fields=(proc/'stat').read_text().rsplit(') ',1)[1].split()
            if fields[19]==row['debut'] and fields[0]!='Z':alive.append(row)
        except (FileNotFoundError,ProcessLookupError):pass
        except OSError as error:owned_unknown.append(dict(pid=row['pid'],error=repr(error)))
    for proc in Path('/proc').iterdir():
        if not proc.name.isdigit() or int(proc.name)==os.getpid():continue
        attributed=False
        try:
            if proc.stat().st_uid!=os.getuid():continue
            fields=(proc/'stat').read_text().rsplit(') ',1)[1].split()
            if fields[0]=='Z':continue
            values=[('exe',os.readlink(proc/'exe')),('cwd',os.readlink(proc/'cwd'))]
            for fd in (proc/'fd').iterdir():
                try:values.append(('fd:'+fd.name,os.readlink(fd)))
                except (FileNotFoundError,ProcessLookupError):continue
            for field,value in values:
                if any(value==str(root) or value.startswith(str(root)+'/') for root in roots):
                    attributed=True;found.append(dict(pid=proc.name,start=fields[19],field=field,path=value))
        except (FileNotFoundError,ProcessLookupError):continue
        except OSError as error:unknown.append(dict(pid=proc.name,attributed=attributed,error=repr(error)))
    return dict(observed=time.monotonic(),found=found,unknown=unknown,owned_alive=alive,owned_unknown=owned_unknown)

FAILED = re.compile(r'^test .* \.\.\. FAILED$|^\s*FAILED\s*$|^test result: FAILED\.', re.M)

def first_red(reason):
    try:
        with (P / 'first-red.json').open('x') as f:
            json.dump(dict(reason=reason, observed=time.monotonic()), f)
            f.flush(); os.fsync(f.fileno())
    except FileExistsError:
        pass

def active_logs(name):
    paths = [P / (name + '.log')]
    if name == 'official-pass':
        paths += [P / 'official' / (item + '.log') for item in
                  ['temoins', 'e2e_code', 'e2e_lectures', 'check_features',
                   'preuve', 'lib', 'navigateur', 'resume']]
    return paths

def watch_logs(name):
    # Only files produced by this invocation, never cached template evidence.
    for log in active_logs(name):
        if not log.exists():
            continue
        with log.open('rb') as stream:
            before = os.fstat(stream.fileno())
            data = stream.read()
            after = os.fstat(stream.fileno())
        text = data.decode(errors='replace')
        match = FAILED.search(text)
        if match is None and log.name in ['check_features.log', 'resume.log']:
            match = re.search(r'^(?:CHECK_FEATURES )?ROUGE\b.*$', text, re.M)
        if match:
            evidence = dict(invocation=name, path=str(log), observed=time.monotonic(),
                            device=before.st_dev, inode=before.st_ino, size_before=before.st_size,
                            size_after=after.st_size, mtime_ns=after.st_mtime_ns,
                            bytes_read=len(data), sha256=hashlib.sha256(data).hexdigest(),
                            matched=match.group(), meaning='active invocation output')
            with (P / 'watcher-first-match.json').open('x') as stream:
                json.dump(evidence, stream, indent=2)
                stream.flush(); os.fsync(stream.fileno())
            first_red('active log failure: ' + str(log))
            raise AssertionError('first red in active log ' + str(log))

def run(label,args,env,end,cap):
    limit=min(end-30,time.monotonic()+cap);deadline(limit)
    with (P/(label+'.log')).open('x') as output:
        child=subprocess.Popen(args,cwd=C,env=env,stdout=output,stderr=subprocess.STDOUT)
        while True:
            watch_logs(label)
            deadline(limit)
            if child.poll() is not None:break
            require(int(subprocess.check_output(['du','-sb',str(T)],text=True).split()[0])<=32*1024**3,'target over 32GiB') if T.exists() else None
            time.sleep(.2)
        require(child.wait()==0,label+' returned nonzero')
    deadline(limit);return (P/(label+'.log')).read_text(errors='replace')

def runtime_command():
    return ['bash','-c','. "$1"; exporter_la_lib || exit 2; shift; exec "$@"','second-lib-sdk-once',str(C/'scripts/lib_du_magasin.sh'),sys.executable,str(Q/'qualify.py'),'--exec-second']

def exec_second():
    cfg=json.loads((Q/'configuration.json').read_text());guard(cfg)
    ledger=json.loads((P/'ledger-before-second-lib.json').read_text());binary=Path(ledger['binary'])
    expected=json.loads((P/'profile.json').read_text())
    sdk=str(R.parent/'rag3db-libs'/PIN)
    expected.update(RAG3DB_SHARED='1',RAG3DB_LIBRARY_DIR=sdk,RAG3DB_INCLUDE_DIR=sdk,LD_LIBRARY_PATH=sdk)
    actual={key:os.environ.get(key) for key in expected}
    differences={key:dict(expected=value,actual=actual[key]) for key,value in expected.items() if actual[key]!=value}
    save(P,'sdk-exec-profile.json',dict(observed=time.monotonic(),pid=os.getpid(),binary=str(binary),binary_sha256=sha(binary),expected=expected,actual=actual,diff=differences,manifest_path=str(C),manifest_exists=C.is_dir(),manifest_canonical=str(C.resolve())))
    require(expected.get('CARGO_MANIFEST_DIR')==str(C) and actual.get('CARGO_MANIFEST_DIR')==str(C) and C.is_dir() and C.resolve()==C,'runtime CARGO_MANIFEST_DIR is not the exact existing crate')
    require(not differences,'exported runtime profile differs')
    require(sha(binary)==sha(ledger['copy'])==ledger['sha256'],'exact ELF changed before exec')
    receipt=Path(os.environ['RAG3WEAVER_VALIDATION_RECU']);end=json.loads(receipt.read_text())['echeance'];deadline(end-15)
    os.execve(str(binary),[str(binary)],dict(os.environ))

def driver():
    cfg=json.loads((Q/'configuration.json').read_text());guard(cfg)
    subprocess.run([sys.executable,str(Q/'confiner_validation.py'),'verifier'],check=True)
    receipt=Path(os.environ['RAG3WEAVER_VALIDATION_RECU']);auth=json.loads(receipt.read_text());end=auth['echeance']
    recipe=json.loads((receipt.parent/'recette.json').read_text())
    require(recipe['delai']==3600 and auth['etat']=='actif','not a fresh distinct union authority')
    group=Path('/sys/fs/cgroup')/auth['cgroup'].lstrip('/')
    selected={name:(group/name).read_text().strip() for name in ['memory.max','memory.high','memory.swap.max','cpu.max']}
    require(selected['memory.max']=='34359738368' and selected['memory.high']=='max' and selected['memory.swap.max']=='0','memory contract differs')
    require(selected['cpu.max'].split()[0]!='max','CPU quota absent')
    save(P,'authority-path.json',dict(path=str(receipt),initial=end-3600,deadline=end,limits=selected))
    env={k:os.environ[k] for k in ['HOME','USER','LOGNAME','PATH','LANG','LC_ALL','CARGO_HOME','RUSTUP_HOME','DBUS_SESSION_BUS_ADDRESS','RAG3WEAVER_VALIDATION_RECU','RAG3WEAVER_VALIDATION_JETON'] if k in os.environ}
    for name in ['cache','config','data','xdg-state','state-fixture']:(P/name).mkdir()
    short=contract.create_short(R);save(P,'short-tmp.json',short)
    env.update(CARGO_MANIFEST_DIR=str(C),CARGO_TARGET_DIR=str(T),CIBLE=T.name,CARGO_BUILD_JOBS=str(min(8,int(os.environ.get('CARGO_BUILD_JOBS','8')))),CARGO_INCREMENTAL='0',CARGO_TERM_COLOR='never',RUST_TEST_THREADS='8',
        TMPDIR=short['path'],XDG_CACHE_HOME=str(P/'cache'),XDG_CONFIG_HOME=str(P/'config'),XDG_DATA_HOME=str(P/'data'),XDG_STATE_HOME=str(P/'xdg-state'),RAG3WEAVER_STATE_DIR=str(P/'state-fixture'),
        POSTE_MEM_MAX='32G',POSTE_SWAP_MAX='0',RAG3WEAVER_BUILD_MEMORY_HIGH='0',RAG3WEAVER_TARGET_MAX_GO='32',RAG3DB_LIB=PIN,RAG3DB_LIBS_DIR=str(R.parent/'rag3db-libs'),RAG3DB_BUFFER_POOL_SIZE='1073741824',
        RAG3WEAVER_EXIGER_NAVIGATEUR='1',RAG3WEAVER_PAQUETS_REACT=str(R/'I-paquets-react'),SUITE_DU_TEMOIN='lib',SUITES='-')
    for suffix in ['','_EMBEDDER','_RERANKER','_OCR']:env['RAG3WEAVER_BURN_DEVICE'+suffix]='cpu'
    # Match the unchanged canonical pass's PC profile for the second direct invocation.
    env.update(RAG3WEAVER_EMBED_SERVICE='127.0.0.1:7878,127.0.0.1:7879',RAG3WEAVER_PG='postgres://rag3weaver:rag3weaver@127.0.0.1:5433/rag3weaver_test',RAG3WEAVER_TESTS_REPLI_PERIPHERIQUE='igpu:0',RAG3WEAVER_TARGET_REPRENDRE='1')
    require(not Path(env['XDG_STATE_HOME']).is_relative_to(Path(env['RAG3WEAVER_STATE_DIR'])) and not Path(env['RAG3WEAVER_STATE_DIR']).is_relative_to(Path(env['XDG_STATE_HOME'])),'state roots overlap')
    save(P,'profile.json',{k:v for k,v in env.items() if k!='RAG3WEAVER_VALIDATION_JETON'})
    fresh=contract.capture(cfg,sha,git);save(P,'carry-after-poste.json',fresh);contract.verify(fresh,cfg['carry'])
    oldreceipt=Path(json.loads((R/'cx-memoire-script-fixtures-qualification-v3/authority-path.json').read_text())['path'])
    observed=scan(json.loads((oldreceipt.parent/'enfants.json').read_text()));save(P,'scan-before-carry.json',observed)
    require(not observed['found'] and not observed['owned_alive'] and not observed['owned_unknown'] and not any(v['attributed'] for v in observed['unknown']),'activity or owned UNKNOWN before carry')
    contract.check_same_short(short);guard(cfg);deadline(end-30)
    before_guard=cache_state.inventory(T,None);save(P,'same-owner-before.json',before_guard)
    run('same-owner-preflight',['bash','-c','. "$1"; if build_rs_d_un_autre_arbre "$2" "$3"; then exit 42; fi; sous_modules_de "$3"','same-owner-readonly',str(C/'scripts/garde_target.sh'),str(T),str(C)],env,end,30)
    submodules=(P/'same-owner-preflight.log').read_text().strip();require((T/'.sous-modules-du-target').read_text().strip()==submodules,'gitlink path or commit changed')
    guard_log=run('same-owner-guard',['bash','-c','. "$1"; garder_le_target "$2" "$3"','same-owner',str(C/'scripts/garde_target.sh'),str(T),str(C)],env,end,30)
    after_guard=cache_state.inventory(T,None);save(P,'same-owner-after.json',after_guard);require(after_guard==before_guard and 'empreintes effacées' not in guard_log,'same-owner guard changed cache')
    required=subprocess.run(['bash',str(W/'tools/poste/required_suites.sh'),BASE,APP],cwd=W,capture_output=True,text=True,timeout=15)
    save(P,'required-suites.json',dict(rc=required.returncode,stdout=required.stdout,stderr=required.stderr));require(required.returncode==0 and not required.stdout.strip(),'unexpected required suite for this lot')
    guard(cfg);deadline(end-30)
    run('official-pass',['bash',str(R/'passe_epinglee.sh'),W.name,str(P.relative_to(R)/'official'),'codex_threads'],env,end,3000)
    canonical=P/'official/resume.log';require(canonical.read_text().splitlines()[-1]=='VERT','canonical summary not green')
    first=(P/'official/lib.log').read_text();counts=summary(first);require(counts['filtered']==0,'first lib filtered')
    binary=large_elf(first,T);copied=P/'large-unit-tests';shutil.copy2(binary,copied);elf_sha=sha(binary);require(sha(copied)==elf_sha,'ELF copy differs')
    with copied.open('rb') as stream,mmap.mmap(stream.fileno(),0,access=mmap.ACCESS_READ) as data:
        source_offset=data.find(APP[:9].encode());require(source_offset>=0,'new union source prefix absent from ELF')
    save(P,'ledger-before-second-lib.json',dict(source=APP,pilot=cfg['pilot'],binary=str(binary),copy=str(copied),sha256=elf_sha,features=FEATURES,source_offset=source_offset,first=counts,canonical_summary_sha256=sha(canonical),meaning='hash after canonical pass and before second invocation, not pre-first-test attestation'))
    guard(cfg);deadline(end-30)
    second=run('second-lib',runtime_command(),env,end,600)
    counts2=summary(second);require(counts2==counts and counts2['filtered']==0,'second lib counts differ or filtered')
    comparison=compare_judge_names(first,second,counts);save(P,'judge-names-comparison.json',comparison)
    require(comparison['qualified'],'library judge identities differ, duplicate or incomplete')
    require(sha(binary)==sha(copied)==elf_sha,'large ELF changed')
    guard(cfg);require(sha(canonical)==json.loads((P/'ledger-before-second-lib.json').read_text())['canonical_summary_sha256'],'canonical summary changed')
    deadline(end-15)
    save(P,'driver-result.json',dict(status='GREEN',source=APP,pilot=cfg['pilot'],first=counts,second=counts2,sha256=elf_sha,deadline=end,elapsed=time.monotonic()-(end-3600),canonical_summary_unchanged=True))

def finalize(cfg,rc):
    outcome=dict(status='RED',returncode=rc)
    try:
        scopes=list(S.glob('validation-*/verdict.json'));require(len(scopes)==1,'one scope required')
        root=scopes[0].parent;verdict=json.loads(scopes[0].read_text());auth=json.loads((root/'autorite.json').read_text());end=auth['echeance']
        require(rc==0 and verdict['verdict']=='VERT' and verdict['code']==0 and not verdict['restes'] and verdict['nettoye'],'scope failed')
        require(auth['etat']=='termine' and not Path(verdict['prive']).exists(),'authority active or private bus remains')
        result=json.loads((P/'driver-result.json').read_text());require(result['status']=='GREEN','driver failed')
        require(json.loads((root/'recette.json').read_text())['delai']==3600,'final allocation is not distinct 3600')
        ledger=json.loads((P/'ledger-before-second-lib.json').read_text());require(sha(ledger['binary'])==sha(ledger['copy'])==ledger['sha256'],'ELF final hash differs')
        guard(cfg)
        runtime=json.loads((P/'sdk-exec-profile.json').read_text());require(not runtime['diff'] and runtime['binary_sha256']==ledger['sha256'] and runtime['actual']['CARGO_MANIFEST_DIR']==str(C),'final runtime snapshot differs')
        require(sha(P/'official/resume.log')==ledger['canonical_summary_sha256'],'final canonical summary changed')
        contract.check_same_short(json.loads((P/'short-tmp.json').read_text()))
        observed=scan(json.loads((root/'enfants.json').read_text()));save(P,'scan-final.json',observed)
        require(not observed['found'] and not observed['owned_alive'] and not observed['owned_unknown'],'remaining activity or owned UNKNOWN')
        require(not any(x['attributed'] for x in observed['unknown']),'attributed UNKNOWN')
        require(not (P/'first-red.json').exists(),'first red exists');deadline(end)
        outcome=dict(status='GREEN',returncode=0,source=APP,pilot=cfg['pilot'],scope=verdict,authority_terminated=True,bus_absent=True,deadline=end,elapsed=time.monotonic()-(end-3600))
        deadline(end) # Fresh, after scan and hashes, immediately before publication.
    except Exception as error:outcome=dict(status='RED',returncode=rc,error=repr(error))
    if outcome['status']=='GREEN' and time.monotonic()>=outcome['deadline']:outcome=dict(status='RED',error='deadline before publication')
    outcome=save(P,'composed-result.json',outcome,publication_deadline=outcome.get('deadline'));print(json.dumps(outcome));return 0 if outcome['status']=='GREEN' else 1

def launch(pilot):
    control=Path(__file__).resolve().parents[4]
    require(os.uname().nodename.lower()=='luciepc','qualification only on PC')
    require(re.fullmatch('[0-9a-f]{40}',pilot),'exact pilot SHA required')
    require(git(control,'rev-parse','HEAD')==pilot and not git(control,'status','--porcelain'),'pilot worktree differs')
    require(not any(p.exists() or p.is_symlink() for p in [P,Q,S]),'P/Q/S must be fresh; same prior target retained')
    require(git(W,'rev-parse','HEAD')==APP and not git(W,'status','--porcelain'),'source differs')
        # All prep below is still without Cargo; no service/manager health prerequisite.
    P.mkdir(mode=0o700);Q.mkdir(mode=0o700)
    shutil.copy2(__file__,Q/'qualify.py');shutil.copy2(C/'scripts/confiner_validation.py',Q/'confiner_validation.py');shutil.copy2(C/'scripts/poste',Q/'poste');shutil.copy2(Path(__file__).with_name('profile_contract.py'),Q/'profile_contract.py')
    for name in ['source_proof.py','cache_state.py']:shutil.copy2(Path(__file__).with_name(name),Q/name)
    cfg=dict(app=APP,base=BASE,pilot=pilot,control=str(control),root=str(R),worktree=str(W),target=str(T),copies={n:sha(Q/n) for n in ['qualify.py','confiner_validation.py','poste','profile_contract.py','source_proof.py','cache_state.py']})
    cfg['transition_sha256']=sha(source_proof.TRANSITION/'transition.json')
    carry=contract.capture(cfg,sha,git);save(P,'carry-before-poste.json',carry);contract.verify(carry,carry);cfg['carry']=carry
    save(Q,'configuration.json',cfg);guard(cfg)
    env={k:os.environ[k] for k in ['HOME','USER','LOGNAME','PATH','LANG','LC_ALL','CARGO_HOME','RUSTUP_HOME','XDG_RUNTIME_DIR','DBUS_SESSION_BUS_ADDRESS'] if k in os.environ}
    env.update(POSTE_MEM_MAX='32G',POSTE_SWAP_MAX='0',RAG3WEAVER_BUILD_MEMORY_HIGH='0',CARGO_BUILD_JOBS='8',RUST_TEST_THREADS='8')
    command=[str(Q/'poste'),'lourd','timeout','--signal=TERM','--kill-after=15s','3660s',sys.executable,str(Q/'confiner_validation.py'),'lancer','--delai','3600','--preuves',str(S),'--',sys.executable,str(Q/'qualify.py'),'--driver']
    save(P,'launch.json',dict(command=command,copies=cfg['copies']))
    with (P/'outer.log').open('x') as output:rc=subprocess.run(command,cwd='/var/tmp',env=env,stdout=output,stderr=subprocess.STDOUT).returncode
    return finalize(cfg,rc)

if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--launch');parser.add_argument('--driver',action='store_true');parser.add_argument('--exec-second',action='store_true');args=parser.parse_args()
    if args.exec_second:exec_second();sys.exit(0)
    if args.driver:
        try:driver()
        except BaseException as error:
            if P.exists() and not (P/'first-red.json').exists():save(P,'first-red.json',dict(observed=time.monotonic(),error=repr(error)))
            raise
    else:
        require(args.launch is not None,'explicit --launch EXACT_PILOT required');sys.exit(launch(args.launch))
