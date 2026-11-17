"""GO-only coupled launcher/exporter: no human ACK, no ELF read by exporter.
The preflight is a local frozen copy. Arming precedes launch, with a 15s lease.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time
import sys


def local_clock():
    # Same local clock domain as /proc parent start ticks, including suspend.
    return time.clock_gettime(time.CLOCK_BOOTTIME)


def run_export(local_deadline, children):
    parser = argparse.ArgumentParser()
    parser.add_argument('--preflight', required=True)
    parser.add_argument('--output', required=True)
    parser.add_argument('--launch-log', required=True)
    args = parser.parse_args()
    preflight = Path(args.preflight)
    data = json.loads(preflight.read_text())
    assert hashlib.sha256(Path(__file__).read_bytes()).hexdigest() == data['copy_hashes'][Path(__file__).name]
    package = data['package']
    assert package == '/home/lucied/.cache/rag3weaver-build/cx-lucivy-fs-admission-red-17nov-v2'
    assert data['source'] == '8b9435363dbd8ec0e3973f2368b641189f0726eb'
    destination = Path(args.output)
    assert destination.is_absolute() and not destination.exists() and not destination.is_symlink()
    assert destination.parent.is_dir()
    launch_log = Path(args.launch_log)
    assert launch_log.is_absolute() and not launch_log.exists() and not launch_log.is_symlink()
    assert launch_log.parent.is_dir()
    def remaining(reserve=0):
        value = local_deadline - local_clock() - reserve
        if value <= 0:
            raise TimeoutError('one local END exhausted')
        return value

    def rpc(body):
        return subprocess.run(['ssh', '-o', 'BatchMode=yes', 'luciepc', 'python3 -'],
            input=body, text=True, capture_output=True, timeout=min(8, remaining(2)))

    # No separate human or session must notice a message during the validation.
    # A successful ARM round trip must fit the full 8s transport allowance.
    arm_start = local_clock()
    wire_seconds = local_deadline - arm_start - 8
    assert wire_seconds > 30
    arm_code = '''import json,pathlib,os,time
q=pathlib.Path("/home/lucied/.cache/rag3weaver-build/cx-lucivy-fs-admission-red-17nov-v2")
expected=json.loads(%r)
p=q/"preflight.json"
assert json.loads(p.read_text())==expected
now=time.monotonic()
armed=dict(source=expected["source"],nonce=expected["nonce"],armed_at=now,
 launch_by=now+15,physical_deadline=now+%r,budget_seconds=%r)
with (q/"exporter-armed.json").open("x") as f:
 json.dump(armed,f);f.flush();os.fsync(f.fileno())
print(json.dumps(armed))
''' % (json.dumps(data), wire_seconds, wire_seconds)
    arm = rpc(arm_code)
    assert local_clock() - arm_start <= 8, 'ARM exceeded reserved transport duration'
    assert arm.returncode == 0, arm.stderr
    arm_reply = json.loads(arm.stdout)
    assert arm_reply['nonce'] == data['nonce']
    assert arm_reply['budget_seconds'] == wire_seconds
    remaining(15)
    print(json.dumps(dict(exporter_armed=True, nonce=data['nonce'])), flush=True)
    launch_body = """set -euo pipefail
set -C
test "$PWD" = /home/lucied/.cache/rag3weaver-build/cx-lucivy-fs-admission-red-17nov-v2
exec python3 - <<'PYBOUND' > launch.log 2>&1
import json,math,os,pathlib,time
q=pathlib.Path.cwd()
armed=json.loads((q/"exporter-armed.json").read_text())
term=math.floor(armed["physical_deadline"]-time.monotonic())-5
assert term>15
os.environ.update(POSTE_MEM_MAX="32G",POSTE_SWAP_MAX="0",POSTE_CPU_QUOTA="400%",RAG3WEAVER_BUILD_MEMORY_HIGH="0")
# Whole physical timeout includes poste acquisition and its actual return.
os.execv("/usr/bin/timeout",["timeout","--signal=TERM","--kill-after=5s",str(term),
 "./support/poste","lourd","timeout","--signal=TERM","--kill-after=5s","295",
 "python3","./admit-fs-admission-red.py"])
