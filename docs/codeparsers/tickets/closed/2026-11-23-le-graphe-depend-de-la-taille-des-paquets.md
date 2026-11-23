# Le graphe dépend de la taille des paquets de fichiers

- **État** : fermé le 23 novembre 2026 par le lot `batch-independence` (session codeparsers, règles 20). Voir « Fermeture ».
- **Famille** : produit
- **Gravité** : moyenne. Le même arbre indexé par le même binaire ne donne pas le même graphe selon la taille des
  paquets de fichiers (`RAG3WEAVER_BATCH_FILES`) : des liens et des symboles en plus ou en moins, que rien ne dit.
  Une mesure qui change la taille des paquets compare alors deux graphes différents ; et une borne des paquets en
  durée (le lot de l'arrêt pendant une repasse) rendrait le graphe dépendant de la vitesse de la machine.
- **Atteignable en service** : oui. Toute indexation entière, dès que l'arbre tient en plus d'un paquet.
- **Touche rag3weaver** : oui (`code_sync`, la résolution entre paquets, `Catalog::ingest_code`, les rendez-vous)
- **Pour** : session codeparsers
- **Signalé par** : la session codeparsers (mesure du coût d'`OCCURS`, 23 novembre 2026)

## Constat

Un même instantané de rag3db (fe38b5e5d8, 9 656 fichiers, 8 830 retenus) indexé de rien par le chemin de l'hôte, en
plein texte seul (le banc `worktrees_index_cost_measure.rs` de la branche `codeparsers/occurrences-cost-measure`), sous
`poste mesure` à la tablette, par le même binaire (8e42489ffa, release) :

| | paquets de 2 048 (défaut) | paquets de 128 |
|---|---|---|
| `Symbol` | 73 219 | 73 364 |
| `CONSUMES` | 131 733 | 131 770 |
| `MENTIONS` | 367 650 | 369 657 |
| `OCCURS` | 726 860 | 729 309 |
| `DEFINES` | 99 673 | 99 673 |
| `File`, `Scope` | 8 830, 86 060 | 8 830, 86 060 |

Les fichiers et les scopes sont les mêmes ; les liens et les symboles non. Avec de plus petits paquets, il y en a
**plus** : une part de la résolution dépend de ce qui est déjà en base quand un fichier est écrit.

## Ce qu'il faut

- Un témoin : un petit dépôt à plusieurs fichiers liés indexé d'un seul paquet, puis à un fichier par paquet ; les
  mêmes liens par relation, et les mêmes symboles.
- Trouver ce qui dépend de l'ordre : un rendez-vous (`MENTIONS`) posé quand sa cible n'est pas encore là et qu'un
  paquet suivant ne reprend pas, un symbole créé selon ce qui précède, ou un site d'`OCCURS` compté deux fois.

## Mesure (pour mémoire)

Durée de la passe entière, même instantané et même machine, une passe de chaque côté (bruit non mesuré) :
avant les occurrences (3da33881ce) 120,6 s ; après (8e42489ffa) 131,8 s ; après, paquets de 128 : 185,6 s. Taille de
la base : 1 040,1 Mio, 1 058,3 Mio, 1 056,0 Mio. Journaux à la tablette :
`~/.cache/rag3weaver-build/mesures/occurrences-cout-20261123-001511/`.

## Fermeture (23 novembre 2026, session codeparsers)

**La cause** : `code.rs` écartait le rendez-vous d'un membre d'une bibliothèque sûre (`externes_sures`, provenance
`builtin` ou `dependency`), mais cet ensemble se lisait sur les bibliothèques importées par **les fichiers du
paquet**, dans une table indexée par le seul nom, toutes langues confondues. Trois effets, chacun vu sur l'instantané
puis reproduit par le témoin :

- `tempfile::tempdir()` dans un fichier qui ne fait pas `use tempfile` : externe si un autre fichier du paquet
  importait `tempfile`, rendez-vous sinon ;
- `std::to_string` en C++ : externe quand un fichier Rust du paquet faisait `use std::…` (sa bibliothèque `std`),
  rendez-vous dans un paquet sans Rust ;
- `queue.Empty` en Python : rendez-vous quand un fichier TypeScript du paquet importait un paquet `queue` que le dépôt
  ne déclare pas (la première provenance rencontrée l'emportait).

**Le correctif** : la provenance sûre se décide par le fichier seul. C'est la bibliothèque standard de sa langue
(`est_builtin`), l'espace de noms standard de sa langue (`std` en C++, et dans un `.h`, dit par les règles de
codeparsers : `[builtin] espaces_de_noms`, `provenance::est_espace_standard`), ou une dépendance que son dépôt déclare
(les manifestes, par langue). L'ensemble du paquet est retiré. Un fichier édité seul, resynchronisé dans un paquet d'un
fichier, garde exactement les arêtes de l'index entier.

**Témoin** : `daemon_host::worktrees::batch_independence_tests`. Un petit dépôt Rust, C++, Python et TypeScript est
indexé en un paquet, puis à un fichier par paquet, puis après un fichier édité seul : les mêmes `MENTIONS`, `CONSUMES`
et symboles. Il a été rouge pour chacun des trois effets, puis vert.

**Sur l'instantané de rag3db** (a47ab7f5bb, 2 048 contre 128) : `MENTIONS`, `DEFINES` et symboles sont identiques.
Il reste 41 `CONSUMES` qui diffèrent, tous dans le rattachement d'un usage TypeScript à un homonyme (37 vers un fichier
Rust ou Python, 4 vers un autre homonyme TypeScript) : une autre cause, suivie par
`2026-11-23-un-nom-se-rattache-a-l-homonyme-d-une-autre-langue-selon-le-paquet` (lot 20 des relations fabriquées).

