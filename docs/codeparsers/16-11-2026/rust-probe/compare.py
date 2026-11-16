#!/usr/bin/env python3
"""Comparer les occurrences conservées, sans relancer de parseur ni normaliser les identités."""
from collections import Counter
import hashlib
import json
from pathlib import Path

FIELDS = ['caller_path', 'caller_start_line', 'caller_end_line', 'caller_start_byte', 'caller_end_byte',
          'caller_name', 'caller_type', 'caller_key', 'line', 'target_path', 'target_start_line',
          'target_end_line', 'target_start_byte', 'target_end_byte', 'target_name', 'target_type',
          'target_key', 'resolution']
SIX = ['caller_path', 'line', 'caller_name', 'target_name', 'target_path', 'resolution']


def historical_fields(row):
    return tuple(str(row[name]) for name in SIX)


def occurrence(row):
    return tuple(row[name] for name in FIELDS)


def compare_rows(before, after):
    # Le multiensemble préserve plusieurs relations aux mêmes six champs historiques.
    left = Counter(occurrence(row) for row in before)
    right = Counter(occurrence(row) for row in after)
    def records(counter):
        return [{'occurrence': dict(zip(FIELDS, key)), 'count': count}
                for key, count in sorted(counter.items())]
    def native_ids(rows):
        result = {}
        for row in rows:
            result.setdefault(occurrence(row), set()).add((row['caller_uuid'], row['target_uuid']))
        return result
    before_ids, after_ids = native_ids(before), native_ids(after)
    drift = [{'occurrence': dict(zip(FIELDS, key)), 'before_ids': sorted(before_ids[key]),
              'after_ids': sorted(after_ids[key])} for key in sorted(before_ids.keys() & after_ids.keys())
             if before_ids[key] != after_ids[key]]
    return {'before_rows': len(before), 'after_rows': len(after), 'native_identity_changes': drift,
        'unchanged_rows': sum((left & right).values()),
        'added': records(right - left), 'removed': records(left - right)}


def historical_presence(expected, rows):
    exact = Counter(historical_fields(row) for row in rows)
    without_mark = Counter(key[:-1] for key in exact.elements())
    result = []
    for row in expected:
        fields = tuple(str(v) for v in row)
        result.append({'historical_fields': list(fields), 'same_resolution_rows': exact[fields],
            'any_resolution_rows': without_mark[fields[:-1]], 'judgment': 'unjudged'})
    return result


def review_sample(roots, excluded, limit=20):
    ranked = {}
    for label, result in roots.items():
        records = [row for row in result['removed']
            if historical_fields(row['occurrence']) not in excluded]
        def score(row):
            raw = json.dumps(row['occurrence'], sort_keys=True, ensure_ascii=False)
            return hashlib.sha256(('rust-path-probe-20261116\0' + label + '\0' + raw).encode()).hexdigest()
        ranked[label] = sorted(records, key=score)
    selected = []
    for label in ['rw', 'engine']:
        selected.extend({'root': label, **row} for row in ranked[label][:limit // 2])
        ranked[label] = ranked[label][limit // 2:]
    for label in ['rw', 'engine']:
        room = limit - len(selected)
        selected.extend({'root': label, **row} for row in ranked[label][:room])
    return selected


def compare_pair(pair, read_edges, check_outputs):
    root = Path(pair['proofs'])
    for phase in ['baseline', 'candidate']:
        assert json.loads((root / phase / 'classification.json').read_text())['phase_completed'] is True
        check_outputs(pair, phase)
    manifest = json.loads(Path(pair['manifest']).read_text())
    history = json.loads(Path(pair['historical_var_file']).read_text())
    findings = {'corpus_manifest_sha256': pair['corpus_manifest_sha256'], 'sdk': pair['sdk'],
        'sources': {phase: config['source'] for phase, config in pair['phases'].items()},
        'global_pass': False, 'semantic_judgment': 'pending_manual_review', 'roots': {}}
    for n, label in enumerate(['rw', 'engine']):
        before = read_edges(root / 'baseline' / f'edges-{n}.jsonl')
        after = read_edges(root / 'candidate' / f'edges-{n}.jsonl')
        output = compare_rows(before, after)
        if label == 'rw':
            expected = manifest['historical_reference_pairs']
            fields = [[r['caller_path'], r['line_1based'], r['caller_name'], r['target_name'],
                r['target_path'], r['resolution']] for r in expected]
            output['fourteen_sites_eighteen_pairs_baseline'] = historical_presence(fields, before)
            output['fourteen_sites_eighteen_pairs_candidate'] = historical_presence(fields, after)
            qualified = [r['fields'] for r in history['rows'] if r['line_contains_std_env_var']]
            other = [r['fields'] for r in history['rows'] if not r['line_contains_std_env_var']]
            output['historical_107_qualified_baseline'] = historical_presence(qualified, before)
            output['historical_107_qualified_candidate'] = historical_presence(qualified, after)
            output['historical_other_var_candidate'] = historical_presence(other, after)
        findings['roots'][label] = output
    excluded = set()
    for row in manifest['historical_reference_pairs']:
        excluded.add(tuple(str(v) for v in [row['caller_path'], row['line_1based'], row['caller_name'],
            row['target_name'], row['target_path'], row['resolution']]))
    excluded.update(tuple(row['fields']) for row in history['rows'])
    findings['removed_review_sample'] = review_sample(findings['roots'], excluded)
    findings['sample_rule'] = 'seed rust-path-probe-20261116; up to ten per root then fill to twenty; exclude historical cases; no retry or success inference'
    output_path = root / 'comparison.json'
    with output_path.open('x') as stream:
        json.dump(findings, stream, ensure_ascii=False, indent=2)
    return output_path
