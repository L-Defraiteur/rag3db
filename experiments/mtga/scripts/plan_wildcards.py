"""Plan desired totals using catalog and inventory fetched through the actual MCP."""
import asyncio,json,os,sys
from pathlib import Path
from mcp import ClientSession,StdioServerParameters
from mcp.client.stdio import stdio_client
ROOT=Path(__file__).resolve().parents[3]
sys.path.insert(0,str(ROOT/'experiments/mtga'))
from catalog_model import wildcard_plan,normalized_name
async def plan(requests):
 params=StdioServerParameters(command='bash',args=[str(ROOT/'experiments/mtga/scripts/serve_engine_mcp.sh')],env=dict(os.environ))
 async with stdio_client(params) as streams:
  async with ClientSession(*streams) as session:
   await session.initialize()
   async def call(name,args):
    result=await session.call_tool(name,{**args,'response_format':'json'})
    if result.isError:raise RuntimeError(str(result))
    return [r['data'] for r in result.structuredContent['result']]
   rows=await call('select_catalog',{'filter':{'field':{'key':'name_normalized','value':[{'op':'in','value':sorted({normalized_name(r['name']) for r in requests})}]}}})
   inventory=await call('select_wildcardinventory',{'filter':{'must':[]}})
   if len(inventory)!=1:raise RuntimeError('Expected one inventory snapshot')
   if {r['snapshot_id'] for r in rows}-{inventory[0]['snapshot_id']}:raise RuntimeError('Mixed snapshots; resync before planning')
   result=wildcard_plan(requests,rows,inventory[0])
   closed=await session.call_tool('close_backend',{});assert not closed.isError,closed
   return result
if __name__=='__main__':
 requests=json.loads(Path(sys.argv[1]).read_text())
 result=asyncio.run(plan(requests))
 Path(sys.argv[2]).write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n')
 print(json.dumps(result,ensure_ascii=False,indent=2))
