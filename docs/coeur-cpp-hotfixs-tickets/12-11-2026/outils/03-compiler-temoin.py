"""Compilation/lien seuls, dans le cgroup privé déjà borné par le lanceur."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import threading
import time

root = Path(__file__).resolve().parent
recipe = json.loads((root / 'recipe.json').read_text())
report = {'status': 'started', 'stages': [], 'ready_to_run': False}
started = time.monotonic()


def digest(path):
    h = hashlib.sha256()
    with Path(path).open('rb') as stream:
        for block in iter(lambda: stream.read(1 << 20), b''):
            h.update(block)
    return h.hexdigest()


def same(path, expected):
    path = Path(path)
    assert path.is_file() and not path.is_symlink(), str(path)
    assert digest(path) == expected['sha256'], 'SHA mismatch: ' + str(path)
    if 'bytes' in expected:
        assert path.stat().st_size == expected['bytes'], str(path)


def command(name, argv, seconds):
    remaining = 230 - (time.monotonic() - started)
    assert remaining > 0, 'global deadline'
    entry = {'name': name, 'argv': argv}
    report['stages'].append(entry)
    proc = subprocess.Popen(argv, cwd=root, start_new_session=True,
                            stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    entry['pid'] = proc.pid
    counts = [0]

    def drain():
        with (root / (name + '.log')).open('xb') as out:
            for block in iter(lambda: proc.stdout.read(65536), b''):
                room = max(0, (2 << 20) - counts[0])
                out.write(block[:room])
                counts[0] += len(block)

    reader = threading.Thread(target=drain)
    reader.start()
    try:
        entry['exit_code'] = proc.wait(timeout=min(seconds, remaining))
    except subprocess.TimeoutExpired:
        os.killpg(proc.pid, signal.SIGKILL)
        proc.wait()
        entry['timeout'] = True
        raise RuntimeError(name + ': timeout')
    finally:
        reader.join(timeout=2)
        if reader.is_alive():
            os.killpg(proc.pid, signal.SIGKILL)
            reader.join(timeout=2)
        entry['output_bytes'] = counts[0]
    assert entry['exit_code'] == 0, name + ': command failed'
    assert counts[0] <= 2 << 20, name + ': output limit'


try:
    # Pas de compilation si le cgroup ne porte pas les limites promises.
    membership = Path('/proc/self/cgroup').read_text()
    rel = next(line.split(':', 2)[2] for line in membership.splitlines() if line.startswith('0::'))
    cg = Path('/sys/fs/cgroup') / rel.lstrip('/')
    report['cgroup'] = {'path': str(cg), 'memory.max': (cg / 'memory.max').read_text().strip(),
                        'memory.swap.max': (cg / 'memory.swap.max').read_text().strip()}
    assert report['cgroup']['memory.max'] == '1073741824', 'memory.max'
    assert report['cgroup']['memory.swap.max'] == '0', 'memory.swap.max'
    same(root / 'recipe.json', {'sha256': sys.argv[1]})
    manifest_path = Path(recipe['accepted_manifest']['path'])
    same(manifest_path, recipe['accepted_manifest'])
    manifest = json.loads(manifest_path.read_text())
    package = Path(recipe['required_archive']).parent
    assert manifest['source_commit'] == recipe['accepted_manifest']['source_commit']
    assert manifest['configuration_identity'] == recipe['accepted_manifest']['configuration_identity']
    assert manifest['historical_archive'] is False
    for path, expected in manifest['files'].items():
        same(package / path, expected)
    report['package_files_rehashed'] = len(manifest['files'])
    for path, expected in recipe['dependency_archives'].items():
        same(path, expected)
    same(recipe['required_archive'], {'sha256': recipe['accepted_manifest']['archive_sha256']})
    same(recipe['flags_reference']['path'], recipe['flags_reference'])
    same(root / 'quarantine_released_diagnostic.cpp', recipe['diagnostic_source'])
    same(root / 'test-headers/transaction/flaky_checkpointer.h', recipe['test_header'])
    source_root = Path(recipe['required_headers']['third_party_checkout']).parent
    assert subprocess.check_output(['git', '-C', str(source_root), 'rev-parse', 'HEAD'], text=True).strip() == manifest['source_commit']
    assert not subprocess.check_output(['git', '-C', str(source_root), 'status', '--porcelain', '--untracked-files=no'], text=True).strip()
    for path, expected in recipe['required_staging_not_executed']['files'].items():
        dest = root / 'generated-build-src/include' / Path(path).relative_to('headers-generes')
        dest.parent.mkdir(parents=True, exist_ok=True)
        assert not dest.exists()
        shutil.copyfile(package / path, dest)
        same(dest, expected)
    report['input_hashes'] = {str(path): digest(path) for path in (
        root / 'recipe.json', root / 'quarantine_released_diagnostic.cpp', manifest_path,
        Path(recipe['required_archive']), Path(recipe['flags_reference']['path']))}
    command('compiler-version', ['/usr/bin/c++', '--version'], 5)
    command('compile', recipe['compile_argv'], 120)
    command('link', recipe['link_argv'], 60)
    binary = root / 'quarantine_released_diagnostic'
    command('elf-dynamic', ['/usr/bin/readelf', '-d', str(binary)], 5)
    command('elf-header', ['/usr/bin/readelf', '-h', str(binary)], 5)
    text = (root / 'elf-dynamic.log').read_text()
    assert 'RPATH' not in text and 'RUNPATH' not in text, 'ELF project path'
    import re
    needed = re.findall(r'\(NEEDED\).*?\[(.*?)\]', text)
    assert set(needed) == {'libstdc++.so.6', 'libm.so.6', 'libgcc_s.so.1', 'libc.so.6', 'ld-linux-x86-64.so.2'}, needed
    report['needed'] = needed
    report['outputs'] = {str(path): {'sha256': digest(path), 'bytes': path.stat().st_size}
                         for path in (root / 'diagnostic.o', binary)}
    # Relecture des entrées après lien, sans exécuter le binaire.
    for path, expected in report['input_hashes'].items():
        assert digest(path) == expected, 'input changed: ' + path
    for path, expected in manifest['files'].items():
        same(package / path, expected)
    report['status'] = 'COMPILE_LINK_ELF_VERIFIED'
except BaseException as error:
    report['status'] = 'STOPPED'
    report['error'] = type(error).__name__ + ': ' + str(error)
finally:
    report['elapsed_seconds'] = time.monotonic() - started
    (root / 'build-report.json').write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n')
    print(json.dumps({'status': report['status'], 'error': report.get('error'),
                      'elapsed_seconds': report['elapsed_seconds']}, ensure_ascii=False), flush=True)
sys.exit(0 if report['status'] == 'COMPILE_LINK_ELF_VERIFIED' else 1)
