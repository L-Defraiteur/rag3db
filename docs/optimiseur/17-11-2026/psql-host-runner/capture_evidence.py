"""Future RealCapture CPU evidence, same test process, no observer fork.

Only the published Python fixtures may call this helper under a separate GO.
No candidate runs at import. A capture/archival refusal is never converted into
success; raw and record are saved before re-raising the original capture error.
"""
import hashlib
import json
import os
from pathlib import Path
import stat
import sys
import time


def write_new(path, raw):
    fd=os.open(path,os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600)
    with os.fdopen(fd,'wb') as stream:
        stream.write(raw);stream.flush();os.fsync(stream.fileno())


def sync(directory):
    fd=os.open(directory,os.O_RDONLY|os.O_DIRECTORY)
    try:os.fsync(fd)
    finally:os.close(fd)


def proc_state(pid,start):
    """No signal sent. PID reuse is explicitly distinct from the original."""
    try:
        text=(Path('/proc')/str(pid)/'stat').read_text().rsplit(')',1)[1].split()
    except FileNotFoundError:return dict(original='ABSENT',state=None)
    if text[19]!=str(start):return dict(original='ABSENT_PID_REUSED',state=text[0])
    return dict(original='PRESENT',state=text[0])


def fd_snapshot():
    # Parent test process only, no cross-process FD scanning or observer fork.
    result={}
    for name in os.listdir('/proc/self/fd'):
        try:result[name]=os.readlink('/proc/self/fd/'+name)
        except FileNotFoundError:pass  # The directory enumeration FD already closed.
    return result


def archived_capture(runner,test_name,argv,*,env,stdin,deadline):
    # The exact private root is frozen in the proposed recipe, not discovered
    # in user configs. No fallback writing into a source tree or shared cache.
    raw_root=os.environ.get('PSQL_CPU_CAPTURE_PROOFS')
    if not raw_root:raise ValueError('missing private CPU evidence root')
    root=Path(raw_root).absolute()
    if root.resolve()!=root or any(x in ('.env','.vault') for x in root.parts):
        raise ValueError('invalid CPU evidence root')
    if not root.exists():root.mkdir(mode=0o700)
    info=root.stat()
    if info.st_uid!=os.getuid() or stat.S_IMODE(info.st_mode)!=0o700:
        raise ValueError('CPU evidence directory must be owned700')
    if not test_name.replace('_','').isalnum():raise ValueError('invalid test identity')
    target=root/test_name;target.mkdir(mode=0o700)
    requested=dict(test=test_name,argv=argv,environment=env,stdin_sha256=hashlib.sha256(stdin).hexdigest(),
        started_monotonic=time.monotonic(),deadline_monotonic=deadline,
        requested_python=str(Path(sys.executable).resolve()),
        requested_python_sha256=hashlib.sha256(Path(sys.executable).resolve().read_bytes()).hexdigest())
    expected_python=os.environ.get('PSQL_CPU_PYTHON_SHA256')
    if requested['requested_python_sha256']!=expected_python:
        raise ValueError('fixture Python pin differs')
    before_fds=fd_snapshot()
    requested['parent_fds_before']=before_fds
    write_new(target/'requested.json',(json.dumps(requested,sort_keys=True)+'\n').encode())
    sync(target)
    error=None
    try:
        record=runner.capture(argv,env=env,stdin=stdin,deadline=deadline)
    except runner.CaptureRefused as caught:
        error=caught;record=caught.record
    # Other exceptions are not caught or interpreted as a correct refusal.
    for channel in ['stdout','stderr']:
        raw=record.get(channel)
        if type(raw) is not bytes:raise ValueError('missing bounded capture raw')
        write_new(target/channel,raw)
    after_fds=fd_snapshot()
    meta={k:v for k,v in record.items() if k not in ('stdout','stderr')}
    observed_exe=record.get('exe')
    observed_matches=(observed_exe==requested['requested_python'] and record.get('observed_unknown') is False)
    meta.update(capture_refused=error is not None,observed_python_matches_requested=observed_matches,
        parent_fds_after=after_fds,parent_fds_equal=before_fds==after_fds,
        stdout_sha256=hashlib.sha256(record['stdout']).hexdigest(),
        stderr_sha256=hashlib.sha256(record['stderr']).hexdigest(),
        original_process_after_capture=proc_state(record['pid'],record['start'])
            if record.get('pid') and record.get('start') else dict(original='UNKNOWN',state=None))
    write_new(target/'capture.json',(json.dumps(meta,sort_keys=True,allow_nan=False)+'\n').encode())
    sync(target)
    completed=time.monotonic()
    write_new(target/'archived.json',(json.dumps(dict(ended_monotonic=completed,
        within_original_deadline=completed<deadline,original_capture_refusal=error is not None))+'\n').encode())
    sync(target);sync(root)
    # Evidence latency is inside the original cap as well, not a fresh budget.
    if time.monotonic()>=deadline:
        raise AssertionError('capture evidence exceeded original deadline') from error
    if (not observed_matches or not record.get('start') or before_fds!=after_fds or
            not meta['original_process_after_capture']['original'].startswith('ABSENT')):
        raise AssertionError('owned Python identity/reap/parent-pipe evidence incomplete') from error
    if error is not None:raise error
    return record