PYBOUND
"""
    with launch_log.open('xb') as file:
        launch = subprocess.Popen(['ssh', '-o', 'BatchMode=yes', 'luciepc',
            'cd /home/lucied/.cache/rag3weaver-build/cx-lucivy-fs-admission-red-17nov-v2\nexec bash -s'],
            stdin=subprocess.PIPE, stdout=file, stderr=subprocess.STDOUT)
        children.append(launch)
        launch.stdin.write(launch_body.encode()); launch.stdin.close()
    # Same END established by the physical parent, never reset after ARM/launch.
    code = '''import hashlib,json,pathlib,sys,time
q=pathlib.Path("/home/lucied/.cache/rag3weaver-build/cx-lucivy-fs-admission-red-17nov-v2")
bound=q/"scope/scope-provenance.json"
if not (q/"logs/elf-ledger-ready.json").exists(): sys.exit(3)
b=json.loads(bound.read_text())
assert time.monotonic()<b["deadline"]
p=q/"logs/elf-ledger.json"
if not p.exists(): sys.exit(3)
raw=p.read_bytes()
d=json.loads(raw)
assert d["source"]=="8b9435363dbd8ec0e3973f2368b641189f0726eb"
print(json.dumps({"nonce":b["nonce"],"raw":raw.decode(),"sha256":hashlib.sha256(raw).hexdigest()}))
'''
    while local_clock() < local_deadline:
        result = rpc(code)
        if result.returncode == 3:
            assert launch.poll() is None, 'launch ended before ledger publication'
            time.sleep(0.25)
            continue
        assert result.returncode == 0, result.stderr
        row = json.loads(result.stdout)
        assert row['nonce'] == data['nonce']
        raw = row['raw'].encode()
        assert hashlib.sha256(raw).hexdigest() == row['sha256']
        with destination.open('xb') as file:
            file.write(raw); file.flush(); os.fsync(file.fileno())
        assert hashlib.sha256(destination.read_bytes()).hexdigest() == row['sha256']
        parent = os.open(destination.parent, os.O_RDONLY | os.O_DIRECTORY)
        try:
            os.fsync(parent)
        finally:
            os.close(parent)
        ack = dict(source=data['source'], nonce=data['nonce'], sha256=row['sha256'],
                   copied_and_fsynced_on_portable=True)
        # Only after the local durable write. Remote body validates bounds and nonce.
        code = '''import json,pathlib,os,time
q=pathlib.Path("/home/lucied/.cache/rag3weaver-build/cx-lucivy-fs-admission-red-17nov-v2")
ack=json.loads(%r)
b=json.loads((q/"scope/scope-provenance.json").read_text())
assert time.monotonic()<b["deadline"] and ack["nonce"]==b["nonce"]
tmp=q/"elf-ledger-export-ack.pending"
with tmp.open("x") as f:
 json.dump(ack,f);f.flush();os.fsync(f.fileno())
os.link(tmp,q/"elf-ledger-export-ack.json")
tmp.unlink()
fd=os.open(q,os.O_RDONLY|os.O_DIRECTORY)
os.fsync(fd);os.close(fd)
''' % json.dumps(ack)
        acknowledged = rpc(code)
        assert acknowledged.returncode == 0, acknowledged.stderr
        print(json.dumps(dict(copied=str(destination), **ack)), flush=True)
        code = launch.wait(timeout=remaining(10))
        assert code == 0, f'actual poste/SSH wrapper failed: {code}'
        final_code = '''import json,pathlib,time
q=pathlib.Path("/home/lucied/.cache/rag3weaver-build/cx-lucivy-fs-admission-red-17nov-v2")
b=json.loads((q/"scope/scope-provenance.json").read_text())
assert time.monotonic()<b["deadline"]
print(json.dumps(dict(remaining=b["deadline"]-time.monotonic(),
 files={name:(q/name).read_text() for name in ["final-accepted.json","final-evidence.json"]})))
'''
        transfer_start = local_clock()
        final = rpc(final_code)
        assert final.returncode == 0, final.stderr
        final_payload = json.loads(final.stdout)
        with destination.with_name('exported-final.json').open('x') as file:
            json.dump(final_payload, file); file.flush(); os.fsync(file.fileno())
        assert local_clock() - transfer_start < final_payload['remaining'], 'final export exceeded physical budget'
        remaining()
        return 0
    raise TimeoutError('No exported ELF ledger within the single proposed envelope')


def physical_entry():
    """Compose coreutils timeout before any argparse/preflight/transport I/O.

    The worker refuses a forged private flag unless its real parent is the
    exact timeout command. Only local parent metadata is read; no process scan.
    """
    flag = '--_physical-watchdog-child'
    script = str(Path(__file__).absolute())
    prefix = ['timeout', '--signal=TERM', '--kill-after=5s', '295s', sys.executable, script]
    if len(sys.argv) < 2 or sys.argv[1] != flag:
        os.environ['PYTHONDONTWRITEBYTECODE'] = '1'
        os.execv('/usr/bin/timeout', prefix + [flag] + sys.argv[1:])
        raise RuntimeError('execv must not return')
    parent = os.getppid()
    assert os.readlink(f'/proc/{parent}/exe') == '/usr/bin/timeout'
    command = Path(f'/proc/{parent}/cmdline').read_bytes().split(bytes([0]))
    assert command[-1] == b''
    observed_command = [item.decode() for item in command[:-1]]
    assert observed_command[0] in ('timeout', '/usr/bin/timeout')
    assert observed_command[1:] == prefix[1:] + sys.argv[1:]
    fields = Path(f'/proc/{parent}/stat').read_text().rsplit(') ', 1)[1].split()
    parent_start = int(fields[19]) / os.sysconf('SC_CLK_TCK')
    assert 0 < parent_start <= local_clock() and os.getppid() == parent
    # Jiffy quantization rounds the parent's start down, conservatively.
    local_deadline = parent_start + 285
    assert local_clock() < local_deadline
    del sys.argv[1]
    children = []
    try:
        return run_export(local_deadline, children)
    finally:
        # Ordinary errors must not leave a local SSH child alive. If Python is
        # stuck in I/O, the physical timeout kills its whole process group.
        for child in children:
            if child.poll() is None:
                child.terminate()
                try:
                    child.wait(timeout=max(0.01, min(2, local_deadline + 3 - local_clock())))
                except subprocess.TimeoutExpired:
                    child.kill()
                    child.wait(timeout=max(0.01, min(1, local_deadline + 5 - local_clock())))


if __name__ == '__main__':
    raise SystemExit(physical_entry())
