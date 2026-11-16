"""Proposed nine memory judges. Native index-building test bodies are never selected."""
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

SOURCE = '1ee5b366ca67d7fe6926da232a6650d1b1d81407'
CORE_SHA = '2fa59faafe18634a055da4e770a96d4eb87da79864332f8d2a5d8ed222a8fa14'
COMMON_SHA = '6439fdd8b52c85fa4e74af0def2c667ef0bedae9b814cbbfd48e49946c4f74ea'
OLD_ELF_SHA = '7c6f65e2e8427b53828a58cecd313d4a6c02cd7a3074f358d14ce5f2114ea9e8'
TARGETS = ['test_derived_fixture_admission', 'test_derived_in_ram', 'test_sfx_v3_ground_truth']
TESTS = ['missing_panel_is_rejected_before_any_builder', 'loader_error_is_not_an_empty_corpus_or_no_hit', 'invalid_utf8_is_not_silently_replaced_or_dropped', 'strict_unicode_folding_maps_to_original_bytes', 'relaxed_spans_cross_separators_without_losing_offsets', 'underscore_is_a_boundary_but_a_content_suffix_is_not', 'regex_keeps_nonempty_original_byte_spans', 'fuzzy_uses_the_existing_occurrence_definition_on_admitted_bytes', 'oracle_decode_failure_is_distinct_from_zero_matches']

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
    assert authority['etat'] == 'termine' and authority['pid'] == 588652 and authority['debut'] == '119729720'
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
    assert sha(checked_path(str(package / 'red-consumer-elf'), kind='file')) == OLD_ELF_SHA
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
    preflight = checked_path(parser.parse_args().preflight, kind='file')
    preflight_sha256 = sha(preflight)
    data = check_preflight(str(preflight), scope_exists=True)
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
    assert 0 < deadline - time.monotonic() <= 285 and deadline <= bound['deadline']
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
        budget()
        with (output / name).open('x') as file:
            budget()  # Opening a file may block: refuse before writing any payload.
            json.dump(payload, file, indent=2)
            file.flush(); os.fsync(file.fileno())
        budget()

    def budget():
        live = json.loads(receipt_path.read_text())
        assert live['etat'] == 'actif' and live['echeance'] == deadline and deadline-time.monotonic() > 5

    publish('entry.json', dict(source=SOURCE, pilot_sha256=sha(Path(__file__)), nonce=data['nonce'],
            preflight_sha256=preflight_sha256,
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
    assert sorted(inventories[TARGETS[0]]['tests']) == sorted(TESTS)
    assert not inventories[TARGETS[0]]['ignored']
    # Complete inventory ledger BEFORE all nine memory judges. Native test bodies never run.
    publish('inventory-ledger.json', dict(source=SOURCE, observed=time.monotonic(), inventories=inventories))
    judges = []
    for number, name in enumerate(TESTS):
        log_name = f'judge-{number}.log'
        text = run([artifacts[TARGETS[0]]['path'], '--exact', name, '--test-threads=1', '--nocapture'], log_name)
        assert 'running 1 test' in text
        assert re.search(r'test result: ok\. 1 passed; 0 failed; 0 ignored; 0 measured; 8 filtered out;', text)
        row = dict(name=name, passed=1, failed=0, ignored=0, filtered_out=8, log_sha256=sha(output / log_name))
        judges.append(row)
        publish(f'judge-{number}.json', row)
    post = quiescence(data, 'post-witness')
    assert not list(scratch.iterdir()) and sha(target / 'validation-provenance.json') == data['target_creator_sha256']
    final_artifacts = {}
    for name, row in artifacts.items():
        budget()
        path = checked_path(row['path'], kind='file')
        digest = sha(path)
        assert digest == row['sha256'], f'ELF changed after inventory: {name}'
        final_artifacts[name] = dict(path=str(path), sha256=digest)
    assert subprocess.check_output(['git', '-C', str(root), 'rev-parse', 'HEAD'], text=True).strip() == SOURCE
    assert not subprocess.check_output(['git', '-C', str(root), 'status', '--porcelain'], text=True).strip()
    for relative, digest in data['copy_hashes'].items():
        budget()
        assert not Path(relative).is_absolute() and '..' not in Path(relative).parts
        assert sha(checked_path(str(package / relative), kind='file')) == digest, f'frozen copy changed: {relative}'
    assert sha(preflight) == preflight_sha256, 'preflight changed after admission'
    budget()
    publish('result.json', dict(source=SOURCE, tests=len(TESTS), passed=len(judges), failed=0, ignored=0,
            judges=judges, executable_sha256=final_artifacts[TARGETS[0]]['sha256'], final_artifacts=final_artifacts,
            final_source=SOURCE, source_clean=True, frozen_copies_rechecked=True,
            preflight_sha256=preflight_sha256, post=post, deadline=deadline,
            native_index_builders_executed=False, native_test_bodies_executed=False,
            memory_judges_executed=9, real_corpus_qualified=False, full_core_qualified=False,
            final_common_verdict='pending supervisor cleanup and external owned scan'))

    return 0


if __name__ == '__main__':
    raise SystemExit(main())
