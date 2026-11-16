#!/usr/bin/env python3
"""RO equality/geometry and named-path metadata only. Never emits corpus bodies."""
from pathlib import Path
import argparse,importlib.util,json,os,subprocess,time
HERE=Path(__file__).resolve().parent

def load(name):
 spec=importlib.util.spec_from_file_location(name,HERE/(name+'.py'));m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);return m

def main():
 ap=argparse.ArgumentParser();ap.add_argument('--poste',choices=['pc','portable'],required=True);a=ap.parse_args()
 p=load('diagnose-debt-profile');fs=load('inspect-fts');root=p.DATA/('vector-debt-geometry-'+a.poste+'-01')
 query=p.QUERY if a.poste=='pc' else p.QUERY.with_name('read-only-query-v2')
 query_sha='48ae2328a29676fa69154cb9c67d9125d4d04cf92b029bd2b2a09981dea95616' if a.poste=='pc' else '43cffb082a2487e9f33ffa62b5a246081e5c458683f4e6b7de50e46d60c01329'
 for path,sha in [(query,query_sha),(p.LIB/'librag3db.so','b7e9fc75a3d2de04c72680b0309e8bb1df9eaadb5211ebbacf39716c6d0a2101'),(HERE/'inspect-fts.py','b846b0991999040c04f298747eda27aa07a4aa9ad0c277f8b7dc485eb24465a8')]:
  if p.digest(path)!=sha:raise ValueError('pin mismatch '+str(path))
 root.mkdir(parents=True,mode=0o700,exist_ok=False)
 def save(name,value):
  with (root/name).open('x') as stream:json.dump(value,stream,ensure_ascii=False,indent=2)
 bases={'principal':p.OLD/'code-principal.rag3db','migrated':p.NEW/'code-principal.rag3db','lourd':p.DATA/'id-sets-pc-01/code-lourd/code-lourd.rag3db'} if a.poste=='pc' else {'portable':p.DATA/'id-sets-portable-01/copy/code-rag3db-depot.rag3db'}
 before={k:fs.file_inventory(v.parent) for k,v in bases.items()}
 for k,v in before.items():save(k+'-before.json',v)
 queries=[]
 if a.poste=='pc':
  debt='(n.embedding__granite_278m IS NULL OR n._embed_hash__granite_278m IS NULL OR n._embed_hash__granite_278m <> n._text_hash)'
  pat='MATCH (n:ScopeContent_Chunk)-[:ScopeContent_CHUNKED_FROM]->(v:ScopeContent)-[:ScopeContent_DERIVED_FROM]->(s:Scope) WHERE '+debt
  queries += [('parent-equality','migrated','MATCH (v:ScopeContent)-[:ScopeContent_DERIVED_FROM]->(s:Scope) RETURN count(s),sum(CASE WHEN coalesce(v.content,\'\')=coalesce(s.content,\'\') THEN 1 ELSE 0 END)'),('cohort-parent-equality','migrated',pat+" RETURN count(DISTINCT n),count(DISTINCT s),sum(CASE WHEN coalesce(v.content,'')=coalesce(s.content,'') THEN 1 ELSE 0 END)"),('cohort-new-geometry','migrated',pat+' RETURN n._uuid,s._uuid,n._index,n._start_char,n._end_char,n._core_start_char,n._core_end_char,n._text_hash'),('cohort-old-geometry','migrated',pat+' MATCH (o:Scope_Chunk)-[:Scope_CHUNKED_FROM]->(s) RETURN DISTINCT s._uuid,o._uuid,o._index,o._start_char,o._end_char,o._core_start_char,o._core_end_char,o._text_hash')]
  corrected={}
  for name in ['old-scope','new-scope','new-content']:
   raw=p.DATA/'vector-debt-profile-01'/(name+'.stdout');values=p.rows(raw.read_text())
   if len(values)!=1:raise ValueError('profile rows')
   corrected[name]=dict(raw_sha256=p.digest(raw),profile=p.profile(json.loads(values[0])))
  save('profiles-corrected-from-immutable-raw.json',corrected)
 for base in bases:
  if base=='migrated':continue
  paths=['extension/llm/test/.env.example']+(['extension/lucivy/ld-lucivy/docker-compose.test.yml'] if base=='lourd' else [])
  for i,path in enumerate(paths):
   for table,col,fields in [('File','path','n._uuid,n.path,n.repo_path,n.language,n.size_bytes,n._absent_since'),('Scope','file_path','n._uuid,n.file_path,n.repo_path,n.language,n.start_byte,n.end_byte,n._absent_since')]:
    queries.append((base+'-path'+str(i)+'-'+table,base,"MATCH (n:"+table+") WHERE n."+col+" ENDS WITH '/"+path+"' RETURN "+fields))
 save('queries.json',[dict(name=n,base=str(bases[b]),query=q) for n,b,q in queries]);save('provenance.json',dict(poste=a.poste,tools={str(x):p.digest(x) for x in [Path(__file__),HERE/'diagnose-debt-profile.py',HERE/'inspect-fts.py',query]},qualification='DIAGNOSTIC_RO_ONLY',plaintext_emitted=False))
 start=time.monotonic();end=start+560;error=None;observations={}
 try:
  for name,base,sql in queries:
   remaining=end-time.monotonic()
   if remaining<=5:raise ValueError('deadline')
   result=subprocess.run([str(query),str(bases[base]),sql],env=dict(os.environ,LD_LIBRARY_PATH=str(p.LIB)),capture_output=True,timeout=min(90,remaining-5))
   (root/(name+'.stdout')).write_bytes(result.stdout);(root/(name+'.stderr')).write_bytes(result.stderr);save(name+'-process.json',dict(rc=result.returncode))
   if result.returncode or b'59ac9c86bc2b3529b590a7462ab7051e3ad26b8a' not in result.stderr:raise ValueError('query refused '+name)
   observations[name]=p.rows(result.stdout.decode())
 except Exception as exc:error=exc
 finally:
  for k,v in bases.items():
   after=fs.file_inventory(v.parent);save(k+'-after.json',after)
   if after!=before[k] and error is None:error=ValueError('private source changed '+k)
 save('observations.json',observations)
 save('verdict.json',dict(verdict='RED' if error else 'READ_ONLY_GEOMETRY_METADATA',error=str(error) if error else None,elapsed=time.monotonic()-start,migration_qualified=False))
 if error:raise error
if __name__=='__main__':main()
