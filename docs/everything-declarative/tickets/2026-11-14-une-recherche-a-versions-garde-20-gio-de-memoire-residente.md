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
