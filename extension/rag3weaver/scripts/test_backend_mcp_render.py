"""Transport test: rendered text must not secretly duplicate the JSON payload."""
import json,os,sys,tempfile,unittest,time
from pathlib import Path
from mcp import ClientSession,StdioServerParameters
from mcp.client.stdio import stdio_client
SCRIPT=Path(__file__).with_name('serve_backend_mcp.py')
class RenderTransport(unittest.IsolatedAsyncioTestCase):
 async def test_text_and_json_are_distinct_and_errors_recover(self):
  with tempfile.TemporaryDirectory() as tmp:
   fake=Path(tmp)/'backend'
   fake.write_text('#!'+sys.executable+'\n'+'''import sys,json,time
for line in sys.stdin:
 c=json.loads(line)
 if c['op']=='shutdown':
  time.sleep(2.2)
  print(json.dumps({'ok':True,'result':{'closed':True}}),flush=True)
  break
 if c['op']=='describe':r={'name':'test','tools':[{'name':'search','description':'test','inputSchema':{'type':'object','properties':{},'additionalProperties':False}}]}
 else:
  assert not c['arguments']
  r={'result':[{'data':{'text':'FULL_DATA_ONLY'}}],'presentation':'Card\\n`-- effect: complete','metadata':{'partial':False}}
 print(json.dumps({'ok':True,'result':r}),flush=True)
''')
   fake.chmod(0o700)
   params=StdioServerParameters(command=sys.executable,args=[str(SCRIPT),str(Path(tmp)/'manifest'),'--binary',str(fake),'--response-format','text'],env=dict(os.environ))
   async with stdio_client(params) as streams:
    async with ClientSession(*streams) as session:
     await session.initialize()
     schema=(await session.list_tools()).tools[0].inputSchema
     self.assertEqual(schema['properties']['response_format']['default'],'text')
     result=await session.call_tool('search',{})
     self.assertFalse(result.isError)
     self.assertIsNone(result.structuredContent)
     text='\n'.join(c.text for c in result.content)
     self.assertIn('effect: complete',text)
     self.assertIn('partial',text)
     self.assertNotIn('FULL_DATA_ONLY',text)
     result=await session.call_tool('search',{'response_format':'json'})
     self.assertEqual(result.structuredContent['result'][0]['data']['text'],'FULL_DATA_ONLY')
     self.assertNotIn('presentation',result.structuredContent)
     result=await session.call_tool('search',{'response_format':'invalid'})
     self.assertTrue(result.isError)
     self.assertFalse((await session.call_tool('search',{})).isError)
     start=time.monotonic()
     closed=await session.call_tool('close_backend',{})
     self.assertFalse(closed.isError)
     self.assertTrue(closed.structuredContent['closed'])
     self.assertGreater(time.monotonic()-start,2)
if __name__=='__main__':unittest.main()
