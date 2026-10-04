# Revue des amonts : les correctifs de Ladybug et de Vela qui nous manquent

Session « banc » (`rag3db-76`), 4 octobre 2026, à la demande de l'orchestration.

**Méthode.**
- Les amonts ont été lus en lecture seule, depuis nos tags `ladybug-main-2026-08-31`
  (commits depuis octobre 2025) et `vela-master-2026-09-03` / `vela-checkpoint-2026-09-03`
  (depuis notre base commune `89f0263cc`).
- Quatre agents ont trié les commits ; chaque correctif présenté comme manquant a ensuite
  été reproduit chez nous, sur master, par une sonde, avant d'être rangé.
- Les témoins sont écrits d'après le symptôme : rien n'est repris du code ni des tests des
  amonts. On ne leur transmet rien.

**Où.**
- Témoins : `test/transaction/concurrence/upstream_fixes_test.cpp`, suite `UpstreamFixes`.
- Rangement : `known_red.txt`, sous la marche « correctifs de l'amont à reprendre ».
- Commit : `a10ee1c51`, poussé sur master `6fe2e4fc0`.

**Ordre de reprise proposé à la session cœur C++** : perte, puis plantage, puis blocage,
puis réponse fausse.

## 1. Confirmés chez nous : 21 témoins rouges, un seul écrivain, en service

| # | gravité | défaut, tel qu'on l'observe | amont | témoin |
|---|---|---|---|---|
| 1 | **perte** | Après la suppression de toutes les relations d'un bloc de 1 024 nœuds, le point de reprise efface aussi les relations de tous les autres blocs du même groupe de 131 072 nœuds, dans ce sens-là. Le sens inverse les garde, et la perte reste après réouverture. Cause : `csr_node_group.cpp:543-551` ne compte que les régions modifiées. | Ladybug `0be6fa597`, Vela `e5e700e73` (défaut Kuzu `eaf97b755`) | `RelationsOfAnUntouchedRegionSurviveACheckpoint` |
| 2 | plantage | Après `ALTER TABLE … DROP` d'une colonne qui n'est pas la dernière, le `CHECKPOINT` d'une table qui a des données sur disque tue le processus. Les données restent justes après réouverture, mais tout point de reprise suivant plante encore. Vrai pour les nœuds comme pour les relations. | Vela `e5e700e73` | `CheckpointAfterDroppingANodeColumn`, `…ARelationColumn` |
| 3 | plantage | Une relation créée puis supprimée depuis le dernier point de reprise : le point de reprise suivant passe, puis la lecture de la table tue le processus. | Ladybug `ba5f38815` | `ReadAfterCheckpointOfACreatedThenDeletedRelation` |
| 4 | plantage | `MERGE` avec deux `ON MATCH SET` sur la même relation, et la même clé deux fois dans le lot. | Ladybug `a0063158d` | `MergeWithTwoOnMatchSetsAndARepeatedKey` |
| 5 | plantage | Une branche de `UNION` qui projette deux fois la même propriété. | Ladybug `26be67f84` | `UnionArmProjectingAPropertyTwice` |
| 6 | plantage | Un chemin utilisé seulement dans le corps d'un lambda. | Ladybug `0c19d7816` | `PathUsedOnlyInsideALambda` |
| 7 | plantage | Une relation comparée à une variable de lambda (`r <> t`) ; `label()` sur un élément de `nodes(p)`. | Ladybug `0f3f03d63`, `57dc0b4ed` | `RelationComparedToALambdaVariable` |
| 8 | plantage | `WITH 1 AS gate WHERE $p … MATCH …`. | Ladybug `8281c845e` | `WithGateFilteredByAParameter` |
| 9 | blocage | Une colonne UUID qui contient l'UUID nul : « cannot negate INT128_MIN », aucun point de reprise ne passe plus, même après réouverture. | Ladybug `231a0b854` | `CheckpointOfAColumnHoldingTheNilUuid` |
| 10 | blocage | Un `CHECKPOINT` retient la validation d'un lecteur jusqu'à son délai, puis échoue. Un lecteur de 0,8 à 0,9 s seul dure 5,7 s pendant le point de reprise. | Vela `326d40dbd` | `CheckpointLetsAFinishingReaderGo` |
| 11 | faux | `RETURN DISTINCT … SKIP n` sans `ORDER BY` ni `LIMIT` rend 0 ligne ; `SKIP` sans `LIMIT` après une extension récursive rend trop peu de lignes. | Ladybug `079aa8458` | `SkipWithoutLimit` |
| 12 | faux | Un terme de `WHERE` (dans un `MATCH` ou un `OPTIONAL MATCH`) qui ne cite que des paramètres est jeté, même relié par `AND`. | Ladybug `8281c845e` | `PredicateOnParametersOnly` |
| 13 | faux | L'étiquette d'une variable liée plus tôt sans étiquette est ignorée. | Ladybug `413079079` | `ExplicitLabelOfAVariableBoundEarlier` |
| 14 | faux | Un nœud lié sans étiquette, répété seul dans un `OPTIONAL MATCH`, double les lignes. | Ladybug `e36c93e59` | `OptionalMatchRepeatingABoundNode` |
| 15 | faux | `MERGE` de la même clé deux fois dans un lot : la seconde ligne rend une valeur par défaut réévaluée (un UUID jamais stocké), ou l'identifiant d'un autre nœud. | Ladybug `c5a3c7385` | `MergeOfARepeatedKeyReturnsTheStoredNode` |
| 16 | faux | Dans une transaction, un balayage de plusieurs tables de relations relit sous R2 les relations locales de R1. | Ladybug `a8814a143` | `ScanOfSeveralRelationTablesInATransaction` |
| 17 | faux | `COPY` d'un champ CSV `""` donne NULL. | Ladybug `2fc419036` | `QuotedEmptyCsvFieldIsNotNull` |
| 18 | faux | `COPY` d'un CSV qui finit par un champ vide sans saut de ligne est refusé. | Ladybug `a8c619830` | `CsvEndingWithAnEmptyFieldAndNoNewline` |
| 19 | faux | Un accent grave doublé dans un nom (`` `a``b` ``) n'est pas réduit. | Ladybug `f03139f95` | `EscapedBacktickInAName` |
| 20 | faux (API) | `MaterializedQueryResult::toString()` consomme le curseur. | Ladybug `ef1e0e3cc` | `ResultToStringKeepsTheCursor` |

