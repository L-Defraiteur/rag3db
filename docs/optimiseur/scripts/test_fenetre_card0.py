import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('fenetre', Path(__file__).with_name('fenetre_card0.py'))
f = importlib.util.module_from_spec(spec)
spec.loader.exec_module(f)


class Fenetre(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.racine = Path(self.tmp.name)
        self.recette = {'nom': 'fixture', 'modele': 'modele-non-charge', 'dimensions': None,
                        'consommateurs': ['fixture-cpu'], 'profil': 'rocm', 'duree_max_sec': 120,
                        'argv': ['/usr/bin/true', '--device', 'ROCm0', '--no-warmup'],
                        'executable_sha256': 'a' * 64}

    def date(self, jour, heure, minute=0, seconde=0):
        return f.dt.datetime(2026, 11, jour, heure, minute, seconde, tzinfo=f.PARIS)

    def test_refus_avant_minuit(self):
        with self.assertRaisesRegex(f.Refus, 'hors fenêtre'):
            f.budget(self.date(12, 23, 59, 59))

    def test_minuit_autorise_et_budget_borne(self):
        self.assertEqual(f.budget(self.date(13, 0)), 7 * 3600 + 54 * 60)

    def test_minute_de_marge_non_consommee_par_un_demarrage(self):
        with self.assertRaisesRegex(f.Refus, 'moins de 60'):
            f.budget(self.date(13, 7, 53, 1))

    def test_refus_coupure_08h_et_nuit_suivante(self):
        for date in [self.date(13, 7, 54), self.date(13, 8), self.date(14, 0)]:
            with self.subTest(date=date), self.assertRaises(f.Refus):
                f.budget(date)

    def test_refus_horloge_sans_fuseau(self):
        with self.assertRaisesRegex(f.Refus, 'sans fuseau'):
            f.budget(self.date(13, 0).replace(tzinfo=None))

    def test_session_reconnectee_fermee_meme_sans_timer_joue(self):
        with self.assertRaisesRegex(f.Refus, 'hors fenêtre'):
            f.verifier(self.date(13, 8), self.racine)

    def test_marqueur_ferme_interdit_reprise_meme_horloge_reculant(self):
        (self.racine / 'ferme.json').write_text('{}')
        with self.assertRaisesRegex(f.Refus, 'définitivement fermée'):
            f.verifier(self.date(13, 1), self.racine)

    def test_pci_modifie_refuse(self):
        with patch.object(f, 'carte', side_effect=f.Refus('PCI changé')):
            with self.assertRaisesRegex(f.Refus, 'PCI changé'):
                f.verifier(self.date(13, 1), self.racine)

    def test_preparation_non_qualifiee_refuse(self):
        with patch.object(f, 'carte'):
            with self.assertRaisesRegex(f.Refus, 'préparation non qualifiée'):
                f.verifier(self.date(13, 1), self.racine)

    def test_source_qualifiee_et_version_differente(self):
        script = Path(__file__).with_name('fenetre_card0.py')
        pret = self.racine / 'pret.json'
        pret.write_text(json.dumps({'source_sha256': f.sha_fichier(script)}))
        with patch.object(f, 'carte'), patch.object(f, 'SCRIPT', script):
            self.assertGreater(f.verifier(self.date(13, 1), self.racine), 0)
            pret.write_text(json.dumps({'source_sha256': '0' * 64}))
            with self.assertRaisesRegex(f.Refus, 'source différente'):
                f.verifier(self.date(13, 1), self.racine)

    def test_unite_groupe_borne_sans_restart(self):
        c = f.commande_unite(self.recette, self.racine / 'fixture.json', 'a' * 64, 120)
        for a in ['--property=Restart=no', '--property=KillMode=control-group',
                  '--property=TimeoutStopSec=15s', '--property=RuntimeMaxSec=120', '--slice=' + f.SLICE]:
            self.assertIn(a, c)
        self.assertTrue(any(x.startswith('--property=ExecCondition=') and x.endswith(' condition') for x in c))

    def test_nom_ne_peut_pas_designer_service_lucie(self):
        self.recette['nom'] = '../steam.service'
        with self.assertRaisesRegex(f.Refus, 'nom privé invalide'):
            f.valider_spec(self.recette)

    def test_depart_differe_ne_renouvelle_pas_le_budget(self):
        f.commande_unite(self.recette, self.racine / 'fixture.json', 'a' * 64, 200)
        with self.assertRaisesRegex(f.Refus, 'durée demandée au-delà'):
            f.commande_unite(self.recette, self.racine / 'fixture.json', 'a' * 64, 119)

    def test_rocm_explicite_et_pas_echauffement_avant_qualification(self):
        self.recette['argv'] = ['/usr/bin/true', '--device', 'ROCm1', '--no-warmup']
        with self.assertRaisesRegex(f.Refus, 'ROCm0 explicite'):
            f.valider_spec(self.recette)

    def test_rocr_agent_unique_pci07(self):
        f.placement_rocr('  BDFID: 1792\n  Device Type: GPU\n')
        for t in ['BDFID: 1024\nDevice Type: GPU\n', 'BDFID: 1792\nDevice Type: GPU\nBDFID: 1024\nDevice Type: GPU\n', '']:
            with self.subTest(texte=t), self.assertRaises(f.Refus):
                f.placement_rocr(t)

    def manager(self, etranger=False, survivant=False):
        p = self.racine / 'specs/fixture.json'
        p.parent.mkdir()
        p.write_text(json.dumps(self.recette))
        empreinte = f.digest(p.read_bytes())
        class Manager:
            def __init__(self): self.stop = []
            def info(self, unite):
                return {'LoadState': 'loaded', 'ActiveState': 'inactive' if self.stop else 'active',
                        'MainPID': '0' if self.stop else '123', 'Description': 'Lucie' if etranger else f.MARQUE + empreinte,
                        'Slice': f.SLICE, 'Restart': 'no', 'KillMode': 'control-group'}
            def arreter(self, unites): self.stop.extend(unites)
            def pids(self, info): return [123, 456] if not self.stop else [456] if survivant else []
        return Manager()

    def test_coupure_cible_seulement_unites_enregistrees_et_descendants(self):
        m = self.manager()
        v = f.couper(self.date(13, 7, 54), self.racine, m)
        self.assertEqual(m.stop, [f.PREFIXE + 'fixture.service'])
        self.assertEqual(v['descendants_restants'], [])
        self.assertEqual(v['pids_groupes_avant'][m.stop[0]], [123, 456])
        self.assertTrue((self.racine / 'ferme.json').exists())

    def test_service_etranger_jamais_arrete(self):
        m = self.manager(etranger=True)
        with self.assertRaisesRegex(f.Refus, 'étrangère'):
            f.couper(self.date(13, 7, 54), self.racine, m)
        self.assertEqual(m.stop, [])

    def test_descendant_survivant_interdit_verdict_vert(self):
        m = self.manager(survivant=True)
        with self.assertRaisesRegex(f.Refus, 'descendants privés'):
            f.couper(self.date(13, 7, 54), self.racine, m)
        self.assertFalse((self.racine / 'restitution.json').exists())

    def test_coupure_sans_consommateur_ne_charge_rien(self):
        m = self.manager()
        for p in (self.racine / 'specs').glob('*'): p.unlink()
        v = f.couper(self.date(13, 7, 54), self.racine, m)
        self.assertEqual(m.stop, [])
        self.assertEqual(v['unites_privees_arretees'], [])


if __name__ == '__main__':
    unittest.main()
