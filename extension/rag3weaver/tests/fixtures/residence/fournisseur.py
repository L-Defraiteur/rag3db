#!/usr/bin/env python3
"""Fournisseur CPU explicitement factice, sans gestionnaire de résidence.

Ticket docs/embarquement/tickets/2026-11-12-gerer-activation-et-residence-des-modeles.md.
Les gates bloquent la construction/chauffe avant écoute. Le journal est la vérité
terrain du processus ; l'appelant réel doit fournir ses propres états et annulations.
Aucun téléchargement, import ML ni accès GPU.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import signal
import sys
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer


def arguments():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--adresse', default='127.0.0.1:0')
    p.add_argument('--modele', default='fixture-residence-384')
    p.add_argument('--dim', type=int, default=384)
    p.add_argument('--generation', required=True)
    p.add_argument('--journal', type=Path, required=True)
    p.add_argument('--liberer-chargement', type=Path)
    p.add_argument('--liberer-chauffe', type=Path)
    p.add_argument('--budget-pret-ms', type=int, default=10000)
    p.add_argument('--echec-chauffe')
    a = p.parse_args()
    if a.dim <= 0 or a.budget_pret_ms <= 0:
        p.error('dimension et budget strictement positifs exigés')
    if not a.adresse.startswith('127.0.0.1:'):
        p.error('fixture privée sur 127.0.0.1 uniquement')
    return a


class Fixture(ThreadingHTTPServer):
    daemon_threads = True

    def __init__(self, args):
        self.args = args
        self.lock = threading.Lock()
        self.deadline = time.monotonic() + args.budget_pret_ms / 1000
        self.log = args.journal.open('x', encoding='utf-8')
        self.stopped = False
        super().__init__(('127.0.0.1', int(args.adresse.rsplit(':', 1)[1])), Handler, bind_and_activate=False)

    def event(self, event, **values):
        with self.lock:
            self.log.write(json.dumps(dict(event=event, monotonic_ns=time.monotonic_ns(), pid=os.getpid(),
                                           generation=self.args.generation, modele=self.args.modele,
                                           dim=self.args.dim, factice=True, **values)) + '\n')
            self.log.flush()

    def gate(self, path, phase):
        self.event(phase + '_attente')
        while path is not None and not path.is_file():
            if time.monotonic() >= self.deadline:
                self.event('erreur', phase=phase, cause='budget de préparation dépassé')
                raise TimeoutError(phase)
            time.sleep(.01)
        if time.monotonic() >= self.deadline:
            raise TimeoutError(phase)
        self.event(phase + '_termine')

    def identity(self):
        # Contrat santé existant, sans inventer le schéma futur de chauffe.
        identity = dict(service='rag3weaver-embeddings', modele=self.args.modele, dim=self.args.dim,
                    dual=False, sparse=False, factice=True, precision='f32-fixture',
                    executable=str(Path(__file__).resolve()), lot_conseille=[8, 64])
        # Déclaration reçue du gestionnaire, pas preuve de placement physique.
        keys = {'profil': 'RAG3WEAVER_ACTIVATION_PROFIL',
                'peripherique': 'RAG3WEAVER_ACTIVATION_PERIPHERIQUE',
                'recette': 'RAG3WEAVER_ACTIVATION_IDENTITE'}
        declared = {field: os.environ.get(key) for field, key in keys.items()}
        if all(value is not None for value in declared.values()):
            identity['activation'] = declared
        return identity

    def close(self):
        if not self.stopped:
            self.stopped = True
            self.server_close()
            self.event('arrete')
            self.log.close()


class Handler(BaseHTTPRequestHandler):
    server_version = 'FixtureResidenceCPU/1'

    def log_message(self, *_):
        pass

    def answer(self, status, value):
        data = json.dumps(value, separators=(',', ':')).encode()
        self.send_response(status)
        self.send_header('Content-Type', 'application/json')
        self.send_header('Content-Length', str(len(data)))
        # HTTP/1.0 ferme après chaque réponse : le dire, pour qu'aucun client
        # ne garde la connexion pour la requête suivante.
        self.send_header('Connection', 'close')
        self.end_headers()
        self.wfile.write(data)

    def drain(self):
        """Lire le corps d'une requête qu'on refuse : fermer une connexion
        avec des octets non lus envoie un reset, que le client peut lire comme
        « Peer disconnected » sur sa requête suivante (une fois sur vingt,
        13 novembre 2026, depuis que l'activation pose un bail : POST /bail,
        route que ce fournisseur ne sert pas)."""
        try:
            size = int(self.headers.get('Content-Length', '0'))
        except ValueError:
            size = 0
        if 0 < size <= 65536:
            self.rfile.read(size)

    def do_GET(self):
        if self.path != '/sante':
            return self.answer(404, {'error': 'route absente'})
        self.server.event('sante_servie')
        self.answer(200, self.server.identity())

    def do_POST(self):
        if self.path != '/embed':
            self.drain()
            return self.answer(404, {'error': 'route absente'})
        try:
            size = int(self.headers.get('Content-Length', '-1'))
            if not 0 < size <= 65536:
                return self.answer(400, {'error': 'corps absent ou trop grand'})
            request = json.loads(self.rfile.read(size))
            texts = request.get('texts') if isinstance(request, dict) else None
            if not isinstance(texts, list) or not texts or len(texts) > 8 or any(not isinstance(x, str) for x in texts):
                return self.answer(400, {'error': 'texts doit être une liste non vide de 1 à 8 chaînes'})
        except (ValueError, UnicodeError):
            return self.answer(400, {'error': 'JSON invalide'})
        self.server.event('embed_commence', textes=len(texts))
        vectors = []
        for text in texts:
            h = hashlib.sha256(text.encode()).digest()
            vectors.append([(h[i % len(h)] - 127.5) / 127.5 for i in range(self.server.args.dim)])
        self.answer(200, {'vectors': vectors})
        self.server.event('embed_termine', textes=len(texts))


def main():
    args = arguments()
    fixture = Fixture(args)
    def stop(_sig, _frame):
        raise SystemExit(0)
    signal.signal(signal.SIGTERM, stop)
    signal.signal(signal.SIGINT, stop)
    try:
        fixture.event('constructeur_demarre')
        fixture.gate(args.liberer_chargement, 'chargement')
        fixture.event('chauffe_commence')
        fixture.gate(args.liberer_chauffe, 'chauffe')
        if args.echec_chauffe:
            fixture.event('erreur', phase='chauffe', cause=args.echec_chauffe)
            return 3
        fixture.server_bind()
        fixture.server_activate()
        fixture.event('pret', adresse=f'127.0.0.1:{fixture.server_address[1]}')
        print('adresse liée : 127.0.0.1:' + str(fixture.server_address[1]), flush=True)
        fixture.serve_forever(poll_interval=.02)
        return 0
    except TimeoutError:
        return 4
    finally:
        fixture.close()


if __name__ == '__main__':
    sys.exit(main())
