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

    def test_recu_vivant_puis_expire_ou_superviseur_deplace(self):
        import time
        pid = os.getpid()
        table = {pid: dict(parent=777, debut='enfant', cgroup='/job'),
                 777: dict(parent=0, debut='superviseur', cgroup='/job')}
        r = self.p / 'recu-vivant'
        r.touch(mode=0o600)
        recu = dict(pid=777, debut='superviseur', cgroup='/job', etat='actif', jeton='x', echeance=time.monotonic()+60)
        r.write_text(json.dumps(recu))
        with patch.dict(os.environ, {v.RECU: str(r), v.JETON: 'x'}), patch.object(v, 'poste_tenu', return_value=1), patch.object(v, 'signature_superviseur', return_value=True), patch.object(v, 'plafond', return_value={}):
            self.assertEqual(v.autorite(table)['pid'], 777)
            table[777]['cgroup'] = '/soeur'
            with self.assertRaises(v.Refus):
                v.autorite(table)
            table[777]['cgroup'] = '/job'
            recu['echeance'] = 0
            r.write_text(json.dumps(recu))
            with self.assertRaises(v.Refus):
                v.autorite(table)

    def test_cli_transmet_commande_et_conserve_plafond_plus_bas(self):
        argv = ['confiner_validation.py', 'lancer', '--delai', '29', '--mode', 'mesure', '--', 'une commande', 'a b']
        class ExecControle(Exception):
            pass
        with patch.object(sys, 'argv', argv), patch.object(v, 'poste_tenu', return_value=None), patch.dict(os.environ, {'POSTE_MEM_MAX': '8G', 'RAG3WEAVER_VERROU_TENU': 'prétendu'}, clear=True), patch.object(os, 'execve', side_effect=ExecControle) as executer:
            with self.assertRaises(ExecControle):
                v.main()
        path, command, env = executer.call_args.args
        self.assertEqual(Path(path).name, 'poste')
        self.assertEqual(command[1], 'mesure')
        self.assertEqual(command[-2:], ['une commande', 'a b'])
        self.assertEqual(env['POSTE_MEM_MAX'], '8G')
        self.assertNotIn('RAG3WEAVER_VERROU_TENU', env)

    def test_scope_soeur_refusee_avec_preuve(self):
        d = v.Descendants(777, '/job', '10')
        row = dict(parent=0, session=777, debut='10', cgroup='/autre')
        with patch.object(v, 'plafond', return_value=dict(direct='max', effectif=v.MAX_MEMOIRE)):
            with self.assertRaises(v.Refus):
                d.relever({777: row})
        self.assertIn('hors du cgroup', next(iter(d.observes.values()))['refus'])

    def test_pid_reutilise_n_est_jamais_signale(self):
        d = v.Descendants(777, '/job', 'ancien')
        d.possedes[777] = 'ancien'
        with patch.object(v, 'processus', return_value={777: dict(debut='nouveau')}), patch.object(os, 'pidfd_open') as ouvrir:
            d.signaler(signal.SIGTERM)
        ouvrir.assert_not_called()

    def test_racine_absente_puis_pid_reutilise_jamais_adopte(self):
        d = v.Descendants(777, '/job', 'zombie-original')
        d.relever({})  # Le premier inventaire ignore la racine déjà Zombie.
        table = {777: dict(parent=0, session=777, debut='nouveau-tiers', cgroup='/autre')}
        d.relever(table)
        self.assertEqual(d.possedes[777], 'zombie-original')
        self.assertEqual(d.observes, {})
        with patch.object(v, 'processus', return_value=table), patch.object(os, 'pidfd_open') as ouvrir:
            d.signaler(signal.SIGTERM)
        ouvrir.assert_not_called()

    def test_enfant_du_parent_reutilise_jamais_adopte_ni_signale(self):
        d = v.Descendants(777, '/job', 'ancien')
        d.possedes[777] = 'ancien'
        table = {777: dict(parent=0, session=777, debut='nouveau-tiers', cgroup='/autre'),
                 888: dict(parent=777, session=777, debut='enfant-tiers', cgroup='/autre')}
        with patch.object(v, 'plafond', return_value={}):
            d.relever(table)
        self.assertNotIn(888, d.possedes)
        with patch.object(v, 'processus', return_value=table), patch.object(os, 'pidfd_open') as ouvrir:
            d.signaler(signal.SIGTERM)
        ouvrir.assert_not_called()

    def test_reutilisation_entre_open_pidfd_et_recheck_ne_signale_pas(self):
        d = v.Descendants(777, '/job', 'ancien')
        d.possedes[777] = 'ancien'
        etats = [{777: dict(debut='ancien')}, {777: dict(debut='nouveau')}]
        with patch.object(v, 'processus', side_effect=etats), patch.object(os, 'pidfd_open', return_value=42) as ouvrir, patch.object(os, 'close') as fermer, patch.object(signal, 'pidfd_send_signal') as envoyer:
            d.signaler(signal.SIGTERM)
        ouvrir.assert_called_once_with(777)
        fermer.assert_called_once_with(42)
        envoyer.assert_not_called()

    def test_recu_forge_coherent_ancetre_reel_sans_superviseur_refuse(self):
        import time
        table = v.processus()
        pid = table[os.getpid()]['parent']
        self.assertIn(pid, table)  # Ancêtre réellement vivant, pas un PID inventé.
        r = self.p / 'forge'
        r.touch(mode=0o600)
        r.write_text(json.dumps(dict(pid=pid, debut=table[pid]['debut'], cgroup=table[pid]['cgroup'],
            etat='actif', jeton='choisi', echeance=time.monotonic()+60,
            script=str(ICI/'confiner_validation.py'), script_sha256='choisi')))
        # Le poste/plafond sont simulés valides ; le processus ancêtre, lui, est réel.
        with patch.dict(os.environ, {v.RECU: str(r), v.JETON: 'choisi'}), patch.object(v, 'poste_tenu', return_value=pid), patch.object(v, 'plafond', return_value={}):
            with self.assertRaises(v.Refus):
                v.autorite(table)

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

    def cas_processus(self, source, delai=5, plafond=None):
        # Faux helper bus : lance de vrais petits processus CPU privés, aucun IPC réseau.
        helper = self.p / 'bus-factice'
        helper.write_text('#!' + sys.executable + '\nimport sys,subprocess\ni=sys.argv.index("--")\nsys.exit(subprocess.call(sys.argv[i+1:]))\n')
        helper.chmod(0o700)
        command = self.p / 'command.py'
        command.write_text(source)
        preuves = self.p / 'preuves'
        vrai_popen = subprocess.Popen
        def creer(*args, **kwargs):
            enfant = vrai_popen(*args, **kwargs)
            debut = (Path('/proc')/str(enfant.pid)/'stat').read_text().rsplit(') ', 1)[1].split()[19]
            self.racine_attendue = (enfant.pid, debut)
            return enfant
        with patch.object(v, 'poste_tenu', return_value=1), patch.object(v, 'plafond', side_effect=plafond or (lambda *a: dict(direct='max', effectif=v.MAX_MEMOIRE))), patch.object(v.shutil, 'which', return_value=str(helper)), patch.object(v.subprocess, 'Popen', side_effect=creer):
            rc = v.superviser([sys.executable, str(command)], delai, str(preuves))
        evidence, = preuves.iterdir()
        verdict = json.loads((evidence / 'verdict.json').read_text())
        self.enfants_du_cas = json.loads((evidence / 'enfants.json').read_text())
        self.assertEqual(verdict['restes'], [])
        self.assertTrue(verdict['nettoye'])
        self.assertFalse(Path(verdict['prive']).exists())
        self.assertEqual(json.loads((evidence / 'autorite.json').read_text())['etat'], 'termine')
        return rc, verdict

    def test_ecart_de_confinement_arrete_aussi_les_descendants(self):
        appels = 0
        def limite(*args):
            nonlocal appels
            appels += 1
            if appels == 1:
                return dict(direct='max', effectif=v.MAX_MEMOIRE)
            raise v.Refus('scope simulée hors plafond')
        rc, verdict = self.cas_processus('import time\ntime.sleep(60)\n', plafond=limite)
        self.assertEqual(rc, 1)
        self.assertTrue(any((row['pid'], row['debut']) == self.racine_attendue and row.get('refus') == 'scope simulée hors plafond' for row in self.enfants_du_cas), self.enfants_du_cas)

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

    def test_echec_nettoyage_apres_commande_verte_reste_rouge(self):
        original = v.shutil.rmtree
        def refuser_prive(path, *args, **kwargs):
            if Path(path).name.startswith('r3v-'):
                raise PermissionError('refus simulé du nettoyage privé')
            return original(path, *args, **kwargs)
        helper = self.p / 'bus-factice'
        helper.write_text('#!'+sys.executable+'\nimport sys,subprocess\ni=sys.argv.index("--")\nsys.exit(subprocess.call(sys.argv[i+1:]))\n')
        helper.chmod(0o700)
        preuves = self.p / 'preuves-nettoyage'
        with patch.object(v, 'poste_tenu', return_value=1), patch.object(v, 'plafond', return_value={}), patch.object(v.shutil, 'which', return_value=str(helper)), patch.object(v.shutil, 'rmtree', side_effect=refuser_prive):
            rc = v.superviser([sys.executable, '-c', 'pass'], 5, str(preuves))
        dossier, = preuves.iterdir()
        verdict = json.loads((dossier/'verdict.json').read_text())
        self.assertEqual((rc, verdict['verdict'], verdict['code']), (1, 'ROUGE', 0))
        self.assertIn('nettoyage refusé', verdict['erreur'])
        self.assertFalse(verdict['nettoye'])
        self.assertEqual(verdict['restes'], [])
        original(verdict['prive'])  # Seul temporaire CPU attribué à ce témoin.

    def test_residu_decouvert_au_dernier_inventaire_ne_devient_pas_vert(self):
        class InventaireTardif:
            def __init__(self, *args):
                self.observes = {}
                self.possedes = {}
                self.appels = 0
            def relever(self, table):
                pass
            def vivants(self):
                self.appels += 1
                return [] if self.appels == 1 else [99999999]
            def arreter(self):
                return []  # Résidu simulé nettoyé, aucun vrai signal.
        with patch.object(v, 'Descendants', InventaireTardif):
            rc, verdict = self.cas_processus('pass\n')
        self.assertEqual(rc, 1)
        self.assertIn('découverts au nettoyage', verdict['erreur'])

    def test_enfant_detache_residuel_arrete_et_reste_rouge(self):
        rc, verdict = self.cas_processus('import subprocess,sys\nsubprocess.Popen([sys.executable,"-c","import time;time.sleep(60)"],start_new_session=True)\n')
        self.assertEqual(rc, 1)
        self.assertIn('descendants laissés', verdict['erreur'])


if __name__ == '__main__':
    unittest.main(verbosity=2, failfast=True)
