#!/usr/bin/env python3
"""Le comparateur du juge de parité de search_code (banc, 14 novembre 2026).

Il juge deux relevés écrits par `tests/sonde_parite_search_code.rs` (le même jeu, deux bases) et
nomme chaque écart. Deux régimes, choisis par --regime :

- `optimisation` (par défaut) : la même base, avant et après une optimisation. La parité est au bit
  près : même `_uuid`, même ordre et même score, à chaque rang des k premiers. Le moindre écart est
  ROUGE.
- `migration` : la base d'avant les versions contre la base migrée. Les scores ne peuvent pas être
  égaux, parce que le lexical indexe le contenu dédoublonné et que les statistiques BM25 bougent. On
  juge l'ensemble des k premiers (la part de `_uuid` communs) et le déplacement de rang. Il est ROUGE
  sous --ensemble-min (0,9 par défaut), ou si un résultat bouge de plus de --rang-max rangs (3 par
  défaut).

L'identité d'un résultat est son `_uuid`, et à défaut (file_path, name, start_line).

Usage : parite_search_code.py AVANT.json APRES.json [--regime optimisation|migration]
        [--ensemble-min 0.9] [--rang-max 3]
La dernière ligne est VERT ou ROUGE ; le code de sortie est 0 ou 1.
"""
import argparse
import json
import sys


def cle(r):
    if r.get("_uuid"):
        return ("uuid", r["_uuid"])
    return ("repli", r.get("file_path"), r.get("name"), r.get("start_line"))


def nom(r):
    return f"{r.get('name')} ({r.get('file_path')}:{r.get('start_line')})"


def main():
    p = argparse.ArgumentParser()
    p.add_argument("avant")
    p.add_argument("apres")
    p.add_argument("--regime", choices=["optimisation", "migration"], default="optimisation")
    p.add_argument("--ensemble-min", type=float, default=0.9)
    p.add_argument("--rang-max", type=int, default=3)
    a = p.parse_args()
    A, B = json.load(open(a.avant)), json.load(open(a.apres))
    if A["jeu"] != B["jeu"]:
        print("ROUGE : les deux relevés n'ont pas le même jeu de requêtes")
        return 1
    print(f"avant : {A['base']} ({A['forme_de_la_base']})")
    print(f"après : {B['base']} ({B['forme_de_la_base']})")
    print(f"régime : {a.regime} ; k = {A['k']}")
    par_b = {(x["requete"], x["forme"]): x for x in B["releve"]}
    rouges, cas, communs_total, places_total = 0, 0, 0, 0
    for xa in A["releve"]:
        q, f = xa["requete"], xa["forme"]
        xb = par_b.get((q, f))
        cas += 1
        if xb is None:
            print(f"ROUGE  « {q} » · {f} : absent du second relevé")
            rouges += 1
            continue
        ra, rb = xa["resultats"], xb["resultats"]
        ka, kb = [cle(r) for r in ra], [cle(r) for r in rb]
        ecarts = []
        if a.regime == "optimisation":
            if len(ra) != len(rb):
                ecarts.append(f"{len(ra)} résultats avant, {len(rb)} après")
            for i, (x, y) in enumerate(zip(ra, rb), 1):
                if cle(x) != cle(y):
                    ecarts.append(f"rang {i} : {nom(x)} devient {nom(y)}")
                elif x.get("score") != y.get("score"):
                    ecarts.append(f"rang {i} : {nom(x)}, score {x.get('score')} devient {y.get('score')}")
        else:
            communs = set(ka) & set(kb)
            part = len(communs) / max(1, len(ka))
            communs_total += len(communs)
            places_total += len(ka)
            if part < a.ensemble_min:
                ecarts.append(f"ensemble : {len(communs)} communs sur {len(ka)} ({part:.2f} < {a.ensemble_min})")
            hb = {y.get("content_hash") for y in rb if y.get("content_hash")}
            for x in ra:
                if cle(x) not in set(kb):
                    meme = " ; même contenu qu'un résultat de l'autre relevé" if x.get("content_hash") and x.get("content_hash") in hb else ""
                    ecarts.append(f"sorti : {nom(x)} (rang {x['rang']}){meme}")
            ha = {x.get("content_hash") for x in ra if x.get("content_hash")}
            for y in rb:
                if cle(y) not in set(ka):
                    meme = " ; même contenu qu'un résultat de l'autre relevé" if y.get("content_hash") and y.get("content_hash") in ha else ""
                    ecarts.append(f"entré : {nom(y)} (rang {y['rang']}){meme}")
            for x in ra:
                if cle(x) in set(kb):
                    j = kb.index(cle(x)) + 1
                    if abs(j - x["rang"]) > a.rang_max:
                        ecarts.append(f"rang : {nom(x)} passe de {x['rang']} à {j}")
        dur = [e for e in ecarts if not (a.regime == "migration" and (e.startswith("sorti") or e.startswith("entré")))]
        if dur:
            rouges += 1
            print(f"ROUGE  « {q} » · {f}")
        else:
            print(f"vert   « {q} » · {f}" + (f" ({len(ecarts)} écart(s) nommé(s), sous les tolérances)" if ecarts else ""))
        for e in ecarts:
            print(f"         {e}")
    if a.regime == "migration" and places_total:
        print(f"ensemble, toutes requêtes : {communs_total} communs sur {places_total} ({communs_total / places_total:.2f})")
    print(f"{cas} cas, {rouges} rouge(s)")
    print("VERT" if rouges == 0 else "ROUGE")
    return 0 if rouges == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
