# Supprimer une ligne qui porte un vecteur indexé (HNSW) coûte plusieurs secondes

- **État** : ouvert. Le moteur n'est pas corrigé ; rag3weaver le contourne (le vecteur est
  remis à NULL avant la suppression, ci-dessous)
- **Gravité** : lenteur sévère (la réindexation d'un fichier par la montre tenait le catalogue
  plusieurs secondes ; retirer un fichier, des minutes)
- **Atteignable en service** : oui, à toute réindexation ou tout retrait de fichier d'un index
  à vecteurs, avant le contournement
- **Touche rag3weaver** : oui (`record_nodes.rs`, `dialect.rs` : le contournement)
- **Ouvert le** : 12 octobre 2026, session everything declarative (la mesure par phase des
  « 6 s » de Lucie)
- **Pour** : hotfixs (l'extension vector)

## Ce que c'est

Sur une table de morceaux dont la colonne `FLOAT[n]` porte un index HNSW,
`UNWIND $uuids AS uuid MATCH (c:Scope_Chunk {_parent_uuid: uuid}) DETACH DELETE c` coûte
environ **2,5 s par ligne supprimée**, et ce coût croît d'un essai à l'autre. La même
instruction, sur les mêmes lignes dont le vecteur vient d'être remis à NULL, prend quelques
millisecondes.

## Mesuré (12 octobre 2026, poste principal, lib `build/lecteurs-csv` de 2 h 34)

Le test `measure_one_file_reindexing` (`backend.rs`, `#[ignore]`) indexe `src/` (180 fichiers,
15 631 morceaux, embarqueur `hashembedder`), réindexe cinq fois un fichier dont un scope change,
puis retire un fichier de 32 scopes (45 morceaux). Une sonde jetable a chronométré chaque
instruction de `DeleteRecordNode` :

| instruction | avant | vecteur remis à NULL d'abord |
|---|---|---|
| lire les morceaux d'un scope (par `_parent_uuid` ou par l'arête) | 2 à 8 ms | — |
| remettre le vecteur à NULL (1 morceau / 45 morceaux) | — | 7 ms / 251 ms |
| `DETACH DELETE` des morceaux d'un scope (1 morceau) | 2 095 à 2 937 ms | 4 à 6 ms |
| `DETACH DELETE` des morceaux de 32 scopes (45 morceaux) | 112 351 ms | 8 ms |
| relire puis supprimer le parent, désindexer le plein texte | < 10 ms | < 10 ms |
| réindexer un fichier, de bout en bout | 2,5 à 3,9 s | 0,73 à 1,03 s |
| retirer un fichier de 32 scopes | 96 à 114 s | 0,3 s |

Ce n'est donc ni le balayage par `_parent_uuid` (sans index) ni la suppression du parent :
c'est l'effacement, dans l'index HNSW, d'une ligne qui y figure encore.

Autre chiffre de la même passe, à part : l'index initial a passé 309 s sur 329 à écrire
15 631 vecteurs (`write_ms` du nœud `embed`), soit environ 20 ms par vecteur.

## Le contournement (rag3weaver)

`SchemaDialect::vector_columns_to_clear` décrit la table (`CALL table_info`). Pour chaque
tableau de taille fixe, `clear_vector_by_parent` remet la colonne à NULL sur les morceaux des
parents cités, juste avant le `DETACH DELETE`. C'est fait dans `DeleteRecordNode` (retrait d'un
scope ou d'un fichier) et dans `RechunkDeleteNode` (redécoupage). PostgreSQL ne le fait pas.

