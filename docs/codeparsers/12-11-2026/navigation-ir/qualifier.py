#!/usr/bin/env python3
"""Résumer des sorties parse_file, sans simuler une matérialisation d'index."""
import argparse
import hashlib
import json
from pathlib import Path

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--fixtures', type=Path, required=True)
p.add_argument('--preuves', type=Path, required=True)
p.add_argument('--complement-rust', type=Path, required=True)
p.add_argument('--sortie', type=Path, required=True)
a = p.parse_args()
result = {'portee': 'parse_file uniquement ; projection runtime relue séparément', 'fichiers': []}
for source in sorted(a.fixtures.iterdir()):
    raw = a.complement_rust if source.suffix == '.rs' else a.preuves / (source.name + '.json')
    data = json.loads(raw.read_text())
    text = source.read_bytes()
    starts = [0] + [i + 1 for i, byte in enumerate(text) if byte == 10]
    scopes = []
    for s in data['scopes']:
        begin, end = s['scope_start_byte'], s['scope_end_byte']
        line, last = s['scope_start_line'], s['scope_end_line']
        assert begin == starts[line - 1], (source.name, s['name'], 'début')
        expected = starts[last] - 1 if last < len(starts) else len(text)
        assert end == expected and 0 <= begin <= end <= len(text), (source.name, s['name'], 'fin')
        text[begin:end].decode('utf-8')
        scopes.append({key: s[key] for key in ('name', 'type', 'parent', 'scope_start_line',
                       'scope_end_line', 'scope_start_byte', 'scope_end_byte', 'members', 'modifiers')})
    equal = []
    for i, s in enumerate(scopes):
        for j, other in enumerate(scopes[:i]):
            if (s['scope_start_byte'], s['scope_end_byte']) == (other['scope_start_byte'], other['scope_end_byte']):
                equal.append({'indices': [j, i], 'noms': [other['name'], s['name']]})
    assert data['octets'] == len(text), source.name
    result['fichiers'].append({'fichier': source.name, 'fixture_sha256': hashlib.sha256(text).hexdigest(),
                              'preuve': str(raw), 'preuve_sha256': hashlib.sha256(raw.read_bytes()).hexdigest(),
                              'ast_valid': data['ast_valid'], 'ast_issues': data['ast_issues'],
                              'scopes': scopes, 'plages_egales': equal})
with a.sortie.open('x') as out:
    json.dump(result, out, ensure_ascii=False, indent=2)
    out.write('\n')
print(f'{len(result["fichiers"])} fichiers qualifiés ; positions UTF-8/lignes vérifiées, aucune base matérialisée')
