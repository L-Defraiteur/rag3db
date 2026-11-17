"""Stdlib-only import boundary, loaded by its literal SHA in approved entries.

Source imports are preparation only. A copied Q entry requires the independently
supplied prepared pin, and verifies its entire controller closure before any
controller code is evaluated. This module supplies no lifecycle or backend.
"""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import sys


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def option(name):
    values = [sys.argv[index + 1] for index, value in enumerate(sys.argv[:-1]) if value == name]
    assert len(values) <= 1, 'duplicate bootstrap option'
    return values[0] if values else None


def verify_entry(entry, *, source_entry):
    entry = Path(entry)
    assert entry.resolve(strict=True) == entry
    if source_entry is not None and entry == Path(source_entry):
        # This audited repository path is for imports/prepare. Runtime methods
        # separately require Q; this does not grant phase execution in source.
        return None
    proofs = option('--proofs')
    selected = option('--prepared')
    assert selected is not None or proofs is not None, 'Q entry needs prepared path'
    prepared = Path(selected) if selected is not None else Path(proofs) / 'prepared.json'
    expected = option('--pin')
    assert expected is not None and re.fullmatch('[0-9a-f]{64}', expected), 'independent prepared pin required'
    assert prepared.is_absolute() and prepared.resolve(strict=True) == prepared
    assert prepared.is_file() and prepared.stat().st_uid == os.getuid()
    assert digest(prepared) == expected, 'prepared pin changed before import'
    pair = json.loads(prepared.read_text())
    control = Path(pair['control'])
    assert control.resolve(strict=True) == control == entry.parent
    layout = pair['publication_layout']
    assert str(prepared) == layout['prepared']
    assert entry.name in layout['entries'].values()
    assert {'publication_loader.py', 'publication_relay.py', entry.name} <= set(pair['controllers'])
    snapshots = {}
    for name, sha in pair['controllers'].items():
        assert re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9._-]{0,95}', name)
        assert isinstance(sha, str) and re.fullmatch('[0-9a-f]{64}', sha)
        path = control / name
        assert path.resolve(strict=True) == path and path.is_file()
        payload = path.read_bytes()
        assert path.stat().st_uid == os.getuid() and hashlib.sha256(payload).hexdigest() == sha, 'controller changed before import'
        snapshots[str(path)] = payload
    return pair, expected, snapshots


def load_module(name, path, *, context=None, expected_sha=None):
    """Exécuter les octets vérifiés, jamais rouvrir après leur empreinte."""
    path = Path(path)
    assert path.resolve(strict=True) == path
    payload = context[2][str(path)] if context is not None else path.read_bytes()
    sha = hashlib.sha256(payload).hexdigest()
    if expected_sha is not None:
        assert sha == expected_sha, 'controller bytes differ at import'
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    # Dataclasses et imports récursifs voient uniquement le module dont les
    # octets viennent d'être vérifiés. Aucun corps n'est lu par SourceFileLoader.
    module._load_verified_module = load_module
    sys.modules[name] = module
    try:
        exec(compile(payload, str(path), 'exec'), module.__dict__)
    except BaseException:
        sys.modules.pop(name, None)
        raise
    module.__verified_source_sha256__ = sha
    return module
