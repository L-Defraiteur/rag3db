"""Independent verification only: engine query outputs versus source snapshot."""
import json
from pathlib import Path
p=Path(__file__).resolve().parents[1]/'data';rows=list(map(json.loads,(p/'engine-collection.jsonl').open()))
checks={'inventory':lambda r:True,'creatures':lambda r:'Creature' in r['card_types'],'green_lands':lambda r:r['is_land'] and 'G' in r['mana_symbols_possible'],'red_black_lands':lambda r:r['is_land'] and {'R','B'}<=set(r['mana_symbols_possible']),'red_black_treasures':lambda r:not r['is_land'] and not set(r['color_identity'])&{'White','Blue','Green'} and 'Treasure' in r['text_en'],'nested_treasure':lambda r:not r['is_land'] and not set(r['color_identity'])&{'White','Blue','Green'} and any('Treasure' in a['text_en'] for a in r['abilities'])}
report={}
for name,fn in checks.items():
 expected={r['arena_id'] for r in rows if fn(r)};hits=json.loads((p/'engine-results'/f'{name}.json').read_text())['result'];actual={r['data']['arena_id'] for r in hits}
 assert len(hits)==len(actual),(name,'duplicate identities')
 assert actual==expected,(name,'missing',expected-actual,'extra',actual-expected)
 report[name]={'expected':len(expected),'actual':len(actual),'missing':0,'extra':0}
 if name=='inventory':
  originals={r['arena_id']:r for r in rows}
  for hit in hits:
   data=hit['data']
   for key,value in originals[data['arena_id']].items():assert data[key]==value,(data['arena_id'],key)
for name in ['bm25','vector','hybrid']:
 x=json.loads((p/'engine-results'/f'{name}_treasures.json').read_text());assert len(x['result'])==20
 for h in x['result']:
  assert not h['data']['is_land'] and not set(h['data']['color_identity'])&{'White','Blue','Green'}
 meta=x['metadata']['render.meta'];assert not meta.get('partial',False)
 if name in ['vector','hybrid']:assert meta['vectorCount']>0
 if name in ['bm25','hybrid']:assert meta['bm25Count']>0
 report[name]={'returned':len(x['result']),'meta':meta}
report['snapshot_id']=rows[0]['snapshot_id'];report['copies']=sum(r['owned'] for r in rows)
(p/'engine-validation.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n')
print('PASS: complete persisted payload roundtrip; exact typed and nested selections; filtered BM25, dense and hybrid searches.')
