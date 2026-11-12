#!/usr/bin/env python3
"""Un passage prive35ea ; montage/preflight sans test ou runtime explicitement autorise.
Meme ticket annulation09nov ; aucune compilation, aucun retry.
"""
import hashlib
import json
import os
from pathlib import Path
import resource
import selectors
import signal
import subprocess
import sys
import time
import xml.etree.ElementTree as ET

MODE, PACKAGE, OUTPUT, ANCHOR, MANIFEST, REVIEW = sys.argv[1:]
PACKAGE, OUTPUT, ANCHOR, MANIFEST, REVIEW = map(Path, (PACKAGE, OUTPUT, ANCHOR, MANIFEST, REVIEW))
assert MODE in ('preflight', 'run')
SOURCE = ANCHOR / 'source'
WORK = ANCHOR / 'runtime/work'
TMP = ANCHOR / 'runtime/tmp'
MAX_LOGS = 16 * 1024**2
CAPTURE_RESERVE = 1024**2  # <1280 * 512 + schema/meta/first/status (<1Mio).
STREAM_LIMIT = MAX_LOGS - CAPTURE_RESERVE
started = time.monotonic()
deadline = started + 1188  # Arret+collecte avant RuntimeMax1200, puis filet10s.


def sha(path):
    h = hashlib.sha256()
    with path.open('rb') as stream:
        for chunk in iter(lambda: stream.read(1024**2), b''):
            h.update(chunk)
    return h.hexdigest()


def write(name, obj):
    (OUTPUT / name).write_text(json.dumps(obj, indent=2) + '\n')


manifest = json.loads(MANIFEST.read_text())
review = json.loads(REVIEW.read_text())
assert manifest['source_commit'] == '35ea70d3a4a5cc4917586de02f6bfc862c76a02c'
assert manifest['source_commit'] == review['source_commit']
for item in manifest['files']:
    assert sha(PACKAGE / item['path']) == item['sha256'], item['path']
for item in review['host_direct_dependencies'].values():
    assert sha(Path(item['resolved'])) == item['sha256']
for artifact in ('bin/concurrence_test', 'extension/vector/build/libvector.rag3db_extension',
                 'extension/geo/build/libgeo.rag3db_extension'):
    assert review['build_id'].encode() in (PACKAGE / artifact).read_bytes()
assert SOURCE == Path(manifest['source_root_at_build'])
assert (SOURCE / 'extension/vector/build/libvector.rag3db_extension').resolve() == (
    SOURCE / 'extension/vector/build/libvector.rag3db_extension')
assert sha(SOURCE / 'extension/vector/build/libvector.rag3db_extension') == next(
    i['sha256'] for i in manifest['files'] if i['path'] == 'extension/vector/build/libvector.rag3db_extension')
assert os.statvfs(SOURCE).f_flag & os.ST_RDONLY
assert os.statvfs(PACKAGE).f_flag & os.ST_RDONLY
assert not (os.statvfs(OUTPUT).f_flag & os.ST_RDONLY)
assert os.statvfs(ANCHOR).f_blocks * os.statvfs(ANCHOR).f_frsize == 2 * 1024**3
assert len(Path('/proc/net/route').read_text().splitlines()) == 1
cg = next(line.split(':', 2)[2] for line in Path('/proc/self/cgroup').read_text().splitlines()
          if line.startswith('0::'))
cgroup = Path('/sys/fs/cgroup') / cg.lstrip('/')
limits = {'cgroup': cg, 'memory.max': (cgroup / 'memory.max').read_text().strip(),
          'memory.swap.max': (cgroup / 'memory.swap.max').read_text().strip(),
          'scratch_bytes': os.statvfs(ANCHOR).f_blocks * os.statvfs(ANCHOR).f_frsize,
          'network_routes': Path('/proc/net/route').read_text(), 'source_readonly': True,
          'package_readonly': True, 'output_persistent_outside_scratch': not OUTPUT.is_relative_to(ANCHOR),
          'logs_limit_bytes': MAX_LOGS, 'stream_limit_bytes': STREAM_LIMIT,
          'capture_structural_reserve_bytes': CAPTURE_RESERVE}
