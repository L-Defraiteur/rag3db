#!/usr/bin/env python3
"""Recette RO sous la primitive commune : une seule fenêtre, premier rouge arrêté, preuves hors target."""
from pathlib import Path
import ast, datetime, hashlib, json, mmap, os, re, shlex, shutil, signal, subprocess, sys, tempfile, time
R=Path('/home/lucied/.cache/rag3weaver-build'); W=R/'cx-wt-publication-composite'; C=W/'extension/rag3weaver'
P=R/'cx-passe-publication-composite-v1'; T=R/'cx-target-contenu-derive'
SOURCE=sys.argv[1]; RECEIPT=Path(os.environ['RAG3WEAVER_VALIDATION_RECU']); AUTH=json.loads(RECEIPT.read_text()); DEADLINE=AUTH['echeance']; INITIAL=DEADLINE-5400
F='rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres,openai-llm,anthropic-llm'
NATIVE='rag3db-build-29a7bc52aaf11e6204f9beafaffb154105417e13-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF'
PG='cx_pub_2191_20261113_v1'; ROLE='rag3weaver'
assert re.fullmatch('[0-9a-f]{40}',SOURCE) and time.monotonic()<DEADLINE-15
P.mkdir(exist_ok=False)
CG=Path('/proc/self/cgroup').read_text().strip().split('::',1)[1]; G=Path('/sys/fs/cgroup'+CG)
assert (G/'memory.max').read_text().strip()=='34359738368'
assert (G/'memory.high').read_text().strip()=='max'
E=os.environ.copy()
for key in list(E):
    if key.startswith(('RAG3WEAVER_SERVICE_', 'RAG3WEAVER_EMBED_')) or key in ('RAG3WEAVER_PG','RAG3WEAVER_STATE_DIR','RAG3WEAVER_GARDER','GOOGLE_APPLICATION_CREDENTIALS'):
        E.pop(key,None)
# Reprendre uniquement la connexion de fixture déjà publiée ; aucun .env ni base réelle.
old=ast.parse((R/'cx-passe-apercu-transfert-v1.py').read_text())
values=[ast.literal_eval(n.value) for n in old.body if isinstance(n,ast.Assign) and any(isinstance(t,ast.Name) and t.id=='PG_CONNECTION' for t in n.targets)]
assert len(values)==1
fields=dict(s.split('=',1) for s in shlex.split(values[0]))
assert fields['host'] in ('127.0.0.1','localhost') and fields['port']=='5433' and fields['user']==ROLE
fields['dbname']=PG
DSN=' '.join(k+'='+shlex.quote(v) for k,v in fields.items())
E.update(CARGO_MANIFEST_DIR=str(C),CARGO_TARGET_DIR=str(T),CARGO_BUILD_JOBS='8',CARGO_INCREMENTAL='0',LUCIVY_SCHEDULER_THREADS='8',RUST_TEST_THREADS='8',POSTE_MEM_MAX='32G',RAG3WEAVER_TARGET_MAX_GO='32',RAG3WEAVER_TARGET_REPRENDRE='1',RAG3DB_LIB=NATIVE,RAG3WEAVER_MOTEUR_ANCIEN='1',RAG3WEAVER_BUILD_MEMORY_HIGH='0',RAG3WEAVER_PG=DSN,RAG3WEAVER_PAQUETS_REACT=str(R/'I-paquets-react'),RAG3WEAVER_EXIGER_NAVIGATEUR='1',CHECK_FEATURES_LOGS=str(P/'features'),CHECK_FEATURES_TIMEOUT='1500',FEATURES=F,RAG3WEAVER_E2E_FEATURES=F,RAG3WEAVER_NAVIGATEUR='/usr/lib/chromium/chromium',RAG3WEAVER_JOURNAUX_DIR=str(P/'commandes'),RAG3WEAVER_BUILTIN_TEMPLATES_DIR=str(P/'builtin-cache'))
for suffix in ('','_EMBEDDER','_RERANKER','_OCR'): E['RAG3WEAVER_BURN_DEVICE'+suffix]='cpu'
assert all(isinstance(k,str) and isinstance(v,str) for k,v in E.items())
EXPORT=['bash','-c','. scripts/lib_du_magasin.sh; exporter_la_lib || exit 2; exec "$@"','native-test']
owned={}; seen={}; devices={}; mappings={}; artifacts={}; phases=[]; phase='preparation'; child=None; bus_tmp=None

