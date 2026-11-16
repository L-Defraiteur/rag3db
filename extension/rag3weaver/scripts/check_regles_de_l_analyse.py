#!/usr/bin/env python3
"""**La garde des règles de l'analyse** (page 06 d'everything declarative, § 13). Un lot qui change
un fichier de l'analyse monte `REGLES_DE_L_ANALYSE` (`src/code.rs`) : la version de l'analyse entre
dans l'empreinte indexée des fichiers, et sans montée, une base à générations garderait sur ses
fichiers inchangés ce qu'une analyse d'avant en a tiré. Un lot sans effet sur l'analyse le dit dans
un de ses messages : « sans effet sur l'analyse ».

Le lot est `<base>..HEAD`, la base étant `merge-base HEAD origin/master` par défaut. Une montée fait
repasser toutes les bases à leur prochaine synchronisation : les nouveautés du lot le disent.

Usage : python3 extension/rag3weaver/scripts/check_regles_de_l_analyse.py [--depot D] [--base B]
(rc 0 : vert ; 1 : rouge, avec ce qui manque)
"""
import argparse
import subprocess
import sys

FICHIERS = [
    "extension/rag3weaver/src/code.rs",
    "extension/rag3weaver/src/manifestes.rs",
    "extension/rag3weaver/src/queries_via.rs",
]
CONSTANTE = "REGLES_DE_L_ANALYSE"
MENTION = "sans effet sur l'analyse"


def git(depot, *args, check=True):
    r = subprocess.run(["git", "-C", depot, *args], capture_output=True, text=True)
    if check and r.returncode != 0:
        raise SystemExit(f"check_regles_de_l_analyse : git {' '.join(args)} : {r.stderr.strip()}")
    return r.stdout


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--depot", default=".")
    p.add_argument("--base", default=None)
    a = p.parse_args()
    depot = git(a.depot, "rev-parse", "--show-toplevel").strip()
    base = a.base
    if base is None:
        r = subprocess.run(["git", "-C", depot, "merge-base", "HEAD", "origin/master"], capture_output=True, text=True)
        if r.returncode != 0:
            print("VERT  règles de l'analyse : pas d'origin/master, rien à comparer")
            return 0
        base = r.stdout.strip()
    touches = [f for f in git(depot, "diff", "--name-only", f"{base}..HEAD", "--", *FICHIERS).split() if f]
    if not touches:
        print("VERT  règles de l'analyse : le lot ne touche pas l'analyse")
        return 0
    diff_code = git(depot, "diff", "-U0", f"{base}..HEAD", "--", FICHIERS[0])
    montee = any(l.startswith(("+", "-")) and not l.startswith(("+++", "---")) and CONSTANTE in l for l in diff_code.splitlines())
    if montee:
        print(f"VERT  règles de l'analyse : {CONSTANTE} monte")
        return 0
    messages = git(depot, "log", "--format=%B", f"{base}..HEAD")
    # Sans casse : « Sans effet sur l'analyse » en tête de phrase compte aussi (arbre principal, 16 novembre 2026).
    if MENTION.casefold() in messages.casefold():
        print(f"VERT  règles de l'analyse : « {MENTION} » dit dans le lot")
        return 0
    print(
        f"ROUGE règles de l'analyse : le lot change {', '.join(touches)} sans monter {CONSTANTE} "
        f"(src/code.rs) ni dire « {MENTION} » dans un message. Une montée fait repasser toutes les "
        "bases à leur prochaine synchronisation : dites-le dans les nouveautés."
    )
    return 1


if __name__ == "__main__":
    sys.exit(main())
