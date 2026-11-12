"""Capture /proc maps of the actual backend at main, after dynamic loading.

Only --help runs. The debugger supplies a deterministic observation point; no
backend service or model is started and no artifact is rebuilt or replaced.
"""
from pathlib import Path
import hashlib,json,os,subprocess,sys,time
binary=Path(sys.argv[1]);proof=Path(sys.argv[2]);proof.mkdir(exist_ok=True)
assert binary.is_absolute() and binary.is_file()
receipt=json.loads(Path(os.environ['RAG3WEAVER_VALIDATION_RECU']).read_text())
assert receipt['etat']=='actif' and time.monotonic()<receipt['echeance']-5
native=os.environ['RAG3DB_LIB'];assert native.startswith('rag3db-build-29a7bc52a')
command=proof/'backend-main.gdb';assert not command.exists()
command.write_text('''set confirm off
set pagination off
set startup-with-shell off
start
python
import gdb,json,os
from pathlib import Path
inferior=gdb.selected_inferior()
assert inferior.pid>0
proc=Path('/proc')/str(inferior.pid)
proof=Path(os.environ['CX_BACKEND_MAPS_PROOF'])
meta=dict(pid=inferior.pid,starttime=(proc/'stat').read_text().rsplit(') ',1)[1].split()[19],exe=os.readlink(proc/'exe'),cgroup=(proc/'cgroup').read_text().strip().split('::',1)[1],maps='backend-main-maps.txt',capture='gdb-main')
(proof/meta['maps']).write_text((proc/'maps').read_text())
(proof/'backend-main.json').write_text(json.dumps(meta,indent=2)+'\\n')
end
continue
''')
env=dict(os.environ,CX_BACKEND_MAPS_PROOF=str(proof),DEBUGINFOD_URLS='')
with (proof/'backend-main.log').open('w') as output:
 result=subprocess.run(['gdb','--nx','--batch','--return-child-result','-x',str(command),'--args',str(binary),'--help'],env=env,stdout=output,stderr=subprocess.STDOUT,timeout=min(20,max(.1,receipt['echeance']-time.monotonic()-5)))
assert result.returncode==0,('gdb/backend failed',result.returncode)
meta=json.loads((proof/'backend-main.json').read_text());assert meta['exe']==str(binary)
maps=(proof/meta['maps']).read_text();rows=[line for line in maps.splitlines() if 'librag3db.so' in line]
assert rows and all('/'+native+'/librag3db.so' in line for line in rows),rows
assert 'exited normally' in (proof/'backend-main.log').read_text()
with binary.open('rb') as f:digest=hashlib.file_digest(f,'sha256').hexdigest()
meta.update(native=rows,sha256=digest)
(proof/'backend-main.json').write_text(json.dumps(meta,indent=2)+'\n')
print(json.dumps(meta),flush=True)
