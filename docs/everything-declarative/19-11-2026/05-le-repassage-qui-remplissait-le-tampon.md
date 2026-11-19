# Le repassage qui remplissait le tampon : la cause, le correctif, la mesure

Session everything declarative, 19 novembre 2026, 19 h 15. C'est le défaut qui a fait annuler la bascule des bases de code
ce soir. Ticket de G : `docs/optimiseur/tickets/2026-11-19-le-repassage-d-une-base-migree-remplit-le-tampon.md`
(branche `ticket-repassage-tampon-plein`).

## Le rouge, reproduit

- **Base** : un reflink de `mesures/code-rag3db-migree-soir-20261119` à luciepc, la vraie base du dépôt passée par 1b
  et les générations, jamais repassée. La copie de G n'a pas été ouverte.
- **Conditions** : la sonde `sonde_parite_search_code::repasse_une_base_migree_sous_le_tampon_du_produit`, en
  release, lib 6e2476d41, tampon par la règle du produit (8 Gio).
- **Rouge** : en 20 s, au premier paquet, « recopier les arêtes DEFINED_IN : … Buffer manager exception: … The buffer
  pool is full ». La mémoire du processus est montée à 11,7 Gio.

| s | rss du processus |
|---|---|
| 0 | 1,2 Gio |
| 5 | 2,3 Gio |
| 10 | 3,6 Gio |
| 15 | 5,4 Gio |
| fin (échec) | 11,7 Gio |

## La cause : le plan d'une seule instruction, pas la taille du lot

**Le premier passage de l'écrivain** écrit déjà par paquets de fichiers (`file_batch_size`) et valide **une
transaction par paquet**. Chaque paquet ne recopie que ses propres paires (ligne remplacée, ligne neuve), par lots de
2 000. Ce n'est donc ni une transaction qui porterait tout, ni un nombre de paires qui grandirait avec la base.

**La recopie d'aujourd'hui** :

```
UNWIND $paires AS p
MATCH (x:Scope)-[r:DEFINED_IN]->(a:File {_uuid: p.a}), (n:File {_uuid: p.n})
WHERE <visible(x)> AND <hors du lot> AND NOT EXISTS { MATCH (x)-[:DEFINED_IN]->(n) }
CREATE (x)-[:DEFINED_IN]->(n) RETURN count(*)
```

Le plan montre deux `CROSS_PRODUCT` après l'`UNWIND`. `(n:File {_uuid: p.n})`, posé dans le même `MATCH` que le
motif, n'est pas lu par sa clé : il est croisé avec un balayage de `File`. Le `NOT EXISTS` se calcule par un
`HASH_JOIN` sur un `SCAN_REL_TABLE` de toutes les arêtes DEFINED_IN, agrégées. La mémoire suit donc la base.

## Le correctif

**Chaque bout par sa clé, dans un `MATCH` à lui**. C'est le rendu rag3db de `Write::CopyEdges`, dans
`src/dialect.rs` :

```
UNWIND $paires AS p
MATCH (a:File {_uuid: p.a})
MATCH (n:File {_uuid: p.n})
MATCH (x:Scope)-[r:DEFINED_IN]->(a)
WHERE … NOT EXISTS { MATCH (x)-[:DEFINED_IN]->(n) }
CREATE (x)-[:DEFINED_IN]->(n) RETURN count(*)
```

**Mesuré sur le même lot de 2 000 paires réelles**, chaque fichier vers le suivant, dans une transaction annulée
(sonde `mesure_les_formes_de_la_recopie`) :

| forme | rendu | durée | mémoire en plus |
|---|---|---|---|
| d'aujourd'hui | refusée, tampon plein | 3,8 s | 8 Gio (pic 9,96 Gio) |
| corrigée | 6 861 arêtes recopiées | 631 ms | 16 Mio |

## Le vert : le repassage entier de la vraie base

Sur un reflink neuf de la même copie, sous la même règle (8 Gio), avec le correctif (`a9f43d20ff`) :
**« indexation terminée » en 1 049 s**. Le pic du processus est de 6,94 Gio, tampon compris.

| s | rss du processus |
|---|---|
| 0 | 2,0 Gio |
| 60 | 6,7 Gio |
| 120 | 5,9 Gio |
| 180 | 3,9 Gio |
| 300 | 1,0 Gio |
| 420 | 5,8 Gio |
| 540 | 5,1 Gio |
| 660 | 1,4 Gio |
| 780 | 3,8 Gio |
| 900 | 3,0 Gio |
| 1 020 | 4,1 Gio |
| fin | 3,7 Gio, rendus à 1,9 Gio par `malloc_trim` |

La mémoire monte et redescend avec les paquets ; elle ne s'accumule pas.

**La parité** : les dix témoins `code_sync::ir_parity_tests` restent verts avec le nouveau rendu. La recopie laisse le
même état qu'avant, arêtes et propriétés comprises.

## Ce qui reste, et pour qui

- **La répétition complète**, chez G : 1b, les générations, la réouverture, **le repassage**, puis le critère, sur un
  reflink neuf de la copie. La bascule se reprogramme à son vert.
- **Le trou de la répétition** (ticket de G) : une répétition de bascule joue désormais le repassage jusqu'au bout.
  C'est une règle de G, pas du code.
- **La même forme ailleurs** : les autres recopies d'arêtes (CONSUMES, PARENT_OF…) passent par le même rendu, et sont
  corrigées du même coup. Le rendu postgres du lot B (`ed-postgres-generations`, en file) est une jointure sur
  `jsonb_to_recordset`, sans motif croisé ; il se mesurera de même sur une base postgres de taille réelle.
