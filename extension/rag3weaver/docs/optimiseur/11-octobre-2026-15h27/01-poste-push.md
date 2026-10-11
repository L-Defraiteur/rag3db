# `poste push` : un seul tenant pour pousser sur master

**11 octobre 2026, session optimiseur, à la demande de l'orchestration.** Ce
jour-là, trois sessions ont perdu trois courses chacune : du code arrivait sur
master pendant leur rejeu, et l'orchestration fermait des fenêtres à la main.
`poste push` remplace ces fenêtres : un verrou, un tenant à la fois, les
autres attendent en le disant.

## La forme

```bash
~/.cache/rag3weaver-build/poste push <session> -- <commande…>
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

## La commande type d'une session

La passe complète se fait **avant**, hors verrou, dans le worktree du lot et
son sous-target. Sous le verrou, la commande ne fait que le rejeu court,
**dans ce même worktree et ce même sous-target, déjà chauds** :

```bash
#!/bin/bash
set -u
W=<worktree du lot>            # celui de la passe complète, jamais un neuf
export CARGO_TARGET_DIR=<le sous-target du lot, déjà chaud>
rebaser() {
  git -C "$W" fetch -q origin master
  git -C "$W" rebase -q origin/master
  gd=$(git -C "$W" rev-parse --absolute-git-dir)
  if [ -d "$gd/rebase-merge" ] || [ -d "$gd/rebase-apply" ]; then
    echo "rebase arrêté sur un conflit, rien poussé"; git -C "$W" rebase --abort; exit 8
  fi
}
rebaser
base=$(git -C "$W" rev-parse origin/master)
cd "$W/extension/rag3weaver"
~/.cache/rag3weaver-build/poste lourd bash scripts/check_features.sh || exit 4
# … les suites du lot, courtes …
git -C "$W" fetch -q origin master
if [ "$(git -C "$W" rev-parse origin/master)" != "$base" ]; then
  # Une avance faite de .md seuls ne demande pas de rejeu : on rebase et on pousse.
  if git -C "$W" diff --name-only "$base" origin/master | grep -qv '\.md$'; then
    echo "master a bougé (pas que des .md), rien poussé"; exit 9
  fi
  rebaser
fi
git -C "$W" push -q origin HEAD:master
```

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
   on pousse sans rejouer. Les docs de l'orchestration et des sessions
   arrivent bien plus souvent que le code ; sans cela, une session rejoue pour
   un rapport et perd sa course.
4. **Sous le verrou, on rejoue dans le worktree et le sous-target déjà chauds
   de la passe complète, jamais dans un worktree neuf.** Le premier
   embouteillage (11 octobre) : une session a tenu le verrou 11 minutes à
   compiler un worktree neuf, puis s'est arrêtée sans pousser — et tout le
   monde attendait derrière elle.

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
