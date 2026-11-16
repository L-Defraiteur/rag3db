#!/usr/bin/env python3
"""One fresh allocation: canonical pass, then the identical large unit-test ELF."""
from pathlib import Path
import argparse, hashlib, json, os, re, shutil, stat, subprocess, sys, time

APP = 'ff37eb82e52aa733503f3bf60fbf7a4d052ead85'
BASE = 'f83c6c4dd6a40200883c6325326f093ecdba3e23'
REL = 'docs/memoire/16-11-2026/script-fixtures-qualification/qualify.py'
PIN = 'rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF'
SO = {'librag3db.so':'b7e9fc75a3d2de04c72680b0309e8bb1df9eaadb5211ebbacf39716c6d0a2101',
      'libvector.rag3db_extension':'1669d6392ec8067fcf92ae2ccc972fd7cd3086629fb82d1f5d254e90d72d7561'}
PASS_SHA = '2150fcd847b1df7de04908d357a0c78abc05e50ce599e2793ff08ab48108825d'
FEATURES = 'rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres,openai-llm,anthropic-llm'
R = Path.home()/'.cache/rag3weaver-build'
W = Path.home()/'git_workspaces/cx-memoire-script-fixtures-r3'
C = W/'extension/rag3weaver'
T = R/'target-cx-memoire-script-fixtures'
P = R/'cx-memoire-script-fixtures-qualification-v1'
Q = R/'cx-memoire-script-fixtures-controller-v1'
S = R/'cx-memoire-script-fixtures-scope-v1'


def sha(path):
    with Path(path).open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()

def git(root,*args):return subprocess.check_output(['git','-C',str(root),*args],text=True).strip()
def save(root,name,value):
    with (root/name).open('x') as stream:
        json.dump(value,stream,indent=2);stream.write('\n');stream.flush();os.fsync(stream.fileno())

def require(condition,message):
    if not condition:raise RuntimeError(message)

def deadline(end,now=None):
    require((time.monotonic() if now is None else now)<end,'allocation deadline reached')

def summary(text):
    rows=re.findall(r'^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;.*? (\d+) filtered out;',text,re.M)
    require(len(rows)==1,'expected exactly one successful test summary')
    passed,failed,ignored,filtered=map(int,rows[0]);require(passed>0 and failed==0,'no passing tests or failed tests')
    return dict(passed=passed,failed=failed,ignored=ignored,filtered=filtered)

def large_elf(text,target):
    rows=re.findall(r'Running unittests src/lib\.rs \(([^\n]+)\)',text)
    require(len(rows)==1,'expected one large unit-test ELF')
    binary=Path(rows[0]);require(binary.is_absolute() and binary.is_relative_to(target),'ELF outside private target')
    require(binary.resolve()==binary and binary.is_file(),'ELF path not canonical')
    with binary.open('rb') as stream:require(stream.read(4)==b'\x7fELF','not an ELF')
    return binary

def first_red(text):
    return re.search(r'^test [^\n]* \.\.\. FAILED\b|^\s*FAILED\s*$|^test result: FAILED\.',text,re.M)

def guard(cfg):
    for root,head in [(W,APP),(Path(cfg['control']),cfg['pilot'])]:
        require(git(root,'rev-parse','HEAD')==head and not git(root,'status','--porcelain'),'source/control changed')
    require(git(W,'merge-base',APP,'origin/master')==BASE,'master base changed since source freeze')
    sub=C/'codeparsers'
    require(git(sub,'rev-parse','HEAD')==git(W,'ls-tree',APP,'extension/rag3weaver/codeparsers').split()[2] and not git(sub,'status','--porcelain'),'gitlink mismatch/dirty')
    for name,value in cfg['copies'].items():require(sha(Q/name)==value,'controller copy changed: '+name)
    require(sha(R/'passe_epinglee.sh')==PASS_SHA and sha(W/'tools/poste/passe_epinglee.sh')==PASS_SHA,'canonical pass differs')
    for name,value in SO.items():require(sha(R.parent/'rag3db-libs'/PIN/name)==value,'SDK changed')

