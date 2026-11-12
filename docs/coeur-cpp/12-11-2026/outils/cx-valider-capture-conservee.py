#!/usr/bin/env python3
"""Lecture stricte post-arret uniquement : CSV/ XML conserves ; n'execute aucun moteur."""
import json
from pathlib import Path
import re
import sys
import xml.etree.ElementTree as ET

root = Path(sys.argv[1])
fields = ['at_ns', 'read_end_ns', 'used_bytes', 'ddl', 'pk_calls', 'pk_completed_ns',
          'pk_active', 'active_tx', 'writers_tx', 'oldest_snapshot_ts', 'transferred_rows',
          'published_commits', 'actor_ok', 'actor_error', 'writer_ok', 'writer_error']
schema = ['cx-saturation', 'schema', *fields]
errors = []
raw = (root / 'capture.csv').read_bytes() if (root / 'capture.csv').exists() else b''
try:
    text = raw.decode('ascii')
except UnicodeDecodeError:
    text = raw.decode('ascii', errors='replace')
lines = text.splitlines()
if not lines or lines[0].split(',') != schema:
    errors.append('exact schema missing/mismatch')
seen = {'schema': 0, 'sample': 0, 'first_meta': 0, 'first': 0, 'first_saved': 0, 'status': 0}
samples, first, metadata, footer = [], None, None, None
first_saved_valid_prefix = False


def uint(value):
    if not re.fullmatch(r'[0-9]+', value, flags=re.ASCII):
        raise ValueError('not an unsigned decimal integer: ' + repr(value))
    result = int(value)
    if result > 2**64 - 1:
        raise ValueError('uint64 overflow')
    return result


def pairs(values, keys):
    result = {}
    for value in values:
        key, number = value.split('=', 1)
        if key in result:
            raise ValueError('duplicate key ' + key)
        result[key] = uint(number)
    if set(result) != set(keys):
        raise ValueError('wrong keys')
    return result


for no, line in enumerate(lines, 1):
    try:
        if not line.isascii():
            raise ValueError('CSV row not ASCII')
        row = line.split(',')
        if len(row) < 2 or row[0] != 'cx-saturation' or row[1] not in seen:
            raise ValueError('unknown CSV record')
        kind = row[1]
        if seen['status']:
            raise ValueError('record after final status')
        seen[kind] += 1
        if kind != 'sample' and seen[kind] != 1:
            raise ValueError('duplicate ' + kind)
        if kind == 'schema':
            if no != 1 or row != schema:
                raise ValueError('schema order/value')
        elif kind in ('sample', 'first'):
            if len(row) != 18:
                raise ValueError('expected 16 numeric columns')
            values = dict(zip(fields, map(uint, row[2:])))
            if values['at_ns'] > values['read_end_ns'] or values['ddl'] > 6:
                raise ValueError('clock interval/DDL value')
            if kind == 'sample':
                if samples and values['at_ns'] < samples[-1]['read_end_ns']:
                    raise ValueError('overlapping/reversed sampler reads')
                for key in ['pk_calls', 'pk_completed_ns', 'transferred_rows', 'published_commits',
                            'actor_ok', 'actor_error', 'writer_ok', 'writer_error']:
                    if samples and values[key] < samples[-1][key]:
                        raise ValueError('cumulative counter decreased: ' + key)
                samples.append(values)
            else:
                if metadata is None or seen['first_saved']:
                    raise ValueError('first outside meta/marker sequence')
                first = values
        elif kind == 'first_meta':
            if first is not None or seen['first_saved']:
                raise ValueError('meta order')
            metadata = pairs(row[2:], ('reason', 'phase', 'site', 'tx', 'requested_bytes'))
            if metadata['reason'] not in (1, 2) or metadata['phase'] > 2 or metadata['site'] > 2:
                raise ValueError('meta enum out of domain')
            if metadata['reason'] == 1 and metadata['requested_bytes'] == 0:
                raise ValueError('POOL requested bytes zero')
        elif kind == 'first_saved':
            if len(row) != 2 or metadata is None or first is None:
                raise ValueError('first_saved without valid complete first/meta')
            first_saved_valid_prefix = not errors
        elif kind == 'status':
            footer = pairs(row[2:], ('complete', 'first_state', 'sample_count', 'overflow'))
            if footer['complete'] > 1 or footer['first_state'] > 2:
                raise ValueError('footer domain')
    except (ValueError, TypeError) as exc:
        errors.append({'line': no, 'error': str(exc)})
if raw and not raw.endswith(b'\n'):
    errors.append('last CSV line not terminated')
if len(samples) > 1280:
    errors.append('sample capacity exceeded')
if footer is None:
    errors.append('final footer missing')
else:
    if footer['sample_count'] != len(samples):
        errors.append('sample_count mismatch')
    if footer['complete'] != 1 or footer['overflow'] or footer['first_state'] == 1:
        errors.append('footer declares incomplete/overflow')
    if footer['first_state'] == 2 and not (first_saved_valid_prefix and seen['first_meta'] == seen['first'] == seen['first_saved'] == 1):
        errors.append('published first missing valid meta/record/marker')
    if footer['first_state'] == 0 and any(seen[k] for k in ['first_meta', 'first', 'first_saved']):
        errors.append('unexpected first for state0')
    if metadata and metadata['reason'] == 2 and first and first['used_bytes'] != 2**64 - 1:
        errors.append('FRAME_GROUPS used_bytes must be unknown sentinel')

result = {'capture_complete_strict': not errors, 'errors': errors, 'records': seen,
          'first_saved_valid_prefix': first_saved_valid_prefix, 'first_meta': metadata,
          'first': first, 'footer': footer, 'samples': len(samples),
          'pk_completed_ns_excludes_active_calls': True,
          'no_cost_exclusion_if_pk_active_positive': True}
if first:
    before = [r for r in samples if r['read_end_ns'] <= first['at_ns']]
    after = [r for r in samples if r['at_ns'] >= first['read_end_ns']]
    result['nearest_sample_before_first'] = before[-1] if before else None
    result['nearest_sample_after_first'] = after[0] if after else None
try:
    xmlroot = ET.parse(root / 'result.xml').getroot()
    cases = list(xmlroot.iter('testcase'))
    target = [c for c in cases if c.attrib.get('name') == 'DdlEveryMinute' and
              c.attrib.get('classname') == 'TwentyWriters']
    exact = (xmlroot.attrib.get('tests') == '1' and len(cases) == len(target) == 1 and
             target[0].attrib.get('status') == 'run' and target[0].attrib.get('result') == 'completed')
    result['xml_exactly_one_requested_test_run'] = exact
    result['xml_root'] = dict(xmlroot.attrib)
    result['xml_cases'] = [dict(c.attrib) for c in cases]
    result['xml_failures'] = [f.attrib for f in xmlroot.iter('failure')]
except (ET.ParseError, OSError) as exc:
    result['xml_exactly_one_requested_test_run'] = False
    result['xml_error'] = str(exc)
(root / 'validation-stricte.json').write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps(result, indent=2))
sys.exit(0 if result['capture_complete_strict'] and result['xml_exactly_one_requested_test_run'] else 2)
