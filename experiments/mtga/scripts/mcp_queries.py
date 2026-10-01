"""Run a batch of requests through the actual generated MCP; save structured evidence."""
import asyncio,json,os,sys
from pathlib import Path
from mcp import ClientSession,StdioServerParameters
from mcp.client.stdio import stdio_client
ROOT=Path(__file__).resolve().parents[3]
async def main():
 requests=json.loads(Path(sys.argv[1]).read_text());output=Path(sys.argv[2]);output.mkdir(parents=True,exist_ok=True)
 parameters=StdioServerParameters(command='bash',args=[str(ROOT/'experiments/mtga/scripts/serve_engine_mcp.sh')],env=dict(os.environ))
 async with stdio_client(parameters) as streams:
  async with ClientSession(*streams) as session:
   await session.initialize()
   for r in requests:
    result=await session.call_tool(r['name'],{**r['arguments'],'response_format':'json'})
    if result.isError:raise RuntimeError(str(result))
    payload=result.structuredContent
    (output/(r['label']+'.json')).write_text(json.dumps(payload,ensure_ascii=False,indent=2)+'\n')
    print(r['label'],len(payload.get('result',[])),flush=True)
   await session.call_tool('close_backend',{})
asyncio.run(main())
