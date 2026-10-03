#!/usr/bin/env python3
"""Snapshot batches on an entity with a lifecycle: guarded, not refused.

Requires a real local embedding daemon, like test_backend_persistence.py.
A batch goes through the same state machine as the ingestion: births take a
declared state, a changed state follows a declared transition, and one refused
row rejects the whole batch before anything is written (all-or-nothing, like
the schema check).
"""
import json
import os
from pathlib import Path
import subprocess
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

def states(ask):
    reply=ask(name='list_tasks',arguments={'limit':0})
    assert reply['ok'],reply
    return {r['data']['key']:r['data']['status'] for r in reply['result']['result']}

with tempfile.TemporaryDirectory(prefix='rag3-lifecycle-batch-') as tmp:
    tmp=Path(tmp)
    sample=CRATE/'templates/backends/notebook/backend.json'
    base=json.loads(sample.read_text())
    # Un service d'embarquement déjà en place (RAG3WEAVER_EMBED_SERVICE) l'emporte
    # sur l'adresse du gabarit : `address` vide, et le backend choisit par le modèle.
    if os.environ.get('RAG3WEAVER_SERVICE_EMBED') or os.environ.get('RAG3WEAVER_EMBED_SERVICE'):base['embeddings']=dict(base['embeddings'],address='')
    (tmp/'task.json').write_text(json.dumps({
        'type':'object','additionalProperties':False,'required':['key','title'],
        'properties':{'key':{'type':'string','minLength':1},'title':{'type':'string'},
                      # No enum here: the state machine is the guard under test.
                      'status':{'type':'string'}}}))
    manifest={
        'version':1,'name':'lifecycle-batch','database':str(tmp/'tasks.rag3db'),
        'embeddings':base['embeddings'],
        'vector_extension':str(ROOT/'extension/vector/build/libvector.rag3db_extension'),
        'entities':{'Task':{'schema':str(tmp/'task.json'),'config':{
            'fields':{'key':{'type':'string','isTitle':True},'title':{'type':'string','isContent':True},
                      'status':{'type':'string'}},
            'hashsafe':['key'],'signals':['bm25'],
            'lifecycle':{'field':'status','initial':'open','transitions':[
                {'name':'start','from':'open','to':'in_progress'},
                {'name':'finish','from':'in_progress','to':'closed'}]}}}},
        'tools':{
            'ingest_tasks':{'graph':str(CRATE/'templates/tools/ingest_snapshot.mmd'),'bindings':{'entity':'Task'}},
            'list_tasks':{'graph':str(CRATE/'templates/tools/select_structured.mmd'),'bindings':{'entity':'Task'}}}}
    (tmp/'backend.json').write_text(json.dumps(manifest))
    with host(tmp/'backend.json') as ask:
        # Births: a declared state, or none (the initial state applies).
        born=ask(name='ingest_tasks',arguments={'records':[
            {'key':'a','title':'Alpha'},{'key':'b','title':'Beta','status':'open'}]})
        assert born['ok'],born
        assert states(ask)=={'a':'open','b':'open'},states(ask)
        # An undeclared birth state rejects the batch.
        bad=ask(name='ingest_tasks',arguments={'records':[{'key':'c','title':'Gamma','status':'done'}]})
        assert not bad['ok'] and "non déclaré" in bad['error'],bad
        # One declared transition and one undeclared (open → closed): nothing
        # is written, and the refusal names what would have been allowed.
        mixed=ask(name='ingest_tasks',arguments={'records':[
            {'key':'a','title':'Alpha','status':'in_progress'},{'key':'b','title':'Beta','status':'closed'}]})
        assert not mixed['ok'],mixed
        assert "'open' → 'closed'" in mixed['error'] and 'start → in_progress' in mixed['error'],mixed
        assert 'nothing written' in mixed['error'],mixed
        assert states(ask)=={'a':'open','b':'open'},states(ask)
        # Declared transitions pass, a repeated state is no transition.
        ok=ask(name='ingest_tasks',arguments={'records':[
            {'key':'a','title':'Alpha','status':'in_progress'},{'key':'b','title':'Beta','status':'open'}]})
        assert ok['ok'],ok
        assert states(ask)=={'a':'in_progress','b':'open'},states(ask)
print('PASS: lifecycle snapshot batches — births, declared transitions, all-or-nothing refusal naming the allowed ones.')
