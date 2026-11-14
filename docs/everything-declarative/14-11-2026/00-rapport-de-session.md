# Everything declarative, 14 novembre 2026 : le rapport de session

## Ce qui est sur master

- **Le lot des octets d'un scope** (`500b1ede4`, rag3db ; `b9029dc`, codeparsers master en avance
  rapide) : le contenu d'un scope est borné à l'empan de son nœud (`node_start_byte`,
  `node_end_byte`), et non à ses lignes. La cause des 844 Mo mesurés par lucivy, avec la page
  `02-le-contenu-d-un-scope-borne-a-ses-octets.md`. Pas de `code-scope-v2` : l'adresse suit le texte.
- **Le plan de 2b-B** (`01-plan-du-chargeur-durable-des-couches-2b-b.md`), avec les décisions de Lucie
  du 14 novembre (§ 7 bis) et la relecture d'arbre principal (§ 7 ter, le pas 2 coupé en 2a′ et 2b′).
- **La page de 2a′** (`03-le-pas-2a-prime-stocker-les-generations-sans-fuite.md`) :
  - l'ancre a des générations ;
  - des lignes `Scope` et `File` immuables, à visibilité par `_couche`, `_depuis` et `_jusqu_a`, sur
    les nœuds et sur les arêtes ;
  - l'uuid par génération pour les lignes neuves seulement ;
  - le relevé des lecteurs, les témoins 0 à 6, la mesure du prédicat (§ 6).
- **La relecture de la page 30** et de la liaison (`0672d624f`) ; le défaut de la parente non suivie,
  corrigé par arbre principal (`912ea2084`).

## Ce qui est en cours

- **2a′-1** (branche `generations-de-l-ancre`, worktree `~/.cache/rag3weaver-build/wt-generations`) :
  la visibilité sans changer aucun comportement. Le feu vert de c7 est donné, avec l'estampille
  `code-scope-v1/content/generations` et `_couche` = l'identifiant de la suivie du backend (`tracked`).
  - Une base existante se lit comme avant, et `/etat` dit « forme sans couches ».
  - Les témoins 4 (le prédicat) et 0 (la non-fuite) sont à poser rouges d'abord.
  - Avant le code, la sonde (`tests/sonde_cout_du_predicat.rs`) cherche la forme de clause d'un saut
    qui tient ×1,3. La conjonction nœuds et arête dans une même clause coûte ×7,3 ; à défaut d'une
    forme qui tienne, c'est un ticket pour le cœur, avec l'EXPLAIN.
- **Ensuite** : 2a′-2 (l'ancre immuable, et la mesure bornée des déplacés sur 50 commits de
  code-lourd), puis 2a′-3 (l'écriture d'une suivie, après la généralisation des sources suivies de
  `docs/orchestration/14-11-2026/02-les-sources-suivies-une-abstraction-pour-les-couches.md`),
  puis 2b′.

## Reprendre

- **La copie de mesure** : `~/.cache/rag3weaver-build/ed-mesure-2a/code-lourd/` à luciepc. C'est un
  reflink de l'instantané de G, `~/.local/share/rag3weaver/instantanes/avant-0c41f9fe6/code-lourd/`,
  qu'on n'ouvre jamais en place. Son `.extensions` est repointé sur la lib 0c41f9fe6.
- **Les scripts** sont à luciepc, sous `~/.cache/rag3weaver-build/ed-navigateur/` : `sonde-bati.sh`
  (sous `poste lourd`), `sonde-mesure.sh` (sous `poste mesure`), et `octets-passe.sh` comme modèle de
  passe. La lib s'exporte par `exporter_la_lib`, avec `RAG3DB_LIB` explicite.
- **Le target** : `target-ed-navigateur`, sur le worktree `~/git_workspaces/rag3db-ed-navigateur`
  à luciepc. Pas de target neuf avant le vert d'archi (openssl).
- **Avant chaque push** : `push_garde.sh --code-apporte <parent du premier commit du lot>
  origin/master` décide du rejeu ; puis `--derive "$PWD"` après le rebase, le dernier pas seul sous
  `poste push` et dans `if grep -qx VERT`, et les trois contrôles.

## À effacer au mot de c7

- **Sur le portable** : les worktrees `wt-installeur`, `wt-equipier-fin`, `wt-premier-appel`,
  `wt-flux-partage`, `wt-octets`, `wt-codeparsers-empan` et `~/git_workspaces/rag3db-ed-navigateur`,
  et le target `target-codeparsers-empan`.
- **À luciepc** : `rag3db-ed-installeur`, `rag3db-ed-equipier` et leurs targets, et la copie de mesure
  une fois 2a′-2 mesuré.
