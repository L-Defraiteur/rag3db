#!/usr/bin/env python3
"""**L'index des tickets dit vrai** : chaque en-tête de table compte ses lignes,
chaque ligne mène à un fichier qui existe, et chaque fichier a sa ligne.

Les en-têtes « Ouverts — moteur (N) », « Ouverts — produit (N) » et
« Fermés (N) » de `docs/tickets/README.md` étaient corrigés à la main et se
décalaient d'un rebase à l'autre (43 annoncés pour 24 lignes, 11 octobre
2026). Ce script recompte sur les fichiers et refuse en nommant ce qui
diffère ; `check_features.sh` l'appelle avant tout hash.

Usage : scripts/check_tickets.py [racine du dépôt]   (rc 0 : vrai ; 1 : refus)
"""
import os
import re
import sys

ENTETE = re.compile(r"^### (Ouverts — \S+|Fermés) \((\d+)\)")
LIEN = re.compile(r"^\| \[[^\]]*\]\(([^)]+)\)")


def lire_les_tables(readme):
    """Les tables de l'index : titre → (compte annoncé, liens des lignes)."""
    tables, courante = {}, None
    with open(readme, encoding="utf-8") as f:
        for ligne in f:
            m = ENTETE.match(ligne)
            if m:
                courante = m.group(1)
                if courante in tables:
                    raise SystemExit(f"REFUS : la table « {courante} » paraît deux fois dans {readme}")
                tables[courante] = (int(m.group(2)), [])
                continue
            if ligne.startswith("#"):
                courante = None
                continue
            m = LIEN.match(ligne)
            if m and courante:
                tables[courante][1].append(m.group(1))
    return tables


def verifier(racine):
    dossier = os.path.join(racine, "docs", "tickets")
    readme = os.path.join(dossier, "README.md")
    tables = lire_les_tables(readme)
    refus = []
    if not any(t.startswith("Ouverts") for t in tables) or "Fermés" not in tables:
        refus.append(f"tables trouvées : {sorted(tables)} ; il faut au moins une table « Ouverts — … » et « Fermés »")

    ouverts_lignes, fermes_lignes = [], []
    for titre, (annonce, liens) in tables.items():
        if annonce != len(liens):
            refus.append(f"« {titre} » annonce {annonce} et compte {len(liens)} lignes")
        vus = set()
        for lien in liens:
            if lien in vus:
                refus.append(f"« {titre} » : la ligne de {lien} paraît deux fois")
            vus.add(lien)
            chemin = os.path.normpath(os.path.join(dossier, lien))
            if not os.path.isfile(chemin):
                refus.append(f"« {titre} » : lien mort {lien}")
            elif titre == "Fermés" and os.path.dirname(chemin) != os.path.join(dossier, "closed"):
                refus.append(f"« Fermés » : {lien} n'est pas dans closed/")
            elif titre != "Fermés" and os.path.dirname(chemin) != dossier:
                refus.append(f"« {titre} » : {lien} n'est pas un ticket ouvert (hors de docs/tickets/)")
        (fermes_lignes if titre == "Fermés" else ouverts_lignes).extend(os.path.normpath(os.path.join(dossier, l)) for l in liens)

    def fichiers(d):
        return {os.path.join(d, n) for n in os.listdir(d) if n.endswith(".md") and n != "README.md"}

    for nom, d, lignes in (("ouvert", dossier, ouverts_lignes), ("fermé", os.path.join(dossier, "closed"), fermes_lignes)):
        for f in sorted(fichiers(d) - set(lignes)):
            refus.append(f"ticket {nom} sans ligne dans l'index : {os.path.relpath(f, dossier)}")
    deux_fois = {l for l in ouverts_lignes if ouverts_lignes.count(l) > 1}
    for l in sorted(deux_fois):
        refus.append(f"{os.path.relpath(l, dossier)} est dans deux tables d'ouverts")
    return tables, refus


def main():
    racine = sys.argv[1] if len(sys.argv) > 1 else os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "..")
    tables, refus = verifier(os.path.abspath(racine))
    if refus:
        print("REFUS index des tickets (docs/tickets/README.md) :")
        for r in refus:
            print(f"  - {r}")
        sys.exit(1)
    print("VERT  index des tickets : " + ", ".join(f"{t} {n}" for t, (n, _) in tables.items()))


if __name__ == "__main__":
    main()
