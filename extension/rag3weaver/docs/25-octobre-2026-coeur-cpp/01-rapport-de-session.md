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
| 25 oct. 17 h 27 | vector : le degré converti explicitement (MSVC C2398, AppleClang) | 04063ee06 | Windows et macOS rebâtissent l'extension vecteur ; base de la lib de la bascule 41/42 |
| 25 oct. 18 h 31 | V2-1b pièce 1 : le segment neuf au point de reprise | 6b63e4922 | un segment changé est rebâti sur des pages neuves, l'ancien rend ses pages à sa mort ; plus de réécriture en place ni d'ombre pour les colonnes ; format inchangé ; témoin `checkpoint_new_segment_test.cpp` (4 cas) ; mesure : une ligne modifiée = 344 Kio des deux côtés, 1 % = 2,97 Mio contre 3,53, 10 % = 2,97 contre 5,79 (page 18 §6) |
| 25 oct. 19 h 50 | V2-1c : un point de reprise ne fige plus les lecteurs nouveaux | 66994dd43 | porte à deux niveaux (`checkpointWriting`, écriture hors du verrou public), bloc neuf bâti hors du verrou de la collection et échangé sous lui, pages ombres recopiées sous le verrou de page ; emporte la pièce 2 de V2-1b ; témoin `checkpoint_under_readers_test.cpp` (3 cas) ; `ReadDuringACheckpointDoesNotWait` vert (2,5 ms pendant 1 057 ms), sorti de known_red ; ASAN sans rapport ; format inchangé (page 18 §7) |
| 25 oct. 21 h 10 | V2-2a+2b : un lecteur ouvert avant le point de reprise n'est plus attendu | f8c0c8df0 | il relit sa génération des blocs (nœuds : la liste ; CSR : persistant, blocs, copie de l'index), gardée jusqu'au point de reprise suivant ; T et F du gestionnaire au Checkpointer ; la fonte à F essayée d'abord perdait ]F, T] à la réouverture (page 19 §3) ; témoin `checkpoint_open_reader_test.cpp` ; deux témoins qui tenaient un lecteur réécrits avec un écrivain et doublés ; format inchangé |

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
- Un témoin « pages libres » vise le seul cas où master restait en place (segment 16 bits, valeur
  dans l'intervalle) : hors intervalle du bitpacking ou segment qui remplit ses pages pile,
  l'ancien chemin réécrivait déjà hors place.
- Le produit d'une passe se joue avec HEAD au dernier commit **moteur** du lot : la lib publiée
  porte cet identifiant-là (common/build_id), pas celui d'un commit de tests ou de docs au-dessus.
- Le shell `rag3db` ne se pilote pas par un tube (il attend la réponse de ESC[6n) : toujours
  `< fichier` ; `poste` est `~/.cache/rag3weaver-build/poste`.
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
2. Le retrait des générations dans toutes les tables (`v22-retrait`, rebasé sur master), petit
   lot après rejeu réduit, avec la forme du banc pour son témoin ; relire le rebase de hotfixs sur
   `transaction_manager`.
3. V2-2c (journal scellé à T et journal neuf, format 44 ; la porte ouverte aux écrivains
   nouveaux ; la garde de groupe) ; V2-3 ; l'écriture à la demande (page 17 §3.1).
4. Lecture en mémoire (wip `lecture-en-memoire`, 8c83302d4, poussé tel quel), E11 (I2).
