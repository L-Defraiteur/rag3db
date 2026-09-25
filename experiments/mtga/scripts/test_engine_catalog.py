"""Verify persisted catalog completeness and combined filters through real MCP."""
import asyncio,json,os
from pathlib import Path
from mcp import ClientSession,StdioServerParameters
from mcp.client.stdio import stdio_client
ROOT=Path(__file__).resolve().parents[3];P=ROOT/'experiments/mtga/data'
def field(k,v):return {'field':{'key':k,'value':v}}
def op(k,operation,v):return field(k,[{'op':operation,'value':v}])
def array(k,operation,v):return {'path':{'path':[k],'value':[{'op':operation,'value':v}]}}
async def main():
 params=StdioServerParameters(command='bash',args=[str(ROOT/'experiments/mtga/scripts/serve_engine_mcp.sh')],env=dict(os.environ))
 report={};out=P/'catalog-research';out.mkdir(exist_ok=True)
 manifest=json.loads((P.parent/'backend/backend.json').read_text())
 assert manifest['fts_positions'] is False
 report['database']=manifest['database'];report['fts_positions_for_new_indexes']=False
 async with stdio_client(params) as streams:
  async with ClientSession(*streams) as session:
   await session.initialize()
   async def call(tool,args):
    result=await session.call_tool(tool,{**args,'response_format':'json'})
    assert not result.isError,(tool,result)
    return result.structuredContent
   tools={t.name for t in (await session.list_tools()).tools}
   assert {'select_catalog','search_catalog','search_catalogability','search_catalog_ability_cards'}<=tools
   # Exact payload equality after reopening the database, bounded response sizes.
   for entity,tool in [('CatalogCard','select_catalog'),('CatalogAbility','select_catalogability')]:
    rows=[json.loads(s) for s in (P/f'engine-{entity}.jsonl').open()]
    for offset in range(0,len(rows),256):
     batch=rows[offset:offset+256]
     found=await call(tool,{'filter':op('key','in',[r['key'] for r in batch])})
     assert {r['data']['key']:{k:v for k,v in r['data'].items() if not k.startswith('_')} for r in found['result']}=={r['key']:r for r in batch},(entity,offset)
    report[entity]=len(rows)
    print(entity,len(rows),'persisted payloads verified',flush=True)
   cards=[json.loads(s) for s in (P/'engine-CatalogCard.jsonl').open()]
   filters={'must':[field('craft_preferred',True),field('has_owned_copy',False),op('rarity','in',['common','uncommon']),op('printed_mana_value','lte',2),array('color_identity','has_none',['Blue','Red']),field('is_land',False)]}
   found=await call('select_catalog',{'filter':filters})
   expected={r['key'] for r in cards if r['craft_preferred'] and not r['has_owned_copy'] and r['rarity'] in ['common','uncommon'] and r['printed_mana_value']<=2 and not set(r['color_identity'])&{'Blue','Red'} and not r['is_land']}
   assert {r['data']['key'] for r in found['result']}==expected
   report['combined_filter_matches']=len(expected)
   lands={'must':[field('is_land',True),array('mana_symbols_possible','has_any',['B','G'])]}
   found=await call('select_catalog',{'filter':lands})
   assert {r['data']['key'] for r in found['result']}=={r['key'] for r in cards if r['is_land'] and set(r['mana_symbols_possible'])&{'B','G'}}
   report['land_filter_matches']=len(found['result'])
   plants={'must':[array('card_types','has_any',['Creature']),array('subtypes','has_any',['Plant','Fungus'])]}
   found=await call('select_catalog',{'filter':plants})
   assert {r['data']['key'] for r in found['result']}=={r['key'] for r in cards if 'Creature' in r['card_types'] and set(r['subtypes'])&{'Plant','Fungus'}}
   report['plant_fungus_creatures']=len(found['result'])
   for label,tool,query,opts in [
    ('cheap-graveyard-drain','search_catalog','Whenever creature cards leave your graveyard each opponent loses life',{'filter_condition':filters}),
    ('token-drain','search_catalog','Whenever another creature enters the battlefield under your control each opponent loses life',{'filter_condition':filters}),
    ('graveyard-abilities','search_catalogability','Whenever cards leave your graveyard or are shuffled into your library each opponent loses life',{})]:
    result=await call(tool,{'query':query,'options':{'signals':['vector'],'limit':20,**opts}})
    assert result['result']
    meta=result['metadata']['render.meta'];assert meta['vectorCount']>0 and not meta['partial']
    if tool=='search_catalog':assert all(r['data']['key'] in expected for r in result['result'])
    (out/f'{label}.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n')
    report[label]=len(result['result'])
   related=await call('search_catalog_ability_cards',{'query':'cards leave your graveyard each opponent loses life','options':{'signals':['vector'],'limit':5}})
   assert related['result'] and all(r['entity']=='CatalogCard' for r in related['result'])
   (out/'related-cards.json').write_text(json.dumps(related,ensure_ascii=False,indent=2)+'\n')
   report['related_cards']=len(related['result'])
   closed=await session.call_tool('close_backend',{});assert not closed.isError,closed
 (P/'engine-catalog-validation.json').write_text(json.dumps(report,indent=2)+'\n')
 print(json.dumps(report),flush=True)
asyncio.run(main())