def now():return datetime.datetime.now().astimezone().isoformat()
def save(name,obj):(P/name).write_text(json.dumps(obj,indent=2,ensure_ascii=False)+'\n')
def git(*args):return subprocess.check_output(['git',*args],cwd=W,text=True).strip()
def guard():
    assert git('rev-parse','HEAD')==SOURCE and not git('status','--porcelain'),'source changée/sale'
    for n in ('rebase-merge','rebase-apply'): assert not Path(git('rev-parse','--path-format=absolute','--git-path',n)).exists()
def events():return dict((k,int(v)) for k,v in (l.split() for l in (G/'memory.events').read_text().splitlines()))
def state(verdict,detail=''):
    save('etat.json',dict(source=SOURCE,verdict=verdict,phase=phase,detail=detail,time=now(),elapsed_s=time.monotonic()-INITIAL,remaining_s=DEADLINE-time.monotonic(),phases=phases))
def digest(path):
    with Path(path).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def observe():
    table={}
    for q in Path('/proc').iterdir():
        if not q.name.isdigit():continue
        try:
            if q.stat().st_uid!=os.getuid():continue
            st=(q/'stat').read_text().rsplit(') ',1)[1].split();table[int(q.name)]=(int(st[1]),st[19],st[0])
        except (OSError,ProcessLookupError):continue
    changed=True
    while changed:
        changed=False
        for pid,(ppid,stamp,_) in table.items():
            # PID réutilisé : ne devient nôtre que par une filiation observée vivante.
            parent_owned=ppid in owned and ppid in table and table[ppid][1]==owned[ppid]
            if (pid not in owned or owned[pid]!=stamp) and (ppid==os.getpid() or parent_owned):
                owned[pid]=stamp;changed=True
    live=[]
    for pid,stamp in list(owned.items()):
        if pid not in table or table[pid][1]!=stamp or table[pid][2]=='Z':continue
        live.append(pid)
        try:
            q=Path('/proc')/str(pid);cg=(q/'cgroup').read_text().strip().split('::',1)[1]
            maximum=(Path('/sys/fs/cgroup'+cg)/'memory.max').read_text().strip();exe=os.readlink(q/'exe')
            seen[(pid,stamp,cg,exe)]=dict(pid=pid,starttime=stamp,cgroup=cg,memory_max=maximum,exe=exe)
            assert cg==CG or cg.startswith(CG+'/'),('hors scope',pid,cg)
            assert maximum=='34359738368',('plafond enfant',pid,maximum)
            for fd in (q/'fd').iterdir():
                try:dest=os.readlink(fd)
                except OSError:continue
                if dest.startswith('/dev/dri/') or dest=='/dev/kfd':devices[(pid,stamp,dest)]=dict(pid=pid,starttime=stamp,device=dest,cgroup=cg)
            if exe.startswith(str(T/'debug/')) and (('/deps/rag3weaver-' in exe) or '/deps/e2e-' in exe or exe.endswith('/rag3weaver-backend')):
                maps=(q/'maps').read_text();rows=[r for r in maps.splitlines() if 'librag3db.so' in r]
                if rows:
                    assert all('/'+NATIVE+'/librag3db.so' in r for r in rows),('native inattendu',rows)
                    if (pid,stamp) not in mappings:
                        file=f'maps-{pid}-{stamp}.txt';(P/file).write_text(maps)
                        mappings[(pid,stamp)]=dict(pid=pid,starttime=stamp,exe=exe,maps=file,native=rows,cgroup=cg)
        except (FileNotFoundError,ProcessLookupError):continue
    return live

