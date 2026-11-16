"""Strict receipt for a missing public diagnostic, never a green delivery pass."""
import hashlib
from pathlib import Path
import re
import xml.etree.ElementTree as ET

SOURCE = '5099a367ddf1e48db2b4065d91a61403de994571'
ENGINE = '59ac9c86bc2b3529b590a7462ab7051e3ad26b8a'
CASE = 'VectorIndexDiagnosticsTest.ACompleteIndexReportsAKnownZero'
BINDER = 'Binder exception: Variable rows_outside_graph is not in scope.'
BUILD_ID = re.compile(r'rag3db-build-[0-9a-f]{40}(?:-[A-Za-z0-9]+)+')


def admitted_cpu(cpu_max, poste_threads):
    quota, period = cpu_max.split()
    if quota == 'max' or min(int(quota), int(period), int(poste_threads)) <= 0:
        raise ValueError('invalid effective admission')
    return {'jobs': min(8, int(poste_threads), max(1, int(quota)//int(period))),
            'quota_percent': 100*int(quota)/int(period)}


def qualify_red(xml_path, stdout_path, code, marker):
    raw = Path(stdout_path).read_bytes()
    text = raw.decode('utf-8', errors='strict')
    if not marker.startswith('rag3db-build-'+ENGINE+'-') or '-dirty-' in marker:
        raise ValueError('unexpected engine identity')
    if re.findall(r'^BUILD_ID=(.+)$', text, re.M) != [marker] or set(BUILD_ID.findall(text)) != {marker}:
        raise ValueError('missing, repeated or foreign native identity')
    root = ET.parse(xml_path).getroot()
    cases = list(root.iter('testcase'))
    if code != 1 or len(cases) != 1 or root.get('errors', '0') != '0':
        raise ValueError('not the one expected native failure')
    case = cases[0]
    if case.get('classname')+'.'+case.get('name') != CASE or case.get('status') != 'run':
        raise ValueError('wrong case or not executed')
    failures = case.findall('failure')
    if len(failures) != 1 or case.find('error') is not None or case.find('skipped') is not None:
        raise ValueError('different outcome')
    message = failures[0].text or failures[0].get('message', '')
    # Tout préfixe, suffixe ou échec de prémisse doit refuser. Le format GTest
    # natif et ce message seront qualifiés lors du seul run admis, sans élargir
    # silencieusement ce parseur si la forme observée diffère.
    expected = ('Value of: result->isSuccess()\n  Actual: false\nExpected: true\n'
                '[check: rows-outside-graph-is-readable] '+BINDER)
    location = r'(?:/[^\n]+/)?test/transaction/vector_index_diagnostics_test\.cpp:[0-9]+\n'
    body = re.sub('^'+location, '', message, count=1)
    if body.strip() != expected or BINDER not in text:
        raise ValueError('not the exact public SHOW_INDEXES binder refusal')
    return {'status': 'EXPECTED_BINDER_RED_OBSERVED', 'case': CASE,
            'native_exit': code, 'build_id': marker,
            'stdout_sha256': hashlib.sha256(raw).hexdigest(),
            'product_green': False, 'retry': False}


def verify_native(out, manifest):
    result = qualify_red(out/'raw/red/result.xml', out/'raw/red/stdout.log',
                         manifest['native_exit'], manifest['build_id_header'])
    receipt = __import__('json').loads((out/'raw/parsing/receipt.json').read_text())
    if receipt['result'] != result or receipt['source'] != SOURCE:
        raise ValueError('native receipt differs')
    return result
