"""PURE classification of one preserved canonical run. Never a composite GO.

No subprocess, native library, database or network access. Inventories describe
events observed AFTER the run; they are not an independent ELF --list inventory.
The historical RED controller/scope and its expired deadline remain unchanged.
"""
import argparse
from collections import Counter
import hashlib
import json
import os
from pathlib import Path
import re

LOG_NAMES = ('temoins', 'lib', 'navigateur', 'e2e_code', 'e2e_lectures', 'check_features', 'preuve', 'resume')
MANIFEST_SHA = '2754c91e37815873fbdfd4bbeff2800ca2b3f259f1f24397b2038c88a5a9ddde'
INVENTORY_SHA = '02455373cba66f02d8aba186c5a7de7c7f5ef2eb5bfc8fa985381c41e00cbec3'
SOURCE = 'b472035a4e2944a9d90727817c95290cc0fba1ea'
APP_CRATE = '/home/lucied/git_workspaces/rag3db-cx-ed-vertex-token-union-r1/extension/rag3weaver'
CONTRACT = {
    'temoins': [7, 0, 0, 0, 2677],
    'lib': [2612, 0, 72, 0, 0],
    'navigateur': [45, 0, 0, 0, 2641],
    'e2e_code/filets': [2, 0, 0, 0, 942],
    'e2e_code/suite': [36, 0, 0, 0, 908],
    'e2e_lectures/filets': [2, 0, 0, 0, 942],
    'e2e_lectures/suite': [27, 0, 0, 0, 917],
}
GAMES = ['sans-feature', 'daemon', 'rag3db-native,daemon',
         'rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres',
         'rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres,openai-llm,anthropic-llm',
         'rag3db-native,code,daemon,openai-llm', 'rag3db-native,code', 'rag3db-native']
