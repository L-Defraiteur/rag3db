"""Filets CPU : preuves de placement/identité et nettoyage de nos seuls groupes."""
import copy
import os
from pathlib import Path
import sys
import tempfile
import time
import unittest
from mesurer_prive import Actor, health_guard, memory_guard, members, MIB, rows

SOURCE={'model':'granite-107m','dimensions':384,'activation':{'profil':'profil-privé','peripherique':'pci:0000:04:00.0'}}
HEALTH={'modele':'granite-107m','dim':384,'factice':False,
        'activation':{'profil':'profil-privé','peripherique':'pci:0000:04:00.0','recette':'identité-fixe'},
        'chauffe':{'embed':{'etat':'complete','duree_ms':200,'formes':[{'lot':1,'mots':40}]}}}


class PreuvesCPU(unittest.TestCase):
    def test_sante_valide(self):
        self.assertEqual(health_guard(HEALTH,SOURCE,'identité-fixe')['etat'],'complete')

    def test_identites_etrangeres_refusees(self):
        for field,value in [('modele','autre'),('dim',768),('factice',True)]:
            h=copy.deepcopy(HEALTH);h[field]=value
            with self.subTest(field=field),self.assertRaises(RuntimeError):health_guard(h,SOURCE,'identité-fixe')

    def test_profil_ou_recette_etrangers(self):
        for field in ['profil','peripherique','recette']:
            h=copy.deepcopy(HEALTH);h['activation'][field]='autre'
            with self.subTest(field=field),self.assertRaises(RuntimeError):health_guard(h,SOURCE,'identité-fixe')

    def test_chauffe_ancienne_inconnue_refusee(self):
        h=copy.deepcopy(HEALTH);h['chauffe']=None
        with self.assertRaisesRegex(RuntimeError,'chauffe non observée'):health_guard(h,SOURCE,'identité-fixe')

    def test_chauffe_partielle_conserve_erreur(self):
        h=copy.deepcopy(HEALTH);h['chauffe']['embed']['etat']='partielle'
        h['chauffe']['embed']['formes'].append({'lot':8,'mots':40,'erreur':'échec explicite'})
        self.assertEqual(health_guard(h,SOURCE,'identité-fixe')['formes'][1]['erreur'],'échec explicite')

    def test_chauffe_complete_contradictoire_refusee(self):
        h=copy.deepcopy(HEALTH);h['chauffe']['embed']['formes'][0]['erreur']='échec'
        with self.assertRaises(RuntimeError):health_guard(h,SOURCE,'identité-fixe')

    def test_contexte_seat1_tolere_metrique_inconnue(self):
        self.assertEqual(memory_guard({'clients':[{'pci':'0000:07:00.0','vram_bytes':40960,'gtt_bytes':2129920,'engine_ns':{}}]}),'INCONNUS')

    def test_depassement_seat1_refuse(self):
        with self.assertRaises(RuntimeError):memory_guard({'clients':[{'pci':'0000:07:00.0','vram_bytes':16*MIB+1,'gtt_bytes':0,'engine_ns':{}}]})

    def test_ligne_partielle_non_inventee(self):
        with tempfile.TemporaryDirectory() as tmp:
            p=Path(tmp)/'events';p.write_bytes(b'{"event":"un"}\n{"event":"de')
            self.assertEqual(rows(p),[{'event':'un'}])

    def test_nettoyage_groupe_parent_enfant_cpu(self):
        with tempfile.TemporaryDirectory() as tmp:
            p=Path(tmp);script=p/'fixture.py'
            script.write_text('''import subprocess,signal,sys,time
child=subprocess.Popen([sys.executable,'-c','import time;time.sleep(30)'])
def stop(sig,frame):
 child.terminate();child.wait();raise SystemExit(0)
signal.signal(signal.SIGTERM,stop)
print(child.pid,flush=True)
time.sleep(30)
''')
            a=Actor(sys.executable,script,p)
            try:
                deadline=time.monotonic()+5
                while not (p/'stdout').read_text().strip():
                    if time.monotonic()>deadline:raise AssertionError('fixture CPU non prête')
                    time.sleep(.01)
                a.stop(time.monotonic()+3)
                self.assertEqual(members(a.pid,a.birth),[])
            finally:a.stop(time.monotonic()+3)


if __name__=='__main__':unittest.main(verbosity=2)
