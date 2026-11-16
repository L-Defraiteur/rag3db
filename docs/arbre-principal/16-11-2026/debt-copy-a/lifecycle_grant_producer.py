"""Exclusive grant publication; observation supplied by the coordination owner.

This module never contacts a provider itself. The callback must attest the real
provider identity and return the schema-complete receipt. All clocks are those
of lifecycle_protocol on the PC; no deadline is inferred from a message.
"""
import hashlib
import json
import os
from pathlib import Path
import stat
import tempfile
import time
import lifecycle_protocol as protocol


def _ready(path, expected_sha256, expected_nonce):
    info = path.lstat()
    if not stat.S_ISREG(info.st_mode) or info.st_uid != os.getuid():
        raise ValueError('READY must be a regular owned file')
    raw = path.read_bytes()
    if hashlib.sha256(raw).hexdigest() != expected_sha256:
        raise ValueError('READY bytes changed')
    ready = protocol.decode(raw)
    if ready.get('nonce') != expected_nonce:
        raise ValueError('READY nonce changed')
    if ready.get('no_embedding_calls_authorized') is not True:
        raise ValueError('only no-embedding lifecycle coordination is allowed')
    return ready


def _stage(destination, grant):
    fd, name = tempfile.mkstemp(prefix='.grant-', dir=destination.parent)
    path = Path(name)
    try:
        with os.fdopen(fd, 'wb') as out:
            out.write(json.dumps(grant, allow_nan=False, sort_keys=True).encode())
            out.flush()
            os.fsync(out.fileno())
    except BaseException:
        path.unlink()
        raise
    return path


def publish_grant(ready_path, destination, *, expected_sha256, expected_nonce,
                  expected_protocol_sha256, observe_provider, clock=time.monotonic,
                  phase_reader=protocol.read_phase_state, health_margin=5.):
    """Observe only inside admission, preserve existing destination exclusively.

    observe_provider(ready) performs the owner's bounded health/identity checks,
    returning a complete grant. A publication refusal never edits first-red or
    READY. The callback's observations remain the owner's evidence to preserve.
    """
    ready_path, destination = Path(ready_path), Path(destination)
    def pinned_protocol():
        if hashlib.sha256(Path(protocol.__file__).read_bytes()).hexdigest() != expected_protocol_sha256:
            raise ValueError('shared lifecycle protocol differs from admitted hash')
    pinned_protocol()
    ready = _ready(ready_path, expected_sha256, expected_nonce)
    phase = phase_reader(ready)
    now = clock()
    protocol.validate_ready_phase(ready, now, phase)
    if type(health_margin) not in (int, float) or not 0 < health_margin < float('inf'):
        raise ValueError('finite positive health margin required')
    if ready['grant_deadline_monotonic'] - now <= health_margin:
        raise ValueError('READY deadline too close for health')
    if destination.exists() or destination.is_symlink():
        raise FileExistsError('coordination destination already exists')
    grant = observe_provider(ready)
    phase = phase_reader(ready)
    protocol.validate_grant_phase(ready, grant, clock(), phase, expected_sha256)
    staged = _stage(destination, grant)
    try:
        pinned_protocol()
        current = _ready(ready_path, expected_sha256, expected_nonce)
        phase = phase_reader(current)
        protocol.validate_grant_phase(current, grant, clock(), phase, expected_sha256)
        os.link(staged, destination)
    finally:
        staged.unlink()
    return grant
