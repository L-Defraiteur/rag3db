"""Prepare complete owned snapshot. No thematic or color filtering at ingestion."""
import json,re,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3];P=ROOT/'experiments/mtga/data';B=ROOT/'experiments/mtga/backend'
sys.path.insert(0,str(P.parent));from identity import card_id
from card_rows import build_rows, card_schema
# La collection est le catalogue restreint aux impressions possédées : même fiche,
# mêmes champs (sous-types, rareté, possession par nom, faits de capacité…).
schema=card_schema();props=schema['properties']
(B/'schemas/card.json').write_text(json.dumps(schema,indent=2,ensure_ascii=False))
cs=list(map(json.loads,(P/'cards.jsonl').open()));snapshot=json.loads((P/'card-records.jsonl').open().readline())['snapshot_id']
rows=[r for r in build_rows(cs,snapshot) if r['owned']>0]
(P/'engine-collection.jsonl').write_text(''.join(json.dumps(r,ensure_ascii=False)+'\n' for r in rows))
crate=ROOT/'extension/rag3weaver';manifest={'search_graphs':True,'fts_positions':False,'version':1,'name':'magic_collection','database':str(P/'engine-search-lucivy43-recovered.rag3db'),'embeddings':{'address':'127.0.0.1:7878','model':'bge-m3','dimensions':1024},'vector_extension':str(ROOT/'extension/vector/build/libvector.rag3db_extension'),'entities':{'OwnedCard':{'schema':'schemas/card.json','config':{'fields':{'name_en':{'type':'string','isTitle':True},'text':{'type':'string','isContent':True}},'hashsafe':['key'],'returnFields':list(props),'signals':['bm25','vector'],'contentKind':'record'}}},'tools':{'ingest_cards':{'graph':'graphs/ingest.mmd','bindings':{'entity':'OwnedCard'}},'select_cards':{'graph':'graphs/select.mmd','bindings':{'entity':'OwnedCard'}},'search_cards':{'graph':str(crate/'templates/tools/search_structured.mmd'),'bindings':{'target':'OwnedCard'},'metadata':[{'node':'render','port':'meta'}]}}}
(B/'backend.json').write_text(json.dumps(manifest,indent=2))
print(json.dumps({'records':len(rows),'copies':sum(r['owned'] for r in rows),'lands':sum(r['is_land'] for r in rows),'snapshot':snapshot}))

# Keep the extended model when regenerating the base descriptor.
import runpy
runpy.run_path(str(Path(__file__).with_name("prepare_engine_relations.py")))

runpy.run_path(str(Path(__file__).with_name("prepare_engine_catalog.py")), run_name="__main__")
