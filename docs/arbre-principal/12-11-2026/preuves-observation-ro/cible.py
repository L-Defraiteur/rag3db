from pathlib import Path
import hashlib,json,mmap,os,re,shutil,subprocess,sys,time
R=Path('/home/lucied/.cache/rag3weaver-build');W=R/'cx-wt-observation-faits';C=W/'extension/rag3weaver';T=R/'cx-target-contenu-derive';P=R/'cx-observation-ro-cible-v1'
SOURCE=sys.argv[1];FILTERS=['storage_inspection::','catalog::metadata::tests::','dataflow::migrations::']
P.mkdir(exist_ok=False)
receipt=json.loads(Path(os.environ['RAG3WEAVER_VALIDATION_RECU']).read_text());END=receipt['echeance']-5
E=os.environ.copy()
for k in list(E):
 if k.startswith(('RAG3WEAVER_SERVICE_','RAG3WEAVER_EMBED_')) or k in ('RAG3WEAVER_PG','GOOGLE_APPLICATION_CREDENTIALS','RAG3WEAVER_GARDER'):E.pop(k,None)
NATIVE='rag3db-build-29a7bc52aaf11e6204f9beafaffb154105417e13-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF'
E.update(CARGO_MANIFEST_DIR=str(C),CARGO_TARGET_DIR=str(T),CARGO_BUILD_JOBS='8',CARGO_INCREMENTAL='0',LUCIVY_SCHEDULER_THREADS='8',RUST_TEST_THREADS='8',RAG3WEAVER_TARGET_MAX_GO='32',RAG3WEAVER_TARGET_REPRENDRE='1',RAG3DB_LIB=NATIVE,RAG3WEAVER_MOTEUR_ANCIEN='1',RAG3WEAVER_STATE_DIR=str(P/'etat'),RAG3WEAVER_JOURNAUX_DIR=str(P/'commandes'),RAG3WEAVER_BUILTIN_TEMPLATES_DIR=str(P/'builtin-cache'))
for suffix in ('','_EMBEDDER','_RERANKER','_OCR'):E['RAG3WEAVER_BURN_DEVICE'+suffix]='cpu'
EXPORT=['bash','-c','. scripts/lib_du_magasin.sh; exporter_la_lib || exit 2; exec "$@"','native-test']
record=dict(source=SOURCE,phases=[],verdict='ROUGE',deadline_monotonic=receipt['echeance'])
def git(*args):return subprocess.check_output(['git',*args],cwd=W,text=True).strip()
def save(name,obj):(P/name).write_text(json.dumps(obj,indent=2)+'\n')
def guard():assert git('rev-parse','HEAD')==SOURCE and not git('status','--porcelain'),'source modifiée/sale'
def run(name,args,expected=0):
 guard();assert time.monotonic()<END
 with (P/(name+'.log')).open('w') as out:
  proc=subprocess.run(args,cwd=C,env=E,stdout=out,stderr=subprocess.STDOUT,timeout=max(.1,END-time.monotonic()))
 record['phases'].append(dict(name=name,argv=args,rc=proc.returncode));assert proc.returncode==expected,(name,proc.returncode,expected)
 guard();return (P/(name+'.log')).read_text(errors='replace')
try:
 guard();save('source.json',dict(source=SOURCE,tree=git('rev-parse','HEAD^{tree}'),native=NATIVE,features='rag3db-native,code',driver_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),receipt=receipt,filters=FILTERS,expected_rc=0,synthetic_metadata_only=True))
 run('target',['bash','-c','. scripts/garde_target.sh; garder_le_target "$CARGO_TARGET_DIR" "$PWD"'])
 log=run('compile',['bash','scripts/cargo_magasin.sh','test','--lib','--no-default-features','--features','rag3db-native,code','-j8','--no-run','--message-format=json'])
 artifacts=[]
 for line in log.splitlines():
  try:a=json.loads(line)
  except ValueError:continue
  if a.get('reason')=='compiler-artifact' and a.get('target',{}).get('name')=='rag3weaver' and a.get('profile',{}).get('test') and a.get('executable'):artifacts.append(a)
 assert len(artifacts)==1
 a=artifacts[0];b=Path(a['executable']);assert b.is_relative_to(T) and {'rag3db-native','code'}<=set(a['features'])
 shutil.copy2(b,P/'binaire-tests')
 with b.open('rb') as f:sha=hashlib.file_digest(f,'sha256').hexdigest()
 with b.open('rb') as f,mmap.mmap(f.fileno(),0,access=mmap.ACCESS_READ) as raw:assert raw.find(SOURCE[:9].encode())>=0
 save('binaire.json',dict(path=str(b),sha256=sha,features=a['features'],source=SOURCE))
 listed=run('inventaire',EXPORT+[str(b),*FILTERS,'--list','--format','terse'])
 names=re.findall(r'^(.*): test$',listed,re.M);assert names and len(names)==len(set(names))
 ignored=run('ignores',EXPORT+[str(b),*FILTERS,'--list','--ignored','--format','terse'])
 assert not re.findall(r'^(.*): test$',ignored,re.M)
 counts={prefix:len([n for n in names if n.startswith(prefix)]) for prefix in ['storage_inspection::observation_tests::','catalog::metadata::tests::','dataflow::migrations::facts::tests::','dataflow::migrations::transfer::tests::']}
 assert list(counts.values())==[10,3,18,15],counts
 assert 'storage_inspection::native_tests::native_snapshot_restores_configs_relations_and_journal_without_changing_fixture_bytes' in names
 save('inventaire.json',dict(names=names,count=len(names),groups=counts,ignored=[],read_before_runtime=True))
 linkage=run('liaison-native',EXPORT+['ldd',str(b)]);assert '/'+NATIVE+'/librag3db.so' in linkage
 runtime=run('temoins',EXPORT+[str(b),*FILTERS,'--test-threads=8'])
 results=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',runtime)
 assert results and tuple(map(int,results[-1]))==(len(names),0,0),results
 successes=set(re.findall(r'^test (.*?)(?: - should panic)? \.\.\. ok$',runtime,re.M))
 assert successes==set(names),'témoins manquants'
 record.update(verdict='VERT',tests=list(map(int,results[-1])),groups=counts)

except BaseException as error:record['error']=repr(error)
finally:
 record['finished_monotonic']=time.monotonic();save('resultat.json',record);print(json.dumps(record),flush=True)
raise SystemExit(0 if record['verdict']=='VERT' else 1)
