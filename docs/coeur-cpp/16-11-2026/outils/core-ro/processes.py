"""Owned process groups, bounded waits and per-command cgroup residue checks."""
import json
import os
import signal
import subprocess
import time
process = None

def write(path, data):
    path.write_text(json.dumps(data,indent=2)+'\n')

def group_exists(group):
    try:
        os.killpg(group,0)
        return True
    except ProcessLookupError:
        return False


def cgroup_pids():
    return {int(x) for x in (cgp/'cgroup.procs').read_text().split()}


def finish_child():
    if process is None:
        return False
    residue=process.poll() is not None and group_exists(process.pid)
    if group_exists(process.pid):
        try:
            os.killpg(process.pid,signal.SIGTERM)
        except ProcessLookupError:
            pass
        end=time.monotonic()+3
        while group_exists(process.pid) and time.monotonic()<end:
            process.poll()  # Reap an exited leader before interpreting killpg(0).
            if not group_exists(process.pid):
                break
            time.sleep(.05)
        if group_exists(process.pid):
            try:
                os.killpg(process.pid,signal.SIGKILL)
            except ProcessLookupError:
                pass
    if process.poll() is None:
        process.wait(timeout=3)
    return residue


def run(argv, label, seconds, env=None):
    global process
    process = None
    baseline_pids = cgroup_pids()
    directory = raw / label
    directory.mkdir(parents=True, exist_ok=True)
    remaining = deadline - time.monotonic()
    if remaining <= 10:
        raise TimeoutError('global deadline before ' + label)
    end = time.monotonic() + min(seconds, remaining - 10)
    write(directory / 'argv.json', argv)
    begin = time.monotonic()
    timed_out = False
    with (directory / 'stdout.log').open('xb') as log, (directory / 'stderr.log').open('xb') as err:
        try:
            process = subprocess.Popen(argv, stdout=log, stderr=err, env=env, start_new_session=True)
            code = process.wait(timeout=max(.01, end - time.monotonic()))
        except subprocess.TimeoutExpired:
            timed_out = True
            finish_child()
            code = process.returncode
        finally:
            group_residue = finish_child()
            settle = time.monotonic() + 3
            remaining_pids = cgroup_pids() - baseline_pids
            while remaining_pids and time.monotonic() < settle:
                time.sleep(.05)
                remaining_pids = cgroup_pids() - baseline_pids
            write(directory / 'exit.json', {'code': process.returncode if process else None,
                'timeout': timed_out, 'pid': process.pid if process else None,
                'group_residue_after_leader': group_residue, 'remaining_phase_pids': sorted(remaining_pids),
                'waited': process is not None and process.poll() is not None,
                'elapsed_s': time.monotonic() - begin})
    if timed_out:
        raise TimeoutError(label)
    if group_residue or remaining_pids:
        raise RuntimeError(label + ' left attributed descendants')
    if code != 0:
        raise RuntimeError(label + ' returned ' + str(code))
    return code
