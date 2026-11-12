#!/usr/bin/env python3
"""Témoins CPU : verrous/cgroups isolés ou simulés, aucun bus/service réel."""
import importlib.util
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
import xml.etree.ElementTree as ET

ICI = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('validation', ICI / 'confiner_validation.py')
v = importlib.util.module_from_spec(spec)
spec.loader.exec_module(v)


class Montage(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix='test-validation-')
        self.p = Path(self.tmp.name)

    def tearDown(self):
        self.tmp.cleanup()

    def test_bus_sans_activation_et_chemin_xml_echappe(self):
        root = ET.fromstring(v.configuration_bus(self.p / 'a&b'))
        self.assertEqual([x.tag for x in root], ['type', 'listen', 'auth', 'policy'])
        self.assertEqual(root.find('listen').text, 'unix:dir=' + str(self.p / 'a&b'))
        self.assertEqual(root.find('policy/allow').get('user'), str(os.getuid()))

    def test_plafond_herite_et_refus_absence_de_limite(self):
        for n, m in [('a', '34359738368'), ('a/b', 'max')]:
            (self.p / n).mkdir(exist_ok=True)
            (self.p / n / 'memory.max').write_text(m)
        self.assertEqual(v.plafond('/a/b', self.p), dict(direct='max', effectif=v.MAX_MEMOIRE))
        (self.p / 'a/memory.max').write_text('max')
        with self.assertRaises(v.Refus):
            v.plafond('/a/b', self.p)
        with self.assertRaises(v.Refus):
            v.plafond('/a/../b', self.p)

    def test_marqueur_seul_et_recu_perime_sont_refuses(self):
        with patch.dict(os.environ, {'RAG3WEAVER_VERROU_TENU': 'prétendu'}, clear=True):
            with self.assertRaises(v.Refus):
                v.autorite(v.processus())
        r = self.p / 'recu'
        r.touch(mode=0o600)
        r.write_text(json.dumps(dict(pid=99999999, jeton='x', etat='termine')))
        with patch.dict(os.environ, {v.RECU: str(r), v.JETON: 'x'}):
            with self.assertRaises(v.Refus):
                v.autorite(v.processus())

    def test_scope_soeur_refusee_avec_preuve(self):
        d = v.Descendants(777, '/job')
        row = dict(parent=0, session=777, debut='10', cgroup='/autre')
        with patch.object(v, 'plafond', return_value=dict(direct='max', effectif=v.MAX_MEMOIRE)):
            with self.assertRaises(v.Refus):
                d.relever({777: row})
        self.assertIn('hors du cgroup', next(iter(d.observes.values()))['refus'])

    def test_pid_reutilise_n_est_jamais_signale(self):
        d = v.Descendants(777, '/job')
        d.possedes[777] = 'ancien'
        with patch.object(v, 'processus', return_value={777: dict(debut='nouveau')}), patch.object(os, 'pidfd_open') as ouvrir:
            d.signaler(signal.SIGTERM)
        ouvrir.assert_not_called()

    def test_dependance_absente_refuse_avant_commande(self):
        with patch.object(v, 'poste_tenu', return_value=1), patch.object(v, 'plafond', return_value={}), patch.object(v.shutil, 'which', return_value=None), patch.object(v.subprocess, 'Popen') as lancer:
            with self.assertRaises(v.Refus):
                v.superviser([sys.executable, '-c', 'pass'], 3, str(self.p))
        lancer.assert_not_called()

    def test_adoption_reelle_des_deux_entrees_sans_cargo(self):
        # Copier les vraies entrées, substituer seulement la frontière commune.
        w = self.p / 'weaver'
        scripts = w / 'scripts'
        scripts.mkdir(parents=True)
        for src, dst in [(ICI.parent / 'run_e2e.sh', w / 'run_e2e.sh'), (ICI / 'temoins_navigateur.sh', scripts / 'temoins_navigateur.sh')]:
            dst.write_bytes(src.read_bytes())
        (scripts / 'confiner_validation.py').write_text("import sys,json,os\nif sys.argv[1]=='verifier':sys.exit(1)\nopen(os.environ['TRACE'],'w').write(json.dumps(sys.argv[1:]))\nsys.exit(23)\n")
        for entry in [w / 'run_e2e.sh', scripts / 'temoins_navigateur.sh']:
            trace = self.p / 'trace'
            env = dict(os.environ, TRACE=str(trace), RAG3WEAVER_VERROU_TENU='mensonger', RAG3WEAVER_VALIDATION_TIMEOUT='29')
            r = subprocess.run(['bash', str(entry)], env=env, capture_output=True)
            self.assertEqual(r.returncode, 23, r.stderr)
            argv = json.loads(trace.read_text())
            self.assertEqual(argv[0], 'lancer')
            self.assertEqual(argv[argv.index('--delai') + 1], '29')
            self.assertEqual(argv[-2:], ['bash', str(entry)])

    def cas_processus(self, source, delai=5):
        # Faux helper bus : lance de vrais petits processus CPU privés, aucun IPC réseau.
        helper = self.p / 'bus-factice'
        helper.write_text('#!' + sys.executable + '\nimport sys,subprocess\ni=sys.argv.index("--")\nsys.exit(subprocess.call(sys.argv[i+1:]))\n')
        helper.chmod(0o700)
        command = self.p / 'command.py'
        command.write_text(source)
        preuves = self.p / 'preuves'
        with patch.object(v, 'poste_tenu', return_value=1), patch.object(v, 'plafond', return_value=dict(direct='max', effectif=v.MAX_MEMOIRE)), patch.object(v.shutil, 'which', return_value=str(helper)):
            rc = v.superviser([sys.executable, str(command)], delai, str(preuves))
        evidence, = preuves.iterdir()
        verdict = json.loads((evidence / 'verdict.json').read_text())
        self.assertEqual(verdict['restes'], [])
        self.assertTrue(verdict['nettoye'])
        self.assertFalse(Path(verdict['prive']).exists())
        self.assertEqual(json.loads((evidence / 'autorite.json').read_text())['etat'], 'termine')
        return rc, verdict

    def test_commande_normale_et_nettoyage_prive(self):
        rc, verdict = self.cas_processus('pass\n')
        self.assertEqual((rc, verdict['verdict']), (0, 'VERT'))

    def test_code_rouge_conserve_sans_retry(self):
        marker = self.p / 'tentatives'
        rc, verdict = self.cas_processus(f'from pathlib import Path\np=Path({str(marker)!r})\np.write_text(p.read_text()+"x" if p.exists() else "x")\nraise SystemExit(7)\n')
        self.assertEqual(rc, 1)
        self.assertEqual(verdict['code'], 7)
        self.assertEqual(marker.read_text(), 'x')

    def test_delai_arrete_parent_et_enfant_possedes(self):
        rc, verdict = self.cas_processus('import subprocess,sys,time\nsubprocess.Popen([sys.executable,"-c","import time;time.sleep(60)"])\ntime.sleep(60)\n', 3)
        self.assertEqual(rc, 1)
        self.assertIn('échéance', verdict['erreur'])

    def test_enfant_detache_residuel_arrete_et_reste_rouge(self):
        rc, verdict = self.cas_processus('import subprocess,sys\nsubprocess.Popen([sys.executable,"-c","import time;time.sleep(60)"],start_new_session=True)\n')
        self.assertEqual(rc, 1)
        self.assertIn('descendants laissés', verdict['erreur'])


if __name__ == '__main__':
    unittest.main(verbosity=2)
