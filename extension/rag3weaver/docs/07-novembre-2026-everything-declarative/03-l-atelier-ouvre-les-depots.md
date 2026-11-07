# L'atelier ouvre les dépôts

7 novembre 2026, everything declarative ; page de conception, à c7 pour Lucie avant tout code.
La première marche concrète de la page 02. Ce que Lucie décrit : « j'arrive sur une session, je
lui dis "j'aimerais que tu travailles sur tel nouveau projet de code", hop elle ouvre un backend,
crée un git si elle a ssh ; ou "j'aimerais créer tel nouveau projet rag3weaver", peut-être même
l'agent est incité à demander quel genre de projet on veut, et pour l'instant il ne peut faire que
les projets de code ; ou "ouvre telle branche sur tel repo git sur lequel j'ai accès", ou "fetch
et pull telle autre" ; tout se fait pour que les backends se créent à la volée, par repo » ; et
« rien n'empêche un agent d'ouvrir trois worktrees différents dans le même repo et de les tenir un
moment ».

## 1. Une conversation d'atelier, non attachée, avec des outils de dépôts

- **L'atelier** : une conversation qui ne tient encore aucun dépôt (la page d'accueil de l'IDE, le
  terminal de tty). Ses outils sont **déclarés** dans un manifeste d'atelier (gabarit livré
  `builtin:backends/atelier`), pas codés dans l'hôte.
- **Les outils** (noms en anglais, paramètres en vocabulaire git) :
  - `attach_repo {path | url, branch?}` : attacher un dépôt (un chemin local, ou cloner une URL) ;
  - `open_branch {repo, branch, from?}` : un worktree frère sur cette branche (créée depuis `from`
    si elle n'existe pas) ;
  - `fetch {repo}`, `pull {repo, worktree}` ;
  - `new_code_project {name, path?, remote?}` : `git init`, un premier commit, le remote s'il est
    dit ;
  - `new_rag3weaver_project {name, kind}` : **pas encore** — il rend « seuls les projets de code
    se créent pour l'instant » ; la consigne de l'atelier dit de demander le genre de projet quand
    la demande est ambiguë, avant d'appeler l'un ou l'autre.
  - `list_repos` : les dépôts attachés, leurs worktrees, leurs branches, l'état de leur index.
- **Plusieurs worktrees par dépôt** : une session tient N worktrees, plusieurs dans le même dépôt
  s'il le faut (la page 02 disait « une tâche par dépôt » : corrigé, c'est un lot de worktrees).

## 2. Un worktree attaché devient un backend de code, à la volée

- Pour chaque worktree attaché, l'hôte **instancie le manifeste de code** depuis le gabarit, comme
  `code-tree add` le fait déjà (`code_tree::add_files` : le gabarit copié, la racine posée,
  `task_writes`, la porte des commandes), avec le modèle et la politique de la session, puis
  l'ouvre par `POST /backend/open`.
- **L'indexation est asynchrone** : le premier index part en fond, sa progression s'affiche sous le
  dépôt dans la barre « Dépôts » (la ligne d'état de l'index, déjà rendue par la recherche) ;
  l'étanchéité (page 01, lot 1) fait travailler l'agent sur les fichiers pendant ce temps.
- **Ce que la barre montre** : attacher un dépôt fait apparaître **un enfant sous la session**,
  « chargé », son rag « en cours » avec sa progression, puis « indexé » ; détacher le renvoie en
  « disponibles » (page 02, § vocabulaire et la forme de `/ide/etat`).
- **Où vit la conversation** : sous le dépôt ou le worktree qu'elle tient ; une conversation
  d'atelier qui ouvre un worktree s'y rattache (elle garde son fil).
- Un worktree d'un dépôt déjà indexé peut naître d'un **instantané** de la base du dépôt (le
  reflink de `code-tree add --from`) : quelques secondes au lieu d'un premier index.

## 3. La sécurité

- **Git passe par la porte déclarée** (`workspace.commands`, la `Garde`, argv sans shell), avec les
  commandes git de l'atelier déclarées au manifeste d'atelier, jamais en dur.
- **Les clés ssh** sont celles de la session de l'utilisatrice (son agent ssh) : jamais lues,
  copiées ni stockées par rag3weaver.
- **Cloner, pousser, créer un remote** : demandés à la première occurrence pour un dépôt (une
  question dans la boîte, `kind: "approval"`), la réponse notée pour ce dépôt et cette session.
- **Rien vers un amont** (push, PR) sans le mot de Lucie.

## 4. Le disque

En attendant les vues par worktree (vision du 6 novembre), **un worktree = une base**, copiée par
reflink quand c'est possible, et **jetable** : fermer ou fusionner un worktree efface sa base, son
worktree et ses cibles (la règle des tâches jetables). La barre « Dépôts » dit la taille tenue par
chaque worktree.

## 5. Les témoins (par le chemin de l'hôte)

- « Ouvre la branche X du dépôt Y » : `attach_repo` + `open_branch` → le worktree existe sur la
  branche, le manifeste est posé, le backend ouvert, l'index lancé (sa progression lisible), la
  conversation rangée sous le dépôt.
- « Travaille sur un nouveau projet de code Z » : `new_code_project` → `git init` et un premier
  commit, le manifeste, l'index ; la conversation sous Z.
- « Crée un projet rag3weaver » : le refus dit, et l'agent a demandé le genre avant.
- Un `clone` sans accord : la question dans la boîte, rien n'est cloné avant la réponse.

## Les lots

1. Le manifeste d'atelier et ses outils de dépôts (`attach_repo`, `open_branch`, `list_repos`,
   `fetch`, `pull`), l'instanciation et l'ouverture à la volée, la barre « Dépôts » (avec
   hooks-essai pour la barre).
2. `new_code_project` et les accords (clone, remote, push).
3. La conversation qui suit le worktree qu'elle tient, plusieurs worktrees par session.
