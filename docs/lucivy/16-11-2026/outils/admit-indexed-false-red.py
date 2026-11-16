"""Fresh path admission after real poste; this does not grant a run by itself."""
import json
import hashlib
import stat
import math
import os
from pathlib import Path
import runpy
import sys
import time

start = time.monotonic()
package = Path(__file__).absolute().parent
# No resolve(): the reviewed checker rejects symlink components explicitly.
pilot = package / 'qualify-indexed-false-red.py'
for location in [package, pilot, package / 'preflight.json', Path(__file__).absolute()]:
    assert location.is_absolute() and '..' not in location.parts
    current = Path('/')
    for part in location.parts[1:]:
        current /= part
        info = current.lstat()
        assert not stat.S_ISLNK(info.st_mode), f'symlink refused: {current}'
    assert location.lstat().st_uid == os.getuid()
manifest = json.loads((package / 'preflight.json').read_text())
assert hashlib.sha256(pilot.read_bytes()).hexdigest() == manifest['copy_hashes'][pilot.name]
functions = runpy.run_path(str(pilot), run_name='reviewed_path_checks')
admission = functions['check_preflight'](str(package / 'preflight.json'), scope_exists=False)
assert admission['package'] == str(package)
scope = Path(admission['scope'])
scope.mkdir(exist_ok=False)
record = dict(nonce=admission['nonce'], start=start, deadline=start + 600, uid=os.getuid())
with (scope / 'scope-provenance.json').open('x') as file:
    json.dump(record, file, indent=2)
    file.flush(); os.fsync(file.fileno())
remaining = math.floor(record['deadline'] - time.monotonic()) - 2
assert remaining > 5
argv = [sys.executable, str(package / 'support/confiner_validation.py'), 'lancer',
        '--delai', str(remaining), '--preuves', str(scope), '--', sys.executable, str(pilot),
        '--worktree', admission['worktree'], '--target', admission['target'],
        '--output', admission['output'], '--preflight', str(package / 'preflight.json')]
os.execv(sys.executable, argv)
