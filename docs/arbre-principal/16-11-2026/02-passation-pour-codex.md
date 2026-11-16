# Arbre principal : la passation pour Codex (16 novembre 2026)

Cette page est faite pour une session Codex neuve, qui n'a que les docs et les tickets. Elle dit ce qui est sur
master, le travail à prendre dans l'ordre (chaque entrée renvoie à un ticket réel), ce qui attend Lucie, et les
règles du périmètre. Chemins de code relatifs à `extension/rag3weaver/` sauf mention. Le détail de la session est
dans `docs/arbre-principal/16-11-2026/00-rapport-de-session.md` et dans les pages qu'il cite.

## 1. L'état

**Sur master** (le 16 novembre, au matin) :
- **Les générations** (ED, everything declarative) : `Scope` et `File` portent `_couche`, `_depuis` et `_jusqu_a`.
  Une lecture voit une couche à un rang. L'écrivain (`src/code_sync.rs`) écrit au **rang en attente**, que les
  lectures sans vue voient ; `_generation` garde une ligne par génération publiée. Seule la couche `backend` a des
  générations, et **rien du produit ne publie encore** (`Catalog::publier_la_generation` n'a pas d'appelant).
- **`migration generations --base <chemin.rag3db>`** (`src/migration_des_versions.rs`,
  `migrate_base_to_generations`) : une base passée par 1b (`migration versions --base`) monte aux générations.
- **2b′, les vues** (`src/views.rs`, `src/generations.rs`) : une vue `(layer, rank)` se lit dans `_generation` en
  pile (`load_stack`, `load_stack_on`, refus typé `StackError`) ; `Visibilite::with_stack` la compose en prédicat
  (`Predicate::Visible.mask`, par uuid et par fichier). Une pile d'un étage donne la condition d'avant au bit près.
- **2b-C** (recherche) : le paramètre `view = {layer, rank?}` de tous les outils de lecture, traité une fois dans
  `call_tool_on` (`src/backend.rs`, `CALL_VIEW`, `load_call_view_stack`) ; la réponse dit la vue lue, le rang en
  attente et s'il porte des lignes. « Index pas prêt, ~N s » et l'attente bornée `search.index_wait_s`.
- **Les suivies** (`src/suivies.rs`) : la déclaration `tracked` d'un manifeste (racine, sources à sélecteur git,
  `views {max_depth, load_timeout_ms}`), l'id stable d'une couche, `layer_id_by_name`, `repo_id`.

**En branche** : rien d'arbre principal. Tous les lots ont atterri.

**Ce qu'il faut savoir pour ne rien casser** :
- **Une vue sans rang lit la génération publiée**, qui reste 0 tant que rien ne publie : sous `view: {layer:
  backend}`, une base vivante se lit dans son état de la bascule. C'est voulu et dit dans la réponse ; le remède est
  un ticket (2.B1).
- **`rang` de la `Visibilite` du catalogue est le rang où l'écrivain écrit.** Une vue se pose sur la visibilité
  d'une lecture (l'instantané d'un appel, `with_stack`), jamais sur celle du catalogue qui écrit.
- **La bascule des bases de Lucie est préparée, pas faite** (partie 3). Ses copies et journaux de répétition sont à
  luciepc sous `~/.cache/rag3weaver-build/bascule/` : n'y rien effacer avant le ticket du ménage (2.B4).
