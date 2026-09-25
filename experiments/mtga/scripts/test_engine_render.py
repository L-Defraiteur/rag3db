"""Actual Magic MCP: long rules survive rendering, raw JSON is opt-in."""
import asyncio,json,os
from pathlib import Path
from mcp import ClientSession,StdioServerParameters
from mcp.client.stdio import stdio_client
ROOT=Path(__file__).resolve().parents[3];P=ROOT/'experiments/mtga/data'
async def main():
 rows=[json.loads(s) for s in (P/'engine-CatalogCard.jsonl').open()]
 card=max((r for r in rows if r['is_primary'] and not r['is_token']),key=lambda r:len(r['text_en']))
 args={'filter':{'field':{'key':'arena_id','value':card['arena_id']}}}
 params=StdioServerParameters(command='bash',args=[str(ROOT/'experiments/mtga/scripts/serve_engine_mcp.sh')],env=dict(os.environ))
 async with stdio_client(params) as streams:
  async with ClientSession(*streams) as session:
   await session.initialize()
   full=await session.call_tool('select_catalog',{**args,'response_format':'json'})
   assert not full.isError,full
   assert len(full.structuredContent['result'])==1
   rendered=await session.call_tool('select_catalog',args)
   assert not rendered.isError,rendered
   assert rendered.structuredContent is None
   text='\n'.join(c.text for c in rendered.content)
   assert card['name_en'] in text
   assert card['text_en'].replace('\r','\\r').replace('\n',' ↵ ').replace('\t','\\t') in text
   for ability in card['abilities']:
    assert ability['text_en'].replace('\r','\\r').replace('\n',' ↵ ').replace('\t','\\t') in text
   assert full.structuredContent['result'][0]['uuid'] in text
   assert '_content_hash' not in text
   out=P/'catalog-research';out.mkdir(exist_ok=True)
   (out/'rendered-example.txt').write_text(text+'\n')
   report={'card':card['name_en'],'rules_characters':len(card['text_en']),'text_characters':len(text),'json_characters':len(json.dumps(full.structuredContent,ensure_ascii=False)),'structured_content_in_text_mode':False}
   (out/'render-validation.json').write_text(json.dumps(report,indent=2)+'\n')
   closed=await session.call_tool('close_backend',{});assert not closed.isError,closed
   print(json.dumps(report))
asyncio.run(main())
