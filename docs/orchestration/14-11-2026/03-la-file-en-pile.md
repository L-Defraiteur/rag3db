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
4. **Au « à toi », rien ne se rejoue si tout a atterri comme prévu.** Le dernier pas, **sans rebase à
   la main** (appris aux trois premiers pushs : rebaser avant déplace HEAD et la garde ne retrouve
   plus l'intervalle testé) : le worktree reste sur la tête testée, les commits de docs ajoutés
   (un ticket) se posent par-dessus, et `PILE_AVANT=<tête du lot d'avant> poste push … push_garde.sh`
   fait lui-même `rebase --onto master <tête d'avant>` après avoir vérifié que la pile d'avant est
   sur master avec le contenu testé (§ « Le dernier pas en mode pile »). Le push part sous `poste
   push`, dans `if grep -qx VERT`, trois contrôles lus. Si la pile a changé — un lot devant a changé
   ou a été retiré —, « ARRÊT : la pile a changé », et c'est un rejeu court sur la nouvelle pile.
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
| 2 | tty | tty-6 (tmux/mosh mesurés) | cx-tty-6-2 → cx-tty-6-pile | 7ac2c13d9 sur cd6d02907 — **atterri** à 16 h 25, master ce4a3140a, range-diff 5/5 en `=`, premier push en mode pile |
| 3 | recherche | surveillant, puis plafond d'instantané | surveillant-2, plafond-d-instantane → recherche-pile | 2c3bc3eb7 sur 7ac2c13d9 — **atterri** à 16 h 27, master a48b535d1, dix commits en un push, pile tenue |
| 4 | G | cas 6c du poste (trap avant la CI) | tenant-signal → tenant-signal-pile | c93f73f06 sur 2c3bc3eb7 — **atterri** à 16 h 30, master 89f46c87d, pile tenue ; cas 6c clos |
| 5 | embarquement | reprise après plantage | reprise-apres-plantage → reprise-apres-plantage-pile | 512b5e420 sur c93f73f06 — **atterri** à 16 h 39, master 67dabc121, 6/6 en `=` plus le `>` du ticket ; le `!` attendu sur nouveautes.md n'a pas eu lieu, la pile d'avant ayant atterri avec les mêmes lignes de tête |
| 6 | 96 | miroir, nom exact, about | lot-96-pile (miroir, nomex, about) | 84f949d2c sur 512b5e420 — **atterri** à 16 h 40, master c350dc9c3, trois lots en un push, 3/3 en `=` |
| 7 | hotfixs | témoin du dictionnaire (C++ test seul) | chaine-au-point-de-reprise-2 → chaine-au-point-de-reprise-pile | cabcc8640 sur 84f949d2c — **atterri** à 16 h 44, master 42c84cb77, pile tenue |
| 8 | ED | 2a′-1 (générations de l'ancre) | generations-de-l-ancre-3 → generations-de-l-ancre-pile | 9b0620f66 → pile c22fdbf31, puis **e00fd562d** sur cabcc8640 (19 commits + le témoin e2e_scope_versions corrigé ; rang gardé, un commit ajouté) |
| 9 | archi | jauge du contexte (+ quatre mineurs s'ils sont verts) | jauge-ce-qui-pese-3 → jauge-ce-qui-pese-pile | 5ad3c990e → pile 214e9a99b, puis **67d759962** sur e00fd562d (jauge-ce-qui-pese-pile-2) |
| 10 | cœur | correctif (a) perte de Note | annulation-garde-les-autres-2 → annulation-garde-les-autres-pile | 9b0e73c9c → pile 960eb8e3a, puis **41a7de35a** sur 67d759962 (annulation-garde-les-autres-pile-2) ; juge vers 17 h 15 |
| 11 | recherche | la CI : `fetch_update` déprécié par la chaîne stable de la CI (read_pool.rs:261), pas par le poste 1.98.1 | ci-try-update-pile | à venir, sur la tête du cœur 41a7de35a |
| 12 | G | la garde apprend la pile | garde-pile | cd103b619 sur c93f73f06 (garde 2f1914405 : --onto, push refusé remis, 27 témoins ; relue par recherche à 16 h 2x ; **posée au cache des deux postes à 16 h 22**, sha256 c57a3fb1 → a58a649f, l'ancienne en push_garde.sh.avant-garde-pile) |
| 13 | codeparsers | l'import pointé (lot 1) | à venir | à son vert |
| 14 | recherche | exclusions 1 et 5 dans le manifeste | à venir | à son vert |
| 15 | tty | tty-7, le journal de séance et le repli sous mosh | cx-tty-7 dc6d484c6 | rejoue sur 41a7de35a ; rejeu court probable à son tour, accepté |

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

L'appelant pose la variable, **sans avoir rebasé** : `PILE_AVANT=<tête du lot d'avant> poste push <session>
-- push_garde.sh <worktree> <journaux>`, le worktree sur la tête testée. La garde remplace alors « HEAD descend du testé » et « code depuis la base »
par `verifier_pile` : (b) la tête d'avant est un ancêtre du commit testé ; (c) `code_apporte(avant,
master)` vide, sinon « ARRÊT : la pile a changé » ; (a) `git range-diff avant..testé master..HEAD` à
`=`, `!` toléré si les deux versions du commit ont le même diff hors `.md`, `>` toléré s'il ne touche
que des `.md`, le reste refusé. Sans `PILE_AVANT`, rien ne change. Les têtes d'avant : tty cd6d02907 ·
recherche 7ac2c13d9 · G 2c3bc3eb7 · embarquement c93f73f06 · 96 512b5e420 · hotfixs 84f949d2c · ED
cabcc8640 · archi e00fd562d · cœur 67d759962. Un push refusé (« fetch first ») remet le worktree sur
la tête testée : à ajouter dans garde-pile (aléa de lucivy).

## Un lot change dans la pile (16 h 3x) : le rang se garde quand c'est un commit ajouté

La passe entière d'ED était rouge à un témoin d'avant le lot (`une_base_neuve_recoit_l_estampille_des_versions`,
qui figeait la forme Versions ; 2a′-1 pose Generations, c'est l'objet du lot). Le contrat dit « sort et revient
derrière » ; ici sortir coûtait plus : archi et le cœur sont posés sur ses 19 commits, les retirer de sous eux
changeait leur code autant qu'un témoin ajouté. **Règle précisée** : un changement qui est un **commit ajouté**
par-dessus la pile (pas une réécriture) garde son rang ; ceux derrière se reposent sur la nouvelle tête et
rejouent leur court (un fichier de tests/ est du code pour la garde) ; un C++ seul derrière ne rejoue rien.
Une réécriture des commits, elle, sort de la pile.

La relecture de recherche sur garde-pile : (c) juge les deux côtés, le `!` compare les diffs hors .md (les `@@`
gardés, côté prudent), sans PILE_AVANT rien ne change ; **un défaut** : le chemin du push faisait `git rebase
$neuf` sur une HEAD de pile au lieu de `git rebase --onto $neuf $PILE_AVANT` — un lot d'avant atterri après un
conflit aurait fait s'arrêter le push à tort. Cas 8 sur le chemin complet, rouge d'abord, puis la recopie.

## Premier push en mode pile (16 h 25, tty)

La garde du cache (a58a649f) a imprimé : « garde en retard sur le dépôt » (attendu), « lib de toute la
passe : 0c41f9fe6 », « mode pile : tête d'avant cd6d02907, commit testé 7ac2c13d9 », « pile tenue :
les commits du lot sont ceux du rejeu, et la pile d'avant est sur master », les trois contrôles, POUSSE.
Deux écarts de forme pour garde-pile : `$J/base` lu avant de connaître le mode (« cat : Aucun
fichier », sans effet) ; la liste des journaux où la garde cherche la lib ne couvre pas une passe
courte (tty a copié `e2e_tty.log` en `temoins.log`) — à élargir à `*.log`, une seule lib, zéro journal
reste un arrêt. Et la lecture de `nproc` sous un lourd (G) : le quota du cgroup rend 5, pas 24 ;
`nproc --all` dans `fils_par` et le cas 8 du témoin, dans garde-pile, avec un ticket.

## Trois pushs de pile en cinq minutes (16 h 25 → 16 h 30)

tty (ce4a3140a), recherche (a48b535d1, dix commits en un push), G (89f46c87d) : trois « pile tenue »,
zéro rejeu. Deux défauts de forme de plus pour garde-pile, vus par G : `--derive` fait `realpath "$0"`
après le `cd` et imprime une empreinte vide quand `$0` est relatif ; et le `cat base` déjà dit.

## Cinq atterris (16 h 39) ; ce que la garde devrait dire de plus

embarquement : « mode pile : tête d'avant c93f73f06, commit testé 512b5e420 / pile tenue ». Le `!`
attendu sur nouveautes.md n'a pas eu lieu : la pile d'avant a atterri avec les mêmes lignes de tête
que celles sur lesquelles le conflit avait été résolu, le commit est `=`. Pour garde-pile : la phrase
« garde en retard sur le dépôt » dit l'inverse quand c'est la copie du cache qui est en avance (dire
les deux empreintes et le sens) ; `verifier_pile` doit imprimer le range-diff jugé, pas seulement
« pile tenue ».

## La CI rouge (16 h 45) : la chaîne de la CI flotte, celle du poste non

`poste push` signale « CI rouge depuis 15 h 30 UTC » à chaque push de la pile : depuis a48b535d1
(recherche), le jeu `sans-feature` de check_features rougit sur la CI sous `-D warnings` —
`Atomic::<u64>::fetch_update` déprécié (renommé `try_update`) à `src/read_pool.rs:261`. Le poste est
en rustc 1.98.1 et ne le voit pas ; la CI prend `dtolnay/rust-toolchain@stable`, plus récent. Ce
n'est ni un lot cassé ni le poste : c'est deux chaînes. Recherche corrige en une ligne au rang 11 ;
G ouvre le ticket du fond (épingler la chaîne de la CI à celle du poste par `rust-toolchain.toml`,
ou laisser flotter et monter le poste), décision à Lucie.
