"""Run the one authorized pre-fix witness under the common validation receipt."""
import hashlib,json,os,subprocess,sys,time
from pathlib import Path
W=Path(sys.argv[1]).resolve();P=Path(sys.argv[2]).resolve();P.mkdir(exist_ok=False)
C=W/'extension/rag3weaver'
SOURCE='187d58a7b5'
TEST='catalog::tests::existing_fts_live_ids_reads_closed_files_without_writer_or_changes'
LIB='rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF'
PIN='2150fcd847b1df7de04908d357a0c78abc05e50ce599e2793ff08ab48108825d'
def save(name,value):
 with (P/name).open('x') as f:json.dump(value,f,indent=2)
def sha(path):
 with Path(path).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def git(*args):return subprocess.check_output(['git','-C',str(W),*args],text=True).strip()
receipt=Path(os.environ['RAG3WEAVER_VALIDATION_RECU']);authority=json.loads(receipt.read_text());END=authority['echeance']
subprocess.run([sys.executable,str(C/'scripts/confiner_validation.py'),'verifier'],check=True)
assert authority['etat']=='actif' and 0<END-time.monotonic()<=1200
assert git('rev-parse','HEAD').startswith(SOURCE) and not git('status','--porcelain')
assert sha(Path.home()/'.cache/rag3weaver-build/passe_epinglee.sh')==PIN
assert sha(W/'tools/poste/passe_epinglee.sh')==PIN
assert os.environ['RAG3DB_LIB']==LIB
assert not any(os.environ.get(k) for k in ['RAG3WEAVER_MOTEUR_ANCIEN','RAG3WEAVER_EMBED_SERVICE','RAG3WEAVER_PG','RAG3WEAVER_TESTS_REPLI_PERIPHERIQUE'])
cg=Path('/sys/fs/cgroup')/Path('/proc/self/cgroup').read_text().split('0::',1)[1].strip().lstrip('/')
def attest_scope(cgroup):
 values={key:(cgroup/key).read_text().strip() for key in ['memory.max','memory.high','memory.swap.max','cpu.max']}
 assert values['memory.max']=='34359738368'
 assert values['memory.high']=='max'
 assert values['memory.swap.max']=='0'
 quota,period=values['cpu.max'].split()
 assert quota!='max' and 0<int(quota)<=8*int(period)
 return values
scope_profile=attest_scope(cg)
assert int(os.environ['CARGO_BUILD_JOBS'])<=8
T=Path(os.environ['CARGO_TARGET_DIR']);assert not T.exists(),'new session target unexpectedly exists'
save('entry.json',dict(source=git('rev-parse','HEAD'),test=TEST,features=['rag3db-native'],target=str(T),deadline=END,cgroup=str(cg),scope_profile=scope_profile,profile={k:os.environ.get(k) for k in ['CARGO_BUILD_JOBS','CARGO_INCREMENTAL','RAG3DB_LIB','RUST_TEST_THREADS','RAG3WEAVER_TARGET_MAX_GO']},pin_sha256=PIN))
def run(name,argv,cap):
 remaining=END-time.monotonic()-5
 assert remaining>0
 save(name+'.command.json',dict(argv=argv,start=time.monotonic(),timeout=min(cap,remaining)))
 with (P/(name+'.log')).open('wb') as f:
  result=subprocess.run(argv,cwd=C,stdout=f,stderr=subprocess.STDOUT,timeout=min(cap,remaining))
 save(name+'.result.json',dict(returncode=result.returncode,end=time.monotonic()))
 return result.returncode
export_script='source "$1"; exporter_la_lib || exit; python3 -c \'import json,os; print(json.dumps({k:os.environ.get(k) for k in ["RAG3DB_SHARED","RAG3DB_LIBRARY_DIR","RAG3DB_INCLUDE_DIR","LD_LIBRARY_PATH"]}))\''
native=json.loads(subprocess.check_output(['bash','-c',export_script,'export-profile',str(C/'scripts/lib_du_magasin.sh')],cwd=C,text=True))
os.environ.update(native)
save('native-profile.json',dict(env=native,library_sha256=sha(Path(native['RAG3DB_LIBRARY_DIR'])/'librag3db.so')))
code=run('compile',[sys.executable,'-B',str(C/'scripts/cargo_garde.py'),'test' ,'--locked','--no-default-features','--features','rag3db-native','--lib','--no-run','--message-format=json-render-diagnostics','-j8'],900)
if code:sys.exit(code)
artifacts=[]
for line in (P/'compile.log').read_text().splitlines():
 try:row=json.loads(line)
 except ValueError:continue
 if row.get('reason')=='compiler-artifact' and row.get('target',{}).get('name')=='rag3weaver' and row.get('profile',{}).get('test') and row.get('executable'):artifacts.append(row)
assert len(artifacts)==1
artifact=artifacts[0];assert artifact['features']==['rag3db-native']
elf=Path(artifact['executable'])
with elf.open('rb') as f:assert f.read(4)==b'\x7fELF'
if run('inventory',[str(elf),'--list','--format','terse'],15):sys.exit(2)
if run('exact-inventory',[str(elf),TEST,'--exact','--list','--format','terse'],15):sys.exit(2)
assert (P/'exact-inventory.log').read_text().splitlines()==[TEST+': test']
save('ledger.json',dict(source=git('rev-parse','HEAD'),artifact=artifact,elf_sha256=sha(elf),elf_bytes=elf.stat().st_size,inventory_sha256=sha(P/'inventory.log'),exact_test=TEST,published_at=time.monotonic()))
assert time.monotonic()<END-5
code=run('witness',[str(elf),TEST,'--exact','--nocapture','--test-threads=1'],120)
assert sha(elf)==json.loads((P/'ledger.json').read_text())['elf_sha256']
assert time.monotonic()<END
save('observed.json',dict(returncode=code,elapsed_since_supervisor=time.monotonic()-(END-1200),only_test=TEST,source=git('rev-parse','HEAD'),next_tests_launched=False))
sys.exit(code if code else 2) # Unexpected green is a stop, never authority for another test.
