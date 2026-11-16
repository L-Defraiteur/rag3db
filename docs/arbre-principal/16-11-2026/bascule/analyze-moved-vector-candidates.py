#!/usr/bin/env python3
"""Pure proof-file analysis: candidate counts do not qualify vector reuse."""
from pathlib import Path
from collections import Counter,defaultdict
import hashlib,json
R=Path.home()/'.cache/rag3weaver-build/bascule-live-20261116'
P=R/'vector-debt-replay-pc-local-01';G=R/'vector-debt-geometry-pc-local-01';D=R/'vector-debt-diagnostic-pc-local-01';OUT=R/'vector-debt-moved-candidates-01'
def digest(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def verified(root,sha):
 manifest=root/'proof-manifest.json'
 if digest(manifest)!=sha:raise ValueError('proof manifest changed')
 for row in json.loads(manifest.read_text()):
  path=root/row['name']
  if path.stat().st_size!=row['size'] or digest(path)!=row['sha256']:raise ValueError('proof file changed')
def stored(name):
 raw=(P/(name+'-stored.stdout')).read_text().splitlines()
 if raw[0]!='BEGIN_QUERY\t0' or not raw[-1].startswith('END_QUERY\t'):raise ValueError('tuple framing')
 rows=[tuple(row[4:].split('|')) for row in raw[1:-1] if row.startswith('ROW\t')]
 if len(rows)!=len(raw)-2 or len(rows)!=int(raw[-1].split('\t')[1]) or any(len(row)!=7 for row in rows):raise ValueError('tuple rows')
 return Counter(rows)
def main():
 for root,sha in [(P,'6d4667936fc876e4a415d5cd12698486f0d231b7f8ebd1cca57250b475868cc6'),(G,'dd5157b51fac6d4ed1877585655039ac30ca41b4f46f3841a8192f37088c2568'),(D,'5447d16abcb0bf27b117522fca27354ae30dcaf6086cc1552055f3002236a28c')]:verified(root,sha)
 a=stored('BEFORE');b=stored('AFTER');new_only=b-a;by=defaultdict(list)
 for row in a.elements():by[(row[0],row[-1])].append(row)
 geo=json.loads((G/'observations.json').read_text());ids={(r[1],*r[2:]):r[0] for r in [v.split('|') for v in geo['cohort-new-geometry']]}
 if len(ids)!=469 or Counter(ids.keys())!=new_only:raise ValueError('debt/new-only not exact')
 model=json.loads((D/'model-and-pins.json').read_text());obs=json.loads((D/'observations.json').read_text())
 if model['model']!=dict(name='granite-278m',dim=768,storage='suffixed') or obs['old-debt']!=0 or obs['new-old-debt']!=0:raise ValueError('model/old debt proof differs')
 rows=[]
 for row in new_only.elements():
  candidates=by[(row[0],row[-1])]
  if not candidates:continue
  rows.append(dict(debt_uuid=ids[row],parent=row[0],new_index=int(row[1]),hash=row[-1],new_geometry=list(map(int,row[2:6])),old_candidates=[dict(index=int(x[1]),geometry=list(map(int,x[2:6])),same_index=x[1]==row[1],same_full_span=x[2:4]==row[2:4],same_core_span=x[4:6]==row[4:6]) for x in candidates],same_parent_field='content (both stored queries explicitly filter it)',stored_title_comparison=None,stored_literal_text_comparison=None,replayed_input_range_equal=all(x[2:4]==row[2:4] for x in candidates),actual_vector_dimension=None,vector_bytes_or_conflicts=None,hash_collision_excluded=False,qualified_reuse=False))
 summary=dict(debt_new_only=sum(new_only.values()),same_parent_hash_candidates=len(rows),without_same_parent_hash_candidate=sum(new_only.values())-len(rows),candidate_multiplicities=dict(Counter(len(x['old_candidates']) for x in rows)),same_full_span=sum(any(c['same_full_span'] for c in x['old_candidates']) for x in rows),same_core_span=sum(any(c['same_core_span'] for c in x['old_candidates']) for x in rows),same_index=sum(any(c['same_index'] for c in x['old_candidates']) for x in rows),ambiguous_candidates=sum(len(x['old_candidates'])>1 for x in rows),qualified_reuse=0,actual_vector_dimensions_observed=False,vector_conflicts_observed=False,stored_literal_text_compared=False,title_compared=False,model_registry=model['model'],old_vector_and_marker_debt=0,scope='same parent and content field only; no global hash lookup',database_opened=False,embedding_called=False)
 OUT.mkdir(mode=0o700)
 (OUT/'candidates.json').write_text(json.dumps(rows,indent=2));(OUT/'summary.json').write_text(json.dumps(summary,indent=2));(OUT/'provenance.json').write_text(json.dumps(dict(script_sha256=digest(Path(__file__)),proof_manifests={str(root):digest(root/'proof-manifest.json') for root in [P,G,D]},model_proof_sha256=digest(D/'model-and-pins.json'),old_debt_proof_sha256=digest(D/'observations.json')),indent=2))
 files=[dict(name=p.name,size=p.stat().st_size,sha256=digest(p)) for p in sorted(OUT.iterdir())]
 (OUT/'proof-manifest.json').write_text(json.dumps(files,indent=2));print(json.dumps(summary));print('manifest',digest(OUT/'proof-manifest.json'))
if __name__=='__main__':main()
