"""Physical watcher shared by pinned policies. Preparation does not launch it.

CLI: python -B grant_watch_actor.py --watch CONFIG_PATH EXTERNAL_CONFIG_SHA256.
ARMED is emitted inside the first guarded loop; the parent must independently
sample the OS and poll its own child. A file alone is never physical admission.
"""
import hashlib
import importlib.util
import json
import math
import os
import re
from pathlib import Path
import stat
import sys
import tempfile
import time

CLOCK = 'CLOCK_MONOTONIC@luciepc'
LIVE = ('R', 'S', 'D', 'I')
IDENTITY = ('pid', 'ppid', 'start', 'uid', 'cgroup', 'exe', 'exe_sha256',
            'argv', 'script_path', 'script_sha256')


def finite(value):
    if type(value) not in (int, float) or not math.isfinite(value) or value < 0:
        raise ValueError('invalid finite monotonic value')
    return value


def decode(raw):
    def pairs(items):
        value = {}
        for key, item in items:
            if key in value:
                raise ValueError('duplicate JSON field')
            value[key] = item
        return value
    def constant(_):
        raise ValueError('nonfinite JSON value')
    return json.loads(raw, object_pairs_hook=pairs, parse_constant=constant)


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def load_bound(path, sha):
    path = Path(path)
    info = path.lstat()
    if not stat.S_ISREG(info.st_mode) or info.st_size > 1024*1024:
        raise ValueError('bound input not a bounded regular file')
    raw = path.read_bytes()
    if hashlib.sha256(raw).hexdigest() != sha:
        raise ValueError('bound input digest changed')
    return decode(raw)


def exists(path):
    try:
        Path(path).lstat()
        return True
    except FileNotFoundError:
        return False


def _python_snapshot(pid, expected_script_path, watch, proc_root, clock):
    root = Path(proc_root)/str(pid)
    if expected_script_path is not None and not Path(expected_script_path).is_absolute():
        raise ValueError('expected Python script must be absolute')
    def sample():
        fields = (root/'stat').read_text().rsplit(')', 1)[1].split()
        groups = [x[3:] for x in (root/'cgroup').read_text().splitlines() if x.startswith('0::')]
        argv = (root/'cmdline').read_bytes().rstrip(b'\0').decode('utf-8').split('\0')
        if len(groups) != 1 or not argv or not re.fullmatch(r'python(?:[0-9]+(?:\.[0-9]+)*)?', Path(argv[0]).name):
            raise ValueError('Python role cgroup/interpreter unknown')
        index = 2 if len(argv) > 1 and argv[1] == '-B' else 1
        if len(argv) <= index or not Path(argv[index]).is_absolute():
            raise ValueError('Python role must execute an absolute script, no -c/-m/other flags')
        script_path = argv[index]
        if expected_script_path is not None and script_path != str(expected_script_path):
            raise ValueError('Python role script differs BEFORE hashing script content')
        if watch and (len(argv) != index+4 or argv[index+1] != '--watch'):
            raise ValueError('watcher argv unknown')
        return dict(pid=pid, ppid=int(fields[1]), start=fields[19], uid=root.stat().st_uid,
                    cgroup=groups[0], exe=os.readlink(root/'exe'), exe_sha256=digest(root/'exe'),
                    argv=argv, script_path=script_path, script_sha256=digest(script_path), state=fields[0])
    before, after = sample(), sample()
    if any(before[k] != after[k] for k in IDENTITY) or any(x['state'] not in LIVE for x in (before, after)):
        raise ValueError('Python role process changed or not live')
    return dict(after, observed_monotonic=clock())


def process_snapshot(pid, *, proc_root=Path('/proc'), clock=time.monotonic):
    """Strict watcher argv; full stable identity and two living states."""
    return _python_snapshot(pid, None, True, proc_root, clock)


def python_role_snapshot(pid, expected_script_path, *, proc_root=Path('/proc'), clock=time.monotonic):
    """Controller/driver sampler: bind the expected script BEFORE hashing it.

    Only direct Python script invocation, optionally with -B, is accepted.
    No environment is read or returned; launch-environment policy is the parent.
    """
    return _python_snapshot(pid, expected_script_path, False, proc_root, clock)


