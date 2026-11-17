#!/usr/bin/env python3
"""Adaptateur Rust du relais commun ; comportement D45 à deux publications."""
import importlib.util
import os
from pathlib import Path
import signal
import socket
import sys
import time

HERE = Path(__file__).resolve().parent
COMMON = HERE / 'publication_relay.py'
if not COMMON.is_file():
    root = Path(__file__).resolve().parents[4]
    assert root / 'docs/codeparsers/17-11-2026/rust-probe-relay/relay.py' == Path(__file__).resolve()
    COMMON = root / 'extension/rag3weaver/scripts/publication_relay.py'
if 'publication_relay' in sys.modules:
    engine = sys.modules['publication_relay']
    assert Path(engine.__file__).resolve() == COMMON.resolve(strict=True)
else:
    spec = importlib.util.spec_from_file_location('publication_relay', COMMON)
    engine = importlib.util.module_from_spec(spec)
    sys.modules['publication_relay'] = engine
    spec.loader.exec_module(engine)

Relay = engine.Relay
SSHTransport = engine.SSHTransport
digest = engine.digest
atomic_json = engine.atomic_json
publish_result = engine.publish_result


def main():
    return engine.relay_main(__file__)


if __name__ == '__main__':
    sys.exit(main())
