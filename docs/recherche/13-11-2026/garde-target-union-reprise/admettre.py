#!/usr/bin/env python3
"""After actual poste: receipt is bounded by the ORIGINAL absolute deadline."""
from pathlib import Path
import json, math, os, sys, time
START = 914593.915525602
ORIGINAL_END = 919393.915525602

def remaining(now):
    return math.floor(ORIGINAL_END - now) - 3

def main():
    q = Path(__file__).parent
    s = Path('/home/lucied/.cache/rag3weaver-build/cx-confinement-garde-target-union-v2')
    duration = remaining(time.monotonic())
    assert duration >= 3, 'original deadline exhausted, refuse without receipt'
    (q / 'remaining-after-poste.json').write_text(json.dumps(dict(initial=START, deadline=ORIGINAL_END, duration=duration, observed=time.monotonic())))
    os.execv(sys.executable, [sys.executable, str(q / 'confiner_validation.py'), 'lancer', '--delai', str(duration), '--preuves', str(s), '--', 'python3', str(q / 'driver.py')])

if __name__ == '__main__': main()
