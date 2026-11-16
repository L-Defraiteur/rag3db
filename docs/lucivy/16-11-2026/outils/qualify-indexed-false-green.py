"""Fresh bounded validation of three configuration tests, reusing only our red target."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import runpy
import subprocess
import stat
import time

SOURCE = '890c86af44989f3fea6c6fa62fd40d688b4084df'
OLD_SOURCE = '0c795f99e5f49926a510971c521eb056a4880adc'
OLD_NONCE = '3552903efab590b44f44cd8002f7b028'
OLD_ELF_SHA256 = 'c2478e972de2295a79c41cd651461653f38706e02cccd2ceace45c49fe33b01b'
SUPERVISOR_SHA256 = '6439fdd8b52c85fa4e74af0def2c667ef0bedae9b814cbbfd48e49946c4f74ea'
TEST_TARGET = 'test_text_field_indexed_false'
TESTS = ['unsupported_text_indexing_options_are_reported_once_per_field',
         'warning_escapes_field_names_without_extra_lines',
         'reporting_preserves_existing_field_indexing']

# Shared checks are copied byte-for-byte from the reviewed red package.
# Loading them performs no test, command or scope admission.
base_path = Path(__file__).absolute().with_name('qualify-indexed-false-red.py')
current = Path('/')
for part in base_path.parts[1:]:
    current /= part
    assert not stat.S_ISLNK(current.lstat().st_mode), f'symlink refused: {current}'
assert base_path.lstat().st_uid == os.getuid() and stat.S_ISREG(base_path.lstat().st_mode)
assert hashlib.sha256(base_path.read_bytes()).hexdigest() == '0a269dd90b0ca745811713e21f66b45fda563074bfc06dd6993f04ab03bd2352'
BASE = runpy.run_path(str(base_path), run_name='shared_red_checks')
checked_path = BASE['checked_path']
cpu_jobs = BASE['cpu_jobs']


def classify_scan(records, baseline):
    """Do not turn inaccessible baseline metadata into a claim of universal quiet."""
    allowed_unknown = {(r['pid'], r['uid'], r['start'], r['cgroup']) for r in baseline}
    blocked, unknown = [], []
    for row in records:
        if not row['present']:
            continue
        identity = (row['pid'], row['uid'], row['start'], row['cgroup'])
        if row['owned'] or row['attributed']:
            blocked.append(dict(record=row, reason='attributed_unknown' if row['errors'] else 'known_activity'))
        elif row['errors']:
            # This is not a universal absence-of-processes gate. Unattributed
            # opaque metadata stays UNKNOWN; baseline membership is evidence,
            # not a claim that an unrelated process is absent.
            unknown.append(dict(row, baseline_identity=identity in allowed_unknown))
    return dict(admitted=not blocked, blocked=blocked, unknown=unknown,
                scope='owned identities and attributable target/worktree activity only',
                universal_quiescence_claimed=False)


def collect_scan(data):
    owned = {int(c['pid']): c['debut'] for c in json.loads(Path(data['old_children']).read_text())}
    roots = [data['target'], data['worktree']]
    records = []
    for path in Path('/proc').iterdir():
        if not path.name.isdigit():
            continue
        row = dict(pid=int(path.name), uid=None, start=None, cgroup=None, present=True,
                   owned=False, attributed=False, exe=None, cwd=None, related_fds=[],
                   accessible_fd_count=0, errors=[])
        try:
            row['uid'] = path.stat().st_uid
            if row['uid'] != os.getuid():
                continue
            fields = (path / 'stat').read_text().rsplit(') ', 1)[1].split()
            row['start'] = fields[19]
            row['owned'] = owned.get(row['pid']) == row['start']
            if fields[0] == 'Z':
                row['present'] = False; row['state'] = 'exited_zombie'
                records.append(row); continue
        except (FileNotFoundError, ProcessLookupError):
            continue
        except OSError as error:
            row['errors'].append(dict(field='identity', errno=error.errno, error=str(error)))
            row['owned'] = row['pid'] in owned
        try:
            row['cgroup'] = (path / 'cgroup').read_text().strip()
        except OSError as error:
            row['errors'].append(dict(field='cgroup', errno=error.errno, error=str(error)))
        for field in ['exe', 'cwd']:
            try:
                row[field] = os.readlink(path / field).removesuffix(' (deleted)')
                if any(row[field] == root or row[field].startswith(root + '/') for root in roots):
                    row['attributed'] = True
            except OSError as error:
                row['errors'].append(dict(field=field, errno=error.errno, error=str(error)))
        try:
            fds = list((path / 'fd').iterdir())
        except OSError as error:
            fds = []
            row['errors'].append(dict(field='fd_directory', errno=error.errno, error=str(error)))
        for fd in fds:
            try:
                value = os.readlink(fd).removesuffix(' (deleted)')
                row['accessible_fd_count'] += 1
                if any(value == root or value.startswith(root + '/') for root in roots):
                    row['related_fds'].append(dict(fd=fd.name, target=value)); row['attributed'] = True
            except FileNotFoundError:
                pass  # An individual descriptor may close during this read-only scan.
            except OSError as error:
                row['errors'].append(dict(field='fd/' + fd.name, errno=error.errno, error=str(error)))
        try:
            current = (path / 'stat').read_text().rsplit(') ', 1)[1].split()[19]
            if current != row['start']:
                row['errors'].append(dict(field='identity', error='PID changed during scan'))
        except (FileNotFoundError, ProcessLookupError):
            row['present'] = False
        records.append(row)
    return records


def save_scan(data, records, assessment, phase):
    directory = checked_path(data['scan_directory'])
    path = directory / f"{phase}-{time.monotonic_ns()}.json"
    payload = dict(phase=phase, observed_at=time.monotonic(), records=records, assessment=assessment)
    with path.open('x') as file:
        json.dump(payload, file, indent=2); file.flush(); os.fsync(file.fileno())
    return path


def quiescence(data):
    old = json.loads(checked_path(data['old_receipt'], kind='file').read_text())
    verdict = json.loads(checked_path(data['old_verdict'], kind='file').read_text())
    records = collect_scan(data)
    assessment = classify_scan(records, data['baseline_unknown'])
    snapshot = save_scan(data, records, assessment, 'check')
    # Evidence exists even when these assertions refuse admission.
    assert old['etat'] == 'termine' and old['pid'] == 856197 and old['debut'] == '117998665'
    assert verdict['code'] == 1 and verdict['erreur'] is None and not verdict['restes'] and verdict['nettoye']
    assert assessment['admitted'], f'owned/attributed activity or new unknown; see {snapshot}'
    return dict(old_authority='terminated', old_pid=old['pid'], old_start=old['debut'],
                assessment=assessment, snapshot=str(snapshot), checked_at=time.monotonic())


def check_preflight(filename, scope_exists):
    manifest = checked_path(filename, kind='file')
    data = json.loads(manifest.read_text())
    package = checked_path(data['package'])
    assert manifest.parent == package and data['uid'] == os.getuid()
    assert package.stat().st_ino == data['package_inode'] and package.stat().st_dev == data['package_device']
    assert re.fullmatch(r'[0-9a-f]{32}', data['nonce'])
    target = checked_path(data['target'])
    checked_path(data['worktree'])
    checked_path(data['output'], exists=False)
    checked_path(data['scope'], exists=scope_exists)
    assert target.stat().st_ino == data['target_inode'] and target.stat().st_dev == data['target_device']
    creator_path = checked_path(str(target / 'validation-provenance.json'), kind='file')
    creator = json.loads(creator_path.read_text())
    assert creator['source'] == OLD_SOURCE and creator['nonce'] == OLD_NONCE and creator['uid'] == os.getuid()
    assert hashlib.sha256(creator_path.read_bytes()).hexdigest() == data['target_creator_sha256']
    elf = checked_path(data['old_elf'], kind='file')
    assert elf.is_relative_to(target) and hashlib.sha256(elf.read_bytes()).hexdigest() == OLD_ELF_SHA256
    assert hashlib.sha256(checked_path(str(package / 'red-test-elf'), kind='file').read_bytes()).hexdigest() == OLD_ELF_SHA256
    for relative, expected in data['copy_hashes'].items():
        assert not Path(relative).is_absolute() and '..' not in Path(relative).parts
        copy = checked_path(str(package / relative), kind='file')
        assert hashlib.sha256(copy.read_bytes()).hexdigest() == expected, f'copy hash mismatch: {relative}'
    quiescence(data)
    return data


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--preflight', required=True)
    args = parser.parse_args()
    admission = check_preflight(args.preflight, scope_exists=True)
    package = Path(admission['package'])
    output = Path(admission['output'])
    target = Path(admission['target'])
    root = Path(admission['worktree'])
    assert subprocess.check_output(['git', '-C', str(root), 'rev-parse', 'HEAD'], text=True).strip() == SOURCE
    assert not subprocess.check_output(['git', '-C', str(root), 'status', '--porcelain'], text=True).strip()
    receipt_path = checked_path(os.environ['RAG3WEAVER_VALIDATION_RECU'], kind='file')
    assert receipt_path.parents[1] == Path(admission['scope'])
    receipt = json.loads(receipt_path.read_text())
    bound = json.loads((Path(admission['scope']) / 'scope-provenance.json').read_text())
    deadline = receipt['echeance']
    assert receipt['etat'] == 'actif' and receipt['script_sha256'] == SUPERVISOR_SHA256
    assert 0 < deadline - time.monotonic() <= 600 and deadline <= bound['deadline']
    assert bound['nonce'] == admission['nonce']
    cgroup = Path('/proc/self/cgroup').read_text().split('0::', 1)[1].strip()
    cgroup_root = Path('/sys/fs/cgroup') / cgroup.lstrip('/')
    memory = {name: (cgroup_root / name).read_text().strip() for name in ['memory.max', 'memory.high']}
    assert memory == {'memory.max': '34359738368', 'memory.high': 'max'}
    jobs, cpu = cpu_jobs(cgroup_root)
    assert os.environ.get('LUCIVY_CORPUS_ABSENT_OK') != '1'
    output.mkdir(exist_ok=False)
    entry = dict(source=SOURCE, target_creator=OLD_SOURCE,
                 old_nonce=OLD_NONCE, nonce=admission['nonce'], cgroup=cgroup, memory=memory,
                 jobs=jobs, cpu=cpu, deadline=deadline, quiescence=quiescence(admission))
    (output / 'entry.json').write_text(json.dumps(entry, indent=2))
    env = dict(os.environ, CARGO_TARGET_DIR=str(target), CARGO_BUILD_JOBS=str(jobs),
               CARGO_TERM_COLOR='never', CARGO_NET_OFFLINE='true')

    def budget():
        live = json.loads(receipt_path.read_text())
        assert live['etat'] == 'actif' and live['echeance'] == deadline and deadline-time.monotonic() > 5

    def run(argv, name):
        budget()
        with (output / name).open('w') as file:
            child = subprocess.Popen(argv, cwd=root, env=env, stdout=file, stderr=subprocess.STDOUT)
            start = Path(f'/proc/{child.pid}/stat').read_text().rsplit(') ', 1)[1].split()[19]
            (output / (name + '.identity.json')).write_text(json.dumps(dict(pid=child.pid, start=start, argv=argv)))
            code = child.wait()
            file.flush(); os.fsync(file.fileno())
        budget()
        if code:
            with (output / 'first-red.json').open('x') as file:
                json.dump(dict(source=SOURCE, phase=name, code=code), file, indent=2)
                file.flush(); os.fsync(file.fileno())
            raise SystemExit(code if code > 0 else 1)
        return (output / name).read_text()

    log = run(['cargo', 'test', '--offline', '--release', f'-j{jobs}', '-p', 'lucivy-core',
               '--test', TEST_TARGET, '--no-run', '--message-format=json-render-diagnostics'], 'compile.jsonl')
    artifacts = []
    for line in log.splitlines():
        try:
            item = json.loads(line)
        except ValueError:
            continue
        if item.get('reason') == 'compiler-artifact' and item.get('target', {}).get('name') == TEST_TARGET and item.get('executable'):
            artifacts.append(checked_path(item['executable'], kind='file'))
    assert len(artifacts) == 1 and artifacts[0].is_relative_to(target)
    executable = artifacts[0]
    inventory = run([str(executable), '--list'], 'inventory.log')
    assert sorted(line[:-6] for line in inventory.splitlines() if line.endswith(': test')) == sorted(TESTS)
    for number, name in enumerate(TESTS, 1):
        result = run([str(executable), '--exact', name, '--nocapture'], f'test-{number}.log')
        assert 'running 1 test' in result and re.search(r'test result: ok\. 1 passed; 0 failed; 0 ignored;', result)
    hashes = {file.name: hashlib.sha256(file.read_bytes()).hexdigest() for file in output.iterdir() if file.is_file()}
    (output / 'hashes.json').write_text(json.dumps(hashes, indent=2))
    executable_sha = hashlib.sha256(executable.read_bytes()).hexdigest()
    budget()
    (output / 'result.json').write_text(json.dumps(dict(source=SOURCE, tests=3, passed=3, failed=0, ignored=0,
        executable_sha256=executable_sha, deadline=deadline,
        scope_verdict='pending common supervisor quiescence')))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
