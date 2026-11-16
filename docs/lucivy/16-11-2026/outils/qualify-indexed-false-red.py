"""Prepare one schema assertion red; execute only under the reviewed fresh scope."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import time

SOURCE = '0c795f99e5f49926a510971c521eb056a4880adc'
SUPERVISOR_SHA256 = '6439fdd8b52c85fa4e74af0def2c667ef0bedae9b814cbbfd48e49946c4f74ea'
TEST_TARGET = 'test_text_field_indexed_false'
TEST_NAME = 'unsupported_text_indexing_options_are_reported_once_per_field'


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--worktree', required=True)
    parser.add_argument('--target', required=True)
    parser.add_argument('--output', required=True)
    args = parser.parse_args()
    root = Path(args.worktree).resolve()
    target = Path(args.target).resolve()
    receipt_path = Path(os.environ['RAG3WEAVER_VALIDATION_RECU'])
    receipt = json.loads(receipt_path.read_text())
    deadline = receipt['echeance']
    assert receipt['etat'] == 'actif' and 0 < deadline - time.monotonic() <= 600
    assert receipt['script_sha256'] == SUPERVISOR_SHA256
    source = subprocess.check_output(['git', '-C', str(root), 'rev-parse', 'HEAD'], text=True).strip()
    assert source == SOURCE
    assert not subprocess.check_output(['git', '-C', str(root), 'status', '--porcelain'], text=True).strip()
    cgroup = Path('/proc/self/cgroup').read_text().split('0::', 1)[1].strip()
    cgroup_root = Path('/sys/fs/cgroup') / cgroup.lstrip('/')
    memory = {name: (cgroup_root / name).read_text().strip() for name in ['memory.max', 'memory.high']}
    assert memory == {'memory.max': '34359738368', 'memory.high': 'max'}, memory
    assert len(os.sched_getaffinity(0)) >= 6, 'four build jobs must leave two CPUs free'
    assert os.environ.get('LUCIVY_CORPUS_ABSENT_OK') != '1', 'corpus skipping is not admitted'
    output = Path(args.output).resolve()
    output.mkdir(parents=True, exist_ok=False)
    env = dict(os.environ, CARGO_TARGET_DIR=str(target), CARGO_BUILD_JOBS='4',
               CARGO_TERM_COLOR='never', CARGO_NET_OFFLINE='true')
    metadata = dict(source=source, cgroup=cgroup, memory=memory, deadline=deadline,
                    jobs=4, target=str(target), worktree=str(root), network='offline')
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

    command = ['cargo', 'test', '--offline', '--release', '-j4', '-p', 'lucivy-core',
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
            artifacts.append(Path(item['executable']).resolve())
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
