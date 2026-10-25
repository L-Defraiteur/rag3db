#!/usr/bin/env python3
"""Témoins du magasin de libs (scripts/rag3db_libs.py ; page
docs/11-octobre-2026-recherche/07-le-magasin-de-libs.md). Des bâtis factices :
une lib = /bin/true à qui objcopy pose la section .rag3db_build_id, un en-tête
qui définit RAG3DB_BUILD_ID, une extension qui porte la chaîne de l'identifiant.
Usage : python3 scripts/test_rag3db_libs.py"""
import os
import subprocess
import sys
import tempfile
import textwrap
import time
import unittest
from pathlib import Path

OUTIL = Path(__file__).resolve().parent / "rag3db_libs.py"


def faux_bati(racine: Path, nom: str, ident: str, ext_ident: str | None = None, contenu: str = "") -> Path:
    """Un dossier de bâti (`<bati>/src/…`) et l'extension de son arbre."""
    src = racine / nom / "src"
    (src / "include/common").mkdir(parents=True)
    lib = src / "librag3db.so"
    subprocess.run(["cp", "/bin/true", lib], check=True)
    section = racine / f"{nom}.section"
    section.write_bytes(ident.encode() + b"\0")
    subprocess.run(["objcopy", f"--add-section=.rag3db_build_id={section}", lib], check=True)
    if contenu:
        with open(lib, "ab") as f:
            f.write(contenu.encode())
    (src / "rag3db.hpp").write_text("// en-tête unique factice\n")
    (src / "include/common/build_id.h").write_text(f'#pragma once\n#define RAG3DB_BUILD_ID "{ident}"\n')
    ext = racine / nom / "arbre/extension/vector/build"
    ext.mkdir(parents=True)
    (ext / "libvector.rag3db_extension").write_bytes(b"\x7fELF-factice\0" + (ext_ident or ident).encode() + b"\0")
    return racine / nom


# Les témoins jouent sans planchers (un fichier absent : aucun plancher), sauf
# ceux qui disent les leurs : leurs bâtis factices n'ont pas de commit.
SANS_PLANCHERS = str(Path(tempfile.gettempdir()) / "rag3db-libs-sans-planchers-absent")
LES_PLANCHERS = str(OUTIL.parent / "planchers-de-la-lib")


def outil(magasin: Path, *args: str, env: dict | None = None) -> subprocess.CompletedProcess:
    e = {**os.environ, "RAG3DB_LIBS_DIR": str(magasin), "RAG3DB_LIBS_PLANCHERS": SANS_PLANCHERS, **(env or {})}
    return subprocess.run([sys.executable, OUTIL, *args], capture_output=True, text=True, env=e)


def publier(magasin: Path, bati: Path, *extra: str) -> subprocess.CompletedProcess:
    return outil(magasin, "publier", str(bati), "--racine", str(bati / "arbre"), *extra)


# Deux bâtis réels du moteur : 87a362e30 est sous les planchers (ni les
# tranches de 16 Gio, 4f9274a38, ni le ROLLBACK du COPY de relations,
# a45a15adf) ; 4f9274a38 les contient.
SOUS_LE_PLANCHER = "rag3db-build-87a362e304200776000095c5a3d6531373560d9a-Release-p12"
# Au plancher : le commit où ce script se trouve. Tout plancher est un
# ancêtre de master, ce qui reste vrai quand un plancher s'ajoute — un commit
# fixe (4f9274a38) est passé sous le plancher dcbaef7b1 le jour où il est entré.
_TETE = subprocess.run(["git", "-C", str(Path(__file__).resolve().parent), "rev-parse", "HEAD"],
                       capture_output=True, text=True).stdout.strip()
AU_PLANCHER = f"rag3db-build-{_TETE}-Release-p12"


def git_connait(*commits: str) -> bool:
    """Le dépôt de ce script a-t-il ces commits (pas un clone superficiel) ?"""
    return all(subprocess.run(["git", "-C", str(OUTIL.parent), "cat-file", "-e", f"{c}^{{commit}}"],
                              capture_output=True).returncode == 0 for c in commits)


