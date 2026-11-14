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
| 1 | lucivy | montée 4.3.7 | lucivy-4-3-7-4 | cd6d02907, la base de la pile — **atterri** à 16 h 16, master c99b2f05e, contenu identique |
| 2 | tty | tty-6 (tmux/mosh mesurés) | cx-tty-6-2 → cx-tty-6-pile | 4583eff8e → pile 7ac2c13d9 sur cd6d02907 |
| 3 | recherche | surveillant, puis plafond d'instantané | surveillant-2, plafond-d-instantane → recherche-pile | a36f707dd, 9b7ff76e8 → pile 2c3bc3eb7 sur 7ac2c13d9 |
| 4 | G | cas 6c du poste (trap avant la CI) | tenant-signal → tenant-signal-pile | ab7c44fb5 → pile c93f73f06 sur 2c3bc3eb7 |
| 5 | embarquement | reprise après plantage | reprise-apres-plantage → reprise-apres-plantage-pile | 17afd3820 → pile 512b5e420 sur c93f73f06 (conflit de docs/nouveautes.md résolu) |
| 6 | 96 | miroir, nom exact, about | lot-96-pile (miroir 48445fdeb, nomex bff4ea79f, about 84f949d2c) | pile 84f949d2c sur 512b5e420 |
| 7 | hotfixs | témoin du dictionnaire (C++ test seul) | chaine-au-point-de-reprise-2 → chaine-au-point-de-reprise-pile | 9ded42107 → pile cabcc8640 sur 84f949d2c |
| 8 | ED | 2a′-1 (générations de l'ancre) | generations-de-l-ancre-3 → generations-de-l-ancre-pile | 9b0620f66 → pile c22fdbf31 sur cabcc8640 (19 commits) |
| 9 | archi | jauge du contexte (+ quatre mineurs s'ils sont verts) | jauge-ce-qui-pese-3 → jauge-ce-qui-pese-pile | 5ad3c990e → pile 214e9a99b sur c22fdbf31 |
| 10 | cœur | correctif (a) perte de Note | annulation-garde-les-autres-2 → annulation-garde-les-autres-pile | 9b0e73c9c → pile 960eb8e3a sur 214e9a99b ; juge vers 17 h 15 |
| 11 | G | la garde apprend la pile | garde-pile | 06c0a3771 sur c93f73f06 (vert 16 h 15, 25 témoins ; relecture recherche ; la copie du cache la porte avant master) |
| 12 | codeparsers | l'import pointé (lot 1) | à venir | à son vert |

Chaque session dit à c7 et au rang suivant la tête de sa branche de pile dès qu'elle est poussée.

## La garde apprend la pile (16 h 2x, vu par recherche)

`push_garde.sh` ne savait pas la pile : après `rebase --onto origin/master <tête d'avant>`, HEAD ne
descend plus du commit testé, et la base du rejeu ferait voir le code des lots d'avant. G écrit un
mode `PILE_AVANT=<tête du lot d'avant>` dans un lot à part, `garde-pile` (rang 11 : sa tête de pile était
déjà publiée et prise pour base par embarquement) : (a) `git range-diff
<avant>..<testé> origin/master..HEAD` à `=` pour tout commit qui touche du code ; un `!` toléré si la
différence entre les deux versions ne touche que des `.md` (le conflit de `docs/nouveautes.md` résolu au
rebase), un `>` toléré s'il ne touche que des `.md` ; (b) `<avant>` ancêtre du testé ; (c) `code_apporte(<avant>, origin/master)` vide, sinon
« la pile a changé » et rejeu. Rouge d'abord dans `test_push_garde.sh`, relecture par recherche,
recopie au cache des deux postes au mot de c7, avant le dernier pas de tty, le premier de la pile.
La copie du cache fait autorité et n'attend pas master : entre la recopie et le rang 11, `--derive`
dit « garde en retard sur le dépôt » sur les pushs de la pile, c'est attendu et ce n'est pas un arrêt.

## Premier atterrissage (16 h 16) et une leçon de la garde

lucivy a poussé c99b2f05e : ses trois commits de cd6d02907 rebasés par la garde sur abe3ff1e7,
contenu identique, « rien de code » entre la base testée et master. La pile tient : tty rejoue sur
cd6d02907. Deux aléas au push : un premier essai refusé « fetch first » (master avait bougé entre la
relecture et le push) a laissé le worktree rebasé, et le second essai s'est arrêté sur « HEAD ne
descend pas du commit testé » ; remis sur la tête testée par `checkout --detach`, le troisième est
passé. **À faire dans garde-pile** : un push refusé remet le worktree sur la tête testée.

## Le dernier pas en mode pile (dès que la copie du cache a garde-pile)

L'appelant pose la variable : `PILE_AVANT=<tête du lot d'avant> poste push <session> -- push_garde.sh
<worktree> <journaux>`. La garde remplace alors « HEAD descend du testé » et « code depuis la base »
par `verifier_pile` : (b) la tête d'avant est un ancêtre du commit testé ; (c) `code_apporte(avant,
master)` vide, sinon « ARRÊT : la pile a changé » ; (a) `git range-diff avant..testé master..HEAD` à
`=`, `!` toléré si les deux versions du commit ont le même diff hors `.md`, `>` toléré s'il ne touche
que des `.md`, le reste refusé. Sans `PILE_AVANT`, rien ne change. Les têtes d'avant : tty cd6d02907 ·
recherche 7ac2c13d9 · G 2c3bc3eb7 · embarquement c93f73f06 · 96 512b5e420 · hotfixs 84f949d2c · ED
cabcc8640 · archi c22fdbf31 · cœur 214e9a99b. Un push refusé (« fetch first ») remet le worktree sur
la tête testée : à ajouter dans garde-pile (aléa de lucivy).
