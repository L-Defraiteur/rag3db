# Un agent en place commite sur une branche qui suit un amont

- **État** : ouvert, le 23 novembre 2026
- **Famille** : produit
- **Gravité** : moyenne : dans un arbre qui n'avance que par `merge --ff-only` (l'arbre principal de rag3db), un tour
  qui change un fichier commite sur la branche locale, qui diverge alors de son amont ; rien ne prévient avant
- **Atteignable en service** : oui (une tâche en place, le mode par défaut, sur un dépôt dont la branche courante suit
  un amont)
- **Touche rag3weaver** : oui (`src/daemon_host/land.rs`, `land_in_place` ; `src/daemon_host/taches.rs`, le choix du mode)
- **Pour** : archi
- **Signalé par** : c7, avant l'essai de Lucie dans son arbre principal, le 23 novembre 2026 au soir

## Ce qui se passe

Une tâche ouverte dans le dossier de l'utilisatrice travaille en place (le défaut depuis le 23 novembre). À la fin d'un
tour qui a changé des fichiers, l'hôte commite ces fichiers sur sa branche courante (`land_in_place`). Sur un `master`
qui suit `origin/master`, ce commit fait diverger la branche locale ; l'arbre de Lucie n'avance que par
`merge --ff-only`, et aucune session n'y commite (règle du dépôt).

Le mode copie ne l'évite pas : la tâche y crée un worktree et une branche `agent/<nom>` dans le dépôt, et la fusion de
fin de tour atterrit aussi dans son arbre.

Un tour sans fichier changé ne commite rien et ne crée aucune branche dans son arbre ; mais une commande qui laisse un
fichier non ignoré suffit à faire un commit.

## Le chemin de l'ancien atelier : le même risque

Quand le dépôt du backend est le dépôt de l'utilisatrice (le backend `rag3db.json` de son arbre principal, la forme de
l'ancien atelier), le mode en place ne s'applique pas : il ne joue que pour un dépôt de code distinct de celui du
backend. La tâche ouvre alors un worktree et une branche `agent/<nom>` **dans son dépôt** (`git worktree list` les
montre), et la fin de tour commite le travail sur cette branche puis tente `merge --ff-only` dans son dossier, ce qui
avance son `master`.

Le soir du 23 novembre (tâche `12389e156742/23-11-20h48`, essai de Lucie depuis le terminal dans
`~/git_workspaces/rag3db`), **seules deux gardes fortuites ont tenu** : son `master` avait avancé depuis le départ de la
tâche (`git pull` entre-temps), et son dossier portait une modification suivie (le sous-module
`extension/rag3weaver/codeparsers`). L'une ou l'autre retient la fusion (`merge_held`). Sans elles, le fichier écrit
par l'agent serait arrivé sur son `master` par avance rapide.

## Ce qu'on attend

Un garde qui lit l'effet, pas une liste de dépôts : une tâche en place sur une branche qui suit un amont, dans un dépôt
déclaré protégé, ou dont la politique le demande, n'atterrit pas par commit. Elle laisse ses changements non commités,
et le dit en une ligne avec la commande pour les reprendre, ou elle propose une branche à elle avant le premier
atterrissage. Le choix et sa déclaration (une clé de `teammate` dans le manifeste) sont à concevoir avec Lucie.

## En attendant

Dans l'arbre principal : des questions seulement. Pour écrire : un worktree du dépôt, où un commit va sur la branche
du worktree (le lot `worktree-reuses-repo-index` y donne la recherche par l'index du dépôt).
