# Une tâche ne s'ouvre pas quand le manifeste est hors dépôt

- **État** : corrigé (7 novembre 2026, branche tache-hors-depot), relevé par c7 en lançant la
  première tâche de l'équipier sur l'hôte de Lucie.
- **Famille** : produit
- **Gravité** : bloquant pour l'équipier. Sur toute configuration réelle où le manifeste vit dans
  un dossier de configuration, aucune tâche ne s'ouvre.
- **Atteignable en service** : oui. `POST /ide/teammate/mailbox` (kind request) sur le backend
  12389e156742 de Lucie : « le backend n'est pas dans un dépôt git (git rev-parse
  --show-toplevel …) : une tâche est une branche ».
- **Touche rag3weaver** : oui (`src/daemon_host/taches.rs`, `ouvrir_une_tache`).

## Ce que c'était

`ouvrir_une_tache` cherchait le dépôt de la tâche dans le dossier du manifeste. Celui de Lucie
(`~/.config/rag3weaver/code-rag3db/templates/backends/code/rag3db.json`) n'est suivi par aucun
git. Sa `workspace.root` (`/home/lucied/git_workspaces/rag3db`) est pourtant un dépôt. C'est le
même défaut que celui corrigé pour `kind`, `repo` et `branch` dans `/ide/etat`.

## Le correctif

Le dépôt de la tâche est celui des déclarations, ou à défaut celui de `workspace.root`. Dans le
second cas, le worktree est une branche `agent/<nom>` du dépôt du code, et la racine de la tâche
est dans ce worktree. Les déclarations, qu'aucun git ne versionne, sont copiées dans le dossier
de la tâche (`declarations/`), sans les bases ni les dossiers cachés. La base et la racine de code
de la tâche restent données par ses surcharges. Le cas d'un manifeste dans un dépôt et d'un code
dans un autre (le second worktree `code`) n'a pas changé.

Témoin : `daemon_host::taches::tests::une_tache_s_ouvre_quand_seul_le_code_est_dans_un_depot`,
rouge sur b1efe6406 (le refus de l'hôte de Lucie, mot pour mot), vert avec le correctif.

La suite, une tâche sans aucun git, a son propre ticket :
`2026-11-07-une-tache-ne-s-ouvre-pas-sans-git`.
