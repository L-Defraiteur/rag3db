# Les relations d'un index dépendaient du chemin d'écriture : un héritage posé en CONSUMES par le COPY

- **État** : **fermé** le 18 octobre 2026 (arbre principal), ouvert le même jour.
- **Gravité** : **réponse fausse**. Des `HAS_PARENT`/`PARENT_OF` (une méthode C++ définie hors
  ligne, vers sa classe) et des `INHERITS_FROM` manquaient, remplacés sur le même couple par un
  `CONSUMES` générique. `usages`, `impact` et les liens disaient « consomme » au lieu de « hérite
  de » ou « est membre de ».
- **Atteignable en service** : oui, sur tout premier index ou réindexation qui pose au moins
  200 liens d'un même groupe (`COPY_SEUIL`), c'est-à-dire tout dépôt réel.
- **Touche rag3weaver** : oui (`dataflow/record_nodes.rs`, `copier_les_liens`).

## Ce qui a été mesuré

Codeparsers, sur code-lourd (même arbre, binaire et lib), réindexé au seuil du COPY par défaut
contre le seuil haut (`RAG3WEAVER_LINK_COPY_THRESHOLD=1000000000`, sans COPY) :
`HAS_PARENT`/`PARENT_OF` −18, `INHERITS_FROM` −11, `CONSUMES` +17, souvent sur les mêmes couples ;
les 16 autres relations égales, et la base vive identique arête par arête au seuil par défaut :
déterministe. Listes : `~/.cache/codeparsers-notes/pertes-18oct/{ref,bas}-lourd-aretes.txt`.

## La cause

Une même arête `MENTIONS` (scope → symbole) arrivait deux fois dans un drain, avec deux `kind` :
`CONSUMES` depuis la référence d'identifiant, puis `INHERITS_FROM` (ou `HAS_PARENT`) depuis la
clause d'héritage ou le parent hors ligne, poussés après (`code.rs`, l'analyse d'un scope).
- **Le chemin par lots** pose chaque paire par `MERGE … SET r.kind = item.kind` : la **dernière**
  occurrence gagne, et une paire déjà en base reçoit les propriétés du lot.
- **Le COPY** (`copier_les_liens`) gardait la **première** occurrence d'une paire et sautait une
  paire déjà en base. Le `kind` restait `CONSUMES`, et `materialiser_les_symboles`, qui lit ce
  `kind` sur `MENTIONS`, posait `CONSUMES` au lieu de la relation fine.

Ce n'est pas le défaut du moteur du ticket 42a32bd52 (des relations persistantes perdues par un
COPY) : rien ne disparaissait du stockage, c'est la relation choisie qui changeait.

## Le correctif

`copier_les_liens` a la sémantique du chemin par lots :
- le dédoublonnage parcourt le groupe **à rebours** (la dernière occurrence gagne) ;
- une paire déjà en base, quand le groupe a des propriétés à écrire, renvoie le groupe au chemin
  par lots (UNWIND … MERGE … SET). Sans propriété, la sauter revient au même que la fusionner.

## Les témoins

- `dataflow::chemin_d_ecriture_tests::les_aretes_du_premier_index_ne_dependent_pas_du_chemin_d_ecriture` :
  393 fichiers réels (`src/common`, `src/include/common`, antlr4 `tree` et `atn`), premier index
  par `sync_source` en paquets de 64, au seuil 1 (tout par COPY) contre le seuil haut, arête par
  arête. **Rouge avant** : 5 `HAS_PARENT` et leurs 5 `PARENT_OF`, 10 `INHERITS_FROM` en moins,
  10 `CONSUMES` en plus, sur les mêmes couples. Vert après.
- `les_aretes_d_un_index_ne_dependent_pas_du_chemin_d_ecriture` : un petit dépôt synthétique
  ingéré en une fois, qui ne montrait pas l'écart (gardé : il couvre `ingest_code`).

## Ce qui reste

- `RAG3WEAVER_LINK_COPY_THRESHOLD=1000000000` dans l'environnement des hôtes (posé par
  l'orchestration le 18) : à retirer une fois les hôtes rebâtis sur ce correctif.
- Les index déjà bâtis au seuil par défaut gardent leurs `CONSUMES` à tort jusqu'à leur
  réindexation : codeparsers les compte (`e2e_banc_relations`, la sonde des arêtes).
