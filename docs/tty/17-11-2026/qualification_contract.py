"""Admission without network probes; an inactive schema never grants a capability."""
import hashlib
import json
import os
from pathlib import Path
import time


def cpu_allocation():
    assert int(os.environ['POSTE_FILS']) >= 8
    assert len(os.sched_getaffinity(0)) >= 8
    relative = Path('/proc/self/cgroup').read_text().strip().split('0::', 1)[1]
    current = Path('/sys/fs/cgroup') / relative.lstrip('/')
    limits = {}
    while True:
        path = current / 'cpu.max'
        if path.exists():
            quota, period = path.read_text().split()
            limits[str(path)] = [quota, period]
            assert quota == 'max' or int(quota) / int(period) >= 8, limits
        if current == Path('/sys/fs/cgroup'):
            break
        assert Path('/sys/fs/cgroup') in current.parents
        current = current.parent
    assert limits
    return dict(poste_fils=os.environ['POSTE_FILS'], affinity=sorted(os.sched_getaffinity(0)), limits=limits)


def contract(q, config, allocation, expected_sha, phase_end):
    inactive = json.loads((q / 'qualification-contract-inactive.json').read_text())
    assert inactive['state'] == 'INACTIVE' and not config['execution_ready']
    raise RuntimeError('HOLD: common PC actor/TTY policy, cross-host clock binding and real SQL qualification not integrated; no file grant substitutes for physical ARM/poll/heartbeat')
