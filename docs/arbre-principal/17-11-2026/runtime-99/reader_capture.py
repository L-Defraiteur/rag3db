"""SDK59 adaptation of capture-stable (64abd), same EXEC/MAIN inferior.

Source only until separate qualification. No import outside GDB at runtime.
"""
import hashlib,json,os,time
from pathlib import Path
import gdb

def write(path,value):
    with Path(path).open('x') as stream:
        json.dump(value,stream,indent=2,allow_nan=False);stream.flush();os.fsync(stream.fileno())

def capture(stage,config):
    root=Path(config['proof']);pid=gdb.selected_inferior().pid;proc=Path('/proc')/str(pid)
    attempt=dict(stage=stage,pid=pid,inferior=gdb.selected_inferior().num,observed=time.monotonic())
    try:
        stamp=(proc/'stat').read_text().rsplit(') ',1)[1].split()[19]
        data=(proc/'environ').read_bytes();pairs=[value.split(b'=',1) for value in data.split(b'\0') if b'=' in value]
        raw=dict(pairs)
        if len(raw)!=len(pairs):raise ValueError('duplicate native environment key')
        actual={key:os.fsdecode(raw[os.fsencode(key)]) if os.fsencode(key) in raw else None for key in config['expected']}
        exe=os.readlink(proc/'exe');cg=(proc/'cgroup').read_text();maps=(proc/'maps').read_text()
        sha=hashlib.sha256((proc/'exe').read_bytes()).hexdigest()
        argv=[os.fsdecode(value) for value in (proc/'cmdline').read_bytes().split(b'\0') if value]
        native=[line for line in maps.splitlines() if 'librag3db.so' in line]
        diff={key:dict(expected=value,actual=actual[key]) for key,value in config['expected'].items() if actual[key]!=value}
        after=dict(start=(proc/'stat').read_text().rsplit(') ',1)[1].split()[19],exe=os.readlink(proc/'exe'),cgroup=(proc/'cgroup').read_text(),exe_sha256=hashlib.sha256((proc/'exe').read_bytes()).hexdigest())
        attempt.update(start=stamp,uid=proc.stat().st_uid,exe=exe,exe_sha256=sha,cgroup=cg,argv=argv,rawlen=len(data),present={key:os.fsencode(key) in raw for key in config['expected']},expected=config['expected'],actual=actual,diff=diff,maps=maps,native=native,after=after)
        write(root/('capture-'+stage+'.json'),attempt) # RAW before assertions.
        if not data or actual!=config['expected'] or diff or not all(attempt['present'].values()):raise ValueError('native reader profile absent/different')
        if exe!=config['argv'][0] or argv!=config['argv'] or sha!=config['sha256'] or cg!=config['cgroup'] or attempt['uid']!=os.getuid():raise ValueError('native reader identity/role differs')
        if after!=dict(start=stamp,exe=exe,cgroup=cg,exe_sha256=sha):raise ValueError('reader changed during capture')
        if time.monotonic()>=config['deadline']:raise ValueError('reader capture deadline')
        if stage=='main':
            first=json.loads((root/'capture-exec.json').read_text())
            if any(first[key]!=attempt[key] for key in ['pid','start','inferior','exe','exe_sha256','cgroup','argv']):raise ValueError('different EXEC/MAIN reader')
            if not native or any(line.split()[-1]!=config['library'] for line in native):raise ValueError('actual SDK59 map absent/foreign')
        return attempt
    except BaseException as error:
        write(root/('capture-error-'+stage+'.json'),dict(attempt=attempt,error=repr(error)))
        raise

def install_exit(config,main):
    def exited(event):
        if event.inferior.num!=main['inferior']:raise ValueError('unrelated inferior exit')
        record=dict(pid=main['pid'],start=main['start'],inferior=event.inferior.num,exit_code=getattr(event,'exit_code',None),observed=time.monotonic())
        write(Path(config['proof'])/'native-exit.json',record)
    gdb.events.exited.connect(exited)
