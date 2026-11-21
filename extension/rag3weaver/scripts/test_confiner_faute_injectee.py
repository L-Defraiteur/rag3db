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



class PorteeReelle(unittest.TestCase):
    """A real systemd scope with a ceiling, and a real oom_kill in it: never under an active graphical session."""

    @classmethod
    def setUpClass(cls):
        session = session_graphique_active()
        if session:
            raise unittest.SkipTest(f'SKIP : {session} active sur ce poste — un vrai oom_kill ne se joue pas sous un écran '
                                    '(règle du 20 nov. 2026) ; à jouer sur un poste sans session graphique, compté rouge sinon')

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix='test-validation-')
        self.p = Path(self.tmp.name)

    def tearDown(self):
        self.tmp.cleanup()

    def test_vraie_portee_systemd_crevee_dit_l_enveloppe(self):
        # La vraie portée : systemd-run --user --scope -p MemoryMax=100M, un enfant qui en demande 400.
        # Pas de saut : sans systemd-run --user, le témoin est rouge.
        r = subprocess.run(['systemd-run', '--user', '--scope', '-q', '--', 'true'], capture_output=True)
        self.assertEqual(r.returncode, 0, 'systemd-run --user --scope indisponible : ' + r.stderr.decode())
        helper = self.p / 'bus-factice'
        helper.write_text('#!' + sys.executable + '\nimport sys,subprocess\ni=sys.argv.index("--")\nsys.exit(subprocess.call(sys.argv[i+1:]))\n')
        helper.chmod(0o700)
        preuves = self.p / 'preuves-portee'
        pilote = self.p / 'pilote.py'
        pilote.write_text(
            'import importlib.util,sys\nfrom unittest.mock import patch\n'
            f'spec=importlib.util.spec_from_file_location("validation",{str(ICI / "confiner_validation.py")!r})\n'
            'v=importlib.util.module_from_spec(spec);spec.loader.exec_module(v)\n'
            f'with patch.object(v,"poste_tenu",return_value=1),patch.object(v.shutil,"which",return_value={str(helper)!r}):\n'
            f'    sys.exit(v.superviser([sys.executable,"-c","b=[bytearray(10**6) for _ in range(400)]"],30,{str(preuves)!r}))\n')
        r = subprocess.run(['systemd-run', '--user', '--scope', '-q', '-p', 'MemoryMax=100M', '-p', 'MemorySwapMax=0',
                            '--', sys.executable, str(pilote)], capture_output=True, timeout=60)
        dossier, = preuves.iterdir()
        verdict = json.loads((dossier / 'verdict.json').read_text())
        self.assertEqual(r.returncode, 1, r.stderr)
        self.assertIn('enveloppe mémoire crevée', verdict['erreur'] or '')
        self.assertGreaterEqual(verdict.get('enveloppe', {}).get('oom_kill', 0), 1)
        self.assertEqual(verdict['restes'], [])


if __name__ == '__main__':
    unittest.main()
