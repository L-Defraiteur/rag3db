#!/usr/bin/env python3
"""The backend host after a failed checkpoint (REOPEN_AFTER_FAILED_CHECKPOINT).

Requires a real local embedding daemon, like test_backend_persistence.py.
The failure is injected by the connection's test hook
(RAG3WEAVER_TEST_FAILED_CHECKPOINT_AT_CALL=k: the k-th tool call meets the
engine's refusal); the real engine failure is covered by the C++ tests.
The host answers the call with mustReopen, then exits with 75 (EX_TEMPFAIL)
so its launcher restarts it; the restarted host has what was committed, and
the failed call can be made again.
"""
import json
import os
from pathlib import Path
import subprocess
import tempfile
ROOT = Path(__file__).resolve().parents[3]
CRATE = ROOT/'extension/rag3weaver'
BINARY = CRATE/'target/debug/rag3weaver-backend'
os.environ['LD_LIBRARY_PATH']=str(ROOT/'build/lecteurs-csv/src')+':'+os.environ.get('LD_LIBRARY_PATH','')
os.environ['RAG3DB_BUFFER_POOL_SIZE']='268435456'
os.environ['RAG3DB_MAX_DB_SIZE']='2147483648'
NAME='A checkpoint of this database failed, so it must be closed and reopened'
EXIT_MUST_REOPEN=75

def start(manifest,env=None):
    p=subprocess.Popen([str(BINARY),str(manifest)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,text=True,
                       env={**os.environ,**(env or {})})
    def ask(op='call',**kw):
        p.stdin.write(json.dumps({'op':op,**kw})+'\n');p.stdin.flush()
        line=p.stdout.readline()
        if not line: raise AssertionError(f'backend exited: {p.poll()}')
        return json.loads(line)
    return p,ask

def note(key):
    return {'key':key,'text':f'Note {key}.'}

with tempfile.TemporaryDirectory(prefix='rag3-reopen-') as tmp:
    tmp=Path(tmp)
    base=json.loads((CRATE/'templates/backends/notebook/backend.json').read_text())
    (tmp/'note.json').write_text(json.dumps({
        'type':'object','additionalProperties':False,'required':['key','text'],
        'properties':{'key':{'type':'string','minLength':1},'text':{'type':'string'}}}))
    manifest={
        'version':1,'name':'reopen','database':str(tmp/'notes.rag3db'),
        'embeddings':base['embeddings'],
        'vector_extension':str(ROOT/'extension/vector/build/libvector.rag3db_extension'),
        'entities':{'Note':{'schema':str(tmp/'note.json'),'config':{
            'fields':{'key':{'type':'string','isTitle':True},'text':{'type':'string','isContent':True}},
            'hashsafe':['key'],'signals':['bm25']}}},
        'tools':{
            'ingest_notes':{'graph':str(CRATE/'templates/tools/ingest_snapshot.mmd'),'bindings':{'entity':'Note'}},
            'list_notes':{'graph':str(CRATE/'templates/tools/select_structured.mmd'),'bindings':{'entity':'Note'}},
            'search_notes':{'graph':str(CRATE/'templates/tools/search_structured.mmd'),'bindings':{'target':'Note'}}}}
    (tmp/'backend.json').write_text(json.dumps(manifest))

    def keys(ask):
        r=ask(name='list_notes',arguments={'limit':0}); assert r['ok'],r
        return sorted(x['data']['key'] for x in r['result']['result'])

    # The second tool call meets the engine's refusal.
    p,ask=start(tmp/'backend.json',{'RAG3WEAVER_TEST_FAILED_CHECKPOINT_AT_CALL':'2'})
    first=ask(name='ingest_notes',arguments={'records':[note('n1')]}); assert first['ok'],first
    failed=ask(name='ingest_notes',arguments={'records':[note('n2')]})
    assert not failed['ok'] and failed.get('mustReopen') is True and NAME in failed['error'],failed
    # The host answered, then stops by itself with 75 — no shutdown needed.
    assert p.wait(timeout=60)==EXIT_MUST_REOPEN,p.returncode
    p.stdin.close();p.stdout.close()

    # Restarted, as its launcher would: the committed call is there, the failed one is not, and it can be made again.
    p,ask=start(tmp/'backend.json')
    assert keys(ask)==['n1'],keys(ask)
    # And findable in full text, not only present in the table.
    found=ask(name='search_notes',arguments={'query':'n1','options':{'signals':['bm25'],'limit':5}})
    assert found['ok'] and 'key=n1' in found['result']['presentation'],found
    again=ask(name='ingest_notes',arguments={'records':[note('n2')]}); assert again['ok'],again
    assert keys(ask)==['n1','n2'],keys(ask)
    closed=ask(op='shutdown'); assert closed['ok'] and closed['result']['closed'],closed
    p.stdin.close()
    assert p.wait(timeout=30)==0,p.returncode
    p.stdout.close()
print('PASS: after a failed checkpoint the backend answers mustReopen, exits with 75, and a restarted backend has what was committed.')
