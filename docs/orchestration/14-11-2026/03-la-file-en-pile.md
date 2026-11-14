# La file en pile : rejouer ensemble, pousser un par un

14 novembre 2026, 16 h 06. Lucie : « rien de commençable en parallèle, même si invalidable de
suite ? si on définit des contrats ça peut se faire non ? ». Oui : c'est le pipeline dépendant
(Zuul) ou le train de fusion (bors). Ce qui coûtait n'était pas le push mais le rejeu : chaque lot
qui atterrissait invalidait le rejeu de tous ceux qui attendaient, et ils rejouaient l'un après
l'autre, dix fois de suite, même sans rouge.

## Le contrat

1. **La file est publiée, fixe, avec le hash de chaque lot** (ci-dessous). Un lot en file ne change
   plus son contenu. S'il doit changer (un rouge, une relecture), il le dit à c7 et au lot derrière
   lui **dans la minute**, sort de la file et revient derrière.
2. **Chaque session rejoue sur la pile de ceux devant elle**, pas sur master : sa branche rebasée
   sur la tête du lot d'avant, elle-même rebasée sur celle d'avant, jusqu'à master. La branche de
   pile se pousse sous un nouveau nom (`<branche>-pile`) et son hash se dit au lot derrière et à c7 :
   c'est la base du suivant. Tout le monde rejoue **en même temps**, dans la limite des lourds (au
   plus quatre à luciepc, un au portable) : les quatre premiers de la file bâtissent d'abord, les
   autres préparent leur worktree.
3. **Le rejeu court sur la pile vaut rejeu du lot** : ce que le lot touche, plus la lib entière et
   check_features pour du Rust, comme aujourd'hui ; `e2e_lectures` si le lot touche l'arrêt ou le bus,
   `e2e_postgres` si code.rs, l'IR ou un gabarit.
4. **Au « à toi », rien ne se rejoue si tout a atterri comme prévu.** Le dernier pas :
   `git rebase --onto origin/master <tête du lot d'avant>` (ses commits seuls),
   `push_garde.sh --code-apporte <tête du lot d'avant> origin/master` : s'il ne voit que des docs
   (les lots d'avant ont atterri avec le contenu testé, à des hashes qui peuvent différer), le push
   part sans rejeu, sous `poste push`, dans `if grep -qx VERT`, `--derive` « garde à jour », trois
   contrôles lus. S'il voit du code — un lot devant a changé ou a été retiré —, rejeu court sur la
   nouvelle pile, à son tour.
5. **Le « à toi » reste le seul mot de push**, un à la fois ; il ne porte plus le rejeu. Les docs
   seules passent hors file comme avant.
6. **Un rouge sur la pile se lit d'abord contre son propre lot** : avant d'accuser un lot d'avant,
   la session rejoue le témoin sur sa branche seule rebasée sur master. Si le rouge vient d'un lot
   d'avant, elle le lui dit et à c7, avec le témoin : c'est ce lot qui sort.
7. **Le C++ du moteur** (`src/`, `test/`) n'est pas lié par la lib épinglée : un lot C++ seul
   rebase sans rejeu Rust et rejoue ses tests sur la tête rebasée ; il entre dans la pile comme les
   autres, sans en changer la base Rust.

## La file au 14 novembre, 16 h 06 (master aa953c8f6)

| rang | session | lot | branche | tête testée |
|---|---|---|---|---|
| 1 | lucivy | montée 4.3.7 | lucivy-4-3-7-4 | cd6d02907, rebasée sur aa953c8f6 : la base de la pile (rejeu court en cours à luciepc) |
| 2 | tty | tty-6 (tmux/mosh mesurés) | cx-tty-6-2 → cx-tty-6-pile | 4583eff8e → pile 7ac2c13d9 sur cd6d02907 |
| 3 | recherche | surveillant, puis plafond d'instantané | surveillant-2, plafond-d-instantane → recherche-pile | a36f707dd, 9b7ff76e8 → pile 2c3bc3eb7 sur 7ac2c13d9 |
| 4 | G | cas 6c du poste (trap avant la CI) | tenant-signal → tenant-signal-pile | ab7c44fb5 → pile c93f73f06 sur 2c3bc3eb7 |
| 5 | embarquement | reprise après plantage | reprise-apres-plantage (luciepc) | 17afd3820 |
| 6 | 96 | miroir, nom exact, about | (miroir), lot-96-nom-exact, lot-96-about | —, ccd7f721b, 73c20f333 |
| 7 | hotfixs | témoin du dictionnaire (C++ test seul) | chaine-au-point-de-reprise-2 | 9ded42107 |
| 8 | ED | 2a′-1 (générations de l'ancre) | generations-de-l-ancre-3 | 9b0620f66 (check_features corrigé ; passe en cours) |
| 9 | archi | jauge du contexte (+ quatre mineurs s'ils sont verts) | jauge-ce-qui-pese-3 | 5ad3c990e |
| 10 | cœur | correctif (a) perte de Note | annulation-garde-les-autres-2 | 9b0e73c9c, à son juge vert |
| 11 | G | la garde apprend la pile | garde-pile | à venir (la copie du cache la porte avant master) |
| 12 | codeparsers | l'import pointé (lot 1) | à venir | à son vert |

Chaque session dit à c7 et au rang suivant la tête de sa branche de pile dès qu'elle est poussée.

## La garde apprend la pile (16 h 2x, vu par recherche)

`push_garde.sh` ne savait pas la pile : après `rebase --onto origin/master <tête d'avant>`, HEAD ne
descend plus du commit testé, et la base du rejeu ferait voir le code des lots d'avant. G écrit un
mode `PILE_AVANT=<tête du lot d'avant>` dans un lot à part, `garde-pile` (rang 11 : sa tête de pile était
déjà publiée et prise pour base par embarquement) : (a) `git range-diff
<avant>..<testé> origin/master..HEAD` à `=` seuls, sauf des commits ajoutés qui ne touchent que des
`.md` ; (b) `<avant>` ancêtre du testé ; (c) `code_apporte(<avant>, origin/master)` vide, sinon
« la pile a changé » et rejeu. Rouge d'abord dans `test_push_garde.sh`, relecture par recherche,
recopie au cache des deux postes au mot de c7, avant le dernier pas de tty, le premier de la pile.
La copie du cache fait autorité et n'attend pas master : entre la recopie et le rang 11, `--derive`
dit « garde en retard sur le dépôt » sur les pushs de la pile, c'est attendu et ce n'est pas un arrêt.
