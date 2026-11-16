#!/usr/bin/env python3
"""Relire les résultats figés ; aucune nouvelle analyse de fichier source."""
import argparse
from collections import Counter
import hashlib
from itertools import zip_longest
import json
import math
from pathlib import Path
import statistics


def digest(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def identity(row):
    return row['repository_id'], row['relative_path'], row['sha256']


def bucket():
    return {'files': 0, 'execution': Counter(), 'routing': Counter(), 'ast': Counter(),
        'capture': Counter(), 'strata': Counter(), 'amended_strata': Counter(),
        'scope_types': Counter(), 'parser_elapsed_ns': [], 'worker_wall_seconds_total': 0.0, 'compact_omitted': 0,
        'files_with_compact_omissions': 0, 'counter_disagreements': 0}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--campaign', type=Path, required=True)
    parser.add_argument('--manifest', type=Path, required=True)
    parser.add_argument('--amendment', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    results = args.campaign / 'results.jsonl'
    provenance = json.loads((args.campaign / 'provenance.json').read_text())
    if provenance['manifest_sha256'] != digest(args.manifest):
        raise ValueError('manifest differs from campaign provenance')
    with args.amendment.open() as stream:
        generated = {identity(json.loads(line)) for line in stream}
    languages = {}
    failures = []
    peak = 0
    generated_seen = set()
    with args.manifest.open() as selected, results.open() as recorded:
        for sequence, (expected, actual) in enumerate(zip_longest(selected, recorded), 1):
            if expected is None or actual is None:
                raise ValueError('manifest/result length differs')
            expected = json.loads(expected)
            row = json.loads(actual)
            if row['input'] != expected or row['sequence'] != sequence:
                raise ValueError('result identity or order differs from selection')
            measurement = row['measurement']
            language = expected['language']
            stats = languages.setdefault(language, bucket())
            stats['files'] += 1
            stats['worker_wall_seconds_total'] += row['controller_elapsed_seconds']
            for source, target in [('execution_status', 'execution'), ('routing_status', 'routing'),
                    ('ast_status', 'ast'), ('capture_status', 'capture')]:
                stats[target][measurement.get(source, 'not_reported')] += 1
            stats['strata'][expected['stratum']] += 1
            key = identity(expected)
            is_generated = key in generated
            if is_generated:
                generated_seen.add(key)
            stats['amended_strata']['confirmed_generated_upb' if is_generated else
                expected['stratum'] + '_not_in_upb_amendment'] += 1
            stats['scope_types'].update(measurement.get('scope_types', {}))
            elapsed = measurement.get('elapsed_ns')
            if elapsed is not None:
                stats['parser_elapsed_ns'].append(elapsed)
            omitted = measurement.get('compact_inventory_omitted', 0)
            stats['compact_omitted'] += omitted
            stats['files_with_compact_omissions'] += bool(omitted)
            reported, derived = measurement.get('reported_total_scopes'), measurement.get('scopes')
            if reported is not None and derived is not None and reported != derived:
                stats['counter_disagreements'] += 1
            peak = max(peak, row.get('process_peak_rss_bytes') or 0)
            if measurement['execution_status'] not in ('completed', 'not_attempted'):
                failures.append({'sequence': sequence, 'input': expected, 'measurement': measurement,
                    'worker_returncode': row.get('worker_returncode')})
    if generated_seen != generated:
        raise ValueError('amendment contains entries outside selected results')
    for stats in languages.values():
        timings = sorted(stats.pop('parser_elapsed_ns'))
        stats['parser_call_timing'] = {'samples': len(timings), 'total_ms': sum(timings) / 1e6,
            'median_ms': statistics.median(timings) / 1e6 if timings else None,
            'p95_ms': timings[math.ceil(.95 * len(timings)) - 1] / 1e6 if timings else None,
            'scope': 'worker call including lazy initialization, not file IO/hash/compact serialization'}
    summary = {'schema_version': 1, 'files': sum(v['files'] for v in languages.values()),
        'languages': languages, 'failures': failures,
        'max_observed_worker_peak_rss_bytes': peak or None,
        'peak_rss_scope': 'maximum VmHWM observation across reused workers; no per-file attribution; crash peak may be unobserved',
        'classification_amendment_files': len(generated_seen),
        'results_sha256': digest(results), 'manifest_sha256': digest(args.manifest),
        'amendment_sha256': digest(args.amendment), 'summarizer_sha256': digest(__file__),
        'binary_sha256': provenance['binary_sha256'], 'coverage_status': 'unjudged',
        'project_resolution': 'not_run', 'scope_counter_rule': 'recursive count of returned IR, separate from reported_total_scopes'}
    with args.output.open('x') as stream:
        json.dump(summary, stream, ensure_ascii=False, indent=2)


if __name__ == '__main__':
    main()
