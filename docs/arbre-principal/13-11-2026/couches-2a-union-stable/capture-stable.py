from pathlib import Path
import gdb,hashlib,json,os,time

def capture(stage,require_native):
 proof=Path('/home/lucied/.cache/rag3weaver-build/cx-validation-couches-2a-union-v3')
 config=json.loads((proof/'stable-config.json').read_text());pid=gdb.selected_inferior().pid;proc=Path('/proc')/str(pid)
 attempt=dict(stage=stage,observed=time.monotonic(),pid=pid)
 path=proof/('sdk-attempt-'+stage+'.json')
 try:
  st=(proc/'stat').read_text().rsplit(') ',1)[1].split();stamp=st[19]
  data=(proc/'environ').read_bytes();raw=dict(x.split(b'=',1) for x in data.split(b'\0') if b'=' in x)
  actual={k:os.fsdecode(raw[os.fsencode(k)]) if os.fsencode(k) in raw else None for k in config['expected']}
  exe=os.readlink(proc/'exe');cg=(proc/'cgroup').read_text().strip().split('::',1)[1];maps=(proc/'maps').read_text()
  with (proc/'exe').open('rb') as f:sha=hashlib.file_digest(f,'sha256').hexdigest()
  native=[line for line in maps.splitlines() if 'librag3db.so' in line]
  diff={k:dict(expected=v,actual=actual[k]) for k,v in config['expected'].items() if actual[k]!=v}
  attempt.update(starttime=stamp,exe=exe,exe_sha256=sha,cgroup=cg,rawlen=len(data),present={k:os.fsencode(k) in raw for k in config['expected']},expected=config['expected'],actual=actual,diff=diff,native=native)
  with path.open('x') as f:json.dump(attempt,f,indent=2)
  assert exe==config['exe'] and sha==config['sha256'] and cg==config['cgroup'] and not diff
  assert (proc/'stat').read_text().rsplit(') ',1)[1].split()[19]==stamp
  sdk=json.loads((proof/'sdk-env-lib-large.json').read_text());assert sdk['pid']==pid and sdk['starttime']==stamp and sdk['env']==actual
  if require_native:
   first=json.loads((proof/'sdk-attempt-exec.json').read_text());assert first['pid']==pid and first['starttime']==stamp
   assert native and all('/'+config['expected']['RAG3DB_LIB']+'/librag3db.so' in row for row in native)
   snapshot='large-main-sdk-maps.txt'
   with (proof/snapshot).open('x') as f:f.write(maps)
   main=dict(pid=pid,starttime=stamp,expected_exe=exe,expected_sha256=sha,cgroup=cg,phase='lib-large')
   profile=dict(pid=pid,starttime=stamp,exe=exe,cgroup=cg,env=actual,phase='lib-large')
   for name,value in [('large-main-process.json',main),('large-real-sdk-env.json',profile),('large-main-attestation.json',dict(profile,exe_sha256=sha,maps=snapshot,native=native,observed=time.monotonic(),principal=True,capture='gdb-main-same-inferior'))]:
    with (proof/name).open('x') as f:json.dump(value,f,indent=2)
 except BaseException as error:
  with (proof/('sdk-capture-error-'+stage+'.json')).open('x') as f:json.dump(dict(attempt,error=repr(error)),f,indent=2)
  gdb.execute('quit 91')
