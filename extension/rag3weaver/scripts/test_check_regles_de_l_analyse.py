#!/usr/bin/env python3
"""Le témoin de `check_regles_de_l_analyse.py` (page 06 d'everything declarative, § 13) : un lot qui
change un fichier de l'analyse (`src/code.rs`, `src/manifestes.rs`, `src/queries_via.rs`) monte
`REGLES_DE_L_ANALYSE`, ou dit « sans effet sur l'analyse » dans un de ses messages ; un lot qui ne
touche pas ces fichiers passe.

Usage : python3 extension/rag3weaver/scripts/test_check_regles_de_l_analyse.py   (rc 0 : vert)
"""
import os
import subprocess
import sys
import tempfile
import unittest

ICI = os.path.dirname(os.path.abspath(__file__))
SCRIPT = os.path.join(ICI, "check_regles_de_l_analyse.py")
CODE = "extension/rag3weaver/src/code.rs"


def git(depot, *args):
    return subprocess.run(["git", "-C", depot, *args], check=True, capture_output=True, text=True).stdout


def ecrire(depot, chemin, texte):
    p = os.path.join(depot, chemin)
    os.makedirs(os.path.dirname(p), exist_ok=True)
    with open(p, "w") as f:
        f.write(texte)


def commit(depot, message):
    git(depot, "add", "-A")
    git(depot, "-c", "user.email=temoin@exemple", "-c", "user.name=temoin", "commit", "-q", "-m", message)


def garde(depot, base):
    return subprocess.run([sys.executable, SCRIPT, "--depot", depot, "--base", base], capture_output=True, text=True)


class Regles(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.d = self.tmp.name
        git(self.d, "init", "-q")
        ecrire(self.d, CODE, "pub const REGLES_DE_L_ANALYSE: u32 = 1;\nfn analyse() {}\n")
        ecrire(self.d, "extension/rag3weaver/src/ide.rs", "fn vue() {}\n")
        commit(self.d, "base")
        self.base = git(self.d, "rev-parse", "HEAD").strip()

    def tearDown(self):
        self.tmp.cleanup()

    def test_un_changement_de_l_analyse_sans_montee_est_refuse(self):
        ecrire(self.d, CODE, "pub const REGLES_DE_L_ANALYSE: u32 = 1;\nfn analyse() { autre(); }\n")
        commit(self.d, "feat: une règle d'import changée")
        r = garde(self.d, self.base)
        self.assertNotEqual(r.returncode, 0, r.stdout + r.stderr)
        self.assertIn("REGLES_DE_L_ANALYSE", r.stdout)

    def test_la_montee_de_la_constante_passe(self):
        ecrire(self.d, CODE, "pub const REGLES_DE_L_ANALYSE: u32 = 2;\nfn analyse() { autre(); }\n")
        commit(self.d, "feat: une règle d'import changée, les règles montent")
        r = garde(self.d, self.base)
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)

    def test_la_mention_sans_effet_passe(self):
        ecrire(self.d, CODE, "pub const REGLES_DE_L_ANALYSE: u32 = 1;\n// un commentaire\nfn analyse() {}\n")
        commit(self.d, "docs: un commentaire, sans effet sur l'analyse")
        r = garde(self.d, self.base)
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)

    def test_un_lot_hors_de_l_analyse_passe(self):
        ecrire(self.d, "extension/rag3weaver/src/ide.rs", "fn vue() { autre(); }\n")
        commit(self.d, "feat: l'IDE")
        r = garde(self.d, self.base)
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)


if __name__ == "__main__":
    unittest.main()