- **Un lot qui change une requête, une signature ou un champ** relit tous ses appelants sur la tête reposée après
  chaque rebase, pas seulement les conflits (le 16 novembre, l'écrivain avait ajouté un appelant de
  `merge_generation` que le rebase n'a pas signalé ; treize témoins ont rougi).

## 2. Le travail pour Codex, dans l'ordre

Chaque entrée : le ticket, ce qu'il faut faire, la preuve, la taille, ce qui bloque. **La commande de passe et de
push est commune**, en fin de partie. Une suite exigée par un chemin (`tools/poste/required_suites.txt` : tout lot
qui touche `src/code.rs`, `src/code_sync.rs`, `src/catalog.rs`, `src/catalog/*`, `src/generations.rs`,
`src/marque_d_ecrivain.rs` ou `tests/e2e_code_sync.rs` joue `e2e_code_sync`) est ajoutée par la passe d'elle-même.
`docs/procedures/push-d-un-lot.md` y ajoute `e2e_postgres` pour `code.rs`, l'IR (`ir/`) ou un gabarit, et
`e2e_lectures` pour le bus ou l'hôte.

### A. Sans dépendance : commencer ici

**A1. Les sondes et bancs qui lisent `Scope_Chunk` en dur** — `docs/arbre-principal/tickets/2026-11-13-les-sondes-bancs-et-mesures-du-code-lisent-scope-chunk-en-dur.md`
- **En cours par arbre principal** (16 novembre, 11 h), branche `arbre-principal/scope-chunks-des-sondes` sur master 1e45d6f21 : Codex la saute et part de la première entrée libre. Une fois poussée, le ticket passe dans `docs/tickets/closed/` avec son hash.
- **Faire** : dans `tests/sonde_morceaux_vides.rs` (18 occurrences), `tests/e2e_mesure_index_entier.rs` (16),
  `tests/e2e_banc_etage.rs` (4), `tests/e2e_banc_hnsw.rs` (1), `tests/e2e_mesure_ingestion_code.rs` (2), lire la
  table des morceaux par `src/code.rs` `scope_chunks(&cat)` (champs `table`, `parent`, `chunked_from`, `to_scope`)
  au lieu de `Scope_Chunk`. Chaque mesure exige au moins un morceau avant de mesurer. Une mesure qui porte
  vraiment sur la forme d'avant le dit et prend `register_code_schema_sans_versions`. Ne change pas ce qu'elles
  mesurent. `tests/e2e_code.rs` (l. 1677, 1684, 1756, 1771) lit aussi `Scope_Chunk` : vérifier si c'est la forme
  d'avant voulue, et le dire dans le message.
- **Preuve** : la garde « au moins un morceau » ajoutée d'abord rougit sur une base neuve (rouge) ; puis chaque
  cible compile (`scripts/check_features.sh` les compile toutes) et se joue une fois. Pas d'`e2e_code_sync` sauf
  si `e2e_code.rs` change.
- **Taille** : 2 à 3 h. **Bloque** : rien.

**A2. Un fichier de plus sous `templates/` pendant la lib entière** — `docs/arbre-principal/tickets/2026-11-13-un-fichier-de-plus-apparait-sous-templates-pendant-la-lib-entiere.md`
- **Faire** : d'abord le diagnostic dans le test même, `src/builtin_templates.rs:507`
  `every_shipped_template_is_embedded_and_sorted` : en cas d'écart, le message liste le différentiel entre les
  fichiers trouvés et `BUILTIN`. Ensuite, le test qui écrit sous `templates/` écrit ailleurs (les suspects : les
  tests qui chargent `templates/backends/code/backend.json` en place, `src/backend.rs` l. ~6959, 6986, 9660, 9698,
  9740, 9796).
- **Preuve** : un test bouchon qui écrit un fichier sous `templates/` fait sortir son nom dans le message ; puis
  la lib entière plusieurs fois au portable sans écart. `cargo test --lib builtin_templates`.
- **Taille** : 1 h de diagnostic, 1 à 2 h de correctif. **Bloque** : rien.

**A3. Le creux d'une version avant son embarquement doit dire « pas prêt »** — `docs/arbre-principal/tickets/2026-11-13-le-creux-d-une-version-avant-son-embarquement-doit-dire-pas-pret.md`
- **Faire** : un témoin dans `tests/e2e_entites_derivees.rs`, `sparse_search_before_its_embedding_says_not_ready` :
  ouvrir la fixture comme `ouvrir_catalogue_projection` (l. ~596) **sans** `set_sparse_embedder`, enregistrer
  l'entité en `HYBRID | SPARSE` comme `catalogue_projection_sur` (l. ~611), poser des occurrences, chercher avec
  `SearchOptions { signals: SPARSE, strict_signals: true }`, et exiger l'erreur ou l'avertissement de
  `expliquer_le_silence_d_un_signal` (`src/catalog.rs` ~4967) au lieu d'un résultat vide. S'il rougit, le correctif
  est dans le chemin des entités de contenu (`src/dataflow/generic_search_nodes.rs` ~1175 et ~1942). Ne remets pas
  le refus générique retiré le 13 novembre.
- **Preuve** : `./run_e2e.sh --test e2e_entites_derivees` ; `e2e_code_sync` si `catalog.rs` change.
- **Taille** : 1 à 2 h pour le témoin, 2 à 4 h de plus s'il rougit. **Bloque** : rien.

**A4. `e2e_impact` et `e2e_usages` dépendent de la forme du code vivant** — `docs/arbre-principal/tickets/2026-10-11-e2e-impact-depend-de-la-forme-du-code-vivant.md`
- **Faire** : un corpus figé (`tests/data/impact-corpus/*.rs`, à créer : fonction → aide → aide → test, trois sauts)
  porte l'attente exacte de chemin ; sur le code vivant, `tests/e2e_impact.rs` ne garde que « le test est atteint à
  profondeur ≤ N », et `tests/e2e_usages.rs` (l. ~105) passe d'une égalité stricte à un sous-ensemble (les appels
  relevés à la main ⊆ trouvés). Ensuite seulement, rendre à `src/catalog.rs` `ingest_entities_jusqu_a` (~6571) sa
  forme lisible (le corps dans la cellule, l. ~6606, dans une fonction à part). Ne touche ni `split_unchanged` ni la
  logique d'ingestion.
