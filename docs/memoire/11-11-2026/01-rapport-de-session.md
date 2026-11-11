# Rapport de la session « mémoire » (96) — état au 11 novembre 2026

Ce rapport est écrit pour un agent qui reprend ce périmètre sans aucun souvenir de nos échanges.
Le vocabulaire est défini dans le second fichier du dossier
(`docs/memoire/11-11-2026/02-knowledge-dump.md`, § 1) ; les mots les plus courants le sont aussi
ici, à leur première apparition.

- **Lucie** : la propriétaire du projet. Elle seule décide de ce qui touche sa mémoire, son poste
  et ses bases en service.
- **c7** : la session d'orchestration (son adresse change d'une session à l'autre ; aujourd'hui
  `uds:/run/user/1000/cc-socks/3576675.sock`, nom « rag3db-5a »). Elle ordonne les pushes de
  toutes les sessions et transmet les décisions de Lucie.
- **Un lot** : un ensemble de commits qui part sur `master` d'un coup, après une passe de tests.
- **La passe** (ou le **tour**) : la batterie de tests jouée à luciepc avant un push (§ 4 du
  second fichier).

## Actualisation, 11 novembre 2026

Le lot 4xx a déjà atterri sur master sous `2d6a08a65` : les consignes de push de ce lot plus bas sont historiques. La reprise de l’archivage sur copies et les preuves du 400 sont dans [03-reprise-archivage.md](03-reprise-archivage.md). Aucun archivage dans la mémoire réelle n’a été effectué.

## 1. Ce qui est sur master

| hash | ce que c'est |
|---|---|
| `662dda2cc` | M4, pas 1 : un backend sert un modèle de langage (`models.llm`) aux graphes de ses outils |
| `6e1dc868e` | M4, pas 2 : le nœud `SplitSections` (découpe un Markdown en sections avec leur titre) |
| `ec9a333e3` | la politique des outils : `policy.call_models` (un outil a le droit d'appeler un modèle), `Route` et `SplitSections` dans les nœuds de base, le modèle tenu par le `Backend` |
| `5eb6fb277` | `run_e2e.sh` : le jeu de features de la procédure par défaut, une suite sans sa feature est rouge |
| `ff7a18fe2` | M4, pas 3 : l'outil `archive` de la mémoire (l'archiviste, § 3 du second fichier) |
| `3b119beb6` | la tentative 5 du banc des fiches classées, et `allow_create` éteint par défaut dans `archive` |
| `a9872fb87` | un « banc à services » ne fait jamais partie de l'e2e entier ; nommé sans sa voie, refusé avant tout bâti |
| `6a4d55d13` | la page de l'essai de l'équipier sur les README, et trois tickets |
| `0fff7aa00` | la page de conception de M8 (la montée des gabarits) |
| `0b0ff221a` | les témoins n'écrivent plus dans le dossier d'état réel de Lucie (racine temporaire, garde, `.init_array` du binaire e2e) |
| `a6637eeee` | le recul du client du modèle se dit et se borne (`GenOptions.retry_budget`) |
| `dc7741cb8` | la page de conception du classeur local dans le rappel |
| `8a20e0914` | le rattrapage des deux README depuis `docs/nouveautes.md` (novembre) |
| `5e9ebaf9a` | la page de la précision du rappel d'aujourd'hui |

Les pages citées sont dans `extension/rag3weaver/docs/25-octobre-2026-memoire-longue/` (pages 01
à 06, M4) et `extension/rag3weaver/docs/09-novembre-2026-memoire-longue/` (pages 01 à 04).

## 2. Ce qui n'est pas sur master

