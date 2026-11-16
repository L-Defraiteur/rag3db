"""Strict pre/post provenance inside the live, already acquired Common scope."""
import json
import os
from pathlib import Path
import subprocess
import sys
import time
from strict_target_guard import scan, snapshot, require_provenance, digest

worktree, target, evidence = map(Path, sys.argv[1:4])
phase = sys.argv[4]
q = Path(os.environ['TTY_CONTROLLER_DIR'])
config = json.loads((q / 'prepared.json').read_text())
a = json.loads((q / 'allocation.json').read_text())
subprocess.run([sys.executable, str(q / 'confiner_validation.py'), 'verifier'], check=True)
record = dict(phase=phase)
try:
    assert time.monotonic() < a['end']
    old = json.loads((q / 'previous-autorite.json').read_text())
    assert old['etat'] == 'termine'
    assert digest(q / 'target-ledger.json') == config['ledger_sha']
    record['scan'] = scan('/proc', [worktree, target])
    assert record['scan']['found'] == []
    if phase == 'before':
        record['target'] = require_provenance(target, json.loads((q / 'target-ledger.json').read_text()))
    else:
        record['target'] = snapshot(target)
        (evidence / 'target-final.json').write_text(json.dumps(record['target'], indent=2))
    receipt = Path(os.environ['RAG3WEAVER_VALIDATION_RECU'])
    (evidence / 'authority-path.json').write_text(json.dumps(dict(path=str(receipt), end=a['end'])))
    assert time.monotonic() < a['end']
finally:
    (evidence / ('profile-' + phase + '.json')).write_text(json.dumps(record, indent=2))
