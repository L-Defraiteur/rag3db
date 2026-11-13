# Pousser un lot sur master

Ce qui est en vigueur le 25 octobre 2026, rassemblé par c7 depuis le fil et les mémoires des
sessions. Court, parce que chaque ligne a coûté un rouge ou un push perdu.

## Avant la passe

- **Tout nouveau travail a un ticket**, y compris une suite de lot, une mesure, une conception
  à compléter ou un signalement à une autre session. Créer ou compléter le ticket avant la
  transmission ; citer son chemin, le porteur attendu, les preuves et les critères de fin.
  Un envoi ne ferme pas le ticket. À la reprise, relier les actions encore sans ticket.
  Convention complète : `docs/tickets/README.md` et `docs/README.md`.
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

**Mémoire (c7, 13 nov. 2026)** : **32 Gio pour toute validation**, l'invariant de `scripts/confiner_validation.py`. `poste` garde son défaut de **40 Gio pour les bâtis**. Donc `run_e2e.sh` se lance seul (il prend `poste` avec 32G), ou bien `POSTE_MEM_MAX=32G poste lourd … run_e2e.sh` : sous un `poste lourd` sans cette variable, la portée existe déjà à 40 Gio et le confineur refuse la passe.

### Portée commune des validations Linux

`run_e2e.sh` et `scripts/temoins_navigateur.sh` entrent désormais par
`scripts/confiner_validation.py`. La primitive prend le vrai `poste` avant de poser
un marqueur, exige un plafond mémoire effectif d’au plus32Gio, pose HIGH=0 et crée
un bus de session privé sans répertoire d’activation. Elle contrôle les cgroups
des descendants et leur arrêt ; elle n’ouvre pas le bus de Lucie et ne change
aucun argument du navigateur ou de sa sandbox. Un marqueur seul, un reçu périmé,
un outil manquant ou une limite absente provoquent un refus explicite.

Pour une passe composée, posséder une seule portée et un seul budget :

```bash
python3 scripts/confiner_validation.py lancer --delai 2400 -- bash /chemin/passe-privee.sh
```

Les entrées e2e/navigateur appelées à l’intérieur vérifient le reçu du superviseur
vivant et héritent de son budget/bus/plafond ; elles ne reprennent pas de verrou ni
ne créent de scope sœur. Avec un poste extérieur, poser POSTE_MEM_MAX=32G **avant**
son acquisition ; un parent à40Gio n’est pas une preuve de32Gio et est refusé.
Le mode est acquis pour toute la portée : une commande imbriquée ne peut pas
transformer un verrou partagé en exclusif. Si la passe composée contient un
`run_e2e.sh --build` ou `--build-only`, choisir `lancer --mode mesure` dès l’extérieur
(et appliquer nice/ionice aux rebâtis ordinaires, sans déclarer une mesure de coût).
Les témoins argv des rebâtis couvrent l’entrée autonome ; ils ne prouvent pas un
upgrade de mode dans une portée déjà active. Une passe e2e sans rebâti utilise lourd.
Hors portée composée, `RAG3WEAVER_VALIDATION_TIMEOUT` fixe le délai global de l’entrée
(18000s e2e,600s navigateur par défaut). Réduire ces valeurs aux bornes du lot.
Le délai commence après acquisition et inclut l’arrêt/nettoyage ; aucun réessai.

