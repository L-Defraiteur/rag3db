#!/usr/bin/env python3
"""Freeze public GitHub sources as inert data; never execute repository files."""
import concurrent.futures, hashlib, json, os, sys, tarfile, time, urllib.request
from pathlib import Path, PurePosixPath

BASE = Path(sys.argv[1]).resolve()
REPOS = sys.argv[2:]
MAX_ARCHIVE = 180 * 1024 * 1024
MAX_FILE = 1024 * 1024
MAX_REPO = 768 * 1024 * 1024
EXTENSIONS = {
 '.ts':('typescript','scope'), '.tsx':('typescript','scope'),
 '.js':('javascript','scope'), '.jsx':('javascript','scope'), '.mjs':('javascript','scope'), '.cjs':('javascript','scope'),
 '.py':('python','scope'), '.pyi':('python','scope'), '.rs':('rust','scope'), '.go':('go','scope'), '.c':('c','scope'),
 **{e:('cpp','scope') for e in ['.h','.cpp','.cc','.cxx','.hpp','.hxx']}, '.cs':('csharp','scope'),
 '.css':('css','css'), '.scss':('scss','scss'), '.sass':('sass','scss'), '.vue':('vue','vue'),
 '.svelte':('svelte','svelte'), '.md':('markdown','markdown'), '.mdx':('mdx','markdown'),
 '.html':('html','html'), '.htm':('html','html'),
}
HEADERS = {'User-Agent':'rag3db-public-corpus-freeze/1','Accept':'application/vnd.github+json'}

def get_json(url):
 with urllib.request.urlopen(urllib.request.Request(url,headers=HEADERS),timeout=45) as r:return json.load(r)

def secret(parts):
 for part in parts:
  b=part.lower()
  if b in ['.git','.vault','.netrc','.npmrc','.pypirc','credentials.json','service-account.json'] or b.startswith('.env') or b.startswith(('id_rsa','id_ed25519')) or b.endswith(('.pem','.key','.p12','.pfx','.jks','.keystore')):return True
 return False

def freeze(repo):
 meta=get_json('https://api.github.com/repos/'+repo)
 pin=get_json('https://api.github.com/repos/'+repo+'/commits/HEAD')['sha']
 assert len(pin)==40 and all(c in '0123456789abcdef' for c in pin)
 owner,name=meta['full_name'].split('/'); rid='github.com/'+owner.lower()+'/'+name.lower()+'@'+pin
 folder=BASE/'repositories'/(owner+'--'+name+'--'+pin)
 if folder.exists():
  receipt=json.loads((folder/'freeze.json').read_text());return {'existing':True,**receipt}
 folder.mkdir(parents=True,exist_ok=False); root=folder/'source';root.mkdir()
 archive=folder/'source.tar.gz';digest=hashlib.sha256();compressed=0
 with urllib.request.urlopen(urllib.request.Request('https://codeload.github.com/'+repo+'/tar.gz/'+pin,headers=HEADERS),timeout=90) as response,archive.open('xb') as f:
  while data:=response.read(1024*1024):
   compressed+=len(data)
   if compressed>MAX_ARCHIVE:raise RuntimeError(repo+': archive size cap reached; partial archive preserved')
   digest.update(data);f.write(data)
 license_id=(meta.get('license') or {}).get('spdx_id')
 records=[];licenses=[];total=0;counts={};seen=set()
 with tarfile.open(archive,'r:gz') as tf:
  for member in tf:
   parts=PurePosixPath(member.name).parts
   if not parts or member.isdir():continue
   if member.name.startswith('/') or '..' in parts:raise RuntimeError('unsafe archive path')
   rel=PurePosixPath(*parts[1:]);s=str(rel)
   if not s or s=='.':continue
   if s in seen:raise RuntimeError('duplicate archive path '+s)
   seen.add(s);row={'repository_id':rid,'repository_url':meta['html_url'],'repository_commit':pin,'relative_path':s,'size_bytes':member.size}
   reason=None
   if secret(rel.parts):reason='secret_or_git_metadata_not_read'
   elif not member.isfile():reason='symlink_or_special_not_followed'
   elif member.size>MAX_FILE:reason='file_above_1MiB'
   elif any(x in ['node_modules','target','.venv','__pycache__'] for x in rel.parts):reason='build_or_dependency_tree'
   if reason:row['excluded_reason']=reason;records.append(row);continue
   total+=member.size
   if total>MAX_REPO:raise RuntimeError(repo+': expanded data cap reached')
   with tf.extractfile(member) as f:data=f.read()
   assert len(data)==member.size
   dest=root.joinpath(*rel.parts);dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(data)
   row['sha256']=hashlib.sha256(data).hexdigest();row['stored']=True
   try:text=data.decode('utf-8');row['encoding']='utf-8'
   except UnicodeDecodeError:text=None;row['encoding']='invalid_utf8'
   ext=rel.suffix.lower();language,facade=EXTENSIONS.get(ext,(None,None));row.update(language=language,facade=facade)
   lower=[x.lower() for x in rel.parts]
   test=any(x in ['test','tests','testing','testdata','fixtures','samples','examples','spec','specs','__tests__'] for x in lower) or rel.name.lower().endswith(('_test.go','test.cs','tests.cs','.test.ts','.spec.ts','.test.js','.spec.js'))
   doc=language in ['markdown','mdx'] or any(x in ['docs','doc','documentation'] for x in lower)
   row['stratum']='documentation' if doc else 'test' if test else 'production'
   row['generated']=any(x in ['generated','dist','baselines'] for x in lower) or (text is not None and any(x in text[:2048] for x in ['@generated','DO NOT EDIT','AUTO-GENERATED']))
   row['third_party']=any(x in ['third_party','third-party','vendor','deps','external'] for x in lower)
   row['intentionally_invalid_candidate']=test and any(x in s.lower() for x in ['invalid','malformed','negative','badsyntax','syntaxerror','errorcases'])
   row['dialect']=ext.lstrip('.')
   records.append(row)
   if language:counts[language]=counts.get(language,0)+1
   if rel.name.lower().startswith(('license','licence','copying','copyright')) and len(rel.parts)<=2:licenses.append({'path':s,'sha256':row['sha256']})
 inv=folder/'inventory.jsonl';inv.write_text(''.join(json.dumps(x,sort_keys=True,ensure_ascii=False)+'\n' for x in sorted(records,key=lambda x:x['relative_path'])))
 receipt={'repository_id':rid,'repository_url':meta['html_url'],'repository_commit':pin,'root':str(root),'license_id':license_id,'license_evidence':licenses,'github_metadata_license_is_not_per_file_audit':True,'archive_sha256':digest.hexdigest(),'archive_bytes':compressed,'stored_bytes':total,'inventory_sha256':hashlib.sha256(inv.read_bytes()).hexdigest(),'entries':len(records),'language_occurrences':counts,'executed_repository_code':False,'frozen_unix':time.time()}
 (folder/'metadata.json').write_text(json.dumps(meta,indent=2)+'\n');(folder/'freeze.json').write_text(json.dumps(receipt,indent=2)+'\n');return receipt

BASE.mkdir(parents=True,exist_ok=True)
with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool:
 futures={pool.submit(freeze,r):r for r in REPOS}
 for f in concurrent.futures.as_completed(futures):
  repo=futures[f]
  try:result=f.result()
  except Exception as error:
   print(json.dumps({'repository':repo,'error':str(error),'partial_data_preserved':True}),flush=True)
   for other in futures:other.cancel()
   raise
  print(json.dumps(result),flush=True)