Les témoins 2 et 3 comptent chacun pour deux cas, d'où 21 rouges pour 20 lignes.

**Ce que cela veut dire pour rag3weaver** (vérifié par la session de l'arbre principal) :
- aucun site n'émet les formes fausses 11 à 14 ;
- rag3weaver supprime des relations et laisse passer des points de reprise, donc la perte
  (1) et le plantage (3) le concernent ;
- il n'y a aucun `ALTER … DROP` chez nous : le 2 est un défaut du moteur, sans urgence
  produit.

**La condition de la perte (1), pour une base existante.**
- Elle exige une table de nœuds de plus de 1 024 lignes, lignes supprimées comprises.
- Entre deux points de reprise, on supprime toutes les relations d'un bloc de 1 024 nœuds,
  dans un sens, sans rien écrire d'autre dans ce groupe de 131 072 nœuds.
- La signature à chercher : une dissymétrie entre le sens direct et le sens inverse.

## 2. Non reproduits avec les recettes déduites

| amont | défaut annoncé | ce qu'on a fait |
|---|---|---|
| Ladybug `254a7444d` | chaînes de relations réécrites aux mauvaises lignes, au point de reprise d'une seule région | Recette déduite (3 000 relations, une supprimée, point de reprise) : juste. Gardée comme garde-fou vert (`RelationStringsKeepTheirRowsAcrossARegionCheckpoint`). Le défaut peut exiger un dictionnaire trié ou une autre forme. |
| Ladybug `a10cecbc7` | lecture filtrée d'une colonne de chaînes sur plusieurs segments | 20 000 chaînes de 300 caractères, une ligne sur 97 supprimée : juste, avant et après point de reprise. |

## 3. Non atteignables au banc sans crochet dans `src/`

Il faut une faute d'E/S, une coupure de courant, ou une mort placée au bon instant.

| amont | défaut | où chez nous |
|---|---|---|
| Vela `a96f7a7d9`, Ladybug `b824045f0` | La validation est publiée en mémoire avant l'écriture du journal. Après un échec d'écriture (disque plein), les validations suivantes s'ajoutent derrière un enregistrement déchiré. | `transaction.cpp:63-71` |
| Vela `645d68973` | Le répertoire n'est jamais synchronisé après la création ou la suppression du journal et du fichier fantôme. | `wal.cpp`, `shadow_file.cpp` |
| Vela `e5e700e73`, Ladybug `e2ada3f90` | Le rejeu des pages fantômes n'est pas synchronisé avant la suppression du journal. | `shadow_file.cpp:134-141` |
| idem | La reprise supprime le fichier fantôme avant le journal : une mort entre les deux rend la base impossible à ouvrir. | `wal_replayer.cpp:752-755` |
| Vela `dfee83bf3`, Ladybug `9ca5e8913` | Les pages ajoutées par un point de reprise raté ou par un arrêt brutal ne sont jamais récupérées : le fichier grossit. | `checkpointer.cpp` |
| Ladybug `9ca5e8913` | Le compteur de pages libres croît sur une plage libérée deux fois. Aucune double libération trouvée chez nous. | `free_space_manager.cpp:34-39` |
| Ladybug `22ee54a71` | Un point de reprise échoué après un `DROP` laisse les colonnes compactées. | `node_table.cpp:808-815` |
| Ladybug `7e4248202`, `eff87c1e1`, `d6adbd2b7`, Vela `e5e700e73` | Le `spinLock` d'une page boucle sur une valeur périmée ; course entre évinceurs ; file d'éviction ; page verrouillée après une erreur d'E/S. Courses rares ou bases en mémoire. | `page_state.h`, `buffer_manager.cpp` |
| Ladybug `1a233d280` | Une frontière d'algorithme de graphe est dimensionnée hors instantané ; une validation concurrente déborde le tampon. | `gds_frontier.cpp:90-93` |
| Ladybug `bb52f183c` | `madvise` sur des pages de 16 ou 64 Ko efface des trames voisines (aarch64 seulement). | `vm_region.cpp:69` |

## 4. Autres manques, moins graves ou par l'API (sans témoin)

- **Ladybug `ec0e37d27`** : détruire une `Connection` pendant qu'un autre fil y exécute une
  requête (mauvais usage de l'API).
- **Ladybug `d20e53790` et `31795d69f`** : `StorageDriver::getNumRels` lève une exception et
  laisse une transaction pendante.
- **Ladybug `d42320ebd`** : un très grand motif `MATCH` dépasse les plafonds du planificateur
  et rend un message obscur.
- **Ladybug `c20f0dd66`** : environ 1 Mo de pages d'en-tête gaspillées par table de nœuds.
- **Ladybug `4ff9dbdf5`** : les vidages mémoire englobent la réservation virtuelle du pool.
- **Ladybug `a921a4c47`** : les statistiques de valeurs distinctes s'effondrent avec des
  insertions ligne à ligne (qualité des plans).
- **Ladybug `87b6829cc`** : `CALL create_vector_index` refusé dans une requête (choix
  discutable).

## 5. Déjà chez nous, ou à ne pas porter

- **Déjà chez nous** : Ladybug `47942787d`, `a8ad3d7cf`, `0ba05cf10`, `e98835d3f`,
  `c915e8c55` (autrement), `646474804` (autrement) ; Vela `2efa20b67` (journalisation),
  `b73e093c0` (barrière de redémarrage), `d298e9ce1`.
- **Probablement chez nous** : Ladybug `029d7aef2`, le `SET` après `DROP_VECTOR_INDEX`, couvert
  par `f5acca417`. Un témoin vert serait à ajouter pour le confirmer.
- **À ne pas porter** :
  - Ladybug `756d4c1cb`, volet rollback, et `5086a20eb`. Rendre réutilisables avant le point
    de reprise des pages encore référencées par le dernier point de reprise est un risque de
    corruption, et c'est sans doute ce qui crée chez eux la double libération de `9ca5e8913`.
  - Ladybug `e46d6542e` : un simple raccourci.
- **Inapplicables** : la suite de la recherche par clé ligne par ligne de Ladybug (`06a899c64`,
  `e72eb059f`, `ac83fda3d`, `d395e5e41`), et les fonctionnalités propres aux amonts (cache
  de plans, ART, icebug-disk, parquet, arrow, attach, partitions, graphes ANY, point de
  reprise non bloquant et écritures concurrentes de Vela).
