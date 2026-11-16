"""One private C++ build and one public diagnostic read; no production change."""
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import signal
import subprocess
import sys
import time

from receipt import CASE, ENGINE, SOURCE, qualify_red

scratch, out = map(Path, sys.argv[1:3])
jobs, deadline = int(sys.argv[3]), float(sys.argv[4])
raw = out/'raw'
recipe = json.loads((out/'recipe.json').read_text())
source, build, tmp, home = [scratch/name for name in ['source', 'build', 'tmp', 'home']]
process = None
phase = 'staging'
started = time.monotonic()


def write(path, value):
    path.write_text(json.dumps(value, indent=2)+'\n')


def sha(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def group_exists(group):
    try:
        os.killpg(group, 0)
        return True
    except ProcessLookupError:
        return False


def finish_child():
    if process is None:
        return False
    residue = process.poll() is not None and group_exists(process.pid)
    if group_exists(process.pid):
        try:
            os.killpg(process.pid, signal.SIGTERM)
        except ProcessLookupError:
            pass
        end = time.monotonic()+3
        while group_exists(process.pid) and time.monotonic() < end:
            process.poll()
            if not group_exists(process.pid):
                break
            time.sleep(.05)
        if group_exists(process.pid):
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
    if process.poll() is None:
        process.wait(timeout=3)
    return residue


def pids():
    return {int(value) for value in (cgroup/'cgroup.procs').read_text().split()}


def run(argv, label, cap, allowed=(0,)):
    global process
    process = None
    before = pids()
    directory = raw/label
    directory.mkdir(parents=True, exist_ok=True)
    seconds = min(cap, deadline-time.monotonic()-10)
    if seconds <= 0:
        raise TimeoutError('global budget before '+label)
    write(directory/'argv.json', argv)
    begin = time.monotonic()
    timed_out = False
    with (directory/'stdout.log').open('wb') as log:
        try:
            process = subprocess.Popen(argv, stdout=log, stderr=subprocess.STDOUT,
                                       start_new_session=True)
            code = process.wait(timeout=seconds)
        except subprocess.TimeoutExpired:
            timed_out = True
            finish_child()
            code = process.returncode
        finally:
            residue = finish_child()
            settle = time.monotonic()+3
            remaining = pids()-before
            while remaining and time.monotonic() < settle:
                time.sleep(.05)
                remaining = pids()-before
            write(directory/'exit.json', {'code': process.returncode if process else None,
                  'timeout': timed_out, 'group_residue_after_leader': residue,
                  'remaining_phase_pids': sorted(remaining),
                  'waited': process is not None and process.poll() is not None,
                  'elapsed_s': time.monotonic()-begin})
    if timed_out or residue or remaining:
        raise RuntimeError(label+' timed out or left descendants')
    if code not in allowed:
        raise RuntimeError(label+' returned '+str(code))
    return code


def confined(argv, testing=False):
    cmd = ['bwrap', '--unshare-user', '--unshare-net', '--ro-bind', '/', '/',
           '--proc', '/proc', '--dev', '/dev', '--ro-bind', str(source), str(source)]
    if not testing:
        cmd += ['--bind', str(build), str(build)]
        extension = str(source/'extension/vector/build')
        cmd += ['--bind', extension, extension]
    cmd += ['--bind', str(tmp), str(tmp), '--bind', str(home), str(home),
            '--bind', str(out), str(out), '--chdir', str(tmp if testing else build)]
    return cmd+argv


try:
    assert recipe['source_commit'] == SOURCE and recipe['engine_commit'] == ENGINE
    assert 1 <= jobs <= 8 and not any(scratch.iterdir())
    cg = next(line.split(':', 2)[2] for line in Path('/proc/self/cgroup').read_text().splitlines()
              if line.startswith('0::'))
    cgroup = Path('/sys/fs/cgroup')/cg.lstrip('/')
    admission = json.loads((raw/'driver/admission.json').read_text())
    quota, period = (cgroup/'cpu.max').read_text().split()
    assert quota != 'max' and int(quota)/int(period) <= admission['child_cpu_quota_percent']/100+.00001
    assert jobs <= int(admission['poste_threads'])
    assert (cgroup/'memory.max').read_text().strip() == '34359738368'
    assert (cgroup/'memory.swap.max').read_text().strip() == '0'
    assert os.statvfs(scratch).f_blocks*os.statvfs(scratch).f_frsize == 34359738368
    assert len(Path('/proc/net/route').read_text().splitlines()) == 1
    for name in ['artifacts', 'parsing']:
        (raw/name).mkdir(parents=True, exist_ok=True)
    (out/'base').write_text(recipe['base']+'\n')
    (out/'resume.log').write_text('source '+SOURCE+'; engine '+ENGINE+'; RED DIAGNOSTIC ONLY\n')
    os.environ.clear()
    os.environ.update(PATH='/usr/bin:/bin', LANG='C', LC_ALL='C', TZ='UTC', HOME=str(home),
                      TMPDIR=str(tmp), RUNNER_TEMP=str(tmp), IN_MEM_MODE='false',
                      MAX_NUM_THREADS='2', OMP_NUM_THREADS=str(jobs),
                      OPENBLAS_NUM_THREADS=str(jobs), CCACHE_DISABLE='1',
                      CMAKE_BUILD_PARALLEL_LEVEL=str(jobs), GIT_OPTIONAL_LOCKS='0',
                      GIT_TERMINAL_PROMPT='0', GIT_ALLOW_PROTOCOL='file',
                      GIT_CONFIG_COUNT='2', GIT_CONFIG_KEY_0='gc.auto',
                      GIT_CONFIG_VALUE_0='0', GIT_CONFIG_KEY_1='maintenance.auto',
                      GIT_CONFIG_VALUE_1='false')
    for directory in [build, tmp, home]:
        directory.mkdir()
    assert sha(out/'source.bundle') == recipe['bundle_sha256']
    staging_end = min(deadline-10, time.monotonic()+60)
    run(['git', 'clone', '--no-checkout', str(out/'source.bundle'), str(source)],
        'staging/clone', staging_end-time.monotonic())
    run(['git', '-C', str(source), 'checkout', '--detach', SOURCE],
        'staging/checkout', staging_end-time.monotonic())
    run(['git', '-C', str(source), 'status', '--porcelain'], 'staging/status',
        staging_end-time.monotonic())
    assert not (raw/'staging/status/stdout.log').read_text().strip()
    run(['git', '-C', str(source), 'rev-parse', 'HEAD'], 'staging/head',
        staging_end-time.monotonic())
    assert (raw/'staging/head/stdout.log').read_text().strip() == SOURCE
    for name, digest in recipe['native_files'].items():
        assert sha(source/name) == digest
    (source/'extension/vector/build').mkdir(parents=True, exist_ok=True)
    phase = 'configure'
    config = ['cmake', '-S', str(source), '-B', str(build), '-G', 'Ninja',
              '-DCMAKE_BUILD_TYPE=Release', '-DBUILD_TESTS=ON', '-DBUILD_SHELL=OFF',
              '-DBUILD_EXTENSIONS=vector', '-DBUILD_SHARED_LIBS=OFF',
              '-DENABLE_RUNTIME_CHECKS=OFF', '-DENABLE_ADDRESS_SANITIZER=OFF',
              '-DENABLE_THREAD_SANITIZER=OFF', '-DENABLE_UBSAN=OFF', '-DENABLE_LTO=OFF',
              '-DENABLE_BACKTRACES=OFF', '-DCMAKE_EXPORT_COMPILE_COMMANDS=ON',
              '-DAUTO_UPDATE_GRAMMAR=OFF', '-DFETCHCONTENT_FULLY_DISCONNECTED=ON',
              '-DFETCHCONTENT_UPDATES_DISCONNECTED=ON', '-DCCACHE_PROGRAM:FILEPATH=']
    run(confined(config), 'configure', 120)
    phase = 'build'
    run(confined(['cmake', '--build', str(build), '--target',
                  'vector_index_diagnostics_test', 'rag3db_vector_extension', '-j', str(jobs)]),
        'build', 1800)
    phase = 'artifacts'
    files = {'test': build/'test/transaction/vector_index_diagnostics_test',
             'engine.a': build/'src/librag3db.a',
             'vector': source/'extension/vector/build/libvector.rag3db_extension',
             'build_id.h': build/'src/include/common/build_id.h',
             'CMakeCache.txt': build/'CMakeCache.txt',
             'compile_commands.json': build/'compile_commands.json'}
    inventory = {}
    for name, path in files.items():
        if time.monotonic() >= deadline-10:
            raise TimeoutError('artifact export')
        shutil.copy2(path, raw/'artifacts'/name)
        inventory[name] = {'sha256': sha(path), 'bytes': path.stat().st_size, 'path': str(path)}
    marker = re.search(r'RAG3DB_BUILD_ID "([^"]+)"', (files['build_id.h']).read_text())[1]
    assert marker.startswith('rag3db-build-'+ENGINE+'-') and '-dirty-' not in marker
    for name in ['test', 'vector']:
        assert marker.encode() in files[name].read_bytes()
    manifest = {'source': SOURCE, 'engine': ENGINE, 'build_id_header': marker, 'files': inventory}
    write(raw/'artifacts/manifest.json', manifest)
    # Tout artefact est exporté et identifié AVANT la seule invocation native.
    phase = 'red'
    code = run(confined([str(files['test']), '--gtest_filter='+CASE, '--gtest_color=no',
                        '--gtest_fail_fast', '--gtest_output=xml:'+str(raw/'red/result.xml')], True),
               'red', 30, allowed=(1,))
    result = qualify_red(raw/'red/result.xml', raw/'red/stdout.log', code, marker)
    manifest['native_exit'] = code
    write(raw/'artifacts/manifest.json', manifest)
    for name, path in files.items():
        assert sha(path) == inventory[name]['sha256']
    write(raw/'parsing/receipt.json', {'source': SOURCE, 'result': result,
                                     'other_three_cases': 'NOT RUN', 'retry': False})
except BaseException as error:
    finish_child()
    (raw/'parsing').mkdir(parents=True, exist_ok=True)
    write(raw/'parsing/first-failure.json', {'phase': phase, 'type': type(error).__name__,
          'message': str(error), 'elapsed_s': time.monotonic()-started, 'retry': False})
    raise