Les preuves sont hors target sous `~/.cache/rag3weaver-build/validations/validation-*`
(recette/config, PID/starttime/cgroups/plafonds, code/verdict/quiescence). L’option
`--preuves <parent privé>` choisit un autre parent pour une passe composée. Les
seuls processus attribués sont arrêtés par pidfd ; un résidu après succès rend la
validation rouge. Le temporaire du bus n’est supprimé qu’après quiescence ; en cas
de détenteur subsistant il reste conservé et rouge. Les témoins CPU de la primitive
se jouent par `python3 scripts/test_confiner_validation.py`, sans Cargo/bus réel.


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
  si le juge ou l'équipier. Les trois lignes à poser au début de la passe, et rien d'autre (le crate
  lit lui-même ces deux variables ; le fichier ne se lit, ne s'affiche et ne se copie jamais) :

  ```bash
  export GOOGLE_APPLICATION_CREDENTIALS=$HOME/git_workspaces/rag3db/.vault/vertex-sa.json
  export GOOGLE_CLOUD_PROJECT=lr-hub-472010
  export RAG3WEAVER_EXIGER_GEMINI=1
  ```

  Sans `RAG3WEAVER_EXIGER_GEMINI=1`, les témoins Gemini se sautent (« ignored ») et la suite se dit
  verte à tort : lire « running N tests » et « 0 ignored » sur `e2e_juge` et `e2e_equipier`.
- Un témoin qui se saute n'est pas vert : `SAUTÉ` compte comme un rouge dans la passe.
- **Les bancs** ne sont pas des suites de la passe. Un banc à services (aujourd'hui
  `e2e_banc_fiches_classees`, sa voie : `RAG3WEAVER_BANC_VERTEX` avec la feature `openai-llm`, ou
  `RAG3WEAVER_BANC_DECIDE` et un llama-server) n'entre jamais dans l'e2e entier : `run_e2e.sh`
  l'écarte en le disant, ses variables posées ou non, et la passe se dit « complète hors bancs à
  services ». Il se joue **seul**, quand sa tentative ou son code change :
  `RAG3WEAVER_BANC_VERTEX=1 RAG3WEAVER_LLM=vertex ./run_e2e.sh --test e2e_banc_fiches_classees`,
  les identifiants Vertex par le chemin de `.vault`. Nommé sans sa voie, ou avec une voie dont la
  feature manque au jeu, il est refusé rouge avant tout bâti en disant quoi poser : c'est voulu,
  un banc sauté n'est pas un banc vert. Une mesure de durée (un banc de charge) se joue sous
  `poste mesure`.
- **Les tickets du lot se relisent** avant le push : `python3 extension/rag3weaver/scripts/check_tickets.py --lot origin/master` imprime ceux que le lot ouvre, ferme ou change d'état (l'index ne se compte plus à la main depuis le 7 novembre 2026 ; il se génère par `--index`).
- **Un contrôle qui ne trouve rien à juger est ROUGE**, jamais vert par défaut : un filtre ctest qui ne trouve aucun test (« No tests were found », code 0) a fait dire VERTE une passe dont la comparaison du banc n'avait pas tourné (hotfixs, 7 novembre 2026). Le verdict se lit sur la sortie qui juge (« bench matches », « running N tests »), pas sur un code de sortie seul.
- **Les e2e d'une passe se jouent sous le jeu large de features**
  (`rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres,openai-llm,anthropic-llm`) ; une
  suite **non jouée faute de feature est ROUGE** et nomme la suite, et le verdict dit « N suites
  jouées sur N attendues » (96 et c7, 7 novembre 2026 : `run_e2e.sh` sautait les suites sous
  `openai-llm` en prévenant seulement, et un témoin du pas 1 de M4 est arrivé sur master sans
  avoir jamais été joué). Lire « running N tests » de chaque suite du changement avant de dire
  vert.

- **Une suite rend son TMPDIR vide** (8 novembre 2026, lot residus-de-suite) : `run_e2e.sh` regarde
  le TMPDIR de chaque suite avant de l'effacer. Une suite qui y laisse quelque chose est ROUGE,
  avec la liste de ce qu'elle laisse, sauf si `tests/e2e/suites_qui_laissent_des_residus.txt` la
  nomme. Une entrée `<suite>` seule : elle en laisse à chaque passe, et propre, elle est rouge.
  Une entrée `<suite> <ticket>` : un résidu intermittent dont la cause est en ticket, dit
  « résidu connu : … ». Un lot qui rend une suite propre la retire de la liste dans son commit,
  prouvé par plusieurs passes propres, et le ticket le dit.
- **Un e2e entier arrête tout son arbre à l'échéance**, jamais le seul haut. Le 8 novembre 2026,
  `poste lourd timeout 14000` a expiré : la place s'est rendue, mais `run_e2e.sh` et son binaire,
  orphelins, ont tenu bge-m3 (7879) à 100 % de la carte une heure de plus. `timeout` ne signale
  que son fils. Le lanceur pose `run_e2e.sh` dans sa propre session (`setsid`), arrête son groupe
  entier (`kill -TERM -- -<pid>`, puis KILL) à son échéance ou s'il est lui-même arrêté (un
  `trap`), et le délai de `poste lourd` dépasse le sien. Après toute expiration, chercher les
  restes par l'exécutable du target (`readlink /proc/<pid>/exe`) avant de rendre la carte, et les
  arrêter par l'arbre de leurs pids, jamais par nom. Un e2e entier qui embarque en creux est un
  lourd GPU : il passe quand la carte est à personne. Une mesure qui tient la carte longtemps
  (e2e_charge_ingestion, près de deux heures) s'écarte d'un e2e entier par
  `RAG3WEAVER_E2E_SAUF="<suite> …"` (dite NON JOUÉE) et se joue seule sous `poste mesure`.

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
- **Le verdict de la CI pour un commit** se lit sur le run qui l'a joué, ou sur le premier run
  plus récent qui a fini. Un « cancelled » sans job est un run remplacé par le suivant : GitHub ne
  garde qu'un run en attente par groupe, et sur master le run qui tourne n'est jamais annulé. Ce
  n'est pas un échec. Le bandeau de `poste push` dit le dernier run fini (c7, 8 novembre 2026).
- Le hash va à c7 ; les docs seules se poussent sans verrou ni passe (voir
  `extension/rag3weaver/docs/optimiseur/11-octobre-2026-15h27/01-poste-push.md`).

## Après

