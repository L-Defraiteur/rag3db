from pathlib import Path
import hashlib,json,os,shutil,subprocess,sys,time
R=Path('/home/lucied/.cache/rag3weaver-build');Q=Path(__file__).parent;P=R/'cx-sonde-cargo-union-v1';W=R/'cx-wt-couches-union-r2';C=W/'extension/rag3weaver';T=R/'cx-target-contenu-derive';META=json.loads((Q/'provenance.json').read_text())
NATIVE='rag3db-build-29a7bc52aaf11e6204f9beafaffb154105417e13-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF';LIB=R.parent/'rag3db-libs'/NATIVE
EXPECTED=dict(RAG3DB_LIB=NATIVE,RAG3DB_SHARED='1',RAG3DB_LIBRARY_DIR=str(LIB),RAG3DB_INCLUDE_DIR=str(LIB),LD_LIBRARY_PATH=str(LIB),RAG3DB_BUFFER_POOL_SIZE='1073741824',RAG3WEAVER_MOTEUR_ANCIEN='1')
def save(name,value):(P/name).write_text(json.dumps(value,indent=2)+'\n')
def digest(path):
 with Path(path).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def snapshot():
 return {str(p.relative_to(T)):(p.lstat().st_mode,p.lstat().st_size,p.lstat().st_mtime_ns) for root in T.rglob('.fingerprint') for p in [root,*root.rglob('*')]}|{'.arbre-du-target':digest(T/'.arbre-du-target')}
if len(sys.argv)>1:
 assert sys.argv[1:] == ['exported']
 actual={key:os.environ.get(key) for key in EXPECTED};diff={k:dict(expected=EXPECTED[k],actual=actual[k]) for k in EXPECTED if actual[k]!=EXPECTED[k]}
 save('sdk-export-exact.json',dict(pid=os.getpid(),expected=EXPECTED,actual=actual,diff=diff));assert not diff
 assert digest(LIB/'librag3db.so')=='b014febe51417ca535218d1c0b18308e7d301157abf45027a5cbba70dfc54929'
 proxy=Path(shutil.which('cargo'));assert proxy.resolve().name=='rustup','probe must reproduce the existing rustup shim path'
 script=P/'cargo-version.gdb';script.write_text('''set confirm off
set pagination off
set debuginfod enabled off
set startup-with-shell off
set follow-fork-mode child
python
import gdb,json,os,hashlib
from pathlib import Path
def capture_cargo():
 pid=gdb.selected_inferior().pid
 proc=Path('/proc')/str(pid)
 exe=os.readlink(proc/'exe')
 if Path(exe).name!='cargo': return
 proof=Path(os.environ['CX_CARGO_ENV_PROBE'])
 expected=json.loads((proof/'sdk-export-exact.json').read_text())['expected']
 cg=(proc/'cgroup').read_text().strip().split('::',1)[1]
 assert cg==Path('/proc/self/cgroup').read_text().strip().split('::',1)[1]
 raw=dict(x.split(b'=',1) for x in (proc/'environ').read_bytes().split(b'\\0') if b'=' in x)
 actual={k:(os.fsdecode(raw[os.fsencode(k)]) if os.fsencode(k) in raw else None) for k in expected}
 diff={k:dict(expected=expected[k],actual=actual[k]) for k in expected if expected[k]!=actual[k]}
 with (proc/'exe').open('rb') as handle:sha=hashlib.file_digest(handle,'sha256').hexdigest()
 record=dict(pid=pid,starttime=(proc/'stat').read_text().rsplit(') ',1)[1].split()[19],exe=exe,exe_sha256=sha,cwd=os.readlink(proc/'cwd'),cgroup=cg,actual=actual,expected=expected,diff=diff,argv=(proc/'cmdline').read_bytes().decode().split('\\0'),meaning='observation only: no relaxed SDK contract, no compile')
 (proof/'cargo-env-observe.json').write_text(json.dumps(record,indent=2))
end
catch exec
commands
silent
python capture_cargo()
continue
end
run
''')
 env=dict(os.environ,CX_CARGO_ENV_PROBE=str(P),DEBUGINFOD_URLS='')
 result=subprocess.run(['gdb','--nx','--batch','--return-child-result','-x',str(script),'--args',str(proxy),'--version'],cwd=C,env=env)
 assert result.returncode==0 and (P/'cargo-env-observe.json').exists();sys.exit(0)