Le `SET` à NULL par lot est l'opération que `record_nodes::write_vectors` évite pour une ligne
qui garde sa place (l'index en perd des lignes). Ici, les lignes partent juste après :
ce qu'il ferait perdre ne concerne qu'elles. `e2e_invariant_des_vecteurs` doit rester vert.

## Témoins au banc (12 octobre 2026, session du banc)

`test/transaction/concurrence/vector_index_cost_test.cpp`, FLOAT[768], cosine, paramètres par
défaut, vecteurs groupés autour de 64 centres. Moteur de master acdb91764 ; chaque rouge ci-dessous
a été joué, pas déduit.

- `VectorIndexCost.ProductShapeDeleteCost` : la forme du produit au plus près (index créé sur la
  table vide, COPY de 15 000 morceaux sans vecteur avec une relation sortante chacun, SET depuis
  NULL en lots de 64, aucun point de reprise, `DETACH DELETE` filtré sur `_parent_uuid`). **Rouge**
  sur `delete-close-to-set-null-then-delete` (le DELETE direct au plus cinq fois SET NULL puis
  DELETE, plus 2 ms) : médianes 219 contre 5,7 ms, puis 282 contre 6,8 ms, soit un facteur 38 à
  41 (300 chez le proto). `delete-cost-does-not-grow` reste vert (la montée de 2,1 à 2,9 s n'est
  pas reproduite). Après un `CHECKPOINT` (24,9 s), le même DELETE prend 36 à 48 ms.
- `Builds/VectorIndexDeleteCost.DoesNotDependOnIndexSize` : lots de 10 lignes en une requête, temps
  médian par ligne de 1 500 à 15 000 lignes, la grande au plus deux fois la petite plus 2 ms.

  | l'index est fait | sans point de reprise | après un point de reprise |
  |---|---|---|
  | par CREATE sur la table pleine | 35 → 32 ms, vert | 35 → 33 ms, vert |
  | par SET depuis NULL en lots de 64 | 127 → 895, puis 39 → 236, 52 → 361 ms : **rouge** | 143 → 116, 39 → 35, 50 → 50 ms, vert |
  | le même, une relation entrante par ligne | 95 → 644 ms : **rouge** | 68 → 77 ms, vert |

  Le coût dépend de la taille de l'index tant que ses arêtes n'ont pas été checkpointées. Après le
  point de reprise, il reste un écart : un index rempli par SET coûte 50 à 140 ms par ligne contre
  33 pour un index fait par CREATE. Et un lot de 10 coûte jusqu'à 900 ms par ligne quand un
  morceau seul en coûte 220 (forme du produit) : `finalizeDelete` paierait plus que linéairement
  en voisins à réparer.

Les mesures (`MeasureDelete`, `MeasureInsert`, `MeasureInsertWithCheckpoints`, sous
`CONCURRENCE_MEASURE=1` et `poste mesure`) :
- une suppression sous index fait par CREATE coûte 20 à 50 ms par ligne, sans dépendre de la
  taille ni du point de reprise ; 0,03 à 1 ms sans index ; 0,7 ms par la voie SET NULL puis DELETE ;
- un SET depuis NULL sous index coûte 1 à 4 ms par vecteur (0,25 sans index) ; 1, 8 et 32 fils
  donnent les mêmes chiffres ; le coût monte d'un lot de 500 à l'autre, et un point de reprise
  entre chaque lot ne change pas la courbe ;
- `CREATE_VECTOR_INDEX` sur 15 000 vecteurs : 9 à 12 s, soit 0,6 à 0,8 ms par vecteur.

Profil par échantillonnage de piles (eu-stack toutes les 0,2 s, fils actifs seulement ; outil dans
les notes du banc, `profil.sh`) :
- la suppression sans point de reprise : `finalizeDelete` 98 %, dont `keepNodeReachable` 88 %
  (une recherche par voisin survivant) ; la lecture des voisins passe par
  `CSRNodeGroup::scanCommittedInMemRandom` 75 % puis `ChunkedNodeGroup::lookup` 73 %, chaque ligne
  payant un verrou partagé (34 % de temps propre) et `VersionInfo::isSelected` (21 %) ;
  `findMatchingRow` 7 %, `UpdateInfo::lookup` 12 %, `updateCSRIndex` et son tri environ 2 % ;
- l'insertion : `insertToLayer` 90 %, `scanCommittedInMemRandom` 66 %, et `shrinkForNode` puis
  `detachDelete` puis `findMatchingRow` 29 % ;
- le point de reprise après environ 15 000 SET : `ColumnChunkData::resize` 80 % de temps propre,
  sous `UpdateInfo::iterateVectorInfo`, `applyCommittedUpdates` et `ListChunkData::write` (un
  agrandissement par mise à jour appliquée, à confirmer dans le code).

Les trois rouges sont en known_red et dans long.txt (215 à 390 s chacun).

## Ce qu'il faut pour le fermer

Un `DETACH DELETE` d'une ligne indexée en quelques millisecondes sur un index de cette taille,
mesuré par le même test. Alors le contournement s'enlève : les deux méthodes du dialecte et
`vider_les_vecteurs_avant_suppression`.
