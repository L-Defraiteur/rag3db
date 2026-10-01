#!/usr/bin/env python3
"""Real restart + generated MCP test. Requires a real local embedding daemon."""
import asyncio
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
from contextlib import contextmanager
ROOT = Path(__file__).resolve().parents[3]
CRATE = ROOT/'extension/rag3weaver'
BINARY = CRATE/'target/debug/rag3weaver-backend'
os.environ['LD_LIBRARY_PATH']=str(ROOT/'build/lecteurs-csv/src')+':'+os.environ.get('LD_LIBRARY_PATH','')
os.environ['RAG3DB_BUFFER_POOL_SIZE']='268435456'
os.environ['RAG3DB_MAX_DB_SIZE']='2147483648'

@contextmanager
def host(manifest):
    p=subprocess.Popen([str(BINARY),str(manifest)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,text=True)
    def ask(op='call',**kw):
        p.stdin.write(json.dumps({'op':op,**kw})+'\n');p.stdin.flush()
        line=p.stdout.readline()
        if not line: raise AssertionError(f'backend exited: {p.poll()}')
        return json.loads(line)
    try: yield ask
    finally:
        p.stdin.close()
        try: assert p.wait(timeout=30)==0
        except subprocess.TimeoutExpired:
            p.kill();p.wait();raise
        p.stdout.close()

def invoke(ask,name,arguments):
    reply=ask(name=name,arguments=arguments)
    assert reply['ok'],reply
    return reply['result']['result']

async def mcp_roundtrip(manifest):
    from mcp import ClientSession, StdioServerParameters
    from mcp.client.stdio import stdio_client
    parameters=StdioServerParameters(command=sys.executable,args=[str(CRATE/'scripts/serve_backend_mcp.py'),str(manifest)],env=dict(os.environ))
    async with stdio_client(parameters) as streams:
        async with ClientSession(*streams) as session:
            await session.initialize()
            tools=await session.list_tools()
            names={t.name for t in tools.tools}
            assert names=={'put_note','get_note','save_snapshot','get_snapshot','search_notes'},names
            assert all('entity' not in t.inputSchema.get('properties',{}) for t in tools.tools)
            put=next(t for t in tools.tools if t.name=='put_note')
            properties=put.inputSchema['properties']['record']['properties']
            assert properties['labels']['type']=='array' and properties['stage']['enum']==['working','reviewed']
            assert 'created_at' not in properties and 'revision' not in properties
            bad=await session.call_tool('put_note',{'record':{'key':'bad','text':'oops','labels':'Blue','stage':'working'}})
            assert bad.isError
            result=await session.call_tool('get_note',{'record':{'key':'alpha'}})
            assert not result.isError,result
            assert result.structuredContent['result']['data']['revision']==2,result
            hidden=await session.call_tool('not_exposed',{})
            assert hidden.isError

with tempfile.TemporaryDirectory(prefix='rag3-backend-persistence-') as tmp:
    sample=CRATE/'templates/backends/notebook/backend.json'
    config=json.loads(sample.read_text())
    config['database']=str(Path(tmp)/'persistent.rag3db')
    config['vector_extension']=str(ROOT/'extension/vector/build/libvector.rag3db_extension')
    for entity in config['entities'].values():entity['schema']=str(sample.parent/entity['schema'])
    for tool in config['tools'].values():tool['graph']=str((sample.parent/tool['graph']).resolve())
    manifest=Path(tmp)/'backend.json';manifest.write_text(json.dumps(config))
    payload={'key':'alpha','text':'Flying creature with ward. Créature volante protégée.','labels':['Blue','flying'],'stage':'working'}
    with host(manifest) as ask:
        created=invoke(ask,'put_note',{'record':payload})
        before=created['data'];assert before['revision']==1 and before['reviewed_at'] is None,before
        assert before['created_at']>0 and before['updated_at']>=before['created_at']
        snapshot=invoke(ask,'save_snapshot',{'record':{'key':'alpha-v1','text':payload['text'],'labels':payload['labels']}})
        assert snapshot['data']['created_at']>0
    # New process, same disk base. No ingestion before these reads/searches.
    with host(manifest) as ask:
        restored=invoke(ask,'get_note',{'record':{'key':'alpha'}})
        assert restored['data']==before,(restored,before)
        for signals in [['bm25'],['vector'],['bm25','vector']]:
            reply=ask(name='search_notes',arguments={'query':'Flying','options':{'signals':signals,'filter_condition':{'path':{'path':['labels'],'value':[{'op':'has_any','value':['Blue']}]}},'limit':1}})
            assert reply['ok'],reply
            hits=reply['result']['result'];assert len(hits)==1 and hits[0]['uuid']==restored['uuid'],reply
            meta=reply['result']['metadata']['render.meta'];assert not meta.get('partial',False),meta
        updated=invoke(ask,'put_note',{'record':{**payload,'stage':'reviewed'},'expected_revision':1})
        assert updated['data']['revision']==2 and updated['data']['created_at']==before['created_at']
        assert updated['data']['reviewed_at']>=before['created_at']
        assert not ask(name='put_note',arguments={'record':payload,'expected_revision':1})['ok']
        assert not ask(name='put_note',arguments={'record':{**payload,'stage':'invalid'},'expected_revision':2})['ok']
        assert not ask(name='put_note',arguments={'record':{**payload,'created_at':0},'expected_revision':2})['ok']
        assert invoke(ask,'get_note',{'record':{'key':'alpha'}})['data']==updated['data']
        assert not ask(name='get_note',arguments={'entity':'Snapshot','record':{'key':'alpha'}})['ok']
        assert not ask(name='save_snapshot',arguments={'record':{'key':'alpha-v1','text':'different','labels':[]}})['ok']
        unchanged=invoke(ask,'save_snapshot',{'record':{'key':'alpha-v1','text':payload['text'],'labels':payload['labels']}})
        assert unchanged['unchanged']
    # Third process, this time through the actual SDK MCP transport.
    asyncio.run(mcp_roundtrip(manifest))
print('PASS: persistent data + lucivy + local dense after process restart; revisions, dates, immutable records; generated MCP list/call.')