- **Preuve** : sortir le corps de la l. ~6606 dans une fonction fait rougir le test d'aujourd'hui (rouge) ; les
  témoins neufs `impact_names_the_test_through_three_hops_on_the_stable_corpus` et
  `impact_reaches_the_live_test_within_depth_n` sont verts. `./run_e2e.sh --test e2e_impact`, puis
  `--test e2e_usages`. `catalog.rs` touché : `e2e_code_sync`.
- **Taille** : 3 à 4 h. **Bloque** : rien.

**A5. Le témoin de mort du raccord physique meurt par un signal une fois sur dix** — `docs/arbre-principal/tickets/2026-11-13-le-temoin-de-mort-du-raccord-physique-meurt-par-un-signal-une-fois-sur-dix.md`
- **Faire** : dans `src/dataflow/migrations/physical_index/tests.rs` (`native_process_death_recovers_same_program_and_lost_ack_once`,
  l. ~523, assertion l. ~545), nommer le signal par `std::os::unix::process::ExitStatusExt::signal()`. Boucler le
  test seul (`--exact`, cent tours), puis en lib entière, pour départager. Piste non vérifiée : un autre test de la
  lib envoie des signaux par pid ou par groupe (`src/commande.rs` `signaler_le_groupe` ~1159, les `kill -KILL` de
  `src/catalog.rs` ~14376 et `src/code_sync.rs` ~2193-2229) ; si c'en est un, borner ses envois à ses propres pids.
- **Preuve** : le signal nommé dans le message ; cent tours verts en lib entière sous `poste lourd`.
  `cargo test --lib dataflow::migrations::physical_index`.
- **Taille** : 2 à 4 h. **Bloque** : si la cause n'est pas trouvée, la question du ticket (sortir ces témoins du jeu
  par défaut) est à Lucie, par l'orchestration.

**A6. L'estimation du reste, partie 1 : le journal horodaté et le débit par phase** — `docs/arbre-principal/tickets/2026-11-16-l-estimation-du-reste-d-une-vue-par-phase.md`
- **Faire** : points 1, 2 et 4 du ticket. Horodater chaque ligne du journal d'index (`src/dataflow/index_nodes.rs`)
  sans changer la forme de la dernière ligne (`DONE_LINE` l. ~242, `FAILED_PREFIX`, `STOPPED_PREFIX`) ; un débit
  par phase (plein texte, relations, vecteurs) lissé sur les derniers paquets ; un champ `remaining_s` dans
  `index_states` (`src/backend.rs` ~5988). Le contrat est celui de
  `docs/rag3weaver-archi/15-11-2026/12-conception-l-attente-d-index-du-pas.md`.
