"""Read-only primitives for the explicitly scoped C++ pass receipt."""
import hashlib
from pathlib import Path
import re
import xml.etree.ElementTree as ET

BUILD_ID = re.compile(r'rag3db-build-[0-9a-f]{40}(?:-[A-Za-z0-9]+)+')
RECOVERY_FIXTURE = 'ExtensionBuildIdTest.AtRecoveryAnExtensionFromAnotherBuildLeavesTheIndexBehind'
DROP_CASE = 'transaction~ddl~ddl_empty.DropNodeTableReclaim'
DROP_QUERY = 'call file_info() with num_pages match (s:stats) where s.id = 0 return num_pages <= s.numPages'
DROP_MESSAGE = '''Expected equality of these values:
  resultTuples[i]
    Which is: "False"
  testAnswer.expectedResult[i]
    Which is: "True"
Result tuple at index 0 did not match the expected value'''


def digest_bytes(data):
    return hashlib.sha256(data).hexdigest()


def excluded_scope(long_path, probabilistic_path):
    def names(path):
        return {line.strip() for line in Path(path).read_text().splitlines()
                if line.strip() and not line.lstrip().startswith('#')}
    long_names, probabilistic = names(long_path), names(probabilistic_path)
    return {'long': sorted(long_names), 'probabilistic': sorted(probabilistic),
            'intersection': sorted(long_names & probabilistic),
            'union_not_run': sorted(long_names | probabilistic)}


def verify_attestation(raw_path, root_path, expected_id):
    raw, root = Path(raw_path).read_bytes(), Path(root_path).read_bytes()
    if raw != root:
        raise ValueError('root attestation is not exact native stdout')
    text = raw.decode(errors='strict')
    emitted = re.findall(r'^BUILD_ID=(.+)$', text, re.M)
    if emitted != [expected_id] or set(BUILD_ID.findall(text)) != {expected_id}:
        raise ValueError('native identity absent, repeated or mixed')
    return {'raw_sha256': digest_bytes(raw), 'root_sha256': digest_bytes(root),
            'bytes': len(raw), 'build_id': emitted[0]}


def inspect_raw_identities(path, expected_id):
    """Keep every foreign ID visible; admit only the exact refusal fixture line."""
    raw = Path(path).read_bytes()
    text = raw.decode(errors='strict')
    prefix = 'rag3db-build-'
    if not expected_id.startswith(prefix):
        raise ValueError('invalid expected identity')
    commit = expected_id[len(prefix):len(prefix)+40]
    fake_commit = ''.join('e' if ch == 'f' else 'f' for ch in commit[:8])+commit[8:]
    fake_id = prefix+fake_commit+expected_id[len(prefix)+40:]
    active = None
    accepted_fixture_lines = []
    for number, line in enumerate(text.splitlines(), 1):
        match = re.match(r'^\[ RUN      \] (\S+)', line)
        if match:
            active = match[1]
        for marker in BUILD_ID.findall(line):
            if marker == expected_id:
                continue
            if not (active == RECOVERY_FIXTURE and marker == fake_id
                    and 'could not be loaded at open' in line
                    and 'was built from' in line and expected_id in line):
                raise ValueError('unapproved foreign build identity at line '+str(number))
            accepted_fixture_lines.append({'line': number, 'identity': marker,
                                            'case': active, 'text': line})
        if re.match(r'^\[ (?:      OK | FAILED  | SKIPPED )\]', line):
            active = None
    return {'sha256': digest_bytes(raw), 'bytes': len(raw),
            'expected_identity': expected_id, 'intentional_refusal_fixture': accepted_fixture_lines}


def admit_drop_failure(case, raw_case_text):
    if case.get('classname')+'.'+case.get('name') != DROP_CASE:
        raise ValueError('different failing e2e case')
    failures = case.findall('failure')
    if len(failures) != 1 or case.find('skipped') is not None or case.find('error') is not None:
        raise ValueError('different failure count or outcome')
    message = failures[0].text or failures[0].get('message', '')
    start = message.find('Expected equality of these values:')
    if start < 0 or ' '.join(message[start:].split()) != ' '.join(DROP_MESSAGE.split()):
        raise ValueError('different assertion, not historical False versus True')
    if DROP_QUERY not in raw_case_text:
        raise ValueError('missing historical page-reuse query or malformed raw case')
    return {'case': DROP_CASE, 'exception': 'historical page reuse False versus True only'}


def inspect_outcomes(xml_path, suite, drop_raw_text=''):
    root = ET.parse(xml_path).getroot()
    cases = list(root.iter('testcase'))
    if not cases or root.get('errors', '0') != '0':
        raise ValueError('empty suite or XML errors')
    allowed_skips = {'VectorIndexCrashReopenProbe.OpenInAFreshProcess', 'WalReader.PrintAFile'} if suite == 'transaction' else set()
    allowed_disabled = {'ReadOnlyOpenTest.DISABLED_MeasureCostOfRevalidationOnABigJournal'} if suite == 'transaction' else set()
    executed = 0
    skips, disabled, admitted = [], [], []
    for case in cases:
        name = case.get('classname')+'.'+case.get('name')
        if case.get('status') == 'notrun':
            if name not in allowed_disabled:
                raise ValueError('unexpected disabled test '+name)
            disabled.append(name)
            continue
        if case.get('status') != 'run':
            raise ValueError('case not actually run '+name)
        executed += 1
        if case.find('error') is not None:
            raise ValueError('test error '+name)
        if case.find('skipped') is not None:
            if name not in allowed_skips:
                raise ValueError('unexpected skipped test '+name)
            skips.append(name)
        if case.find('failure') is not None:
            if suite != 'e2e':
                raise ValueError('new failing test '+name)
            admitted.append(admit_drop_failure(case, drop_raw_text))
    if not executed:
        raise ValueError('zero tests actually executed')
    return {'xml': root.attrib, 'executed': executed, 'nominal_helper_skips': skips,
            'declared_disabled': disabled, 'historical_failure_exceptions': admitted}


def admitted_cpu(cpu_max, poste_threads):
    parts = cpu_max.split()
    if len(parts) != 2 or parts[0] == 'max':
        raise ValueError('missing effective poste CPU quota')
    quota, period, threads = int(parts[0]), int(parts[1]), int(poste_threads)
    if quota <= 0 or period <= 0 or threads <= 0:
        raise ValueError('invalid CPU admission')
    return {'jobs': min(8, threads, max(1, quota//period)),
            'quota_percent': 100*quota/period}
