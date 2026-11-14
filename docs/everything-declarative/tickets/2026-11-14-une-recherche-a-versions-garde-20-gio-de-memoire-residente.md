# Une recherche sur une base à versions garde 20 Gio de mémoire résidente

- **État** : ouvert (14 novembre 2026).
- **Famille** : produit
- **Gravité** : mémoire. Avec le masque en prédicat (vue-publiee, 959d73513), un processus qui cherche sur une base à versions de la taille du dépôt rag3db voit son RSS monter d'environ 300 Mio par recherche sans filtre, jusqu'à 20 Gio sur 50 appels, puis rester vers 20 Gio en filtré. La base d'avant les versions plafonne à 8,6 Gio
- **Atteignable en service** : oui, dès que 1b fait monter une vraie base aux versions. Le court-circuit d'arbre principal retire le cas sans filtre ; le cas filtré reste
- **Touche rag3weaver** : à départager (la jointure version → occurrence, rejouée par le lexical et par le dense à chaque recherche) ; ou le réglage de l'hôte (les arènes de glibc)
- **Pour** : everything declarative
- **Signalé par** : everything declarative, 14 novembre 2026, luciepc, `poste mesure`, lib 373b5477b, `tests/sonde_recherche_sans_filtre.rs` (journaux `ed-generations-2/s-p-v.log` et `s-p-r.log`) ; la charge du poste était haute (un contrôle de G hors du verrou), les durées sont pessimistes, le RSS non

## Ce que c'est

10 requêtes × 5 tours par forme, embarqueur connecté, dans un même processus :

| | R, base d'avant | base à versions, prédicat |
|---|---|---|
| à l'ouverture | 2,3 Gio | 2,2 Gio |
| après le sans-filtre | 7,3 Gio | 20,6 Gio (+301 Mio par appel, des sauts de 2 à 3 Gio et des replis) |
| après le chemin | 8,6 Gio | 19,9 Gio (plat) |
| après le langage | 8,6 Gio | 20,7 Gio (plat) |

## La première chose à faire

Rejouer la même série sous `MALLOC_ARENA_MAX=2`. Si le RSS tombe, c'est le réglage de l'hôte (G),
et le ticket se ferme sur ce réglage, mesuré. Sinon, il y a une rétention à chercher dans le chemin de
la recherche, étage par étage, comme pour la projection.

## Le réglage tranche (14 novembre 2026, 21 h 35 à 21 h 39)

La même série, l'état A sans filtre, sous `MALLOC_ARENA_MAX=2` (vue-publiee 2bdb8a6e4, lib 373b5477b) :
- RSS : 2,2 Gio à l'ouverture, 4,7 après le premier appel, 9,2 au onzième, puis **un palier jusqu'au
  cinquantième** (9,4 à 9,8 Gio). Sans le réglage : +301 Mio par appel, jusqu'à 20,6 Gio ;
- la durée ne bouge pas : 2 515 ms en médiane, contre 2 832.

La pente venait donc des **arènes de glibc**, une par fil, la mémoire rendue restant dans l'arène du
fil précédent (le cœur). **La fermeture** : `MALLOC_ARENA_MAX=2` dans l'unité de l'hôte, posé par G et
mesuré sur l'hôte réel. Ce ticket reste ouvert jusqu'à cette pose.

L'écart qui reste (9,8 Gio contre 7,3 à 8,6 en R, mesuré sans le réglage) est l'ordre de grandeur
des jointures du prédicat. Il ne se mesurera honnêtement qu'avec R sous le même réglage.

## Le réglage écarté sur l'hôte réel (G, 14 novembre 2026, 22 h 10)

L'hôte de master (10b7a0edb, lib 373b5477b, port 7996), sur une copie de la même base migrée, 50
`search_code`, deux paires dans les deux ordres :
- **mémoire** : 6,0 → 5,0 Gio, la pente aplatie ;
- **vitesse** : les appels sont 70 à 77 % plus lents sous le réglage, quel que soit l'ordre.

**Le réglage ne se pose pas.** La ligne reste une note dans la page 03 de G.

**Pourquoi l'hôte de master est à 6 Gio et ma sonde montait à 20** : master porte le court-circuit
d'arbre principal. Sans filtre ni vue, il n'y a plus de masque du tout, seulement la remontée des k
résultats. Ma sonde tournait sur vue-publiee, sans ce court-circuit : sans filtre, elle rejouait la
jointure du prédicat sur 72 652 versions, dans le lexical et dans le dense, à chaque appel. Ce n'était
pas la projection de 2,9 Mo, déjà retirée par la vue.

Ma sonde ne voyait aucun coût de vitesse au réglage (2,5 s contre 2,8 s), là où l'hôte en voit 70 à
77 %. Je n'en ai pas de cause sûre. La seule différence que je vois est le nombre de fils qui
allouent ensemble dans l'hôte.

## Ce qui reste ouvert

**Le cas filtré, sur une base à versions.** Il garde le masque : en liste sur master aujourd'hui, en
prédicat avec vue-publiee. La sonde le voyait plat vers 20 Gio, sans réglage. Il se mesure **sur
l'hôte réel une fois vue-publiee posée**, pour la mémoire comme pour la vitesse. Ce ticket se ferme sur
cette mesure, ou se change en ticket de la jointure si elle garde la mémoire.

