# Cœur C++ — rapport du 19 au 25 octobre 2026

Session « cœur c++ » (Kuzu bifurqué, `src/`, `extension/vector`, `test/transaction`). Le 19 au
matin puis la reprise du 25 à 13 h 30.

## Sur master

| quand | lot | hash | ce que ça change |
|---|---|---|---|
| 19 oct. 09 h 11 | EXISTS à paramètre avec un autre paramètre dehors → aucune ligne | 8b98f19f7 | la préparation voit les paramètres écrits dans une sous-requête (`ParsedExpressionVisitor::visitChildren` descend dans motifs et WHERE) ; témoin `test/api/prepare_subquery_parameters_test.cpp` ; ticket de l'arbre principal fermé |
| 19 oct. 09 h 53 | COPY : le mot NULL convenu vaut pour toutes les colonnes | dcbaef7b1 | `setVectorNull` ne le consultait que pour STRING (forme de l'amont) ; témoin `copy_null_strings_test.cpp` ; ticket de la mémoire longue fermé, son filet `copier_les_liens` tombé ensuite (388f2260b) |
| 25 oct. 14 h 12 | COMMIT dont le point de reprise échoue après la validation | 58b46a9bf | `TransactionManager::isActive`, `TransactionContext::commit` n'annule plus une transaction que le gestionnaire a retirée ; témoin du banc sorti de `known_red`, prouvé sous ASAN par le banc (3/3, 0 rapport) |
| 25 oct. 15 h 54 | format de stockage 41 : version posée à la main, bâti écrivain, refus nommés | f84433579 | `STORAGE_FORMAT_VERSION` (CATALOG_VERSION_NO), en-tête avec le bâti du dernier écrivain, `FormatRecord` (type 253) en tête du journal, `StorageVersionException` dans les deux sens, base ancienne estampillée au point de reprise ; témoin par octets, compilable sur master ; ticket de Lucie du 18 fermé |
| 25 oct. 16 h 33 | V2-1a : les numéros de colonnes ne changent jamais (format 42) | 4ae060070 | une colonne supprimée laisse un trou (entrée nulle, type ANY) ; plus de `vacuumColumnIDs` ni de `renumberColumns` au point de reprise ; bloc sur disque avec un octet « présente » par numéro ; témoin `stable_column_ids_test.cpp` (9 cas) ; quatre tickets du 19 fermés |

Docs poussées : page 18 (V2-1, onze obstacles et ce qui est tranché, §5 écrit avec le lot) ;
tickets : quatre défauts des numéros de colonnes (tous fermés le 25), la CI rouge (cassert et
format clang). Branche `ci-cassert-geo` (baf19b9e5) prête pour la fenêtre de l'orchestration.

## Ce que les témoins du banc ont trouvé dans V2-1a (et que le mien ne voyait pas)

Trois rouges neufs au compare : `DropBeforeThePrimaryKey` ×2 (signal 11 au point de reprise)
et `AttachedDatabaseIsReadWithItsOwnCatalog` (clé absente). Deux vrais trous dans le lot, quand
la **première** propriété déclarée (avant la clé) est supprimée et que la colonne 0 devient un
trou :
1. un bloc lisait son nombre de lignes sur `chunks[0]` (nul) → `firstPresentChunk()` ;
2. `checkpointDataTypesNoLock` était appelé après la copie vidée et la fusion : la colonne
   supprimée renaissait vide avec son ancien type, et le bloc se relisait à zéro ligne → les
   trous se posent en tête de `NodeGroup::checkpoint` et `CSRNodeGroup::checkpoint`.
Reproduits au shell (compte à 0 après réouverture), deux cas ajoutés au témoin.

## Règles apprises ou reçues

- **Tout commit sur origin dans la minute**, branche à son nom, jamais de force, nom neuf après
  un rebase ou un amendement (Lucie, 25 oct.) : d'où `colonnes-stables` → `-6`, `format-41` →
  `-3`, `commit-transaction-partie` → `-2`.
- Un rouge par refus de compiler n'est pas un rouge : le témoin d'un lot compile sur master et
  rougit à l'exécution (le témoin du format lit et forge lui-même les octets).
- Deux passes en même temps = deux targets cargo (`target-coeur`, `target-coeur-c`) ; partagés,
  `run_e2e.sh` de l'une efface les empreintes de l'autre (cargo 101 sans rapport avec le code).
- La phase rouge d'une passe emprunte au lot son seul témoin et l'inscrit au CMake de master
  elle-même (le CMake du lot nomme des témoins absents de master).
- `git -C rag3db worktree add <chemin relatif>` crée le worktree **dans** l'arbre de Lucie :
  toujours un chemin absolu (égaré retiré le 19 à 09 h 29 sur luciepc).
- `EmptyDBTest` ne crée pas la connexion ; un CSV d'une ligne sans `header=false` est pris pour
  un en-tête ; `DUMMY_TRANSACTION` ne voit pas le catalogue validé (prendre
  `DUMMY_CHECKPOINT_TRANSACTION` hors requête).
- Les écarts du banc jugés sur la base d'origine : l'intermittent de (δ) (course du test, fermé
  par 6fe9ffe9b), `CostPerBatchSettles` (borne de temps sous la charge, passé en probabilistic
  par le banc).

## La bascule (orchestration)

Une base sur laquelle une lib ≥ f84433579 a passé un point de reprise est refusée par toute lib
d'avant ; une lib d'avant 41 n'est plus une cible de retour (seul l'instantané). Courant et les
hôtes restent sur 58b46a9bf jusqu'à la bascule coordonnée : instantanés reflink des bases
partagées hôtes arrêtés, verdict e2e_code, courant aux deux postes ensemble, hôtes redémarrés
annoncés. L'arbre principal a coupé le chemin par lequel `batir.sh` posait courant.

## Ce qui reste à moi

1. `ci-cassert-geo` vers master à la fenêtre ; le commit de formatage pur (clang-format-18,
   `.clang-format-ignore` pour les annexes des docs) dans la fenêtre du ticket CI.
2. V2-1b : bloc neuf au point de reprise, plus de pages ombres pour les colonnes, balayages qui
   tiennent leur bloc, pages rendues à la mort du bloc ; mesure de l'amplification sur le banc des
   vingt (une grande table, une ligne modifiée), chiffres au commit, décision de Lucie.
3. V2-1c : la porte à deux niveaux, l'écriture hors du verrou public ;
   `ReadDuringACheckpointDoesNotWait` au vert.
4. Lecture en mémoire (wip `lecture-en-memoire`, 8c83302d4, poussé tel quel), E11 (I2).
