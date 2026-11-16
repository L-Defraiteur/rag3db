#!/usr/bin/env python3
"""Deterministic selection from immutable inventories; sources are never executed."""
import hashlib,json,sys
from pathlib import Path
from collections import Counter,defaultdict
base=Path(sys.argv[1]).resolve();out=Path(sys.argv[2]).resolve();out.mkdir(parents=True,exist_ok=False)
seed='rag3db-public-corpus-20261116-v1';groups=defaultdict(list);roots={};excluded=Counter();receipts=[];occurrences=[]
audit_path=base/'license-audit.json';audits=json.loads(audit_path.read_text()) if audit_path.exists() else {}
for fp in sorted((base/'repositories').glob('*/freeze.json')):
 receipt=json.loads(fp.read_text());receipts.append(receipt);rid=receipt['repository_id'];roots[rid]=receipt['root'];inv=fp.parent/'inventory.jsonl'
 assert hashlib.sha256(inv.read_bytes()).hexdigest()==receipt['inventory_sha256']
 lic=audits.get(rid,{}).get('license_id',receipt['license_id']);proof=receipt['license_evidence'];license_path=audits.get(rid,{}).get('license_path') or next((x['path'] for x in proof if '/' not in x['path'] and x['path'].lower().startswith(('license','copying'))),None)
 for line in inv.read_text().splitlines():
  r=json.loads(line)
  if not r.get('language'):continue
  reason=r.get('excluded_reason')
  if not reason and r.get('encoding')!='utf-8':reason='invalid_utf8'
  if not reason and r.get('third_party'):reason='third_party_separate'
  if not reason and r.get('generated'):reason='generated_separate'
  if not reason and audits.get(rid,{}).get('allowed_prefixes') and not any(r['relative_path'].startswith(prefix) for prefix in audits[rid]['allowed_prefixes']):reason='outside_reviewed_license_scope'
  if not reason and (not lic or lic in ['NOASSERTION','OTHER'] or not license_path):reason='license_unresolved'
  r['selection_excluded_reason']=reason;occurrences.append(r)
  if reason:excluded[r['language']+':'+reason]+=1;continue
  r['license_id']=lic;r['license_path']=license_path
  r['selection_rank']=hashlib.sha256((seed+'\0'+rid+'\0'+r['relative_path']).encode()).hexdigest()
  groups[r['language']].append(r)
selected=[];summary={}
for lang,rows in sorted(groups.items()):
 unique=[];seen=set()
 for r in sorted(rows,key=lambda r:r['selection_rank']):
  key=(r['language'],r['facade'],r['dialect'],r['sha256'])
  if key in seen:continue
  seen.add(key);unique.append(r)
 picks=[]
 for stratum,n in [('production',1800),('test',500),('documentation',200)]:picks.extend([r for r in unique if r['stratum']==stratum][:n])
 ids={(r['repository_id'],r['relative_path']) for r in picks}
 picks.extend([r for r in unique if (r['repository_id'],r['relative_path']) not in ids][:2500-len(picks)])
 for r in picks:
  p=Path(roots[r['repository_id']])/r['relative_path'];raw=p.read_bytes();assert len(raw)==r['size_bytes'] and hashlib.sha256(raw).hexdigest()==r['sha256'];raw.decode('utf-8')
  selected.append({k:r[k] for k in ['repository_id','repository_url','repository_commit','license_id','license_path','relative_path','sha256','language','facade','stratum','size_bytes']})
 summary[lang]={'admissible_occurrences':len(rows),'unique_route_dialect_contents':len(unique),'selected':len(picks),'selected_unique_sha256':len({r['sha256'] for r in picks}),'strata':dict(Counter(r['stratum'] for r in picks)),'dialects':dict(Counter(r['dialect'] for r in picks)),'intentionally_invalid_candidates':sum(r['intentionally_invalid_candidate'] for r in picks),'under_target':len(picks)<2000}
manifest=out/'selected.jsonl';manifest.write_text(''.join(json.dumps(r,sort_keys=True,ensure_ascii=False)+'\n' for r in sorted(selected,key=lambda r:(r['language'],r['repository_id'],r['relative_path']))))
(out/'roots.json').write_text(json.dumps({'schema_version':1,'repositories':roots},indent=2)+'\n')
(out/'occurrences.jsonl').write_text(''.join(json.dumps(r,sort_keys=True,ensure_ascii=False)+'\n' for r in occurrences))
result={'seed':seed,'repositories':len(receipts),'repository_pins':[r['repository_id'] for r in receipts],'selected_manifest_sha256':hashlib.sha256(manifest.read_bytes()).hexdigest(),'roots_sha256':hashlib.sha256((out/'roots.json').read_bytes()).hexdigest(),'occurrences_sha256':hashlib.sha256((out/'occurrences.jsonl').read_bytes()).hexdigest(),'selected_files':len(selected),'selected_unique_content_sha256':len({r['sha256'] for r in selected}),'cohorts':summary,'excluded':dict(excluded),'parser_executed':False}
(out/'summary.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result))