assert limits['memory.max'] == '8589934592' and limits['memory.swap.max'] == '0'
assert limits['output_persistent_outside_scratch']
WORK.mkdir(parents=True)
TMP.mkdir(parents=True)
# Pas de HOME/GTEST/SCENARIO/buffer/threads/LD_LIBRARY_PATH herites.
env = {'PATH': '/usr/bin:/bin', 'LANG': 'C', 'LC_ALL': 'C', 'TZ': 'UTC',
       'TMPDIR': str(TMP), 'RUNNER_TEMP': str(TMP),
       'CX_SATURATION_OUTPUT': str(OUTPUT / 'capture.csv')}
argv = [str(PACKAGE / 'bin/concurrence_test'), '--gtest_filter=TwentyWriters.DdlEveryMinute',
        '--gtest_color=no', '--gtest_output=xml:' + str(OUTPUT / 'xml.pipe')]
plan = {'mode': MODE, 'source_commit': manifest['source_commit'],
        'manifest_sha256': sha(MANIFEST), 'review_sha256': sha(REVIEW),
        'driver_sha256': sha(Path(__file__)), 'argv': argv, 'environment': env,
        'cwd': str(WORK), 'package_files_verified': len(manifest['files']),
        'build_id': review['build_id'], 'native_spawned': 0,
        'xml_transport': 'private FIFO -> controller -> persistent result.xml',
        'runtime_budget_s': 1200, 'stop_budget_s': 10, 'automatic_retry': False}
write(MODE + '-plan.json', plan)
write(MODE + '-limits.json', limits)
if MODE == 'preflight':
    write('preflight-ok.json', {'native_spawned': 0, 'mount_and_hash_checks': True,
                               'scenario_run': False, 'driver_sha256': plan['driver_sha256']})
    sys.exit(0)

# Tout l'armement natif reste dans le binaire fige. Ce marqueur interdit une
# seconde tentative meme apres interruption du controller.
with (OUTPUT / 'attempt.json').open('x') as marker:
    json.dump({'argv': argv, 'environment': env, 'start_monotonic': started,
               'driver_sha256': plan['driver_sha256']}, marker)
for name in ('capture.csv', 'stdout.log', 'result.xml', 'xml.pipe'):
    assert not (OUTPUT / name).exists(), name
os.mkfifo(OUTPUT / 'xml.pipe', 0o600)
xml_fd = os.open(OUTPUT / 'xml.pipe', os.O_RDWR | os.O_NONBLOCK)
sel = selectors.DefaultSelector()
sel.register(xml_fd, selectors.EVENT_READ, 'xml')
outputs = {'stdout': (OUTPUT / 'stdout.log').open('xb'), 'xml': (OUTPUT / 'result.xml').open('xb')}
bytes_written = 0
stop_reason = None
child = None
kill_events = []


def interrupt(signum, _frame):
    global stop_reason
    stop_reason = 'controller_signal_' + str(signum)


for sig in (signal.SIGTERM, signal.SIGINT):
    signal.signal(sig, interrupt)


def no_core():
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))


def stop_child():
    if child is None or child.poll() is not None:
        return
    kill_events.append('SIGTERM private process group')
    try:
        os.killpg(child.pid, signal.SIGTERM)
    except ProcessLookupError:
        child.wait(timeout=1)
        return
    try:
        child.wait(timeout=2)
    except subprocess.TimeoutExpired:
        kill_events.append('SIGKILL private process group')
        os.killpg(child.pid, signal.SIGKILL)
        child.wait(timeout=3)


