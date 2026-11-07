#!/usr/bin/env python3
"""**Chaque ticket dit ce qu'il est, dans son en-tête**, et l'index se génère.

Jusqu'au 7 novembre 2026, `docs/tickets/README.md` portait une table par famille,
une ligne par ticket et un compte épinglé dans chaque titre (« Ouverts — moteur
(41) »). Le compte forçait à relire, mais deux lots qui ouvraient chacun un
ticket écrivaient le même titre avec deux nombres, et le second rebase
s'arrêtait : quatre lots sur cinq le 7 novembre au soir (ticket
2026-11-07-l-index-des-tickets-epingle-ses-comptes-et-fait-un-conflit-a-chaque-lot).

La garde passe dans chaque ticket. Son en-tête (les lignes « - **Champ** :
valeur » avant la première section) porte au moins :

- **État** : « ouvert … » pour un ticket de `docs/tickets/`, et pour un ticket de
  `docs/tickets/closed/` un premier mot qui dit qu'il est clos (corrigé, fermé,
  fait, résolu, abandonné, non reproduit, accepté) ;
- **Famille**, pour un ticket ouvert : `moteur` (le correctif vit dans `src/`, ses extensions C++
  ou son harnais) ou `produit` (`extension/rag3weaver`, codeparsers, lucivy, les outils) ;
- **Gravité**, **Atteignable en service**, **Touche rag3weaver**.

L'index n'est plus écrit à la main ni suivi par git : `--index` l'écrit dans
`docs/tickets/INDEX.md` (ignoré par git), trié par nom, les comptes calculés.

Usage :
  scripts/check_tickets.py [racine]               rc 0 : les en-têtes disent vrai ; 1 : refus nommés
  scripts/check_tickets.py --index [racine]       écrit docs/tickets/INDEX.md, puis contrôle
  scripts/check_tickets.py --lot <base> [racine]  les tickets que le lot ouvre, ferme ou change
                                                  d'état depuis <base>, à relire avant le push
"""
import os
import re
import subprocess
import sys

CHAMP = re.compile(r"^- \*\*([^*]+?)\s*:?\s*\*\*\s*:?\s*(.*)$")
OBLIGATOIRES = ("État", "Famille", "Gravité", "Atteignable en service", "Touche rag3weaver")
FAMILLES = ("moteur", "produit")
CLOS = ("corrigé", "corrigée", "fermé", "fermée", "fait", "faite", "résolu", "résolue", "abandonné",
        "abandonnée", "non", "accepté", "acceptée", "dissous", "dissoute", "remplacé", "remplacée")


def entete(chemin):
    """Le titre et les champs de l'en-tête d'un ticket (noms ramenés à leur forme canonique)."""
    titre, champs, courant = None, {}, None
    canon = {c.lower(): c for c in OBLIGATOIRES + ("Pour", "Ouvert le")}
    with open(chemin, encoding="utf-8") as f:
        for ligne in f:
            ligne = ligne.rstrip("\n")
            if titre is None and ligne.startswith("# "):
                titre = ligne[2:].strip()
                continue
            if ligne.startswith("## "):
                break
            m = CHAMP.match(ligne)
            if m:
                nom = canon.get(m.group(1).strip().lower(), m.group(1).strip())
                champs[nom] = m.group(2).strip()
                courant = nom
            elif courant and ligne.startswith("  ") and ligne.strip():
                champs[courant] += " " + ligne.strip()
            else:
                courant = None
    return titre, champs


def premier_mot(valeur):
    m = re.match(r"\**\s*([^\s,;.:—*()]+)", valeur)
    return m.group(1).lower() if m else ""


def tickets(racine):
    """(chemin relatif à docs/tickets, clos ?) pour chaque ticket."""
    dossier = os.path.join(racine, "docs", "tickets")
    out = []
    for sous, clos in (("", False), ("closed", True)):
        d = os.path.join(dossier, sous)
        for nom in sorted(os.listdir(d)):
            if nom.endswith(".md") and nom not in ("README.md", "INDEX.md"):
                out.append((os.path.join(sous, nom) if sous else nom, clos))
    return dossier, out


def verifier(racine):
    dossier, liste = tickets(racine)
    refus, lignes = [], []
    for rel, clos in liste:
        titre, champs = entete(os.path.join(dossier, rel))
        if not titre:
            refus.append(f"{rel} : pas de titre « # … » en première ligne")
        for c in OBLIGATOIRES:
            if c == "Famille" and clos:
                continue  # la table des fermés ne se partage pas par famille
            if not champs.get(c):
                refus.append(f"{rel} : champ « {c} » absent de l'en-tête")
        famille = champs.get("Famille", "").split()[0].strip(".,;") if champs.get("Famille") else ""
        if champs.get("Famille") and famille not in FAMILLES:
            refus.append(f"{rel} : Famille « {champs['Famille']} », il faut moteur ou produit")
        mot = premier_mot(champs.get("État", ""))
        if champs.get("État"):
            if clos and mot not in CLOS:
                refus.append(f"{rel} : rangé dans closed/ mais son État commence par « {mot} »")
            if not clos and mot in CLOS:
                refus.append(f"{rel} : son État dit « {mot} » mais il n'est pas dans closed/")
        lignes.append((rel, clos, famille, titre or rel, champs))
    return lignes, refus


