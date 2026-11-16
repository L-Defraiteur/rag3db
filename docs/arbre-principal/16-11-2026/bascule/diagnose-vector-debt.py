#!/usr/bin/env python3
"""Complete debt matrix on private before/after copies. No writer, model calculation or HTTP."""
from pathlib import Path
import hashlib,importlib.util,json,os,re,subprocess,time
HOME=Path.home();HERE=Path(__file__).resolve().parent;DATA=HOME/'.cache/rag3weaver-build/bascule-live-20261116'
ROOT=DATA/'vector-debt-diagnostic-01';OLD=DATA/'fresh-sets-01/code-principal/copy';NEW=DATA/'migration-rehearsal-service-corrective-02/code-principal/post-copy'
MODELS=DATA/'migration-rehearsal-service-corrective-02/code-principal/models.stdout'
QUERY=HOME/'.cache/optimiseur-notes/bascule-16nov-927/read-only-query-v2-luciepc'
LIB=HOME/'.cache/rag3db-libs/rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF'
def digest(path):
 h=hashlib.sha256()
 with Path(path).open('rb') as stream:
  for chunk in iter(lambda:stream.read(1048576),b''):h.update(chunk)
 return h.hexdigest()
def parse_rows(text):
 lines=text.splitlines()
 if len(lines)<2 or lines[0]!='BEGIN_QUERY\t0' or not re.fullmatch(r'END_QUERY\t[0-9]+',lines[-1]):raise ValueError('incomplete query observation')
 rows=[]
 for line in lines[1:-1]:
  if not line.startswith('ROW\t'):raise ValueError('unexpected query output')
  rows.append(line[4:].split('|'))
 if int(lines[-1].split('\t')[1])!=len(rows):raise ValueError('query row count mismatch')
 return rows

