# Pousser un lot sur master

Ce qui est en vigueur le 25 octobre 2026, rassemblé par c7 depuis le fil et les mémoires des
sessions. Court, parce que chaque ligne a coûté un rouge ou un push perdu.

## Avant la passe

- **Un worktree par lot**, jamais l'arbre de Lucie (`~/git_workspaces/rag3db`) : on n'y commite
  pas, on n'y bascule pas, on n'y laisse rien.
- **Identité gmail** (`user.email = luciedefraiteur@gmail.com`), vérifiée avant chaque commit ; pas
  de ligne d'attribution dans les messages.
- **Chaque commit poussé sur sa branche dans la minute** (`origin/<nom-du-lot>`), rien ne reste en
  local, et aucun travail ne reste sur luciepc.
- **Après un rebase ou un amend, un nom de branche neuf** (`-2`, `-3`) : jamais de force, jamais
  d'amend d'un commit déjà poussé.

## La passe (à luciepc, sous `poste lourd`)

- **`RAG3DB_LIB` épinglée** par son identifiant complet au début de la passe
  (`rag3db-build-<hash>-Release-…`), jamais `courant` : une bascule le repointe pendant la passe.
  Quand la lib est plus vieille que le C++ de master, les e2e portent `RAG3WEAVER_MOTEUR_ANCIEN=1`.
- **`scripts/check_features.sh`** (cargo check --tests sous les jeux de features), puis **la lib
  entière** sous le jeu le plus large :
  `rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres,openai-llm` (`RAG3WEAVER_PG` posé).
- **L'étape navigateur** à part, après la lib : `FEATURES=… scripts/temoins_navigateur.sh`
  (`RAG3WEAVER_EXIGER_NAVIGATEUR=1`). Depuis l'IDE en React (25 octobre 2026), elle **exige les
  paquets React** de l'aperçu à chaud : une fois par poste, `scripts/paquets_react.sh <dossier>`
  (react, react-dom, scheduler, react-refresh épinglés, déballés par `npm pack`), puis
  `export RAG3WEAVER_PAQUETS_REACT=<dossier>` au début de la passe (à luciepc :
  `/home/lucied/.cache/rag3weaver-build/I-paquets-react`). Sans la variable ou sans le dossier,
  les témoins de l'aperçu rougissent en le disant : « RAG3WEAVER_PAQUETS_REACT ne désigne pas les
  paquets React : préparer par scripts/paquets_react.sh <dossier> ». Chrome ou Chromium sur le
  poste, et `RAG3WEAVER_PAQUETS_REACT` posé, c'est tout ce que l'étape demande.
- **Les e2e du changement** (une suite par appel de `run_e2e.sh`, lire « running N tests ») :
  `e2e_postgres` si `code.rs`, l'IR ou un gabarit sont touchés ; `e2e_lectures` si le bus,
  `Backend::run` ou l'hôte ; les `e2e_tx_*` si le catalogue ; `e2e_juge` et `e2e_equipier` (Gemini
  exigé, `RAG3WEAVER_EXIGER_GEMINI=1`, identifiants par le chemin de `.vault`, jamais leur contenu)
  si le juge ou l'équipier.
- Un témoin qui se saute n'est pas vert : `SAUTÉ` compte comme un rouge dans la passe.

## Le push (sous `poste push`, dans le worktree de push)

- **Le rebase sur master au moment du push**, puis **le diff du rebase lu** :
  `git diff origin/master...HEAD --stat` (trois points), et ce que master a reçu depuis la base :
  si du code est arrivé sous `extension/rag3weaver` (hors `.md`, hors `src/` et `test/` du moteur,
  qui ne sont pas liés par la lib épinglée), **la passe se rejoue** sur le nouveau master ; des docs
  ou du C++ du moteur seuls : rebase sans rejeu.
- **Le garde du rebase en conflit**, juste après le rebase et avant toute lecture de HEAD :

  ```bash
  git rebase -q "$M" || true
  d=$(git rev-parse --git-dir)
  if [ -d "$d/rebase-merge" ] || [ -d "$d/rebase-apply" ]; then
    echo "ARRÊT : rebase en conflit"; git status --short | grep '^UU'; git rebase --abort; exit 1
  fi
  ```

  Un `[ ! -d … ] && echo …` n'est pas un garde ; pendant un rebase arrêté, HEAD est la tête de
  master, le contrôle d'ancêtre passe, et `git rev-parse HEAD` rend un hash qui n'est pas le lot.
- **Le push de master dans `if grep -qx VERT`**, jamais enchaîné après la passe par `;` ou `&&`.
- **Les trois contrôles lus avant de dire « poussé »** : `git status --short` propre, aucun
  rebase en cours (`rebase-merge`, `rebase-apply`), et `git log -1 origin/master` après un `fetch`
  égal à HEAD.
- Deux passes qui écrivent le même journal, c'est deux passes : une passe arrêtée se relit
  (`pgrep` de ses propres pids, jamais par nom) avant d'en relancer une dans le même worktree.
- Le hash va à c7 ; les docs seules se poussent sans verrou ni passe (voir
  `extension/rag3weaver/docs/optimiseur/11-octobre-2026-15h27/01-poste-push.md`).

## Après

- Les worktrees et les targets du lot s'effacent une fois le lot sur master (ceux de luciepc à la
  fin de la session).
- Les rapports de session (`01-rapport`, `02-knowledge-dump` du jour) se tiennent à jour après
  chaque lot ; un défaut connu non corrigé devient un ticket dans `docs/tickets/`.
