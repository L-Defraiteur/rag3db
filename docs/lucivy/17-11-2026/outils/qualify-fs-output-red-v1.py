"""FS output/identity RED pilot (five negatives and one control). Preparation is not authorization to execute."""
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
import tomllib

SOURCE = 'bdf83ad76cf73a74cdde8358affb364b857e27e2'
CORE_SHA = '2fa59faafe18634a055da4e770a96d4eb87da79864332f8d2a5d8ed222a8fa14'
COMMON_SHA = '6439fdd8b52c85fa4e74af0def2c667ef0bedae9b814cbbfd48e49946c4f74ea'
LOCK_SHA = 'dbdb560202562f17536624472a0e730627df163f4bb00c53b2baf604e77637f6'
TARGETS = ['test_fs_fixture_admission', 'test_sfx_v3_ground_truth']
NEGATIVES = [
    ('report_receipt_collision_is_refused_before_consumer', 'Collision', 'unsafe declared outputs accepted before consumer'),
    ('report_inside_input_is_refused_before_consumer', 'InsideInput', 'unsafe declared outputs accepted before consumer'),
    ('aliased_output_parent_is_refused_before_consumer', 'AliasParent', 'unsafe declared outputs accepted before consumer'),
    ('missing_document_ordinal_is_not_document_zero', 'Ok(0)', 'missing identity was admitted as document zero'),
    ('invalid_flags_have_no_receipt_or_corpus_effects', 'Err(Flags', 'invalid flags caused an output or corpus effect'),
]
POSITIVE = 'distinct_outputs_and_present_ordinal_reach_consumer_once'
TESTS = [r[0] for r in NEGATIVES] + [POSITIVE]
# Full compiled inventory is separate from the six selected judges.
INVENTORY = [
    'empty_view_is_refused_before_builder',
    'enumeration_error_is_not_an_empty_subtree',
    'metadata_error_is_not_a_filtered_file',
    'read_error_is_not_a_filtered_file',
    'invalid_utf8_is_refused_not_dropped',
    'uncovered_panel_is_refused_before_zero_equals_zero',
    'covered_fixture_reaches_builder_once_on_the_captured_bytes',
    'flag_presence_is_refused_before_any_corpus_operation',
    'absent_root_is_a_precise_refusal_without_skip',
    'existing_receipt_prevents_recapture_after_refusal',
] + TESTS
OLD_ELFS = {
    'test_fs_fixture_admission-9128a97001eece0d': '0801ea5ecc0d02844edff7ef032c2aafff6a25ad4dab712b4f2d65106c23bea9',
    'test_sfx_v3_ground_truth-71572a69e410ce61': 'ee1965d705b1e41e85fbf7645d856793a235b9024686a41af0c95685d7890a04',
    'test_derived_fixture_admission-0bd690ba36035513': '0a61a812a09a4adaead13cab8f5344965c390144bba1c6508804bff595525961',
    'test_derived_in_ram-539df445d54ae12e': '78fef16a51b354336989c4031f1032f14cd4883fab1ffbcc0762a382f7e17f17',
    'test_sfx_v3_ground_truth-32bb5ee824ccaa93': '917928a151c81a22324ca93eeb007a9aa17338781c7d2131c407f8d4fd5f14d4',
}

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
    # enfants.json lists descendants, not the supervisor itself. Check its
    # exact old identity too; inaccessible owned metadata must block admission.
    old_common = dict(pid=2593434, expected_start='127640555', same_live_identity=None)
    try:
        fields = Path('/proc/2593434/stat').read_text().rsplit(') ', 1)[1].split()
        old_common.update(observed_start=fields[19], state=fields[0],
            same_live_identity=fields[19] == '127640555' and fields[0] != 'Z')
    except (FileNotFoundError, ProcessLookupError):
        old_common['same_live_identity'] = False
    except OSError as error:
        old_common['error'] = dict(errno=error.errno, kind=type(error).__name__)
    assessment['old_common_identity'] = old_common
    if old_common['same_live_identity'] is not False:
        assessment['admitted'] = False
        assessment['blocked'].append(dict(reason='old_common_active_or_unknown', record=old_common))
    snapshot = SHARED['save_scan'](data, records, assessment, phase)
    assert authority['etat'] == 'termine' and authority['pid'] == 2593434 and authority['debut'] == '127640555'
    assert verdict['code'] == 0 and verdict['erreur'] is None and not verdict['restes'] and verdict['nettoye']
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
    assert sha(checked_path(str(package / 'Cargo.lock.seed'), kind='file')) == LOCK_SHA
    checked_path(data['output'], exists=False)
    checked_path(data['scope'], exists=scope_exists)
    assert Path.cwd() == package, 'parent and pilot cwd must be the package, outside source/target'
    assert subprocess.check_output(['git', '-C', str(root), 'rev-parse', 'HEAD'], text=True).strip() == SOURCE
    assert not subprocess.check_output(['git', '-C', str(root), 'status', '--porcelain'], text=True).strip()
    for relative, digest in data['copy_hashes'].items():
        assert not Path(relative).is_absolute() and '..' not in Path(relative).parts
        assert sha(checked_path(str(package / relative), kind='file')) == digest
    assert stat.S_IMODE((package / 'support/poste').stat().st_mode) == 0o755
    assert data['source'] == SOURCE
    assert sha(checked_path(str(root / 'Cargo.lock'), kind='file')) == LOCK_SHA
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
    assert 0 < deadline - time.monotonic() <= 285 and deadline <= bound['deadline'] - 8
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
    assert not any(name.startswith('V3_') for name in os.environ)
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
        if name == 'elf-ledger.json':
            print(json.dumps({'ELF_LEDGER_BEFORE_EXECUTION': payload}), flush=True)

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
            try:
                start = Path(f'/proc/{child.pid}/stat').read_text().rsplit(') ', 1)[1].split()[19]
            except FileNotFoundError:
                start = None  # Very short child already exited; Common still owns cleanup.
            publish(name + '.identity.json', dict(pid=child.pid, start=start, argv=argv))
            code = child.wait(timeout=max(1, deadline - time.monotonic() - 5))
            file.flush(); os.fsync(file.fileno())
        budget()
        if code != expected:
            publish('first-unexpected-red.json', dict(phase=name, code=code, expected=expected, source=SOURCE))
            raise SystemExit(code if code > 0 else 1)
        return (output / name).read_text()

    backups = package / 'prior-elf-readonly'
    backups.mkdir(exist_ok=False)
    for name, digest in OLD_ELFS.items():
        budget()
        original = checked_path(str(target / 'release/deps' / name), kind='file')
        assert sha(original) == digest
        content = original.read_bytes()
        assert hashlib.sha256(content).hexdigest() == digest
        with (backups / name).open('xb') as file:
            budget()
            file.write(content); file.flush(); os.fsync(file.fileno())
        (backups / name).chmod(0o444)
        assert sha(backups / name) == digest
    publish('prior-elf-backups.json', dict(files=OLD_ELFS, sources=['1ee5b366ca67d7fe6926da232a6650d1b1d81407', '0f610f437a341f745e5936492114142ff3afedd4'], executed=False))

    argv = ['cargo', 'test', '--offline', '--locked', '--release', f'-j{jobs}', '-p', 'lucivy-core']
    for name in TARGETS:
        argv.extend(['--test', name])
    log = run(argv + ['--no-run', '--message-format=json-render-diagnostics'], 'compile.jsonl')
    seed = tomllib.loads((package / 'Cargo.lock.seed').read_text())
    after_bytes = (root / 'Cargo.lock').read_bytes()
    publish('lock-after.json', dict(sha256=hashlib.sha256(after_bytes).hexdigest(),
            text=after_bytes.decode(), observed=time.monotonic()))
    after = tomllib.loads(after_bytes.decode())
    core = [p for p in seed['package'] if p['name'] == 'lucivy-core']
    assert len(core) == 1 and 'sha2' in core[0]['dependencies']
    assert after == seed and hashlib.sha256(after_bytes).hexdigest() == LOCK_SHA, 'locked seed changed'
    publish('lock-delta.json', dict(before=LOCK_SHA, after=sha(root / 'Cargo.lock'),
            sole_change=None, lock_invariant=True, exact_versions_unchanged=True))
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
    # External exporter must durably copy this exact ledger before any ELF runs.
    ledger_digest = sha(output / 'elf-ledger.json')
    publish('elf-ledger-ready.json', dict(sha256=ledger_digest, nonce=data['nonce']))
    ack_path = package / 'elf-ledger-export-ack.json'
    while not ack_path.exists():
        budget()
        time.sleep(0.1)
    ack = json.loads(checked_path(str(ack_path), kind='file').read_text())
    assert ack == dict(source=SOURCE, nonce=data['nonce'], sha256=ledger_digest,
                       copied_and_fsynced_on_portable=True)
    publish('elf-export-ack-verified.json', ack)
    inventories = {}
    for number, name in enumerate(TARGETS):
        path = artifacts[name]['path']
        listing = run([path, '--list'], f'{number}-inventory.log')
        ignored = run([path, '--list', '--ignored'], f'{number}-ignored.log')
        inventories[name] = dict(tests=[line[:-6] for line in listing.splitlines() if line.endswith(': test')],
                                ignored=[line[:-6] for line in ignored.splitlines() if line.endswith(': test')])
    assert sorted(inventories[TARGETS[0]]['tests']) == sorted(INVENTORY)
    assert not inventories[TARGETS[0]]['ignored']
    # Complete inventory BEFORE six real native Rust FS judges. No second-ELF body runs.
    publish('inventory-ledger.json', dict(source=SOURCE, observed=time.monotonic(), inventories=inventories))
    assert 'v3_ground_truth_contains' in inventories[TARGETS[1]]['tests']
    judges = []
    for number, name in enumerate(TESTS):
        log_name = f'judge-{number}.log'
        negative = number < len(NEGATIVES)
        text = run([artifacts[TARGETS[0]]['path'], '--exact', name, '--test-threads=1', '--nocapture'],
                   log_name, expected=101 if negative else 0)
        assert 'running 1 test' in text
        if negative:
            _, marker, assertion = NEGATIVES[number]
            assert f'test {name} ...' in text and 'FAILED' in text
            assert re.search(r'test result: FAILED\. 0 passed; 1 failed; 0 ignored; 0 measured; 15 filtered out;', text)
            assert marker in text and assertion in text
            if number < 3:
                assert f'output_fault={marker} expected_refusal=true result=Ok(Ok(1)) builds=1 events=' in text
                assert 'Builder, Report' in text and 'Enumerate(' in text and 'Read(' in text
                actual_manifest = re.search(r'manifest=Some\((\[[0-9, ]+\])\) report=Some\((\[[0-9, ]+\])\)', text)
                assert actual_manifest, 'missing actual receipt/report byte observations'
                manifest_bytes, report_bytes = [bytes(json.loads(value)) for value in actual_manifest.groups()]
                assert report_bytes == b'index-report documents=1\n'
                if number == 0:
                    assert manifest_bytes == report_bytes, 'collision did not actually truncate the receipt'
                else:
                    envelope = json.loads(manifest_bytes)
                    assert envelope['body']['verdict'] == 'ADMITTED'
                if number == 2:
                    assert 'alias-to-input' in text
            elif number == 3:
                assert 'document_ordinal(None,1)=Ok(0)' in text
            else:
                assert 'invalid_flags result=Err(Flags { names: ["V3_INDEX_DIR"] }) events=[Manifest(' in text
                assert not any(event in text for event in ['Enumerate(', 'Metadata(', 'Read(', 'Builder', 'Report'])
                actual_receipt = re.search(r'receipt=Some\((\[[0-9, ]+\])\)', text)
                assert actual_receipt
                envelope = json.loads(bytes(json.loads(actual_receipt.group(1))))
                assert envelope['body']['verdict'] == 'REFUSED'
                assert envelope['body']['refusal'] == {'Flags': {'names': ['V3_INDEX_DIR']}}
            cause = dict(expected_refusal=['Plan.output_collision', 'Plan.report_inside_input',
                'Plan.aliased_output_parent', 'OrdinalError.Missing', 'Flags.without_any_effect'][number],
                actual=marker, assertion=assertion, infrastructure_error=False)
        else:
            assert re.search(r'test result: ok\. 1 passed; 0 failed; 0 ignored; 0 measured; 15 filtered out;', text)
            assert 'expected_refusal=false result=Ok(Ok(1)) builds=1 events=' in text
            assert 'ADMISSION_RECEIPT ' in text and '"verdict":"ADMITTED"' in text
            cause = dict(positive=True, distinct_outputs=True, builder_calls=1, present_ordinal=0)
        row = dict(name=name, expected_semantic_red=negative, passed=int(not negative),
                   failed=int(negative), ignored=0, filtered_out=15,
                   cause=cause, log_sha256=sha(output / log_name))
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
    for name, digest in OLD_ELFS.items():
        budget()
        assert sha(backups / name) == digest
    budget()
    final_lock_sha256 = sha(checked_path(str(root / 'Cargo.lock'), kind='file'))
    assert final_lock_sha256 == LOCK_SHA, 'effective Cargo.lock changed after the six judges'
    budget()
    publish('result.json', dict(final_lock_sha256=final_lock_sha256, lock_invariant=True,
            source=SOURCE, tests=len(TESTS), passed=1, failed=5, ignored=0,
            judges=judges, executable_sha256=final_artifacts[TARGETS[0]]['sha256'], final_artifacts=final_artifacts,
            final_source=SOURCE, source_clean=True, frozen_copies_rechecked=True,
            preflight_sha256=preflight_sha256, post=post, deadline=deadline,
            native_index_builders_executed=False, second_elf_test_bodies_executed=False,
            native_fs_judges_executed=True,
            memory_judges_executed=0, fs_fixture_judges_executed=6,
            semantic_reds_observed=5, blanket_refusal_excluded=True, real_corpus_qualified=False, full_core_qualified=False,
            final_common_verdict='pending supervisor cleanup and external owned scan'))

    return 0  # Expected semantic reds are recorded; infrastructure errors never become success.


if __name__ == '__main__':
    raise SystemExit(main())
