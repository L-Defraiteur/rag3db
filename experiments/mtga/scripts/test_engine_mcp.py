"""Exercise discovery, arbitrary graphs and dense ability search via real MCP stdio.
Run only when no other host owns the persistent Magic database.
"""
import asyncio,json,os
from pathlib import Path
from mcp import ClientSession,StdioServerParameters
from mcp.client.stdio import stdio_client
ROOT=Path(__file__).resolve().parents[3];P=ROOT/'experiments/mtga/data';B=ROOT/'experiments/mtga/backend';C=ROOT/'extension/rag3weaver'
async def main():
 params=StdioServerParameters(command='bash',args=[str(ROOT/'experiments/mtga/scripts/serve_engine_mcp.sh')],env=dict(os.environ))
 report={}
 async with stdio_client(params) as streams:
  async with ClientSession(*streams) as session:
   await session.initialize()
   tools=await session.list_tools();names={t.name for t in tools.tools}
   assert {'describe_backend','validate_search','run_search','search_ability'}<=names
   async def call(name,args):
    r=await session.call_tool(name,{**args,'response_format':'json'})
    assert not r.isError,(name,r)
    assert r.structuredContent is not None,(name,r)
    return r.structuredContent
   description=await call('describe_backend',{})
   assert description['entities']['Ability']['signals']==['bm25','vector']
   assert description['relations']['CardAbility']=={'from':'OwnedCard','to':'Ability','properties':None}
   assert description['backend']['embeddings']['model']=='bge-m3'
   (P/'engine-mcp-description.json').write_text(json.dumps(description,ensure_ascii=False,indent=2))
   mermaid=(C/'templates/tools/select_structured.mmd').read_text()
   inventory=await call('run_search',{'mermaid':mermaid,'arguments':{'entity':'OwnedCard','filter':{'must':[]}}})
   expected=[json.loads(l) for l in (P/'engine-collection.jsonl').open()]
   assert len(inventory['result'])==5380
   assert {r['data']['key'] for r in inventory['result']}=={r['key'] for r in expected}
   report['owned_records']=len(inventory['result'])
   request=json.loads((B/'queries/nested_treasure.json').read_text())
   args={'mermaid':mermaid,'arguments':{'entity':'OwnedCard',**request['arguments']}}
   valid=await call('validate_search',args);assert valid['valid']
   result=await call('run_search',args)
   assert result['graph_hash']==valid['graph_hash']
   reference=json.loads((P/'engine-results/nested_treasure.json').read_text())['result']
   assert {r['uuid'] for r in result['result']}=={r['uuid'] for r in reference}
   report['nested_treasure_matches']=len(result['result'])
   for entity in ['Ability','Mechanic','Deck','DeckEntry']:
    found=await call('run_search',{'mermaid':mermaid,'arguments':{'entity':entity,'filter':{'must':[]}}})
    source=[json.loads(l) for l in (P/f'engine-{entity}.jsonl').open()]
    assert {r['data']['key'] for r in found['result']}=={r['key'] for r in source}
    report[entity]=len(found['result'])
   dense=await call('search_ability',{'query':'ramener une créature du cimetière sur le champ de bataille','options':{'signals':['vector'],'limit':10}})
   assert len(dense['result'])==10
   assert dense['metadata']['render.meta']['vectorCount']>0 and not dense['metadata']['render.meta']['partial']
   assert all(r['entity']=='Ability' for r in dense['result'])
   (P/'engine-ability-dense-results.json').write_text(json.dumps(dense,ensure_ascii=False,indent=2))
   report['dense_ability_hits']=len(dense['result'])
   # One graph: semantic ability hits -> linked owned cards; no Python filtering.
   graph=(C/'templates/tools/search_dense_related.mmd').read_text()
   args={'mermaid':graph,'arguments':{'target':'Ability','relation':'CardAbility','direction':'Incoming','query':'ramener une créature du cimetière sur le champ de bataille','options':{'signals':['vector'],'limit':10}}}
   await call('validate_search',args)
   related=await call('run_search',args)
   assert related['result'] and all(r['entity']=='OwnedCard' for r in related['result'])
   assert len({r['uuid'] for r in related['result']})==len(related['result'])
   (P/'engine-ability-dense-cards.json').write_text(json.dumps(related,ensure_ascii=False,indent=2))
   report['dense_related_cards']=len(related['result'])
   # Exhaustive definition/ability/card union, intersected with one captured deck.
   def contains(field,text):return {'field':{'key':field,'value':[{'op':'contains','value':text}]}}
   deck_id='df511012-2ab2-47e9-846d-8351e56cbc35'
   filters={'mechanic_filter':contains('text','graveyard'),'ability_filter':contains('text_en','graveyard'),'card_filter':contains('text_en','graveyard'),'scope_filter':{'field':{'key':'arena_deck_id','value':deck_id}},'options':{'limit':1000},'duplicates':'merge'}
   scoped=await call('search_deck_mechanics',filters)
   mechanisms=[json.loads(l) for l in (P/'engine-Mechanic.jsonl').open()]
   ability_rows=[json.loads(l) for l in (P/'engine-Ability.jsonl').open()]
   deck_entries=[json.loads(l) for l in (P/'engine-DeckEntry.jsonl').open()]
   relevant_m={r['key'] for r in mechanisms if 'graveyard' in r['text']}
   relevant_a={r['key'] for r in ability_rows if 'graveyard' in r['text_en'] or relevant_m.intersection(r['glossary_ids'])}
   linked={e['from']['key'] for e in json.loads((P/'engine-links.json').read_text())['CardAbility'] if e['to']['key'] in relevant_a}
   scope={r['card_key'] for r in deck_entries if r['arena_deck_id']==deck_id and r['in_owned_collection']}
   expected_keys=scope.intersection(linked | {r['key'] for r in expected if 'graveyard' in r['text_en']})
   assert expected_keys and {r['data']['key'] for r in scoped['result']}==expected_keys
   assert len(scoped['result'])==len(expected_keys)
   composed=(C/'templates/tools/search_related_scoped.mmd').read_text()
   bindings=json.loads((B/'backend.json').read_text())['tools']['search_deck_mechanics']['bindings']
   scoped_dynamic=await call('run_search',{'mermaid':composed,'arguments':{**bindings,**filters}})
   assert {r['uuid'] for r in scoped_dynamic['result']}=={r['uuid'] for r in scoped['result']}
   report['scoped_graveyard_union']=len(expected_keys)
   (P/'engine-scoped-graveyard.json').write_text(json.dumps(scoped_dynamic,ensure_ascii=False,indent=2))
   # Replaying a real snapshot link is idempotent; a missing identity is rejected.
   pair=json.loads((P/'engine-links.json').read_text())['CardAbility'][0]
   await call('link_cardability',{'links':[pair,pair]})
   missing=await session.call_tool('link_cardability',{'links':[pair,{'from':{'key':'missing-endpoint'},'to':pair['to']}]})
   assert missing.isError
   # Rejected requests must not break the protocol or perform writes.
   for bad in [
    {'mermaid':mermaid,'arguments':{'entity':'Missing','filter':{'must':[]}}},
    {'mermaid':(C/'templates/tools/ingest_snapshot.mmd').read_text(),'arguments':{'entity':'OwnedCard','records':[]}},
    {'mermaid':mermaid,'arguments':{'entity':'OwnedCard','filter':{'must':[]}},'metadata':[{'node':'select','port':'missing'}]},
   ]:
    r=await session.call_tool('run_search',bad);assert r.isError,r
   await call('describe_backend',{})
   report['rejections_recoverable']=True
   closed=await session.call_tool('close_backend',{});assert not closed.isError,closed
 (P/'engine-mcp-validation.json').write_text(json.dumps(report,indent=2)+'\n')
 print(json.dumps(report,indent=2))
asyncio.run(main())