- **Preuve** : rouge d'abord, un index de N paquets dont le premier est rapide et les suivants lents (un faux
  embarqueur lent) : l'estimation à mi-chemin est à moins de 25 % du réel. Lib filtrée, puis `e2e_lectures`.
- **Taille** : 6 à 8 h. **Bloque** : rien ; le débit de départ par banc d'étalonnage est à banc et embarquement.

**A7. Le ménage des tickets parapluies** (docs seules, un lot)
- `docs/arbre-principal/tickets/2026-11-12-migrer-les-ancres-de-code-vers-le-contenu-partage.md` : remplacer
  l'État par la liste de ce qui est sur master (1a, 1b, la liaison branche → couche, le chargeur `views::load_stack`,
  le masque en prédicat 7a2765baf1, `view` ce79df0ffc) et ne garder ouverts que 2a′-3 et la bascule. 1 h.
- `docs/arbre-principal/tickets/2026-11-13-appliquer-la-visibilite-des-couches-avant-les-requetes-des-vues.md` :
  rapprocher un à un les treize contre-exemples de `docs/arbre-principal/13-11-2026/28-plan-visibilite-couches-2b.md`
  (l. 69-99) des tests de master ; fermer le couvert, laisser les manques en tickets nommés. 2 h.
- `docs/arbre-principal/tickets/2026-10-11-l-index-ne-suit-pas-l-arbre-edite.md` : le minimum (la fraîcheur dite,
  `src/dataflow/catalog_read.rs` `freshness_of`) est sur master ; rattacher le reste à 2a′-3. 15 min.
- `docs/arbre-principal/tickets/2026-10-11-un-appel-d-outil-attend-derriere-un-index-sans-le-dire.md` : fermer ce
  qui est couvert (la lecture sans mutex, « index pas prêt », le rattrapage en fond) avec ses hashes ; le seul reste,
  la priorité des requêtes au service d'embarquement, devient un ticket pour embarquement (C3). 20 min.
- **Preuve** : `python3 -I extension/rag3weaver/scripts/check_tickets.py` VERT. Push en docs seules.

### B. Après la bascule des bases de Lucie

**B1. 2a′-3 : les couches par branche et par worktree** — `docs/arbre-principal/tickets/2026-11-16-2a-prime-3-les-couches-par-branche-et-par-worktree.md`
- **Faire** : la page de conception `docs/arbre-principal/16-11-2026/01-2a-prime-3-couches-par-branche-et-par-worktree.md`,
  entière : couches d'histoire par les objets git sans checkout ; couches de travail par worktree, parente au rang du
  HEAD ; un seul fil de montre ; `tracked.writes.max_work_layers` (32, refus nommé) ; le sélecteur
  `worktree:<motif>` (`src/suivies.rs`) ; la fin d'une couche. **Avec**, la publication de la couche du backend à
  chaque fin d'index (`docs/arbre-principal/tickets/2026-11-16-une-vue-sans-rang-lit-l-etat-de-la-bascule-tant-que-rien-ne-publie.md`) :
  dans `src/dataflow/index_nodes.rs`, à côté de `note_indexed_tree` (~417-427, sous le même verrou), appeler
  `publier_la_generation`, **avant** `DONE_LINE`, qui reste la dernière ligne. Attention au témoin
  `src/backend.rs` ~8659 `a_view_reports_the_pending_rank_and_whether_it_holds_rows` : il exige qu'après une
  réaction de la montre la vue ne voie pas encore le fichier ; qui publie (l'index entier, ou aussi `tip`) se lit
  dans la page 2a′-3, § 1, et se demande à l'orchestration en cas de doute.
- **Preuve** : les sept témoins de la page, rouges d'abord (le premier : une recherche venue d'une autre tâche, vue
  sur un worktree de test, voit son fichier non commité et pas celui du backend), puis
  `an_index_run_on_a_generations_base_publishes_its_rank`. Passe entière avec `e2e_code_sync` et `e2e_lectures`.
- **Taille** : 3 à 4 jours, en lots : publication, couches d'histoire, couches de travail, montre et plafond,
  sélecteur. **Bloque** : la bascule des bases de Lucie (partie 3).

