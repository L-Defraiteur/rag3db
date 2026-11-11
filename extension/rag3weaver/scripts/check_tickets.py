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

**Où vit un ticket** (11 novembre 2026) : un ticket ouvert vit soit dans `docs/tickets/`, soit
dans le dossier de la session qui le porte, `docs/<session>/tickets/` (par exemple
`docs/recherche/tickets/`). Un ticket fermé vit dans `docs/tickets/closed/`, d'où qu'il vienne :
fermer, c'est `git mv` vers `docs/tickets/closed/`. Un même nom de fichier ne vit qu'à un endroit.
Les tickets d'une session s'écrivent `<session>/tickets/<nom>.md` dans les messages (chemin depuis
`docs/`), ceux de `docs/tickets/` comme avant (`<nom>.md`, `closed/<nom>.md`). L'index porte le
dossier de chaque ticket ; `--lot` suit un ticket déplacé d'un dossier à l'autre sans le compter
ouvert puis fermé.

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


def est_un_ticket(nom):
    return nom.endswith(".md") and nom not in ("README.md", "INDEX.md")


def classer(chemin_depuis_racine):
    """Pour un chemin « docs/… » : (nom affiché, clos ?, dossier) s'il désigne un ticket, sinon None.

    docs/tickets/<nom>.md → (<nom>.md, False, tickets) ; docs/tickets/closed/<nom>.md →
    (closed/<nom>.md, True, tickets) ; docs/<session>/tickets/<nom>.md →
    (<session>/tickets/<nom>.md, False, <session>)."""
    parts = chemin_depuis_racine.replace(os.sep, "/").split("/")
    if len(parts) < 3 or parts[0] != "docs" or not est_un_ticket(parts[-1]):
        return None
    if parts[1] == "tickets":
        if len(parts) == 3:
            return parts[2], False, "tickets"
        if len(parts) == 4 and parts[2] == "closed":
            return "closed/" + parts[3], True, "tickets"
        return None
    if len(parts) == 4 and parts[2] == "tickets":
        return f"{parts[1]}/tickets/{parts[3]}", False, parts[1]
    return None


def tickets(racine):
    """(nom affiché, chemin absolu, clos ?, dossier) pour chaque ticket, ceux de docs/tickets/
    d'abord (ouverts puis fermés), puis ceux des sessions, triés par dossier puis par nom."""
    docs = os.path.join(racine, "docs")
    candidats = [os.path.join("docs", "tickets", n) for n in os.listdir(os.path.join(docs, "tickets"))]
    candidats += [os.path.join("docs", "tickets", "closed", n) for n in os.listdir(os.path.join(docs, "tickets", "closed"))]
    for session in sorted(os.listdir(docs)):
        d = os.path.join(docs, session, "tickets")
        if session != "tickets" and os.path.isdir(d):
            candidats += [os.path.join("docs", session, "tickets", n) for n in os.listdir(d)]
    out = []
    for rel in candidats:
        c = classer(rel)
        if c and os.path.isfile(os.path.join(racine, rel)):
            out.append((c[0], os.path.join(racine, rel), c[1], c[2]))
    out.sort(key=lambda t: (t[3] != "tickets", t[2], t[3], t[0]))
    return os.path.join(docs, "tickets"), out


def verifier(racine):
    dossier, liste = tickets(racine)
    refus, lignes = [], []
    lieux = {}
    for rel, _, _, _ in liste:
        lieux.setdefault(os.path.basename(rel), []).append(rel)
    for nom, rels in sorted(lieux.items()):
        if len(rels) > 1:
            refus.append(f"{nom} : un même ticket à {len(rels)} endroits ({', '.join(rels)}) ; il n'en vit qu'un")
    for rel, chemin, clos, lieu in liste:
        titre, champs = entete(chemin)
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
        lignes.append((rel, clos, famille, titre or rel, champs, chemin, lieu))
    return lignes, refus


def court(texte, n=150):
    texte = re.sub(r"\s+", " ", texte).strip()
    return texte if len(texte) <= n else texte[: n - 1].rstrip() + "…"


