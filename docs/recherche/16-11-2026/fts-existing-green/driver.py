"""Run the one authorized pre-fix witness under the common validation receipt."""
import hashlib,json,os,subprocess,sys,time
from pathlib import Path
W=Path(sys.argv[1]).resolve();P=Path(sys.argv[2]).resolve();P.mkdir(exist_ok=False)
C=W/'extension/rag3weaver'
SOURCE='04634cfed667711192efd313ff1418ef8dfd5c26'
TEST='catalog::tests::existing_fts_live_ids_reads_closed_files_without_writer_or_changes'
MODULE='catalog::tests::existing_fts_'
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
T=Path(os.environ['CARGO_TARGET_DIR']);assert T.is_dir() and not T.is_symlink()
OLD=Path.home()/'.cache/rag3weaver-build/cx-wt-recherche-fts-pending-schema-red-source'
marker=T/'.arbre-du-target';assert marker.is_file() and not marker.is_symlink()
assert marker.read_text().strip()==str(OLD/'extension/rag3weaver')
old_receipt=json.loads((Path.home()/'.cache/rag3weaver-build/cx-archive-recherche-fts-pending-red-v2/supervisor/autorite.json').read_text());assert old_receipt['etat']=='termine'
old_ledger=json.loads((Path.home()/'.cache/rag3weaver-build/cx-archive-recherche-fts-pending-red-v2/driver/ledger.json').read_text());assert sha(old_ledger['artifact']['executable'])==old_ledger['elf_sha256']
assert subprocess.check_output(['git','-C',str(OLD),'rev-parse','HEAD'],text=True).strip()=='c1b10a4993650a82e0cac9d40f4cb3567a0593d9'
assert not subprocess.check_output(['git','-C',str(OLD),'status','--porcelain'],text=True).strip()
assert os.environ['RAG3WEAVER_TARGET_REPRENDRE']=='1' 
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
library=Path(native['RAG3DB_LIBRARY_DIR']);assert library==Path(os.environ['RAG3DB_LIBS_DIR'])/LIB
assert sha(library/'librag3db.so')=='b7e9fc75a3d2de04c72680b0309e8bb1df9eaadb5211ebbacf39716c6d0a2101'
assert sha(library/'libvector.rag3db_extension')=='1669d6392ec8067fcf92ae2ccc972fd7cd3086629fb82d1f5d254e90d72d7561'
save('native-profile.json',dict(env=native,library_sha256=sha(library/'librag3db.so'),vector_sha256=sha(library/'libvector.rag3db_extension'),store=str(library.parent)))
# Observation fraîche juste avant la garde commune : aucun processus attribué
# au target ou à son ancien arbre, aucune disparition masquée d'un tel processus.
found=[];unknown=[]
for proc in Path('/proc').iterdir():
 if not proc.name.isdigit():continue
 attributed=False
 try:
  if proc.stat().st_uid!=os.getuid():continue
  stamp=(proc/'stat').read_text().rsplit(') ',1)[1].split()[19]
  def relevant(value):return any(value==str(root) or value.startswith(str(root)+'/') for root in [T,OLD])
  for field in ['exe','cwd']:
   value=os.readlink(proc/field)
   if relevant(value):attributed=True;found.append(dict(pid=proc.name,start=stamp,field=field,value=value))
  for fd in (proc/'fd').iterdir():
   try:value=os.readlink(fd)
   except FileNotFoundError:continue
   if relevant(value):attributed=True;found.append(dict(pid=proc.name,start=stamp,field='fd',value=value))
 except (FileNotFoundError,ProcessLookupError):
  if attributed:raise
 except PermissionError:
  unknown.append(dict(pid=proc.name,attributed=attributed))
  if attributed:raise
save('target-before-guard.json',dict(marker=marker.read_text(),observed=time.monotonic(),found=found,unknown=unknown,old_receipt_state=old_receipt['etat']))
assert not found
code=run('compile',[sys.executable,'-B',str(C/'scripts/cargo_garde.py'),'test' ,'--locked','--no-default-features','--features','rag3db-native','--lib','--no-run','--message-format=json-render-diagnostics','-j'+os.environ['CARGO_BUILD_JOBS']],900)
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
module_names=[line.removesuffix(': test') for line in (P/'inventory.log').read_text().splitlines() if line.startswith(MODULE) and line.endswith(': test')]
assert len(module_names)==10 and len(set(module_names))==10
save('module-inventory.json',dict(names=module_names,count=10,ignored=0))
save('ledger.json',dict(source=git('rev-parse','HEAD'),artifact=artifact,elf_sha256=sha(elf),elf_bytes=elf.stat().st_size,inventory_sha256=sha(P/'inventory.log'),exact_test=TEST,published_at=time.monotonic()))
assert time.monotonic()<END-5
code=run('witness',[str(elf),TEST,'--exact','--nocapture','--test-threads=1'],30)
if code:sys.exit(code)
first_text=(P/'witness.log').read_text()
assert 'test result: ok. 1 passed; 0 failed; 0 ignored;' in first_text
code=run('module',[str(elf),MODULE,'--show-output','--test-threads=1'],120)
if code:sys.exit(code)
text=(P/'module.log').read_text()
assert 'test result: ok. 10 passed; 0 failed; 0 ignored;' in text
for name in module_names:assert 'test '+name+' ... ok' in text,name
assert sha(elf)==json.loads((P/'ledger.json').read_text())['elf_sha256']
assert time.monotonic()<END
save('observed.json',dict(returncode=0,elapsed_since_supervisor=time.monotonic()-(END-1200),first_test=TEST,module_tests=module_names,distinct=10,invocations=11,source=git('rev-parse','HEAD'),scope='cold FTS reading only'))