def validate_armed(receipt, config, configsha, actual, expected_pid):
    if receipt.get('kind') != 'ARMED' or config.get('clock') != CLOCK:
        raise ValueError('ARMED kind/clock invalid')
    for key in ('actor_nonce', 'ready_nonce', 'pilot', 'allocation', 'common_end_monotonic'):
        if receipt.get(key) != config[key]:
            raise ValueError('ARMED config binding '+key)
    if receipt.get('config_sha256') != configsha:
        raise ValueError('ARMED config digest')
    seen = finite(receipt.get('loop_observed_monotonic'))
    if not finite(config['issued_monotonic']) <= seen < finite(config['arm_deadline_monotonic']) <= finite(config['common_end_monotonic']):
        raise ValueError('ARMED observation outside original admission')
    if type(receipt.get('loop_counter')) is not int or receipt['loop_counter'] < 1:
        raise ValueError('ARMED without a guarded loop')
    claimed = receipt.get('process', {})
    for sample in (claimed, actual):
        if type(expected_pid) is not int or sample.get('pid') != expected_pid or sample.get('state') not in LIVE:
            raise ValueError('ARMED PID/liveness')
        if sample.get('ppid') != config['controller']['pid'] or sample.get('uid') != config['authority']['uid'] or sample.get('cgroup') != config['authority']['cgroup']:
            raise ValueError('ARMED parent/uid/cgroup')
        for key in ('script_path', 'script_sha256', 'exe', 'exe_sha256'):
            if sample.get(key) != config['actor'][key]:
                raise ValueError('ARMED pinned executable/script')
        if sample.get('argv') != config['actor']['argv_prefix']+[configsha]:
            raise ValueError('ARMED actual argv')
        if finite(sample.get('observed_monotonic')) < seen:
            raise ValueError('ARMED process observation precedes loop')
    if any(claimed.get(k) != actual.get(k) for k in IDENTITY):
        raise ValueError('ARMED differs from independent physical observation')
    return receipt


def validate_heartbeat(receipt, config, heartbeat, now):
    now = finite(now)
    if heartbeat.get('kind') != 'HEARTBEAT' or heartbeat.get('phase') not in ('WAIT_READY', 'PUBLISHING', 'WAIT_RELEASE'):
        raise ValueError('heartbeat kind/phase')
    for key in ('config_sha256', 'actor_nonce'):
        if heartbeat.get(key) != receipt[key]:
            raise ValueError('heartbeat binding')
    if any(heartbeat.get(k) != receipt['process'][k] for k in ('pid', 'start')):
        raise ValueError('heartbeat process')
    counter = heartbeat.get('loop_counter')
    if type(counter) is not int or counter < receipt['loop_counter']:
        raise ValueError('heartbeat loop counter')
    seen = finite(heartbeat.get('observed_monotonic'))
    if not receipt['loop_observed_monotonic'] <= seen <= now < finite(config['common_end_monotonic']) or now-seen > 10:
        raise ValueError('heartbeat stale/future/outside original scope')
    return heartbeat


def _basic_process(pid):
    root = Path('/proc')/str(pid)
    fields = (root/'stat').read_text().rsplit(')', 1)[1].split()
    groups = [s[3:] for s in (root/'cgroup').read_text().splitlines() if s.startswith('0::')]
    if len(groups) != 1:
        raise ValueError('scope cgroup unknown')
    return dict(pid=pid, start=fields[19], state=fields[0], uid=root.stat().st_uid, cgroup=groups[0])


def basic_process(pid, *, clock=time.monotonic):
    """Public minimal stable OS identity; no executable/script assumptions."""
    before, after = _basic_process(pid), _basic_process(pid)
    keys = ('pid', 'start', 'uid', 'cgroup')
    if any(before[k] != after[k] for k in keys) or any(x['state'] not in LIVE for x in (before, after)):
        raise ValueError('basic process changed or not live')
    return dict(after, observed_monotonic=clock())


