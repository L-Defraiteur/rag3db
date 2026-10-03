#!/usr/bin/env python3
"""A backend that declares a sparse signal really gets a sparse embedder.

Until 3 October 2026 the backend counted the `sparse` signal to require an
embedding service, then never gave the catalog a sparse embedder: the signal
stayed silent. Now `models.sparse` declares it (same declaration as every
other model), and a sparse signal without it is refused at startup.

Requires an embedding service that serves bge-m3 dense + sparse:
RAG3WEAVER_SERVICE_EMBED (or the older RAG3WEAVER_EMBED_SERVICE), or a local
daemon on 127.0.0.1:7878 as in test_backend_persistence.py.
"""
import json
import os
from pathlib import Path
import subprocess
import tempfile
from contextlib import contextmanager

ROOT = Path(__file__).resolve().parents[3]
CRATE = ROOT / 'extension/rag3weaver'
BINARY = CRATE / 'target/debug/rag3weaver-backend'
os.environ['LD_LIBRARY_PATH'] = str(ROOT / 'build/lecteurs-csv/src') + ':' + os.environ.get('LD_LIBRARY_PATH', '')
os.environ['RAG3DB_BUFFER_POOL_SIZE'] = '268435456'
os.environ['RAG3DB_MAX_DB_SIZE'] = '2147483648'
SERVICE = os.environ.get('RAG3WEAVER_SERVICE_EMBED') or os.environ.get('RAG3WEAVER_EMBED_SERVICE')


@contextmanager
def host(manifest):
    p = subprocess.Popen([str(BINARY), str(manifest)], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)

    def ask(op='call', **kw):
        p.stdin.write(json.dumps({'op': op, **kw}) + '\n')
        p.stdin.flush()
        line = p.stdout.readline()
        if not line:
            raise AssertionError(f'backend exited: {p.poll()}')
        return json.loads(line)

    try:
        yield ask
    finally:
        p.stdin.close()
        try:
            assert p.wait(timeout=30) == 0
        except subprocess.TimeoutExpired:
            p.kill()
            p.wait()
            raise
        p.stdout.close()


def manifest_in(tmp, name, sparse_model):
    sample = CRATE / 'templates/backends/notebook/backend.json'
    config = json.loads(sample.read_text())
    config['database'] = str(Path(tmp) / f'{name}.rag3db')
    config['vector_extension'] = str(ROOT / 'extension/vector/build/libvector.rag3db_extension')
    for entity in config['entities'].values():
        entity['schema'] = str(sample.parent / entity['schema'])
    for tool in config['tools'].values():
        tool['graph'] = str((sample.parent / tool['graph']).resolve())
    # The notebook's search graph has no sparse branch: the same graph, with one.
    graph = (CRATE / 'templates/tools/search_structured.mmd').read_text()
    graph = graph.replace('    vector["VectorSearchNode"]\n', '    vector["VectorSearchNode"]\n    sparse["SparseSearchNode"]\n')
    graph = graph.replace("weights='bm25:0.5,vector:0.5'", "weights='bm25:0.4,vector:0.3,sparse:0.3'")
    graph = graph.replace('    source -->|query| vector\n', '    source -->|query| vector\n    source -->|query| sparse\n')
    graph = graph.replace('    vector -->|meta| render\n', '    vector -->|meta| render\n    sparse -->|meta| render\n')
    graph = graph.replace('    vector -->|results:vector| fuse\n', '    vector -->|results:vector| fuse\n    sparse -->|results:sparse| fuse\n')
    assert 'results:sparse' in graph and 'source -->|query| sparse' in graph, graph
    (Path(tmp) / 'search_with_sparse.mmd').write_text(graph)
    config['tools']['search_notes']['graph'] = str(Path(tmp) / 'search_with_sparse.mmd')
    # With a service variable set, the addresses come from it (chosen by model).
    address = '' if SERVICE else config['embeddings']['address']
    config['embeddings'] = dict(config['embeddings'], address=address)
    config['entities']['Note']['config']['signals'] = ['bm25', 'vector', 'sparse']
    if sparse_model:
        declared = {'provider': 'service', 'model': sparse_model}
        if not SERVICE:
            declared['address'] = address
        config['models'] = {'sparse': declared}
    path = Path(tmp) / f'{name}.json'
    path.write_text(json.dumps(config))
    return path


with tempfile.TemporaryDirectory(prefix='rag3-backend-sparse-') as tmp:
    # 1. A sparse signal without `models.sparse`: refused at startup, saying what to write.
    refused = subprocess.run([str(BINARY), str(manifest_in(tmp, 'undeclared', None))], input='', capture_output=True, text=True, timeout=120)
    assert refused.returncode != 0, refused
    assert 'models.sparse' in (refused.stderr + refused.stdout), refused.stderr[-600:]

    # 2. Declared: the sparse signal answers, alone and fused.
    manifest = manifest_in(tmp, 'declared', 'bge-m3')
    payload = {'key': 'alpha', 'text': 'Flying creature with ward. Créature volante protégée.', 'labels': ['Blue', 'flying'], 'stage': 'working'}
    with host(manifest) as ask:
        created = ask(name='put_note', arguments={'record': payload})
        assert created['ok'], created
        uuid = created['result']['result']['uuid']
        for signals in [['sparse'], ['vector', 'sparse'], ['bm25', 'vector', 'sparse']]:
            reply = ask(name='search_notes', arguments={'query': 'flying creature', 'options': {'signals': signals, 'limit': 1}})
            assert reply['ok'], reply
            hits = reply['result']['result']
            assert len(hits) == 1 and hits[0]['uuid'] == uuid, (signals, reply)
            meta = reply['result']['metadata']['render.meta']
            assert not meta.get('partial', False), (signals, meta)
            if signals == ['sparse']:
                assert meta.get('sparseCount', 0) >= 1 and meta.get('vectorCount', 0) == 0, meta

    # 3. Declared with a model nobody serves: refused, saying what each address serves.
    wrong = subprocess.run([str(BINARY), str(manifest_in(tmp, 'wrong', 'granite-278m'))], input='', capture_output=True, text=True, timeout=120)
    assert wrong.returncode != 0, wrong
    out = wrong.stderr + wrong.stdout
    assert 'models.sparse (granite-278m)' in out, out[-600:]

print('PASS: a declared sparse signal answers; undeclared or unserved, the backend refuses and says why.')
