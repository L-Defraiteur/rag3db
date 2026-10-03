#!/usr/bin/env python3
"""Synchronisation by scope through the declarative backend, on a synthetic
entity (index cards in binders — neither code nor cards).

Requires a real local embedding daemon, like test_backend_persistence.py.
Batches carry a session id and one scope; the finish tool removes, within the
scope only, the rows the session did not carry, and refuses an empty session.
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

def keys(ask,binder):
    reply=ask(name='list_cards',arguments={'limit':0})
    assert reply['ok'],reply
    return sorted(r['data']['key'] for r in reply['result']['result'] if r['data']['binder']==binder)

def card(key,binder):
    return {'key':key,'binder':binder,'text':f'Card {key} in binder {binder}.'}

with tempfile.TemporaryDirectory(prefix='rag3-snapshot-') as tmp:
    tmp=Path(tmp)
    base=json.loads((CRATE/'templates/backends/notebook/backend.json').read_text())
    # Un service d'embarquement déjà en place (RAG3WEAVER_EMBED_SERVICE) l'emporte
    # sur l'adresse du gabarit : `address` vide, et le backend choisit par le modèle.
    if os.environ.get('RAG3WEAVER_SERVICE_EMBED') or os.environ.get('RAG3WEAVER_EMBED_SERVICE'):base['embeddings']=dict(base['embeddings'],address='')
    (tmp/'card.json').write_text(json.dumps({
        'type':'object','additionalProperties':False,'required':['key','binder','text'],
        'properties':{'key':{'type':'string','minLength':1},'binder':{'type':'string','minLength':1},'text':{'type':'string'}}}))
    manifest={
        'version':1,'name':'snapshot','database':str(tmp/'cards.rag3db'),
        'embeddings':base['embeddings'],
        'vector_extension':str(ROOT/'extension/vector/build/libvector.rag3db_extension'),
        'entities':{'Card':{'schema':str(tmp/'card.json'),'config':{
            'fields':{'key':{'type':'string','isTitle':True},'binder':{'type':'string'},'text':{'type':'string','isContent':True}},
            'hashsafe':['key'],'signals':['bm25'],
            'snapshot':{'scope':['binder'],'fineScope':['key']}}}},
        'tools':{
            'ingest_cards':{'graph':str(CRATE/'templates/tools/ingest_snapshot.mmd'),'bindings':{'entity':'Card'}},
            'finish_cards':{'graph':str(CRATE/'templates/tools/finish_snapshot.mmd'),'bindings':{'entity':'Card'}},
            'begin_cards':{'graph':str(CRATE/'templates/tools/begin_snapshot.mmd'),'bindings':{'entity':'Card'}},
            'abort_cards':{'graph':str(CRATE/'templates/tools/abort_snapshot.mmd'),'bindings':{'entity':'Card'}},
            'undo_cards':{'graph':str(CRATE/'templates/tools/undo_snapshot.mmd'),'bindings':{'entity':'Card'}},
            'list_cards':{'graph':str(CRATE/'templates/tools/select_structured.mmd'),'bindings':{'entity':'Card'}}}}
    (tmp/'backend.json').write_text(json.dumps(manifest))
    with host(tmp/'backend.json') as ask:
        def ingest(session,cards):
            r=ask(name='ingest_cards',arguments={'records':cards,'snapshot':session}); assert r['ok'],r; return r['result']['result']
        def finish(session,binder,**opts):
            return ask(name='finish_cards',arguments={'scope':{'binder':binder},'snapshot':session,**opts})
        def begin(binder,**opts):
            r=ask(name='begin_cards',arguments={'scope':{'binder':binder},**opts}); return r
        sa=begin('A'); assert sa['ok'],sa; s1=sa['result']['result']['session']
        # One session at a time per scope; another scope is free.
        again=begin('A'); assert not again['ok'] and s1 in again['error'],again
        sb=begin('B'); assert sb['ok'],sb; b1=sb['result']['result']['session']
        first=ingest(s1,[card('a1','A'),card('a2','A'),card('a3','A')])
        assert first['snapshot']==s1 and first['scope']=={'binder':'A'},first
        ingest(b1,[card('b1','B')])
        # A batch carries one scope.
        mixed=ask(name='ingest_cards',arguments={'records':[card('a4','A'),card('b2','B')],'snapshot':s1})
        assert not mixed['ok'] and 'un seul périmètre' in mixed['error'],mixed
        # A session id the engine did not give is refused.
        stale=ask(name='ingest_cards',arguments={'records':[card('a1','A')],'snapshot':'made-up'})
        assert not stale['ok'],stale
        assert finish(s1,'A')['ok'] and finish(b1,'B')['ok']
        # Session s2 of binder A no longer carries a3.
        s2=begin('A')['result']['result']['session']
        ingest(s2,[card('a1','A'),card('a2','A')])
        done=finish(s2,'A'); assert done['ok'],done
        report=done['result']['result']
        assert report['inScope']==3 and report['seen']==2 and len(report['removed'])==1,report
        assert keys(ask,'A')==['a1','a2'] and keys(ask,'B')==['b1'],(keys(ask,'A'),keys(ask,'B'))
        # An empty session is refused, nothing removed; abort closes it.
        s3=begin('A')['result']['result']['session']
        empty=finish(s3,'A'); assert not empty['ok'] and 'instantané vide' in empty['error'],empty
        assert keys(ask,'A')==['a1','a2']
        aborted=ask(name='abort_cards',arguments={'scope':{'binder':'A'},'snapshot':s3}); assert aborted['ok'],aborted
        # A session left open is taken over explicitly.
        s4=begin('A')['result']['result']['session']
        took=begin('A',takeover=True); assert took['ok'] and took['result']['result']['replaced']==s4,took
        # The fine grain: refused while the coarse session holds the binder; open on another binder.
        fine=ask(name='begin_cards',arguments={'scope':{'binder':'A','key':'a1'}})
        assert not fine['ok'] and 'grain' in fine['error'],fine
        sf=ask(name='begin_cards',arguments={'scope':{'binder':'B','key':'b1'}}); assert sf['ok'],sf
        sf=sf['result']['result']['session']
        carried=ingest(sf,[card('b1','B')]); assert carried['scope']=={'binder':'B','key':'b1'},carried
        assert finish(sf,'B',)['ok'] is False  # the fine session is finished on its own scope, not the binder's
        done=ask(name='finish_cards',arguments={'scope':{'binder':'B','key':'b1'},'snapshot':sf}); assert done['ok'],done
        assert done['result']['result']['seen']==1,done
        # Set aside before purge (keepFor, 7 days by default): a finish that removes keeps a copy,
        # and undoes as a block.
        s5=took['result']['result']['session']
        ingest(s5,[card('a1','A')])
        done=finish(s5,'A'); assert done['ok'],done
        removed=done['result']['result']['removed']
        assert len(removed)==1 and done['result']['result']['setAside']==removed,done
        assert keys(ask,'A')==['a1']
        undone=ask(name='undo_cards',arguments={'scope':{'binder':'A'},'snapshot':s5}); assert undone['ok'],undone
        assert undone['result']['result']['restored']==removed,undone
        assert keys(ask,'A')==['a1','a2']
        again=ask(name='undo_cards',arguments={'scope':{'binder':'A'},'snapshot':s5})
        assert not again['ok'] and 'déjà annulée' in again['error'],again
        # A row removed then carried again with the same content comes back from its copy, said by the batch.
        s6=begin('A')['result']['result']['session']; ingest(s6,[card('a1','A')]); assert finish(s6,'A')['ok']
        s7=begin('A')['result']['result']['session']
        back=ingest(s7,[card('a1','A'),card('a2','A')])
        assert back['restored']==removed,back
        assert finish(s7,'A')['ok'] and keys(ask,'A')==['a1','a2']
print('PASS: synchronisation by scope — engine sessions (one per scope, takeover, abort), one scope per batch, removal within the scope, empty session refused; set aside before purge, block undo, a removed row coming back from its copy.')
