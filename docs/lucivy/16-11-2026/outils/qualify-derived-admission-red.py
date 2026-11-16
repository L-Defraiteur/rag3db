"""Proposed single consumer witness. No index-building test is ever selected."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import runpy
import stat
import subprocess
import time

SOURCE = '81b0e74f0254c8216864f90a460f8969842432ed'
CORE_SHA = '2fa59faafe18634a055da4e770a96d4eb87da79864332f8d2a5d8ed222a8fa14'
COMMON_SHA = '6439fdd8b52c85fa4e74af0def2c667ef0bedae9b814cbbfd48e49946c4f74ea'
OLD_ELF_SHA = '106fa398fcc24296c1d4b94198a150ed87917fac66e46531af5c617444fcf9a2'
TARGETS = ['test_derived_fixture_admission', 'test_derived_in_ram', 'test_sfx_v3_ground_truth']
WITNESS = 'missing_panel_is_rejected_before_any_builder'

helper = Path(__file__).absolute().with_name('qualify-indexed-false-core.py')
current = Path('/')
for part in helper.parts[1:]:
    current /= part
    assert not stat.S_ISLNK(current.lstat().st_mode), f'link refused: {current}'
assert helper.stat().st_uid == os.getuid() and hashlib.sha256(helper.read_bytes()).hexdigest() == CORE_SHA
SHARED = runpy.run_path(str(helper), run_name='shared_metadata_checks_only')
checked_path = SHARED['checked_path']
cpu_jobs = SHARED['cpu_jobs']


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def quiescence(data, phase):
    authority = json.loads(checked_path(data['old_receipt'], kind='file').read_text())
    verdict = json.loads(checked_path(data['old_verdict'], kind='file').read_text())
    records = SHARED['collect_scan'](data)
    assessment = SHARED['classify_scan'](records, data['baseline_unknown'])
    snapshot = SHARED['save_scan'](data, records, assessment, phase)
    assert authority['etat'] == 'termine' and authority['pid'] == 3528029 and authority['debut'] == '119072861'
    assert verdict['code'] == 101 and verdict['erreur'] is None and not verdict['restes'] and verdict['nettoye']
    assert assessment['admitted'], f'owned/attributed activity; partial evidence: {snapshot}'
    return dict(snapshot=str(snapshot), assessment=assessment, observed=time.monotonic())


def check_preflight(filename, scope_exists):
    manifest = checked_path(filename, kind='file')
    data = json.loads(manifest.read_text())
    package = checked_path(data['package'])
    assert manifest.parent == package and data['uid'] == os.getuid()
    assert package.stat().st_ino == data['package_inode'] and package.stat().st_dev == data['package_device']
    assert re.fullmatch('[0-9a-f]{32}', data['nonce'])
    root = checked_path(data['worktree'])
    target = checked_path(data['target'])
    assert target.stat().st_ino == 164536077 and target.stat().st_dev == 55
    creator_path = checked_path(str(target / 'validation-provenance.json'), kind='file')
    assert sha(creator_path) == '5a1671d22dc784bec78f60cf25c8b3003acc5f21e28290f05d0dfc22e0c394df'
    creator = json.loads(creator_path.read_text())
    assert creator['source'] == '0c795f99e5f49926a510971c521eb056a4880adc'
    assert creator['nonce'] == '3552903efab590b44f44cd8002f7b028' and creator['uid'] == os.getuid()
    old_elf = checked_path(data['old_elf'], kind='file')
    assert old_elf.is_relative_to(target) and sha(old_elf) == OLD_ELF_SHA
    assert sha(checked_path(str(package / 'red-core-elf'), kind='file')) == OLD_ELF_SHA
    checked_path(data['output'], exists=False)
    checked_path(data['scope'], exists=scope_exists)
    assert Path.cwd() == package, 'parent and pilot cwd must be the package, outside source/target'
    assert subprocess.check_output(['git', '-C', str(root), 'rev-parse', 'HEAD'], text=True).strip() == SOURCE
    assert not subprocess.check_output(['git', '-C', str(root), 'status', '--porcelain'], text=True).strip()
    for relative, digest in data['copy_hashes'].items():
        assert not Path(relative).is_absolute() and '..' not in Path(relative).parts
        assert sha(checked_path(str(package / relative), kind='file')) == digest
    assert stat.S_IMODE((package / 'support/poste').stat().st_mode) == 0o755
    quiescence(data, 'preflight')
    return data


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--preflight', required=True)
    data = check_preflight(parser.parse_args().preflight, scope_exists=True)
    package = Path(data['package'])
    output = Path(data['output'])
    root = Path(data['worktree'])
    target = Path(data['target'])
    receipt_path = checked_path(os.environ['RAG3WEAVER_VALIDATION_RECU'], kind='file')
    assert receipt_path.parents[1] == Path(data['scope'])
    receipt = json.loads(receipt_path.read_text())
    bound = json.loads((Path(data['scope']) / 'scope-provenance.json').read_text())
    deadline = receipt['echeance']
    assert receipt['etat'] == 'actif' and receipt['script_sha256'] == COMMON_SHA
    assert 0 < deadline - time.monotonic() <= 600 and deadline <= bound['deadline']
    assert bound['nonce'] == data['nonce']
    cgroup = Path('/proc/self/cgroup').read_text().split('0::', 1)[1].strip()
    cgroup_root = Path('/sys/fs/cgroup') / cgroup.lstrip('/')
    resources = {name: (cgroup_root / name).read_text().strip()
                 for name in ['memory.max', 'memory.high', 'memory.swap.max', 'cpu.max']}
    assert resources['memory.max'] == '34359738368' and resources['memory.high'] == 'max'
    assert resources['memory.swap.max'] == '0'
    quota, period = resources['cpu.max'].split()
    assert quota != 'max' and int(quota) / int(period) <= 4
    jobs, cpu = cpu_jobs(cgroup_root)
    assert 'LUCIVY_CORPUS_ABSENT_OK' not in os.environ
    assert not any(name in os.environ for name in
                   ['RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTDOCFLAGS', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER'])
    scratch = checked_path(data['scratch'])
    assert not list(scratch.iterdir())
    output.mkdir(exist_ok=False)
    env = dict(os.environ, CARGO_TARGET_DIR=str(target), CARGO_BUILD_JOBS=str(jobs),
               CARGO_TERM_COLOR='never', CARGO_NET_OFFLINE='true', TMPDIR=str(scratch))

    def publish(name, payload):
        with (output / name).open('x') as file:
            json.dump(payload, file, indent=2)
            file.flush(); os.fsync(file.fileno())

    def budget():
        live = json.loads(receipt_path.read_text())
        assert live['etat'] == 'actif' and live['echeance'] == deadline and deadline-time.monotonic() > 5

    publish('entry.json', dict(source=SOURCE, pilot_sha256=sha(Path(__file__)), nonce=data['nonce'],
            resources=resources, jobs=jobs, cpu=cpu, cgroup=cgroup, deadline=deadline,
            quiescence=quiescence(data, 'entry'), index_building_tests_selected=False))

    def run(argv, name, expected=0):
        budget()
        with (output / name).open('x') as file:
            child = subprocess.Popen(argv, cwd=root, env=env, stdout=file, stderr=subprocess.STDOUT)
            start = Path(f'/proc/{child.pid}/stat').read_text().rsplit(') ', 1)[1].split()[19]
            publish(name + '.identity.json', dict(pid=child.pid, start=start, argv=argv))
            code = child.wait()
            file.flush(); os.fsync(file.fileno())
        budget()
        if code != expected:
            publish('first-unexpected-red.json', dict(phase=name, code=code, expected=expected, source=SOURCE))
            raise SystemExit(code if code > 0 else 1)
        return (output / name).read_text()

    argv = ['cargo', 'test', '--offline', '--release', f'-j{jobs}', '-p', 'lucivy-core']
    for name in TARGETS:
        argv.extend(['--test', name])
    log = run(argv + ['--no-run', '--message-format=json-render-diagnostics'], 'compile.jsonl')
    artifacts = {}
    for line in log.splitlines():
        try:
            row = json.loads(line)
        except ValueError:
            continue
        if row.get('reason') == 'compiler-artifact' and row.get('profile', {}).get('test') and row.get('executable'):
            path = checked_path(row['executable'], kind='file')
            assert path.is_relative_to(target)
            artifacts[row['target']['name']] = dict(path=str(path), sha256=sha(path))
    assert sorted(artifacts) == sorted(TARGETS)
    # Durable ELF ledger BEFORE any inventory process or test process.
    publish('elf-ledger.json', dict(source=SOURCE, observed=time.monotonic(), artifacts=artifacts,
                                  native_builders_executed=False))
    inventories = {}
    for number, name in enumerate(TARGETS):
        path = artifacts[name]['path']
        listing = run([path, '--list'], f'{number}-inventory.log')
        ignored = run([path, '--list', '--ignored'], f'{number}-ignored.log')
        inventories[name] = dict(tests=[line[:-6] for line in listing.splitlines() if line.endswith(': test')],
                                ignored=[line[:-6] for line in ignored.splitlines() if line.endswith(': test')])
    assert len(inventories[TARGETS[0]]['tests']) == 9 and WITNESS in inventories[TARGETS[0]]['tests']
    assert not inventories[TARGETS[0]]['ignored']
    # Complete inventory ledger BEFORE the single mock test. Native test bodies never run.
    publish('inventory-ledger.json', dict(source=SOURCE, observed=time.monotonic(), inventories=inventories))
    text = run([artifacts[TARGETS[0]]['path'], '--exact', WITNESS, '--test-threads=1', '--nocapture'],
               'consumer-witness.log', expected=101)
    assert 'running 1 test' in text and 'an uncovered panel must be refused before the builder' in text
    assert 'left: 1' in text and 'right: 0' in text
    assert re.search(r'test result: FAILED\. 0 passed; 1 failed; 0 ignored;', text)
    post = quiescence(data, 'post-witness')
    assert not list(scratch.iterdir()) and sha(target / 'validation-provenance.json') == data['target_creator_sha256']
    budget()
    publish('expected-red.json', dict(source=SOURCE, witness=WITNESS, code=101, passed=0, failed=1, ignored=0,
            explanation='baseline consumer calls mock builder once despite uncovered panel; correction not implemented',
            executable_sha256=artifacts[TARGETS[0]]['sha256'], post=post, deadline=deadline,
            native_index_builders_executed=False, other_mock_tests_executed=False,
            full_core_qualified=False, final_common_verdict='pending supervisor cleanup and external owned scan'))
    return 101


if __name__ == '__main__':
    raise SystemExit(main())
