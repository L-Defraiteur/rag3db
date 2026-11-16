"""GDB main stop: capture the actual inferior once, then caller continues it."""
import gdb,hashlib,json,os
from pathlib import Path
p=Path.home()/'.cache/rag3weaver-build/cx-proofs-recherche-fts-double-v1'
def capture():
 pid=gdb.selected_inferior().pid
 proc=Path('/proc')/str(pid)
 expected=json.loads((p/'profile.json').read_text())['expected']
 actual={}
 for field in (proc/'environ').read_bytes().split(b'\0'):
  if b'=' in field:
   key,value=field.split(b'=',1);key=os.fsdecode(key)
   if key in expected:actual[key]=os.fsdecode(value)
 raw=(proc/'stat').read_text().rsplit(') ',1)[1].split()
 record=dict(pid=pid,start=raw[19],expected=expected,actual=actual,diff={k:dict(expected=v,actual=actual.get(k)) for k,v in expected.items() if actual.get(k)!=v},exe=os.readlink(proc/'exe'),exe_sha256=hashlib.sha256((proc/'exe').read_bytes()).hexdigest(),maps=(proc/'maps').read_text())
 with (p/'main-profile.json').open('x') as f:json.dump(record,f,indent=2)
 ledger=json.loads((p/'ledger.json').read_text())
 if record['diff'] or record['exe_sha256']!=ledger['elf_sha256'] or '/librag3db.so' not in record['maps']:raise gdb.GdbError('main profile/ELF/SDK differs; refuse before continue')
try:
 capture()
except BaseException as error:
 gdb.write("MAIN_PROFILE_REFUSED "+repr(error)+"\n",gdb.STDERR)
 gdb.execute("kill")
 gdb.execute("quit 1")
