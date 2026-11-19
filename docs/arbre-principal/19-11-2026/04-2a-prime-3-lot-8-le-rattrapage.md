# 2a′-3, lot 8 : le rattrapage de l'histoire, et le doublon au bord haut

19 novembre 2026, arbre principal. La forme est acceptée par c7 le même jour. Le lot prolonge
`docs/arbre-principal/18-11-2026/03-2a-prime-3-en-sept-lots.md` et la page 03 du 14 novembre (la règle (2)).

## Le rang d'une génération d'histoire

- **Le rang est la position du commit sur la première parente de la ref**, comptée depuis la base de la couche :
  `git rev-list --first-parent --count <base>..<commit>`.
  - La base est le commit de la génération de la parente épinglée, ou la racine de la ref pour une couche sans
    parente.
  - Les lots 3 à 6 donnaient le rang en attente, c'est-à-dire le rang publié plus un.
- **La pointe a le plus grand rang.** Une génération rattrapée tombe **dans le trou**, sous elle. La vue sans rang lit
  le plus grand rang publié, donc toujours la pointe.
- **Chaque génération enregistre son commit**, comme aujourd'hui (`_generation.commit`).

## Écrire à un rang donné

`Catalog::write_in_layer` écrit aujourd'hui au rang en attente. Le lot lui donne un rang r et un plafond R, le
prochain rang déjà écrit au-dessus de r, ou rien.

- **Les fichiers écrits** sont ceux qui changent entre la génération écrite juste sous r (ou la base) et le commit de
  r.
- **Une ligne née à r est retirée à R** quand R a réécrit son fichier. Elle reste vivante au-dessus sinon.
- **Une ligne plus ancienne du même fichier**, née sous r et retirée à R, reste telle quelle. Aucune ligne publiée ne
  change : sa fermeture n'est pas abaissée à r, et la vue à r la masque par la règle de visibilité de la couche, la
  ligne née à r prenant sa place.

## Le doublon au bord haut d'un trou

Quand R portait déjà, pour un fichier, le même contenu que r, il y a deux lignes identiques : l'une née à r et retirée
à R, l'autre née à R. On ne descend jamais `_depuis`, qui est dans l'uuid. **L'état compte ces doublons, par couche.**
La règle coûte des lignes, jamais une identité.

## Le rythme

- `history: all` et `{last: N}` rattrapent paresseusement, le plus récent d'abord.
- La synchro (`sync_tracked_layers`) fait au plus un pas : 5 générations ou 30 s.
- `tip` ne rattrape rien.

## Une ref réécrite

À chaque synchro, le commit enregistré de chaque génération écrite est vérifié : il doit être à sa position sur la
première parente de la ref. Si ce n'est plus le cas (un rebase, un push forcé ailleurs), la couche se dit
**« à rebâtir »** en nommant la ref et le commit perdu. Elle ne reçoit plus de génération tant qu'elle n'est pas
rebâtie. Jamais un rang faux en silence.

## Le témoin 7, rouge d'abord

1. La pointe d'abord, au rang R.
2. Le rattrapage d'un commit plus ancien, au rang r < R. La vue au rang R est inchangée.
3. La vue au rang r lit le fichier tel qu'à son commit, et la vue sans rang lit toujours la pointe.
4. L'état compte le doublon au bord haut.
5. Une ref réécrite rend la couche « à rebâtir », en le nommant.
