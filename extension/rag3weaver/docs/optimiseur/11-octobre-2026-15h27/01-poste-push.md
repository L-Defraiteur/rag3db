# `poste push` : un seul tenant pour pousser sur master

**11 octobre 2026, session optimiseur, à la demande de l'orchestration.** Ce
jour-là, trois sessions ont perdu trois courses chacune : du code arrivait sur
master pendant leur rejeu, et l'orchestration fermait des fenêtres à la main.
`poste push` remplace ces fenêtres : un verrou, un tenant à la fois, les
autres attendent en le disant.

## La forme

```bash
~/.cache/rag3weaver-build/poste push <session> [--lourd] -- <commande…>
~/.cache/rag3weaver-build/poste push --qui
```

- `push` attend le verrou `master-push.lock`, le dit toutes les 30 s
  (« B attend depuis 30 s — tenu par A depuis 14:53:30 (pid …) »), puis
  exécute **la commande de la session**, verrou tenu. Le verrou tombe à la fin
  de la commande, succès ou échec ; le code de la commande est rendu tel quel
  (75 est réservé au « verrou pas pris », une commande qui rend 75 est rendue
  74 pour ne jamais être relancée par la boucle d'attente).
- Au-delà de 20 minutes (`POSTE_PUSH_MAX`, en secondes), le tenant est **dit**
  sur la sortie d'erreur et dans la fiche, **pas tué**.
- `--qui` : « libre », ou « tenu — <session> depuis HH:MM:SS (pid …) ».
- Le script ne fetch, ne rebase et ne pousse rien lui-même ; il n'y a aucune
  force. Pas de plafond de mémoire sur `push` : le rejeu, dans la commande,
  passe par `poste lourd` comme d'habitude.

## L'ordre des verrous : le poste d'abord, le push ensuite

**Le trou** (arbre principal, 11 octobre au soir) : sa commande, sous le verrou de push,
rejouait sous `poste lourd`. Elle a attendu le poste 28 minutes derrière une mesure exclusive et
sa porte, en tenant le verrou de push pour rien, et tout le monde attendait derrière elle.

**La forme** : un ordre fixe des verrous.

```bash
~/.cache/rag3weaver-build/poste push <session> --lourd -- <commande…>   # ou POSTE_PUSH_LOURD=1
```

- `--lourd` prend d'abord la place `lourd` du poste, la porte puis le partagé, au plus
  `POSTE_PUSH_LOURD_MAX` secondes (60). S'il ne l'obtient pas, il dit « poste occupé par
  <commande> (pid …) » et rend **code 75, rien de pris** : ni le poste ni le push. La session se
  remet en file plus tard.
- Puis le verrou de push, puis la commande, dans le plafond de mémoire et la priorité basse du
  lourd.
- La commande n'appelle plus `poste lourd` : dedans (`POSTE_DANS_LOURD=1`), `poste lourd` passe
  la main tel quel, et `poste mesure` est refusé (code 2), car il attendrait sa propre place.
- **Jamais l'inverse.** Une mesure exclusive se lance avec `poste mesure`, qui attend les
  partagés en cours comme avant. Elle ne connaît pas le push, et n'a pas à le connaître.
- Sur luciepc, `--lourd` prend la place du poste **local**, puis le verrou de push de
  lucie-tablette par le relais.

Témoins, sur des verrous d'essai (`POSTE_LOCK_DIR` pour la porte et le poste, `POSTE_PUSH_DIR`
pour le push, `POSTE_PUSH_LOURD_MAX=5`) :

| cas | ce qui se passe |
|---|---|
| une mesure tient le poste, un push lourd demande | refus en 5 s, code 75, le tenant nommé (`sleep 15`), le verrou de push libre, aucune fiche |
| un push lourd tient le poste, une mesure demande | la mesure commence à la seconde où le push finit |
| dedans le push lourd | `poste lourd true` passe en 0 s ; `poste mesure` refusé, code 2 |
| poste libre | le push lourd passe tout de suite, code 0 |
| sans `--lourd` | rien ne change |
| luciepc, par le relais | « dedans sur LuciePC : POSTE_DANS_LOURD=1 », code 0 |

