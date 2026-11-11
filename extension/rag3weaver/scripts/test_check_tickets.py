#!/usr/bin/env python3
"""Le témoin de `check_tickets.py` (ticket 2026-11-07-l-index-des-tickets-epingle-ses-comptes-et-
fait-un-conflit-a-chaque-lot) : la garde des en-têtes, l'index généré, la relecture d'un lot, et
deux lots qui ouvrent chacun un ticket et se rebasent l'un sur l'autre sans conflit, ce que
l'index épinglé refusait ; puis (11 novembre 2026) les tickets ouverts des dossiers de session,
`docs/<session>/tickets/`, leur dossier dans l'index, et un ticket déplacé que `--lot` suit.

Usage : python3 extension/rag3weaver/scripts/test_check_tickets.py   (rc 0 : vert)
"""
import os
import subprocess
import sys
import tempfile
import unittest

ICI = os.path.dirname(os.path.abspath(__file__))
SCRIPT = os.path.join(ICI, "check_tickets.py")

ENTETE = """# {titre}

- **État** : {etat}
- **Famille** : moteur
- **Gravité** : réponse fausse
- **Atteignable en service** : non
- **Touche rag3weaver** : non

## Ce que c'est

Un ticket de témoin.
"""


def ecrire(racine, rel, titre, etat="ouvert", sans=None):
    chemin = os.path.join(racine, "docs", "tickets", rel)
    os.makedirs(os.path.dirname(chemin), exist_ok=True)
    texte = ENTETE.format(titre=titre, etat=etat)
    if sans:
        texte = "\n".join(l for l in texte.split("\n") if not l.startswith(f"- **{sans}**"))
    with open(chemin, "w", encoding="utf-8") as f:
        f.write(texte)


def jouer(racine, *args):
    r = subprocess.run([sys.executable, SCRIPT, *args, racine], capture_output=True, text=True)
    return r.returncode, r.stdout + r.stderr


def git(racine, *args):
    subprocess.run(["git", "-C", racine, *args], check=True, capture_output=True)