def court(texte, n=150):
    texte = re.sub(r"\s+", " ", texte).strip()
    return texte if len(texte) <= n else texte[: n - 1].rstrip() + "…"


def ecrire_index(dossier, lignes):
    tables = {"moteur": [], "produit": [], "fermés": []}
    for rel, clos, famille, titre, champs in lignes:
        cle = "fermés" if clos else (famille if famille in FAMILLES else "moteur")
        tables[cle].append((rel, titre, champs))
    out = ["# Index des tickets (généré)", "",
           "Écrit par `extension/rag3weaver/scripts/check_tickets.py --index` depuis l'en-tête de",
           "chaque ticket ; ignoré par git. Ne pas l'éditer : éditer l'en-tête du ticket.", ""]
    for cle, titre_table in (("moteur", "Ouverts — moteur"), ("produit", "Ouverts — produit"), ("fermés", "Fermés")):
        out += [f"## {titre_table} ({len(tables[cle])})", "",
                "| ticket | état | gravité | atteignable en service | touche rag3weaver |", "|---|---|---|---|---|"]
        for rel, titre, champs in tables[cle]:
            cellules = [court(champs.get(c, ""), n).replace("|", "\\|") for c, n in
                        (("État", 160), ("Gravité", 90), ("Atteignable en service", 70), ("Touche rag3weaver", 70))]
            out.append(f"| [{titre.replace('|', '/')}]({rel}) | " + " | ".join(cellules) + " |")
        out.append("")
    chemin = os.path.join(dossier, "INDEX.md")
    with open(chemin, "w", encoding="utf-8") as f:
        f.write("\n".join(out))
    return chemin, {k: len(v) for k, v in tables.items()}


def relire_le_lot(racine, base):
    """Les tickets que le lot ouvre, ferme ou change d'état depuis base."""
    diff = subprocess.run(["git", "-C", racine, "diff", "--name-status", "-M", base, "HEAD", "--", "docs/tickets"],
                          capture_output=True, text=True, check=True).stdout.splitlines()
    vus = []
    for ligne in diff:
        parts = ligne.split("\t")
        statut, chemins = parts[0], parts[1:]
        cible = chemins[-1]
        if not cible.endswith(".md") or os.path.basename(cible) in ("README.md", "INDEX.md"):
            continue
        rel = cible[len("docs/tickets/"):]
        if statut.startswith("A"):
            vus.append(f"ouvert   {rel}")
        elif statut.startswith("D"):
            vus.append(f"retiré   {rel}")
        elif statut.startswith("R"):
            sens = "fermé   " if "/closed/" in cible and "/closed/" not in chemins[0] else "déplacé "
            vus.append(f"{sens} {rel}")
        else:
            avant = subprocess.run(["git", "-C", racine, "show", f"{base}:{cible}"], capture_output=True, text=True).stdout
            m = re.search(r"^- \*\*État\*\* ?: ?(.*)$", avant, re.M)
            _, champs = entete(os.path.join(racine, cible))
            if m and premier_mot(m.group(1)) != premier_mot(champs.get("État", "")):
                vus.append(f"état     {rel} : « {premier_mot(m.group(1))} » → « {premier_mot(champs.get('État', ''))} »")
    print(f"▸ les tickets du lot depuis {base} ({len(vus)}), à relire avant le push :")
    for v in vus:
        print(f"    {v}")
    return vus


def main():
    args = sys.argv[1:]
    index = "--index" in args
    base = None
    if "--lot" in args:
        i = args.index("--lot")
        base = args[i + 1]
        del args[i:i + 2]
    args = [a for a in args if a != "--index"]
    racine = args[0] if args else os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "..")
    racine = os.path.abspath(racine)
    lignes, refus = verifier(racine)
    dossier = os.path.join(racine, "docs", "tickets")
    if index:
        chemin, comptes = ecrire_index(dossier, lignes)
        print(f"▸ index écrit : {chemin} ({comptes['moteur']} moteur, {comptes['produit']} produit, {comptes['fermés']} fermés)")
    if base:
        relire_le_lot(racine, base)
    if refus:
        print(f"ROUGE  tickets : {len(refus)} refus")
        for r in refus:
            print(f"  - {r}")
        sys.exit(1)
    ouverts = sum(1 for l in lignes if not l[1])
    print(f"VERT  tickets : {len(lignes)} en-têtes valides ({ouverts} ouverts, {len(lignes) - ouverts} fermés)")


if __name__ == "__main__":
    main()
