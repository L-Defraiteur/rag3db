"""Measure native backend RSS at startup and during representative real MCP queries."""
import asyncio,json,os,time
from pathlib import Path
from mcp import ClientSession,StdioServerParameters
from mcp.client.stdio import stdio_client
ROOT=Path(__file__).resolve().parents[3];P=ROOT/'experiments/mtga/data'
def descendants(pid):
 try: children=Path(f'/proc/{pid}/task/{pid}/children').read_text().split()
 except (FileNotFoundError,PermissionError):return []
 found=[]
 for child in children:found.append(child);found.extend(descendants(child))
 return found
def backend_memory():
 for pid in descendants(os.getpid()):
  try:
   if not Path(f'/proc/{pid}/comm').read_text().startswith('rag3weaver-back'):continue
   status=dict(line.split(':',1) for line in Path(f'/proc/{pid}/status').read_text().splitlines() if ':' in line)
   return {k:int(status[k].split()[0]) for k in ['VmRSS','VmHWM','VmSize']}
  except (FileNotFoundError,PermissionError,KeyError):continue
 return None
async def main():
 phase='startup';running=True;stats={};timings={}
 async def sample():
  while running:
   mem=backend_memory()
   if mem:
    row=stats.setdefault(phase,{'peak_rss_kib':0,'samples':0})
    row['peak_rss_kib']=max(row['peak_rss_kib'],mem['VmRSS']);row['last_rss_kib']=mem['VmRSS'];row['process_high_water_kib']=mem['VmHWM'];row['virtual_kib']=mem['VmSize'];row['samples']+=1
   await asyncio.sleep(.05)
 task=asyncio.create_task(sample());start=time.monotonic()
 try:
  params=StdioServerParameters(command='bash',args=[str(ROOT/'experiments/mtga/scripts/serve_engine_mcp.sh')],env=dict(os.environ))
  async with stdio_client(params) as streams:
   async with ClientSession(*streams) as session:
    await session.initialize();timings['startup']={'seconds':time.monotonic()-start}
    base={'query':'creatures return from the graveyard to the battlefield','options':{'limit':20,'filter_condition':{'field':{'key':'craft_preferred','value':True}}}}
    for label,tool,args in [('first_hybrid','search_catalog',base),('warm_hybrid','search_catalog',base),('dense_abilities','search_catalogability',{'query':'cards leave your graveyard opponent loses life','options':{'signals':['vector'],'limit':20}}),('ability_to_cards','search_catalog_ability_cards',{'query':'cards leave your graveyard opponent loses life','options':{'signals':['vector'],'limit':5}})]:
     phase=label;start=time.monotonic();result=await session.call_tool(tool,{**args,'response_format':'json'})
     assert not result.isError,result
     timings[label]={'seconds':time.monotonic()-start,'results':len(result.structuredContent['result'])}
     await asyncio.sleep(.1)
    phase='shutdown'
    start=time.monotonic()
    closed=await session.call_tool('close_backend',{});assert not closed.isError,closed
    timings['shutdown']={'seconds':time.monotonic()-start}
 finally:
  running=False;await task
 report={'native_backend_only':True,'units':'KiB','sampling_seconds':.05,'configured_pool_gib':15,'phases':{k:{**v,**stats.get(k,{})} for k,v in timings.items()},'notes':['First queries in a freshly opened backend; OS page cache is not flushed.','Embedding service CPU/GPU memory is not included.','These bounded queries do not measure unbounded graph expansion.','Pool budget is not a process-wide memory cap.']}
 (P/'engine-search-memory.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
asyncio.run(main())
