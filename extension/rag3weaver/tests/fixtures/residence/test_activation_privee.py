"""Filets CPU de la barrière de banc, pas validation d'un modèle réel."""
import hashlib
import json
import os
from pathlib import Path
import signal
import socket
import subprocess
import sys
import tempfile
import time
import unittest

D = Path(__file__).resolve().parent


class BarrierePrivee(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.process = None
        self.streams = []
        self.gate = self.root / 'go.json'
        self.events = self.root / 'barriere.jsonl'
        fixture = D / 'fournisseur.py'
        self.plan = dict(model='fixture-residence-384', device='cpu-fixture',
                         artifacts={'provider': {'path': str(fixture), 'sha256': hashlib.sha256(fixture.read_bytes()).hexdigest()}},
                         provider_args=['--journal', str(self.root / 'fixture.jsonl'), '--generation', 'test-wrapper'])
        with socket.socket() as s:
            s.bind(('127.0.0.1', 0))
            self.adresse = '127.0.0.1:' + str(s.getsockname()[1])

    def tearDown(self):
        if self.process is not None:
            if self.process.poll() is None:
                os.killpg(self.process.pid, signal.SIGTERM)
                try:
                    self.process.wait(timeout=3)
                except subprocess.TimeoutExpired:
                    os.killpg(self.process.pid, signal.SIGKILL)
                    self.process.wait(timeout=3)
        for f in self.streams:
            f.close()
        self.tmp.cleanup()

    def spawn(self, budget=5000):
        (self.root / 'plan.json').write_text(json.dumps(self.plan))
        env = dict(os.environ, RAG3WEAVER_ACTIVATION_PROFIL='fixture', RAG3WEAVER_ACTIVATION_PERIPHERIQUE='cpu-fixture',
                   RAG3WEAVER_ACTIVATION_IDENTITE='recette-cpu', RAG3WEAVER_EMBED_MODEL='fixture-residence-384',
                   RAG3WEAVER_BURN_DEVICE_EMBEDDER='cpu-fixture')
        self.streams = [(self.root / 'stdout').open('x'), (self.root / 'stderr').open('x')]
        self.process = subprocess.Popen([sys.executable, str(D / 'activation_privee.py'), '--adresse', self.adresse,
                                        '--plan', str(self.root / 'plan.json'), '--autoriser', str(self.gate),
                                        '--journal', str(self.events), '--budget-ms', str(budget)],
                                       env=env, stdout=self.streams[0], stderr=self.streams[1], start_new_session=True)

    def wait_event(self, path, event):
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            if path.is_file():
                rows = [json.loads(x) for x in path.read_text().splitlines()]
                found = [r for r in rows if r['event'] == event]
                if found:
                    return found[-1]
            if self.process.poll() is not None:
                self.fail('processus terminé avant événement ' + event)
            time.sleep(.005)
        self.fail('événement absent : ' + event)

    def test_barriere_avant_exec_et_meme_pid_groupe(self):
        self.spawn()
        before = self.wait_event(self.events, 'barriere_avant_exec')
        self.assertFalse((self.root / 'fixture.jsonl').exists())
        self.gate.write_text(json.dumps({'ok': True, 'source': 'barrière de test CPU'}))
        ready = self.wait_event(self.root / 'fixture.jsonl', 'pret')
        self.assertEqual(ready['pid'], self.process.pid)
        self.assertEqual(before['pid'], ready['pid'])
        self.assertEqual(before['pgrp'], self.process.pid)
        self.assertTrue(ready['factice'])

    def test_empreinte_modifiee_refusee_sans_exec(self):
        self.plan['artifacts']['provider']['sha256'] = '0' * 64
        self.spawn()
        self.assertEqual(self.process.wait(timeout=5), 2)
        self.assertFalse((self.root / 'fixture.jsonl').exists())
        self.assertIn('artefact modifié', (self.root / 'stderr').read_text())

    def test_signal_non_autorise_refuse(self):
        self.gate.write_text(json.dumps({'ok': False, 'source': 'refus'}))
        self.spawn()
        self.assertEqual(self.process.wait(timeout=5), 2)
        self.assertFalse((self.root / 'fixture.jsonl').exists())

    def test_budget_avant_exec_borne(self):
        self.spawn(budget=100)
        self.assertEqual(self.process.wait(timeout=5), 2)
        self.assertFalse((self.root / 'fixture.jsonl').exists())
        self.assertIn('budget avant exec', (self.root / 'stderr').read_text())


if __name__ == '__main__':
    unittest.main(verbosity=2)