def scan_context():
    found=[]
    for q in Path('/proc').iterdir():
        if not q.name.isdigit() or int(q.name)==os.getpid():continue
        try:
            if q.stat().st_uid!=os.getuid():continue
            stat=(q/'stat').read_text().rsplit(') ',1)[1].split()
            if stat[0]=='Z':continue
            exe=os.readlink(q/'exe');cwd=os.readlink(q/'cwd');args=(q/'cmdline').read_bytes().split(b'\0')
            if exe.startswith(str(T)+'/') or cwd==str(W) or cwd.startswith(str(W)+'/') or (bus_tmp and any(str(bus_tmp).encode() in a for a in args)):
                found.append(dict(pid=int(q.name),starttime=stat[19],exe=exe,cwd=cwd))
        except (OSError,ProcessLookupError):continue
    return found

def observations():
    save('descendants.json',list(seen.values()));save('devices.json',list(devices.values()));save('maps.json',list(mappings.values()))
def stop_owned():
    try:observe()
    except BaseException as error:save('erreur-observation-arret.json',dict(error=repr(error)))
    for sig in (signal.SIGTERM,signal.SIGKILL):
        for pid,stamp in list(owned.items()):
            try:
                fd=os.pidfd_open(pid)
                try:
                    q=Path('/proc')/str(pid)
                    if (q/'stat').read_text().rsplit(') ',1)[1].split()[19]==stamp:signal.pidfd_send_signal(fd,sig)
                finally:os.close(fd)
            except (OSError,ProcessLookupError):pass
        time.sleep(.3)
    if child is not None:
        try:child.wait(timeout=3)
        except subprocess.TimeoutExpired:pass

def interrupted(signum,_):raise RuntimeError(f'interruption {signum}')
for sig in (signal.SIGTERM,signal.SIGINT,signal.SIGHUP):signal.signal(sig,interrupted)

def budget(seconds):return min(DEADLINE-15,time.monotonic()+seconds)
def run(name,argv,limit,extra=None):
    global phase,child
    phase=name;guard();state('EN_COURS');assert time.monotonic()<limit,('budget épuisé',name)
    env=E.copy();env.update(extra or {});env['RAG3WEAVER_STATE_DIR']=str(P/'etat-fixture')
    t0=time.monotonic();path=P/(name+'.log');print('DEBUT',name,now(),flush=True)
    with path.open('w') as out:
        child=subprocess.Popen(argv,cwd=C,env=env,stdout=out,stderr=subprocess.STDOUT)
        while True:
            observe()
            if child.poll() is not None:break
            assert time.monotonic()<limit,('timeout',name)
            if name=='features': assert not re.search(r'^ROUGE\b',path.read_text(errors='replace'),re.M),'premier rouge F5'
            time.sleep(.025)
        rc=child.wait();child=None
    item=dict(phase=name,rc=rc,elapsed_s=time.monotonic()-t0,argv=argv,deadline_monotonic=limit);phases.append(item)
    observations();state('EN_COURS')
    assert rc==0,(name,rc)
    assert time.monotonic()<limit,('dépassement',name)
    assert int(subprocess.check_output(['du','-sb',str(T)],text=True).split()[0])<=32*1024**3,'target >32G'
    guard();print('VERT',name,f'{item["elapsed_s"]:.2f}s',flush=True)
    return path.read_text(errors='replace')

