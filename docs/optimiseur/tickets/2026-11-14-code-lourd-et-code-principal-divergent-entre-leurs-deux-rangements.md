# code-lourd et code-principal divergent entre leurs deux rangements

- **État** : ouvert (14 novembre 2026, 21 h 26) ; la bascule de luciepc à 373b5477b s'est faite quand même, au mot de c7.
- **Famille** : produit
- **Gravité** : donnée ; une lecture par un sens et par l'autre ne rendent pas les mêmes relations.
- **Atteignable en service** : oui, les deux bases sont servies par l'hôte de luciepc.
- **Touche rag3weaver** : non, c'est l'état de la base ; ses requêtes de lecture le voient selon le sens du plan.
- **Pour** : coeur c++/hotfixs, dans sa fenêtre de purge, à départager avec les arêtes en double de la sonde du 13 novembre.
- **Ouvert le** : 14 novembre 2026

## Ce qui a été vu

Le contrôle des deux sens de la bascule vers 373b5477b (`docs/procedures/scripts/controle-des-deux-sens.sh`, outil
`tools/check_rel_directions` de origin/master au commit 207701bb2, bâti contre 373b5477b) a lu l'instantané
`avant-373b5477b` de code-lourd à luciepc : la base telle que 0c41f9fe6 l'a laissée, arrêtée proprement à 20 h 52,
avant toute ouverture sous la lib neuve. La base vivante est datée du 14 novembre à 14 h 25 (1,4 Gio).

Trois tables divergent, les dix-neuf autres sont égales (MENTIONS 466 845, CONSUMES 151 744, DEFINES 96 012…) :

| table | sens avant / arrière | d'un seul côté | vues deux fois | bouts différents |
|---|---|---|---|---|
| INHERITS_FROM | 650 / 658 | 1 | 11 | 13 |
| QUERIES | 20 093 / 20 093 | 5 | 6 | 9 |
| LOCKS | 544 / 546 | 0 | 3 | 5 |

Exemples (identifiants internes, table 11 = les scopes) :
- INHERITS_FROM : `29:0, 11:17705 -> 11:0` et `29:0, 11:74862 -> 11:66873` vues deux fois au sens avant ;
  `29:357` à `29:360`, de `11:5359`… `11:5356` vers `11:5354`, aux bouts différents.
- QUERIES : `47:1244, 11:71894 -> 19:61109` et `47:2185, 11:69226 -> 19:60902` au seul sens avant.
- LOCKS : `45:9` à `45:12` vers `19:71667` aux bouts différents ; `45:0, 11:0 -> 19:79584` vue deux fois au sens arrière.

Le rapport entier, avec les dix exemples de chaque table : à luciepc,
`~/.local/share/rag3weaver/instantanes/avant-373b5477b-controle/code-lourd.log`. L'extension vecteur ne s'y charge pas
(bâtie pour 0c41f9fe6, l'outil tourne sous 373b5477b) : les tables de relations n'en dépendent pas.

### code-principal, la même passe, à 21 h 31

Base datée elle aussi du 14 novembre à 14 h 25. Trois tables divergent, quinze sont égales :

| table | sens avant / arrière | d'un seul côté | vues deux fois | bouts différents | `line` à deux valeurs |
|---|---|---|---|---|---|
| QUERIES | 20 210 / 20 210 | 7 | 8 | — | — |
| IMPLEMENTS | 331 / 334 | 3 | 8 | 9 | 6 |
| LOCKS | 425 / 436 | 0 | 12 | 9 | 16 |

Exemple : `47:12799, 11:55096 -> 19:47004` au seul sens avant (QUERIES). **Une forme de plus qu'à code-lourd** :
des relations dont la propriété `line` rend deux valeurs selon le sens. Pour le cœur, c'est la même cause lue sur
une propriété : l'annulation d'un COMMIT refusé tronquait les deux index séparément, sous l'ancienne lib.
Rapport : à luciepc, `~/.local/share/rag3weaver/instantanes/avant-373b5477b-controle/code-principal.log`.

Au portable, la même passe ne voit aucun écart sur code-rag3db-depot et memoire-longue.

## Ce qui reste à faire

- Départager : défaut ancien de la base (les demi-arêtes de LOCKS et QUERIES du 25 octobre, les arêtes en double de
  la sonde du 13 novembre) ou écriture récente sous 0c41f9fe6. Rejouer l'outil sur un instantané plus ancien de
  code-lourd dit depuis quand.
- Réparer ou réindexer les deux bases dans la fenêtre de purge d'hotfixs, sur instantané, pas à l'aveugle : hotfixs reconstruit déjà les trois tables de code-lourd sur copie, et ajoute QUERIES, IMPLEMENTS et LOCKS de code-principal.

## La décision du 14 novembre

c7 a tranché à 21 h 30 pour code-lourd, puis à 21 h 33 pour code-principal : luciepc bascule à 373b5477b malgré les écarts. Ses raisons :
- l'écart est **antérieur** à la lib neuve : il est lu sur l'instantané pris sous 0c41f9fe6 ;
- il est **petit** : des demi-arêtes, la famille de celles du 25 octobre et du ROLLBACK ;
- il est de la famille que le correctif de 373b5477b vise ;
- revenir en arrière à luciepc seul laisserait les deux postes sur deux libs, ce que la procédure interdit plus
  fermement encore.

La procédure de bascule (§ 8) le dit désormais.