def ecrire_index(dossier, lignes):
    tables = {"moteur": [], "produit": [], "fermés": []}
    for rel, clos, famille, titre, champs, chemin, lieu in lignes:
        cle = "fermés" if clos else (famille if famille in FAMILLES else "moteur")
        tables[cle].append((os.path.relpath(chemin, dossier).replace(os.sep, "/"), titre, champs, lieu))
    out = ["# Index des tickets (généré)", "",
           "Écrit par `extension/rag3weaver/scripts/check_tickets.py --index` depuis l'en-tête de",
           "chaque ticket ; ignoré par git. Ne pas l'éditer : éditer l'en-tête du ticket.", ""]
    for cle, titre_table in (("moteur", "Ouverts — moteur"), ("produit", "Ouverts — produit"), ("fermés", "Fermés")):
        out += [f"## {titre_table} ({len(tables[cle])})", "",
                "| ticket | dossier | état | gravité | atteignable en service | touche rag3weaver |",
                "|---|---|---|---|---|---|"]
        for lien, titre, champs, lieu in tables[cle]:
            cellules = [court(champs.get(c, ""), n).replace("|", "\\|") for c, n in
                        (("État", 160), ("Gravité", 90), ("Atteignable en service", 70), ("Touche rag3weaver", 70))]
            dossier_du_ticket = "docs/tickets" if lieu == "tickets" else f"docs/{lieu}/tickets"
            out.append(f"| [{titre.replace('|', '/')}]({lien}) | `{dossier_du_ticket}` | " + " | ".join(cellules) + " |")
        out.append("")
    chemin = os.path.join(dossier, "INDEX.md")
    with open(chemin, "w", encoding="utf-8") as f:
        f.write("\n".join(out))
    return chemin, {k: len(v) for k, v in tables.items()}


def relire_le_lot(racine, base):
    """Les tickets que le lot ouvre, ferme, déplace ou change d'état depuis base."""
    diff = subprocess.run(["git", "-C", racine, "diff", "--name-status", "-M", base, "HEAD", "--",
                           "docs/tickets", ":(glob)docs/*/tickets/*.md"],
                          capture_output=True, text=True, check=True).stdout.splitlines()
    vus = []
    retires, ajoutes = {}, {}  # nom de fichier → (nom affiché, clos ?)
    for ligne in diff:
        parts = ligne.split("\t")
        statut, chemins = parts[0], parts[1:]
        cible = classer(chemins[-1])
        source = classer(chemins[0]) if len(chemins) > 1 else None
        if statut.startswith("R") and source and cible:
            if not source[1] and cible[1]:
                vus.append(f"fermé    {cible[0]}")
            elif source[1] and not cible[1]:
                vus.append(f"rouvert  {cible[0]}")
            else:
                vus.append(f"déplacé  {cible[0]} (depuis {source[0]})")
            continue
        if statut.startswith("R"):
            # un renommage qui sort des tickets ou y entre : un retrait et un ajout
            if source:
                retires[os.path.basename(chemins[0])] = source[:2]
            if cible:
                ajoutes[os.path.basename(chemins[-1])] = cible[:2]
            continue
        if not cible:
            continue
        if statut.startswith("A"):
            ajoutes[os.path.basename(chemins[-1])] = cible[:2]
        elif statut.startswith("D"):
            retires[os.path.basename(chemins[-1])] = cible[:2]
        else:
            avant = subprocess.run(["git", "-C", racine, "show", f"{base}:{chemins[-1]}"], capture_output=True, text=True).stdout
            m = re.search(r"^- \*\*État\*\* ?: ?(.*)$", avant, re.M)
            _, champs = entete(os.path.join(racine, chemins[-1]))
            if m and premier_mot(m.group(1)) != premier_mot(champs.get("État", "")):
                vus.append(f"état     {cible[0]} : « {premier_mot(m.group(1))} » → « {premier_mot(champs.get('État', ''))} »")
    # Un ticket dont le contenu change beaucoup, git ne le voit pas renommé : un retrait et un ajout de
    # même nom font une fermeture, une réouverture ou un déplacement, jamais un ouvert et un fermé.
    for nom in sorted(set(retires) | set(ajoutes)):
        avant, apres = retires.get(nom), ajoutes.get(nom)
        if avant and apres:
            if not avant[1] and apres[1]:
                vus.append(f"fermé    {apres[0]}")
            elif avant[1] and not apres[1]:
                vus.append(f"rouvert  {apres[0]}")
            else:
                vus.append(f"déplacé  {apres[0]} (depuis {avant[0]})")
        elif apres:
            vus.append(f"ouvert   {apres[0]}")
        else:
            vus.append(f"retiré   {avant[0]}")
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