def compile_elf(label,features,target,limit):
    args=['bash','scripts/cargo_magasin.sh','test','--features',features,'-j8',*target,'--no-run','--message-format=json']
    log=run(label+'-compile',args,limit);found=[];backend=[]
    for line in log.splitlines():
        try:a=json.loads(line)
        except ValueError:continue
        if a.get('reason')!='compiler-artifact' or not a.get('executable'):continue
        if a['target']['name']=='rag3weaver-backend':backend.append(a)
        if a['target']['name']==('e2e' if target==['--test','e2e'] else 'rag3weaver') and a['profile']['test']:found.append(a)
    assert len(found)==1,('artefact unique',label,len(found))
    a=found[0];assert a['target']['src_path']==str(C/('tests/e2e/main.rs' if target==['--test','e2e'] else 'src/lib.rs'))
    binary=Path(a['executable']);assert binary.is_relative_to(T) and binary.is_file()
    assert set(features.split(',')).issubset(a['features'])
    artifacts[label]=dict(actual_path=str(binary),features=a['features'],cargo_artifact=a)
    if target==['--test','e2e']:
        assert len(backend)==1,('backend CLI',len(backend))
        artifacts['backend']=dict(actual_path=backend[0]['executable'],cargo_artifact=backend[0])
    save('artefacts.json',artifacts)
    return binary

def inventory(label,binary,limit,filters=()):
    base=EXPORT+[str(binary),*filters]
    listed=run(label+'-liste',base+['--list','--format','terse'],limit)
    ignored=run(label+'-ignores',base+['--list','--ignored','--format','terse'],limit)
    names=re.findall(r'^(.*): test$',listed,re.M);ignores=re.findall(r'^(.*): test$',ignored,re.M)
    assert names and len(names)==len(set(names)) and set(ignores).issubset(names)
    save(label+'-inventaire.json',dict(names=names,ignored=ignores,count=len(names),read_before_runtime=True))
    return names,ignores

