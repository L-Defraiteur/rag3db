"""One fresh scope, two distinct native profiles; stop at the first red."""
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import time

import processes as proc
from receipt import CASES, PROFILE, inspect_case, verify_artifact_manifests

scratch, out, reference = map(Path, sys.argv[1:4])
jobs, deadline = int(sys.argv[4]), float(sys.argv[5])
recipe = json.loads((out / 'recipe.json').read_text())
raw = out / 'raw'
(raw / 'parsing').mkdir(parents=True, exist_ok=True)
phase = 'entry'
started = time.monotonic()
ledger = [{'variant': v['name'], 'case': c, 'status': 'NON JOUE'}
          for v in recipe['variants'] for c in CASES]


def write(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n')


def save_ledger():
    write(raw / 'parsing/ledger.json', ledger)


def sha(path):
    digest = hashlib.sha256()
    with path.open('rb') as stream:
        while True:
            if time.monotonic() >= deadline:
                raise TimeoutError('checksum deadline')
            block = stream.read(1024 * 1024)
            if not block:
                break
            digest.update(block)
    return digest.hexdigest()


def run(argv, label, cap, **kwargs):
    proc.raw, proc.deadline, proc.cgp = raw, deadline, cgp
    return proc.run(argv, label, cap, **kwargs)


def confined(argv, testing=False):
    command = ['bwrap', '--unshare-user', '--unshare-net', '--ro-bind', '/', '/',
               '--proc', '/proc', '--dev', '/dev', '--ro-bind', str(source), str(source),
               '--ro-bind' if testing else '--bind', str(build), str(build),
               '--bind', str(tmp), str(tmp), '--bind', str(home), str(home),
               '--bind', str(out), str(out), '--chdir', str(tmp if testing else build)]
    if testing:
        command += ['/usr/bin/env', '-i', *[key + '=' + value for key, value in test_env.items()]]
    return command + argv


save_ledger()
try:
    assert 1 <= jobs <= 8 and not any(scratch.iterdir())
    cg = next(x.split(':', 2)[2] for x in Path('/proc/self/cgroup').read_text().splitlines()
              if x.startswith('0::'))
    cgp = Path('/sys/fs/cgroup') / cg.lstrip('/')
    admission = json.loads((raw / 'driver/admission.json').read_text())
    quota, period = (cgp / 'cpu.max').read_text().split()
    limits = {'memory.max': (cgp / 'memory.max').read_text().strip(),
              'memory.swap.max': (cgp / 'memory.swap.max').read_text().strip(),
              'cpu.max': (cgp / 'cpu.max').read_text().strip(),
              'scratch_capacity_bytes': os.statvfs(scratch).f_blocks * os.statvfs(scratch).f_frsize,
              'routes': Path('/proc/net/route').read_text(), 'jobs': jobs}
    write(raw / 'driver/limits.json', limits)
    assert limits['memory.max'] == '34359738368' and limits['memory.swap.max'] == '0'
    assert limits['scratch_capacity_bytes'] == 34359738368
    assert len(limits['routes'].splitlines()) == 1
    assert quota != 'max' and jobs <= int(quota) // int(period)
    assert jobs <= int(admission['poste_threads'])
    assert int(quota) / int(period) <= admission['child_cpu_quota_percent'] / 100 + .00001
    assert sha(out / 'source.bundle') == recipe['bundle_sha256']
    assert (out / 'source.bundle').stat().st_size == recipe['bundle_bytes']
    for variant in recipe['variants']:
        name = variant['name']
        working = scratch / name
        working.mkdir()
        source, build, tmp, home = [working / x for x in ['source', 'build', 'tmp', 'home']]
        for path in [build, tmp, home]:
            path.mkdir()
        for folder in ['staging', 'configure', 'build', 'artifacts', *CASES]:
            (raw / name / folder).mkdir(parents=True, exist_ok=True)
        os.environ.update({'HOME': str(home), 'TMPDIR': str(tmp), 'GIT_OPTIONAL_LOCKS': '0',
                           'GIT_TERMINAL_PROMPT': '0', 'GIT_ALLOW_PROTOCOL': 'file',
                           'CCACHE_DISABLE': '1', 'CMAKE_BUILD_PARALLEL_LEVEL': str(jobs),
                           'GIT_CONFIG_COUNT': '2', 'GIT_CONFIG_KEY_0': 'gc.auto',
                           'GIT_CONFIG_VALUE_0': '0', 'GIT_CONFIG_KEY_1': 'maintenance.auto',
                           'GIT_CONFIG_VALUE_1': 'false'})
        phase = name + '/staging'
        run(['git', 'clone', '--shared', '--no-checkout', str(reference), str(source)], phase + '/clone', 30)
        run(['git', '-C', str(source), 'fetch', str(out / 'source.bundle'), variant['ref']], phase + '/fetch', 30)
        run(['git', '-C', str(source), 'checkout', '--detach', variant['source']], phase + '/checkout', 30)
        # No unbounded helper subprocess: every source check gets its own capped receipt.
        run(['git', '-C', str(source), 'rev-parse', 'HEAD'], phase + '/head', 5)
        actual = (raw / phase / 'head/stdout.log').read_text().strip()
        assert actual == variant['source']
        run(['git', '-C', str(source), 'status', '--porcelain'], phase + '/clean', 5)
        assert not (raw / phase / 'clean/stdout.log').read_bytes()
        engine_paths = ['src', 'extension', 'third_party', 'cmake', 'CMakeLists.txt',
                        ':(exclude)extension/rag3weaver', ':(exclude)extension/lucivy']
        run(['git', '-C', str(source), 'log', '-1', '--format=%H', '--', *engine_paths],
            phase + '/engine', 5)
        assert (raw / phase / 'engine/stdout.log').read_text().strip() == variant['engine']
        for item in recipe['test_files']:
            assert sha(source / item['path']) == item['sha256']
        assert sha(source / 'src/storage/table/dictionary_chunk.cpp') == variant['dictionary_sha256']
        test_env = {'PATH': '/usr/bin:/bin', 'LANG': 'C', 'LC_ALL': 'C', 'TZ': 'UTC',
                    'HOME': str(home), 'TMPDIR': str(tmp), 'RUNNER_TEMP': str(tmp),
                    'MAX_NUM_THREADS': '2', 'OMP_NUM_THREADS': str(jobs), 'OPENBLAS_NUM_THREADS': str(jobs)}
        write(raw / name / 'test-env.json', test_env)
        phase = name + '/configure'
        command = ['cmake', '-S', str(source), '-B', str(build), '-G', 'Ninja',
                   '-DCMAKE_BUILD_TYPE=Release', '-DBUILD_TESTS=ON', '-DBUILD_SHELL=OFF', '-DBUILD_SHARED_LIBS=OFF', '-DBUILD_EXTENSIONS=',
                   '-DENABLE_ADDRESS_SANITIZER=OFF', '-DENABLE_THREAD_SANITIZER=OFF', '-DSINGLE_THREADED=OFF', '-DENABLE_UBSAN=OFF',
                   '-DENABLE_RUNTIME_CHECKS=OFF', '-DENABLE_LTO=OFF', '-DENABLE_BACKTRACES=OFF',
                   '-DCMAKE_EXPORT_COMPILE_COMMANDS=ON', '-DAUTO_UPDATE_GRAMMAR=OFF',
                   '-DFETCHCONTENT_FULLY_DISCONNECTED=ON', '-DFETCHCONTENT_UPDATES_DISCONNECTED=ON',
                   '-DCCACHE_PROGRAM:FILEPATH=']
        run(confined(command), phase, 120)
        phase = name + '/build'
        run(confined(['cmake', '--build', str(build), '--target', 'concurrence_test', '-j', str(jobs)]), phase, 800)
        phase = name + '/artifacts'
        files = {'concurrence': build / 'test/transaction/concurrence/concurrence_test',
                 'engine.a': build / 'src/librag3db.a', 'build_id.h': build / 'src/include/common/build_id.h',
                 'CMakeCache.txt': build / 'CMakeCache.txt', 'compile_commands.json': build / 'compile_commands.json'}
        inventory = {}
        # Export every produced file before judging the header/ELF identity.
        for filename, path in files.items():
            if time.monotonic() >= deadline - 10:
                raise TimeoutError('artifact export')
            shutil.copy2(path, raw / name / 'artifacts' / filename)
            inventory[filename] = {'bytes': path.stat().st_size, 'sha256': sha(path)}
        marker = re.search(r'RAG3DB_BUILD_ID "([^"]+)"', files['build_id.h'].read_text())[1]
        write(raw / name / 'artifacts/manifest.json', {'source': actual, 'engine': variant['engine'],
                                                     'build_id': marker, 'files': inventory})
        assert marker == 'rag3db-build-' + variant['engine'] + PROFILE
        assert marker.encode() in files['concurrence'].read_bytes()
        for case, case_name in CASES.items():
            phase = name + '/' + case
            row = next(x for x in ledger if x['variant'] == name and x['case'] == case)
            row['status'] = 'STARTED'
            save_ledger()
            env = dict(test_env)
            if case == 'strings':
                env['RAG3DB_STRING_DIAGNOSTIC'] = '1'
            write(raw / phase / 'env.json', env)
            code = run(confined([str(files['concurrence']), '--gtest_filter=' + case_name,
                                 '--gtest_color=no', '--gtest_fail_fast',
                                 '--gtest_output=xml:' + str(raw / phase / 'result.xml')], True),
                       phase, 60, env=env, allowed=(0, 1))
            result = inspect_case(raw / phase, case, variant['engine'], code)
            write(raw / phase / 'receipt.json', result)
            row['status'] = result['verdict']
            save_ledger()
            # A complete expected red is evidence, never permission to play the next case.
            if result['verdict'] != 'VERT':
                raise RuntimeError('first native red: ' + phase)
        for filename, record in inventory.items():
            assert sha(files[filename]) == record['sha256']
        run(['git', '-C', str(source), 'status', '--porcelain'], name + '/staging/final-clean', 5)
        assert not (raw / name / 'staging/final-clean/stdout.log').read_bytes()
    verify_artifact_manifests(out, sha)
    write(raw / 'parsing/receipt.json', {'verdict': 'VERT', 'scope': 'four named diagnostic cases only',
                                      'ledger': ledger, 'C1_closed': False, 'installed': False})
except BaseException as exc:
    proc.finish_child()
    write(raw / 'parsing/first-failure.json', {'phase': phase, 'type': type(exc).__name__,
                                             'message': str(exc), 'elapsed_s': time.monotonic() - started,
                                             'retry': False})
    for row in ledger:
        if row['status'] == 'STARTED':
            row['status'] = 'REFUSED'
    save_ledger()
    raise
finally:
    write(raw / 'driver/scratch-final.json', {'capacity': os.statvfs(scratch).f_blocks * os.statvfs(scratch).f_frsize,
                                             'elapsed_s': time.monotonic() - started})
