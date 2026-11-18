# Passation de la session recherche, pour Codex

16 novembre 2026, demandée par Lucie (« demandes claires de tickets à traiter ensuite, chacune pour
Codex »). Cette page est faite pour une session neuve, qui n'a que les docs et les tickets. Le savoir de
fond est dans `docs/recherche/11-11-2026/02-knowledge-dump.md`. Les rapports récents sont
`docs/recherche/14-11-2026/03-rapport-de-session.md` et les pages du 16 (`01-la-vue-des-outils.md`,
`02-plan-index-pas-pret.md`).

## 1. État

**État final, 16 novembre 2026, 11 h, arrêt demandé par Lucie** :
- aucune entrée de la partie 2 n'est en cours, et aucune n'est encore faite ; Codex part de l'entrée 1 ;
- aucune branche inachevée : tous les lots de la session sont sur master ;
- rien ne tourne de cette session, ni passe, ni lourd, ni mesure, aux deux postes (liste vide vérifiée) ;
- le ticket de l'entrée 10, l'attente d'index par vue, porte depuis ce matin la forme de c7 : l'attente
  compte aussi l'embarquement sous le signal vector. Elle attend le débit du banc et le mot de Lucie.

**Sur master, de cette session (le plus récent d'abord)** :

| hash | lot |
|---|---|
| 8b7686025e | « index pas prêt » : un outil `freshness` attend un index en retard sous `search.index_wait_s` (6 s), ou répond `index: {ready: false, eta_s, pending_files, waited_s?}` ; débit observé gardé dans `_catalog_meta` |
| 457892de98 | 2b-C, part recherche : `view: {layer, rank?}` sur tout outil qui lit, pile chargée dans l'instantané de l'appel, réponse `view: {layer, rank, depth, pending, pending_rows}` |
| 6e493fd5db | deux gardes de test : exclusion posée après coup, témoin de temps de `usages` en rapport |
| 68b1454d8 | l'erreur d'origine d'un BEGIN refusé ; pas de recommencement sur une base à rouvrir |
| 262be6fc4 | les orphelins du plein texte : un `FlushNode` suit `DeleteRecordNode` ; offset lu en base hors cache |
| 9fd946ec3 | filtre `_uuid IN` résolu par la clé primaire (`UNWIND … MATCH {_uuid}`) |
| 1bb929755 | `workspace.index_exclude` (minifiés, code trop gros) |

**En branche, pas atterri** : `mesure-exclusions` (8be832525), un test de mesure seul
(`e2e_code_sync::mesure_ce_que_index_exclude_retire_du_plein_texte`). Ses chiffres sont déjà dans
`docs/recherche/14-11-2026/01-ce-qu-on-n-indexe-pas.md`. Rien ne le demande : ne pas le reprendre sans le
mot de l'orchestration.

**Ce qui tourne** : rien de cette session. Les hôtes (7970 aux deux postes) et les services
d'embarquement de luciepc (7878 granite-278m, 7879 bge-m3) sont à G (`optimiseur`). Ne pas les arrêter.

**À savoir pour ne pas casser** :
- **La lib épinglée** est 373b5477b (`rag3db-build-373b5477b4fd413f96605061b8d09393439a1a30-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF`). Le script de passe la pose. Si le C++ des sources est plus neuf qu'elle, la passe se joue sous `RAG3WEAVER_MOTEUR_ANCIEN=1`, et le résumé le dit.
- **Suites exigées par chemins** : un lot qui touche `code.rs`, `code_sync.rs`, `catalog.rs`, `catalog/*`, `generations.rs`, `marque_d_ecrivain.rs` ou `tests/e2e_code_sync.rs` joue `e2e_code_sync`. La passe l'ajoute d'elle-même, et la garde refuse un push sans elle.
- **Règles de l'analyse** : un lot qui touche `code.rs`, `manifestes.rs` ou `queries_via.rs` monte `REGLES_DE_L_ANALYSE` (`src/code.rs`), ou dit « sans effet sur l'analyse » dans un message de commit. Sinon check_features rougit.
- **Un ticket fermé par un lot** : sa copie ouverte part dans le même lot (`git rm`). Sinon check_tickets rougit (« un même ticket à 2 endroits »).
- **Pièges vus** :
  - `poste mesure` attend que les lourds finissent : l'heure d'une mesure se lit à son journal, pas à son lancement ;
  - une copie de `passe_epinglee.sh` se recopie par `install -m 755` ;
  - le témoin du chat d'une tâche rougit sous charge (ticket ouvert, entrée 5 ci-dessous).

## 2. Travail pour Codex, dans l'ordre

Chaque entrée renvoie à un ticket réel. La passe et le push se font par la recette de la partie 4. Les
tailles comptent la passe.

### 1. L'état d'index qui voit les orphelins (4 à 6 h)

> **18 novembre — commencée par Codex, non qualifiée.** Détournée vers le prérequis « ouvrir un index
> existant sans écrivain » (`src/fts_existing.rs`), en branche `cx-recherche-fts-existing-canon-proofs-r1`
> (c16f1a4075). Le relevé vivant dans l'état n'est pas fait. Voir
> `docs/recherche/18-11-2026/01-evaluation-du-travail-de-codex.md`.

Ticket : `docs/recherche/tickets/2026-11-15-l-etat-d-index-ne-voit-pas-les-orphelins-du-plein-texte.md`.
- **Faire** : `index.fts_counts` (l'état, `Backend::index_states` dans `backend.rs`, écrit par
  `Catalog::noter_les_comptes_du_plein_texte`) vient aujourd'hui de la seule fin d'un index complet. Le
  relevé à blanc existe sur master : `POST /backend/maintain` avec `dry_run: true`,
  `src/fts_maintenance.rs` et `src/daemon_host.rs`, qui rend `{live_docs, graph_docs, orphans, …}`.
  Écrire ce relevé dans l'état, avec sa date (`measured_at_ms`), à chaque passe de maintenance et à la
  fin d'un index. L'état dit l'âge du chiffre, et un chiffre ancien ne se présente jamais comme
  l'état présent.
- **Ne pas toucher** : la purge (`maintain` sans `dry_run`) et son jeton.
- **Prouver** : témoin rouge d'abord, dans `backend.rs` : un orphelin posé par un test (un document
  lucivy ajouté sans ligne, comme `la_fin_d_un_index_dit_les_comptes_du_plein_texte` dans
  `dataflow/index_nodes.rs`), puis l'état relu, qui le compte sans index complet entre les deux. Suites :
  `e2e_code_sync` (le chemin touche `catalog.rs`) et `e2e_lectures`.

