"""Poste-admitted entry, under an external2400s watchdog including finalization."""
import hashlib
import json
import math
import os
from pathlib import Path
import signal
import subprocess
import sys
import time

from receipt import admitted_cpu, verify_native

out=Path(sys.argv[1])
started=time.monotonic()
native_end=started+2330
final_end=started+2380
controller_end=started+2310
unit='cx-hotfixs-rows-red-01.service'
scratch=Path.home()/'.cache/rag3weaver-build/cx-hotfixs-rows-red'
raw=out/'raw/driver'
raw.mkdir(parents=True,exist_ok=True)
first=None
invoked=False
code=1
quiet=False
state='not started\n'
remaining_pids=[]
recipe={}
process=None


def time_left(end, reserve=0):
    value=end-time.monotonic()-reserve
    if value<=0:
        raise TimeoutError('entry deadline')
    return value


def write(path,value):
    path.write_text(json.dumps(value,indent=2)+'\n')


def failure(stage,exc):
    global first
    if first is None:
        first={'stage':stage,'type':type(exc).__name__,'message':str(exc),
               'elapsed_s':time.monotonic()-started,'retry':False}
        write(raw/'first-refusal.json',first)


def checksum(path):
    h=hashlib.sha256()
    with path.open('rb') as stream:
        while True:
            time_left(final_end)
            block=stream.read(1024*1024)
            if not block:
                break
            h.update(block)
    return h.hexdigest()


def command(argv,seconds=3):
    return subprocess.run(argv,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True,
                          timeout=min(seconds,time_left(final_end)))


def observe_unit():
    result=command(['systemctl','--user','show',unit,'-p','ActiveState','-p','SubState',
                    '-p','MainPID','-p','ControlGroup','-p','Result'])
    (raw/'unit-state.txt').write_text(result.stdout)
    if result.returncode:
        raise RuntimeError('cannot observe private unit: '+result.stdout)
    return result.stdout


def interrupted(signum,frame):
    raise TimeoutError('external watchdog requested stop')


signal.signal(signal.SIGTERM,interrupted)
try:
    recipe=json.loads((out/'recipe.json').read_text())
    assert recipe['unit']+'.service'==unit
    for filename,expected in recipe['scripts'].items():
        assert checksum(out/filename)==expected
    # Refuse an already-existing unit; cleanup must never stop another invocation.
    before=command(['systemctl','--user','show',unit,'-p','LoadState'])
    (raw/'before-unit.txt').write_text(before.stdout)
    assert 'LoadState=not-found' in before.stdout
    cg=next(x.split(':',2)[2] for x in Path('/proc/self/cgroup').read_text().splitlines() if x.startswith('0::'))
    parent=Path('/sys/fs/cgroup')/cg.lstrip('/')
    quota_text=(parent/'cpu.max').read_text().strip()
    cpu=admitted_cpu(quota_text,os.environ['POSTE_FILS'])
    assert (parent/'memory.max').read_text().strip()=='34359738368'
    assert (parent/'memory.swap.max').read_text().strip()=='0'
    admission={'started_monotonic':started,'parent_cgroup':cg,'parent_cpu_max':quota_text,
               'poste_threads':os.environ['POSTE_FILS'],'build_jobs':cpu['jobs'],
               'child_cpu_quota_percent':cpu['quota_percent'],'cap_s':2400,
               'native_end_monotonic':native_end,'final_end_monotonic':final_end,
               'controller_end_monotonic':controller_end}
    write(raw/'admission.json',admission)
    assert scratch.is_dir() and not any(scratch.iterdir())
    runtime=min(2320,math.floor(time_left(native_end,10)))
    cmd=['systemd-run','--user','--wait','--pipe','--unit='+recipe['unit'],
         '--property=MemoryMax=34359738368','--property=MemorySwapMax=0',
         '--property=CPUQuota='+str(cpu['quota_percent'])+'%',
         '--property=RuntimeMaxSec='+str(runtime),'--property=TimeoutStopSec=10',
         '--property=KillMode=control-group','--property=Nice=19','--property=IOSchedulingClass=idle',
         'bwrap','--unshare-user','--unshare-net','--ro-bind','/','/','--proc','/proc','--dev','/dev',
         '--size','34359738368','--tmpfs',str(scratch),'--bind',str(out),str(out),
         '/usr/bin/env','-i','PATH=/usr/bin:/bin','LANG=C','LC_ALL=C','TZ=UTC',
         '/usr/bin/python3',str(out/'controller.py'),str(scratch),str(out),
         str(cpu['jobs']),str(controller_end)]
    write(raw/'argv.json',cmd)
    with (raw/'unit.log').open('wb') as log:
        process=subprocess.Popen(cmd,stdout=log,stderr=subprocess.STDOUT)
        invoked=True
        code=process.wait(timeout=time_left(native_end))
    if code:
        raise RuntimeError('private native unit returned '+str(code))
