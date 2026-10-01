#!/usr/bin/env python3
"""Real persistence and declarative constraint graphs; no LLM or embedding requests."""
import copy
import json
import os
from pathlib import Path
import subprocess
import tempfile
from contextlib import contextmanager

ROOT = Path(__file__).resolve().parents[3]
CRATE = ROOT / 'extension/rag3weaver'
SAMPLE = CRATE / 'templates/backends/validated-result'
BINARY = CRATE / 'target/debug/rag3weaver-backend'
os.environ['LD_LIBRARY_PATH'] = str(ROOT / 'build/lecteurs-csv/src') + ':' + os.environ.get('LD_LIBRARY_PATH', '')
os.environ['RAG3DB_BUFFER_POOL_SIZE'] = '268435456'
os.environ['RAG3DB_MAX_DB_SIZE'] = '2147483648'

@contextmanager
def host(manifest):
    process = subprocess.Popen([str(BINARY), str(manifest)], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
    def ask(name, arguments):
        process.stdin.write(json.dumps({'op': 'call', 'name': name, 'arguments': arguments}) + '\n')
        process.stdin.flush()
        line = process.stdout.readline()
        assert line, f'backend exited {process.poll()}'
        reply = json.loads(line)
        assert reply['ok'], reply
        return reply['result']
    try:
        yield ask
    finally:
        process.stdin.close()
        try:
            assert process.wait(timeout=30) == 0
        except subprocess.TimeoutExpired:
            process.kill(); process.wait(); raise
        process.stdout.close()

def config_at(folder):
    config = json.loads((SAMPLE / 'backend.json').read_text())
    config['database'] = str(folder / 'submissions.rag3db')
    config['vector_extension'] = str(ROOT / 'extension/vector/build/libvector.rag3db_extension')
    # No signal is indexed in this template. An unreachable embedding URL proves
    # validations and plain record persistence never need an embedding request.
    config['embeddings'] = {'provider': 'compatible', 'address': 'http://127.0.0.1:1/v1', 'model': 'unused', 'dimensions': 8}
    for entity in config['entities'].values():
        entity['schema'] = str(SAMPLE / entity['schema'])
    config['scripts'] = {k: str(SAMPLE / v) for k, v in config['scripts'].items()}
    for tool in config['tools'].values():
        tool['graph'] = str(SAMPLE / tool['graph'])
        harness = tool.get('harness', {})
        if 'input_schema' in harness:
            harness['input_schema'] = str(SAMPLE / harness['input_schema'])
        for stage in ['before', 'after', 'on_accept']:
            for hook in harness.get(stage, []):
                hook['graph'] = str(SAMPLE / hook['graph'])
    return config

def main():
    with tempfile.TemporaryDirectory(prefix='rag3-harness-') as tmp:
        folder = Path(tmp)
        config = config_at(folder)
        # An ordinary tool gets the same reusable hooks, independently of its name.
        config['tools']['store_measurement'] = copy.deepcopy(config['tools']['submit_result'])
        graph = folder / 'throw.mmd'
        graph.write_text('%% tool: fail\n%% description: Deliberately fail a hook.\n%% param: context json! -- Context\n%% result: fail.result\ngraph LR\n    fail["RhaiNode(script_id=broken, value=$context)"]\n')
        script = folder / 'throw.rhai'
        script.write_text('throw "fixture failure";')
        config['scripts']['broken'] = str(script)
        config['tools']['after_failure'] = copy.deepcopy(config['tools']['submit_result'])
        config['tools']['after_failure']['harness']['after'] = [{'graph': str(graph)}]
        config['tools']['delivery_failure'] = copy.deepcopy(config['tools']['submit_result'])
        config['tools']['delivery_failure']['harness']['on_accept'] = [{'graph': str(graph)}]
        manifest = folder / 'backend.json'
        manifest.write_text(json.dumps(config))
        with host(manifest) as ask:
            schema = ask('submit_result', {'result': {'name': 'x', 'value': 'wrong'}})
            assert schema['stage'] == 'input_schema' and not schema['executed'], schema
            rejected = ask('store_measurement', {'result': {'name': ' ', 'value': -3}})
            assert not rejected['validation']['accepted'] and not rejected['executed'], rejected
            issues = rejected['validation']['errors']
            assert {e['node'] for e in issues} == {'positive', 'name'}, rejected
            assert issues[0]['message'] == 'Value must be greater than 0; received -3.', rejected
            assert issues[0]['params'] == {'minimum': 0, 'actual': -3}, rejected
            good = {'result': {'name': 'measurement', 'value': 2}}
            accepted = ask('submit_result', good)
            assert accepted['validation']['accepted'] and accepted['delivery']['ok'], accepted
            record = accepted['result']
            assert json.loads(record['data']['payload_json']) == good['result'], record
            assert record['data']['created_at'] > 0
            replay = ask('submit_result', good)
            assert replay['result']['unchanged'] and replay['result']['uuid'] == record['uuid'], replay
            failed = ask('after_failure', {'result': {'name': 'after', 'value': 1}})
            assert failed['executed'] and failed['stage'] == 'after' and not failed['validation']['accepted'], failed
            assert failed['validation']['errors'][0]['code'] == 'hook_failed', failed
            delivered = ask('delivery_failure', {'result': {'name': 'delivery', 'value': 1}})
            assert delivered['validation']['accepted'] and not delivered['delivery']['ok'], delivered
        with host(manifest) as ask:
            restored = ask('get_result', {'record': {'key': record['data']['key']}})
            assert restored['result'] == {'uuid': record['uuid'], 'data': record['data']}, restored
            # A before rejection never persists. Resolve its deterministic key
            # using the same pure preparation graph, without a DB write.
            inspect = folder / 'inspect.mmd'
            inspect.write_text('%% tool: inspect\n%% description: Prepare identity without persisting.\n%% param: result json! -- Proposal\n%% result: prepare.result\ngraph LR\n    prepare["RhaiNode(script_id=prepare_record, value=$result)"]\n')
        config['tools']['inspect'] = {'graph': str(inspect)}
        manifest.write_text(json.dumps(config))
        with host(manifest) as ask:
            identity = ask('inspect', {'result': {'name': ' ', 'value': -3}})['result']
            absent = ask('get_result', {'record': {'key': identity['key']}})
            assert absent['result']['data'] is None, absent
    print('PASS: independent constraints, all diagnostics, schema, ordinary tools, no write on rejection, replay, restart, after/delivery failures.')

if __name__ == '__main__':
    main()