- **Le lot du refus 4xx** : branche `lot-96-requete-refusee`, commit `42ac0bb9c`, poussée sur
  origin. Sur un refus HTTP 4xx du fournisseur du modèle, le client garde la requête et le corps
  entier de l'erreur dans `<racine d'état>/requetes-refusees/`, et l'erreur dit le chemin. Témoin
  `a_4xx_refusal_keeps_the_faulty_request_and_the_whole_error_body` rouge puis vert (27/27), ticket
  fermé dans le lot. **Rejeu vert le 9 novembre à 14 h 59 sur 3c0ec28b5** (features 5/5, lib
  2093 avec un seul rouge, l'instable connu `un_symbole_cite_se_resout_par_l_index_sans_ouvrir_de_depot`,
  vert seul ; navigateur 33/0). **Il attend le « à toi » de c7** pour son tour de push. Si master a
  bougé, le tour rebase et rejoue court de lui-même.
- **Un ajout à la page de M8**, non commité : dans le worktree `~/.cache/rag3weaver-build/wt-mf-96`,
  `git stash list` → `stash@{0}` (« WIP on docs-96-archivage-blanc »). Il ajoute le cas « la
  réponse de `recall_fiches` chez Lucie n'a pas `via` » à
  `extension/rag3weaver/docs/09-novembre-2026-memoire-longue/02-m8-la-montee-des-gabarits.md`.
  c7 demande aussi d'y noter deux faits de l'archivage à blanc : l'instantané note l'extension
  vector de 9bae60786, refusée à l'ouverture, et « dossier ≠ base : 1 écart ».
- Les autres branches `lot-96-*` et `docs-96-*` sur origin sont des branches de travail dont le
  contenu est déjà sur master ; elles ne se suppriment pas (règle : pas de suppression de branche
  distante sans Lucie).

## 3. Les chantiers en cours

### 3.1 L'archivage à blanc (en cours, interrompu)

Demande de c7 : ce que l'outil `archive` ferait des documents de la semaine dans la mémoire de
Lucie, **sur une copie seulement**, `allow_create` allumé sur la copie, pour que Lucie approuve ou
taille la proposition avant toute écriture dans sa vraie mémoire.

- **La copie** vient de l'instantané pris par G le 9 novembre à 12 h 18, hôte fermé :
  `~/.local/share/rag3weaver/instantanes/avant-29a7bc52a/memoire-longue/` (base, `.extensions`,
  `.fts`, sans `.wal` : arrêt propre), plus le dossier des fiches `~/memoire-longue`. Contrôle : la
  même recherche rend les mêmes 19 fiches sur la copie et sur la base en service.
- **Le montage** est dans le dossier de travail de la session précédente (mémoire vive, il
  disparaît au redémarrage du portable) :
  `/tmp/claude-1000/-home-lucied-git-workspaces-rag3db/25b7ff4b-dabf-42ef-8580-86661c846fb9/scratchpad/blanc/`
  — `templates/` (le gabarit memory de master, son manifeste pointé sur la copie, `models.llm`
  Vertex), `copie/` (la copie neuve), `copie-essai-1..3/` (copies salies par les essais),
  `src/` (les 50 documents figés), `lancer.py` (un processus `rag3weaver-backend <manifeste>` par
  document, requête JSON `{"op":"call","name":"archive",…}` sur l'entrée), `resultats*.jsonl`.
- **Première mesure de M8, acquise** : le gabarit memory d'aujourd'hui s'ouvre sur sa base.
- **Le blocage** : sur le premier document (7 sections), Vertex rend « HTTP 400
  INVALID_ARGUMENT » à 5 décisions sur 7. En écartant les fiches du juge des candidates
  (`search_options` : `must_not via = juge`), 7 sur 7 passent : la cause est dans les étiquettes
  des fiches du juge (bilans, outils), sans être reproduite à la main. Et le temps : 351 s pour
  7 sections (≈ 50 s par décision) avec le binaire installé, qui n'a pas le recul borné.
- **Le binaire neuf** (master 3c0ec28b5 + le lot du refus 4xx) est bâti à luciepc :
  `~/.cache/rag3weaver-build/target-96-fiches4/debug/rag3weaver-backend` (440 Mo, profil dev,
  lié à la lib 29a7bc52a par le magasin). Sa copie vers le portable a été coupée :
  `…/scratchpad/blanc/rag3weaver-backend` est **un fichier partiel de 179 Mo, inutilisable**.
- **c7 a dit** : pas de lancement des 50 pages avant d'avoir compris le 400 et mesuré le temps par
  section sur le binaire neuf ; le premier tiers d'abord (rapports, knowledge dumps, pages de
  conception, ≈ 430 sections), les tickets ensuite.

### 3.2 Les pages qui attendent un mot

- **M8, la montée des gabarits** : page 02 du dossier du 9 novembre ; attend la relecture d'ED
  (la session « everything declarative », qui a écrit le gabarit vivant). Rien ne se code avant.
- **Le classeur local dans le rappel** : page 03 ; attend le mot de Lucie (cinq questions en fin
  de page). La page 04 (la précision du rappel) dit qu'il ne vaut pas sa seconde aujourd'hui :
  le contenu de la mémoire manque, pas le juge.
- **Le ménage du dossier d'état de Lucie** : 477 entrées laissées par les témoins dans
  `~/.local/state/rag3weaver/manifestes/`, listées dans le ticket
  `docs/tickets/2026-11-09-les-temoins-ecrivent-leur-etat-de-manifeste-dans-le-dossier-d-etat-reel.md`.
  Une seule est réelle (son atelier, empreinte `bb347ff93e835435`) et doit rester. Rien n'est
  effacé : Lucie tranche.

### 3.3 L'équipier sur les README (fini, mesuré)

La tâche `readme-nouveautes-novembre` a fini en un pas sans rien livrer (page 01 du dossier du
9 novembre). Restes : la tâche dans `~/.local/share/rag3weaver/taches/e4d8635ecfe7/readme-nouveautes-novembre/`
(backend 852ca016d10f), sa branche `agent/readme-nouveautes-novembre` dans le dépôt de l'arbre
d'essais de c7, et le worktree `~/git_workspaces/rag3db-essais-c7` avancé à 74519fc65 (à c7).
Les trois tickets nés de l'essai sont à everything declarative, sauf le recul (fait, § 1).

## 4. Ce qui reste sur disque

**Le portable** (`lucied@` local) :
- `~/.cache/rag3weaver-build/wt-mf-96` : worktree du dépôt rag3db, branche `lot-96-requete-refusee`,
  status propre, une remise (`stash@{0}`, § 2) ;
- `~/.cache/rag3weaver-build/target-96` : target cargo, 19 Go ;
- `~/.cache/rag3weaver-build/mesurer_fil_par_pas.py` : la mesure des jetons d'entrée par pas d'un
  équipier (§ 6 du second fichier) ;
- le dossier de travail de la session précédente, 635 Mo en mémoire vive :
  `/tmp/claude-1000/-home-lucied-git-workspaces-rag3db/25b7ff4b-dabf-42ef-8580-86661c846fb9/scratchpad/`
  (`blanc/` l'archivage à blanc, `precision/` la mesure du rappel, `essai-readme/` le client de la
  boîte de l'équipier, les diffs et messages de commit des lots).

**luciepc** (l'autre poste, `ssh lucied@luciepc`) :
- `~/git_workspaces/rag3db-p1r-96` : l'arbre du tour (worktree détaché sur 3c0ec28b5, le diff du
  lot du refus posé dans l'index) ;
- `~/.cache/rag3weaver-build/target-96-fiches4` : target cargo, 15 Go ;
- `~/.cache/rag3weaver-build/` : mes scripts (`tour.sh`, `pousser.sh`, `essai.sh`, `un-test.sh`,
  `pose-et-teste.sh`, `nav-seul.sh`, `banc5.sh`, `recul-temoin.sh`, `refus-temoin.sh`,
  `bin-blanc.sh`, `etat-e2e.sh`), les diffs et messages des lots (`<lot>-tout.diff`,
  `<lot>-commit.txt`, `<lot>-suites.txt`, `<lot>-court.txt`, `<lot>-base-jouee.txt`), leurs
  journaux, `lib-des-passes` (la lib épinglée : 29a7bc52a) et `instables.txt`.

**Aucune unité systemd ni aucun service** de cette session ne tourne (vérifié le 11 novembre).
Rien n'a été supprimé.

## 5. Mes tickets ouverts (dans `docs/tickets/`, non déplacés)

| ticket | une ligne |
|---|---|
| `2026-10-18-une-ref-ne-vieillit-jamais.md` | le cycle de vie d'une Ref est déclaré, rien ne le déclenche ; attend le mot de Lucie |
| `2026-10-18-une-relation-au-nom-d-une-entite-casse-au-premier-lien.md` | une relation nommée comme une entité se déclare sans refus et casse au premier lien |
| `2026-10-19-un-plan-qui-ecrit-ne-s-annonce-pas.md` | `Catalog::run_plan` écrit sans événement ; pas atteignable en service |
| `2026-11-07-un-apport-accepte-une-source-qui-n-est-pas-le-chemin-d-une-source.md` | `put_apport` accepte une source qui ne nomme aucune Source ; avec M7 |
| `2026-11-07-une-memoire-ne-sait-pas-que-ses-gabarits-sont-en-retard.md` | M8 : la mémoire de Lucie est une copie du 25 octobre, sans montée possible |
| `2026-11-09-le-code-d-un-refus-du-modele-n-est-garde-nulle-part.md` | les reprises d'un appel ne gardent pas leur code (429, 503) ; en partie couvert par le lot du refus 4xx |
| `2026-11-09-le-recul-du-client-du-modele-ne-se-voit-pas-et-n-a-pas-de-borne-de-pas.md` | fait pour le client (a6637eeee) ; reste un recul court dans la boîte de l'équipier, et le client Anthropic |
| `2026-11-09-les-temoins-ecrivent-leur-etat-de-manifeste-dans-le-dossier-d-etat-reel.md` | corrigé (0b0ff221a) ; reste le ménage des 477 entrées, à Lucie |
| `2026-11-09-une-tache-sans-plan-se-dit-faite-sur-un-step-done-qui-dit-ce-qui-reste.md` | à everything declarative : sans plan, un `step_done` vaut fin même avec `remaining` |

## 6. Ce qui est bloqué, et sur quoi

| quoi | bloqué sur |
|---|---|
| le push du lot du refus 4xx | le « à toi » de c7 |
| l'archivage à blanc | le binaire neuf sur le portable, puis la cause du 400 lue dans une requête gardée |
| M8 (le code) | la relecture d'ED |
| le classeur local | le mot de Lucie |
| le ménage du dossier d'état | le mot de Lucie |
| l'archivage réel dans la mémoire de Lucie | sa décision sur la proposition de l'archivage à blanc, puis M8 (sa mémoire n'a pas l'outil `archive`) |
| M7 étendu (`session` posée par l'hôte sur une Source) | après l'archivage, avec le ticket de l'apport |

## 7. Reprendre

Trois travaux ne sont pas finis. Pour chacun : où, dans quel état, et les commandes pour repartir
de ce point plutôt que de master.

### 7.1 Le lot du refus 4xx — worktree du portable

- **Chemin** : `~/.cache/rag3weaver-build/wt-mf-96` (sur le portable), worktree du dépôt
  `~/git_workspaces/rag3db`.
- **Branche** : `lot-96-requete-refusee`, dernier commit `42ac0bb9c` (« fix(llm): un refus 4xx du
  modèle se relit… »), poussée sur origin, **pas sur master**. Status propre (le sous-module
  `extension/rag3weaver/codeparsers` paraît modifié : c'est l'arbre partagé, ne pas le commiter).
- **Fait** : le correctif, son témoin rouge puis vert (27/27), le ticket fermé, le rejeu vert du
  9 novembre à 14 h 59 sur 3c0ec28b5 sous la lib 29a7bc52a.
- **Reste** : le push, au « à toi » de c7.
- **Le target** : `~/.cache/rag3weaver-build/target-96` (19 Go) n'est pas celui des passes ; les
  passes se jouent à luciepc (§ 7.2).
- **Une remise** dans ce worktree : `stash@{0}`, l'ajout à la page de M8 (§ 2). Pour la reprendre :

      cd ~/.cache/rag3weaver-build/wt-mf-96
      git fetch -q origin && git switch -c docs-m8-ajout origin/master
      git stash pop stash@{0}      # applique l'ajout à 02-m8-la-montee-des-gabarits.md
      # ajouter les deux faits de l'archivage à blanc (§ 3.1), puis :
      git commit -- extension/rag3weaver/docs/09-novembre-2026-memoire-longue/02-m8-la-montee-des-gabarits.md
      git push origin HEAD:master  # docs seules ; puis les trois contrôles

### 7.2 L'arbre du tour — luciepc

- **Chemin** : `~/git_workspaces/rag3db-p1r-96` (sur luciepc), worktree détaché sur
  `3c0ec28b5`, avec le diff du lot du refus posé dans l'index (`git status` : le ticket fermé
  ajouté, `builtin_templates.rs`, `openai_llm.rs`, `tests/openai_llm_retry.rs`). Le tour le remet
  à zéro lui-même (`reset --hard`) à chaque pose : rien à sauver dedans.
- **Le target** : `~/.cache/rag3weaver-build/target-96-fiches4` (15 Go), bâti pour la lib
  29a7bc52a ; il contient aussi le binaire neuf `debug/rag3weaver-backend` (440 Mo, master
  3c0ec28b5 et le lot du refus).
- **Les fichiers du lot** : `~/.cache/rag3weaver-build/refus-tout.diff`, `refus-commit.txt`,
  `refus-suites.txt` (vide), `refus-court.txt` (vide), `refus-base-jouee.txt` (3c0ec28b5 et la
  lib), les journaux `refus-*.log`.
- **Pour pousser** (au « à toi » de c7) :

      ssh lucied@luciepc 'systemd-run --user --unit=refus-push-96 --collect bash ~/.cache/rag3weaver-build/tour.sh refus push'
      ssh lucied@luciepc 'tail -8 ~/.cache/rag3weaver-build/refus-push.log'   # attendre « poussé : HEAD … » et « TOUR_FINI »

  Si master a reçu du Rust depuis 3c0ec28b5, le tour rejoue court avant de pousser. Si la lib
  des passes a changé (`~/.cache/rag3weaver-build/lib-des-passes`), il refuse et demande un rejeu :

      ssh lucied@luciepc 'systemd-run --user --unit=refus-rejeu-96 --collect bash ~/.cache/rag3weaver-build/tour.sh refus rejeu'

### 7.3 L'archivage à blanc — le dossier de travail du portable

- **Chemin** : `/tmp/claude-1000/-home-lucied-git-workspaces-rag3db/25b7ff4b-dabf-42ef-8580-86661c846fb9/scratchpad/blanc/`
  (mémoire vive : perdu au redémarrage du portable ; tout se refait depuis l'instantané).
- **Ce n'est pas un worktree** : `templates/` (le gabarit memory de master, manifeste pointé sur
  `copie/`), `src/` (les 50 documents figés), `lancer.py`, `copie/` (copie neuve, déjà salie par
  l'essai du 9 novembre sans les fiches du juge), `resultats.jsonl` (cet essai : 7/7, 351 s).
- **La base d'essai** : `copie/` vient de
  `~/.local/share/rag3weaver/instantanes/avant-29a7bc52a/memoire-longue/` et de `~/memoire-longue`.
  Pour une copie neuve :

      B=<le dossier blanc>; I=~/.local/share/rag3weaver/instantanes/avant-29a7bc52a/memoire-longue
      rm -r $B/copie && mkdir -p $B/copie   # seulement la copie de travail
      cp -a --reflink=auto $I/memoire-longue.rag3db $I/memoire-longue.rag3db.extensions $I/memoire-longue.rag3db.fts $B/copie/
      cp -a --reflink=auto ~/memoire-longue $B/copie/memoire-longue

- **Le binaire** : `blanc/rag3weaver-backend` est **partiel** (179 Mo sur 440) ; le recopier :

      scp lucied@luciepc:.cache/rag3weaver-build/target-96-fiches4/debug/rag3weaver-backend $B/rag3weaver-backend

- **L'état de `lancer.py`** (vérifié le 11 novembre) : il appelle encore le binaire installé
  (`~/.local/bin/rag3weaver-backend`, sans le recul borné ni la garde des refus) et passe
  toujours `search_options` avec `must_not via = juge`. La retouche qui ajoutait `BINAIRE` et
  `SANS_JUGE` a été coupée et n'est pas appliquée. Pour rejouer le premier document **avec** les
  fiches du juge et le binaire neuf : dans `lancer.py`, remplacer le chemin du binaire par
  `$B/rag3weaver-backend` et retirer la clé `search_options` de la requête, puis :

      cd $B && LIMITE=1 python3 lancer.py $B $B/src
      ls $B/etat/requetes-refusees/   # la requête refusée par Vertex et le corps de l'erreur

  `lancer.py` pose lui-même `GOOGLE_APPLICATION_CREDENTIALS` (le chemin dans `.vault`),
  `GOOGLE_CLOUD_PROJECT`, `RAG3WEAVER_EMBED_SERVICE` (7979 à 7981), `RAG3WEAVER_STATE_DIR`
  (`$B/etat`) et `RAG3DB_LIBRARY_DIR` (la lib 29a7bc52a du magasin), et écrit une ligne par
  document dans `resultats.jsonl` (déplacer l'ancien d'abord : il saute les documents déjà faits).
- **Reste** : comprendre le 400, mesurer le temps par section, puis les 50 pages
  (`LIMITE` absent), et la page de la proposition pour Lucie.

## 8. Les trois prochains pas

1. **Pousser le lot du refus 4xx** au « à toi » de c7 :
   `ssh lucied@luciepc 'systemd-run --user --unit=refus-push-96 --collect bash ~/.cache/rag3weaver-build/tour.sh refus push'`,
   lire la fin de `~/.cache/rag3weaver-build/refus-push.log` (« poussé : HEAD … »), vérifier sur
   origin, donner le hash à c7.
2. **Reprendre l'archivage à blanc** : recopier le binaire neuf de luciepc (ou en rebâtir un à la
   tête de master), refaire `copie/` depuis l'instantané, rejouer le premier document **avec** les
   fiches du juge pour obtenir une requête gardée, lire pourquoi Vertex la refuse, mesurer le
   temps par section ; puis les 50 pages du premier tiers, et la page de la proposition pour
   Lucie (sujets créés, fiches rangées, ancrages, appels, temps).
3. **Pousser l'ajout à la page de M8** (la remise `stash@{0}`, plus les deux faits de l'archivage
   à blanc), docs seules, puis M8 lui-même après la relecture d'ED.
