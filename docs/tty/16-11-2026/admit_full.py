"""Run after the real poste; preserve the unique outer allocation deadline."""
import json
import math
import os
from pathlib import Path
import sys
import time

q = Path(__file__).parent
config = json.loads((q / 'prepared.json').read_text())
a = json.loads((q / 'allocation.json').read_text())
remaining = math.floor(a['end'] - time.monotonic()) - 3
assert 60 < remaining <= config['budget']
(q / 'remaining-after-poste.json').write_text(json.dumps(dict(remaining=remaining, original_end=a['end'])))
os.chdir(q)
os.execve(sys.executable, [sys.executable, str(q / 'confiner_validation.py'), 'lancer',
    '--delai', str(remaining), '--preuves', config['scope'], '--',
    'bash', str(q / config.get('driver', 'check-english-aliases-full.sh'))], dict(os.environ, TTY_CONTROLLER_DIR=str(q)))