class Magasin(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.racine = Path(self.tmp.name)
        self.magasin = self.racine / "magasin"

    def tearDown(self):
        self.tmp.cleanup()

    @unittest.skipUnless(git_connait("87a362e304200776000095c5a3d6531373560d9a", "4f9274a38769a085ab0009a89d1b7dba7597e568",
                                      "a45a15adf2347de1f089ab7d00ee7d36d46e02d0"),
                         "le dépôt n'a pas l'historique du moteur (clone superficiel)")
    def test_une_lib_sous_un_plancher_est_refusee_et_courant_ne_s_y_pose_pas(self):
        les = {"RAG3DB_LIBS_PLANCHERS": LES_PLANCHERS}
        for nom, ident in [("sous", SOUS_LE_PLANCHER), ("au", AU_PLANCHER)]:
            r = publier(self.magasin, faux_bati(self.racine, nom, ident), "--sans-courant")
            self.assertEqual(r.returncode, 0, r.stderr)
        r = outil(self.magasin, "verifier", str(self.magasin / SOUS_LE_PLANCHER), env=les)
        self.assertEqual(r.returncode, 2, r.stderr)
        self.assertIn("4f9274a38", r.stderr)
        self.assertIn("a45a15adf", r.stderr)
        self.assertIn("VERDICT : SOUS LE PLANCHER", r.stderr.strip().splitlines()[-1])
        r = outil(self.magasin, "courant", "--poser", SOUS_LE_PLANCHER, env=les)
        self.assertEqual(r.returncode, 2, "courant posé sur une lib sous le plancher")
        self.assertFalse((self.magasin / "courant").exists())
        r = outil(self.magasin, "verifier", str(self.magasin / AU_PLANCHER), env=les)
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn("VERDICT : planchers tenus", r.stderr.strip().splitlines()[-1])
        self.assertEqual(outil(self.magasin, "courant", "--poser", AU_PLANCHER, env=les).returncode, 0)
        liste = outil(self.magasin, "liste", env=les).stdout
        self.assertIn(f"{SOUS_LE_PLANCHER} : ", liste)
        self.assertIn("SOUS LE PLANCHER 4f9274a38, a45a15adf", liste)

    def test_un_plancher_que_git_ne_peut_juger_n_est_jamais_tenu(self):
        """96, 25 octobre 2026 : un plancher sans réponse de git (un commit que
        le dépôt ne connaît pas, une lib sans commit) rendait 0, et comptait
        comme tenu. Code 3, et le verdict le dit en dernière ligne ; courant ne
        se pose pas."""
        inconnu = self.racine / "planchers-inconnus"
        inconnu.write_text("0123456789abcdef0123456789abcdef01234567 un correctif que ce dépôt ne connaît pas\n")
        env = {"RAG3DB_LIBS_PLANCHERS": str(inconnu)}
        r = publier(self.magasin, faux_bati(self.racine, "au", AU_PLANCHER), "--sans-courant")
        self.assertEqual(r.returncode, 0, r.stderr)
        r = outil(self.magasin, "verifier", str(self.magasin / AU_PLANCHER), env=env)
        self.assertEqual(r.returncode, 3, r.stderr)
        self.assertIn("VERDICT : NON JUGÉ", r.stderr.strip().splitlines()[-1])
        self.assertEqual(outil(self.magasin, "courant", "--poser", AU_PLANCHER, env=env).returncode, 3)
        self.assertFalse((self.magasin / "courant").exists())
        self.assertIn("git ne connaît pas un plancher", r.stderr.strip().splitlines()[-1])
        # Une lib sans commit dans son identifiant ne se juge pas non plus, et
        # le verdict dit que c'est l'identifiant, pas le dépôt.
        publier(self.magasin, faux_bati(self.racine, "sans", "rag3db-build-SANS"), "--sans-courant")
        r = outil(self.magasin, "verifier", str(self.magasin / "rag3db-build-SANS"), env={"RAG3DB_LIBS_PLANCHERS": LES_PLANCHERS})
        self.assertEqual(r.returncode, 3, r.stderr)
        self.assertIn("l'identifiant ne porte pas de commit", r.stderr.strip().splitlines()[-1])

    @unittest.skipUnless(git_connait("4f9274a38769a085ab0009a89d1b7dba7597e568", "a45a15adf2347de1f089ab7d00ee7d36d46e02d0"),
                         "le dépôt n'a pas l'historique du moteur (clone superficiel)")
    def test_verifier_courant_et_un_identifiant_nu_jugent_la_lib_du_magasin(self):
        """96, 25 octobre 2026 : `verifier courant` jugeait le mot, pas la lib
        que le lien désigne. Le mot `courant` et un identifiant nu se
        résolvent au magasin."""
        les = {"RAG3DB_LIBS_PLANCHERS": LES_PLANCHERS}
        publier(self.magasin, faux_bati(self.racine, "au", AU_PLANCHER), "--sans-courant")
        self.assertEqual(outil(self.magasin, "courant", "--poser", AU_PLANCHER, env=les).returncode, 0)
        for cible in ["courant", AU_PLANCHER]:
            r = outil(self.magasin, "verifier", cible, env=les)
            self.assertEqual(r.returncode, 0, f"{cible} : {r.stderr}")
            self.assertIn("planchers tenus", r.stderr.strip().splitlines()[-1])

    def test_publier_pose_un_dossier_immuable_et_courant(self):
        bati = faux_bati(self.racine, "a", "rag3db-build-A")
        r = publier(self.magasin, bati)
        self.assertEqual(r.returncode, 0, r.stderr)
        d = self.magasin / "rag3db-build-A"
        for f in ["librag3db.so", "rag3db.hpp", "include/common/build_id.h", "libvector.rag3db_extension", "BUILD_ID", "MANIFESTE"]:
            self.assertTrue((d / f).exists(), f)
        self.assertEqual((d / "BUILD_ID").read_text().strip(), "rag3db-build-A")
        self.assertEqual(os.path.realpath(self.magasin / "courant"), str(d.resolve()))
        r = outil(self.magasin, "courant")
        self.assertEqual(r.stdout.strip(), str(d.resolve()))
        self.assertEqual(list(self.magasin.glob(".tmp-*")), [], "aucun temporaire ne reste")

    def test_republier_le_meme_ne_change_rien_un_autre_contenu_est_refuse(self):
        bati = faux_bati(self.racine, "a", "rag3db-build-A")
        publier(self.magasin, bati)
        avant = (self.magasin / "rag3db-build-A/MANIFESTE").read_text()
        r = publier(self.magasin, bati)
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn("déjà publié", r.stdout)
        self.assertEqual((self.magasin / "rag3db-build-A/MANIFESTE").read_text(), avant)
        autre = faux_bati(self.racine, "a-bis", "rag3db-build-A", contenu="autre binaire")
        r = publier(self.magasin, autre)
        self.assertEqual(r.returncode, 2)
        self.assertIn("rag3db-build-A", r.stderr)
        self.assertEqual((self.magasin / "rag3db-build-A/MANIFESTE").read_text(), avant, "rien n'est écrasé")

    def test_une_paire_qui_ne_va_pas_ensemble_est_refusee(self):
        bati = faux_bati(self.racine, "a", "rag3db-build-A", ext_ident="rag3db-build-Z")
        r = publier(self.magasin, bati)
        self.assertEqual(r.returncode, 2)
        self.assertIn("libvector", r.stderr)
        self.assertFalse((self.magasin / "rag3db-build-A").exists())
        bati = faux_bati(self.racine, "b", "rag3db-build-B")
        (bati / "src/include/common/build_id.h").write_text('#define RAG3DB_BUILD_ID "rag3db-build-X"\n')
        r = publier(self.magasin, bati)
        self.assertEqual(r.returncode, 2)
        self.assertIn("rag3db-build-X", r.stderr)

    def test_le_menage_epargne_le_courant_l_epingle_le_mappe_et_le_reference(self):
        for n in "ABCDEF":
            publier(self.magasin, faux_bati(self.racine, n.lower(), f"rag3db-build-{n}"), "--sans-courant")
            time.sleep(0.02)
        outil(self.magasin, "courant", "--poser", "rag3db-build-A")
        (self.magasin / "rag3db-build-B/ÉPINGLÉ").write_text("une mesure à refaire\n")
        # C : mappé par un processus vivant.
        tenu = self.magasin / "rag3db-build-C/librag3db.so"
        proc = subprocess.Popen(
            [sys.executable, "-c", f"import mmap,sys;f=open({str(tenu)!r},'rb');m=mmap.mmap(f.fileno(),0,access=mmap.ACCESS_READ);print('ok',flush=True);sys.stdin.read()"],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
        self.assertEqual(proc.stdout.readline().strip(), "ok")
        # D : référencé par le RUNPATH d'un binaire installé.
        bins = self.racine / "bin"
        bins.mkdir()
        c = self.racine / "x.c"
        c.write_text("int main(void){return 0;}\n")
        subprocess.run(["gcc", c, "-o", bins / "hote", f"-Wl,-rpath,{self.magasin}/rag3db-build-D", "-Wl,--enable-new-dtags"], check=True)
        try:
            r = outil(self.magasin, "menage", "--garder", "1", env={"RAG3DB_LIBS_BINS": str(bins), "RAG3DB_LIBS_SANS_UNITES": "1"})
        finally:
            proc.stdin.close()
            proc.wait()
        self.assertEqual(r.returncode, 0, r.stderr)
        restants = sorted(p.name for p in self.magasin.iterdir() if p.name.startswith("rag3db-build-"))
        # F : le plus récent (garder 1) ; A courant, B épinglé, C mappé, D référencé ; E part.
        self.assertEqual(restants, ["rag3db-build-A", "rag3db-build-B", "rag3db-build-C", "rag3db-build-D", "rag3db-build-F"], r.stdout)
        for raison in ["courant", "épinglé", "mappé", "référencé"]:
            self.assertIn(raison, r.stdout)


if __name__ == "__main__":
    unittest.main(verbosity=2)
