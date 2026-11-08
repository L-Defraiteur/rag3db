# Pousser un lot sur master

Ce qui est en vigueur le 25 octobre 2026, rassemblé par c7 depuis le fil et les mémoires des
sessions. Court, parce que chaque ligne a coûté un rouge ou un push perdu.

## Avant la passe

- **Un worktree par lot**, jamais l'arbre de Lucie (`~/git_workspaces/rag3db`) : on n'y commite
  pas, on n'y bascule pas, on n'y laisse rien. **Un seul target par session** (Lucie, 7 novembre
  2026) : le même dossier sous `~/.cache/rag3weaver-build/` d'un lot au suivant, effacé avec le
  lot ; pas un target par essai.
  **Son plafond est de 32 Go** (`run_e2e.sh`, `RAG3WEAVER_TARGET_MAX_GO`, 7 novembre 2026) : une
  passe complète sous le jeu large (check_features, la lib entière, l'étape navigateur, les e2e)
  pèse 19 à 26 Go, une rlib et un binaire de tests par jeu de features. Un target sous 32 Go
  n'est pas une accumulation : il ne s'efface pas en cours de lot.
- **La lib entière se joue avec ou sans `RAG3WEAVER_EMBED_SERVICE`** (depuis le lot du délai,
  « fix(délai): un délai partout », 8 novembre 2026) : un service d'embarquement qui ne rend pas
  fait rougir en N s (« service d'embarquement <adresse> muet après N s »), il ne fige plus. Du 7
  au 8 novembre, la lib entière se jouait **sans** service (règle de c7) : derrière un bge-m3 en
  file, le relecteur et l'OCR attendaient sans délai, et l'arrêt borné des réactions comptait son
  délai sur les deux fils du runtime du crate que ces attentes tenaient — la lib entière se
  figeait (ticket `docs/tickets/closed/2026-11-07-la-lib-entiere-se-fige-ou-rougit-par-intermittence.md`).
- **Un worktree est jetable** (vision du 6 novembre, « Un worktree est jetable, donc
  distribuable » ; Lucie, 7 novembre : 46 worktrees à luciepc ce matin, 32 déjà fusionnés). Un
  contrôle **en début de passe** refuse un worktree dont la branche est déjà atterrie
  (`git cherry origin/master HEAD` sans ligne `+`) ou dont la base a plus de **200** commits de
  retard sur master (`git rev-list --count HEAD..origin/master`, seuil à régler) : on l'efface et
  on en refait un neuf depuis master, on ne le rattrape pas.

  ```bash
  git fetch -q origin master
  [ -n "$(git cherry origin/master HEAD | grep '^+')" ] || { echo "ARRÊT : rien à pousser, branche déjà atterrie — effacer ce worktree"; exit 1; }
  r=$(git rev-list --count HEAD..origin/master)
  [ "$r" -le "${RETARD_MAX:-200}" ] || { echo "ARRÊT : $r commits de retard sur master — effacer ce worktree et repartir de master"; exit 1; }
  ```
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
  `rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres,openai-llm,anthropic-llm` (`RAG3WEAVER_PG` posé).
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
- **Les tickets du lot se relisent** avant le push : `python3 extension/rag3weaver/scripts/check_tickets.py --lot origin/master` imprime ceux que le lot ouvre, ferme ou change d'état (l'index ne se compte plus à la main depuis le 7 novembre 2026 ; il se génère par `--index`).
- **Un contrôle qui ne trouve rien à juger est ROUGE**, jamais vert par défaut : un filtre ctest qui ne trouve aucun test (« No tests were found », code 0) a fait dire VERTE une passe dont la comparaison du banc n'avait pas tourné (hotfixs, 7 novembre 2026). Le verdict se lit sur la sortie qui juge (« bench matches », « running N tests »), pas sur un code de sortie seul.
- **Les e2e d'une passe se jouent sous le jeu large de features**
  (`rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres,openai-llm,anthropic-llm`) ; une
  suite **non jouée faute de feature est ROUGE** et nomme la suite, et le verdict dit « N suites
  jouées sur N attendues » (96 et c7, 7 novembre 2026 : `run_e2e.sh` sautait les suites sous
  `openai-llm` en prévenant seulement, et un témoin du pas 1 de M4 est arrivé sur master sans
  avoir jamais été joué). Lire « running N tests » de chaque suite du changement avant de dire
  vert.

## Le push (sous `poste push`, dans le worktree de push)

- **Le verrou `poste push` ne couvre que le dernier pas** : fetch, rebase, push et les trois contrôles,
  une trentaine de secondes. La passe, et tout rejeu qu'un rebase imposerait, se jouent **hors verrou**,
  avant de le prendre. Si le rebase sous le verrou apporte du code, le script **s'arrête et rend le verrou**
  au lieu de rejouer dessous : on rejoue dehors, puis on reprend le verrou pour pousser. Un verrou tenu au-delà
  de 1 200 s se lit dans `poste --qui` ; on prévient la session, on ne touche jamais à son pid
  (7 novembre 2026 : une passe complète jouée sous le verrou a tenu 44 minutes, deux lots verts derrière).
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
- **Master se relit juste avant le push**, pas seulement au départ de la passe (96, deux courses
  perdues le 7 novembre 2026) : si master a bougé **sans Rust** sous `extension/rag3weaver`, le
  tour repose le lot dessus (rebase) et pousse ; s'il a bougé **avec du Rust**, il repose et
  rejoue court (`check_features`, la lib entière, les e2e du changement), puis relit encore. **Trois
  fois au plus**, puis il demande une fenêtre à c7. C'est la seule façon d'atterrir sans fenêtre
  quand les pushes se suivent à l'heure.
- **Le push de master dans `if grep -qx VERT`**, jamais enchaîné après la passe par `;` ou `&&`.
- **Les trois contrôles lus avant de dire « poussé »** : `git status --short` propre, aucun
  rebase en cours (`rebase-merge`, `rebase-apply`), et `git log -1 origin/master` après un `fetch`
  égal à HEAD.
- Deux passes qui écrivent le même journal, c'est deux passes : une passe arrêtée se relit
  (`pgrep` de ses propres pids, jamais par nom) avant d'en relancer une dans le même worktree.
- Le hash va à c7 ; les docs seules se poussent sans verrou ni passe (voir
  `extension/rag3weaver/docs/optimiseur/11-octobre-2026-15h27/01-poste-push.md`).

## Après

- **Le script de push efface lui-même le worktree, sa branche locale et le target du lot, juste
  après les trois contrôles** — dans le même script, pas « plus tard » (Lucie, 7 novembre 2026) :

  ```bash
  depot=$(git rev-parse --path-format=absolute --git-common-dir); depot=${depot%/.git}
  branche=$(git rev-parse --abbrev-ref HEAD)
  cd "$depot" && git worktree remove --force "$W" && git branch -D "$branche"
  rm -rf "$TARGET"
  echo "EFFACÉ : worktree $W, branche $branche, target $TARGET"
  ```

  La branche distante reste (rien ne s'efface sur origin sans Lucie). Les worktrees de luciepc
  suivent la même règle à la fin de chaque lot, plus seulement à la fin de la session.
- Les rapports de session (`01-rapport`, `02-knowledge-dump` du jour) se tiennent à jour après
  chaque lot ; un défaut connu non corrigé devient un ticket dans `docs/tickets/`.
- **Le README suit ce que l'utilisateur voit** (Lucie, 8 novembre 2026) : un lot dont la ligne de
  `docs/nouveautes.md` change ce que l'utilisateur voit ou fait — un outil, une commande, un
  réglage, un comportement visible — met aussi à jour le README concerné (`README.md` à la racine,
  `extension/rag3weaver/README.md`) dans le même lot. Sinon, un rattrapage des README depuis
  `nouveautes.md` toutes les semaines.