Déployé sur les deux postes, md5 `70fe3f1f…`. La version précédente est gardée en
`poste.avant-lourd`.

## Un seul verrou pour les deux postes

Le verrou vit **sur lucie-tablette**. luciepc a un fichier
`~/.cache/rag3weaver-build/poste-push-hote` (`lucied@lucie-tablette`) : son
`poste push` ne tient jamais de verrou à lui, il prend celui de
lucie-tablette par ssh. La session ssh demande le verrou là-bas et le garde
tant que son entrée reste ouverte ; la commande tourne sur luciepc ; l'entrée
se ferme à sa fin, et le verrou tombe. Le tenant s'y voit comme
`<session>@LuciePC`.

- **ssh muet ou refusé** : refus nommé, code 3, **rien n'est exécuté** —
  jamais un push sans verrou.
- **ssh perdu pendant la commande** : « VERROU PERDU » dit haut et fort, la
  commande n'est pas tuée (elle peut être à mi-push), et un succès est rendu
  comme 70 pour qu'on vérifie master avant de le croire.

## Les docs seules, sans verrou

**Un push de docs seules (`.md`) est dispensé du verrou.** Il ne rejoue rien
et ne change aucun artefact : il rebase et pousse quand il veut. C'est au
tenant du verrou de rattraper ces pushes, par la boucle de la commande type
ci-dessous.

## La commande type d'une session

La passe complète se fait **avant**, hors verrou, dans le worktree du lot et
son sous-target. Sous le verrou, la commande ne fait que le rejeu court,
**dans ce même worktree et ce même sous-target, déjà chauds**. Puis elle
pousse **en boucle** : quand master a reçu une avance qui ne touche pas les
chemins du bâti de son artefact (des `.md`, poussés sans verrou), elle
rebase et repousse dans le même tour, sans rendre le verrou ni se remettre en
file. Elle ne s'arrête que sur une avance de code, selon la liste des chemins
du bâti de sa sorte de lot (section « Ce qu'une avance de master change »
plus bas) :

```bash
#!/bin/bash
set -u
W=<worktree du lot>            # celui de la passe complète, jamais un neuf
export CARGO_TARGET_DIR=<le sous-target du lot, déjà chaud>
# Les chemins du bâti de l'artefact rejoué, selon la sorte du lot (plus bas) ;
# ici, un lot rag3weaver.
BATI='extension/rag3weaver tools/rust_api src cmake CMakeLists.txt test extension/vector extension/geo third_party scripts'
# Les worktrees partagent leurs références : le fetch d'une autre session avance
# origin/master sous nos pieds. On ne lit donc master qu'après NOTRE fetch, par
# FETCH_HEAD (propre au worktree), dans $cible, et tout se fait contre ce hash.
lire() { git -C "$W" fetch -q origin master && cible=$(git -C "$W" rev-parse FETCH_HEAD); }
rebaser() {
  git -C "$W" rebase -q "$cible"
  gd=$(git -C "$W" rev-parse --absolute-git-dir)
  if [ -d "$gd/rebase-merge" ] || [ -d "$gd/rebase-apply" ]; then
    echo "rebase arrêté sur un conflit, rien poussé"; git -C "$W" rebase --abort; exit 8
  fi
}
lire
rebaser
base=$cible
cd "$W/extension/rag3weaver"
~/.cache/rag3weaver-build/poste lourd bash scripts/check_features.sh || exit 4
# … les suites du lot, courtes …
# Pousser en boucle, verrou tenu : une avance hors du bâti se rebase et se
# repousse dans le même tour ; une avance du bâti arrête tout.
while :; do
  lire
  if [ "$cible" != "$base" ]; then
    if [ -n "$(git -C "$W" diff --name-only "$base" "$cible" -- $BATI ':!*.md')" ]; then
      echo "master a reçu du code du bâti depuis la base rejouée, rien poussé"; exit 9
    fi
    rebaser
    base=$cible
  fi
  git -C "$W" push -q origin HEAD:master && break
  # Refusé : soit un push de docs est passé entre le fetch et le push (on
  # relit et on reboucle), soit le refus vient d'ailleurs (on s'arrête).
  lire
  if [ "$cible" = "$base" ]; then
    echo "push refusé sans avance de master, rien poussé"; exit 10
  fi
done
```

