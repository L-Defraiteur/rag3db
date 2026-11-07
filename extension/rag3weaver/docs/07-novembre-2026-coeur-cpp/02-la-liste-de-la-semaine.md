# La liste de la semaine du cœur (à partir du 7 novembre 2026)

Lucie, relayée par c7 le 7 novembre : le moteur continue en parallèle du chantier des vues par
scope, et c'est la **famille correction d'abord** : pas une ligne de perf avant qu'elle soit à
zéro. Un ticket par jour, témoin rouge d'abord. Le banc des vingt clients et ses treize rouges
connus sont le tableau de bord : chaque correctif dit ce qu'il change à la liste. Hotfixs prend la
part CSR et index ((B) les demi-arêtes, la rétention des index, l'index plein texte hors du point
de reprise) ; ce qui se touche se répartit entre nous. Cette page tient la liste et son état, mise
à jour à chaque hash.

| ordre | ticket | état |
|---|---|---|
| 0 | 2026-11-07 un lecteur voit les lignes d'un COPY annulé (le bloc annulé) | corrigé `b7447937d`, sur master en `42b85dc62` |
| 0 | 2026-11-07 une validation et un lecteur HNSW se figent sur les verrous du CSR | corrigé `e526cc4b6`, sur master en `5f0b79466` |
| 2 | une instruction en échec condamne une lecture seule ; une interruption perdue (lot 2, avancé par c7) | corrigé, sur master en `17a094d12` |
| 0 | la projection sous READ ONLY, et DROP_PROJECTED_GRAPH qui honore skip_if_not_exists | sur master en `e86d2f688` |
| 0 | le retrait des générations à chaque point de reprise | sur master en `b3bc6e479` (moteur `7cd11e5fa`) |
| 0 | la lib unique de la bascule 43 (par l'arbre principal) | décision de c7 : bâtie depuis `17a094d12`, la projection et le retrait suivent en libs ordinaires |
| 1 | 2026-10-10 validation à moitié appliquée sur échec après l'ajout des lignes | prochain lot, avec le 2026-10-18 de l'annulation sous tampon plein |
| 2 | 2026-10-18 l'annulation d'un COPY sous un tampon plein peut encore allouer | à faire |
| 3 | 2026-10-18 les lignes d'un COPY forcé validé après un CREATE restent hors du graphe | à faire |
| 4 | 2026-10-04 durabilité sur faute d'entrée-sortie ou coupure (le témoin de la coupure d'abord, injection de faute) | à faire |
| 5 | 2026-10-04 reprise après arrêt exige takeover | à faire |
| 6 | 2026-10-10 échecs de chargement au rejeu que personne ne lit | à faire |
| 7 | V2-2c (format 44, page 19 §3) puis V2-3 | après la liste |

Partage avec hotfixs (7 novembre au soir, accord entre nous, validé par c7) : hotfixs prend les
décalages provisoires (le 2026-10-18 du COPY forcé hors du graphe compris), le COPY refusé faute de
tampon, les extensions à l'ouverture et au rejeu, les statistiques, le journal et HNSW hors verrous ;
le cœur garde la validation et l'annulation (10-10, 10-18 du tampon plein), l'instruction en échec
(fait), la durabilité, V2-2c et V2-3. Un seul des deux lots touche `undo_buffer` à la fois, et
chacun décrit son correctif à l'autre avant de l'écrire.