class CheckTickets(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.racine = self.tmp.name
        os.makedirs(os.path.join(self.racine, "docs", "tickets", "closed"))
        with open(os.path.join(self.racine, "docs", "tickets", "README.md"), "w") as f:
            f.write("# Tickets\n")
        ecrire(self.racine, "2026-11-01-un.md", "Un")
        ecrire(self.racine, "closed/2026-10-01-deux.md", "Deux", etat="corrigé `abc`")

    def tearDown(self):
        self.tmp.cleanup()

    def test_des_entetes_complets_passent(self):
        rc, out = jouer(self.racine)
        self.assertEqual(rc, 0, out)
        self.assertIn("2 en-têtes valides (1 ouverts, 1 fermés)", out)

    def test_un_champ_absent_est_refuse(self):
        ecrire(self.racine, "2026-11-02-trois.md", "Trois", sans="Gravité")
        rc, out = jouer(self.racine)
        self.assertEqual(rc, 1, out)
        self.assertIn("2026-11-02-trois.md : champ « Gravité » absent", out)

    def test_un_etat_qui_contredit_le_dossier_est_refuse(self):
        ecrire(self.racine, "closed/2026-10-02-ouvert-mais-range.md", "R", etat="ouvert (oublié)")
        ecrire(self.racine, "2026-11-03-corrige-mais-ouvert.md", "C", etat="corrigé `def`")
        rc, out = jouer(self.racine)
        self.assertEqual(rc, 1, out)
        self.assertIn("rangé dans closed/ mais son État commence par « ouvert »", out)
        self.assertIn("son État dit « corrigé » mais il n'est pas dans closed/", out)

    def test_l_index_genere_porte_ses_comptes(self):
        rc, out = jouer(self.racine, "--index")
        self.assertEqual(rc, 0, out)
        with open(os.path.join(self.racine, "docs", "tickets", "INDEX.md"), encoding="utf-8") as f:
            index = f.read()
        self.assertIn("## Ouverts — moteur (1)", index)
        self.assertIn("## Fermés (1)", index)
        self.assertIn("[Un](2026-11-01-un.md)", index)

    def test_deux_lots_qui_ouvrent_chacun_un_ticket_se_rebasent_sans_conflit(self):
        r = self.racine
        git(r, "init", "-q", "-b", "master")
        git(r, "-c", "user.email=t@t", "-c", "user.name=t", "add", "-A")
        git(r, "-c", "user.email=t@t", "-c", "user.name=t", "commit", "-q", "-m", "base")
        git(r, "checkout", "-q", "-b", "lot-a")
        ecrire(r, "2026-11-07-a.md", "A")
        git(r, "add", "-A")
        git(r, "-c", "user.email=t@t", "-c", "user.name=t", "commit", "-q", "-m", "a")
        git(r, "checkout", "-q", "master")
        git(r, "checkout", "-q", "-b", "lot-b")
        ecrire(r, "2026-11-07-b.md", "B")
        os.replace(os.path.join(r, "docs/tickets/2026-11-01-un.md"), os.path.join(r, "docs/tickets/closed/2026-11-01-un.md"))
        ecrire(r, "closed/2026-11-01-un.md", "Un", etat="fermé")
        git(r, "add", "-A")
        git(r, "-c", "user.email=t@t", "-c", "user.name=t", "commit", "-q", "-m", "b")
        # lot-b se rebase sur lot-a : aucun fichier commun, aucun conflit.
        rebase = subprocess.run(["git", "-C", r, "-c", "user.email=t@t", "-c", "user.name=t", "rebase", "-q", "lot-a"],
                                capture_output=True, text=True)
        self.assertEqual(rebase.returncode, 0, rebase.stdout + rebase.stderr)
        rc, out = jouer(r)
        self.assertEqual(rc, 0, out)
        self.assertIn("4 en-têtes valides (2 ouverts, 2 fermés)", out)
        # La relecture du lot b depuis lot-a : un ouvert, un fermé.
        rc, out = jouer(r, "--lot", "lot-a")
        self.assertEqual(rc, 0, out)
        self.assertIn("ouvert   2026-11-07-b.md", out)
        self.assertIn("fermé    closed/2026-11-01-un.md", out)

    def test_une_fermeture_qui_reecrit_le_ticket_se_lit_fermee(self):
        r = self.racine
        git(r, "init", "-q", "-b", "master")
        git(r, "-c", "user.email=t@t", "-c", "user.name=t", "add", "-A")
        git(r, "-c", "user.email=t@t", "-c", "user.name=t", "commit", "-q", "-m", "base")
        # La fermeture réécrit tout le ticket : git ne voit plus le renommage.
        os.remove(os.path.join(r, "docs/tickets/2026-11-01-un.md"))
        chemin = os.path.join(r, "docs/tickets/closed/2026-11-01-un.md")
        with open(chemin, "w", encoding="utf-8") as f:
            f.write(ENTETE.format(titre="Un", etat="corrigé `abc`") + "\n".join(["Un texte neuf."] * 40))
        git(r, "add", "-A")
        git(r, "-c", "user.email=t@t", "-c", "user.name=t", "commit", "-q", "-m", "fermer")
        rc, out = jouer(r, "--lot", "HEAD~1")
        self.assertEqual(rc, 0, out)
        self.assertIn("fermé    closed/2026-11-01-un.md", out)
        self.assertNotIn("retiré", out)

    # Les tickets d'une session (11 novembre 2026) : un ticket ouvert vit dans docs/tickets/ ou dans
    # docs/<session>/tickets/ ; fermer, c'est git mv vers docs/tickets/closed/.
    def ecrire_session(self, session, nom, titre, etat="ouvert"):
        chemin = os.path.join(self.racine, "docs", session, "tickets", nom)
        os.makedirs(os.path.dirname(chemin), exist_ok=True)
        with open(chemin, "w", encoding="utf-8") as f:
            f.write(ENTETE.format(titre=titre, etat=etat))

    def test_un_ticket_de_session_est_un_ticket_ouvert(self):
        self.ecrire_session("recherche", "2026-11-10-s.md", "S")
        rc, out = jouer(self.racine)
        self.assertEqual(rc, 0, out)
        self.assertIn("3 en-têtes valides (2 ouverts, 1 fermés)", out)

    def test_un_ticket_de_session_clos_hors_de_closed_est_refuse(self):
        self.ecrire_session("recherche", "2026-11-10-s.md", "S", etat="corrigé `abc`")
        rc, out = jouer(self.racine)
        self.assertEqual(rc, 1, out)
        self.assertIn("recherche/tickets/2026-11-10-s.md : son État dit « corrigé » mais il n'est pas dans closed/", out)

    def test_un_meme_ticket_a_deux_endroits_est_refuse(self):
        self.ecrire_session("recherche", "2026-11-01-un.md", "Un")
        rc, out = jouer(self.racine)
        self.assertEqual(rc, 1, out)
        self.assertIn("2026-11-01-un.md : un même ticket à 2 endroits", out)

    def test_l_index_porte_le_dossier_de_chaque_ticket(self):
        self.ecrire_session("recherche", "2026-11-10-s.md", "S")
        rc, out = jouer(self.racine, "--index")
        self.assertEqual(rc, 0, out)
        with open(os.path.join(self.racine, "docs", "tickets", "INDEX.md"), encoding="utf-8") as f:
            index = f.read()
        self.assertIn("## Ouverts — moteur (2)", index)
        self.assertIn("| [S](../recherche/tickets/2026-11-10-s.md) | `docs/recherche/tickets` |", index)
        self.assertIn("| [Un](2026-11-01-un.md) | `docs/tickets` |", index)

    def test_un_ticket_deplace_vers_une_session_puis_ferme_se_suit(self):
        r = self.racine
        git(r, "init", "-q", "-b", "master")
        git(r, "add", "-A")
        git(r, "-c", "user.email=t@t", "-c", "user.name=t", "commit", "-q", "-m", "base")
        os.makedirs(os.path.join(r, "docs", "recherche", "tickets"))
        git(r, "mv", "docs/tickets/2026-11-01-un.md", "docs/recherche/tickets/2026-11-01-un.md")
        git(r, "-c", "user.email=t@t", "-c", "user.name=t", "commit", "-q", "-m", "déplacer")
        rc, out = jouer(r, "--lot", "HEAD~1")
        self.assertEqual(rc, 0, out)
        self.assertIn("déplacé  recherche/tickets/2026-11-01-un.md (depuis 2026-11-01-un.md)", out)
        self.assertNotIn("ouvert ", out)
        self.assertNotIn("fermé    ", out)
        self.assertIn("2 en-têtes valides (1 ouverts, 1 fermés)", out)
        # Fermer depuis la session : git mv vers docs/tickets/closed/, l'État réécrit.
        git(r, "mv", "docs/recherche/tickets/2026-11-01-un.md", "docs/tickets/closed/2026-11-01-un.md")
        ecrire(r, "closed/2026-11-01-un.md", "Un", etat="corrigé `def`")
        git(r, "add", "-A")
        git(r, "-c", "user.email=t@t", "-c", "user.name=t", "commit", "-q", "-m", "fermer")
        rc, out = jouer(r, "--lot", "HEAD~1")
        self.assertEqual(rc, 0, out)
        self.assertIn("fermé    closed/2026-11-01-un.md", out)
        self.assertNotIn("retiré", out)
        # Depuis la base : le lot entier ne compte ni ouvert ni retiré, seulement la fermeture.
        rc, out = jouer(r, "--lot", "HEAD~2")
        self.assertIn("(1), à relire", out)
        self.assertIn("fermé    closed/2026-11-01-un.md", out)

    def test_un_ticket_de_session_reecrit_en_le_deplacant_reste_un_deplacement(self):
        r = self.racine
        git(r, "init", "-q", "-b", "master")
        git(r, "add", "-A")
        git(r, "-c", "user.email=t@t", "-c", "user.name=t", "commit", "-q", "-m", "base")
        os.remove(os.path.join(r, "docs/tickets/2026-11-01-un.md"))
        self.ecrire_session("banc", "2026-11-01-un.md", "Un")
        with open(os.path.join(r, "docs/banc/tickets/2026-11-01-un.md"), "a", encoding="utf-8") as f:
            f.write("\n".join(["Un texte neuf."] * 40))
        git(r, "add", "-A")
        git(r, "-c", "user.email=t@t", "-c", "user.name=t", "commit", "-q", "-m", "déplacer en réécrivant")
        rc, out = jouer(r, "--lot", "HEAD~1")
        self.assertEqual(rc, 0, out)
        self.assertIn("déplacé  banc/tickets/2026-11-01-un.md (depuis 2026-11-01-un.md)", out)
        self.assertNotIn("ouvert ", out)
        self.assertNotIn("retiré", out)

    def test_le_lot_se_compare_a_sa_fourche_pas_a_la_pointe_de_master(self):
        # Le lot part de master ; master avance ensuite (un autre lot y déplace un ticket). Le
        # contrôle --lot master ne doit lister que ce que le lot fait lui-même.
        r = self.racine
        git(r, "init", "-q", "-b", "master")
        git(r, "add", "-A")
        git(r, "-c", "user.email=t@t", "-c", "user.name=t", "commit", "-q", "-m", "base")
        git(r, "checkout", "-q", "-b", "lot")
        ecrire(r, "2026-11-11-du-lot.md", "Du lot")
        git(r, "add", "-A")
        git(r, "-c", "user.email=t@t", "-c", "user.name=t", "commit", "-q", "-m", "lot")
        git(r, "checkout", "-q", "master")
        os.makedirs(os.path.join(r, "docs", "optimiseur", "tickets"))
        git(r, "mv", "docs/tickets/2026-11-01-un.md", "docs/optimiseur/tickets/2026-11-01-un.md")
        git(r, "-c", "user.email=t@t", "-c", "user.name=t", "commit", "-q", "-m", "un autre lot")
        git(r, "checkout", "-q", "lot")
        rc, out = jouer(r, "--lot", "master")
        self.assertEqual(rc, 0, out)
        self.assertIn("(1), à relire", out)
        self.assertIn("ouvert   2026-11-11-du-lot.md", out)
        self.assertNotIn("2026-11-01-un.md", out)


if __name__ == "__main__":
    unittest.main(verbosity=1)
