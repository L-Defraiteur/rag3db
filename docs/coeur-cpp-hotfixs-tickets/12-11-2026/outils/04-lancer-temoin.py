"""Lanceur préparé seulement ; exige un accord distinct lié au binaire attesté."""
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import threading


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def check(root, approval):
    approval = json.loads(approval.read_text())
    assert approval['approved_to_run'] is True, 'aucun accord runtime'
    report = root / 'build-report.json'
    build = json.loads(report.read_text())
    assert build['status'] == 'COMPILE_LINK_ELF_VERIFIED', 'build non attesté'
    assert sha(report) == approval['build_report_sha256']
    binary = root / 'quarantine_released_diagnostic'
    assert sha(binary) == approval['binary_sha256'] == build['outputs'][str(binary)]['sha256']
    assert sha(root / 'quarantine_released_diagnostic.cpp') == approval['source_sha256']
    assert sha(root / 'runtime.py') == approval['launcher_sha256']
    assert approval['scope'] == 'released-structural-only'
    return approval, binary


def main():
    inside = len(sys.argv) == 5 and sys.argv[1] == '--inside'
    if inside:
        root, approval_file = Path(sys.argv[2]), Path(sys.argv[3])
        assert sha(approval_file) == sys.argv[4], 'accord modifié'
    else:
        assert len(sys.argv) == 3, 'usage: runtime.py <staging absolu> <accord JSON absolu>'
        root, approval_file = Path(sys.argv[1]), Path(sys.argv[2])
    assert root.is_absolute() and approval_file.is_absolute()
    approval, binary = check(root, approval_file)
    if not inside:
        unit = approval['unit']
        assert unit.startswith('cx-hotfixs-quarantaine-runtime-')
        assert all(c.isalnum() or c == '-' for c in unit)
        argv = ['systemd-run', '--user', '--wait', '--pipe', '--collect', '--unit=' + unit,
                '-p', 'MemoryMax=1G', '-p', 'MemorySwapMax=0', '-p', 'RuntimeMaxSec=20',
                '-p', 'TimeoutStopSec=1', '-p', 'KillMode=control-group', '-p', 'TasksMax=64',
                '-p', 'Restart=no', '-p', 'LimitCORE=0', '/usr/bin/python3', str(root / 'runtime.py'),
                '--inside', str(root), str(approval_file), sha(approval_file)]
        # L'accord est revalidé dans le cgroup avant tout accès à une base.
        return subprocess.run(argv).returncode
    rel = next(line.split(':', 2)[2] for line in Path('/proc/self/cgroup').read_text().splitlines()
               if line.startswith('0::'))
    cg = Path('/sys/fs/cgroup') / rel.lstrip('/')
    assert (cg / 'memory.max').read_text().strip() == '1073741824'
    assert (cg / 'memory.swap.max').read_text().strip() == '0'
    run = Path(tempfile.mkdtemp(prefix='runtime-', dir=root))
    artifacts = run / 'artifacts'
    artifacts.mkdir()
    argv = [str(binary), str(artifacts)]
    result = {'argv': argv, 'cgroup': str(cg), 'approval_sha256': sha(approval_file),
              'scope': 'structural-only', 'ready_for_durable_claim': False}
    (run / 'runtime-start.json').write_text(json.dumps(result, indent=2) + '\n')
    print('runtime artefacts: ' + str(run), flush=True)
    proc = subprocess.Popen(argv, start_new_session=True, stdout=subprocess.PIPE,
                            stderr=subprocess.STDOUT)
    size = [0]

    def drain():
        with (run / 'stdout.log').open('xb') as output:
            for block in iter(lambda: proc.stdout.read(65536), b''):
                output.write(block[:max(0, (1 << 20) - size[0])])
                size[0] += len(block)

    reader = threading.Thread(target=drain)
    reader.start()
    try:
        result['exit_code'] = proc.wait(timeout=15)
    except subprocess.TimeoutExpired:
        result['timeout'] = True
        os.killpg(proc.pid, signal.SIGKILL)
        result['exit_code'] = proc.wait()
    finally:
        # Un enfant laissé après la sortie du parent est arrêté, pas détaché du diagnostic.
        try:
            os.killpg(proc.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        reader.join(timeout=1)
        result['output_bytes'] = size[0]
        result['reader_finished'] = not reader.is_alive()
        (run / 'runtime-result.json').write_text(json.dumps(result, indent=2) + '\n')
    return 0 if result['exit_code'] == 0 and not result.get('timeout') and size[0] <= 1 << 20 and not reader.is_alive() else 1


if __name__ == '__main__':
    try:
        sys.exit(main())
    except Exception as error:
        print('RUNTIME REFUSED: ' + type(error).__name__ + ': ' + str(error), file=sys.stderr)
        sys.exit(2)
