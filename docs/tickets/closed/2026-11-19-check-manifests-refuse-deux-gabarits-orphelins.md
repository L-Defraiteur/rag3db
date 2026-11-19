# check-manifests refuse deux gabarits de code orphelins

- **État** : fermé (19 novembre 2026, ED, sur le oui de Lucie) — sept copies effacées : `atelier/templates/backends/{code,notebook,validated-result,memory}` et `memoire-longue/backends/{code,validated-result,atelier}`. **`memoire-longue/backends/notebook` est gardé** : le backend `memory` y prend ses graphes `../notebook/graphs/put.mmd` et `get.mmd` ; la proposition du point 1 l'effaçait à tort, et le contrôle d'avant l'effacement l'a vu. Ensuite, `check-manifests` du binaire posé (72a02fdd1) : 3 arbres inscrits, 0 refusé ; l'atelier et la mémoire longue chargent ok.
- **Famille** : produit
- **Gravité** : du bruit. Chaque contrôle par `check-manifests` dit « REFUSÉ » pour deux arbres que rien n'ouvre, et
  un vrai refus peut s'y perdre.
- **Atteignable en service** : non. Ni l'un ni l'autre n'est hébergé, et aucun n'a de base.
- **Touche rag3weaver** : non. Ce sont deux dossiers de configuration de Lucie, hors du dépôt.
- **Pour** : everything declarative (ED), sur la décision de c7 du 19 novembre.
- **Signalé par** : G, à la réécriture `access`/`provider` des manifestes (19 novembre, 10 h 37). Les refus étaient
  les mêmes avant et après cette réécriture.

## Le relevé (ED, 19 novembre 2026)

| Chemin | Ce que c'est | Base | Racine | Hébergé |
|---|---|---|---|---|
| `~/.config/rag3weaver/atelier/templates/backends/code/` | une copie du gabarit `code` du dépôt (backend.json, snapshot.json, README, depots, graphs, plans), posée le 9 nov. à 09:37 | `data/code-poste.rag3db` absente | `workspace` absente | non |
| `~/.config/rag3weaver/memoire-longue/backends/code/` | la même copie, posée le 15 nov. à 04:00 | absente | absente | non |

Chaque dossier porte en fait une copie de **tous** les gabarits de backends : `atelier`, `code`, `memory`,
`notebook` et `validated-result`. L'atelier n'utilise que `atelier`, la mémoire longue que `memory`. Le reste est
venu avec la copie, et seul `code` se fait refuser : il est le seul à déclarer une racine `workspace` relative, qui
n'existe pas.

**Pourquoi celui de l'atelier compte comme un arbre inscrit.** Sans argument, `check-manifests` lit
`~/.config/rag3weaver/*/templates/backends/code/*.json` (`code_tree::Roots::registered_manifests`). L'atelier range
ses gabarits sous `templates/`, donc sa copie de `code` entre dans ce motif. Celle de la mémoire longue n'y entre pas
(`backends/code`, sans `templates/`) : G l'a passée à la main.

L'hôte ne les ouvre pas : aucun backend de `/etat` ne vise l'un de ces deux manifestes.

## Ce que je propose

1. **Les désinscrire en les effaçant**, avec leurs voisins inutiles, après le oui de Lucie :
   `~/.config/rag3weaver/atelier/templates/backends/{code,notebook,validated-result,memory}` et
   `~/.config/rag3weaver/memoire-longue/backends/{code,notebook,validated-result,atelier}`. Garder seulement le
   backend que chaque dossier sert (`atelier` pour l'un, `memory` pour l'autre). Rien n'y est écrit : ce sont des
   copies du dépôt, que `builtin_templates` sait reposer.
2. **Leur rendre une racine** n'a pas de sens : ce ne sont pas des arbres de code, et aucune base ne les attend.

**Question pour Lucie (n° 1)** : effacer ces copies, selon le point 1 ? Avant de le faire, je reprends le relevé
(aucune base, rien d'hébergé), et je refais `check-manifests` après : il ne doit plus rien refuser.

## Comment le fermer

Les copies effacées sur son oui, puis `check-manifests` sans argument et sur les deux chemins, sans refus.
