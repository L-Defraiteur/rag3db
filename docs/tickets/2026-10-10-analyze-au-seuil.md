# Pas d'analyze automatique : les distincts restent gonflés jusqu'à un CALL analyze

- **État** : ouvert — à ouvrir quand un mauvais plan sera observé (orchestration, 10 octobre
  2026). Écarté le même jour : un analyze automatique après chaque COPY (la sonde ci-dessous ;
  il rebalaierait toute la table à chaque point de reprise de la première indexation)
- **Gravité** : estimation fausse (seuls le planificateur et `STATS_INFO` lisent ces comptes)
- **Atteignable en service** : oui, après des `DELETE`, des `SET` ou des lignes écartées par
  `IGNORE_ERRORS`
- **Touche rag3weaver** : au plus des plans moins bons (il n'appelle pas `analyze`)
- **Ouvert le** : 10 octobre 2026, seconde session cœur C++
- **Pour** : cœur C++
- **Famille** : moteur

## Ce que c'est

Depuis `CALL analyze('Table')` (`dff1adddc`), les comptes de valeurs distinctes ne reculent que
par un appel explicite. La cardinalité, elle, est recalée sur les lignes vivantes à chaque point
de reprise et à l'ouverture. Rien ne déclenche `analyze` de soi-même.

## Mesuré (10 octobre 2026, luciepc, sonde jetable)

`STATS_INFO` contre la vérité (`count(*)`, `count(DISTINCT …)`) :

| Après | Cardinalité | Distincts estimés / vrais |
|---|---|---|
| un COPY de 5 000 sur table vide | 5 000 (exacte) | 5 909 / 5 000 |
| un second COPY de 5 000 | 10 000 (exacte) | 12 233 / 10 000 |
| DELETE de 2 000, COPY de 3 000, CHECKPOINT | 11 000 (exacte) | 16 130 / 11 000 |

Après un COPY simple, l'écart est l'erreur propre de l'HyperLogLog à 64 registres (≈ 13 %
d'erreur type) : un `analyze` n'y change rien. Ce qui gonfle les distincts, ce sont les écritures
qui ne les reculent pas (suppressions, `SET`, refus d'`IGNORE_ERRORS`).

Le coût d'un `analyze` (page `extension/rag3weaver/docs/10-octobre-2026-coeur-cpp-tickets/03-le-recalcul-des-statistiques.md`, §5 ter) :
4 ms pour 100 000 lignes clé + chaîne, 24 ms avec un `FLOAT[768]` (95 ms à froid), plus 1 à 8 ms
au point de reprise qui suit.

## Ce que fait PostgreSQL

L'autovacuum lance `ANALYZE` sur une table quand les lignes insérées, modifiées ou supprimées
depuis le dernier dépassent `autovacuum_analyze_threshold + autovacuum_analyze_scale_factor ×
nombre de lignes`, par défaut 50 + 10 %
([runtime-config-autovacuum](https://www.postgresql.org/docs/current/runtime-config-autovacuum.html),
[§24.1.3](https://www.postgresql.org/docs/current/routine-vacuuming.html)). Neo4j rééchantillonne
un index quand 5 % de sa taille a changé (`db.index_sampling.update_percentage`).

## Pour le fermer — la forme retenue le jour où on l'ouvre

Un drapeau, ou un compte, par table des lignes supprimées, modifiées ou écartées depuis le dernier `analyze`, et
un `analyze` au point de reprise quand il dépasse le seuil de PostgreSQL. Ce compte se pose dans
`NodeTable::delete_` et `update`, et à l'annulation des lignes écartées par un COPY. Témoin
rouge : un `DELETE`, un COPY, un `CHECKPOINT`, puis les distincts dans la marge. À décider sur un
mauvais plan observé, avec la mesure du balayage sur la taille réelle des tables du produit (une
table d'un million de lignes à vecteurs coûterait de l'ordre de 240 ms par balayage, extrapolé,
non mesuré).

Voir aussi `2026-10-10-hyperloglog-a-64-registres.md` (la précision des distincts).

## Le lot du 8 novembre 2026 (`g5-statistiques`, session hotfixs)

- **Le compte.** `NodeTable` compte les lignes validées supprimées, modifiées (un `SET` par ligne et
  par colonne) ou écartées par `IGNORE_ERRORS` depuis le dernier analyze. Elles passent toutes par
  `NodeTable::delete_` ou `update`, et seule la part persistante compte.
- **Le seuil.** Au-delà de 50 + 10 % des lignes de la table, son point de reprise rebâtit ses
  statistiques sur les lignes vivantes, avant de les écrire. Le balayage,
  `NodeTable::rebuildStatsFromLiveRows`, est partagé avec `CALL analyze` et tourne sous
  `DUMMY_CHECKPOINT_TRANSACTION`, puisqu'aucun écrivain ne tourne.
- **Les témoins** (`test/transaction/table_analyze_test.cpp`) :
  - `TheCheckpointAnalyzesATablePastTheThreshold` : rouge avant, avec 1 931 noms estimés pour 1 000 ;
  - `UnderTheThresholdTheCheckpointDoesNotAnalyze` : 10 lignes supprimées sur 2 000, et les
    distincts ne bougent pas.
- **La mesure** (8 novembre 2026, 8 h 58 à 9 h 08, luciepc, `poste mesure`, fenêtre de c7). Six
  copies reflink de code-lourd (1,2 Go), prises quand l'hôte l'avait fermée, une par prise. Sur
  chacune, un `DETACH DELETE` validé de 11 % de sa plus grosse table à index HNSW, `Scope_Chunk`
  (14 314 lignes sur 130 128, vecteurs FLOAT[768]), puis un `CHECKPOINT` chronométré :

  | lib | md5 de `librag3db.so` | CHECKPOINT (trois prises) | distincts |
  |---|---|---|---|
  | sans le lot (build dadbf0041) | `51bddf21913344cd9a2ba69579bc261e` | 838, 841, 849 ms | inchangés |
  | avec le lot (build 84651120e) | `aa9efd3b08c4b25cde3c96376c4eeb0d` | 1 234, 1 229, 1 233 ms | rebâtis |

  L'analyze coûte ≈ 390 ms une fois, au point de reprise qui passe le seuil, pour 116 000 lignes
  vivantes à vecteurs. Les deux libs sont bâties par la cible `rag3db_shared` ; une première copie
  prise par la cible `rag3db` (la statique) donnait deux fois la même `.so`, démasquée par le md5
  avant tout chiffre.

## Limite connue

Le compte vit en mémoire : une réouverture le remet à zéro. Une table qui a beaucoup changé puis
a été rouverte n'est rebâtie qu'après de nouveaux changements au-delà du seuil, ou par un
`CALL analyze`. La forme d'après : écrire le compte au point de reprise, avec les statistiques de
la table. Cela change leur format sur disque, donc à la bascule 44, avec les 1 024 registres de
l'HyperLogLog (`2026-10-10-hyperloglog-a-64-registres.md`).
