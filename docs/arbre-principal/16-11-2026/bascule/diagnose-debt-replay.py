#!/usr/bin/env python3
"""Proposed standalone Rust replay. Reads private DB content in memory only, never writes it."""
from pathlib import Path
import collections,hashlib,importlib.util,json,os,re,subprocess,time
HERE=Path(__file__).resolve().parent

def module(name):
 spec=importlib.util.spec_from_file_location(name,HERE/(name+'.py'));m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);return m

def one_body(raw):
 head=b'BEGIN_QUERY\t0\nROW\t';tail=b'\nEND_QUERY\t1\n'
 if not raw.startswith(head) or not raw.endswith(tail):raise ValueError('single-body framing refused')
 return raw[len(head):-len(tail)]

def main():
 p=module('diagnose-debt-profile');fs=module('inspect-fts');root=p.DATA/'vector-debt-replay-01';geo=p.DATA/'vector-debt-geometry-pc-01';deps=Path.home()/'.cache/optimiseur-notes/bascule-16nov-927/release-archive/deps';rustc=Path.home()/'.rustup/toolchains/1.98.1-x86_64-unknown-linux-gnu/bin/rustc'
 if p.digest(rustc)!='859254978c0a0402c32f949f6de0d99aee73be8d15f45aac00ae1448aac51e74' or p.digest(p.QUERY)!='48ae2328a29676fa69154cb9c67d9125d4d04cf92b029bd2b2a09981dea95616' or p.digest(p.LIB/'librag3db.so')!='b7e9fc75a3d2de04c72680b0309e8bb1df9eaadb5211ebbacf39716c6d0a2101':raise ValueError('tool pin')
 manifest=geo/'proof-manifest.json'
 if p.digest(manifest)!='dd5157b51fac6d4ed1877585655039ac30ca41b4f46f3841a8192f37088c2568':raise ValueError('geometry manifest pin')
 for entry in json.loads(manifest.read_text()):
  file=geo/entry['name']
  if file.stat().st_size!=entry['size'] or p.digest(file)!=entry['sha256']:raise ValueError('geometry evidence changed')
 prepared=json.loads((HERE/'replay-prepared.json').read_text())
 for name,sha in prepared['files'].items():
  if p.digest(HERE/name)!=sha:raise ValueError('prepared source pin')
 if prepared['before_commit']!='49644f064f9f9ece4c9c4bd8714eb86838afa4a0' or prepared['after_commit']!='479a46549e9d3eb04459bbfd05445e3750fbdfd6' or not prepared['after_equals_927']:raise ValueError('chunker Git provenance')
 profiles=json.loads((geo/'profiles-corrected-from-immutable-raw.json').read_text())
 for name in ['old-scope','new-content']:
  raw=p.DATA/'vector-debt-profile-01'/(name+'.stdout')
  if p.digest(raw)!=profiles[name]['raw_sha256']:raise ValueError('configuration raw changed')
  config=json.loads(p.rows(raw.read_text())[0])['chunking']
  if {k:config.get(k) for k in ['maxSize','overlap','maxLines','overlapLines','strategy']}!={'maxSize':1500,'overlap':0,'maxLines':30,'overlapLines':5,'strategy':'lines'}:raise ValueError('persisted chunking parameters changed')
 observations=json.loads((geo/'observations.json').read_text());cohort=[row.split('|') for row in observations['cohort-new-geometry']];parents=sorted({r[1] for r in cohort})
 if len(cohort)!=469 or len(parents)!=51 or any(not re.fullmatch('[0-9a-f-]{36}',x) for x in parents):raise ValueError('cohort identity')
 root.mkdir(mode=0o700,parents=True,exist_ok=False)
 def save(name,value):
  with (root/name).open('x') as stream:json.dump(value,stream,indent=2)
 before=fs.file_inventory(p.NEW);save('source-before.json',before);start=time.monotonic();end=start+570;error=None
 libraries=[deps/'libtext_splitter-a566ac746e53062b.rlib',deps/'libblake3-427059a5348cca07.rlib'];dep_before={str(x):p.digest(x) for x in libraries};save('dependencies-before.json',dep_before)
 def call(args,input=None,cap=60,env=None):
  remaining=end-time.monotonic()
  if remaining<=15:raise ValueError('replay deadline')
  return subprocess.run(args,input=input,capture_output=True,timeout=min(cap,remaining-15),env=env)
 native_env=dict(os.environ,LD_LIBRARY_PATH=str(p.LIB));binary=root/'replay-chunker'
 try:
  command=[str(rustc),'--edition=2021','-C','codegen-units=1',str(HERE/'replay-chunker.rs'),'-L','dependency='+str(deps),'--extern','text_splitter='+str(libraries[0]),'--extern','blake3='+str(libraries[1]),'-o',str(binary)]
  save('compile-command.json',command);build=call(command,cap=90);(root/'compile.stdout').write_bytes(build.stdout);(root/'compile.stderr').write_bytes(build.stderr);save('compile-process.json',dict(rc=build.returncode))
  if build.returncode:raise ValueError('standalone compilation refused')
  save('ledger.json',dict(binary_sha256=p.digest(binary),prepared=prepared,dependencies=dep_before,rustc_sha256=p.digest(rustc),runtime_started=False))
  def replay(text,name):
   result=call([str(binary),'1500','0','30','5','lines'],input=text)
   if result.returncode:raise ValueError('CPU replay refused '+name)
   # This program emits only input length/hash and chunk coordinates/hashes.
   rows=result.stdout.decode().splitlines();output={'BEFORE':[],'AFTER':[]}
   if not rows or not re.fullmatch(r'INPUT\t[0-9]+\t[0-9a-f]{64}',rows[0]):raise ValueError('replay input witness absent')
   if int(rows[0].split('\t')[1])!=len(text):raise ValueError('input byte length changed')
   for row in rows[1:]:
    parts=row.split('\t')
    if len(parts)!=7 or parts[0] not in output or not all(x.isdigit() for x in parts[1:6]) or not re.fullmatch('[0-9a-f]{64}',parts[6]):raise ValueError('malformed chunk witness')
    output[parts[0]].append(tuple(parts[1:]))
   (root/(name+'.hashes.tsv')).write_bytes(result.stdout)
   return output
  synthetic=replay(b'x'*53294,'synthetic')
  if len(synthetic['BEFORE'])!=1 or len(synthetic['AFTER'])!=36 or any(int(x[4])-int(x[3])>1500 for x in synthetic['AFTER']):raise ValueError('synthetic does not distinguish algorithms')
  ids=','.join("'"+x+"'" for x in parents);stored={};queries=[]
  for label,table,relation in [('BEFORE','Scope_Chunk','Scope_CHUNKED_FROM'),('AFTER','ScopeContent_Chunk','ScopeContent_CHUNKED_FROM')]:
   pat='MATCH (c:'+table+')-[:'+relation+']->(s:Scope)' if label=='BEFORE' else 'MATCH (c:'+table+')-[:'+relation+']->(v:ScopeContent)-[:ScopeContent_DERIVED_FROM]->(s:Scope)'
   sql=pat+" WHERE s._uuid IN ["+ids+"] AND c._parent_field='content' RETURN s._uuid,c._index,c._start_char,c._end_char,c._core_start_char,c._core_end_char,c._text_hash"
   queries.append(dict(name=label,query=sql));r=call([str(p.QUERY),str(p.NEW/'code-principal.rag3db'),sql],env=native_env)
   (root/(label+'-stored.stdout')).write_bytes(r.stdout);(root/(label+'-stored.stderr')).write_bytes(r.stderr)
   if r.returncode or b'59ac9c86bc2b3529b590a7462ab7051e3ad26b8a' not in r.stderr:raise ValueError('stored tuple query refused')
   group={x:[] for x in parents}
   for row in p.rows(r.stdout.decode()):
    parts=row.split('|')
    if len(parts)!=7 or parts[0] not in group:raise ValueError('stored tuples malformed')
    group[parts[0]].append(tuple(parts[1:]))
   stored[label]=group
  save('queries.json',queries);checks=[];covered=set()
  for parent in parents:
   sql="MATCH (s:Scope {_uuid:'"+parent+"'}) RETURN s.content"
   result=call([str(p.QUERY),str(p.NEW/'code-principal.rag3db'),sql],env=native_env)
   if result.returncode or b'59ac9c86bc2b3529b590a7462ab7051e3ad26b8a' not in result.stderr:raise ValueError('content query refused')
   text=one_body(result.stdout);replayed=replay(text,parent);del text,result
   for label in ['BEFORE','AFTER']:
    if collections.Counter(replayed[label])!=collections.Counter(stored[label][parent]):raise ValueError('stored/replay mismatch '+label+' parent '+parent)
   for row in cohort:
    if row[1]==parent:
     if tuple(row[2:]) not in replayed['AFTER']:raise ValueError('debt tuple not replayed')
     covered.add(row[0])
   checks.append(dict(parent=parent,before=len(replayed['BEFORE']),after=len(replayed['AFTER']),matched=True));save(parent+'-check.json',checks[-1])
  if len(covered)!=469:raise ValueError('debt coverage incomplete')
  save('coverage.json',dict(parents=checks,debt_ids=sorted(covered),debt_count=len(covered),replay_matched=True,debt_paid=False))
 except Exception as exc:error=exc
 finally:
  after=fs.file_inventory(p.NEW);save('source-after.json',after)
  if after!=before and error is None:error=ValueError('private copy changed')
  dep_after={str(x):p.digest(x) for x in libraries};save('dependencies-after.json',dep_after)
  if dep_before!=dep_after and error is None:error=ValueError('archive dependencies changed')
 if time.monotonic()>=end and error is None:error=ValueError('deadline after final hashes')
 save('verdict.json',dict(verdict='RED' if error else 'READ_ONLY_CHUNKER_REPLAY_MATCHED',error=str(error) if error else None,elapsed=time.monotonic()-start,migration_qualified=False,debt_paid=False))
 if error:raise error
if __name__=='__main__':main()
