#!/usr/bin/env python3
"""A backend that declares a reranker and an OCR model really gets them.

Until 3 October 2026 the nodes existed (RerankNode, OcrNode) and a backend
never gave them a service. Now `models.rerank` and `models.ocr` declare them,
with the same declaration as every other model. Their only form today is
`local` (our burn engine, in this process); a daemon or a third party is
refused, saying so.

Requires the embedding service of test_backend_persistence.py (bge-m3), and
the local weights of msmarco-minilm and ppocrv6-tiny under ~/.cache/rag3weaver.
The reranker and the OCR model load on this machine's card (90 MB and 6 MB).
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


def manifest_in(tmp, name, models):
    sample = CRATE / 'templates/backends/notebook/backend.json'
    config = json.loads(sample.read_text())
    config['database'] = str(Path(tmp) / f'{name}.rag3db')
    config['vector_extension'] = str(ROOT / 'extension/vector/build/libvector.rag3db_extension')
    for entity in config['entities'].values():
        entity['schema'] = str(sample.parent / entity['schema'])
    for tool in config['tools'].values():
        tool['graph'] = str((sample.parent / tool['graph']).resolve())
    if SERVICE:
        config['embeddings'] = dict(config['embeddings'], address='')
    # The base search graph: it has the rerank stage the notebook's own graph lacks.
    config['tools']['search_notes'] = {'graph': str(CRATE / 'templates/tools/search_base.mmd'), 'bindings': {'target': 'Note'},
                                       'metadata': [{'node': 'render', 'port': 'meta'}]}
    config['models'] = models
    path = Path(tmp) / f'{name}.json'
    path.write_text(json.dumps(config))
    return path


with tempfile.TemporaryDirectory(prefix='rag3-backend-models-') as tmp:
    # 1. Declared local: the backend starts with both, and a search really reranks.
    manifest = manifest_in(tmp, 'declared', {'rerank': {'provider': 'local', 'model': 'msmarco-minilm'},
                                             'ocr': {'provider': 'local', 'model': 'ppocrv6-tiny'}})
    notes = [
        {'key': 'a', 'text': 'A flying creature with ward protects its controller.', 'labels': ['Blue'], 'stage': 'working'},
        {'key': 'b', 'text': 'How to bake sourdough bread at home.', 'labels': ['Food'], 'stage': 'working'},
        {'key': 'c', 'text': 'Creatures with flying can only be blocked by flying or reach.', 'labels': ['Rules'], 'stage': 'working'},
    ]
    with host(manifest) as ask:
        for note in notes:
            created = ask(name='put_note', arguments={'record': note})
            assert created['ok'], created
        plain = ask(name='search_notes', arguments={'query': 'which creatures can block a flying creature?', 'limit': 3})
        assert plain['ok'], plain
        assert plain['result']['metadata']['render.meta'].get('rerankedCount', 0) == 0, plain['result']['metadata']
        reranked = ask(name='search_notes', arguments={'query': 'which creatures can block a flying creature?', 'limit': 3, 'rerank': 3})
        assert reranked['ok'], reranked
        meta = reranked['result']['metadata']['render.meta']
        assert meta.get('rerankedCount', 0) >= 2, meta

    # 2. A daemon that does not exist yet, or a model the local engine does not know: refused at startup, saying why.
    for name, models, expected in [
        ('rerank-service', {'rerank': {'provider': 'service', 'model': 'msmarco-minilm'}}, 'models.rerank'),
        ('ocr-unknown', {'ocr': {'provider': 'local', 'model': 'tesseract'}}, 'ppocrv6-tiny'),
    ]:
        refused = subprocess.run([str(BINARY), str(manifest_in(tmp, name, models))], input='', capture_output=True, text=True, timeout=120)
        out = refused.stderr + refused.stdout
        assert refused.returncode != 0 and expected in out, (name, out[-500:])

print('PASS: a declared reranker reranks, a declared OCR model loads; a form that does not exist is refused, saying why.')
