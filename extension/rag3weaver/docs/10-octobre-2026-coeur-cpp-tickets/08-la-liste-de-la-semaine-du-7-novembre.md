# La liste de la seconde session cœur C++ (hotfixs), semaine du 7 novembre 2026

Ordre donné par l'orchestration le 7 novembre, d'après Lucie : le moteur continue en parallèle
du chantier des vues par scope, la correction d'abord. Un ticket par jour. Pour chacun : un
témoin rouge d'abord, puis la cause, puis un correctif générique, poussé sous VERT.

Partage avec le cœur : il prend les transactions, la validation, l'annulation, la durabilité et
la reprise. Ce qui se touche (`transaction_manager`) se dit avant d'y écrire. Le banc est le
tableau de bord.

## Avant la liste : (B) et ce qui la précède

| Étape | État |
|---|---|
| (B) témoin des demi-arêtes LOCKS/QUERIES (`csr-listes-temoin`) | vert sur quatre moteurs (2a29c10c6, 94e21b066, 58b46a9bf, master) dans ses deux premiers cas ; cas des arrêts brutaux et des drains par COPY annulés en cours à luciepc |
| Piste du bloc entièrement annulé (lot `bloc-annule` du cœur, 71dc33ec6) | le témoin des COPY annulés doit rougir sur master et verdir sur ce lot, ou il manque encore un ingrédient |
| Relevé des 18 tables de la base de code de Lucie | sur l'instantané `avant-43` que G prendra à l'arrêt de la bascule 43 ; point zéro le 25 octobre : 0 écart |
| Mesures calmes de (A) | à rebâtir depuis les patchs de `~/.cache/rag3db-tickets-notes` (les arbres de mesure ont été effacés par le banc le 7 au matin) |

## La liste

| # | Ticket | État |
|---|---|---|
| 1 | [Index plein texte en fichiers hors du point de reprise](../../../../docs/tickets/2026-10-10-index-plein-texte-en-fichiers-hors-du-point-de-reprise.md) | à faire |
| 2 | [La rétention des index ne couvre que l'index vectoriel](../../../../docs/tickets/2026-10-18-la-retention-des-index-ne-couvre-que-l-index-vectoriel.md) | à faire |
| 3 | [Le rtree reçoit des décalages provisoires](../../../../docs/tickets/2026-10-05-rtree-recoit-des-decalages-provisoires.md) | à faire |
| 4 | [COPY de relations vers des nœuds locaux sans remappage](../../../../docs/tickets/2026-10-05-copy-de-relations-vers-des-noeuds-locaux-sans-remappage.md) | à faire |
| 5 | [COPY garde un doublon selon les fils](../../../../docs/tickets/2026-10-10-copy-garde-un-doublon-selon-les-fils.md) | à faire |
| 6 | [Deux nœuds liés par leur clé dans un même MATCH se croisent avant la jointure](../../../../docs/tickets/2026-10-12-deux-noeuds-lies-par-leur-cle-dans-un-meme-match-se-croisent-avant-la-jointure.md) | à faire |
| 7 | Le CALL de réparation d'une demi-arête, et un DELETE qui l'efface dans le sens où elle est ou dit qu'il ne la trouve pas (forme produit de `tools/repair_half_edge`, branche `reparer-demi-arete`) | à faire, après (B) |

Cette page se tient à jour à chaque ticket ouvert, rouge, corrigé ou poussé, avec les hashs.