`check_features.sh` vérifie **cinq jeux** de features depuis `eb389901c` : le natif seul,
`rag3db-native`, s'est ajouté aux quatre autres. Depuis `b92d809ae`, il appelle aussi
`scripts/check_tickets.py`, qui recompte l'index des tickets. Le pathspec `':!*.md'` du
contrôle est nécessaire : `BATI` contient `extension/rag3weaver`, et sans lui, un rapport
`.md` poussé là arrêterait la boucle comme du code. Vu le 11 octobre au premier essai de G.

**Le seul conflit qu'une commande résout seule : l'index des tickets.** Chaque lot qui ouvre ou
ferme un ticket touche `docs/tickets/README.md`, au même endroit que les autres, et le rebase y
bute à presque chaque tour. La commande garde le côté master, retire toute ligne de son ticket,
pose sa ligne (ouverte ou fermée) une seule fois, et **recompte chaque section (moteur, produit,
fermés) sur ses lignes réelles, jamais par ±1**. Puis `scripts/check_tickets.py` doit être vert.
Sur tout autre fichier en conflit, ou si elle ne sait pas quelle ligne poser, elle s'arrête
(code 8), rien poussé. Deux exemples :

- `~/.cache/rag3weaver-build/embarquements/pousser.sh` (F) : `FERMER_TICKET` (le fichier) et
  `FERMER_LIGNE` (la ligne fermée complète), résolus par `readme-fermer.py`. Sans les deux
  variables, il s'arrête. Il rebase sans rejouer sur une avance de `.md` seuls, et dit
  `MOTEUR_ANCIEN=1` dans sa sortie quand on l'emploie.
- `~/.cache/rag3weaver-notes-A/pousser2.sh` (arbre principal) : l'index résolu seul, tout autre
  conflit arrête.

La commande de G du 11 octobre faisait ±1, puis `check_tickets.py` contrôlait le résultat. Ce
contrôle seul est juste, mais la forme à suivre est celle des deux exemples.

**Lire master par `FETCH_HEAD`, jamais par `origin/master` après coup.** Les worktrees d'un
même dépôt partagent `refs/remotes/origin/master`. Le 11 octobre, pendant le rejeu court de G,
le fetch d'une autre session l'a avancé. La commande a pris ce hash pour base alors qu'elle avait
rebasé sur l'ancien, le push a été refusé en non-avance rapide, et la boucle s'est arrêtée en
code 10, rien poussé. `FETCH_HEAD` est propre au worktree, et personne d'autre ne l'écrit.

La boucle n'a pas de plafond de tours : chaque tour demande un push de docs
arrivé pendant le précédent, qui ne dure que le temps d'un fetch et d'un
rebase. Un refus qui ne vient pas d'une avance de master l'arrête (code 10).

puis, hors du verrou, les trois contrôles avant d'écrire « poussé » (status
propre, aucun rebase en cours, `git log -1 origin/master` au hash annoncé).

## Les quatre règles à diffuser

1. **La commande se protège elle-même.** Elle s'arrête sur un rebase resté en
   cours, et vérifie que master n'a pas bougé depuis son rebase. Le verrou ne
   corrige pas une commande fautive : le premier jouet du témoin ne faisait
   ni l'un ni l'autre, et il a poussé un rebase arrêté sur un conflit — la
   faute du matin, reproduite sous le verrou.
2. **Un essai ne prend jamais le verrou réel.** `POSTE_PUSH_DIR=<dossier>`
   donne un verrou à part (transmis par le relais). Un premier essai, sans
   cela, s'est mis dans la file réelle derrière une session qui poussait ;
   arrêté à temps, il n'a rien pris.
