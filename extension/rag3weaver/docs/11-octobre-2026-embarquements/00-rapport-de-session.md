# Session embarquements (chantier F) — rapport du 11 octobre 2026, après-midi (mis à jour le 12 octobre vers 15 h)

*Suite du rapport de nuit (`11-octobre-2026-01h14/embarquements/01-rapport-de-session.md`).*

## Ce qui est sur master

| hash | lot |
|---|---|
| `26216e3be` | la page du chaînage (`01-chainage-des-formes.md`), sans code |
| `76cf86cbc` | chaînage, lot 1 : le graphe, la vérification commune, le repli découpé, le refus nommé, le rapport `atomic` / `instructions` / `fused` |
| `56a680169` | PostgreSQL : le plancher de rappel `pg_trgm` voyage avec la requête (`SET LOCAL`), ticket fermé |
| `7da725287` | chaînage, lot 2 : la fusion rag3db, prouvée contre le repli |
| `d0aacfcf7` | chaînage, lot 3 : PostgreSQL traduit le saut ; le scénario joue sur le conteneur |
| `e5a93a913` | chaînage, lot 4 : la fusion PostgreSQL en CTE ; une écriture ne se fond jamais en amont |
| `b10b49300` | le témoin des transactions PostgreSQL crée sa table avec `_uuid` ; défaire un lien a son témoin vivant ; trois tickets fermés |
| `d58ff9a95` | le relevé des vrais appelants (page 03) : run_plan ne fondait presque rien, le coût vient des éventails |
| `318e7e2cd`, `3d2fb70b2` | les éventails : la page 04, puis le code (UNION ALL, colonne `__branch`), sur les deux moteurs |
| `b203d2a0d` | deux tickets : le Cypher brut de l'indexation du code, le rendu d'impact instable |
| `0f62e1d60` | impact passe par `run_plan` ; `fanout` capacité déclarée, fausse par défaut (mesurée sans gain de temps en local) ; callees −2 allers-retours |
| `0e69ebbfe` | le relevé du Cypher brut de l'indexation (page 05) |
| `68dbcfc4b`, `da02a612e`, `bcea5ea9e`, `c8fae292a` | les quatre lots du Cypher brut : `Select::by_uuids`, `Predicate::AllOf` et `In`, `Write::Detach`, les crochets des réglages du moteur ; ticket fermé — le gabarit code s'indexe sur PostgreSQL et impact y rend les mêmes lignes que sur rag3db |
| `17cf90306` | le voisinage de `grep` saute vers la table d'arrivée que le catalogue déclare ; la porte de `grep` admet une relation par le catalogue (identifiant et déclarée), plus par une liste en dur ; ticket fermé |
| `821d70a63`, `dc4919150` | où va le temps d'un aller-retour (page 06) : les phases, les fils, `usages` sur l'index entier est un balayage sous semi-masque ; la suite `e2e_mesure_index_entier` |
| `0edeb8bb5` | les lectures des morceaux lus vides sur une copie de l'index |
| `c56cb3aa6`, `3226e7ae4`, `ec1707ee2` | le saut en deux temps retiré de rag3db (usages 7-10 ms en un temps sur le troisième pas du cœur) ; page 06 ; PostgreSQL traduit un saut sans table d'arrivée filtré sur le seul uuid (l'ingestion de code y échouait depuis 8bfdd74d3) |
| `88120a8b3`, `9a252cf29` | le témoin décompose usages dans les deux formes et imprime le saut Scope → Scope ; page 06 (jointure par clé, second niveau) |
| `79e404f8f`, `e412c3df5` | le témoin du saut par `UNWIND` : plans complets et trois écritures (111,7 / 4,3 / 1,1 ms, mêmes lignes) ; page 06 |
| `198aeeaec`, `6f5554868` | le saut en deux temps (`graph_walk::run_hop`, capacité `two_step_hops`, vraie pour rag3db) : `usages` de 355 à 212-239 ms avec le même rendu ; relu par l'arbre principal |