def scan(owned=()):
    # Unknown global observations are reported; known owned identities fail closed.
    roots=[W,T];found=[];unknown=[];alive=[];owned_unknown=[]
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

def run(label,args,env,end,cap):
    limit=min(end-30,time.monotonic()+cap);deadline(limit)
    with (P/(label+'.log')).open('x') as output:
        child=subprocess.Popen(args,cwd=C,env=env,stdout=output,stderr=subprocess.STDOUT)
        while True:
            # Inspect only newly generated logs; expected should_panic is not a failure.
            bad=[]
            for path in P.rglob('*.log'):
                if first_red(path.read_text(errors='replace')):bad.append(str(path))
            if bad:
                if not (P/'first-red.json').exists():save(P,'first-red.json',dict(label=label,logs=bad,observed=time.monotonic()))
                raise RuntimeError('first unexpected failing test: '+str(bad))
            deadline(limit)
            if child.poll() is not None:break
            require(int(subprocess.check_output(['du','-sb',str(T)],text=True).split()[0])<=32*1024**3,'target over 32GiB') if T.exists() else None
            time.sleep(.2)
        require(child.wait()==0,label+' returned nonzero')
    deadline(limit);return (P/(label+'.log')).read_text(errors='replace')

def driver():
    cfg=json.loads((Q/'configuration.json').read_text());guard(cfg)
    subprocess.run([sys.executable,str(Q/'confiner_validation.py'),'verifier'],check=True)
    receipt=Path(os.environ['RAG3WEAVER_VALIDATION_RECU']);auth=json.loads(receipt.read_text());end=auth['echeance']
    recipe=json.loads((receipt.parent/'recette.json').read_text());require(recipe['delai']==7200 and auth['etat']=='actif','not fresh 7200 authority')
    group=Path('/sys/fs/cgroup')/auth['cgroup'].lstrip('/')
    selected={name:(group/name).read_text().strip() for name in ['memory.max','memory.high','memory.swap.max','cpu.max']}
    require(selected['memory.max']=='34359738368' and selected['memory.high']=='max' and selected['memory.swap.max']=='0','memory contract differs')
    require(selected['cpu.max'].split()[0]!='max','CPU quota absent')
    save(P,'authority-path.json',dict(path=str(receipt),initial=end-7200,deadline=end,limits=selected))
    require(not T.exists() and not T.is_symlink(),'target must start absent')
    observed=scan();save(P,'scan-before-target.json',observed);require(not observed['found'],'source/target in use')
    env={k:os.environ[k] for k in ['HOME','USER','LOGNAME','PATH','LANG','LC_ALL','CARGO_HOME','RUSTUP_HOME','DBUS_SESSION_BUS_ADDRESS','RAG3WEAVER_VALIDATION_RECU','RAG3WEAVER_VALIDATION_JETON'] if k in os.environ}
    for name in ['tmp','cache','config','data','xdg-state','state-fixture']:(P/name).mkdir()
    env.update(CARGO_TARGET_DIR=str(T),CIBLE=T.name,CARGO_BUILD_JOBS='8',CARGO_INCREMENTAL='0',CARGO_TERM_COLOR='never',RUST_TEST_THREADS='8',
        TMPDIR=str(P/'tmp'),XDG_CACHE_HOME=str(P/'cache'),XDG_CONFIG_HOME=str(P/'config'),XDG_DATA_HOME=str(P/'data'),XDG_STATE_HOME=str(P/'xdg-state'),RAG3WEAVER_STATE_DIR=str(P/'state-fixture'),
        POSTE_MEM_MAX='32G',POSTE_SWAP_MAX='0',RAG3WEAVER_BUILD_MEMORY_HIGH='0',RAG3WEAVER_TARGET_MAX_GO='32',RAG3DB_LIB=PIN,RAG3DB_LIBS_DIR=str(R.parent/'rag3db-libs'),RAG3DB_BUFFER_POOL_SIZE='1073741824',
        RAG3WEAVER_EXIGER_NAVIGATEUR='1',RAG3WEAVER_PAQUETS_REACT=str(R/'I-paquets-react'),SUITE_DU_TEMOIN='lib',SUITES='-')
    for suffix in ['','_EMBEDDER','_RERANKER','_OCR']:env['RAG3WEAVER_BURN_DEVICE'+suffix]='cpu'
    # Match the unchanged canonical pass's PC profile for the second direct invocation.
    env.update(RAG3WEAVER_EMBED_SERVICE='127.0.0.1:7878,127.0.0.1:7879',RAG3WEAVER_PG='postgres://rag3weaver:rag3weaver@127.0.0.1:5433/rag3weaver_test',RAG3WEAVER_TESTS_REPLI_PERIPHERIQUE='igpu:0',RAG3WEAVER_TARGET_REPRENDRE='1')
    require(not Path(env['XDG_STATE_HOME']).is_relative_to(Path(env['RAG3WEAVER_STATE_DIR'])) and not Path(env['RAG3WEAVER_STATE_DIR']).is_relative_to(Path(env['XDG_STATE_HOME'])),'state roots overlap')
    save(P,'profile.json',{k:v for k,v in env.items() if k!='RAG3WEAVER_VALIDATION_JETON'})
    run('canonical-pass',['bash',str(R/'passe_epinglee.sh'),W.name,str(P.relative_to(R)/'official'),'codex_threads'],env,end,6600)
    canonical=P/'official/resume.log';require(canonical.read_text().splitlines()[-1]=='VERT','canonical summary not green')
    first=(P/'official/lib.log').read_text();counts=summary(first);require(counts['filtered']==0,'first lib filtered')
    binary=large_elf(first,T);copied=P/'large-unit-tests';shutil.copy2(binary,copied);elf_sha=sha(binary);require(sha(copied)==elf_sha,'ELF copy differs')
    save(P,'ledger-before-second-lib.json',dict(source=APP,pilot=cfg['pilot'],binary=str(binary),copy=str(copied),sha256=elf_sha,features=FEATURES,first=counts,canonical_summary_sha256=sha(canonical),meaning='hash after canonical pass and before second invocation, not pre-first-test attestation'))
    guard(cfg);deadline(end-30)
    script='. "$1"; exporter_la_lib || exit 2; shift; exec "$@"'
    second=run('second-lib',['bash','-c',script,'second-lib',str(C/'scripts/lib_du_magasin.sh'),str(binary)],env,end,600)
    counts2=summary(second);require(counts2==counts and counts2['filtered']==0,'second lib counts differ or filtered')
    require(sha(binary)==sha(copied)==elf_sha,'large ELF changed')
    guard(cfg);require(sha(canonical)==json.loads((P/'ledger-before-second-lib.json').read_text())['canonical_summary_sha256'],'canonical summary changed')
    deadline(end-15)
    save(P,'driver-result.json',dict(status='GREEN',source=APP,pilot=cfg['pilot'],first=counts,second=counts2,sha256=elf_sha,deadline=end,elapsed=time.monotonic()-(end-7200),canonical_summary_unchanged=True))

