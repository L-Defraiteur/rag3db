#!/usr/bin/env python3
"""Témoin sans Cargo/GPU : fonctions réelles du lanceur et aide Rust des tests.

Le petit adaptateur DaemonEmbedder simule seulement identité et protocole HTTP ;
Serveur et bge_m3_par_le_demon sont compilés depuis leurs vraies sources.
Les seuls processus signalés sont les faux services privés de ce témoin.
"""
import json
import os
from pathlib import Path
import re
import signal
import socket
import subprocess
import tempfile
import time
import unittest

CRATE = Path(__file__).resolve().parent.parent

FAUX = r'''#!/usr/bin/env python3
import http.server, json, os, signal, sys, time
from pathlib import Path
host, port = sys.argv[sys.argv.index('--adresse')+1].rsplit(':', 1)
with open(os.environ['FAKE_PIDS'], 'a') as f: f.write(str(os.getpid())+'\n')
def fini(*_):
    if os.environ.get('FAKE_IGNORE_TERM'): return
    time.sleep(.15)
    Path(os.environ['FAKE_DONE']).write_text(str(os.getpid()))
    os._exit(0)
signal.signal(signal.SIGTERM, fini)
class Handler(http.server.BaseHTTPRequestHandler):
    def log_message(self, *_): pass
    def do_GET(self):
        if self.path == '/quitter':
            Path(os.environ['FAKE_QUIT']).write_text(str(os.getpid()))
            self.send_response(200); self.end_headers(); self.wfile.write(b'ok')
            import threading
            threading.Timer(.05, lambda: os._exit(0)).start()
            return
        self.send_response(200); self.end_headers()
        self.wfile.write(json.dumps({'service':'embeddings', 'ancien':bool(os.environ.get('FAKE_OLD'))}).encode())
print('journal du faux service', flush=True)
http.server.HTTPServer((host, int(port)), Handler).serve_forever()
'''

ADAPTATEUR = r'''
use std::io::{Read, Write};
use std::net::TcpStream;
#[derive(Debug, Clone)]
struct Identite { executable: String, modele: String, dim: usize, dual: bool, sparse: bool, factice: bool }
struct DaemonEmbedder { id: Identite, adresse: String, _attache: serveur::Attache }
fn requete(adresse: &str, route: &str) -> Result<String, String> {
    let mut s = TcpStream::connect(adresse).map_err(|e| e.to_string())?;
    s.set_read_timeout(Some(std::time::Duration::from_secs(2))).unwrap();
    write!(s, "GET {route} HTTP/1.0\r\nHost: local\r\n\r\n").unwrap();
    let mut texte=String::new(); s.read_to_string(&mut texte).map_err(|e| e.to_string())?;
    Ok(texte)
}
impl DaemonEmbedder {
    fn serveur(adresse: &str, programme: &str) -> serveur::Serveur {
        serveur::Serveur::new("rag3weaver-embeddings", adresse, programme)
            .args(["--adresse", adresse]).sante("/sante", "embeddings")
    }
    fn assurer(s: &serveur::Serveur) -> Result<Self, String> {
        let attache=s.assurer().map_err(|e| e.to_string())?;
        let texte=requete(s.adresse(), "/sante")?;
        let ancien=texte.contains("\"ancien\": true");
        Ok(Self {id: Identite {executable: if ancien {"ancien"} else {"courant"}.into(),
            modele:"bge-m3".into(), dim:1024, dual:true, sparse:true, factice:false},
            adresse:s.adresse().into(), _attache:attache})
    }
    fn est_a_jour_avec(&self, _: &std::path::Path) -> bool { self.id.executable=="courant" }
    fn identite(&self) -> &Identite { &self.id }
    fn quitter(&self) -> Result<(), String> { requete(&self.adresse, "/quitter").map(|_| ()) }
}
fn main() { if let Err(e)=bge_m3_par_le_demon() { eprintln!("{e}"); std::process::exit(2); } }
'''

E2E = r'''#!/usr/bin/env python3
import os, subprocess, sys
from pathlib import Path
if '--exact' in sys.argv: sys.exit(0)
subprocess.run([os.environ['FAKE_ATTACHE']], check=True)
if 'e2e_search::' in sys.argv and os.environ.get('FAKE_ECHEC'):
    Path(os.environ['TMPDIR'], 'preuve-processus').write_text('processus rouge')
    sys.exit(7)
if 'e2e_search::' in sys.argv and os.environ.get('FAKE_RESIDU'):
    Path(os.environ['TMPDIR'], 'interdit').write_text('preuve du garde')
print('test result: ok. 1 passed; 0 failed; 0 ignored;')
'''


