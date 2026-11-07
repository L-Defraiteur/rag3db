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
| 0 | la projection sous READ ONLY (projection-lecture-seule-3) | en cours : rejeu de la suite transaction, push |
| 0 | le retrait des générations (v22-retrait-2) | à rebaser sur le correctif et rejouer |
| 0 | la lib unique de la bascule 43 (par l'arbre principal) | attend le retrait |
| 1 | 2026-10-10 validation à moitié appliquée sur échec après l'ajout des lignes | à faire |
| 2 | 2026-10-18 l'annulation d'un COPY sous un tampon plein peut encore allouer | à faire |
| 3 | 2026-10-18 les lignes d'un COPY forcé validé après un CREATE restent hors du graphe | à faire |
| 4 | 2026-10-04 durabilité sur faute d'entrée-sortie ou coupure (le témoin de la coupure d'abord, injection de faute) | à faire |
| 5 | 2026-10-04 reprise après arrêt exige takeover | à faire |
| 6 | 2026-10-10 échecs de chargement au rejeu que personne ne lit | à faire |
| 7 | V2-2c (format 44, page 19 §3) puis V2-3 | après la liste |