SUMMARY = re.compile(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out; finished in \d+(?:\.\d+)?s')
EVENT = re.compile(r'test (\S+?)( - should panic)? \.\.\. (ok|ignored(?:, .*)?)')
SLOW = re.compile(r'test (\S+) has been running for over [1-9]\d* seconds')


def need(condition, message):
    if not condition: raise ValueError(message)


def decode(raw):
    def pairs(items):
        value = {}
        for k, v in items:
            need(k not in value, 'duplicate JSON key')
            value[k] = v
        return value
    def constant(value): raise ValueError('nonfinite JSON scalar')
    return json.loads(raw, object_pairs_hook=pairs, parse_constant=constant)


def inventories(value):
    need(isinstance(value, dict) and set(value) == {'kind', 'source', 'phases'}, 'inventory shape')
    need(value['kind'] == 'POST_EXECUTION_RAW_EVENT_INVENTORY_NOT_ELF_LIST' and value['source'] == SOURCE, 'inventory provenance')
    phases = value['phases']
    need(isinstance(phases, dict) and set(phases) == set(CONTRACT), 'inventory phase set')
    for key, rows in phases.items():
        need(isinstance(rows, list) and len(rows) == CONTRACT[key][0]+CONTRACT[key][2], 'inventory cardinality '+key)
        names = []
        for row in rows:
            need(isinstance(row, dict) and set(row) == {'name', 'should_panic', 'status'}, 'inventory row shape')
            need(type(row['name']) is str and all(part.isidentifier() for part in row['name'].split('::')), 'inventory name type')
            need(type(row['should_panic']) is bool and row['status'] in ('ok', 'ignored'), 'inventory status/type')
            names.append(row['name'])
        need(len(names) == len(set(names)), 'duplicate inventory name')
        need(Counter(row['status'] for row in rows) == Counter({'ok': CONTRACT[key][0], 'ignored': CONTRACT[key][2]}), 'inventory status counts')
    filets = {'chaque_suite_est_montee', 'aucune_suite_ne_se_relance_par_son_nom_court'}
    for suite in ['e2e_code', 'e2e_lectures']:
        need({r['name'] for r in phases[suite+'/filets']} == filets, 'filets names')
        need(all(r['name'].startswith(suite+'::') for r in phases[suite+'/suite']), 'suite name scope')
    need(all(r['name'].startswith('openai_llm::vertex_refresh_tests::') for r in phases['temoins']), 'witness scope')
    return phases


def phase(text, key, expected):
    lines = text.splitlines()
    # Consume every running header, including invalid/zero/indented extras.
    headers = [(i, line) for i, line in enumerate(lines) if re.match(r'\s*running(?:\s|$)', line)]
    need(len(headers) == 1, 'one running header '+key)
    header = re.fullmatch(r'running ([1-9]\d*) tests?', headers[0][1])
    need(header is not None, 'malformed/zero running header '+key)
    starts = [(headers[0][0], int(header[1]))]
    ends = [(i, SUMMARY.fullmatch(line)) for i, line in enumerate(lines) if line.startswith('test result:')]
    need(len(starts) == len(ends) == 1 and ends[0][1] is not None, 'one running/result pair '+key)
    first, count = starts[0]
    last, match = ends[0]
    summary = [int(x) for x in match.groups()]
    need(first < last and summary == CONTRACT[key], 'summary contract '+key)
    need(count == summary[0]+summary[2], 'running cardinality '+key)
    events = []
    slow = []
    for i, line in enumerate(lines):
        if line.startswith('test result:'): continue
        if not line.startswith('test '): continue
        event = EVENT.fullmatch(line)
        diagnostic = SLOW.fullmatch(line)
        need(first < i < last and (event is not None or diagnostic is not None), 'unknown/outside test event '+key)
        if diagnostic:
            slow.append(diagnostic[1])
            continue
        events.append(dict(name=event[1], should_panic=bool(event[2]), status='ignored' if event[3].startswith('ignored') else 'ok'))
    names = [row['name'] for row in events]
    need(len(names) == count and len(names) == len(set(names)), 'duplicate/missing test event '+key)
    need(sorted(events, key=lambda x:x['name']) == sorted(expected, key=lambda x:x['name']), 'event inventory mismatch '+key)
    need(set(slow) <= set(names), 'unselected long-running diagnostic '+key)
    return dict(summary=summary, selected=len(names), ignored_names=sorted(r['name'] for r in events if r['status']=='ignored'), total=sum(summary))


def classify(raw, inventory):
    need(isinstance(raw, dict) and set(raw) == set(LOG_NAMES) and all(type(v) is str for v in raw.values()), 'RAW log set/type')
    expected = inventories(inventory)
    result = {}
    for name in ['temoins', 'lib', 'navigateur']:
        need(len(re.findall(r'^     Running unittests src/lib\.rs \([^\n]+\)$', raw[name], re.M)) == 1, 'unit binary header '+name)
        result[name] = phase(raw[name], name, expected[name])
    for name in ['e2e_code', 'e2e_lectures']:
        marker_a = '     Running tests/e2e_filets.rs (binaire e2e unique, ses propres filets)'
        marker_b = f'     Running tests/{name}.rs (binaire e2e unique, filtre {name}::)'
        headers = re.findall(r'^     Running tests/[^\n]+$', raw[name], re.M)
        need(headers == [marker_a, marker_b], 'ordered e2e phase headers '+name)
        a, b = raw[name].index(marker_a), raw[name].index(marker_b)
        need(not any(re.match(r'\s*(?:test|running)(?:\s|$)', line) for line in raw[name][:a].splitlines()), 'event outside e2e phases')
        need(len(re.findall(r'^▸ binaire e2e : /[^\n]+ \(\d+ Mo\)$', raw[name], re.M)) == 1, 'real e2e binary header')
        result[name+'/filets'] = phase(raw[name][a:b], name+'/filets', expected[name+'/filets'])
        result[name+'/suite'] = phase(raw[name][b:], name+'/suite', expected[name+'/suite'])
    feature_lines = raw['check_features'].splitlines()
    need(feature_lines.count('ETAT_SUITE_E2E VERT') == 1, 'feature preflight boundary')
    boundary = feature_lines.index('ETAT_SUITE_E2E VERT')
    need(feature_lines[boundary+1:] == ['VERT  '+game for game in GAMES]+['CHECK_FEATURES VERT 8/8'], 'eight exact feature games')
    need(raw['check_features'].splitlines()[-1] == 'CHECK_FEATURES VERT 8/8', 'features final verdict')
    need(len(re.findall(r'^CHECK_FEATURES ', raw['check_features'], re.M)) == 1, 'features duplicate verdict')
    proof = raw['preuve'].splitlines()
    crate = [(i, line) for i, line in enumerate(proof) if re.fullmatch(r' +(?:Checking|Compiling) rag3weaver v0\.1\.0 \('+re.escape(APP_CRATE)+r'\)', line)]
    finished = [(i, line) for i, line in enumerate(proof) if line.startswith('    Finished')]
    need(len(crate) == len(finished) == 1 and crate[0][0] < finished[0][0], 'proof real crate/path before completion')
    need(re.fullmatch(r'    Finished `dev` profile \[unoptimized \+ debuginfo\] target\(s\) in \d+(?:\.\d+)?s', finished[0][1]) is not None, 'proof expected dev profile')
    resume = raw['resume'].splitlines()
    need(resume[-2:] == ['suites jouées : e2e_lectures e2e_code', 'VERT'], 'canonical final verdict/suites')
    section_contract = [('== témoin', ['temoins']),
                        ('== e2e_lectures', ['e2e_lectures/filets', 'e2e_lectures/suite']),
                        ('== e2e_code', ['e2e_code/filets', 'e2e_code/suite']),
                        ('== check_features', []), ('== lib', ['lib']),
                        ('== témoins navigateur', ['navigateur'])]
    sections = [(i, line) for i, line in enumerate(resume) if line.startswith('== ')]
    need([line for i, line in sections] == [header for header, keys in section_contract], 'canonical phase order')
    need(not any(line.startswith(('test result:', 'CHECK_FEATURES ', 'preuve :')) for line in resume[:sections[0][0]]), 'canonical events before first phase')
    for index, (start, header) in enumerate(sections):
        stop = sections[index+1][0] if index+1 < len(sections) else len(resume)
        body = resume[start+1:stop]
        keys = section_contract[index][1]
        summaries = [line for line in body if line.startswith('test result:')]
        need(len(summaries) == len(keys), 'canonical block summary cardinality '+header)
        for line, key in zip(summaries, keys):
            match = SUMMARY.fullmatch(line)
            need(match is not None and [int(x) for x in match.groups()] == CONTRACT[key], 'canonical block summary '+key)
        markers = [line for line in body if line.startswith(('CHECK_FEATURES ', 'preuve :'))]
        wanted = ['CHECK_FEATURES VERT 8/8', 'preuve : rag3weaver revérifié'] if header == '== check_features' else []
        need(markers == wanted, 'canonical markers in matching feature/proof phase '+header)
    return dict(status='RAW_CANONICAL_CLASSIFIED_ONLY', phases=result,
                historical_controller='RED', historical_scope='ROUGE',
                composite_requalified=False, new_execution=False,
                original_initial=1192303.460980840, original_end=1197633.460980840,
                inventory_kind=inventory['kind'])


def sha(path):
    with path.open('rb') as stream: return hashlib.file_digest(stream, 'sha256').hexdigest()


def archive_read(root):
    need(sha(root/'manifest.json') == MANIFEST_SHA, 'archive manifest pin')
    rows = decode((root/'manifest.json').read_bytes())
    need(isinstance(rows, list) and len(rows) == 54, 'archive count')
    seen = set()
    for row in rows:
        need(isinstance(row, dict) and set(row) == {'path', 'size', 'sha256'}, 'manifest row')
        name = row['path']
        need(type(name) is str and not Path(name).is_absolute() and '..' not in Path(name).parts and name not in seen, 'manifest path/duplicate')
        seen.add(name)
        path = root/name
        need(path.resolve().is_relative_to(root.resolve()) and path.is_file() and not path.is_symlink(), 'archive regular confined file')
        need(type(row['size']) is int and path.stat().st_size == row['size'] and sha(path) == row['sha256'], 'archive size/SHA '+name)
    need(decode((root/'client/official-pass.result.json').read_bytes())['rc'] == 0, 'canonical actual rc')
    need(decode((root/'client/composed-result.json').read_bytes())['status'] == 'RED', 'historical controller preserved')
    need(decode((root/'supervisor/verdict.json').read_bytes())['verdict'] == 'ROUGE', 'historical scope preserved')
    return {name:(root/'client/official'/f'{name}.log').read_text() for name in LOG_NAMES}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--archive', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    inventory = Path(__file__).with_name('observed-inventory.json')
    need(sha(inventory) == INVENTORY_SHA, 'inventory pin')
    value = classify(archive_read(args.archive), decode(inventory.read_bytes()))
    value.update(manifest_sha256=MANIFEST_SHA, inventory_sha256=INVENTORY_SHA, classifier_sha256=sha(Path(__file__)))
    with args.output.open('x') as out:
        json.dump(value, out, indent=2)
        out.flush()
        os.fsync(out.fileno())


if __name__ == '__main__': main()