except BaseException as exc:
    failure('execution',exc)
finally:
    # Reserve finalization time independently of all native phase deadlines.
    if invoked:
        try:
            state=observe_unit()
            fields=dict(x.split('=',1) for x in state.splitlines() if '=' in x)
            if fields.get('MainPID')!='0' or fields.get('ControlGroup') or fields.get('ActiveState') not in ['inactive','failed']:
                stopped=command(['systemctl','--user','stop',unit],10)
                (raw/'stop-output.txt').write_text(stopped.stdout)
                state=observe_unit()
                fields=dict(x.split('=',1) for x in state.splitlines() if '=' in x)
            quiet=fields.get('MainPID')=='0' and not fields.get('ControlGroup') and fields.get('ActiveState') in ['inactive','failed']
            if process is not None and process.poll() is None:
                process.wait(timeout=min(3,time_left(final_end)))
            for path in Path('/proc').glob('[0-9]*/cmdline'):
                time_left(final_end)
                try:
                    args=path.read_bytes().split(b'\0')
                    if (args and args[0].startswith(str(scratch/'build').encode())) or str(out/'controller.py').encode() in args:
                        remaining_pids.append(path.parent.name)
                except OSError:
                    pass
            quiet=quiet and not remaining_pids
            if not quiet:
                raise RuntimeError('private native unit not quiescent')
        except BaseException as exc:
            quiet=False
            failure('quiescence',exc)
    else:
        quiet=True
    try:
        time_left(final_end)
        accepted=first is None and code==0 and quiet and (out/'raw/parsing/receipt.json').is_file()
        if accepted:
            manifest=json.loads((out/'raw/artifacts/manifest.json').read_text())
            verify_native(out, manifest)
            time_left(final_end)
            for filename,info in manifest['files'].items():
                assert checksum(out/'raw/artifacts'/filename)==info['sha256']
        elif first is None:
            raise RuntimeError('missing native result receipt')
        receipt={'unit':state,'invoked':invoked,'quiescent':quiet,
                 'remaining_attributed_pids':remaining_pids,'systemd_returncode':code,
                 'elapsed_s':time.monotonic()-started,'cap_s':2400,'first_refusal':first}
        write(raw/'quiescence.json',receipt)
        records=[]
        for path in sorted((out/'raw').rglob('*')):
            time_left(final_end)
            if path.is_file():
                records.append({'path':str(path.relative_to(out)),
                                'bytes':path.stat().st_size,'sha256':checksum(path)})
        write(out/'raw-inventory.json',records)
        with (out/'resume.log').open('a') as summary:
            for row in records:
                time_left(final_end)
                summary.write('RAW '+row['path']+' bytes '+str(row['bytes'])+' sha256 '+row['sha256']+'\n')
            summary.write('quiescence '+json.dumps(receipt)+'\n')
            time_left(final_end)
            summary.write('EXPECTED_BINDER_RED_OBSERVED; NOT A DELIVERY PASS\n' if accepted else 'DIAGNOSTIC REFUSED\n')
            summary.write('ROUGE\n')
    except BaseException as exc:
        failure('finalization',exc)
        accepted=False
        # This small refusal record is best effort under the external hard watchdog;
        # missing/partial final evidence can never authorize a green push.
        write(raw/'quiescence-incomplete.json',{'unit':state,'quiescent':quiet,
              'first_refusal':first,'elapsed_s':time.monotonic()-started})
        with (out/'resume.log').open('a') as summary:
            summary.write('ROUGE finalization incomplete\nROUGE\n')
raise SystemExit(0 if accepted else 1)
