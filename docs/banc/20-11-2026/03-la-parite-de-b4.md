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

## F300 (à 9 h 30)

C'est en F300 que B4 peut se voir : chaque Decide y reçoit les résumés des fenêtres d'avant et le chevauchement, soit
des dizaines de milliers de caractères stables d'une section à l'autre de la même fenêtre. La paire F300 (environ 75
minutes par passe, comme le 18) ne tenait pas avant la fenêtre du matin de 6 h 45 à 9 h 30. Elle part à 9 h 30, avec les
deux binaires déjà bâtis. Ses chiffres complèteront cette page.
