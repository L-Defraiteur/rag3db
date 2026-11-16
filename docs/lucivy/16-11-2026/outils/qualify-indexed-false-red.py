"""Prepare one schema assertion red; execute only under the reviewed fresh scope."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import stat
import time

SOURCE = '0c795f99e5f49926a510971c521eb056a4880adc'
SUPERVISOR_SHA256 = '6439fdd8b52c85fa4e74af0def2c667ef0bedae9b814cbbfd48e49946c4f74ea'
TEST_TARGET = 'test_text_field_indexed_false'
TEST_NAME = 'unsupported_text_indexing_options_are_reported_once_per_field'


def checked_path(value, exists=True, kind='dir'):
    path = Path(value)
    assert path.is_absolute() and '..' not in path.parts and str(path) == os.path.normpath(value)
    current = Path('/')
    for index, part in enumerate(path.parts[1:]):
        current /= part
        last = index == len(path.parts) - 2
        try:
            info = current.lstat()
        except FileNotFoundError:
            assert last and not exists, f'missing parent or required path: {current}'
            return path
        assert not stat.S_ISLNK(info.st_mode), f'symlink refused: {current}'
        if not last:
            assert stat.S_ISDIR(info.st_mode)
        else:
            assert exists, f'path must be new: {path}'
            assert info.st_uid == os.getuid(), f'wrong owner: {path}'
            assert (stat.S_ISDIR(info.st_mode) if kind == 'dir' else stat.S_ISREG(info.st_mode))
    return path


def check_preflight(filename, scope_exists):
    manifest = checked_path(filename, kind='file')
    data = json.loads(manifest.read_text())
    package = checked_path(data['package'])
    assert manifest.parent == package and data['uid'] == os.getuid() and data['target_absent'] is True
    assert package.stat().st_ino == data['package_inode'] and package.stat().st_dev == data['package_device']
    assert re.fullmatch(r'[0-9a-f]{32}', data['nonce'])
    checked_path(data['worktree'])
    checked_path(data['target'], exists=False)
    checked_path(data['output'], exists=False)
    checked_path(data['scope'], exists=scope_exists)
    for relative, expected in data['copy_hashes'].items():
        assert not Path(relative).is_absolute() and '..' not in Path(relative).parts
        copy = checked_path(str(package / relative), kind='file')
        assert hashlib.sha256(copy.read_bytes()).hexdigest() == expected, f'copy hash mismatch: {relative}'
    poste = package / 'support/poste'
    assert stat.S_IMODE(poste.stat().st_mode) == 0o755
    return data


def cpu_jobs(cgroup_root):
    limits = []
    path = cgroup_root
    boundary = Path('/sys/fs/cgroup')
    quota_jobs = 4
    while True:
        quota, period = (path / 'cpu.max').read_text().split()
        limits.append(dict(path=str(path), quota=quota, period=period))
        if quota != 'max':
            quota_jobs = min(quota_jobs, max(1, int(quota) // int(period)))
        if path == boundary:
            break
        path = path.parent
    allowed = sorted(os.sched_getaffinity(0))
    assert len(allowed) >= 3, 'at least two affinity CPUs must remain free'
    return min(4, len(allowed) - 2, quota_jobs), dict(allowed=allowed, hierarchy=limits)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--worktree', required=True)
    parser.add_argument('--target', required=True)
    parser.add_argument('--output', required=True)
    parser.add_argument('--preflight', required=True)
    args = parser.parse_args()
    admission = check_preflight(args.preflight, scope_exists=True)
    assert args.worktree == admission['worktree'] and args.target == admission['target'] and args.output == admission['output']
    root = checked_path(args.worktree)
    target = checked_path(args.target, exists=False)
    receipt_path = Path(os.environ['RAG3WEAVER_VALIDATION_RECU'])
    checked_path(str(receipt_path), kind='file')
    assert receipt_path.parents[1] == Path(admission['scope'])
    receipt = json.loads(receipt_path.read_text())
    scope_record = json.loads((Path(admission['scope']) / 'scope-provenance.json').read_text())
    assert scope_record['nonce'] == admission['nonce']
    deadline = receipt['echeance']
    assert receipt['etat'] == 'actif' and 0 < deadline - time.monotonic() <= 600
    assert deadline <= scope_record['deadline']
    assert receipt['script_sha256'] == SUPERVISOR_SHA256
    source = subprocess.check_output(['git', '-C', str(root), 'rev-parse', 'HEAD'], text=True).strip()
    assert source == SOURCE
    assert not subprocess.check_output(['git', '-C', str(root), 'status', '--porcelain'], text=True).strip()
    cgroup = Path('/proc/self/cgroup').read_text().split('0::', 1)[1].strip()
    cgroup_root = Path('/sys/fs/cgroup') / cgroup.lstrip('/')
    memory = {name: (cgroup_root / name).read_text().strip() for name in ['memory.max', 'memory.high']}
    assert memory == {'memory.max': '34359738368', 'memory.high': 'max'}, memory
    jobs, cpu = cpu_jobs(cgroup_root)
    assert os.environ.get('LUCIVY_CORPUS_ABSENT_OK') != '1', 'corpus skipping is not admitted'
    output = checked_path(args.output, exists=False)
    output.mkdir(exist_ok=False)
    target.mkdir(exist_ok=False)
    provenance = dict(uid=os.getuid(), nonce=admission['nonce'], source=source,
                      target=str(target), initial_state='absent', package=admission['package'])
    with (target / 'validation-provenance.json').open('x') as file:
        json.dump(provenance, file, indent=2)
        file.flush(); os.fsync(file.fileno())
    env = dict(os.environ, CARGO_TARGET_DIR=str(target), CARGO_BUILD_JOBS=str(jobs),
               CARGO_TERM_COLOR='never', CARGO_NET_OFFLINE='true')
    metadata = dict(source=source, cgroup=cgroup, memory=memory, deadline=deadline,
                    jobs=jobs, cpu=cpu, nonce=admission['nonce'], target=str(target), worktree=str(root), network='offline')
    (output / 'entry.json').write_text(json.dumps(metadata, indent=2))

    def budget():
        live = json.loads(receipt_path.read_text())
        assert live['etat'] == 'actif' and live['echeance'] == deadline
        assert deadline - time.monotonic() > 5, 'global deadline, no reset'

    def finish(kind, code, **details):
        result = dict(kind=kind, source=source, phase_exit_code=code, deadline=deadline, **details)
        with (output / 'first-red.json').open('x') as file:
            json.dump(result, file, indent=2)
            file.flush()
            os.fsync(file.fileno())
        manifest = {file.name: hashlib.sha256(file.read_bytes()).hexdigest()
                    for file in output.iterdir() if file.is_file()}
        (output / 'hashes.json').write_text(json.dumps(manifest, indent=2))
        budget()
        print(json.dumps(result), flush=True)
        # Even an expected TDD red remains RED in the common supervisor.
        return 1

    def run(argv, log):
        budget()
        with (output / log).open('w') as file:
            child = subprocess.Popen(argv, cwd=root, env=env, stdout=file, stderr=subprocess.STDOUT)
            start = Path(f'/proc/{child.pid}/stat').read_text().rsplit(') ', 1)[1].split()[19]
            (output / (log + '.identity.json')).write_text(json.dumps(dict(pid=child.pid, start=start, argv=argv)))
            code = child.wait()
            file.flush()
            os.fsync(file.fileno())
        budget()
        return code, (output / log).read_text()

    command = ['cargo', 'test', '--offline', '--release', f'-j{jobs}', '-p', 'lucivy-core',
               '--test', TEST_TARGET, '--no-run', '--message-format=json-render-diagnostics']
    code, log = run(command, 'compile.jsonl')
    if code:
        return finish('compilation_error_not_tdd_red', code)
    artifacts = []
    for line in log.splitlines():
        try:
            item = json.loads(line)
        except ValueError:
            continue
        if item.get('reason') == 'compiler-artifact' and item.get('target', {}).get('name') == TEST_TARGET and item.get('executable'):
            artifacts.append(checked_path(item['executable'], kind='file'))
    if len(artifacts) != 1 or not artifacts[0].is_relative_to(target):
        return finish('executable_inventory_error', 2)
    executable = artifacts[0]
    exe_sha = hashlib.sha256(executable.read_bytes()).hexdigest()
    code, inventory = run([str(executable), '--list', '--exact', TEST_NAME], 'inventory.log')
    if code or inventory.splitlines().count(TEST_NAME + ': test') != 1:
        return finish('test_inventory_error', code or 2, executable_sha256=exe_sha)
    code, log = run([str(executable), '--exact', TEST_NAME, '--nocapture'], 'test.log')
    expected = (code == 101 and 'running 1 test' in log and 'left: []' in log
                and 'indexed=false is not supported' in log and 'body' in log and 'label' in log
                and re.search(r'test result: FAILED\. 0 passed; 1 failed; 0 ignored;', log))
    if expected:
        return finish('expected_assertion_red_observed', code, executable_sha256=exe_sha, tests=1, ignored=0)
    return finish('unexpected_result_no_fix_authorized', code, executable_sha256=exe_sha)


if __name__ == '__main__':
    raise SystemExit(main())
