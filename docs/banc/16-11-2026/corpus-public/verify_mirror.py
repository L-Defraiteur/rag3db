#!/usr/bin/env python3
"""Verify inert corpus receipts, archives, inventory and selected bytes on a mirror."""
import hashlib,json,sys
from pathlib import Path
base=Path(sys.argv[1]).resolve();selection=base/'selection-02';expected=sys.argv[2];output=Path(sys.argv[3]);output.parent.mkdir(parents=True,exist_ok=True)
def sha(p):
 h=hashlib.sha256()
 with p.open('rb') as f:
  while data:=f.read(1024*1024):h.update(data)
 return h.hexdigest()
manifest=selection/'selected.jsonl';assert sha(manifest)==expected
roots=json.loads((selection/'roots.json').read_text());assert roots['schema_version']==1;receipts={};repo_proofs=[]
for p in sorted((base/'repositories').glob('*/freeze.json')):
 m=json.loads(p.read_text());rid=m['repository_id'];assert rid not in receipts;root=Path(m['root']);assert root.is_absolute() and root.is_dir() and root==p.parent/'source' and str(root)==roots['repositories'][rid]
 assert sha(p.parent/'source.tar.gz')==m['archive_sha256'];assert sha(p.parent/'inventory.jsonl')==m['inventory_sha256']
 receipts[rid]=m;repo_proofs.append({'repository_id':rid,'freeze_sha256':sha(p),'archive_sha256':m['archive_sha256'],'inventory_sha256':m['inventory_sha256'],'root_verified':True})
assert set(receipts)==set(roots['repositories'])
count=0;cohorts={};seen=set();license_proofs={}
with manifest.open() as lines:
 for line in lines:
  r=json.loads(line);rid=r['repository_id'];rel=Path(r['relative_path']);assert not rel.is_absolute() and '..' not in rel.parts and (rid,str(rel)) not in seen;seen.add((rid,str(rel)))
  root=Path(roots['repositories'][rid]);p=root/rel;assert p.resolve().is_relative_to(root.resolve()) and not p.is_symlink();assert p.stat().st_size==r['size_bytes'] and sha(p)==r['sha256'];p.read_bytes().decode('utf-8')
  lp=root/r['license_path'];assert lp.is_file() and lp.resolve().is_relative_to(root.resolve());license_proofs[(rid,r['license_path'])]=sha(lp)
  count+=1;cohorts[r['language']]=cohorts.get(r['language'],0)+1
result={'status':'MIRROR_READY_BYTES_VERIFIED_NO_PARSING','repositories':len(receipts),'selected_files':count,'manifest_sha256':expected,'roots_sha256':sha(selection/'roots.json'),'occurrences_sha256':sha(selection/'occurrences.jsonl'),'cohorts':cohorts,'repository_proofs':repo_proofs,'license_files_verified':len(license_proofs),'parsing_executed':False}
raw=(json.dumps(result,indent=2)+'\n').encode();output.write_bytes(raw);print(json.dumps({'status':result['status'],'repositories':len(receipts),'selected_files':count,'roots_sha256':result['roots_sha256'],'proof_sha256':hashlib.sha256(raw).hexdigest()}))