try:
    child = subprocess.Popen(argv, cwd=WORK, env=env, stdin=subprocess.DEVNULL,
                             stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                             start_new_session=True, preexec_fn=no_core)
    plan['native_spawned'] = 1
    write('native-pid.json', {'pid': child.pid, 'pgid': child.pid, 'controller': os.getpid()})
    os.set_blocking(child.stdout.fileno(), False)
    sel.register(child.stdout, selectors.EVENT_READ, 'stdout')
    exit_observed = None
    while True:
        if time.monotonic() >= deadline and stop_reason is None:
            stop_reason = 'runtime_deadline'
        capture = OUTPUT / 'capture.csv'
        if capture.exists() and capture.stat().st_size > CAPTURE_RESERVE and stop_reason is None:
            stop_reason = 'capture_structural_bound_exceeded'
        if stop_reason:
            stop_child()
        events = sel.select(timeout=0.1)
        read_any = False
        for key, _ in events:
            data = os.read(key.fd, 65536)
            if not data:
                if key.data == 'stdout':
                    sel.unregister(key.fileobj)
                continue
            read_any = True
            remaining = STREAM_LIMIT - bytes_written
            if len(data) > remaining:
                data = data[:max(0, remaining)]
                stop_reason = stop_reason or 'logs_budget_exceeded'
            outputs[key.data].write(data)
            outputs[key.data].flush()
            bytes_written += len(data)
        if child.poll() is not None:
            if read_any:
                exit_observed = None
            elif exit_observed is None:
                exit_observed = time.monotonic()
            elif time.monotonic() - exit_observed > 0.3:
                break
    child.wait(timeout=1)
finally:
    stop_child()
    for stream in outputs.values():
        stream.flush()
        stream.close()
    sel.close()
    os.close(xml_fd)

# Jamais de retry, et statut GTest distinct de celui des donnees de capture.
result = {'native_spawned': 1, 'child_pid': child.pid, 'child_returncode': child.returncode,
          'stop_reason': stop_reason, 'kill_events': kill_events,
          'elapsed_s': time.monotonic() - started, 'streams_bytes': bytes_written,
          'automatic_retry': False, 'child_waited': True, 'scenario': 'TwentyWriters.DdlEveryMinute'}
xml = OUTPUT / 'result.xml'
try:
    tree = ET.parse(xml)
    result['gtest_xml'] = dict(tree.getroot().attrib)
except (ET.ParseError, OSError):
    result['gtest_xml'] = 'missing_or_incomplete'
capture = OUTPUT / 'capture.csv'
rows = capture.read_text().splitlines() if capture.exists() else []
status = [r for r in rows if r.startswith('cx-saturation,status,')]
first_saved = 'cx-saturation,first_saved' in rows
first_rows = [r for r in rows if r.startswith('cx-saturation,first,')]
meta = [r for r in rows if r.startswith('cx-saturation,first_meta,')]
samples = [r for r in rows if r.startswith('cx-saturation,sample,')]
shape_ok = all(len(r.split(',')) == 18 for r in first_rows + samples)
values = dict(x.split('=', 1) for x in status[-1].split(',')[2:]) if len(status) == 1 else {}
complete = bool(rows and rows[0].startswith('cx-saturation,schema,') and shape_ok and
                values.get('complete') == '1' and values.get('overflow') == '0' and
                values.get('first_state') in ('0', '2') and
                values.get('sample_count') == str(len(samples)) and
                (values.get('first_state') != '2' or (first_saved and len(first_rows) == len(meta) == 1)))
result['capture'] = {'complete': complete and stop_reason is None,
                     'partial_first_saved': first_saved and len(first_rows) == len(meta) == 1 and shape_ok,
                     'samples': len(samples), 'footer': values, 'csv_shape_ok': shape_ok}
result['capture_complete_is_not_test_success'] = True
result['artifact_bytes'] = {n: (OUTPUT / n).stat().st_size for n in ('stdout.log', 'result.xml', 'capture.csv') if (OUTPUT / n).exists()}
assert sum(result['artifact_bytes'].values()) <= MAX_LOGS
write('result.json', result)
# Toute modification d'artefact/dependance apres le passage invaliderait le paquet.
for item in manifest['files']:
    assert sha(PACKAGE / item['path']) == item['sha256'], item['path']
write('package-after.json', {'verified_files': len(manifest['files']), 'mismatches': 0})
write('SHA256.json', {p.name: sha(p) for p in OUTPUT.iterdir() if p.is_file() and p.name != 'SHA256.json'})
if stop_reason is not None or not result['capture']['complete']:
    sys.exit(2)
sys.exit(child.returncode if child.returncode >= 0 else 1)