Décisions de Lucie (11 octobre) : pas d'arête d'ordre seul pour commencer, mais
l'arête porte zéro ou plusieurs colonnes ; un graphe non atomique s'exécute et le
dit, refusé seulement si l'appelant exige l'atomicité.

## Ce qui reste

- Les morceaux lus vides : recomptés sur la lib du correctif `scanFiltered` (`e6f626df4`)
  contre un reflink neuf du sceau — 0 sur 128 732, les 11 lectures passent, plus de
  SIGSEGV. Le ticket est fermé par le cœur. Le sceau (`copie-depot-scellee`) reste
  jusqu'à ce que le banc ait reproduit.
- Le saut en deux temps est **retiré de rag3db** (`c56cb3aa6`) : sur les trois pas du
  cœur (jointure par clé `2b11b55ec`, second niveau `97589fc9d`, troisième pas
  `0c0af5bf5`), `usages` sur l'index entier passe de 352 ms à 7-10 ms en un temps,
  devant les deux temps (10-13 ms). Tant que le magasin et le binaire de Lucie n'ont pas
  une lib qui contient `0c0af5bf5`, le un-temps y est plus lent qu'avant le retrait.
- La latence de `/embed` pour l'arbre principal (les trois services de luciepc,
  `lot_conseille`, doux et plein, p99 et max) : quand luciepc est libre.
- La fusion des éventails ne se lève que sur un montage distant mesuré.
- L'entrée `$items` par arête et l'arête d'ordre seul attendent un appelant.
- La cellule dans le filtre de `Detach` de la resynchronisation : l'arbre principal,
  pas d de sa marche 1 bis.
- Les lots 2 à 4 du Cypher brut ont été rejoués sur la lib antérieure à `f024578d5`
  (C++), en le sachant ; le voisinage, lui, sur la lib neuve (A8 + A9, 17 h 39), avec
  `e2e_code`, `e2e_postgres`, `e2e_mesure_impact` et `e2e_chainage` verts.
- PostgreSQL : le ticket « rebâti du plein texte en Cypher » reste ouvert.
- En fin de session : effacer `target-F` ici et `target-embarquements` sur luciepc ;
  le worktree de docs `~/.cache/rag3weaver-build/wt-docs-F`.

## Ce qu'il faut savoir pour reprendre

- Worktree `~/git_workspaces/rag3db-embarquements`, branche `chainage-1`, poussée
  sur master par `git push origin HEAD:master`. Sous-modules à initialiser dans un
  worktree neuf (`git submodule update --init`), sinon cargo ne trouve pas
  `codeparsers`.
- Fusion : `poste push embarquements -- bash ~/.cache/rag3weaver-build/embarquements/pousser.sh <suites…>` (fetch, rebase, rejeu court, master inchangé ou docs seules, push). Un lot qui ferme un ticket passe `FERMER_TICKET` et `FERMER_LIGNE` : un conflit sur le seul README des tickets se résout alors seul (`readme-fermer.py`).
- Scripts : `~/.cache/rag3weaver-build/embarquements/` — `chainage.sh <étiquette>`
  (lib, check --tests, e2e_chainage ; `FILTRE=…` pour la lib, `SANS_E2E=1`),
  `plancher.sh` (e2e_postgres sur le conteneur), `deux-suites.sh <suites…>`. Tout
  sous `~/.cache/rag3weaver-build/poste lourd`.
- Le dialecte rag3db s'appelle `rag3db`, celui de PostgreSQL `postgresql` (le nom
  qui paraît dans les refus).
- Une fusion Cypher repose sur le `WITH` qui fait sortir de portée les variables
  d'amont : l'aval redéclare `m`, `d`, `r`, `u` sans conflit, et les prédicats
  compilés (`Predicate::Compiled`, sur l'alias `m`) restent justes.
- Un ticket qui se ferme : `git mv` vers `closed/` **et** sa ligne déplacée dans
  `docs/tickets/README.md`, dans le même commit.