def finalize(cfg,rc):
    outcome=dict(status='RED',returncode=rc)
    try:
        scopes=list(S.glob('validation-*/verdict.json'));require(len(scopes)==1,'one scope required')
        root=scopes[0].parent;verdict=json.loads(scopes[0].read_text());auth=json.loads((root/'autorite.json').read_text());end=auth['echeance']
        require(rc==0 and verdict['verdict']=='VERT' and verdict['code']==0 and not verdict['restes'] and verdict['nettoye'],'scope failed')
        require(auth['etat']=='termine' and not Path(verdict['prive']).exists(),'authority active or private bus remains')
        result=json.loads((P/'driver-result.json').read_text());require(result['status']=='GREEN','driver failed')
        ledger=json.loads((P/'ledger-before-second-lib.json').read_text());require(sha(ledger['binary'])==sha(ledger['copy'])==ledger['sha256'],'ELF final hash differs')
        guard(cfg)
        observed=scan(json.loads((root/'enfants.json').read_text()));save(P,'scan-final.json',observed)
        require(not observed['found'] and not observed['owned_alive'] and not observed['owned_unknown'],'remaining activity or owned UNKNOWN')
        require(not any(x['attributed'] for x in observed['unknown']),'attributed UNKNOWN')
        require(not (P/'first-red.json').exists(),'first red exists');deadline(end)
        outcome=dict(status='GREEN',returncode=0,source=APP,pilot=cfg['pilot'],scope=verdict,authority_terminated=True,bus_absent=True,deadline=end,elapsed=time.monotonic()-(end-7200))
        deadline(end) # Fresh, after scan and hashes, immediately before publication.
    except Exception as error:outcome=dict(status='RED',returncode=rc,error=repr(error))
    if outcome['status']=='GREEN' and time.monotonic()>=outcome['deadline']:outcome=dict(status='RED',error='deadline before publication')
    save(P,'composed-result.json',outcome);print(json.dumps(outcome));return 0 if outcome['status']=='GREEN' else 1

