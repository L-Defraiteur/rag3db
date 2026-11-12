"""Proposition de reprise publique74ad : jamais invoquée durant sa préparation."""
import hashlib
import json
import os
import re
from pathlib import Path
import selectors
import signal
import subprocess
import sys
import time
import xml.etree.ElementTree as ET

scratch, out, previous, reference = map(Path, sys.argv[1:])
started = time.monotonic()
deadline = started + 1470
commit = '74ad9c25cd2a63e01abd54f25314992cda6fea60'
proc = None
phase = 'verification'

def write(name, data):
    (out / name).write_text(json.dumps(data, indent=2) + '\n')

def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()

def command(argv):
    remaining = deadline - time.monotonic()
    if remaining <= 0:
        raise TimeoutError('deadline preparation')
    return subprocess.check_output(argv, stderr=subprocess.STDOUT, timeout=remaining)

try:
    cg = next(x.split(':', 2)[2] for x in Path('/proc/self/cgroup').read_text().splitlines() if x.startswith('0::'))
    cgp = Path('/sys/fs/cgroup') / cg.lstrip('/')
    limits = {'memory.max': (cgp / 'memory.max').read_text().strip(),
              'memory.swap.max': (cgp / 'memory.swap.max').read_text().strip(),
              'scratch_bytes': os.statvfs(scratch).f_blocks * os.statvfs(scratch).f_frsize,
              'routes': Path('/proc/net/route').read_text(), 'started_monotonic': started}
    write('limites.json', limits)
    assert limits['memory.max'] == '34359738368'
    assert limits['memory.swap.max'] == '0'
    assert limits['scratch_bytes'] == 34359738368
    assert len(limits['routes'].splitlines()) == 1
    assert not any(scratch.iterdir())
    manifest = json.loads((previous / 'manifest-build.json').read_text())
    assert manifest['source_commit'] == commit and len(manifest['files']) == 834
    package = previous / 'package'
    assert os.statvfs(package).f_flag & os.ST_RDONLY
    for item in manifest['files']:
        path = package / item['path']
        assert path.is_relative_to(package) and '..' not in Path(item['path']).parts
        assert path.stat().st_size == item['bytes'] and digest(path) == item['sha256']
    assert digest(previous / 'source.pack') == '4b2291566a3fbfd56585bdacb933dbab7fd59774074b85a2fd9f671140c0fd19'
    phase = 'source_staging'
    source = scratch / 'source'
    command(['git', 'clone', '--shared', '--no-checkout', str(reference), str(source)])
    with (previous / 'source.pack').open('rb') as stream:
        subprocess.run(['git', '-C', str(source), 'unpack-objects', '-q'], stdin=stream,
                       check=True, timeout=max(1, deadline-time.monotonic()))
    command(['git', '-C', str(source), 'checkout', '--detach', commit])
    assert command(['git', '-C', str(source), 'rev-parse', 'HEAD']).decode().strip() == commit
    assert not command(['git', '-C', str(source), 'status', '--porcelain'])
    for private in ['test/transaction/quarantine_released_diagnostic.cpp', 'src/common/private_saturation_capture.cpp']:
        assert not (source / private).exists()
    for ext in ['vector', 'geo']:
        target = source / 'extension' / ext / 'build'
        target.mkdir()
        # Only the saved, verified extension is used; no build or installation.
        import shutil
        shutil.copy2(package / 'lib' / ext, target / ('lib' + ext + '.rag3db_extension'))
    tmp, work = scratch / 'test-tmp', scratch / 'test-work'
    tmp.mkdir(); work.mkdir()
    original_source = Path('/home/lucied/.cache/rag3weaver-build/cx-ci-format18-74ad-02/source')
    env = {'PATH': '/usr/bin:/bin', 'LANG': 'C', 'LC_ALL': 'C', 'TZ': 'UTC',
           'TMPDIR': str(tmp), 'RUNNER_TEMP': str(tmp), 'MAX_NUM_THREADS': '2'}
    binary = package / 'bin/transaction_test'
    assert manifest['build_id'].encode() in binary.read_bytes()
    argv = [str(binary), '--gtest_color=no', '--gtest_fail_fast', '--gtest_output=xml:' + str(out/'transaction.xml')]
    cmd = ['bwrap', '--unshare-user', '--unshare-net', '--ro-bind', '/', '/', '--proc', '/proc', '--dev', '/dev',
           '--ro-bind', str(source), str(original_source), '--ro-bind', str(source), str(source),
           '--bind', str(tmp), str(tmp), '--bind', str(work), str(work), '--bind', str(out), str(out),
           '--chdir', str(work), '/usr/bin/env', '-i', *[k+'='+v for k,v in env.items()], *argv]
    write('transaction-environment.json', env)
    write('transaction-argv.json', cmd)
    write('manifest-avant-test.json', manifest)
    phase = 'transaction_test'
    test_start = time.monotonic()
    test_deadline = min(test_start + 1440, deadline - 6)
    proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, start_new_session=True)
    sel = selectors.DefaultSelector(); sel.register(proc.stdout, selectors.EVENT_READ)
    with (out/'transaction.log').open('wb') as raw, (out/'transaction-timeline.jsonl').open('w') as timeline:
        while sel.get_map():
            remaining = test_deadline - time.monotonic()
            if remaining <= 0:
                raise TimeoutError('suite deadline1440/global1470')
            for key, _ in sel.select(min(1, remaining)):
                block = os.read(key.fd, 65536)
                if not block:
                    sel.unregister(key.fileobj)
                else:
                    raw.write(block); raw.flush()
                    timeline.write(json.dumps({'elapsed_suite_s': time.monotonic()-test_start,
                                               'received_text': block.decode(errors='replace')})+'\n')
                    timeline.flush()
        code = proc.wait(timeout=max(.01, test_deadline-time.monotonic()))
    write('transaction-exit.json', {'code': code, 'elapsed_s': time.monotonic()-test_start, 'waited': True})
    assert code == 0
    root = ET.parse(out/'transaction.xml').getroot()
    cases = list(root.iter('testcase'))
    skips = [c.get('classname')+'.'+c.get('name') for c in cases if c.find('skipped') is not None]
    failures = [c.attrib for c in cases if c.find('failure') is not None]
    selected = re.search(r'\[==========\] Running (\d+) tests', (out/'transaction.log').read_text(errors='replace'))
    disabled = [c for c in cases if c.get('status') == 'notrun']
    assert selected and int(selected[1]) == 359
    assert len(cases) == 360 and len(disabled) == 1 and not failures
    assert root.get('failures') == '0' and root.get('errors') == '0'
    assert sorted(skips) == ['VectorIndexCrashReopenProbe.OpenInAFreshProcess', 'WalReader.PrintAFile']
    for item in manifest['files']:
        assert digest(package/item['path']) == item['sha256']
    write('verdict.json', {'xml': root.attrib, 'skips': skips, 'failures': failures,
                          'elapsed_s': time.monotonic()-started, 'source': commit,
                          'build': False, 'installed': False, 'master_authorized': False})
except BaseException as exc:
    if proc is not None and proc.poll() is None:
        os.killpg(proc.pid, signal.SIGTERM)
        try:
            proc.wait(timeout=3)
        except subprocess.TimeoutExpired:
            os.killpg(proc.pid, signal.SIGKILL); proc.wait(timeout=3)
    write('premier-echec.json', {'phase': phase, 'type': type(exc).__name__, 'message': str(exc),
                               'elapsed_s': time.monotonic()-started, 'retry': False})
    raise
