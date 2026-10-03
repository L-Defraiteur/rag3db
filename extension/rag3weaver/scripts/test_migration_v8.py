#!/usr/bin/env python3
"""A real v7 base, migrated to v8 by the current backend, then read, searched and written.

Usage: test_migration_v8.py <v7 rag3weaver-backend binary>
(built from a commit where SCHEMA_VERSION is "7"). Requires the local
embedding daemon, like test_backend_persistence.py. The v7 base is created
by the v7 binary, copied (reflink when the filesystem allows it), and the
copy is opened by target/debug/rag3weaver-backend (v8): the migration adds
`_snapshot` to every entity table without touching the rows, then the
reads, searches, writes and a synchronisation session run on it.
"""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
from contextlib import contextmanager
ROOT = Path(__file__).resolve().parents[3]
CRATE = ROOT/'extension/rag3weaver'
V8 = CRATE/'target/debug/rag3weaver-backend'
V7 = Path(sys.argv[1])
os.environ['LD_LIBRARY_PATH']=str(ROOT/'build/lecteurs-csv/src')+':'+os.environ.get('LD_LIBRARY_PATH','')
os.environ['RAG3DB_BUFFER_POOL_SIZE']='268435456'
os.environ['RAG3DB_MAX_DB_SIZE']='2147483648'

@contextmanager
def host(binary, manifest):
    p=subprocess.Popen([str(binary),str(manifest)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,text=True)
    def ask(op='call',**kw):
        p.stdin.write(json.dumps({'op':op,**kw})+'\n');p.stdin.flush()
        line=p.stdout.readline()
        if not line: raise AssertionError(f'backend exited: {p.poll()}')
        return json.loads(line)
    try: yield ask
    finally:
        p.stdin.close()
        assert p.wait(timeout=60)==0
        p.stdout.close()

def invoke(ask,name,arguments):
    reply=ask(name=name,arguments=arguments); assert reply['ok'],reply
    return reply['result']['result']

def manifest_for(tmp, database, with_sync):
    sample=CRATE/'templates/backends/notebook/backend.json'
    config=json.loads(sample.read_text())
    # Un service d'embarquement déjà en place (RAG3WEAVER_EMBED_SERVICE) l'emporte
    # sur l'adresse du gabarit : `address` vide, et le backend choisit par le modèle.
    if os.environ.get('RAG3WEAVER_EMBED_SERVICE'):config['embeddings']=dict(config['embeddings'],address='')
    config['database']=str(database)
    config['vector_extension']=str(ROOT/'extension/vector/build/libvector.rag3db_extension')
    for entity in config['entities'].values(): entity['schema']=str(sample.parent/entity['schema'])
    for tool in config['tools'].values(): tool['graph']=str((sample.parent/tool['graph']).resolve())
    # A plain entity for the synchronisation part, declared in both versions
    # (v7 ignores nothing: `snapshot` is only added on the v8 side).
    (tmp/'card.json').write_text(json.dumps({'type':'object','additionalProperties':False,'required':['key','binder','text'],
        'properties':{'key':{'type':'string'},'binder':{'type':'string'},'text':{'type':'string'}}}))
    card={'fields':{'key':{'type':'string','isTitle':True},'binder':{'type':'string'},'text':{'type':'string','isContent':True}},
          'hashsafe':['key'],'signals':['bm25']}
    if with_sync: card['snapshot']={'scope':['binder']}
    config['entities']['Card']={'schema':str(tmp/'card.json'),'config':card}
    config['tools']['ingest_cards']={'graph':str(CRATE/'templates/tools/ingest_snapshot.mmd'),'bindings':{'entity':'Card'}}
    if with_sync:
        config['tools']['finish_cards']={'graph':str(CRATE/'templates/tools/finish_snapshot.mmd'),'bindings':{'entity':'Card'}}
        config['tools']['begin_cards']={'graph':str(CRATE/'templates/tools/begin_snapshot.mmd'),'bindings':{'entity':'Card'}}
    path=tmp/('backend-v8.json' if with_sync else 'backend-v7.json'); path.write_text(json.dumps(config))
    return path

with tempfile.TemporaryDirectory(prefix='rag3-migration-v8-', dir='/var/tmp') as tmp:
    tmp=Path(tmp)
    v7db=tmp/'v7.rag3db'
    payload={'key':'alpha','text':'Flying creature with ward.','labels':['Blue','flying'],'stage':'working'}
    with host(V7, manifest_for(tmp, v7db, with_sync=False)) as ask:
        note=invoke(ask,'put_note',{'record':payload})['data']
        invoke(ask,'ingest_cards',{'records':[{'key':k,'binder':'A','text':f'Card {k}.'} for k in ['a1','a2','a3']]})
    # The copy is what v8 opens; the v7 base stays as it was.
    v8db=tmp/'v8.rag3db'
    subprocess.run(['cp','--reflink=auto',str(v7db),str(v8db)],check=True)
    for side in ['.wal','.shadow']:
        if Path(str(v7db)+side).exists(): shutil.copy(str(v7db)+side, str(v8db)+side)
    with host(V8, manifest_for(tmp, v8db, with_sync=True)) as ask:
        # Rows written by v7 read back unchanged.
        got=invoke(ask,'get_note',{'record':{'key':'alpha'}})['data']
        # Unchanged, plus the new column, empty: no session has carried it.
        snap,absent=got.pop('_snapshot',None),got.pop('_absent_since','MANQUANTE')
        assert snap=='' and absent is None and got==note,(snap,absent,got,note)
        # Search still answers on the migrated indexes.
        reply=ask(name='search_notes',arguments={'query':'Flying','options':{'signals':['bm25'],'limit':1}})
        assert reply['ok'] and len(reply['result']['result'])==1,reply
        # Writes work, and a synchronisation runs on rows that predate the column.
        invoke(ask,'put_note',{'record':{**payload,'stage':'reviewed'},'expected_revision':1})
        s1=invoke(ask,'begin_cards',{'scope':{'binder':'A'}})['session']
        invoke(ask,'ingest_cards',{'records':[{'key':k,'binder':'A','text':f'Card {k}.'} for k in ['a1','a2']],'snapshot':s1})
        done=invoke(ask,'finish_cards',{'scope':{'binder':'A'},'snapshot':s1})
        assert done['inScope']==3 and done['seen']==2 and len(done['removed'])==1,done
    # The version and the column, read on the closed copy.
    query=CRATE/'target/debug/examples/db_query'
    out=subprocess.run([str(query),str(v8db),"MATCH (m:_catalog_meta {_key: 'schema_version'}) RETURN m._value",
                        "MATCH (n:Note) RETURN n._snapshot, n._absent_since"],capture_output=True,text=True,check=True).stdout
    assert '"8"' in out.split('## ')[1],out
    assert 'ERREUR' not in out,out
print('PASS: v7 base migrated to v8 — reads, search, writes and a synchronisation session on the copy.')