def launch(pilot):
    control=Path(__file__).resolve().parents[4]
    require(os.uname().nodename.lower()=='luciepc','qualification only on PC')
    require(re.fullmatch('[0-9a-f]{40}',pilot),'exact pilot SHA required')
    require(git(control,'rev-parse','HEAD')==pilot and not git(control,'status','--porcelain'),'pilot worktree differs')
    require(not any(p.exists() or p.is_symlink() for p in [P,Q,S,T]),'P/Q/S/target must be fresh')
    require(git(W,'rev-parse','HEAD')==APP and not git(W,'status','--porcelain'),'source differs')
    # All prep below is still without Cargo; no service/manager health prerequisite.
    P.mkdir(mode=0o700);Q.mkdir(mode=0o700)
    shutil.copy2(__file__,Q/'qualify.py');shutil.copy2(C/'scripts/confiner_validation.py',Q/'confiner_validation.py');shutil.copy2(C/'scripts/poste',Q/'poste')
    cfg=dict(app=APP,base=BASE,pilot=pilot,control=str(control),target=str(T),copies={n:sha(Q/n) for n in ['qualify.py','confiner_validation.py','poste']})
    save(Q,'configuration.json',cfg);guard(cfg)
    env={k:os.environ[k] for k in ['HOME','USER','LOGNAME','PATH','LANG','LC_ALL','CARGO_HOME','RUSTUP_HOME','XDG_RUNTIME_DIR','DBUS_SESSION_BUS_ADDRESS'] if k in os.environ}
    env.update(POSTE_MEM_MAX='32G',POSTE_SWAP_MAX='0',RAG3WEAVER_BUILD_MEMORY_HIGH='0',CARGO_BUILD_JOBS='8',RUST_TEST_THREADS='8')
    command=[str(Q/'poste'),'lourd','timeout','--signal=TERM','--kill-after=15s','7260s',sys.executable,str(Q/'confiner_validation.py'),'lancer','--delai','7200','--preuves',str(S),'--',sys.executable,str(Q/'qualify.py'),'--driver']
    save(P,'launch.json',dict(command=command,copies=cfg['copies']))
    with (P/'outer.log').open('x') as output:rc=subprocess.run(command,cwd='/var/tmp',env=env,stdout=output,stderr=subprocess.STDOUT).returncode
    return finalize(cfg,rc)

if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--launch');parser.add_argument('--driver',action='store_true');args=parser.parse_args()
    if args.driver:
        try:driver()
        except BaseException as error:
            if P.exists() and not (P/'first-red.json').exists():save(P,'first-red.json',dict(observed=time.monotonic(),error=repr(error)))
            raise
    else:
        require(args.launch is not None,'explicit --launch EXACT_PILOT required');sys.exit(launch(args.launch))
