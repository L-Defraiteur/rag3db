"""Read-only location of all eleven declared derived-in-RAM panel queries.

This is metadata about pinned public sources, never a Lucivy test or oracle.
"""
import hashlib
import json
import os
from pathlib import Path
import re
import time

START = time.monotonic()
END = START + 120
PACKAGE = Path.home() / '.cache/rag3weaver-build/cx-lucivy-indexed-warning-core-16nov-v1'
CASE = json.loads((PACKAGE / 'native-case.json').read_text())
MANIFEST_PATH = PACKAGE / 'corpora/linux-manifest.json'
SELECTION_PATH = PACKAGE / 'corpora/linux-selections.json'
SOURCE = Path.home() / 'git_workspaces/cx-lucivy-indexed-false-warning-16nov/lucivy_core/tests/test_derived_in_ram.rs'
assert hashlib.sha256(SOURCE.read_bytes()).hexdigest() == CASE['test_source_sha256']
assert hashlib.sha256(MANIFEST_PATH.read_bytes()).hexdigest() == 'd4faf3a7988f42db9d0801ee5bd6630d01f97f7be58ce9838fb6cbe189d4f204'
manifest = json.loads(MANIFEST_PATH.read_text())
selection = json.loads(SELECTION_PATH.read_text())
root = Path(manifest['root'])
assert root.stat().st_uid == os.getuid() and not root.is_symlink()
first300 = set(selection['first300'])
assert len(first300) == 300
pattern = r'Q \{ text: ("(?:[^"\\]|\\.)*"), strict: (true|false), distance: (RX|\d+), anchor: (true|false), exact: (true|false), label: ("(?:[^"\\]|\\.)*") \}'
panel = []
for text, strict, distance, anchor, exact, label in re.findall(pattern, SOURCE.read_text()):
    panel.append(dict(text=json.loads(text), strict=strict == 'true', distance=distance,
                      anchor=anchor == 'true', exact=exact == 'true', label=json.loads(label)))
assert len(panel) == 11 and panel[0]['text'] == 'mutex_lock'


def edit_distance(left, right):
    prev = list(range(len(right) + 1))
    for i, char in enumerate(left, 1):
        row = [i]
        for j, other in enumerate(right, 1):
            row.append(min(row[-1] + 1, prev[j] + 1, prev[j-1] + (char != other)))
        prev = row
    return prev[-1]


def locator(query):
    if query['distance'] == 'RX':
        return 'ASCII regex location only', re.compile(query['text'], re.ASCII | re.IGNORECASE), None
    if query['distance'] in ['1', '2']:
        # Positive examples of close substrings only. Zero examples cannot prove
        # fuzzy absence; the whole Lucivy matching/offset semantics is not run.
        base = {'schdule': 'schedule', 'regsiter': 'register'}[query['text']]
        assert edit_distance(query['text'], base) <= int(query['distance'])
        return 'literal witness within edit distance; incomplete locator, not a fuzzy oracle', None, base
    if query['anchor']:
        suffix = r'(?![a-z0-9])' if query['exact'] else ''
        return 'ASCII alphanumeric boundary location; not Lucivy tokenization proof', re.compile(r'(?<![a-z0-9])' + re.escape(query['text']) + suffix, re.ASCII | re.IGNORECASE), None
    if not query['strict']:
        return 'ASCII alphanumeric separator-stripped location; not a Unicode/engine oracle', None, re.sub('[^a-z0-9]', '', query['text'].lower())
    return 'case-insensitive ASCII literal location', None, query['text'].lower()


scopes = ['first300', 'all_eligible', 'all_regular']
results = []
for query in panel:
    kind, regex, needle = locator(query)
    results.append(dict(query=query, locator_kind=kind,
        scopes={name: dict(documents=0, raw_literal_documents=0, raw_ascii_casefold_documents=0,
                          raw_literal_occurrences=0, raw_ascii_casefold_occurrences=0,
                          locator_evaluated_utf8_documents=0, locator_positive_documents=0,
                          raw_examples=[], locator_examples=[]) for name in scopes}))