### 2. `search_code` avant tout index (3 à 5 h)

Ticket : `docs/recherche/tickets/2026-11-14-search-code-avant-l-index-ne-balaie-plus-les-fichiers.md`.
- **Faire** : départager la piste du ticket, `adaptive_search_decision(state, has_source && !projected)`
  dans `dataflow/generic_search_nodes.rs`, la condition `!projected` venue de c67ca39c6. Lire si `Scope` du
  schéma de code porte une `occurrence_projection`. Avant tout index, la recherche doit balayer les
  fichiers pour la cible `Scope` du code.
  - Si c'est une régression : remettre le balayage pour le code, rouge d'abord.
  - Si c'était voulu : l'écrire dans le ticket, puis réécrire l'assertion « avant l'index » de
    `bindings/nodejs/test/essai.js` avec embarquement (session `embarquement`).
- **Prouver** : témoin rouge dans `backend.rs` : un backend de code neuf, aucun index, `search_code`
  sur un mot d'un fichier, la réponse le trouve par balayage. Suites : `e2e_code`.

### 3. Le CSV d'un COPY tué (4 à 6 h)

Ticket : `docs/recherche/tickets/2026-11-11-un-copy-tue-laisse-son-csv-dans-le-dossier-temporaire.md`.
- **Faire** : `fichier_csv` (chercher le nom dans `src/`) écrit dans `std::env::temp_dir()`. Le mettre
  dans un dossier lié à la base (`<base>.copy/`), vidé à l'ouverture suivante de cette base, jamais
  sous un processus vivant (le verrou de l'écrivain le dit).
- **Prouver** : le témoin d'arrêt brutal `e2e_tx_par_paquet_arret`, rejoué plusieurs passes propres. Retirer
  sa ligne de `tests/e2e/suites_qui_laissent_des_residus.txt` dans le même commit (la règle est en tête
  de ce fichier). Suites : `e2e_tx_par_paquet_arret`, `e2e_code_sync`.

### 4. Le filtre e2e vide : vérifier, puis fermer (1 h)

Ticket : `docs/recherche/tickets/2026-11-11-un-filtre-e2e-vide-peut-donner-un-code-de-sortie-vert.md`.
- **Faire** : `run_e2e.sh` refuse déjà un filtre qui ne désigne rien (ligne « le filtre … n'a désigné
  aucun test »). Le prouver par la commande du ticket (`run_e2e.sh --test e2e_lectures qu_on_laisse_tomber`),
  qui doit rendre un code non nul, puis fermer le ticket avec la sortie et le commit qui l'a corrigé
  (`git log -S "n'a désigné aucun test" -- extension/rag3weaver/run_e2e.sh`).
- **Lot** : docs seules.

### 5. Les deux témoins du chat qui rougissent (6 à 8 h)

Tickets : `docs/recherche/tickets/2026-11-14-le-temoin-du-chat-d-une-tache-rougit-sous-charge.md` et
`docs/recherche/tickets/2026-10-25-le-temoin-du-chat-rougit-encore-sur-le-tour-lent.md`.
- **Faire** : sous charge, la page reste vide, sans requête en vol ni erreur
  (`daemon_host::chat_qui_code::le_chat_d_une_tache_mene_a_sa_relecture_dans_la_page`,
  `src/daemon_host/navigateur.rs:362`). Lire ce que la page attend : un module qui ne charge pas, ou un
  premier rendu suspendu à un événement manqué. Corriger la page, ou faire attendre au témoin la cause
  exacte. **Un délai plus long n'est pas une réponse.**
- **Prouver** : le témoin rejoué sous charge. Lancer en parallèle une passe lourde d'un autre worktree,
  au mot de l'orchestration, puis trois passes vertes. Suites : les témoins navigateur (étape de la passe).

### 6. Un arrêt qui coupe la requête du moteur en cours (1 à 2 jours)

Ticket : `docs/recherche/tickets/2026-11-07-un-arret-ne-coupe-pas-une-requete-du-moteur-en-cours.md`.
- **Faire** :
  - `DbConnection::interrupt` (trait de `src/connection.rs`), sans effet par défaut, et branché sur
    `Connection::interrupt` du binding (`tools/rust_api/src/connection.rs`) pour `Rag3dbConnection` ;
  - les nœuds d'un run lisent sur la connexion de son instantané (`src/read_pool.rs`, `Instantane`,
    déjà une connexion par appel). L'arrêt d'un run (`CancelHandle`) interrompt celle-là, jamais la
    connexion partagée.
