#!/usr/bin/env python3
"""Fixtures CPU du protocole Cargo/CMake ; aucune compilation ni bibliothèque chargée."""
import copy
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('bati_vector_ci', Path(__file__).with_name('bati_vector_ci.py'))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class Provenance(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.repo = Path(self.tmp.name)
        self.patch = patch.object(module, 'revision', return_value='a' * 40)
        self.patch.start()
        self.addCleanup(self.patch.stop)
        for p in ['tools/rust_api/build.rs', 'tools/rust_api/Cargo.toml',
                  'extension/rag3weaver/Cargo.toml', 'extension/rag3weaver/Cargo.lock', 'CMakeLists.txt']:
            self.creer(p, 'source fixture\n')
        # Les ID viennent de metadata, jamais d’un préfixe de dossier ou de la version seule.
        self.moteur = 'path+file://' + str(self.repo / 'tools/rust_api') + '#rag3db@0.11.1'
        self.hote = 'path+file://' + str(self.repo / 'extension/rag3weaver') + '#rag3weaver@0.1.0'
        self.metadata = self.repo / 'metadata.json'
        self.meta = {'packages': [
            {'name': 'rag3db', 'id': self.moteur, 'manifest_path': str(self.repo / 'tools/rust_api/Cargo.toml')},
            {'name': 'rag3weaver', 'id': self.hote, 'manifest_path': str(self.repo / 'extension/rag3weaver/Cargo.toml')},
        ]}
        self.metadata.write_text(json.dumps(self.meta))
        self.out = self.repo / 'extension/rag3weaver/target/debug/build/rag3db/024bb70f/out'
        self.bdir = self.out / 'build'
        self.cache(self.bdir)
        self.archive = self.creer(self.bdir / 'src/librag3db.a', '!<arch>\n')
        self.rlib = self.creer('artefacts/librag3db-hash.rlib', 'rlib fixture')
        self.exe = self.creer('target/debug/build/rag3weaver/024bb70f/out/rag3weaver-test', 'test fixture')
        self.messages = self.repo / 'messages.jsonl'
        self.lignes = [module.contexte(self.repo, self.metadata),
            {'reason': 'build-script-executed', 'package_id': self.moteur,
             'out_dir': str(self.out), 'linked_paths': ['native=' + str(self.bdir / 'src')],
             'linked_libs': ['static:+whole-archive=rag3db']},
            {'reason': 'compiler-artifact', 'package_id': self.moteur, 'target': {'kind': ['lib']},
             'filenames': [str(self.rlib)], 'fresh': True},
            {'reason': 'compiler-artifact', 'package_id': self.hote, 'target': {'kind': ['lib']},
             'profile': {'test': True}, 'executable': str(self.exe),
             'features': ['rag3db-native', 'code', 'daemon', 'openai-llm'], 'fresh': True},
            {'reason': 'build-finished', 'success': True}]

    def creer(self, p, texte):
        p = self.repo / p
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(texte)
        return p

    def cache(self, bdir, source=None):
        self.creer(bdir / 'CMakeCache.txt',
                   f'CMAKE_HOME_DIRECTORY:INTERNAL={source or self.repo}\nCMAKE_CACHEFILE_DIR:INTERNAL={bdir}\n')

    def choisir(self):
        self.messages.write_text(''.join(json.dumps(x) + '\n' for x in self.lignes))
        return module.selectionner(self.repo, self.metadata, self.messages)

    def test_out_dir_nouveaux_dossiers_et_artefact_cache_valide(self):
        if 'WORKFLOW_ANCIEN' in os.environ:
            ligne = next(x.strip() for x in Path(os.environ['WORKFLOW_ANCIEN']).read_text().splitlines()
                         if 'bdir=$(ls -td' in x)
            # Le vrai ancien sélecteur, sans exécuter les lignes CMake suivantes.
            ancien = subprocess.run(['bash', '-e', '-o', 'pipefail', '-c', ligne + '\nprintf "%s" "$bdir"'],
                                    cwd=self.repo, text=True, capture_output=True)
            self.assertEqual(ancien.returncode, 0, ancien.stderr)
            self.assertEqual(ancien.stdout, str(self.bdir))
        else:
            self.assertEqual(self.choisir()['cmake_build_dir'], str(self.bdir))

    def test_cache_plus_recent_etranger_ignore(self):
        leurre = self.repo / 'extension/rag3weaver/target/debug/build/rag3db-leurre/out/build'
        self.cache(leurre)
        os.utime(leurre, (2000000000, 2000000000))
        self.assertEqual(self.choisir()['cmake_build_dir'], str(self.bdir))

    def test_ancienne_disposition_acceptee_si_cargo_la_designe(self):
        out = self.repo / 'cache/debug/build/rag3db-hash/out'
        bdir = out / 'build'
        self.cache(bdir)
        self.creer(bdir / 'src/librag3db.a', '!<arch>\n')
        self.lignes[1].update(out_dir=str(out), linked_paths=['native=' + str(bdir / 'src')])
        self.assertEqual(self.choisir()['cmake_build_dir'], str(bdir))

    def test_script_absent_refuse(self):
        del self.lignes[1]
        with self.assertRaisesRegex(module.Refus, 'build-script-executed.*absent'):
            self.choisir()

    def test_deux_out_dir_du_meme_paquet_refuses(self):
        autre = copy.deepcopy(self.lignes[1]); autre['out_dir'] = str(self.repo / 'autre/out')
        self.lignes.insert(2, autre)
        with self.assertRaisesRegex(module.Refus, 'build-script-executed.*ambigu'):
            self.choisir()

    def test_autre_paquet_non_selectionne(self):
        self.lignes[1]['package_id'] = 'registry+rag3db@0.11.1'
        with self.assertRaisesRegex(module.Refus, 'build-script-executed.*absent'):
            self.choisir()

    def test_out_dir_absent_refuse_sans_repli_glob(self):
        self.lignes[1]['out_dir'] = str(self.repo / 'inexistant/out')
        with self.assertRaises(FileNotFoundError):
            self.choisir()

    def test_chemin_de_liaison_autre_bati_refuse(self):
        self.lignes[1]['linked_paths'] = ['native=' + str(self.repo / 'autre/build/src')]
        with self.assertRaisesRegex(module.Refus, 'moteur lié.*OUT_DIR'):
            self.choisir()

    def test_liaison_partagee_refusee(self):
        self.lignes[1]['linked_libs'] = ['dylib=rag3db']
        with self.assertRaisesRegex(module.Refus, 'statique attendu'):
            self.choisir()

    def test_cache_cmake_deplace_refuse(self):
        self.creer(self.bdir / 'CMakeCache.txt', f'CMAKE_HOME_DIRECTORY:INTERNAL={self.repo}\nCMAKE_CACHEFILE_DIR:INTERNAL=/ancien/build\n')
        with self.assertRaisesRegex(module.Refus, 'périmé.*CMAKE_CACHEFILE_DIR'):
            self.choisir()

    def test_source_cmake_etrangere_refusee(self):
        self.cache(self.bdir, '/autre/checkout')
        with self.assertRaisesRegex(module.Refus, 'étranger.*CMAKE_HOME_DIRECTORY'):
            self.choisir()

    def test_archive_absente_refusee(self):
        self.archive.unlink()
        with self.assertRaisesRegex(module.Refus, 'archive statique.*absente'):
            self.choisir()

    def test_artefact_moteur_absent_refuse(self):
        del self.lignes[2]
        with self.assertRaisesRegex(module.Refus, 'artefact Rust.*absent'):
            self.choisir()

    def test_tests_non_construits_refuses(self):
        self.lignes[3]['profile']['test'] = False
        with self.assertRaisesRegex(module.Refus, 'exécutable.*absent'):
            self.choisir()

    def test_fin_cargo_absente_ou_echec_refuses(self):
        for fin in [[], [{'reason': 'build-finished', 'success': False}]]:
            with self.subTest(fin=fin):
                self.lignes = self.lignes[:4] + fin
                with self.assertRaisesRegex(module.Refus, 'bâti Cargo incomplet'):
                    self.choisir()

    def test_contexte_d_un_autre_commit_refuse(self):
        self.lignes[0]['commit'] = 'b' * 40
        with self.assertRaisesRegex(module.Refus, 'contexte.*périmé'):
            self.choisir()

    def test_producteur_modifie_apres_contexte_refuse(self):
        self.creer('tools/rust_api/build.rs', 'autre recette')
        with self.assertRaisesRegex(module.Refus, 'contexte.*périmé'):
            self.choisir()

    def test_metadata_modifiee_refusee(self):
        self.metadata.write_text(json.dumps(self.meta) + '\n')
        with self.assertRaisesRegex(module.Refus, 'contexte.*périmé'):
            self.choisir()

    def test_concatenation_d_invocations_refusee(self):
        self.lignes.insert(2, copy.deepcopy(self.lignes[0]))
        with self.assertRaisesRegex(module.Refus, 'invocations Cargo mélangées'):
            self.choisir()


if __name__ == '__main__':
    unittest.main()
