# Accélérer les passes : ce que mesurent les passes du 22 novembre

Session G (optimiseur), 22 novembre 2026, 22 h. À la demande de c7 : Lucie trouve les passes longues. Trois pistes,
mesurées avant de choisir.

## Ce que coûte un lot aujourd'hui

Relevé sur le journal du pousseur à luciepc (`pusher/log.tsv`) et les dossiers de passe (`pusher-20261122-*`), le
22 novembre de 00 h à 22 h : 70 lots poussés, 17 rendus rouges, 52 passes complètes.

| du dépôt au verdict | médiane | moyenne | max |
|---|---|---|---|
| attente en file | 30 min | 78 min | 274 min |
| train (repose, passe, push) | 13 min | 10 min | 58 min |
| total | 40 min | 88 min | 288 min |

La moyenne d'attente compte les pauses du jour : le disque de luciepc de 08 h à 12 h, deux fenêtres C++. **L'attente
domine, pas la passe.** La file est un guichet unique : chaque minute retirée d'un train est retirée à chaque lot
derrière lui. Un lot rouge coûte en plus son rejeu, 64 min en médiane du dépôt au rouge.

Une passe complète, par étape (heure de fin de chaque journal) :

| étape | médiane | moyenne | max |
|---|---|---|---|
| témoins | 1,1 min | 1,1 | 1,6 |
| e2e_lectures | 1,5 min | 2,0 | 6,5 |
| e2e_code | 1,5 min | 1,5 | 2,3 |
| check_features | 2,8 min | 4,0 | 17,8 |
| lib entière | 3,5 min | 7,8 | 30,0 |
| total | 11,3 min | 17,8 | 54,7 |

## Les trois pistes, par gain

1. **Ne plus rejouer la lib entière** quand l'auteur l'a jouée verte sur la tête exacte que le pousseur prend sans
   repose (même arbre, même lib, preuve dans l'attestation). Gain : 3,5 min par lot en médiane, 7,8 en moyenne, jusqu'à
   30 sous charge. C'est aussi l'étape qui rougit par délai (code 124). Limite : la moindre repose qui apporte du code
   la refait jouer (règle « rejouer après un rebase qui apporte du code »). Sur la journée, la part des lots pris sans
   repose reste à compter : c'est le premier chiffre du lot.
2. **`check_features` en `cargo check`** sur les 8 combinaisons. Gain attendu : la moitié de ses 2,8 à 4 min (pas
   d'édition de liens ni de code machine). Ce qu'il prouve reste le même : chaque combinaison compile, avec ses
   avertissements. Il ne prouve pas qu'un binaire se lie ; la lib et les e2e le prouvent déjà sous la combinaison complète.
3. **sccache partagé entre les targets** (base : `poste-sccache-server-outside-2`). Le gain est surtout hors du
   pousseur : la première compilation de chaque session et de chaque worktree neuf (lucivy et les dépendances), et la
   passe de l'auteur, qui précède le dépôt. Dans le train, le target du pousseur est déjà chaud. Mesure à faire : le
   taux de succès de sccache sur une passe d'auteur à target froid.

## Ce que je propose

Dans l'ordre : (1), puis (2), chacun en lot avec son témoin rouge d'abord ; (3) après une mesure sur une passe
d'auteur. Ensemble, (1) et (2) retirent de 5 à 10 min par train, soit de 40 à 75 % du train médian, et autant à
l'attente de chaque lot derrière.