- **Prouver** : témoin rouge : une requête lente (un `UNWIND range(0, 10^8)` qui compte) arrêtée en vol
  rend la main en moins d'une seconde, et la requête d'un autre run sur le même backend va au bout.
  Suites : `e2e_lectures`, `e2e_code`.

### 7. Les suites qui laissent des résidus (1 jour)

Ticket : `docs/recherche/tickets/2026-10-18-les-tests-ne-nettoient-pas-leurs-dossiers-temporaires.md`.
- **Faire** : cinq suites laissent des fichiers dans leur TMPDIR à chaque passe : `e2e_code`,
  `e2e_prise_atomique`, `e2e_rag3daemon`, `e2e_repli_de_branche` et `e2e_rouvrir`
  (`tests/e2e/suites_qui_laissent_des_residus.txt`). Une suite par commit : la passe montre ce qu'elle
  laisse, le test qui le crée l'efface (un `TempDir` tenu jusqu'à la fin), puis sa ligne sort du fichier.
- **Prouver** : `run_e2e.sh --test <suite>` (sans filtre de tests) ne dit plus de résidu. Une suite propre
  encore listée est rouge : c'est la règle du fichier.

### 8. Le banc de charge d'ingestion hors de l'e2e entier (3 à 4 h, au mot de c7)

Ticket : `docs/recherche/tickets/2026-11-09-le-banc-de-charge-d-ingestion-tient-la-carte-pres-de-deux-heures-dans-l-e2e-entier.md`.
- **Faire** : le banc prend près de deux heures de carte à chaque passe entière. Le sortir de l'e2e
  entier vers un banc joué sous `poste mesure`, avec ses chiffres écrits : morceaux, débit dense et
  creux séparés. La mesure ne relâche rien, elle change de place.
- **Bloqué par** : le mot de c7 sur sa place. Demander avant d'écrire.

### 9. Le juge Gemini et `required` (2 à 3 h, avec everything declarative)

Ticket : `docs/recherche/tickets/2026-11-11-le-juge-gemini-omet-required-dans-la-preuve-du-schema.md`.
- **Faire** : départager avec la session `everything declarative` le contrat du témoin `e2e_juge` (la
  preuve textuelle attendue du schéma). Le modèle varie, et la cause n'est pas établie. Ne pas ajouter de
  retry.
- **Bloqué par** : ED, et le quota Gemini (des appels payants), à voir avec l'orchestration.

### Hors liste, trouvée par Codex : la garde des comptes sous les générations

> **18 novembre — en branche, non qualifiée.** Sur master, `check_fts_counts` et `rebuild_pending_fts` ne
> passent que par `synchroniser`, jamais par `synchroniser_par_generations`. Codex les rend communs
> (`prepare_fts_for_source_sync`, `code_sync.rs`), avec un rouge produit, en branche
> `cx-recherche-fts-generation-union-r2` (6365396b63). À reposer, sans crochets de test dans le code de
> production, puis qualifier avec `e2e_code_sync`.

