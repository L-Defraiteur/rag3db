"""Prepare complete owned snapshot. No thematic or color filtering at ingestion."""
import json,re,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3];P=ROOT/'experiments/mtga/data';B=ROOT/'experiments/mtga/backend'
sys.path.insert(0,str(P.parent));from identity import card_id
colors={'White':'W','Blue':'U','Black':'B','Red':'R','Green':'G'}
base_types={'Plains':'W','Island':'U','Swamp':'B','Mountain':'R','Forest':'G'}
props={k:{'type':'string'} for k in ['key','snapshot_id','name_en','name_fr','type_en','text_en','text_fr','text','mana_cost','set','collector_number','mana_conditions']}
props.update({k:{'type':'integer'} for k in ['arena_id','owned']})
props.update({k:{'type':'boolean'} for k in ['is_land','has_land_face','is_primary','is_token','mana_has_conditions']})
props.update({k:{'type':'array','items':{'type':'string'}} for k in ['colors','color_identity','card_types','land_types','mana_symbols_possible','mana_symbols_explicit','linked_card_ids']})
props['abilities']={'type':'array','items':{'type':'object','additionalProperties':False,'required':['ability_id','text_en','text_fr'],'properties':{'ability_id':{'type':'integer'},'text_en':{'type':'string'},'text_fr':{'type':'string'}}}}
schema={'type':'object','additionalProperties':False,'properties':props,'required':list(props)}
(B/'schemas/card.json').write_text(json.dumps(schema,indent=2))
cs=list(map(json.loads,(P/'cards.jsonl').open()));byid={c['arena_id']:c for c in cs};snapshot=json.loads((P/'card-records.jsonl').open().readline())['snapshot_id'];rows=[]
for c in cs:
 if c['owned']<=0:continue
 types=c['type_en'].split(' — ')[0].split();island='Land' in types;sub=c['type_en'].split(' — ')[1].split() if ' — ' in c['type_en'] else []
 mana_lines=[line for line in c['text_en'].splitlines() if re.search(r'\badd\b',line,re.I)] if island else []
 explicit=set()
 for line in mana_lines:
  clause=re.split(r'\badd\b',line,maxsplit=1,flags=re.I)[1].split('.')[0]
  explicit.update(re.findall(r'\{([WUBRGC])\}',clause))
 explicit.update(base_types[t] for t in sub if t in base_types and island)
 possible=set(explicit)
 if any('any color' in l.lower() or 'chosen color' in l.lower() for l in mana_lines):possible.update('WUBRG')
 # Chosen-color and other dynamic outputs are possibilities, not unconditional sources.
 conditional=any(any(x in l.lower() for x in ['only','if ','chosen','choose','among','spend','could produce','that color','colors of']) for l in mana_lines) or (island and 'choose a color' in c['text_en'].lower())
 row={k:c[k] for k in ['arena_id','owned','name_en','name_fr','type_en','text_en','text_fr','mana_cost','set','collector_number','colors','color_identity','is_primary','is_token']}
 row.update(key=card_id(c['arena_id']),snapshot_id=snapshot,text=c['name_en']+' / '+c['name_fr']+'\n'+c['type_en']+'\n'+c['text_en']+'\n'+c['text_fr'],is_land=island,has_land_face=island or any('Land' in byid.get(i,{}).get('type_en','').split(' — ')[0].split() for i in c['linked_faces']),card_types=[t for t in types if t not in ['Legendary','Basic','Snow','World','Ongoing']],land_types=sub if island else [],mana_symbols_possible=sorted(possible),mana_symbols_explicit=sorted(explicit),mana_has_conditions=conditional,mana_conditions=c['text_en'] if island else '',linked_card_ids=[card_id(i) for i in c['linked_faces']],abilities=[{k:a[k] for k in ['ability_id','text_en','text_fr']} for a in c['abilities']])
 rows.append(row)
(P/'engine-collection.jsonl').write_text(''.join(json.dumps(r,ensure_ascii=False)+'\n' for r in rows))
crate=ROOT/'extension/rag3weaver';manifest={'search_graphs':True,'fts_positions':False,'version':1,'name':'magic_collection','database':str(P/'engine-search-lucivy43-recovered.rag3db'),'embeddings':{'address':'127.0.0.1:7878','model':'bge-m3','dimensions':1024},'vector_extension':str(ROOT/'extension/vector/build/libvector.rag3db_extension'),'entities':{'OwnedCard':{'schema':'schemas/card.json','config':{'fields':{'name_en':{'type':'string','isTitle':True},'text':{'type':'string','isContent':True}},'hashsafe':['key'],'returnFields':list(props),'signals':['bm25','vector']}}},'tools':{'ingest_cards':{'graph':'graphs/ingest.mmd','bindings':{'entity':'OwnedCard'}},'select_cards':{'graph':'graphs/select.mmd','bindings':{'entity':'OwnedCard'}},'search_cards':{'graph':str(crate/'templates/tools/search_structured.mmd'),'bindings':{'target':'OwnedCard'},'metadata':[{'node':'render','port':'meta'}]}}}
(B/'backend.json').write_text(json.dumps(manifest,indent=2))
print(json.dumps({'records':len(rows),'copies':sum(r['owned'] for r in rows),'lands':sum(r['is_land'] for r in rows),'snapshot':snapshot}))

# Keep the extended model when regenerating the base descriptor.
import runpy
runpy.run_path(str(Path(__file__).with_name("prepare_engine_relations.py")))

runpy.run_path(str(Path(__file__).with_name("prepare_engine_catalog.py")), run_name="__main__")
