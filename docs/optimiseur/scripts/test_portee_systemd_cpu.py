#!/usr/bin/env python3
"""Preuve réelle CPU seulement : parent sortant, enfant setsid arrêté par son unité.

Ce probe ne passe pas par une acquisition GPU et ne lance que Python/sleep.
"""
import datetime
import json
from pathlib import Path
import subprocess
import time
from fenetre_card0 import Systemd, SLICE

root = Path.home() / '.cache/optimiseur-notes/card0-20261113/preparation'
unite = 'cx-card0-20261113-preuve-cpu-v2.service'
pidfile = root / 'enfant-cpu-v2.pid'
preuve = root / 'preuve-descendants-v2.json'
manager = Systemd()
assert not pidfile.exists() and not preuve.exists(), 'preuves existantes : pas de rejeu inchangé'
assert manager.info(unite).get('LoadState') == 'not-found', 'nom CPU déjà occupé'
code = ("import subprocess,pathlib,time; "
        "p=subprocess.Popen(['/usr/bin/sleep','60'],start_new_session=True); "
        f"pathlib.Path({str(pidfile)!r}).write_text(str(p.pid)); time.sleep(1)")
subprocess.run(['systemd-run', '--user', '--service-type=exec', '--unit=' + unite,
                '--slice=' + SLICE, '--property=Restart=no', '--property=KillMode=control-group',
                '--property=RuntimeMaxSec=2', '--property=TimeoutStopSec=1s',
                '--property=ExecCondition=/usr/bin/true', '/usr/bin/python3', '-c', code],
               check=True, timeout=10)
avant = manager.info(unite)
fin = time.monotonic() + 10
while not pidfile.exists():
    assert time.monotonic() < fin, 'enfant CPU non déclaré'
    time.sleep(.05)
pid = int(pidfile.read_text())
while True:
    apres = manager.info(unite)
    p = Path('/proc') / str(pid)
    absent = not p.exists() or (p / 'stat').read_text().rsplit(')', 1)[1].split()[0] == 'Z'
    if absent and not manager.pids(apres) and apres.get('ActiveState') not in ['active', 'activating']:
        break
    assert time.monotonic() < fin, 'descendant CPU encore présent'
    time.sleep(.1)
preuve.write_text(json.dumps({'date': datetime.datetime.now().astimezone().isoformat(),
    'unite_cpu': unite, 'enfant_setsid': pid, 'avant': avant, 'apres': apres,
    'descendants_restants': [], 'gpu_utilise': False, 'verdict': 'VERT'}, indent=2) + '\n')
print(preuve.read_text())