def runtime(label,binary,names,ignores,limit,filters=(),extra=None):
    text=run(label,EXPORT+[str(binary),*filters,'--test-threads=8'],limit,extra)
    counts=re.findall(r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;',text)
    expected=(len(names)-len(ignores),0,len(ignores))
    assert counts and tuple(map(int,counts[-1]))==expected,(label,counts,expected)
    successes=set(re.findall(r'^test (.*?)(?: - should panic)? \.\.\. ok$',text,re.M))
    assert set(names)-set(ignores)<=successes,('témoins absents',label)
    assert not re.search(r'\b(?:SAUTÉ|SKIPPED)\b',text),'saut de témoin'
    phases[-1].update(passed=expected[0],failed=0,ignored=expected[2]);state('EN_COURS')
    return text

def attest(limit):
    global phase
    phase='attestation';state('EN_COURS');guard()
    files=sorted(p for p in (C/'templates').rglob('*') if p.is_file());assert len(files)==220
    template_records=[dict(path=str(p.relative_to(C)),sha256=digest(p),size=p.stat().st_size) for p in files]
    for label,a in artifacts.items():
        assert time.monotonic()<limit,'budget attestation'
        b=Path(a['actual_path']);assert b.is_relative_to(T)
        with b.open('rb') as handle,mmap.mmap(handle.fileno(),0,access=mmap.ACCESS_READ) as data:
            assert data.find(SOURCE[:9].encode())>=0,('source embarquée',label)
            missing=[str(p.relative_to(C)) for p in files if data.find(p.read_bytes())<0]
            assert not missing,('templates absents',label,missing)
        a.update(sha256=digest(b),size=b.stat().st_size,templates=220,source=SOURCE)
        assert any(m['exe']==str(b) for m in mappings.values()),('maps runtime manquantes',label)
        shutil.copy2(b,P/('binaire-'+label))
        assert digest(P/('binaire-'+label))==a['sha256']
        observe()
    save('artefacts.json',artifacts);save('templates-220.json',template_records)
    native=Path.home()/'.cache/rag3db-libs'/NATIVE/'librag3db.so'
    save('native.json',dict(path=str(native),sha256=digest(native)))
    assert time.monotonic()<limit,'budget attestation'

before=events();verdict='ROUGE'
try:
    guard();assert E['DBUS_SESSION_BUS_ADDRESS'],'bus privé commun manquant'
    assert C.resolve()==C and (C/'Cargo.toml').is_file()
    assert any(l.startswith('+ ') for l in git('cherry','origin/master','HEAD').splitlines()),'lot déjà intégré'
    assert int(git('rev-list','--count','HEAD..origin/master'))<=200
    save('source.json',dict(source=SOURCE,tree=git('rev-parse','HEAD^{tree}'),origin_master_observed_ref=git('rev-parse','origin/master'),started=now(),initial_monotonic=INITIAL,deadline_monotonic=DEADLINE,global_budget_s=5400,cgroup=CG,supervision_receipt=str(RECEIPT),driver_sha256=digest(Path(__file__)),native=NATIVE,features=F,private_pg=PG,cpu_roles=['common','embedder','reranker','ocr'],services='absent',phase_caps=dict(features=1500,minimal=120,compile_large=1200,lib=300,nav=360,cli=180,pg=300,attestation=120,tx_per_suite=900)))
    state('EN_COURS');limit=budget(120)
    run('supervision',['python3','scripts/confiner_validation.py','verifier'],limit)
    assert Path(E['RAG3WEAVER_PAQUETS_REACT']).is_dir()
    inspected=run('pg-serveur',['docker','inspect','--format','{{.State.Running}} {{json .NetworkSettings.Ports}}','rag3weaver-pg'],limit)
    assert inspected.startswith('true ') and '5433' in inspected
    pg=['docker','exec','rag3weaver-pg','psql','-U',ROLE,'-v','ON_ERROR_STOP=1','-At']
    absence=run('pg-absence',pg+['-d','postgres','-c',f"SELECT 1 FROM pg_database WHERE datname='{PG}';"],limit)
    assert not absence.strip(),'PG synthétique déjà présente, aucun reset autorisé'
    run('pg-creation',['docker','exec','rag3weaver-pg','createdb','-U',ROLE,'-O',ROLE,'--',PG],limit)
    identity=run('pg-identite',pg+['-d',PG,'-c',"SELECT current_database() || '|' || current_user;"],limit)
    assert identity.strip()==PG+'|'+ROLE
    run('pg-propriete',pg+['-d',PG,'-c',f"COMMENT ON DATABASE {PG} IS 'owned by arbre 2191 source {SOURCE}';"],limit)
    save('base-privee.json',dict(name=PG,owner=ROLE,source=SOURCE,current_database=identity.strip(),preserved_for_review=True))
    run('target',['bash','-c','. scripts/garde_target.sh; garder_le_target "$CARGO_TARGET_DIR" "$PWD"'],limit)
    features=run('features',['bash','scripts/check_features.sh'],budget(1500));assert 'CHECK_FEATURES VERT 5/5' in features
    run('minimal',['bash','scripts/cargo_magasin.sh','check','--lib','--no-default-features','-j8'],budget(120))
    limit=budget(1200);lib=compile_elf('lib',F,['--lib'],limit);names,ignores=inventory('lib',lib,limit)
    groups={prefix:len([n for n in names if n.startswith(prefix)]) for prefix in ['storage_inspection::observation_tests::','catalog::metadata::tests::','dataflow::migrations::facts::tests::','dataflow::migrations::transfer::tests::']}
    assert list(groups.values())==[10,3,18,15],groups
    publication=[n for n in names if n.startswith('dataflow::migrations::publication::')]
    assert len(publication)==12 and not set(publication)&set(ignores),publication
    required=publication+[n for n in names if n.startswith(tuple(groups))]+['storage_inspection::native_tests::native_snapshot_restores_configs_relations_and_journal_without_changing_fixture_bytes','dataflow::record_nodes::tests::l_annulation_d_un_lot_de_liens_ne_retire_que_ce_qu_il_a_cree']
    assert set(required)<=set(names) and not set(required)&set(ignores)
    save('groupes-requis.json',dict(groups=groups,required=required))
    runtime('lib',lib,names,ignores,budget(300))
    limit=budget(360);nav=compile_elf('navigateur',F+',navigateur',['--lib'],limit)
    short=set()
    for src in (C/'src').rglob('*.rs'):
        lines=src.read_text().splitlines()
        for i,line in enumerate(lines):
            if 'feature = "navigateur"), ignore' in line:short.update(re.findall(r'fn ([a-z0-9_]+)\(', '\n'.join(lines[i:i+5])))
    filters=sorted(short);assert filters
    navnames,navignores=inventory('navigateur',nav,limit,filters);assert len(navnames)==40 and not navignores
    bus_tmp=Path(tempfile.mkdtemp(prefix='cx-pub.',dir='/var/tmp'));(bus_tmp/'.possede-publication-composite').write_text(SOURCE+'\n')
    save('tmp-navigateur.json',dict(path=str(bus_tmp),source=SOURCE,minted=True,storage='btrfs /var/tmp',bus='unique commun à la validation entière'))
    runtime('navigateur',nav,navnames,navignores,limit,filters,dict(TMPDIR=str(bus_tmp)))
    assert any(r['exe']=='/usr/lib/chromium/chromium' for r in seen.values())
    live=observe();context=scan_context();save('nav-quiescence.json',dict(live=live,context=context,checked=now()));assert not live and not context
    limit=budget(180);cli=compile_elf('cli',F,['--test','e2e'],limit)
    clinames,cliignores=inventory('cli',cli,limit,['e2e_migration_preflight::']);assert len(clinames)==5 and not cliignores
    pgnames,pgignores=inventory('postgres',cli,limit,['e2e_postgres::']);assert len(pgnames)==36
    runtime('cli',cli,clinames,cliignores,limit,['e2e_migration_preflight::'])
    limit=budget(300)
    pgidentity=run('pg-avant-suite',pg+['-d',PG,'-c',"SELECT current_database() || '|' || current_user;"],limit);assert pgidentity.strip()==PG+'|'+ROLE
    pglog=run('postgres',['bash','run_e2e.sh','--test','e2e_postgres'],limit)
    assert re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',pglog)[-1]==('36','0','0')
    assert 'Suites jouées : 1 sur 1 demandées.' in pglog and not re.search(r'\b(?:SAUTÉ|SKIPPED)\b',pglog)
    actual=re.findall(r'▸ binaire e2e : (.+?) \(',pglog);assert actual==[str(cli)],('ELF PG différent de celui inventorié',actual)
    for suite in ('e2e_tx_ligne_a_ligne','e2e_tx_merge_puis_copy','e2e_tx_par_paquet_arret','e2e_tx_vecteurs_apres_rollback'):
        limit=budget(900)
        txnames,txignores=inventory(suite,cli,limit,[suite+'::'])
        txlog=run(suite,['bash','run_e2e.sh','--test',suite],limit)
        assert re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',txlog)[-1]==(str(len(txnames)),'0','0')
        assert 'Suites jouées : 1 sur 1 demandées.' in txlog and not re.search(r'\b(?:SAUTÉ|SKIPPED)\b',txlog)
        assert re.findall(r'▸ binaire e2e : (.+?) \(',txlog)==[str(cli)]
    attest(budget(120));after=events();save('memoire.json',dict(before=before,after=after))
    assert all(after[k]==before[k] for k in ('high','max','oom','oom_kill'))
    live=observe();context=scan_context();save('quiescence.json',dict(live=live,context=context,checked=now()));assert not live and not context
    observations();guard();assert time.monotonic()<DEADLINE-10
    phase='fin';verdict='VERT'
except BaseException as error:
    try:stop_owned()
    except BaseException as e:save('erreur-arret.json',dict(error=repr(e)))
    state('ROUGE',str(error));save('erreur.json',dict(error=repr(error),phase=phase));print('ROUGE',phase,str(error),flush=True)
finally:
    observations();save('verdict-detaille.json',dict(source=SOURCE,verdict=verdict,phase=phase,elapsed_s=time.monotonic()-INITIAL,deadline_monotonic=DEADLINE,phases=phases))
    (P/'VERDICT').write_text(verdict+'\n')
    if verdict=='VERT':state('VERT')
    print(verdict,phase,now(),flush=True)
sys.exit(0 if verdict=='VERT' else 1)