- **Le script de push efface lui-même le worktree, sa branche locale et le target du lot, juste
  après les trois contrôles** — dans le même script, pas « plus tard » (Lucie, 7 novembre 2026) :

  ```bash
  depot=$(git rev-parse --path-format=absolute --git-common-dir); depot=${depot%/.git}
  branche=$(git rev-parse --abbrev-ref HEAD)
  # Preuve hors cible et hors administration : copie fidèle préparée séparément.
  # Répéter --archive pour chaque racine administrative qui serait détruite.
  # La garde ne copie rien, ne répare rien et refuse toujours le partage.
  python3 "$W/extension/rag3weaver/scripts/garde_worktree.py" \
    --depot "$depot" --worktree "$W" --rapport "$RAPPORT_NEUF_HORS_CIBLE" \
    --archive "$ADMIN_DETRUITE" "$COPIE_FIDELE_HORS_CIBLE" || exit 1
  cd "$depot" && git worktree remove --force "$W" && git branch -D "$branche"
  rm -rf "$TARGET"
  echo "EFFACÉ : worktree $W, branche $branche, target $TARGET"
  ```

  La branche distante reste (rien ne s'efface sur origin sans Lucie). Les worktrees de luciepc
  suivent la même règle à la fin de chaque lot, plus seulement à la fin de la session.
- **Avant tout retrait de parent**, inventorier les administrations Git qui seraient détruites
  (celle du worktree et les administrations de sous-modules, y compris les résidus). Le lanceur
  préserve séparément leurs copies fidèles hors cible ; `garde_worktree.py` compare types, modes,
  propriétaires, dates de modification et empreintes des fichiers, sans afficher leur contenu.
  `--archive ADMIN COPIE` désigne chaque racine détruite (une copie d'une racine couvre ses
  administrations imbriquées) ; une administration commune hors zone détruite ne se copie pas.
  Aucune sélection sémantique des références « utiles » : index, références, reflogs, objets et
  configuration de chaque administration détruite doivent tous être préservés.
  Une administration partagée avec un autre worktree est **toujours refusée**, même archivée.
  Une ambiguïté, une copie différente, un verrou, des données locales/ignorées non préservées
  ou un rapport déjà présent refusent le retrait. Le rapport JSON est créé exclusivement hors
  cible, hors administration et hors copie. Garder copies et rapport, ne pas réessayer en force.
  Un lien source suivi dans l'index en mode 120000, inchangé et dans un arbre propre est une
  feuille : readlink/lstat sont enregistrés sans suivre sa cible. Lien source modifié ou non
  suivi, racine/admin/copie/.git liés restent refusés. Les administrations séparées sans nom
  `.git` sous les sources sont aussi inventoriées et attribuées par Git avant retrait.

  Les checkouts sont attribués par leurs **ancrages vérifiés** : source `.git`,
  core.worktree explicite et allers-retours avec l’administration ; les inscriptions
  liées exigent gitdir/commondir réciproques. `git --git-dir ... worktree list` ou
  `--show-toplevel` ne prouvent pas une source : après deinit ils peuvent rendre le
  cwd ou l’administration. Ne jamais changer de cwd ou réécrire core.worktree pour
  faire passer une garde.

  Une administration de sous-module désinitialisée reste inventoriée et archivée.
  L’état `desinitialise` exige une preuve positive : .gitmodules, gitlink160000
  stage0, rangement privé attendu sous modules/nom, source vide ou absente et zéro
  inscription liée. Absence de core.worktree seule, source non vide, partage,
  ancrage cassé ou configuration ambiguë restent rouges. La garde ne lance aucun
  deinit/prune/repair et ne supprime aucun contrôle d’archive.

  Le propriétaire du lot relit le **nouveau** script qualifié intégré et son hash,
  prépare les archives fidèles à l’état courant, puis produit un rapport frais hors
  cible. Une ancienne garde rouge ou une ancienne copie du script ne se convertit
  pas en autorisation de retrait. En particulier, le correctif de l’inventaire ne
  retire pas de lui-même les ressources du porteur ; celui-ci vérifie quiescence,
  fidélité des copies et rapport VERT sans nouvelle mutation entre garde et retrait.
  La quiescence relève du propriétaire ; une réussite est périmée dès qu'un état change avant
  le retrait. La garde ne supprime ni ne répare et ne garantit pas l'arrêt de writers tiers.
  Incident : `docs/codeparsers/tickets/2026-11-11-le-nettoyage-du-parent-invalide-un-worktree-de-sous-module.md`.
- Les rapports de session (`01-rapport`, `02-knowledge-dump` du jour) se tiennent à jour après
  chaque lot et citent les tickets de ses actions et de ses suites. Tout nouveau travail reçoit
  un ticket dans le dossier de son porteur, ou `docs/tickets/` si celui-ci reste à attribuer.
  Travail terminé ou abandonné : `git mv` vers `docs/tickets/closed/` et État mis à jour dans
  le même commit, avec la preuve de réalisation ou la raison de l’abandon. Un blocage ou un
  transfert ne vaut pas clôture.
- **Le README suit ce que l'utilisateur voit** (Lucie, 8 novembre 2026) : un lot dont la ligne de
  `docs/nouveautes.md` change ce que l'utilisateur voit ou fait — un outil, une commande, un
  réglage, un comportement visible — met aussi à jour le README concerné (`README.md` à la racine,
  `extension/rag3weaver/README.md`) dans le même lot. Sinon, un rattrapage des README depuis
  `nouveautes.md` toutes les semaines.