**B2. Les vues des worktrees des sessions** — `docs/arbre-principal/tickets/2026-11-16-les-vues-des-worktrees-des-sessions.md`
- **Faire** : le nom court `view: {"worktree": "<chemin>"}` résolu dans `call_tool_on` ; la vue courante par
  `context.view` de l'invocation (`src/daemon_host/invocation.rs:36` la refuse aujourd'hui), qui remplit `CALL_VIEW`.
- **Preuve** : les deux témoins du ticket, rouges d'abord, dans le module de tests de `src/backend.rs` ; lib filtrée
  `view`, `e2e_lectures`.
- **Taille** : 4 à 6 h. **Bloque** : B1.

**B3. L'estimation du reste, partie 2 : le reste d'une vue** — le même ticket que A6, point 3 : le reste en
fichiers entre la pointe d'une vue et sa génération publiée. 3 à 4 h. **Bloque** : B1.

**B4. Le ménage après la bascule** — `docs/arbre-principal/tickets/2026-11-16-le-menage-apres-la-bascule.md` : la
liste exacte y est (worktrees par `scripts/garde_worktree.py`, copies, binaires, targets ; ce qu'il faut garder).
1 h. **Bloque** : la bascule réelle et le verdict B′ du banc.

### C. En attente d'un mot

**C1. Les dix premiers de `search_code` rendent plusieurs copies d'un même contenu** — `docs/arbre-principal/tickets/2026-11-14-les-dix-premiers-de-search-code-rendent-plusieurs-copies-d-un-meme-contenu.md`
- **Faire** : une version par ligne, avec la liste de ses occurrences dessous (`src/search.rs`
  `project_occurrence_results` ~1607, le rendu `src/dataflow/render_nodes.rs`). Ne touche pas le masque.
- **Preuve** : `the_top_ten_never_repeat_a_content_hash`, rouge d'abord ; puis le juge de parité du banc.
- **Taille** : 4 à 6 h. **Bloque** : la forme de sortie change le contrat de `search_code` : la faire trancher par
  l'orchestration avant d'écrire.

**C2. Le délai du service d'embarquement est un chiffre rond** — `docs/arbre-principal/tickets/2026-11-08-le-delai-du-service-d-embarquement-est-un-chiffre-rond.md`
- **Faire** : mesurer le p99 de `/embed`, `/rerank` et `/ocr` à luciepc sous charge réelle, par service et par
  taille de lot ; écrire la page de mesure ; remplacer le 60 de `src/daemon/embeddings.rs` `default_embed_timeout`
  (~939) par un commit qui la cite. Ne touche pas `RAG3WEAVER_EMBED_TIMEOUT_SECS`.
- **Preuve** : la mesure (le témoin de comportement existe : `le_relecteur_et_l_ocr_d_un_demon_muet_rendent_une_erreur_nommee_dans_leur_delai`).
- **Taille** : 3 à 4 h. **Bloque** : une fenêtre `poste mesure` à luciepc, au mot de l'orchestration et d'embarquement.

**C3. La priorité des requêtes au service d'embarquement** — le reste de
`docs/arbre-principal/tickets/2026-10-11-un-appel-d-outil-attend-derriere-un-index-sans-le-dire.md` (après A7) :
une recherche fait la queue derrière les lots d'index. 4 à 6 h. **Bloque** : la session embarquement possède le
service ; la forme (client ou service) est à son mot.

### La passe et le push (commune à toutes les entrées)

À luciepc (quatre places lourdes) ou au portable (une), dans un worktree **à toi** sous `~/git_workspaces/`
(la passe l'y cherche) :
```
cd ~/git_workspaces/rag3db && git fetch -q origin && git worktree add -b cx-<lot> ~/git_workspaces/rag3db-cx-<lot> origin/master
cd ~/git_workspaces/rag3db-cx-<lot> && git submodule update --init -q && git config user.email luciedefraiteur@gmail.com
# la passe (le témoin, les suites, check_features et sa preuve, la lib entière, les témoins navigateur) :
cd ~/.cache/rag3weaver-build && CIBLE=target-cx-<lot> SUITES=<suites e2e, séparées par des virgules, ou - > \
  POSTE_MEM_MAX=32G ./poste lourd timeout 10800 bash passe_epinglee.sh rag3db-cx-<lot> cx-<lot> <filtre du témoin> -
# le push, au « à toi » de l'orchestration seulement :
if tail -1 ~/.cache/rag3weaver-build/cx-<lot>/resume.log | grep -qx VERT; then
  ./poste push cx-<lot> -- bash ~/.cache/rag3weaver-build/push_garde.sh rag3db-cx-<lot> cx-<lot>
fi
```
- Le journal se nomme par un **nom**, jamais un chemin absolu. `RAG3WEAVER_MOTEUR_ANCIEN=1` seulement si la lib
  épinglée (dans `passe_epinglee.sh`) est plus vieille que le C++ du lot ; le résumé le dit, et ça se dit avec le
  résultat.
- **Reposer un lot** : `git rebase --onto origin/master HEAD~<n>` (n = le nombre de commits du lot, lu par
  `git log --oneline origin/master..HEAD`), puis `git range-diff` qui doit dire « = » ; une branche rebasée se pousse
  sous un nom neuf (`cx-<lot>-2`).
- Les trois contrôles après le push : `git status` vide, aucun rebase en cours, HEAD = master.

## 3. Ce qui attend Lucie (Codex n'y touche pas)

- **La bascule de ses trois bases** (`code-rag3db-depot` au portable, `code-lourd` et `code-principal` à luciepc) :
  instantané, `maintain` de lucivy, 1b, la montée aux générations, puis le repassage base ouverte. Elle est répétée
  verte sur copies (`docs/arbre-principal/15-11-2026/02-la-fenetre-de-la-bascule.md`) ; le plan de G est dans
  `docs/optimiseur/16-11-2026/01-plan-de-la-bascule.md`. **Elle attend** : le verdict de B′ au banc (un index neuf du
  même arbre, qui départage les deux bonnes réponses perdues tous signaux confondus sur 38), lucivy, et le mot de
  Lucie. **À lui dire avant** : environ 9 % des morceaux changent de découpage, donc de vecteur ; le dense s'améliore
  (rang 1 : 8 contre 3 sur 30). Retour arrière : l'instantané et l'ancien index 72 h, puis retrait base par base.
- **Ses manifestes** (`~/.config/rag3weaver/*/templates/backends/*/backend.json`) : aucun changement sans elle,
  ni `tracked`, ni découpage, ni modèle.

## 4. Les règles du périmètre

- **Les identifiants en anglais** pour tout code neuf : identifiants, options, clés de manifeste, noms d'outils,
  noms de tests et de journaux (règle de Lucie, 16 novembre). La prose, les commits et les tickets restent en
  français. **Pas de renommage de l'existant.**
- **Un worktree et un target à toi**, branches `cx-…`. **L'arbre de Lucie** (`~/git_workspaces/rag3db`) est
  intouchable : rien n'y est commité, aucune branche n'y bascule (`git worktree add` depuis lui est permis).
- **Rouge d'abord** : un témoin se voit échouer avant le correctif. **Une suite non jouée est rouge**, un SAUTÉ
  n'est pas vert.
- **La passe et le push par `docs/procedures/push-d-un-lot.md`**, jamais sans le « à toi » de l'orchestration ;
  pas de force, même `--force-with-lease`, pas d'effacement de branche distante.
- **L'identité git est `luciedefraiteur@gmail.com`**, zéro mention d'un assistant dans les commits.
- **Un défaut connu non corrigé a son ticket** sous `docs/<session>/tickets/`, contrôlé par
  `python3 -I extension/rag3weaver/scripts/check_tickets.py`. Fermer, c'est `git mv` vers `docs/tickets/closed/`
  avec l'État mis à jour.
- **Un lot qui touche `src/code.rs`, `src/manifestes.rs` ou `src/queries_via.rs`** monte `REGLES_DE_L_ANALYSE`
  (`src/code.rs`) ou dit « sans effet sur l'analyse » dans un message (`scripts/check_regles_de_l_analyse.py`).
- **Les lourds** passent sous `poste lourd`, jamais `-j$(nproc)` (`-j 8`), sous `POSTE_MEM_MAX=32G`.
