#!/usr/bin/env python3
"""Admission after actual poste; preserve the original allocation deadline."""
import json
import math
import os
from pathlib import Path
import sys
import time

Q = Path(__file__).parent
config = json.loads((Q / 'prepared.json').read_text())
now = time.monotonic()
duration = math.floor(config['original_end'] - now) - 3
assert duration > 60, 'original allocation exhausted before new receipt'
(Q / 'remaining-after-poste.json').write_text(json.dumps(dict(
    observed=now, duration=duration, original_start=config['original_start'],
    original_end=config['original_end'])))
os.execv(sys.executable, [sys.executable, str(Q / 'confiner_validation.py'), 'lancer',
    '--delai', str(duration), '--preuves', config['scope'], '--', sys.executable, str(Q / 'driver.py')])
