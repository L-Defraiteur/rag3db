#!/usr/bin/env python3
"""Classe les relations d'usage qu'une révision de codeparsers retire.

Entrées : deux sorties de l'exemple `relations_tsv` de codeparsers (avant,
après), sur les mêmes fichiers. Une relation est identifiée par (type, uuid
source, uuid cible), comme rag3weaver la dédoublonne. Chaque relation retirée
reçoit une cause, lue dans la source à la ligne de ses sites ; « autre » veut
dire qu'aucune cause connue ne l'explique, et c'est à regarder.

    classer_aretes_retirees.py avant.tsv après.tsv [--racine R] [--exemples 10] [--liste sortie.tsv]

`aretes_retirees.sh` bâtit les deux révisions et appelle ce script.
"""

import argparse
import collections
import os
import re

USAGES = ("CONSUMES", "IMPLEMENTS", "INHERITSFROM")


def charger(chemin):
    rels = {}
    for ligne in open(chemin, encoding="utf-8"):
        champs = ligne.rstrip("\n").split("\t")
        if len(champs) < 7:
            continue
        t, de_u, vers_u, de, vers, fichier, sites = champs[:7]
        e = rels.setdefault((t, de_u, vers_u), {"de": de, "vers": vers, "fichier": fichier, "sites": set()})
        e["sites"].update(s for s in sites.split(",") if s)
    return rels


class Sources:
    def __init__(self, racine):
        self.racine, self.textes = racine, {}

    def texte(self, fichier):
        chemin = fichier if os.path.isabs(fichier) else os.path.join(self.racine, fichier)
        if chemin not in self.textes:
            self.textes[chemin] = open(chemin, encoding="utf-8").read()
        return self.textes[chemin]

    def ligne(self, fichier, n):
        lignes = self.texte(fichier).split("\n")
        return lignes[n - 1] if 0 < n <= len(lignes) else ""


def commentaire(ligne):
    t = ligne.strip()
    return t.startswith("//") or t.startswith("/*") or t.startswith("*") or t.startswith("# ")


def cause(cle, e, apres, sources):
    t, de_u, vers_u = cle
    nom = re.escape(e["vers"])
    lignes = [int(s.split("@")[1]) for s in e["sites"]]
    textes = [sources.ligne(e["fichier"], n) for n in lignes]
    if t != "CONSUMES" and ("CONSUMES", de_u, vers_u) in apres:
        return "héritage deviné, devenu CONSUMES"
    if e["de"] == e["vers"]:
        return "impl vers son propre type"
    if textes and all(commentaire(x) for x in textes):
        return "mot de commentaire"
    # Avant f0faa82, les lignes des zones hors scope glissaient vers le haut
    # d'autant de lignes vides que la zone en avait en tête.
    if e["de"].startswith("file_scope") and all(
        any(commentaire(sources.ligne(e["fichier"], n + d)) and re.search(r"\b" + nom + r"\b", sources.ligne(e["fichier"], n + d)) for d in range(4))
        for n in lignes
    ):
        return "mot de commentaire (ligne décalée avant le correctif)"
    src = sources.texte(e["fichier"])
    if re.search(r"\blet\s+[^=;]*\b" + nom + r"\b", src) or re.search(r"\bfor\s+[^{]*\b" + nom + r"\b[^{]*\bin\b", src) or re.search(r"\b" + nom + r"\s*:=", src):
        return "variable locale (let, for, :=, fermetures comprises)"
    return "autre"


def main():
    p = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    p.add_argument("avant")
    p.add_argument("apres")
    p.add_argument("--racine", default=".")
    p.add_argument("--exemples", type=int, default=10)
    p.add_argument("--liste")
    a = p.parse_args()

    avant, apres = charger(a.avant), charger(a.apres)
    sources = Sources(a.racine)
    retirees = [k for k in avant if k not in apres and k[0] in USAGES]
    ajoutees = [k for k in apres if k not in avant and k[0] in USAGES]
    par_cause = collections.defaultdict(list)
    for k in sorted(retirees, key=lambda k: (avant[k]["fichier"], min(int(s.split("@")[1]) for s in avant[k]["sites"]) if avant[k]["sites"] else 0)):
        par_cause[(k[0], cause(k, avant[k], apres, sources))].append(k)

    def compte(rels, t):
        return len({k for k in rels if k[0] == t})

    print(f"relations d'usage distinctes retirées : {len(retirees)} ; ajoutées : {len(ajoutees)}")
    for t in USAGES:
        print(f"  {t} : {compte(avant, t)} → {compte(apres, t)}")
    print()
    print("| Cause | Type | Nombre |")
    print("|---|---|---|")
    for (t, c), ks in sorted(par_cause.items(), key=lambda x: -len(x[1])):
        print(f"| {c} | {t} | {len(ks)} |")
    for (t, c), ks in sorted(par_cause.items(), key=lambda x: -len(x[1])):
        print(f"\n**{c}** ({t}, {len(ks)})\n")
        pas = max(1, len(ks) // a.exemples)
        for k in ks[::pas][: a.exemples]:
            e = avant[k]
            n = min(int(s.split("@")[1]) for s in e["sites"]) if e["sites"] else 0
            fichier = e["fichier"].split("extension/rag3weaver/")[-1]
            print(f"- `{fichier}:{n}` {e['de']} → {e['vers']} — `{sources.ligne(e['fichier'], n).strip()[:90]}`")
    ajout_autres = [k for k in ajoutees if not any(r[1:] == k[1:] for r in retirees if r[0] != "CONSUMES")]
    print(f"\najoutées sans être d'anciens héritages : {len(ajout_autres)}")
    for k in ajout_autres[:20]:
        e = apres[k]
        print(f"- {k[0]} {e['de']} → {e['vers']} {sorted(e['sites'])[:2]}")

    if a.liste:
        with open(a.liste, "w", encoding="utf-8") as o:
            o.write("cause\ttype\tde\tvers\tfichier\tsites\tligne du premier site\n")
            for (t, c), ks in par_cause.items():
                for k in ks:
                    e = avant[k]
                    n = min(int(s.split("@")[1]) for s in e["sites"]) if e["sites"] else 0
                    o.write(f"{c}\t{t}\t{e['de']}\t{e['vers']}\t{e['fichier']}\t{','.join(sorted(e['sites']))}\t{sources.ligne(e['fichier'], n).strip()[:120]}\n")


if __name__ == "__main__":
    main()