def check_scope(config):
    """Sanitized private-authority gate; never return its token or raw bytes."""
    if os.uname().nodename.lower() != 'luciepc':
        raise ValueError('wrong monotonic clock host')
    expected = config['authority']
    def sample():
        path = Path(expected['path'])
        info = path.lstat()
        if not stat.S_ISREG(info.st_mode) or info.st_uid != expected['uid']:
            raise ValueError('authority not regular/owned')
        raw = decode(path.read_bytes())
        public = {k: raw.get(k) for k in ('etat', 'pid', 'debut', 'cgroup', 'echeance', 'script_sha256')}
        return public, (info.st_dev, info.st_ino), _basic_process(expected['pid']), _basic_process(config['controller']['pid'])
    first = sample()
    if exists(config['paths']['first_red']):
        raise ValueError('first red exists')
    second = sample()
    if first[:2] != second[:2]:
        raise ValueError('authority snapshot changed')
    authority = second[0]
    if authority != dict(etat='actif', pid=expected['pid'], debut=expected['start'], cgroup=expected['cgroup'],
                         echeance=config['common_end_monotonic'], script_sha256=expected['script_sha256']):
        raise ValueError('authority not original active allocation')
    for index, descriptor in ((2, expected), (3, config['controller'])):
        for process in (first[index], second[index]):
            if process['start'] != descriptor['start'] or process['state'] not in LIVE or process['uid'] != expected['uid'] or process['cgroup'] != expected['cgroup']:
                raise ValueError('authority/controller process identity changed')
    if exists(config['paths']['first_red']):
        raise ValueError('first red appeared')


def verify_inputs(config_path, configsha, config):
    if load_bound(config_path, configsha) != config:
        raise ValueError('config replaced')
    prepared = load_bound(config['prepared_path'], config['prepared_sha256'])
    if prepared['pilot'] != config['pilot']:
        raise ValueError('prepared pilot changed')
    root = Path(config['prepared_path']).parent
    for name, sha in prepared['files'].items():
        if Path(name).name != name or digest(root/name) != sha:
            raise ValueError('prepared source changed')
    for pin in (config['actor'], config['policy']):
        path = pin.get('script_path', pin.get('path'))
        sha = pin.get('script_sha256', pin.get('sha256'))
        if Path(path).parent != root or digest(path) != sha or prepared['files'].get(Path(path).name) != sha:
            raise ValueError('unprepared actor/policy source')


def load_policy(config):
    path = config['policy']['path']
    root = str(Path(path).parent)
    # Only byte-pinned Q modules are imported; source verification precedes this.
    sys.path.insert(0, root)
    spec = importlib.util.spec_from_file_location('_grant_watch_policy', path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def write_frame(path, value, *, replace=False, guard=lambda: None):
    path = Path(path)
    raw = json.dumps(value, sort_keys=True, separators=(',', ':'), allow_nan=False).encode()
    fd, staging = tempfile.mkstemp(prefix='.'+path.name+'-', dir=path.parent)
    try:
        with os.fdopen(fd, 'wb') as stream:
            stream.write(raw)
            stream.flush()
            os.fsync(stream.fileno())
        guard()
        if replace:
            os.replace(staging, path)
        else:
            os.link(staging, path)
        directory = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY)
        try:
            os.fsync(directory)
        finally:
            os.close(directory)
    finally:
        if os.path.exists(staging):
            os.unlink(staging)


def validate_ready(config, ready, now):
    phase = ready.get('phase', {})
    if ready.get('clock') != CLOCK or ready.get('nonce') != config['ready_nonce'] or ready.get('deadline') != config['common_end_monotonic']:
        raise ValueError('READY clock/nonce/original END')
    if phase.get('pilot') != config['pilot'] or phase.get('allocation') != config['allocation'] or phase.get('authority') != config['authority'] or phase.get('first_red_path') != config['paths']['first_red']:
        raise ValueError('READY phase binding')
    observed, due = finite(ready.get('ready_observed_monotonic')), finite(ready.get('grant_deadline_monotonic'))
    if not config['issued_monotonic'] <= observed <= now < due <= min(observed+120, config['common_end_monotonic']-500):
        raise ValueError('READY admission expired/extended')