class Journaux(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp = tempfile.TemporaryDirectory(prefix='temoin-journal-demons-')
        cls.root = Path(cls.temp.name)
        cls.faux = cls.root/'faux-service'
        cls.faux.write_text(FAUX); cls.faux.chmod(0o755)
        # La vraie fonction, pas une seconde implémentation de sa politique.
        common = (CRATE/'tests/common/mod.rs').read_text()
        debut = common.index('    fn bge_m3_par_le_demon()')
        fin = common.index('{', debut); profondeur = 1; fin += 1
        while profondeur:
            profondeur += (common[fin] == '{') - (common[fin] == '}'); fin += 1
        fonction = common[debut:fin]
        source = cls.root/'attache.rs'
        source.write_text('#![allow(dead_code, unused_mut)]\n#[path='+json.dumps(str(CRATE/'src/serveur.rs'))+'] mod serveur;\n'+ADAPTATEUR+'\n'+fonction)
        env = os.environ.copy(); env['CARGO_BIN_EXE_rag3weaver-embeddings'] = str(cls.faux)
        cls.attache = cls.root/'attache'
        subprocess.run(['rustc', '--edition=2021', str(source), '-o', str(cls.attache)], env=env, check=True, capture_output=True)
        script = (CRATE/'run_e2e.sh').read_text()
        cls.fonctions = '\n'.join(m.group() for m in re.finditer(r'^[a-zA-Z_][a-zA-Z_0-9]*\(\) \{\n.*?^\}', script, re.M|re.S))

    @classmethod
    def tearDownClass(cls): cls.temp.cleanup()

    def setUp(self):
        self.d = Path(tempfile.mkdtemp(dir=self.root)); (self.d/'bin').mkdir()
        self.pids = self.d/'pids'; self.pids.touch()
        self.env = os.environ.copy()
        for key in ('RAG3WEAVER_PASSE_E2E', 'FAKE_OLD', 'FAKE_IGNORE_TERM', 'FAKE_RESIDU', 'FAKE_ECHEC', 'RAG3WEAVER_GARDER', 'RAG3WEAVER_E2E_JOURNAUX_DEMONS'):
            self.env.pop(key, None)
        with socket.socket() as s:
            s.bind(('127.0.0.1', 0)); port = s.getsockname()[1]
        self.env.update(FAKE_PIDS=str(self.pids), FAKE_DONE=str(self.d/'fini'), FAKE_QUIT=str(self.d/'quitte'),
                        FAKE_ATTACHE=str(self.attache), RAG3WEAVER_EMBEDDINGS_ADDR=f'127.0.0.1:{port}')
        self.existants = []
        # L'énumération est confinée aux pids des faux services de ce cas.
        (self.d/'bin/pidof').write_text('#!/bin/sh\ncat "$FAKE_PIDS"\n')
        (self.d/'bin/pidof').chmod(0o755)
        (self.d/'e2e').write_text(E2E); (self.d/'e2e').chmod(0o755)
        (self.d/'bin/cargo').write_text('#!/bin/sh\nprintf \'%s\\n\' \' '+json.dumps({'reason':'compiler-artifact','target':{'name':'e2e'},'executable':str(self.d/'e2e')})+'\'\n')
        (self.d/'bin/cargo').chmod(0o755)

    def tearDown(self):
        # Uniquement les processus démarrés par ce témoin, jamais pidof réel.
        for pid in map(int, self.pids.read_text().split()):
            try:
                if ('FAKE_PIDS='+str(self.pids)).encode() in Path(f'/proc/{pid}/environ').read_bytes().split(b'\0'): os.kill(pid, signal.SIGTERM)
            except (ProcessLookupError, FileNotFoundError): pass
        time.sleep(.2)
        for pid in map(int, self.pids.read_text().split()):
            try:
                if ('FAKE_PIDS='+str(self.pids)).encode() in Path(f'/proc/{pid}/environ').read_bytes().split(b'\0'): os.kill(pid, signal.SIGKILL)
            except (ProcessLookupError, FileNotFoundError): pass
        for p in self.existants: p.wait(timeout=2)

    def existant(self, ancien=False):
        env = self.env.copy()
        if ancien: env['FAKE_OLD'] = '1'
        p = subprocess.Popen([str(self.faux), '--adresse', env['RAG3WEAVER_EMBEDDINGS_ADDR']], env=env,
                             stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        self.existants.append(p)
        fin = time.monotonic()+3
        while time.monotonic()<fin:
            try:
                with socket.create_connection(tuple([env['RAG3WEAVER_EMBEDDINGS_ADDR'].split(':')[0], int(env['RAG3WEAVER_EMBEDDINGS_ADDR'].split(':')[1])]), timeout=.1): return p
            except OSError: time.sleep(.01)
        self.fail('le faux service privé ne démarre pas')

    def jouer(self, residu=False, ignore_term=False, echec=False):
        disque = self.d/'disque'; disque.mkdir()
        parent = disque/'passe'; parent.mkdir()
        tmp = self.d/'tmp'; tmp.mkdir()
        prelude = '\n'.join(f'{k}={__import__("shlex").quote(str(v))}' for k,v in {
            'WEAVER':CRATE, 'E2E_DISQUE_RACINE':disque, 'E2E_DISQUE_PARENT':parent,
            'E2E_TMP_PARENT':tmp, 'CARGO_TARGET_DIR':self.d/'target', 'LISTE_DES_RESIDUS':CRATE/'tests/e2e/suites_qui_laissent_des_residus.txt',
            'E2E_LOG':self.d/'e2e.log', 'FEATURES':'rag3db-native', 'TEST_FILTER':'', 'CHARGE_PID':'',
            'RAG3WEAVER_PASSE_E2E':'temoin-'+self.d.name}.items())
        harness = self.d/'harness.sh'
        harness.write_text('set -u\n'+self.fonctions+'\n'+prelude+'''
export RAG3WEAVER_PASSE_E2E
confined() { "$@"; }
CARGO_ARGS=(--test e2e_search --test e2e_second)
if declare -F preparer_journaux_des_demons >/dev/null; then preparer_journaux_des_demons; fi
trap a_la_sortie EXIT
jouer_les_suites
exit $?
''')
        env = self.env.copy(); env['PATH'] = str(self.d/'bin')+':'+env['PATH']
        if residu: env['FAKE_RESIDU'] = '1'
        if echec: env['FAKE_ECHEC'] = '1'
        if ignore_term: env['FAKE_IGNORE_TERM'] = '1'
        rendu = subprocess.run(['bash', str(harness)], env=env, text=True, capture_output=True, timeout=30)
        print(rendu.stdout, rendu.stderr)
        return rendu

    def test_partage_journal_de_passe_et_attente_arret(self):
        r = self.jouer()
        self.assertEqual(r.returncode, 0, r.stdout+r.stderr)
        self.assertEqual(len(self.pids.read_text().split()), 1, 'un service pour deux suites')
        self.assertTrue((self.d/'fini').exists(), 'le nettoyage doit attendre la fin effective')
        journaux = list((self.d/'disque/e2e-gardes').rglob('rag3weaver-embeddings.log'))
        self.assertEqual(len(journaux), 1, 'journal partagé conservé hors TMPDIRsuite')
        self.assertIn('journal du faux service', journaux[0].read_text())

    def test_rouge_du_garde_archive_la_preuve(self):
        r = self.jouer(residu=True)
        self.assertEqual(r.returncode, 101)
        self.assertIn('RÉSIDUS e2e_search', r.stdout)
        preuves = list((self.d/'disque/e2e-gardes').rglob('interdit'))
        self.assertEqual(len(preuves), 1, 'rouge du garde archivé même si assertions vertes')
        self.assertEqual(preuves[0].read_text(), 'preuve du garde')

    def test_rouge_du_processus_archive_aussi_son_tmpdir(self):
        r = self.jouer(echec=True)
        self.assertEqual(r.returncode, 101)
        preuves = list((self.d/'disque/e2e-gardes').rglob('preuve-processus'))
        self.assertEqual(len(preuves), 1)
        self.assertEqual(preuves[0].read_text(), 'processus rouge')

    def test_existant_courant_reste_intact(self):
        p = self.existant()
        r = self.jouer()
        self.assertEqual(r.returncode, 0, r.stdout+r.stderr)
        self.assertIsNone(p.poll())
        self.assertFalse((self.d/'quitte').exists())
        self.assertFalse((self.d/'fini').exists())

    def test_existant_perime_est_refuse_sans_remplacement(self):
        p = self.existant(ancien=True)
        r = subprocess.run([str(self.attache)], env=self.env, capture_output=True, text=True, timeout=15)
        self.assertNotEqual(r.returncode, 0, 'un périmé est refusé')
        self.assertIn('périmé', r.stderr)
        self.assertIsNone(p.poll(), 'le service déjà présent doit rester vivant')
        self.assertFalse((self.d/'quitte').exists(), 'aucune requête de remplacement')
        self.assertEqual(len(self.pids.read_text().split()), 1)

    def test_arret_non_termine_preserve_les_ressources(self):
        r = self.jouer(ignore_term=True)
        self.assertEqual(r.returncode, 101, r.stdout+r.stderr)
        self.assertTrue((self.d/'disque/passe/demons/rag3weaver-embeddings.log').exists())
        self.assertFalse((self.d/'fini').exists())


if __name__ == '__main__': unittest.main(verbosity=2)
