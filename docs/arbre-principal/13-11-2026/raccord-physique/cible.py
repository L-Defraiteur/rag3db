#!/usr/bin/env python3
"""One native CPU qualification; confinement owns the deadline, slot and private bus."""
from pathlib import Path
import hashlib, json, mmap, os, re, shutil, subprocess, sys, time
W=Path('/home/lucied/.cache/rag3weaver-build/cx-wt-raccord-physique')
C=W/'extension/rag3weaver'
T=Path('/home/lucied/.cache/rag3weaver-build/cx-target-contenu-derive')
P=Path('/home/lucied/.cache/rag3weaver-build/cx-raccord-physique-cible-v14')
NATIVE='rag3db-build-29a7bc52aaf11e6204f9beafaffb154105417e13-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF'
SOURCE=sys.argv[1]
CONTINUE='--existing-v3' in sys.argv[2:]
if CONTINUE:P=Path('/home/lucied/.cache/rag3weaver-build/cx-raccord-physique-complement-v3')
assert re.fullmatch('[0-9a-f]{40}',SOURCE)
AUTH=json.loads(Path(os.environ['RAG3WEAVER_VALIDATION_RECU']).read_text())
DEADLINE=min(AUTH['echeance'],887138.455654083) if CONTINUE else AUTH['echeance']; START=time.monotonic()
assert time.monotonic()<DEADLINE-10
P.mkdir(exist_ok=False)
for name in ['tmp','state','journaux','builtin_templates','elf','cache']: (P/name).mkdir()
E=os.environ.copy()
for key in list(E):
    if key.startswith(('RAG3WEAVER_SERVICE_','RAG3WEAVER_EMBED_')) or key in ('RAG3WEAVER_PG','GOOGLE_APPLICATION_CREDENTIALS','RAG3WEAVER_PHYSICAL_CRASH','RAG3WEAVER_PHYSICAL_CHILD_ROOT'):
        E.pop(key,None)
E.update(CARGO_TARGET_DIR=str(T),CARGO_BUILD_JOBS='8',CARGO_INCREMENTAL='0',RAG3DB_LIB=NATIVE,RAG3DB_LIBS_DIR='/home/lucied/.cache/rag3db-libs',RAG3WEAVER_MOTEUR_ANCIEN='1',RAG3DB_BUFFER_POOL_SIZE='536870912',
    LUCIVY_SCHEDULER_THREADS='8',CARGO_MANIFEST_DIR=str(C),TMPDIR=str(P/'tmp'),
    XDG_CACHE_HOME=str(P/'cache'),RAG3WEAVER_CACHE_DIR=str(P/'cache'),
    RAG3WEAVER_STATE_DIR=str(P/'state'),RAG3WEAVER_JOURNAUX_DIR=str(P/'journaux'),RAG3WEAVER_BUILTIN_TEMPLATES_DIR=str(P/'builtin_templates'))
for role in ['', '_EMBEDDER','_RERANKER','_OCR']: E['RAG3WEAVER_BURN_DEVICE'+role]='cpu'
CG=Path('/proc/self/cgroup').read_text().strip().split('::',1)[1]
G=Path('/sys/fs/cgroup'+CG)
assert (G/'memory.max').read_text().strip()=='34359738368'
assert (G/'memory.high').read_text().strip()=='max'
phases=[]; mappings={}; descendants={}
def save(name,value): (P/name).write_text(json.dumps(value,ensure_ascii=False,indent=2))
def sha(path):
    with Path(path).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def size(): return sum(p.stat().st_size for p in T.rglob('*') if p.is_file()) if T.exists() else 0
def observe(rootpid):
    table={}
    for q in Path('/proc').iterdir():
        if not q.name.isdigit():continue
        try:
            if q.stat().st_uid!=os.getuid():continue
            stat=(q/'stat').read_text().rsplit(') ',1)[1].split()
            table[int(q.name)]=(int(stat[1]),stat[19],stat[0])
        except (OSError,ProcessLookupError):continue
    owned={rootpid}
    while True:
        expanded=owned|{pid for pid,(ppid,_,_) in table.items() if ppid in owned}
        if expanded==owned:break
        owned=expanded
    for pid in owned:
        if pid not in table or table[pid][2]=='Z':continue
        try:
            q=Path('/proc')/str(pid);cg=(q/'cgroup').read_text().strip().split('::',1)[1]
            assert cg==CG or cg.startswith(CG+'/'),('escaped scope',pid,cg)
            exe=os.readlink(q/'exe'); descendants[(pid,table[pid][1])]=dict(pid=pid,starttime=table[pid][1],exe=exe,cgroup=cg)
            maps=(q/'maps').read_text();native=[line for line in maps.splitlines() if 'librag3db.so' in line]
            if native:
                assert all('/'+NATIVE+'/librag3db.so' in line for line in native),('wrong native library',native)
                if pid not in mappings:
                    name=f'maps-{pid}-{table[pid][1]}.txt';(P/name).write_text(maps)
                    mappings[pid]=dict(pid=pid,exe=exe,maps=name,native=native,cgroup=cg)
        except (FileNotFoundError,ProcessLookupError,PermissionError):continue

