"""Enter a private native unit after real poste admission; publish final quiescence."""
import json
import hashlib
import math
import os
from pathlib import Path
import subprocess
import sys
import time

from parse_receipt import admitted_cpu, verify_attestation

out = Path(sys.argv[1])
started = time.monotonic()
raw = out/'raw/driver'
raw.mkdir(parents=True, exist_ok=True)
recipe = json.loads((out/'recipe.json').read_text())
for filename, expected in recipe['scripts'].items():
    assert hashlib.sha256((out/filename).read_bytes()).hexdigest()==expected
unit = recipe['unit']+'.service'
cg = next(x.split(':', 2)[2] for x in Path('/proc/self/cgroup').read_text().splitlines() if x.startswith('0::'))
cgroup = Path('/sys/fs/cgroup')/cg.lstrip('/')
quota_text = (cgroup/'cpu.max').read_text().strip()
cpu=admitted_cpu(quota_text,os.environ['POSTE_FILS'])
threads=cpu['jobs']
assert (cgroup/'memory.max').read_text().strip() == '34359738368'
assert (cgroup/'memory.swap.max').read_text().strip() == '0'
admission = {'started_monotonic':started,'parent_cgroup':cg,'parent_cpu_max':quota_text,
             'poste_threads':os.environ['POSTE_FILS'],'build_jobs':threads,
             'child_cpu_quota_percent':cpu['quota_percent'], 'cap_s':10800}
(raw/'admission.json').write_text(json.dumps(admission,indent=2)+'\n')
remaining = 10800-(time.monotonic()-started)
runtime = min(10790, math.floor(remaining-10))
assert runtime>0
scratch = Path.home()/'.cache/rag3weaver-build/cx-coeur-dictionary-global'
assert scratch.is_dir() and not any(scratch.iterdir())
cmd = ['systemd-run','--user','--wait','--pipe','--unit='+recipe['unit'],
       '--property=MemoryMax=34359738368','--property=MemorySwapMax=0',
       '--property=CPUQuota='+str(admission['child_cpu_quota_percent'])+'%',
       '--property=RuntimeMaxSec='+str(runtime),'--property=TimeoutStopSec=10',
       '--property=KillMode=control-group','--property=Nice=19','--property=IOSchedulingClass=idle',
       'bwrap','--unshare-user','--unshare-net','--ro-bind','/','/','--proc','/proc','--dev','/dev',
       '--size','34359738368','--tmpfs',str(scratch),'--bind',str(out),str(out),
       '/usr/bin/env','-i','PATH=/usr/bin:/bin','LANG=C','LC_ALL=C','TZ=UTC',
       '/usr/bin/python3',str(out/'controller.py'),str(scratch),str(out),
       str(Path.home()/'git_workspaces/rag3db-colonnes'),str(threads),str(started+10770)]
(raw/'argv.json').write_text(json.dumps(cmd,indent=2)+'\n')
code=1
try:
    with (raw/'unit.log').open('wb') as log:
        code=subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT,timeout=max(1,10790-(time.monotonic()-started))).returncode
except subprocess.TimeoutExpired:
    subprocess.run(['systemctl','--user','stop',unit],timeout=max(1,min(10,10800-(time.monotonic()-started))),check=False)
    code=124
state=subprocess.check_output(['systemctl','--user','show',unit,'-p','ActiveState','-p','SubState',
                              '-p','MainPID','-p','ControlGroup','-p','Result'],text=True)
(raw/'unit-state.txt').write_text(state)
fields=dict(x.split('=',1) for x in state.splitlines() if '=' in x)
quiet=fields.get('MainPID')=='0' and not fields.get('ControlGroup') and fields.get('ActiveState') in ['inactive','failed']
remaining_pids=[]
for command_file in Path('/proc').glob('[0-9]*/cmdline'):
    try:
        args=command_file.read_bytes().split(b'\0')
        if (args and args[0].startswith(str(scratch/'build').encode())) or str(out/'controller.py').encode() in args:
            remaining_pids.append(command_file.parent.name)
    except OSError:
        pass
quiet = quiet and not remaining_pids
receipt={'remaining_attributed_pids':remaining_pids,'unit':state,'quiescent':quiet,'systemd_returncode':code,'elapsed_s':time.monotonic()-started,
         'cap_s':10800,'cpu_quota_percent':admission['child_cpu_quota_percent'],'jobs':threads}
(raw/'quiescence.json').write_text(json.dumps(receipt,indent=2)+'\n')
records=[]
for path in sorted((out/'raw').rglob('*')):
    if path.is_file():
        with path.open('rb') as stream: checksum=hashlib.file_digest(stream,'sha256').hexdigest()
        records.append({'path':str(path.relative_to(out)),'bytes':path.stat().st_size,'sha256':checksum})
(out/'raw-inventory.json').write_text(json.dumps(records,indent=2)+'\n')
accepted=code==0 and quiet and (out/'raw/parsing/receipt.json').is_file()
if accepted:
    manifest=json.loads((out/'raw/artifacts/manifest.json').read_text())
    verify_attestation(out/'raw/offsets/stdout.log',out/'attestation.log',manifest['build_id_header'])
    for filename, info in manifest['files'].items():
        with (out/'raw/artifacts'/filename).open('rb') as stream:
            assert hashlib.file_digest(stream,'sha256').hexdigest()==info['sha256']
with (out/'resume.log').open('a') as log:
    for row in records:log.write('RAW '+row['path']+' bytes '+str(row['bytes'])+' sha256 '+row['sha256']+'\n')
    log.write('quiescence '+json.dumps(receipt)+'\n')
    log.write('VERT\n' if accepted else 'ROUGE\n')
raise SystemExit(0 if accepted else 1)