### 10. Après 2a′-3 d'arbre principal (1 jour chacun)

- `docs/recherche/tickets/2026-11-16-la-vue-d-un-worktree-par-son-nom-court.md` : `view: {"worktree": "<chemin>"}`.
- `docs/recherche/tickets/2026-11-16-l-attente-d-index-se-compte-par-cellule-et-pas-par-vue.md` :
  l'attente d'index par couche.

Les deux sont **bloqués par 2a′-3** (page `docs/arbre-principal/16-11-2026/01-2a-prime-3-couches-par-branche-et-par-worktree.md`).
Chaque ticket dit sa forme et ses témoins.

**Pas pour Codex** : `docs/recherche/tickets/2026-11-15-chaque-scope-d-une-ligne-minifiee-recoit-la-ligne-entiere.md`
est pour la session `codeparsers`, dont le dépôt est le sous-module, avec un autre flux.
`docs/recherche/tickets/2026-11-13-conserver-et-resorber-le-reliquat-de-preparation-garde-target.md`
attend un arbitrage (partie 3).

## 3. Ce qui attend Lucie

Codex n'y touche pas.
- **Le ménage de luciepc** : les worktrees `rag3db-filtre`, `rag3db-orphelins`, `rag3db-begin`,
  `rag3db-vues`, `rag3db-exclu` et `rag3db-attente`, et les targets `target-97f` à `target-97k`. Tous ces
  lots sont atterris. La commande est dans `docs/recherche/14-11-2026/03-rapport-de-session.md`, partie
  « Reprendre ». Rien ne s'efface avant son mot.
- **Le reliquat garde target** : classer les trois références conservées, sans suppression forcée (ticket
  du 13 novembre).
- **La branche `mesure-exclusions`** : atterrir ou non.
- **Les quotas Gemini du juge** (entrée 9).

## 4. Règles du périmètre

- **L'anglais pour tout code neuf** : identifiants, `/commandes`, options, clés de manifeste, noms
  d'outils et de tests. La prose, les commits et les tickets restent en français. On ne renomme pas
  l'existant ; quand on le touche, un alias.
- **Un worktree et un target à soi** : branches `cx-…`. Jamais un commit ni un `git switch` dans l'arbre de
  Lucie (`~/git_workspaces/rag3db`).
- **Rouge d'abord** : le témoin se voit rouge avant le correctif. Un témoin qui passe du premier coup se
  prouve en cassant à la main ce qu'il juge (le témoin du témoin, voir les pages du 16).
- **Une suite non jouée est rouge**, et un test sauté n'est pas vert.
- **La passe** (à luciepc, sous `poste lourd`, jamais `-j$(nproc)`). La procédure entière est dans
  `docs/procedures/push-d-un-lot.md` :
  ```bash
  ssh -T lucied@luciepc bash -s <<'X'
  B=$HOME/.cache/rag3weaver-build; J=passe-cx-<lot>-$(date +%H%M%S)
  cd $B && CIBLE=target-cx-<lot> SUITES=<suites,séparées,par,virgules> POSTE_MEM_MAX=32G \
    $B/poste lourd timeout 7200 bash $B/passe_epinglee.sh <worktree> $J <filtre du témoin> > $B/$J.out 2>&1
  tail -1 $B/$J/resume.log
  X
  ```
  Pour un témoin de lib : `SUITE_DU_TEMOIN=lib`. Ajouter `RAG3WEAVER_MOTEUR_ANCIEN=1` si le C++ est plus
  neuf que la lib épinglée. La dernière ligne du résumé est `VERT` ou `ROUGE <étape>`.
- **check_features vert avant de donner sa tête** dans la pile (c'est une étape de la passe).
- **Le push, seulement au « à toi » de l'orchestration** (`rag3db-5a`, c7), dans le worktree de la passe :
  ```bash
  if tail -1 $B/$J/resume.log | grep -qx VERT; then
    $B/poste push <nom> -- $B/push_garde.sh <worktree> $J      # en pile : env PILE_AVANT=<tête d'avant> devant
  fi
  ```
  Puis les trois contrôles : `git status` vide, aucun rebase en cours, `FETCH_HEAD` = `HEAD`. Ensuite
  seulement, « poussé » avec le hash.
- **Jamais de push en force**, jamais d'effacement d'une branche distante. Une branche rebasée se pousse
  sous un nom neuf.
- **Identité** : `luciedefraiteur@gmail.com`, jamais l'adresse pro ; aucune mention d'un assistant dans les
  commits ; un commit par chemins (`git commit -- <chemins>`).
- **Un défaut connu non corrigé reçoit un ticket** sous `docs/recherche/tickets/`, et pas une ligne dans
  un message.