def run(label,argv,cwd=C):
    assert time.monotonic()<DEADLINE-3 and size()<=32*1024**3
    started=time.monotonic(); logfile=P/(label+'.log')
    with logfile.open('wb') as out:
        process=subprocess.Popen(argv,cwd=cwd,env=E,stdout=out,stderr=subprocess.STDOUT)
        try:
            while process.poll() is None:
                observe(process.pid)
                assert time.monotonic()<DEADLINE-2,'global deadline'
                time.sleep(.02)
            rc=process.wait()
        except BaseException:
            process.terminate();process.wait()
            raise
    phase=dict(label=label,argv=argv,cwd=str(cwd),returncode=rc,elapsed_s=time.monotonic()-started,log=logfile.name)
    phases.append(phase);save('phases.json',phases)
    assert rc==0,(label,rc,logfile)
    return logfile.read_text(errors='replace')

try:
    if CONTINUE:
        assert SOURCE=='10e6c75270a93c4db7a5d42c69c9dbd5d658f46e'
        assert not subprocess.check_output(['git','diff',SOURCE,'HEAD','--','extension/rag3weaver'],cwd=W)
    else:assert subprocess.check_output(['git','rev-parse','HEAD'],cwd=W,text=True).strip()==SOURCE
    assert not subprocess.check_output(['git','status','--porcelain'],cwd=W,text=True).strip()
    assert int(subprocess.check_output(['git','rev-list','--count','HEAD..origin/master'],cwd=W,text=True))<=200
    # Copy only tracked public templates from the qualified source tree.
    records=[]
    for path in sorted((C/'templates').rglob('*')):
        if not path.is_file():continue
        relative=path.relative_to(C/'templates');dest=P/'builtin_templates'/relative;dest.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(path,dest)
        records.append(dict(path=str(relative),sha256=sha(path),size=path.stat().st_size))
    assert records;save('templates.json',records)
    if CONTINUE:
        previous=Path('/home/lucied/.cache/rag3weaver-build/cx-raccord-physique-cible-v3')
        build=(previous/'build.log').read_text()
        attestation=json.loads(Path('/home/lucied/.cache/rag3weaver-build/cx-raccord-physique-attestation-v3.json').read_text())
        assert attestation['verdict']=='VERT_ATTESTATION_RO' and attestation['source']==SOURCE
    else:
        build=run('build',['bash','scripts/cargo_magasin.sh','test','--lib','--no-default-features','--features','rag3db-native,code','-j8','--no-run','--message-format=json'])
    artifacts=[]
    for line in build.splitlines():
        try:message=json.loads(line)
        except json.JSONDecodeError:continue
        if message.get('reason')=='compiler-artifact' and message.get('executable') and message.get('profile',{}).get('test') and message['target']['name']=='rag3weaver':artifacts.append(Path(message['executable']))
    assert len(set(artifacts))==1,artifacts
    binary=artifacts[0]
    if CONTINUE:assert sha(binary)==attestation['sha256']=='8387de3dd95ef2c3d290166d6d2049c0a4cf9f4cb29ace565da362b9b8a9426d'
    dest=P/'elf'/binary.name;shutil.copy2(binary,dest);assert sha(binary)==sha(dest)
    with binary.open('rb') as stream,mmap.mmap(stream.fileno(),0,access=mmap.ACCESS_READ) as data:
        assert data.find(SOURCE[:9].encode())>=0,'source revision absent from ELF'
        required_templates=['tools/search_base.mmd','search_expansion.mmd']
        for name in required_templates:
            payload=(C/'templates'/name).read_bytes()
            assert payload and data.find(payload)>=0,('required template absent from ELF',name)
    save('artifact.json',dict(source=SOURCE,binary=str(binary),copy=str(dest),sha256=sha(binary),templates_inventory=len(records),templates_required_embedded=required_templates))
    # Each direct ELF invocation exports the pinned SDK exactly like Cargo.
    wrapper=['bash','-c','source "$1"; exporter_la_lib || exit 2; shift; exec "$@"','native-export',str(C/'scripts/lib_du_magasin.sh'),str(binary)]
    listing=run('inventory',wrapper+['--list','--format','terse'],cwd=Path('/var/tmp'))
    expected={line.removesuffix(': test') for line in listing.splitlines() if line.endswith(': test') and line.startswith('dataflow::migrations::physical_index::tests::')}
    assert len(expected)==10,expected
    cpu=['lexical_catalog_has_no_dense_services_signature_or_debt','lexical_catalog_refuses_dense_declarations_before_registration','lexical_catalog_refuses_sparse_provider_before_initialization_effects','commit_unknown_invalidates_catalog_even_if_backend_poison_is_noop','committed_rows_with_failed_fts_never_announce_readiness_or_reuse_handle']
    for name in cpu:
        exact='catalog::tests::'+name
        assert exact+': test' in listing
        result=run(name,wrapper+[exact,'--exact','--nocapture','--test-threads=1'],cwd=Path('/var/tmp'))
        assert re.search(r'test result: ok\. 1 passed; 0 failed; 0 ignored;',result),name
    exact='buffered_blob_store::tests::invalidated_buffer_never_retries_persistence_in_drop'
    assert exact+': test' in listing
    result=run('invalidated-buffer-drop',wrapper+[exact,'--exact','--nocapture','--test-threads=1'],cwd=Path('/var/tmp'))
    assert re.search(r'test result: ok\. 1 passed; 0 failed; 0 ignored;',result)
    exact='connection::lifecycle::tests::revocation_preserves_backend_transactions_and_reaches_existing_readers'
    assert exact+': test' in listing
    result=run('connection-family-revocation',wrapper+[exact,'--exact','--nocapture','--test-threads=1'],cwd=Path('/var/tmp'))
    assert re.search(r'test result: ok\. 1 passed; 0 failed; 0 ignored;',result)
    for name in ['register_builtins_has_all_builtin_types','le_schema_declare_les_memes_ports_que_le_noeud']:
        exact='dataflow::node_factories::tests::'+name
        assert exact+': test' in listing
        result=run(name,wrapper+[exact,'--exact','--nocapture','--test-threads=1'],cwd=Path('/var/tmp'))
        assert re.search(r'test result: ok\. 1 passed; 0 failed; 0 ignored;',result)
    exact='cypher_blob_store::tests::read_only_blob_store_never_prepares_or_persists_writes'
    assert exact+': test' in listing
    result=run('read-only-blob-store',wrapper+[exact,'--exact','--nocapture','--test-threads=1'],cwd=Path('/var/tmp'))
    assert re.search(r'test result: ok\. 1 passed; 0 failed; 0 ignored;',result)
    native_results=[]
    for index,exact in enumerate(sorted(expected)):
        result=run(f'physical-{index:02}',wrapper+[exact,'--exact','--show-output','--test-threads=1'],cwd=Path('/var/tmp'))
        assert re.search(r'test result: ok\. 1 passed; 0 failed; 0 ignored;',result),exact
        assert exact in set(re.findall(r'^test (.*?)(?: - should panic)? \.\.\. ok$',result,re.M)),exact
        native_results.append(result)
    result='\n'.join(native_results)
    witnesses=[json.loads(line.split('PHYSICAL_WITNESS ',1)[1]) for line in result.splitlines() if 'PHYSICAL_WITNESS ' in line]
    assert {w['stage'] for w in witnesses}=={'normal-reopen','before-logical-commit','after-logical-commit','before-logical-commit:2','after-logical-commit:2','after-lexical-durability','after-publication'}
    assert all(w['occurrences']==8 and w['content_versions']==6 and w['dense_models']==0 for w in witnesses)
    bootstrap=[json.loads(line.split('BOOTSTRAP_WITNESS ',1)[1]) for line in result.splitlines() if 'BOOTSTRAP_WITNESS ' in line]
    assert {w['stage'] for w in bootstrap}=={'after-root-claim','after-bootstrap-initialize'}
    assert all(w['ordinary_opens_refused']==3 and w['ordinary_bytes_unchanged'] and not w['p0_minted'] for w in bootstrap)
    save('bootstrap-witnesses.json',bootstrap)
    assert mappings,'no native library mapping actually observed'
    assert size()<=32*1024**3
    save('witnesses.json',witnesses)
    save('result.json',dict(verdict='VERT',source=SOURCE,elapsed_s=time.monotonic()-START,deadline=DEADLINE,passed_functions=20,judged_scenarios=19,native_judges=9,native_helper_functions=1,child_death_runs=8,ignored=0,target_bytes=size(),native=NATIVE))
except BaseException as error:
    save('result.json',dict(verdict='ROUGE',source=SOURCE,error=repr(error),elapsed_s=time.monotonic()-START,deadline=DEADLINE))
    raise
finally:
    save('maps.json',list(mappings.values()));save('descendants.json',list(descendants.values()))
