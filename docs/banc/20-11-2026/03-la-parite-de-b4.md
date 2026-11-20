# La parité de B4, le Decide en parts

20 novembre 2026, à la demande de mémoire relayée par c7. B4 (`cx-memoire-b4-decide` `2884c9ed37`) donne au Decide de
l'archiviste ses parts : « ce qui précède », stable sur tout le document ; la section, bornée à 1 000 jetons ; les
candidates, une par ligne. La consigne passe dans le message système. **Ce qu'on veut voir monter, c'est la part des
invites du Decide servie par le cache.** Avant : son parent `c4e92b082f` (`memoire-extraction-cut`).

## S 50 (joué de 3 h 49 à 4 h 53)

Les mêmes 19 fenêtres de 50 000 jetons du corpus `c7-99bc8451-f5` que la parité de B3, S 50 nu (aucun contexte d'avant),
rangées par `archive_slice`. Chaque côté a son binaire, ses gabarits et une base neuve ; Gemini 3.8 Flash sur Vertex ;
les deux passes en série. L'extraction est en `low` des deux côtés (le défaut du parent).

| | avant (`c4e92b082f`) | après B4 (`2884c9ed37`) |
|---|---|---|
| fenêtres en échec | 0 sur 19 | 0 sur 19 |
| Decide | 85 | 98 |
| invite par Decide : médiane, moyenne | 1 994, 2 041 jetons | 2 004, 2 019 jetons |
| **part de l'invite des Decide servie par le cache** | **0 %** (0 appel) | **0 %** (0 appel) |
| fiches | 27 | 33 |
| P1 contre N | 0,778 (42/54) | 0,796 (43/54) |
| P2, doublons | 1 (0,037) | 1 (0,030) |
| l'autre côté couvert | 0,606 des fiches d'après | 0,815 des fiches d'avant |
| invite de l'extraction (rendue) | 580 798 jetons, 20 371 en cache | 580 798 jetons, 179 381 en cache |

- **Le cache ne peut pas monter en S 50 nu.** Un Decide y fait 2 000 jetons d'invite. C'est sous le seuil de 6 144 jetons
  du cache implicite de Gemini 3.8 Flash, avant comme après B4 : aucun Decide n'en a été servi. Sans contexte d'avant,
  la part stable de l'invite (la consigne, le gabarit des candidates) est trop courte.
- **Le cache de l'extraction ne dit rien de B4** : les deux passes rejouent mot pour mot les invites d'extraction des
  passes de la nuit (même tranche, même consigne), que Vertex sert depuis son cache implicite.
- **P1 ne bouge pas** au-delà de l'écart d'un jeu à l'autre (± 0,07, page `02-le-raisonnement-de-l-extraction.md`) :
  0,778 contre 0,796. B4 ne coûte pas de sujets.

## F300 (joué de 9 h 13 à 11 h 50)

La forme F300 du 18 novembre (`x_archive_long.py`, `VARIANTE=F`, fenêtre de 300 000 jetons, chevauchement de 10 %,
résumés de 60 000) : 244 sections, une décision `archive` chacune, avec le contexte d'avant (sa place, les résumés des
fenêtres d'avant, le chevauchement, les tours d'avant de la fenêtre). Les deux binaires bâtis pour S 50, en série, une
base neuve chacun. La passe « après » a duré 100 min, contre 55 : Vertex a ralenti en fin de passe (jusqu'à 65 s par
tour).

| | avant (`c4e92b082f`) | après B4 (`2884c9ed37`) |
|---|---|---|
| décisions, erreurs | 244, 0 | 244, 0 |
| create, fiche, none | 56, 149, 39 | 58, 159, 27 |
| P1 contre N | **0,907** (49/54) | **0,815** (44/54) |
| P2, doublons | 5 (0,128) | 4 (0,091) |
| invite des Decide | 27 112 496 jetons | 27 085 664 jetons |
| **servie par le cache** | **11 545 746 (42,6 %)** | **11 162 764 (41,2 %)** |
| tours servis par le cache | 110 sur 244 | 108 sur 244 |
| part du cache d'un tour servi (médiane) | 89,3 % | 87,8 % |
| par fenêtre (1, 2, 3) | 33,9 %, 43,7 %, 43,5 % | 32,0 %, 44,3 %, 40,3 % |

**Ce qu'on peut en dire**

1. **B4 ne fait pas monter le cache.** Il reste à 41-43 % des deux côtés.
2. **Le cache se joue en tout ou rien, appel par appel.** Un tour sur deux est servi à ~88 % de son invite, l'autre pas
   du tout, avant comme après B4. Le préfixe stable existe donc déjà avant B4 : le contexte de F300 ne s'allonge que par
   sa fin, d'un tour au suivant. C'est le cache **implicite** de Vertex qui ne le sert qu'une fois sur deux. Le levier
   est le cache **explicite** (ticket R, section du 19 novembre), pas l'ordre des parts du Decide.
3. **P1 baisse de 0,907 à 0,815.** Le côté avant retrouve exactement le P1 de la F300 du 18 novembre (0,907) : sur
   F300, P1 a l'air stable d'un jeu à l'autre, contrairement à S 50 (± 0,07). Une baisse de 0,09 est donc à prendre au
   sérieux. Les décisions changent : 27 « none » contre 39, et 10 rangements de plus sous une fiche existante. B4 range
   plus volontiers sous une candidate, et perd 5 sujets de N. **Un second jeu « après » est nécessaire** pour dire si
   la baisse est réelle, avant d'en tirer une conclusion sur B4.

## F300, second jeu « après » (12 h 41 à 13 h 31)

Le même binaire de B4, la même forme, une base neuve.

| | avant | après, 1ᵉʳ jeu | **après, 2ᵉ jeu** |
|---|---|---|---|
| create, fiche, none | 56, 149, 39 | 58, 159, 27 | 64, 147, 33 |
| P1 contre N | 0,907 | 0,815 | **0,944** (51/54) |
| P2, doublons | 5 (0,128) | 4 (0,091) | **14 (0,28)** |
| servie par le cache | 42,6 % | 41,2 % | 43,4 % |

- **La baisse de P1 était l'écart d'un jeu à l'autre.** La même tête donne 0,815 puis 0,944. Sur F300 aussi, P1 varie
  d'une dizaine de points d'un jeu à l'autre. Le 0,907 d'avant, égal à celui du 18, était une coïncidence. **Verdict :
  B4 ne coûte pas de sujets, et il atterrit** (la règle de c7 : « s'il revient vers 0,90, c'était le bruit »).
- **Le cache est confirmé** : 43 %, comme les deux autres passes. B4 ne le fait pas monter.
- **Une réserve, les doublons** : 14 dans ce jeu, contre 4 et 5. Une passe de plus qui crée plus de fiches (64 « create »)
  en a aussi plus en double. On ne peut pas départager sur trois jeux : c'est à suivre aux prochaines mesures de
  l'archiviste, pas une raison de retenir B4.
- **Pour juger un réglage de l'archiviste**, il faut désormais plusieurs jeux par côté, en S 50 comme en F300.
