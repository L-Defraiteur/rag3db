"""Strict source-specific receipts, without historical-red exceptions."""
from pathlib import Path
import json
import re
import xml.etree.ElementTree as ET

PROFILE = '-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF'
CASES = {
    'strings': 'StringRewriteAcrossAddColumn.AutomaticCheckpointsBetweenAddsAndDrops',
    'reader': 'CheckpointOpenReader.OpenReaderOnRelsAcrossAControlledCheckpointWindow',
}


def admitted_cpu(text, threads):
    quota, period = text.split()
    if quota == 'max' or int(quota) <= 0 or int(period) <= 0 or int(threads) <= 0:
        raise ValueError('missing or invalid effective quota')
    jobs = min(8, int(threads), int(quota) // int(period))
    if jobs < 1:
        raise ValueError('effective quota below one core')
    return {'jobs': jobs, 'quota_percent': int(quota) * 100 / int(period)}


def inspect_case(directory, case, engine, code):
    marker = 'rag3db-build-' + engine + PROFILE
    text = (directory / 'stdout.log').read_text()
    emitted = re.findall(r'^BUILD_ID=(.+)$', text, re.M)
    identities = set(re.findall(r'rag3db-build-[0-9a-f]{40}(?:-[A-Za-z0-9]+)+', text))
    if emitted != [marker] or identities != {marker}:
        raise ValueError('missing, repeated or mixed native BUILD_ID')
    root = ET.parse(directory / 'result.xml').getroot()
    cases = list(root.iter('testcase'))
    if len(cases) != 1 or cases[0].get('classname') + '.' + cases[0].get('name') != CASES[case]:
        raise ValueError('missing, additional or different native case')
    item = cases[0]
    if item.get('status') != 'run' or item.get('result') != 'completed':
        raise ValueError('native case did not complete')
    if item.find('skipped') is not None or item.find('error') is not None:
        raise ValueError('native skip or error')
    if int(root.get('tests', '-1')) != 1 or int(root.get('disabled', '-1')) != 0:
        raise ValueError('empty or disabled native scope')
    if root.tag != 'testsuites' or int(root.get('errors', '-1')) != 0:
        raise ValueError('XML root or errors')
    # GTest emits skipped on suites, not on the native testsuites root.
    # If another producer supplies the root field, require consistency too.
    if 'skipped' in root.attrib and int(root.attrib['skipped']) != 0:
        raise ValueError('XML root skipped count')
    suites = root.findall('testsuite')
    if len(suites) != 1 or suites[0].get('name') != item.get('classname'):
        raise ValueError('different or additional native suite')
    suite = suites[0]
    if any(int(suite.get(field, '-1')) != expected for field, expected in
           [('tests', 1), ('disabled', 0), ('errors', 0), ('skipped', 0)]):
        raise ValueError('XML suite counts or skips')
    if len(suite.findall('testcase')) != 1 or list(root.iter('error')) or list(root.iter('skipped')):
        raise ValueError('XML nested errors, skips or counts')
    failures = [dict(x.attrib, text=x.text) for x in item.findall('failure')]
    if any(int(node.get('failures', '-1')) != int(bool(failures)) for node in [root, suite]):
        raise ValueError('XML failure count disagrees')
    if code not in (0, 1) or bool(failures) != (code == 1):
        raise ValueError('native status and XML disagree')
    return {'case': CASES[case], 'build_id': marker, 'code': code,
            'verdict': 'ROUGE' if failures else 'VERT', 'failures': failures}


def verify_artifact_manifests(out, checksum, complete=True):
    recipe = json.loads((out / 'recipe.json').read_text())
    verified = []
    for variant in recipe['variants']:
        directory = out / 'raw' / variant['name'] / 'artifacts'
        if not (directory / 'manifest.json').is_file() and not complete:
            continue
        manifest = json.loads((directory / 'manifest.json').read_text())
        assert manifest['source'] == variant['source']
        assert manifest['build_id'] == 'rag3db-build-' + variant['engine'] + PROFILE
        for filename, record in manifest['files'].items():
            assert checksum(directory / filename) == record['sha256']

        verified.append(variant['name'])
    return verified