def main():
 pins={QUERY:'48ae2328a29676fa69154cb9c67d9125d4d04cf92b029bd2b2a09981dea95616',LIB/'librag3db.so':'b7e9fc75a3d2de04c72680b0309e8bb1df9eaadb5211ebbacf39716c6d0a2101',HERE/'inspect-fts.py':'b846b0991999040c04f298747eda27aa07a4aa9ad0c277f8b7dc485eb24465a8',HERE/'vector-debt-matrix.py':'4400822db3c233a190ef10f786a97892f655c64264adf4738e86b6cdba4299aa',MODELS:'ce61440b3a7ad99c1807fc0044f1697703f37a981266812ce85b381c285ca9f0'}
 for path,expected in pins.items():
  if digest(path)!=expected:raise ValueError('diagnostic pin mismatch '+str(path))
 model_rows=parse_rows(MODELS.read_text())
 if len(model_rows)!=1 or model_rows[0][0]!='embedding_model:granite_278m':raise ValueError('unexpected model registry')
 model=json.loads(model_rows[0][1])
 if model!=dict(name='granite-278m',dim=768,storage='suffixed'):raise ValueError('unexpected model storage')
 ROOT.mkdir(mode=0o700,parents=True,exist_ok=False)
 def save(name,value):
  with (ROOT/name).open('x') as stream:json.dump(value,stream,ensure_ascii=False,indent=2)
 def module(name,file):
  spec=importlib.util.spec_from_file_location(name,HERE/file);m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);return m
 fs=module('inspect_fts','inspect-fts.py');matrix=module('matrix','vector-debt-matrix.py')
 before={name:fs.file_inventory(path) for name,path in [('old',OLD),('new',NEW)]}
 for name,rows in before.items():save(name+'-before.json',rows)
 save('model-and-pins.json',dict(model=model,pins={str(p):v for p,v in pins.items()},vector_column='embedding__granite_278m',marker='_embed_hash__granite_278m'))
 env=dict(os.environ,LD_LIBRARY_PATH=str(LIB));start=time.monotonic();end=start+580;observations={};first_error=None
 debt='(n.embedding__granite_278m IS NULL OR n._embed_hash__granite_278m IS NULL OR n._embed_hash__granite_278m <> n._text_hash)'
 valid='o.embedding__granite_278m IS NOT NULL AND o._embed_hash__granite_278m=o._text_hash'
 field="coalesce(o._parent_field,'')=coalesce(n._parent_field,'')"
 rank=field+' AND coalesce(o._index,-1)=coalesce(n._index,-1)'
 hashed=rank+" AND coalesce(o._text_hash,'')=coalesce(n._text_hash,'')"
 exact=hashed+" AND coalesce(o._title,'')=coalesce(n._title,'')"
 levels=dict(field=field,rank=rank,hash=hashed,exact=exact,valid=exact+' AND '+valid,text=exact+' AND '+valid+" AND coalesce(o._text,'')=coalesce(n._text,'')")
 pattern='MATCH (n:ScopeContent_Chunk)-[:ScopeContent_CHUNKED_FROM]->(v:ScopeContent)-[:ScopeContent_DERIVED_FROM]->(s:Scope)'
 queries=[('old-chunks',OLD,'MATCH (n:Scope_Chunk) RETURN count(n)'),('old-debt',OLD,'MATCH (n:Scope_Chunk) WHERE '+debt+' RETURN count(n)'),('new-old-chunks',NEW,'MATCH (n:Scope_Chunk) RETURN count(n)'),('new-old-debt',NEW,'MATCH (n:Scope_Chunk) WHERE '+debt+' RETURN count(n)'),('new-debt',NEW,'MATCH (n:ScopeContent_Chunk) WHERE '+debt+' RETURN count(n)'),('cohort',NEW,'MATCH (n:ScopeContent_Chunk) WHERE '+debt+' RETURN n._uuid'),('universe',NEW,pattern+' WHERE '+debt+' RETURN n._uuid,v._uuid,s._uuid')]
 queries.extend((level,NEW,pattern+' MATCH (o:Scope_Chunk)-[:Scope_CHUNKED_FROM]->(s) WHERE '+debt+' AND '+condition+' RETURN n._uuid,s._uuid,count(DISTINCT o)') for level,condition in levels.items())
 save('queries.json',[dict(name=name,base=str(root/'code-principal.rag3db'),query=sql) for name,root,sql in queries]);rows_by_name={}
 try:
  for name,root,sql in queries:
   remaining=end-time.monotonic()
   if remaining<=5:raise ValueError('diagnostic deadline')
   try:result=subprocess.run([str(QUERY),str(root/'code-principal.rag3db'),sql],capture_output=True,env=env,timeout=min(90,remaining-5))
   except subprocess.TimeoutExpired as error:
    (ROOT/(name+'.stdout')).write_bytes(error.stdout or b'');(ROOT/(name+'.stderr')).write_bytes(error.stderr or b'');save(name+'-process.json',dict(rc=None,timeout=True));raise
   (ROOT/(name+'.stdout')).write_bytes(result.stdout);(ROOT/(name+'.stderr')).write_bytes(result.stderr);save(name+'-process.json',dict(rc=result.returncode))
   if result.returncode or b'59ac9c86bc2b3529b590a7462ab7051e3ad26b8a' not in result.stderr:raise ValueError('RO query refused '+name)
   rows=parse_rows(result.stdout.decode());rows_by_name[name]=rows
   if name in ['old-chunks','old-debt','new-old-chunks','new-old-debt','new-debt']:
    if len(rows)!=1 or len(rows[0])!=1 or not rows[0][0].isdigit():raise ValueError('invalid scalar output '+name)
    observations[name]=int(rows[0][0]);save(name+'-value.json',dict(count=int(rows[0][0])))
  if observations['new-debt']!=469:raise ValueError('debt cohort changed from COPY02')
  cohort=[row[0] for row in rows_by_name['cohort'] if len(row)==1]
  if len(cohort)!=len(rows_by_name['cohort']):raise ValueError('invalid cohort row')
  universe=[tuple(row) for row in rows_by_name['universe'] if len(row)==3]
  if len(universe)!=len(rows_by_name['universe']):raise ValueError('invalid universe row')
  candidates={}
  for level in matrix.LEVELS:
   candidates[level]=[]
   for row in rows_by_name[level]:
    if len(row)!=3 or not row[2].isdigit():raise ValueError('invalid candidate row')
    candidates[level].append((row[0],row[1],int(row[2])))
  result=matrix.build_matrix(cohort,universe,candidates,469);save('matrix.json',result);save('matrix-audit.json',result['audit'])
  if not result['audit']['complete']:raise ValueError('incomplete 469 matrix')
 except Exception as error:first_error=error
 for name,root in [('old',OLD),('new',NEW)]:
  try:
   after=fs.file_inventory(root);save(name+'-after.json',after)
   if after!=before[name]:raise ValueError('private input changed '+name)
  except Exception as audit_error:
   save(name+'-audit-error.json',dict(error=str(audit_error)))
   if first_error is None:first_error=audit_error
 save('observations.json',observations)
 if first_error:save('verdict.json',dict(verdict='RED',error=str(first_error)));raise first_error
 save('verdict.json',dict(verdict='READ_ONLY_469_MATRIX_COMPLETE',elapsed=time.monotonic()-start,live_qualified=False,migration_qualified=False))
if __name__=='__main__':main()
