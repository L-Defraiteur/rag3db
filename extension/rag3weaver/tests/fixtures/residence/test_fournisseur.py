"""Filets du fournisseur factice ; ne qualifient pas le runtime de résidence."""
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
import unittest
import urllib.error
import urllib.request

SCRIPT = Path(__file__).with_name('fournisseur.py')


class FournisseurCPU(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.process = None
        self.streams = []

    def tearDown(self):
        if self.process is not None:
            if self.process.poll() is None:
                os.killpg(self.process.pid, signal.SIGTERM)
                try:
                    self.process.wait(timeout=3)
                except subprocess.TimeoutExpired:
                    os.killpg(self.process.pid, signal.SIGKILL)
                    self.process.wait(timeout=3)
            self.assertIsNotNone(self.process.poll(), 'le processus fixture doit être arrêté')
        for f in self.streams:
            f.close()
        self.tmp.cleanup()

    def spawn(self, *extra, env=None):
        self.stdout = (self.root / 'stdout').open('x')
        self.stderr = (self.root / 'stderr').open('x')
        self.streams = [self.stdout, self.stderr]
        environment = dict(os.environ)
        for key in ('RAG3WEAVER_ACTIVATION_PROFIL', 'RAG3WEAVER_ACTIVATION_PERIPHERIQUE', 'RAG3WEAVER_ACTIVATION_IDENTITE'):
            environment.pop(key, None)
        environment.update(env or {})
        self.process = subprocess.Popen([sys.executable, str(SCRIPT), '--journal', str(self.root / 'events.jsonl'),
                                         '--generation', 'generation-cpu-1', *extra],
                                        stdout=self.stdout, stderr=self.stderr, start_new_session=True, env=environment)

    def events(self):
        p = self.root / 'events.jsonl'
        return [json.loads(line) for line in p.read_text().splitlines()] if p.exists() else []

    def wait(self, event):
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            found = [v for v in self.events() if v['event'] == event]
            if found:
                return found[-1]
            if self.process.poll() is not None:
                self.fail(f'fixture terminée avant {event}: {(self.root / "stderr").read_text()}')
            time.sleep(.005)
        self.fail(f'événement {event} absent')

    def request(self, path, body=None):
        address = self.wait('pret')['adresse']
        data = json.dumps(body).encode() if body is not None else None
        req = urllib.request.Request('http://' + address + path, data=data,
                                     headers={'Content-Type': 'application/json'})
        with urllib.request.urlopen(req, timeout=2) as r:
            return json.load(r)

    def test_barrieres_avant_ecoute_et_identite_factice(self):
        load = self.root / 'charge.go'
        warm = self.root / 'chauffe.go'
        self.spawn('--liberer-chargement', str(load), '--liberer-chauffe', str(warm))
        self.wait('chargement_attente')
        self.assertFalse(any(e['event'] == 'pret' for e in self.events()))
        load.touch()
        self.wait('chauffe_attente')
        self.assertFalse(any(e['event'] == 'pret' for e in self.events()))
        warm.touch()
        ready = self.wait('pret')
        identity = self.request('/sante')
        self.assertTrue(identity['factice'])
        self.assertEqual(identity['modele'], 'fixture-residence-384')
        self.assertEqual(identity['dim'], 384)
        self.assertNotIn('activation', identity)
        self.assertEqual(ready['pid'], self.process.pid)
        self.assertEqual(sum(e['event'] == 'constructeur_demarre' for e in self.events()), 1)
        self.assertTrue((self.root / 'stdout').read_text().startswith('adresse liée : 127.0.0.1:'))

    def test_reponses_deterministes_et_deux_clients_http(self):
        self.spawn()
        first = self.request('/embed', {'texts': ['mémoire utile', 'autre texte']})
        second = self.request('/embed', {'texts': ['mémoire utile']})
        self.assertEqual(len(first['vectors']), 2)
        self.assertTrue(all(len(v) == 384 for v in first['vectors']))
        self.assertEqual(first['vectors'][0], second['vectors'][0])
        self.assertEqual([e['textes'] for e in self.events() if e['event'] == 'embed_termine'], [2, 1])
        self.assertTrue(all(e['factice'] for e in self.events()))
        # Deux requêtes HTTP ne prouvent PAS le partage d'activation du runtime.

    def test_erreur_de_chauffe_visible_sans_pret(self):
        self.spawn('--echec-chauffe', 'forme refusée par fixture')
        self.assertEqual(self.process.wait(timeout=5), 3)
        self.assertFalse(any(e['event'] == 'pret' for e in self.events()))
        error = [e for e in self.events() if e['event'] == 'erreur']
        self.assertEqual(error[0]['phase'], 'chauffe')
        self.assertEqual(error[0]['cause'], 'forme refusée par fixture')
        self.assertEqual(self.events()[-1]['event'], 'arrete')

    def test_attente_bornee_et_requetes_invalides(self):
        self.spawn()
        for body in ({'texts': []}, {'texts': [12]}, {'texts': ['x'] * 9}):
            with self.assertRaises(urllib.error.HTTPError) as e:
                self.request('/embed', body)
            self.assertEqual(e.exception.code, 400)
            e.exception.close()
        with self.assertRaises(urllib.error.HTTPError) as e:
            self.request('/rerank', {'texts': ['x']})
        self.assertEqual(e.exception.code, 404)
        e.exception.close()
        self.assertFalse(any(e['event'] == 'embed_commence' for e in self.events()))

    def test_identite_activation_exacte_et_chauffe_inconnue(self):
        self.spawn(env={'RAG3WEAVER_ACTIVATION_PROFIL': 'profil-cpu',
                        'RAG3WEAVER_ACTIVATION_PERIPHERIQUE': 'cpu-fixture',
                        'RAG3WEAVER_ACTIVATION_IDENTITE': 'empreinte-declaree'})
        identity = self.request('/sante')
        self.assertEqual(identity['activation'], {'profil': 'profil-cpu', 'peripherique': 'cpu-fixture',
                                                 'recette': 'empreinte-declaree'})
        self.assertNotIn('chauffe', identity)
        self.assertTrue(identity['factice'])

    def test_identite_partielle_non_inventee(self):
        self.spawn(env={'RAG3WEAVER_ACTIVATION_PROFIL': 'profil-cpu',
                        'RAG3WEAVER_ACTIVATION_IDENTITE': 'empreinte-declaree'})
        self.assertNotIn('activation', self.request('/sante'))

    def test_gate_non_liberee_expire(self):
        self.spawn('--liberer-chauffe', str(self.root / 'absente.go'), '--budget-pret-ms', '200')
        self.assertEqual(self.process.wait(timeout=5), 4)
        self.assertFalse(any(e['event'] == 'pret' for e in self.events()))
        self.assertEqual([e['phase'] for e in self.events() if e['event'] == 'erreur'], ['chauffe'])


if __name__ == '__main__':
    unittest.main(verbosity=2)
