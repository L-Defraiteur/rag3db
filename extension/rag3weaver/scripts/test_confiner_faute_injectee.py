#!/usr/bin/env python3
"""**The confiner's witnesses that provoke a real fault** (21 Nov. 2026): a real oom_kill in a systemd scope.

They never run on a machine where a graphical session is active (Lucie's rule of 20 Nov. 2026, broken twice in one
night by a run of the whole ordinary file at the tablet, 21 Nov. 05 h 43 min 17 s): they are SKIPPED, said, and the
judge of this suite counts a SKIP as red. The ordinary file, `test_confiner_validation.py`, stays CPU only.
"""
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

ICI = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('validation', ICI / 'confiner_validation.py')
v = importlib.util.module_from_spec(spec)
spec.loader.exec_module(v)


def session_graphique_active(executer=subprocess.run):
    """The first active graphical session of this machine (`loginctl`), said: « session 3 (wayland) »; None if none."""
    try:
        liste = executer(['loginctl', 'list-sessions', '--no-legend'], capture_output=True, text=True, timeout=10)
    except (OSError, subprocess.TimeoutExpired):
        return None
    for ligne in liste.stdout.splitlines():
        champs = ligne.split()
        if not champs:
            continue
        r = executer(['loginctl', 'show-session', champs[0], '-p', 'Type', '-p', 'Active'], capture_output=True, text=True, timeout=10)
        proprietes = dict(l.split('=', 1) for l in r.stdout.splitlines() if '=' in l)
        if proprietes.get('Active') == 'yes' and proprietes.get('Type') in ('x11', 'wayland', 'mir'):
            return f"session {champs[0]} ({proprietes['Type']})"
    return None


class Garde(unittest.TestCase):
    """CPU only: the guard itself."""

    def test_the_ordinary_witnesses_provoke_no_real_oom(self):
        ordinaire = (ICI / 'test_confiner_validation.py').read_text()
        self.assertNotIn('MemoryMax=', ordinaire, 'a real scope with a ceiling in the ordinary file: it belongs here, guarded')

    def test_an_active_graphical_session_is_seen(self):
        def faux(argv, **kw):
            sortie = {'list-sessions': '3 1000 lucied seat0 4520 user tty2 no -\n4 1000 lucied - 5435 manager - no -\n'}.get(argv[1])
            if sortie is None:
                sortie = 'Type=wayland\nActive=yes\n' if argv[2] == '3' else 'Type=unspecified\nActive=yes\n'
            return subprocess.CompletedProcess(argv, 0, sortie, '')
        self.assertEqual(session_graphique_active(faux), 'session 3 (wayland)')

    def test_no_graphical_session_is_none(self):
        def faux(argv, **kw):
            sortie = '4 1000 lucied - 5435 manager - no -\n' if argv[1] == 'list-sessions' else 'Type=unspecified\nActive=yes\n'
            return subprocess.CompletedProcess(argv, 0, sortie, '')
        self.assertIsNone(session_graphique_active(faux))


if __name__ == '__main__':
    unittest.main()