def run_actor(config_path, configsha, *, clock=time.monotonic, sleeper=time.sleep,
              snapshot=process_snapshot, scope_check=check_scope, policy_loader=load_policy,
              inputs_check=verify_inputs):
    config = load_bound(config_path, configsha)
    paths = config['paths']
    pid = os.getpid()
    receipt = None
    counter = 0
    phase = 'WAIT_READY'
    def guard():
        inputs_check(config_path, configsha, config)
        scope_check(config)
        now = finite(clock())
        if config.get('clock') != CLOCK or not finite(config['issued_monotonic']) <= now < finite(config['common_end_monotonic']):
            raise ValueError('actor outside original common clock/deadline')
        if exists(paths['first_red']):
            raise ValueError('first red exists')
        if receipt is not None:
            actual = snapshot(pid)
            validate_armed(receipt, config, configsha, actual, pid)
        return now
    try:
        guard()
        for key in ('ready', 'grant', 'armed', 'grant_ack', 'result', 'release', 'heartbeat'):
            if exists(paths[key]):
                raise ValueError('actor cannot arm after existing '+key)
        policy = policy_loader(config)
        policy.preflight(config)
        while True:
            now = guard()
            counter += 1
            if receipt is None:
                if now >= config['arm_deadline_monotonic']:
                    raise ValueError('actor exec/first loop missed arm deadline')
                receipt = dict(kind='ARMED', config_sha256=configsha, actor_nonce=config['actor_nonce'],
                               ready_nonce=config['ready_nonce'], pilot=config['pilot'], allocation=config['allocation'],
                               common_end_monotonic=config['common_end_monotonic'], process=snapshot(pid),
                               loop_counter=counter, loop_observed_monotonic=now)
                validate_armed(receipt, config, configsha, receipt['process'], pid)
                def arm_guard():
                    if guard() >= config['arm_deadline_monotonic']:
                        raise ValueError('arm deadline before exclusive link')
                    if exists(paths['ready']) or exists(paths['grant']):
                        raise ValueError('READY/grant appeared before physical ARM')
                write_frame(paths['armed'], receipt, guard=arm_guard)
            hb = dict(kind='HEARTBEAT', config_sha256=configsha, actor_nonce=config['actor_nonce'],
                      pid=pid, start=receipt['process']['start'], loop_counter=counter, phase=phase,
                      observed_monotonic=clock())
            validate_heartbeat(receipt, config, hb, clock())
            write_frame(paths['heartbeat'], hb, replace=True, guard=guard)
            if phase == 'WAIT_READY' and exists(paths['ready']):
                ready_raw = Path(paths['ready']).read_bytes()
                ready_sha = hashlib.sha256(ready_raw).hexdigest()
                ready = decode(ready_raw)
                validate_ready(config, ready, guard())
                phase = 'PUBLISHING'
                hb.update(phase=phase, observed_monotonic=clock())
                write_frame(paths['heartbeat'], hb, replace=True, guard=guard)
                # Owner policy rechecks READY/phase immediately before bounded GET and exclusive grant link.
                grant = policy.publish_ready(config, paths['ready'], ready_sha, config['ready_nonce'])
                guard()
                if digest(paths['ready']) != ready_sha or load_bound(paths['grant'], digest(paths['grant'])) != grant:
                    raise ValueError('policy result differs from immutable READY/grant')
                ack = dict(kind='GRANT', config_sha256=configsha, actor_nonce=config['actor_nonce'],
                           ready_nonce=config['ready_nonce'], ready_sha256=ready_sha,
                           armed_sha256=digest(paths['armed']), grant_sha256=digest(paths['grant']), observed_monotonic=clock())
                write_frame(paths['grant_ack'], ack, guard=guard)
                phase = 'WAIT_RELEASE'
            if exists(paths['release']):
                release = decode(Path(paths['release']).read_bytes())
                expected = dict(kind='DRIVER_FINISHED', actor_nonce=config['actor_nonce'],
                                config_sha256=configsha, pilot=config['pilot'], allocation=config['allocation'])
                if phase != 'WAIT_RELEASE' or any(release.get(k) != v for k, v in expected.items()):
                    raise ValueError('release before grant or unbound controller release')
                result = dict(kind='RELEASED', config_sha256=configsha, actor_nonce=config['actor_nonce'], observed_monotonic=clock())
                write_frame(paths['result'], result, guard=guard)
                return result
            sleeper(min(.2, max(0., config['common_end_monotonic']-clock())))
    except BaseException as error:
        # Preserve the original refusal even if journaling itself fails. No raw authority/token.
        refusal = dict(kind='REFUSED', config_sha256=configsha, actor_nonce=config.get('actor_nonce'),
                       error_type=type(error).__name__, phase=phase, observed_monotonic=clock())
        for destination in (paths['first_red'], paths['result']):
            try:
                write_frame(destination, refusal)
            except BaseException:
                pass
        raise


def main(argv=None):
    args = sys.argv[1:] if argv is None else argv
    if len(args) != 3 or args[0] != '--watch':
        raise ValueError('expected --watch CONFIG_PATH CONFIG_SHA256')
    run_actor(args[1], args[2])


if __name__ == '__main__':
    main()
