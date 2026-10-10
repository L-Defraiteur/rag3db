# Session codeparsers — rapport

Tenu à jour sur place. Dernière mise à jour : 10 octobre 2026, soir.

Chantier décidé par Lucie : **indexer nos propres dépôts**, la couverture la
plus grande possible de codeparsers, C++ et Rust d'abord, mesurée sur nos
dépôts (rag3db `src/` et `test/`, rag3weaver, codeparsers, l'IR). Points
bloquants et ordre : `../orchestration/04-indexer-nos-propres-depots.md`
(B1 à B9). Mien : B4, B5, B2 (et B1), B3.

## L'évaluation de départ : le C++ de rag3db (codeparsers 4c7897c)

Sonde `~/.cache/rag3weaver-build/sonde-cpp` (projet cargo hors de l'arbre,
dépend de codeparsers par chemin), sur tout `src/` : 1 543 fichiers `.h` /
`.cpp`, 6,3 Mo, 1,2 s.

- **Scopes** (`.h` / `.cpp`) : Namespace 1 825 / 1 441, Class 2 378 / 243,
  Method 3 518 / 377 (corps en ligne), Function 1 351 / 6 243, Lambda
  52 / 322, gabarits 371. Une définition hors classe `A::f` sort avec sa
  classe pour parent (6 449 / 6 620) ; la déclaration du `.h` est un membre
  de la classe (6 691), rag3weaver les réunit (HAS_PARENT, `declarations`).
- **Relations** : en fichier seul (le mode de rag3weaver), CONSUMES 6 143,
  aucune entre fichiers — les liens entre fichiers ne passent que par le
  rendez-vous par le nom ; en mode entre fichiers, CONSUMES 22 822 dont
  16 315 entre fichiers, héritage 620 (459). `#include` : 6 581 références,
  aucune relation fichier → fichier.
- **Pas su comprendre** : nœuds ERROR de tree-sitter dans 215 fichiers,
  1,37 % des octets (72 Ko dans le seul `c_api/rag3db.h`). Des 26 028 appels
  relevés (scope, nom), **11 % reçoivent un CONSUMES en fichier seul**, 36 %
  entre fichiers. Le relevé des non-résolues de codeparsers rendait 0 : B4.
- **« Qui appelle NodeTable::update, qui tient le verrou de l'index avant ? »**
  : 0 appelant relié, même entre fichiers (receveurs C++ non typés :
  `table->update(…)`, `cast<NodeTable>()`, `auto`) ; 188 gardes de verrou
  RAII dans le texte, aucune vue comme référence, pas d'identité pour un
  mutex membre.

## Lots

| Lot | Où | Ce qu'il change |
|---|---|---|
| B4 — le relevé des non-résolues | codeparsers `eb41801` | Chaque nom d'un scope qui ne produit aucune relation (locaux et builtins exceptés) est relevé avec sa raison : aucun scope de ce nom ; défini dans un autre fichier (fichier seul) ; plusieurs candidats ; un candidat écarté. Témoin `tests/non_resolues.rs`, rouge puis vert ; 24 suites vertes. |

## Suite

B5 : le banc de couverture, une ligne de référence commitée par dépôt, qui
s'appuie sur le relevé de B4.
