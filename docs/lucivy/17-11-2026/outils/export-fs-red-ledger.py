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


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--preflight', required=True)
    parser.add_argument('--output', required=True)
    parser.add_argument('--launch-log', required=True)
    args = parser.parse_args()
    preflight = Path(args.preflight)
    data = json.loads(preflight.read_text())
    assert hashlib.sha256(Path(__file__).read_bytes()).hexdigest() == data['copy_hashes'][Path(__file__).name]
    package = data['package']
    assert package == '/home/lucied/.cache/rag3weaver-build/cx-lucivy-fs-admission-red-17nov-v1'
    assert data['source'] == '8b9435363dbd8ec0e3973f2368b641189f0726eb'
    destination = Path(args.output)
    assert destination.is_absolute() and not destination.exists() and not destination.is_symlink()
    assert destination.parent.is_dir()
    launch_log = Path(args.launch_log)
    assert launch_log.is_absolute() and not launch_log.exists() and not launch_log.is_symlink()
    assert launch_log.parent.is_dir()
    # No separate human or session must notice a message during the validation.
    arm_code = '''import json,pathlib,os,time
q=pathlib.Path("/home/lucied/.cache/rag3weaver-build/cx-lucivy-fs-admission-red-17nov-v1")
expected=json.loads(%r)
p=q/"preflight.json"
assert json.loads(p.read_text())==expected
now=time.monotonic()
armed=dict(source=expected["source"],nonce=expected["nonce"],armed_at=now,
 launch_by=now+15,physical_deadline=now+300)
with (q/"exporter-armed.json").open("x") as f:
 json.dump(armed,f);f.flush();os.fsync(f.fileno())
print(json.dumps(armed))
''' % json.dumps(data)
    arm = subprocess.run(['ssh', '-o', 'BatchMode=yes', 'luciepc', 'python3 -'],
        input=arm_code, text=True, capture_output=True, timeout=8)
    assert arm.returncode == 0, arm.stderr
    assert json.loads(arm.stdout)['nonce'] == data['nonce']
    print(json.dumps(dict(exporter_armed=True, nonce=data['nonce'])), flush=True)
    launch_body = """set -euo pipefail
set -C
test "$PWD" = /home/lucied/.cache/rag3weaver-build/cx-lucivy-fs-admission-red-17nov-v1
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
            'cd /home/lucied/.cache/rag3weaver-build/cx-lucivy-fs-admission-red-17nov-v1\nexec bash -s'],
            stdin=subprocess.PIPE, stdout=file, stderr=subprocess.STDOUT)
        launch.stdin.write(launch_body.encode()); launch.stdin.close()
    # Bounded polling, started only once the real scope-provenance has appeared.
    local_deadline = time.monotonic() + 300
    code = '''import hashlib,json,pathlib,sys,time
q=pathlib.Path("/home/lucied/.cache/rag3weaver-build/cx-lucivy-fs-admission-red-17nov-v1")
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
    while time.monotonic() < local_deadline:
        result = subprocess.run(['ssh', '-o', 'BatchMode=yes', 'luciepc', 'python3 -'],
            input=code, text=True, capture_output=True, timeout=8)
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
q=pathlib.Path("/home/lucied/.cache/rag3weaver-build/cx-lucivy-fs-admission-red-17nov-v1")
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
        acknowledged = subprocess.run(['ssh', '-o', 'BatchMode=yes', 'luciepc', 'python3 -'],
            input=code, text=True, capture_output=True, timeout=8)
        assert acknowledged.returncode == 0, acknowledged.stderr
        print(json.dumps(dict(copied=str(destination), **ack)), flush=True)
        code = launch.wait(timeout=max(1, local_deadline - time.monotonic()))
        assert code == 0, f'actual poste/SSH wrapper failed: {code}'
        final_code = '''import json,pathlib,time
q=pathlib.Path("/home/lucied/.cache/rag3weaver-build/cx-lucivy-fs-admission-red-17nov-v1")
b=json.loads((q/"scope/scope-provenance.json").read_text())
assert time.monotonic()<b["deadline"]
print(json.dumps(dict(remaining=b["deadline"]-time.monotonic(),
 files={name:(q/name).read_text() for name in ["final-accepted.json","final-evidence.json"]})))
'''
        transfer_start = time.monotonic()
        final = subprocess.run(['ssh', '-o', 'BatchMode=yes', 'luciepc', 'python3 -'],
            input=final_code, text=True, capture_output=True, timeout=8)
        assert final.returncode == 0, final.stderr
        final_payload = json.loads(final.stdout)
        with destination.with_name('exported-final.json').open('x') as file:
            json.dump(final_payload, file); file.flush(); os.fsync(file.fileno())
        assert time.monotonic() - transfer_start < final_payload['remaining'], 'final export exceeded physical budget'
        return 0
    raise TimeoutError('No exported ELF ledger within the single proposed envelope')


if __name__ == '__main__':
    raise SystemExit(main())
