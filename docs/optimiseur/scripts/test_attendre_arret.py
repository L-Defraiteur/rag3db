"""Filets CPU de l'arrêt du privé ; aucune commande de service ni aucun signal."""
import ast
import importlib.util
import os
from pathlib import Path
import types
import unittest
from unittest.mock import patch


class Horloge:
    def __init__(self):
        self.t = 0.0

    def maintenant(self):
        return self.t

    def dormir(self, duree):
        self.t += duree


def ancien_attendre(pid, starttime, *, deadline, processus, port, horloge, dormir):
    """Exécute le vrai bloc historique, SHA d97e71c8…, sans /proc ni ss réels."""
    source = Path(os.environ['WRAPPER_ANCIEN']).read_text()
    arbre = ast.parse(source)
    branche = next(n for n in arbre.body if isinstance(n, ast.If) and isinstance(n.test, ast.Compare)
                   and isinstance(n.test.left, ast.Name) and n.test.left.id == 'mode')
    debut = next(i for i, n in enumerate(branche.body) if isinstance(n, ast.For))
    bloc = ast.Module(body=branche.body[debut:debut + 2], type_ignores=[])

    class FauxProc:
        def __truediv__(self, _):
            return self

        def exists(self):
            return processus(pid, starttime) != 'absent'

        def read_text(self):
            etat = 'Z' if processus(pid, starttime) == 'zombie' else 'S'
            return f'{pid} (prive) {etat}'

    faux_path = types.SimpleNamespace(Path=lambda _: FauxProc())
    espace = {'pid': pid, 'pathlib': faux_path, 'time': types.SimpleNamespace(sleep=dormir),
              'listening': lambda _: 'listener' if port(7882, deadline - horloge()) else ''}
    exec(compile(bloc, '<bloc historique arrêt PID puis assertion port>', 'exec'), espace)


if 'WRAPPER_ANCIEN' in os.environ:
    attendre = ancien_attendre
    ArretRefuse = RuntimeError
else:
    spec = importlib.util.spec_from_file_location('attendre_arret', Path(__file__).with_name('attendre_arret.py'))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    attendre = module.attendre_arret
    ArretRefuse = module.ArretRefuse


class AttenteArret(unittest.TestCase):
    def setUp(self):
        self.h = Horloge()

    def jouer(self, processus, port, budget=3):
        return attendre(123, '456', deadline=budget, processus=processus, port=port,
                        horloge=self.h.maintenant, dormir=self.h.dormir)

    def test_port_retarde_apres_fin_pid(self):
        # Reproduit le rouge : PID déjà absent, ss conserve son listener 1 s.
        self.jouer(lambda *_: 'absent', lambda *_: {123} if self.h.t < 1 else set())
        self.assertEqual(self.h.t, 1)

    def test_budget_commun_processus_et_port(self):
        # Le PID consomme 2 s ; le listener ne doit pas recevoir un second budget.
        with self.assertRaisesRegex(TimeoutError, 'expiration.*port'):
            self.jouer(lambda *_: 'actif' if self.h.t < 2 else 'absent', lambda *_: {123})
        self.assertEqual(self.h.t, 3)

    def test_expiration_pid_encore_actif(self):
        with self.assertRaisesRegex(TimeoutError, 'PID.*actif'):
            self.jouer(lambda *_: 'actif', lambda *_: set())
        self.assertEqual(self.h.t, 3)

    def test_port_repris_par_un_autre_pid(self):
        with self.assertRaisesRegex(ArretRefuse, 'autre PID.*789'):
            self.jouer(lambda *_: 'absent', lambda *_: {789})
        self.assertEqual(self.h.t, 0)

    def test_reprise_du_port_pendant_attente(self):
        with self.assertRaisesRegex(ArretRefuse, 'autre PID.*789'):
            self.jouer(lambda *_: 'absent', lambda *_: {123} if self.h.t < 1 else {789})
        self.assertEqual(self.h.t, 1)

    def test_pid_reutilise_meme_si_port_libre(self):
        with self.assertRaisesRegex(ArretRefuse, 'identité.*PID'):
            self.jouer(lambda *_: 'remplace', lambda *_: set())

    def test_zombie_et_port_libre(self):
        self.jouer(lambda *_: 'zombie', lambda *_: set())
        self.assertEqual(self.h.t, 0)

    def test_proprietaire_du_port_inconnu_refuse(self):
        with self.assertRaisesRegex(ArretRefuse, 'propriétaire inconnu'):
            self.jouer(lambda *_: 'absent', lambda *_: {None})

    def test_erreur_observation_ne_vaut_pas_absence(self):
        def erreur(*_):
            raise PermissionError('lecture refusée')
        with self.assertRaises(PermissionError):
            self.jouer(erreur, lambda *_: set())

    def test_observation_tardive_ne_devient_pas_verte(self):
        def port_lent(*_):
            self.h.t += 4
            return set()
        with self.assertRaisesRegex(TimeoutError, 'hors délai'):
            self.jouer(lambda *_: 'absent', port_lent)


@unittest.skipIf('WRAPPER_ANCIEN' in os.environ, 'sondes propres au correctif')
class SondesCPU(unittest.TestCase):
    def test_identite_du_processus_cpu_courant(self):
        pid = os.getpid()
        start = (Path('/proc') / str(pid) / 'stat').read_text().rsplit(')', 1)[1].split()[19]
        self.assertEqual(module.etat_processus(pid, start), 'actif')
        self.assertEqual(module.etat_processus(pid, 'autre démarrage'), 'remplace')

    def test_ss_identifie_tous_les_occupants_et_son_timeout(self):
        sortie = 'LISTEN 0 128 127.0.0.1:7882 *:* users:(("prive",pid=123,fd=8),("autre",pid=789,fd=9))\n'
        with patch.object(module.subprocess, 'run', return_value=types.SimpleNamespace(stdout=sortie)) as run:
            self.assertEqual(module.occupants_port(7882, .75), {123, 789})
        self.assertEqual(run.call_args.kwargs['timeout'], .75)

    def test_ss_sans_proprietaire_reste_inconnu(self):
        with patch.object(module.subprocess, 'run', return_value=types.SimpleNamespace(stdout='LISTEN 0 128 *:7882 *:*\n')):
            self.assertEqual(module.occupants_port(7882, 1), {None})

    def test_echec_ss_ne_devient_pas_port_libre(self):
        with patch.object(module.subprocess, 'run', side_effect=module.subprocess.CalledProcessError(1, 'ss')):
            with self.assertRaises(module.subprocess.CalledProcessError):
                module.occupants_port(7882, 1)


if __name__ == '__main__':
    unittest.main()