AUTH=Path(os.environ['RAG3WEAVER_VALIDATION_RECU']);authority=json.loads(AUTH.read_text());END=min(authority['echeance'],META['original']['deadline']);CG=Path('/proc/self/cgroup').read_text().strip().split('::',1)[1];G=Path('/sys/fs/cgroup'+CG)
assert authority['echeance']<=META['original']['deadline'] and (G/'memory.max').read_text().strip()=='34359738368' and (G/'memory.high').read_text().strip()=='max'
for name,sha in META['files'].items():assert digest(Q/name)==sha
for root,sha in [(W,META['app']),(Path(META['control']),META['pilot'])]:
 assert subprocess.check_output(['git','-C',str(root),'rev-parse','HEAD'],text=True).strip()==sha
 assert not subprocess.check_output(['git','-C',str(root),'status','--porcelain'],text=True).strip()
for folder in ['tmp','config','data','cache']:(P/folder).mkdir()
E={k:os.environ[k] for k in ['HOME','USER','LOGNAME','PATH','SHELL','LANG','LC_ALL','TZ','CARGO_HOME','RUSTUP_HOME','DBUS_SESSION_BUS_ADDRESS','RAG3WEAVER_VALIDATION_RECU','RAG3WEAVER_VALIDATION_JETON'] if k in os.environ}
E.update(TMPDIR=str(P/'tmp'),XDG_CONFIG_HOME=str(P/'config'),XDG_DATA_HOME=str(P/'data'),XDG_CACHE_HOME=str(P/'cache'),CARGO_MANIFEST_DIR=str(C),CARGO_TARGET_DIR=str(T),CARGO_BUILD_JOBS='8',RAG3DB_LIB=NATIVE,RAG3DB_LIBS_DIR=str(R.parent/'rag3db-libs'),RAG3DB_BUFFER_POOL_SIZE='1073741824',RAG3WEAVER_MOTEUR_ANCIEN='1')
for suffix in ['','_EMBEDDER','_RERANKER','_OCR']:E['RAG3WEAVER_BURN_DEVICE'+suffix]='cpu'
save('source.json',dict(app=META['app'],pilot=META['pilot'],receipt=str(AUTH),scope_deadline=END,original_start=META['original']['initial'],original_deadline=META['original']['deadline'],cgroup=CG,command=['cargo','--version'],command_cap=20,kind='SONDE_NON_COMPILANTE',target=str(T)))
verdict='ROUGE';before=snapshot();save('target-before.json',before)
try:
 subprocess.run(['python3',str(Q/'confiner_validation.py'),'verifier'],cwd='/var/tmp',env=E,check=True)
 export=['bash','-c','. "$1"; exporter_la_lib || exit 2; shift; exec "$@"','probe-sdk',str(C/'scripts/lib_du_magasin.sh')]
 with (P/'cargo-version.log').open('w') as f:
  result=subprocess.run(export+['python3',str(Q/'sonde.py'),'exported'],cwd=C,env=E,stdout=f,stderr=subprocess.STDOUT,timeout=min(20,END-time.monotonic()-5))
 assert result.returncode==0
 record=json.loads((P/'cargo-env-observe.json').read_text());assert record['cgroup']==CG and record['cwd']==str(C) and record['argv'][1:2]==['--version']
 after=snapshot();save('target-after.json',after);assert before==after,'target changed during noncompiling probe'
 for root,sha in [(W,META['app']),(Path(META['control']),META['pilot'])]:
  assert subprocess.check_output(['git','-C',str(root),'rev-parse','HEAD'],text=True).strip()==sha
  assert not subprocess.check_output(['git','-C',str(root),'status','--porcelain'],text=True).strip()
 assert time.monotonic()<END-3;verdict='OBSERVATION_CARGO_SANS_COMPILATION'
except BaseException as error:save('error.json',repr(error))
finally:
 save('verdict-sonde.json',dict(verdict=verdict,scope_deadline=END,original_deadline=META['original']['deadline'],target_unchanged=snapshot()==before,no_rustc_or_build_command=True,not_f8_qualified=True));(P/'VERDICT_SONDE').write_text(verdict+'\n')
sys.exit(0 if verdict=='OBSERVATION_CARGO_SANS_COMPILATION' else 1)
