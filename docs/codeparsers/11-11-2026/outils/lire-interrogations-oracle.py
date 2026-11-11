#!/usr/bin/env python3
"""Relire les TSV de l'oracle figé sans modifier ses entrées ni ses verdicts.

La colonne serveur ne conserve que deux positions : les catégories de corpus
portent sur ces positions visibles, pas sur toute la réponse LSP originale.
"""
import argparse
import collections
import csv
import json
import re
from pathlib import Path


def lire(racine, chemin):
    compte = collections.Counter()
    details = []
    with chemin.open() as fichier:
        lignes = list(csv.DictReader(fichier, delimiter="\t"))
    if not lignes:
        raise ValueError(f"aucun verdict : {chemin}")
    for ligne in lignes:
        source, numero = ligne["fichier:ligne"].rsplit(":", 1)
        texte = (racine / source).read_text(errors="replace").split("\n")
        numero = int(numero)
        if not 1 <= numero <= len(texte):
            raise ValueError(f"position source invalide : {source}:{numero}")
        # Même condition que l'oracle figé avant toute interrogation LSP.
        token_present = bool(re.search(r"\b" + re.escape(ligne["nom"]) + r"\b", texte[numero - 1]))
        positions = ligne["serveur"].split(";") if ligne["serveur"] else []
        if not token_present:
            if positions:
                raise ValueError(f"réponse malgré token absent : {source}:{numero}")
            question = "non_interroge_token_source_absent"
        elif not positions:
            question = "lsp_interroge_sans_reponse"
        else:
            question = "lsp_interroge_avec_reponse"
        dans_corpus = []
        for position in positions:
            fichier_cible, _ = position.rsplit(":", 1)
            dans_corpus.append((racine / fichier_cible).resolve().is_relative_to(racine))
        corpus = ("sans_reponse_visible" if not positions else
                  "hors_corpus_visible" if not any(dans_corpus) else
                  "dans_corpus_visible" if all(dans_corpus) else "mixte_visible")
        cle = (question, corpus, ligne["verdict"])
        compte[cle] += 1
        details.append({**ligne, "interrogation": question, "corpus_positions_visibles": corpus})
    return {"preuve": str(chemin), "total": len(lignes),
            "limite": "au plus deux positions de réponse conservées dans chaque TSV",
            "comptes": [{"interrogation": q, "corpus_positions_visibles": c,
                         "verdict_original": v, "nombre": n}
                        for (q, c, v), n in sorted(compte.items())],
            "lignes": details}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("racine", type=Path)
    parser.add_argument("verdicts", nargs="+", type=Path)
    args = parser.parse_args()
    racine = args.racine.resolve(strict=True)
    for chemin in args.verdicts:
        print(json.dumps(lire(racine, chemin), ensure_ascii=False))


if __name__ == "__main__":
    main()