3. **Une avance de master faite de `.md` seuls ne demande pas de rejeu.** Le
   contrôle « master n'a pas bougé depuis le rebase » l'ignore : on rebase et
   on pousse sans rejouer, en boucle sous le verrou (la commande type). Les docs de l'orchestration et des sessions
   arrivent bien plus souvent que le code ; sans cela, une session rejoue pour
   un rapport et perd sa course.
4. **Sous le verrou, on rejoue dans le worktree et le sous-target déjà chauds
   de la passe complète, jamais dans un worktree neuf.** Le premier
   embouteillage (11 octobre) : une session a tenu le verrou 11 minutes à
   compiler un worktree neuf, puis s'est arrêtée sans pousser — et tout le
   monde attendait derrière elle.

## Ce qu'une avance de master change pour chaque sorte de lot

La règle 3 dite exactement (orchestration, 11 octobre, 16 h) : **une avance de master qui ne
change pas l'artefact rejoué ne demande pas de rejeu**. Ce qui le prouve, c'est la liste des
chemins qui entrent dans le bâti de cet artefact, pas l'extension des fichiers.

- **Un lot du moteur** (l'artefact : les binaires de test C++ et les extensions) rejoue si
  l'avance touche
  `src cmake CMakeLists.txt test extension/vector extension/geo third_party scripts tools/rust_api`.
  Sinon (des docs, `extension/rag3weaver`, codeparsers), il rebase et pousse sans rejouer. Le
  contrôle dans la commande :

  ```bash
  ENGINE='src cmake CMakeLists.txt test extension/vector extension/geo third_party scripts tools/rust_api'
  if [ -n "$(git -C "$W" diff --name-only "$BASE" origin/master -- $ENGINE)" ]; then
    echo "master a reçu du code du moteur depuis la base rejouée, rien poussé"; exit 9
  fi
  ```
- **Un lot rag3weaver** (l'artefact : les binaires cargo, liés à la lib du moteur) rejoue si
  l'avance touche `extension/rag3weaver` (Cargo.lock compris), codeparsers, l'IR, **ou tout chemin
  du moteur ci-dessus**, parce que sa lib change le résultat. Il n'y a pas de symétrique : une
  avance du moteur n'est jamais neutre pour un lot rag3weaver.
- **Une avance de `.md` seuls** est neutre pour les deux.

Vu sur A8 (seconde session cœur C++) : au premier tour, la commande a refusé de pousser parce que
master avait reçu du code rag3weaver depuis sa base. C'était juste selon la forme « `.md` seuls »,
mais sans objet pour un lot du moteur.

## Les témoins

Sur un dépôt jouet (dépôt nu, deux clones ; commande : fetch, rebase, rejeu de
35 à 45 s, push si master n'a pas bougé), verrou d'essai :

| cas | ce qui se passe |
|---|---|
| sans verrou | B perd la course |
| A tient, B demande (même poste) | B le dit à 30 s, prend le verrou à la libération, pousse |
| B tient, A demande | l'inverse ; les pushes passent dans les deux ordres |
| tenant ici, demandeur sur luciepc | LA@LuciePC le dit à 30 s, tourne sur LuciePC à la libération, code 0 |
| tenant sur luciepc par le relais, demandeur ici | ICI le dit à 30 s, tourne à la libération |
| hôte injoignable | refus nommé, code 3, la commande ne tourne pas |
| plafond (3 s pour l'essai) | dit, pas tué, code 0 rendu |

## Déployer `poste`

Copie dans `poste.nouveau`, `sh -n`, **`chmod +x`**, puis `mv` sur `poste` :
le `mv` laisse les instances en cours sur leur ancien fichier, et `scp` ne
garde pas le bit d'exécution (il a manqué quelques secondes sur luciepc au
premier déploiement). Versions précédentes gardées à côté :
`poste.avant-11oct`, `poste.avant-push`, `poste.avant-relais`.
