#!/usr/bin/env python3
"""Ramasse les miettes que cargo ne ramasse jamais.

Appelé au **début** de chaque passe par `run_e2e.sh` : les artefacts périmés ne
vivent donc jamais plus d'une passe, et la question ne se repose pas.

Le 27 août 2026, `target/` pesait 580 Go — dont 367 Go de binaires de test
périmés et 124 Go d'incrémental — sur un disque à 94 %. Cargo suffixe chaque
artefact d'une empreinte et ne supprime jamais les anciennes : à trois passes
par jour, c'est une centaine de gigas quotidiens.

**Ce qu'on garde** : le plus récent de chaque famille — c'est celui du dernier
build — et **toutes les dépendances externes**. Serde, burn, tokio et les cinq
cents autres sont stables ; les effacer coûterait une reconstruction complète
de l'arbre pour ne rien gagner. La règle naïve « supprime ce qui est vieux »
tape précisément là.

`RAG3WEAVER_NO_GC=1` pour ne rien faire.
"""
import os, re, sys, subprocess, collections

NOTRES = re.compile(r'^(librag3weaver|rag3weaver|e2e_[a-z0-9_]+)-[0-9a-f]{16}(\..*)?$')


def go(n: int) -> float:
    return n / 2 ** 30


def menage(debug_dir: str, verbeux: bool = True) -> int:
    """Rend le nombre d'octets libérés. Ne lève jamais : un ménage qui échoue
    ne doit pas empêcher une passe de tourner."""
    libere = 0

    inc = os.path.join(debug_dir, "incremental")
    if os.path.isdir(inc):
        taille = 0
        for d, _, fs in os.walk(inc):
            for f in fs:
                try:
                    taille += os.path.getsize(os.path.join(d, f))
                except OSError:
                    pass
        if subprocess.run(["rm", "-rf", inc]).returncode == 0:
            libere += taille

    deps = os.path.join(debug_dir, "deps")
    n = 0
    if os.path.isdir(deps):
        familles = collections.defaultdict(list)
        for f in os.listdir(deps):
            m = NOTRES.match(f)
            if not m:
                continue
            p = os.path.join(deps, f)
            if not os.path.isfile(p):
                continue
            # famille + extension : un `.rlib` et un `.d` ne se remplacent pas.
            familles[(m.group(1), m.group(2) or '')].append((os.path.getmtime(p), os.path.getsize(p), p))
        for fichiers in familles.values():
            fichiers.sort(reverse=True)
            for _, taille, p in fichiers[1:]:
                try:
                    os.remove(p)
                    libere += taille
                    n += 1
                except OSError:
                    pass

    if verbeux and libere:
        print(f"[ménage] {n} artefacts périmés + l'incrémental — {go(libere):.1f} Go libérés")
    return libere


if __name__ == "__main__":
    if os.environ.get("RAG3WEAVER_NO_GC"):
        sys.exit(0)
    racine = sys.argv[1] if len(sys.argv) > 1 else os.path.join(os.path.dirname(os.path.abspath(__file__)), "target")
    # Jamais pendant une passe : elle travaille précisément là-dedans, et une
    # passe dont les binaires bougent sous elle n'est pas une passe.
    if subprocess.run(["pgrep", "-f", "[c]argo test --features rag3db-native"],
                      capture_output=True).returncode == 0:
        sys.exit(0)
    menage(os.path.join(racine, "debug"))
