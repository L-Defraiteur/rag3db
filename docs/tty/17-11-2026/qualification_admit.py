"""After real poste: validate actual CPU allocation, enter the remaining Common."""
import json
import math
import os
from pathlib import Path
import sys
import time
from qualification_contract import cpu_allocation

q = Path(__file__).parent
c = json.loads((q/'prepared.json').read_text())
a = json.loads((q/'allocation.json').read_text())
(q/'cpu-after-poste.json').write_text(json.dumps(cpu_allocation(), indent=2))
remaining = math.floor(a['end'] - time.monotonic()) - 15
assert 60 < remaining <= c['budget']
(q/'remaining-after-poste.json').write_text(json.dumps(dict(remaining=remaining, original_end=a['end'])))
os.chdir(q)
os.execve(sys.executable, [sys.executable, str(q/'confiner_validation.py'), 'lancer',
    '--delai', str(remaining), '--preuves', c['scope'], '--', sys.executable,
    str(q/'qualification_driver.py')], dict(os.environ, TTY_CONTROLLER_DIR=str(q)))