locators = [locator(query) for query in panel]
checked = 0
eligible_count = 0
for relative, digest in manifest['files'].items():
    assert time.monotonic() < END, 'read-only inspection deadline exceeded'
    parts = Path(relative).parts
    assert not Path(relative).is_absolute() and '..' not in parts
    assert not any(p.startswith('.env') or '.vault' in p or p == '.git' for p in parts)
    path = root / relative
    assert path.is_file() and not path.is_symlink() and path.stat().st_uid == os.getuid()
    raw = path.read_bytes()
    assert hashlib.sha256(raw).hexdigest() == digest
    folded = raw.lower()
    try:
        text = raw.decode('utf-8')
    except UnicodeError:
        text = None
    eligible = path.suffix in ['.c', '.h', '.rst', '.txt'] and 0 < len(raw) <= 60000 and text is not None
    eligible_count += eligible
    names = ['all_regular'] + (['all_eligible'] if eligible else []) + (['first300'] if relative in first300 else [])
    normalized = re.sub('[^a-z0-9]', '', text.lower()) if text is not None else None
    for result, (kind, regex, needle) in zip(results, locators):
        query = result['query']
        literal = query['text'].encode()
        count = raw.count(literal)
        folded_count = folded.count(literal.lower())
        positive = False
        if text is not None:
            if regex:
                positive = regex.search(text) is not None
            elif query['distance'] in ['1', '2'] or not query['strict']:
                positive = needle in normalized
            else:
                positive = needle.encode() in folded
        for name in names:
            row = result['scopes'][name]
            row['documents'] += 1
            row['raw_literal_documents'] += count > 0
            row['raw_ascii_casefold_documents'] += folded_count > 0
            row['raw_literal_occurrences'] += count
            row['raw_ascii_casefold_occurrences'] += folded_count
            row['locator_evaluated_utf8_documents'] += text is not None
            row['locator_positive_documents'] += positive
            if folded_count and len(row['raw_examples']) < 3:
                row['raw_examples'].append(dict(path=relative, sha256=digest,
                                               first_ascii_casefold_byte_offset=folded.find(literal.lower())))
            if positive and len(row['locator_examples']) < 3:
                row['locator_examples'].append(dict(path=relative, sha256=digest, witness=needle))
    checked += 1
assert checked == 15007 and eligible_count == 12133
report = dict(source=CASE['source'],test_source_sha256=CASE['test_source_sha256'],
    manifest_sha256=hashlib.sha256(MANIFEST_PATH.read_bytes()).hexdigest(),
    selection_manifest_sha256=hashlib.sha256(SELECTION_PATH.read_bytes()).hexdigest(),
    scope='public admitted Linux 2.6.0 regular files only; source/selection unchanged; no Linux 7.2 evidence',
    files_rehashed=checked,eligible_files=eligible_count,subset_documents=300,queries=results,
    duration=time.monotonic()-START,deadline=END,read_only=True,cargo_or_test_invoked=False,
    qualified_as_engine_oracle=False,
    limitations=['Raw literal/ASCII counts are exact for pinned files.',
                 'Other locators provide candidate evidence, never a Lucivy semantic qualification.',
                 'Fuzzy locators are incomplete positive witnesses; zero cannot establish fuzzy absence.',
                 'No reselection, ordering change, index creation, corpus write, or fixture execution.'])
output = PACKAGE / 'derived-panel-corpus-audit.json'
with output.open('x') as file:
    json.dump(report, file, indent=2)
print(json.dumps(dict(files=checked,eligible=eligible_count,duration=report['duration'],output=str(output))))
for result in results:
    print(json.dumps(dict(query=result['query'],scopes={k:dict(raw=v['raw_ascii_casefold_documents'],located=v['locator_positive_documents']) for k,v in result['scopes'].items()})))
