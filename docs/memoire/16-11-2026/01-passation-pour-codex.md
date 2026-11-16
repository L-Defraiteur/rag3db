# Passation de la session mémoire (96) pour Codex

16 novembre 2026, demandée par Lucie. Cette page est faite pour une session Codex neuve, qui n'a
que les docs et les tickets. Tout chemin de code part de `extension/rag3weaver/`. Tout ticket est
sous `docs/memoire/tickets/`.

Le contexte long est ailleurs :

- le knowledge dump de la semaine : `docs/memoire/13-11-2026/04-knowledge-dump.md` ;
- les pages du 14 novembre : `docs/memoire/14-11-2026/` ;
- les cinq pages du 15 novembre : `docs/memoire/15-11-2026/01` à `05`.

## 1. État

### Sur master, de cette session

| Hash | Ce qui est livré | Page |
| --- | --- | --- |
| `39ce7c1ae` | `compose_restart` (le contexte d'après une compaction, composé dans un budget) ; le paramètre `context` d'`archive` (montré au modèle, jamais rangé) | 15-11 03 |
| `0653a6aef` | **usage** : chaque Decide pose `usage {ms, prompt_tokens, cached_prompt_tokens, completion_tokens}`, la trace d'`archive` le rend par section, et `usage_total` en fin de rapport | 15-11 04 |
| (même push) | **query_chars** : la requête de la recherche des candidates est bornée à 4 000 caractères (`scripts/archive_query.rhai`, `graphs/archive_recall.mmd`) | — |
| (même push) | **context_title** : le titre devant `context` se règle, et le défaut ne change pas | 15-11 05 |
| `53a83d089` | **segment** : `LlmNode` sous `response_schema` (port `json`) ; `ForEachNode` `when` et `flatten` ; `archive` découpe en sujets `§from-to` les sections de plus de `segment_over` (0 = éteint) ; un `LlmNode` n'est admis dans un outil qu'avec son `response_schema` et sous `call_models` | ci-dessous |

### En branche

- **`cx-memoire-compaction-temoin-archi`, tête `3d30f16ee`** : le témoin
  `tests/e2e_compaction_archiviste.rs`, rouge 4 sur 4 par conception. Il appartient au lot de la
  session **archi** (le code de l'hôte : le déclencheur à 80 %, `POST /ide/chat/compact`,
  `archived_through_turn`, la reprise par `compose_restart`). Codex n'y touche pas sans le mot de
  l'orchestration. La conception est dans `docs/memoire/15-11-2026/03-la-compaction-est-l-archiviste.md`.
- Aucune autre branche mémoire ouverte. Les branches `lot-96-*` et `cx-memoire-*` restantes ont
  toutes atterri.

### Ce qui tourne

- Rien de cette session.
- Les hôtes des deux postes tournent sur le binaire `4802a147a5` (posé par la session optimiseur).
- La mémoire de Lucie est montée en M8 depuis le 15 novembre (27 outils).
  - L'instantané d'avant : `~/.local/share/rag3weaver/instantanes/avant-m8-15-nov/` au portable.
  - Le retour : `~/.cache/rag3weaver-build/m8/montee-15-nov/RETOUR.sh`.
- Les services d'embarquement sont à luciepc :
  - 7878 : granite-278m ;
  - 7879 : bge-m3, dual, joint du portable par le tunnel 7980.

### Ce qu'il faut savoir pour ne rien casser

- **La lib épinglée des passes** est `373b5477b`
  (`rag3db-build-373b5477b4fd413f96605061b8d09393439a1a30-…`, voir le magasin
  `~/.cache/rag3db-libs/courant`). Le moteur de master l'a dépassée : toute passe pose
  `RAG3WEAVER_MOTEUR_ANCIEN=1` et le **dit** dans son résumé.
- **Les suites exigées pour un lot du gabarit memory ou de l'archiviste** :
  - check_features, la lib entière, les témoins navigateur ;
  - puis, une par appel de `run_e2e.sh` (un seul filtre par appel) : `e2e_archiviste`,
    `e2e_compose_restart`, `e2e_memoire_modele`, `e2e_cited_by`, `e2e_memoire_fichiers`,
    `e2e_reconstruction`, `e2e_whois`, `e2e_idempotent_registration`.
  - Un lot qui touche `code.rs`, `code_sync.rs`, `catalog.rs` ou `catalog/*`, `generations.rs` ou
    `marque_d_ecrivain.rs` rejoue aussi `e2e_code_sync`.
- **Les e2e de la mémoire embarquent pour de vrai** : `RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7878,127.0.0.1:7879,127.0.0.1:7880`
  à luciepc, plus `127.0.0.1:7980` au portable. Un témoin sauté faute de service n'est pas vert.
- **Ne jamais jouer contre le bge-m3 partagé le rouge d'un témoin de taille** (un texte long,
  une borne). Le jouer sur l'embarqueur qui note les tailles :
  - `embarqueur_qui_note(dim)` dans `tests/common/mod.rs` pour les e2e ;
  - `crate::embedder::embarqueur_qui_note` sous `cfg(test)` pour la lib.

  Un texte de 8 000 jetons a fait tomber 7879 deux fois le 15 novembre.
- **`e2e_banc_fiches_classees` est un banc à services** : il ne se joue pas dans une batterie.
  On écrit « NON JOUÉ (banc à services) ».
- **Deux rouges connus de master**, chacun avec son ticket. Ils ne viennent pas d'un lot, et on
  les cite sans les compter comme une faute du lot :
  - `e2e_banc_etage::banc_etage_qui_perd` ;
  - `codex_threads::…coupe_au_delai`, sous la charge seulement.
- **La garde du push refuse** si master a apporté du code (n'importe quel fichier du crate) depuis
  la base testée. On repose alors, on rejoue court, et on relance la garde : jamais d'exception.
- **À luciepc, le shell de connexion est fish** : toute commande distante passe par
  `ssh luciepc bash -s <<'EOF' … EOF`.
- **`check_tickets.py`** (`scripts/check_tickets.py`) refuse un ticket dont la **Famille** n'est
  pas `moteur` ou `produit`. Le lancer avant de pousser un ticket.

## 2. Travail pour Codex, dans l'ordre

Chaque entrée renvoie à son ticket, qui porte le détail. Ici : quoi, la preuve, la taille, ce qui
bloque. Les premières n'attendent personne ; les dernières attendent un mot (dit à chaque fois).

**Les commandes, pour toutes les entrées** (depuis `extension/rag3weaver/` de ton worktree, avec ton
propre target) :

```sh
export RAG3DB_LIB=rag3db-build-373b5477b4fd413f96605061b8d09393439a1a30-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF
export CARGO_TARGET_DIR=~/.cache/rag3weaver-build/target-cx-<ton-lot> CARGO_INCREMENTAL=0 RAG3WEAVER_MOTEUR_ANCIEN=1
export RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7878,127.0.0.1:7879,127.0.0.1:7880   # + ,127.0.0.1:7980 au portable
# un filtre de la lib :
( . scripts/lib_du_magasin.sh; exporter_la_lib >/dev/null; ~/.cache/rag3weaver-build/poste lourd cargo test -j 14 --lib \
    --features rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres,openai-llm,anthropic-llm -- <filtre> )
# une suite e2e (une par appel) :
./run_e2e.sh --features openai-llm --test <suite>
```

Une suite e2e neuve se monte en trois pas :

1. le fichier `tests/e2e_<nom>.rs`, ajouté à git ;
2. `python3 tests/e2e/generer.py` ;
3. le test sous `#[test] #[ignore]`.

### 2.1 Le témoin de codex qui rougit sur « Text file busy » — 1 h

`2026-11-15-un-temoin-de-codex-rougit-sur-text-file-busy-sous-la-charge.md`

- **Le défaut** : `src/codex_threads.rs`, aux lignes 261-263 et 303-305. Le test écrit un script et
  le lance aussitôt, et sous la charge les forks des autres tests gardent le descripteur ouvert.
- **À faire** : un utilitaire de test `write_executable` (écrire dans un fichier temporaire, fixer
  les droits, `rename` vers le nom final), utilisé aux deux endroits. `livrer` ne se touche pas.
- **La preuve** : le filtre lib `codex_threads`, puis la lib entière deux fois sans le rouge. Un
  rouge déterministe est difficile à écrire. Si tu en écris un
  (`write_executable_closes_before_exec_under_parallel_forks`), il doit d'abord rougir avec
  l'ancienne écriture.
- **Bloqué par** : rien.

### 2.2 La garde des scripts prend `description` pour un script — 1 h

`2026-11-15-la-garde-des-scripts-prend-description-pour-un-script.md`

- **Le défaut** : `src/dataflow/foreach_node.rs:166` teste `k.contains("script")`, si bien que
  `description=$element.x` est refusé.
- **À faire** : ne reconnaître que les clés exactes `script` et `script_id`. Le refus d'un vrai
  `script` qui substitue `$element` reste en place.
- **La preuve** : `substituted_description_key_is_not_taken_for_a_script`, à côté des tests de
  `foreach_node.rs`. Un sous-graphe avec `CreateRefTypeNode(description=$element.x)` se charge.
  Rouge d'abord. Filtre lib `foreach_node`.
- **Ensuite, au choix** : retirer le contournement de `templates/backends/memory/graphs/gestes_creer.mmd`
  (genres en dur), et jouer `e2e_archiviste`.
- **Bloqué par** : rien.

### 2.3 `banc_etage_qui_perd` refusé par le changement d'index public — 2 à 3 h

`2026-11-15-banc-etage-qui-perd-refuse-par-le-changement-d-index-public.md`

- **Le défaut** : `tests/e2e_banc_etage.rs:791-802` réenregistre `Scope`, déjà peuplée, avec
  d'autres signaux. Le catalogue le refuse à raison (`src/catalog.rs` vers 2232).
- **À faire** : une base par réglage (ou une copie de la base indexée avant chaque
  réenregistrement). **Rien ne se relâche dans `catalog.rs`.**
- **La preuve** : le témoin est déjà rouge. Suite `e2e_banc_etage`, environ 10 minutes. Il passe
  au vert.
- **Bloqué par** : rien. Préviens la session banc, qui tient les bancs d'étage.

### 2.4 Une relation au nom d'une entité casse au premier lien — 2 h

`2026-10-18-une-relation-au-nom-d-une-entite-casse-au-premier-lien.md`

- **Le défaut** : `src/backend.rs` (la boucle des relations, vers la ligne 1823) et
  `src/catalog.rs` (`register_relation_with`) ne comparent pas les noms en ignorant la casse. Une
  relation `NOTE` à côté d'une entité `Note` casse au premier lien.
- **À faire** : refuser au chargement, en nommant les deux. Le message du moteur ne se touche pas.
- **La preuve** : `manifest_refuses_relation_named_like_entity_case_insensitive`, dans
  `src/backend_opening_tests.rs`. Ce fichier a déjà le harnais `manifest()` / `prepare()`. Rouge
  d'abord. Filtre lib `case_insensitive`, puis la lib entière.
- **Bloqué par** : rien.

### 2.5 Le code d'un refus du modèle n'est gardé nulle part — 3 h

`2026-11-09-le-code-d-un-refus-du-modele-n-est-garde-nulle-part.md`

- **Le défaut** : `src/events.rs` (`LlmCall`, vers la ligne 199) ne garde que `retries`. Pourtant
  `RetryEvent` (`src/llm.rs`, vers la ligne 270) porte `reason`, `wait` et `from_server`.
- **À faire** : accumuler les reprises dans le puits de l'agent (`src/agent.rs`), et ajouter
  `retry_log: [{reason, wait_ms, from_server}]` à `LlmCall` et à sa sérialisation.
- **La preuve** : `llm_call_event_keeps_each_retry_reason_and_wait`, avec un faux fournisseur qui
  rend 429 puis 200 (voir `tests/openai_llm_retry.rs` et `tests/common/fake_sse.rs`). Rouge
  d'abord. Filtre lib `retry`, puis la lib entière.
- **Bloqué par** : rien.

### 2.6 Un genre de Ref à trait d'union échoue sans bruit dans un graphe cité — 3 h

`2026-11-15-un-genre-de-ref-a-trait-d-union-ne-se-pose-pas-sans-le-dire.md`

- **Le défaut** : `CreateRefTypeNode` et `AddRefNode` (`src/dataflow/ref_nodes.rs`, vers les
  lignes 220 et 498) rendent `executed:false` puis `Ok(())`. Un graphe qui ne lit pas `report`
  finit donc en « ok ».
- **À faire** : un `executed:false` fait échouer l'appel quand le nœud tourne dans un graphe cité.
  Que ce soit par défaut ou par une option `strict`, c'est ton choix, dit dans le ticket. La
  description du paramètre `name` de `templates/backends/memory/graphs/create_ref_type.mmd` dit
  aussi « soulignés, pas de trait d'union ».
- **La preuve** : `hyphenated_ref_type_in_cited_graph_fails_loudly`, dans `tests/e2e_cited_by.rs`.
  Rouge d'abord. Suites `e2e_cited_by`, puis `e2e_archiviste`.
- **Bloqué par** : rien.

### 2.7 La borne `query_chars` prouvée côté embarqueur — 2 h

`2026-11-16-la-borne-query-chars-n-a-pas-de-temoin-cote-embarqueur.md`

- **À faire** : ouvrir la mémoire sur `embarqueur_qui_note(1024)` au lieu du service, archiver une
  section de 40 000 caractères, et vérifier qu'aucun texte reçu ne dépasse 4 000 caractères.
- **La preuve** : `no_text_over_query_chars_reaches_the_embedder`, dans `tests/e2e_archiviste.rs`.
  Le rouge se joue avec `query_chars: 100000` **sur le faux embarqueur**, jamais sur 7879. Suite
  `e2e_archiviste`.
- **Bloqué par** : rien.

### 2.8 L'usage de l'appel de découpage manque au total — 3 à 4 h

`2026-11-16-l-usage-du-decoupage-manque-au-total-de-la-passe.md`

- **À faire** : `LlmNode` pose `_usage` (la forme de `Decide`, compteurs absents sans bloc usage)
  dans son `json`. `archive_segment_build.rhai` le reporte, et `archive_rapport.rhai` le compte
  dans `usage_total` (avec `segment_calls`).
- **La preuve** :
  - le témoin e2e `segment_usage_is_counted_in_the_pass_total`, sur le faux serveur, avec l'aide
    `avec_usage` de `tests/e2e_archiviste.rs` ;
  - les témoins unitaires dans `src/dataflow/llm_nodes.rs`.

  Rouge d'abord. Filtre lib `llm_nodes`, puis la suite `e2e_archiviste`.
- **Bloqué par** : rien.

### 2.9 `compose_restart` au-delà de deux mille apports — 4 à 6 h

`2026-11-16-compose-restart-lit-au-plus-deux-mille-apports.md`

- **À faire** :
  - lire tous les apports de la session (deux formes : la session, croissante et paginée ; les
    autres, bornées par le budget) ;
  - dire `truncated` s'il reste un plafond.

  La forme rendue ne change pas.
- **La preuve** : `compose_restart_reads_every_session_apport_past_the_page`, dans
  `tests/e2e_compose_restart.rs` (2 100 apports de session). Rouge d'abord. Suite
  `e2e_compose_restart`.
- **Bloqué par** : rien.

### 2.10 Une Ref ne vieillit jamais : le dire — 1 h (la suite plus tard)

`2026-10-18-une-ref-ne-vieillit-jamais.md`

- **À faire maintenant** :
  - `templates/backends/memory/schemas/ref_type.json:9` décrit une seule expression, comme
    aujourd'hui ;
  - la description de `graphs/cited_by.mmd` dit « l'état d'une Ref n'est pas vérifié contre sa
    cible ».
- **La preuve** : `cited_by_reports_ref_state_as_unverified`, dans `tests/e2e_cited_by.rs`.
- **Plus tard, sur le mot de Lucie** : le vieillissement lui-même, par le pont, en 2 à 3 jours.

### 2.11 Un apport accepte une `source` qui n'est pas un chemin de Source — 1 h, puis 2 h

`2026-11-07-un-apport-accepte-une-source-qui-n-est-pas-le-chemin-d-une-source.md`

- **À faire maintenant** : `"pattern": "^sources/.+\\.md$"` sur `source`, dans
  `templates/backends/memory/schemas/apport.json`. `notebook/graphs/put.mmd` ne se touche pas.
- **La preuve** : `put_apport_refuses_source_that_is_not_a_source_path`, dans
  `tests/e2e_cited_by.rs`, plus le cas qui passe. Suites `e2e_cited_by`, `e2e_archiviste` et
  `e2e_memoire_fichiers` (elles posent toutes des apports).
- **Bloqué par**, pour la suite seulement : le refus d'une Source absente, contre la création d'une
  Source vide `origine: inconnue`, est à Lucie (section 3).

### 2.12 Une mémoire ne sait pas que ses gabarits sont en retard : l'annonce — 6 h

`2026-11-07-une-memoire-ne-sait-pas-que-ses-gabarits-sont-en-retard.md`

- **À faire** : à l'ouverture, comparer l'épingle des gabarits d'un backend à ceux du binaire, et
  le dire dans `daemon_status` sans rien changer. L'API de plan existe (`src/template_plan.rs`,
  `src/fusion.rs`) et n'est appelée par personne.
- **La preuve** : `stale_pinned_templates_are_reported_without_change`, dans
  `tests/e2e_catalogue_gabarits.rs`.
- **Ce qui ne se fait pas** : la commande d'upgrade, qui attend les arbitrages de Lucie
  (section 3). Jamais sur la mémoire réelle.

### 2.13 L'enfant du crash physique ne rend pas son code sous la charge : le diagnostic — 2 h

`2026-11-14-l-enfant-du-crash-physique-ne-rend-pas-son-code-sous-la-charge.md`

- **À faire** : dans `src/dataflow/migrations/physical_index/tests.rs` (vers les lignes 533-551),
  ajouter au message d'échec le signal (`ExitStatusExt::signal()`) et l'étape. Rejouer ensuite la
  lib entière sous charge, mémoire bornée (`POSTE_MEM_MAX=32G`). Le ticket reçoit la cause.
- **Ce qui ne se touche pas** : la reprise elle-même, avant que la session arbre principal ait lu
  la cause.

### 2.14 À ne pas commencer sans un mot

- `2026-10-19-un-plan-qui-ecrit-ne-s-annonce-pas.md` : rien à faire tant qu'aucun appelant
  n'écrit par un Plan.
- `2026-11-11-raccorder-l-archiviste-automatique-au-chat.md` : recouvre la compaction de la
  session archi. C'est un lot à ouvrir par l'orchestration.
- `2026-11-11-terminer-l-archivage-a-blanc-des-documents-sur-copie.md` : une mesure sur copie,
  sur consigne de l'orchestration.
- `2026-11-11-trancher-et-verifier-le-classeur-local-du-rappel.md` : les quatre arbitrages de
  Lucie d'abord.
- `2026-11-15-un-genre-de-ref-en-base-sans-fichier-dans-la-memoire-de-lucie.md` : la décision de
  Lucie d'abord. La lecture de la ligne 69e7d6f0… se fait sur une **copie**, jamais sur sa base.

## 3. Ce qui attend Lucie (Codex n'y touche pas)

- **Les trois gestes sur sa mémoire**, montée en M8 le 15 novembre : `confirmer_fiche`,
  `scinder_fiche` et `rejoindre_fiches`. Ils ont été répétés sur une copie
  (`docs/memoire/15-11-2026/02-la-montee-m8-repetee.md`), et c'est elle qui les joue.
- **Le RefType en base sans fichier** (uuid `69e7d6f0…`) : la base ou le dossier fait-il foi ?
- **Les quatre arbitrages du classeur local du rappel** (§8 de son ticket).
- **Un apport dont la Source manque** : le refuser, ou créer la Source vide ?
- **La politique des gabarits épinglés** (vivant ou épinglé, conservation des bases, reprise),
  avant toute commande d'upgrade.
- **Le pont** qui ferait vieillir les Refs.

## 4. Règles du périmètre

- **Tout code neuf est en anglais** : identifiants, `/commandes`, options, clés de manifeste et de
  schéma, noms d'outils, de nœuds et de ports, noms de tests, et le texte des prompts et des
  descriptions qui servent le produit. Restent en français la prose des docs, les commits et les
  tickets. L'existant se traduit quand on le touche, jamais en lot.
- **Un worktree et un target à toi**, sur une branche `cx-…`. L'arbre de Lucie
  (`~/git_workspaces/rag3db`) ne se commite ni ne se bascule jamais.
- **La passe et le push suivent `docs/procedures/push-d-un-lot.md`** : une passe sous
  `poste lourd` (jamais `-j$(nproc)`), puis le push par
  `poste push <toi> -- push_garde.sh <worktree> <journaux>` dans un `if grep -qx VERT`, et les
  trois contrôles. **Jamais sans le « à toi » de l'orchestration.**
- **Rouge d'abord** : le témoin se voit rouge, pour la bonne raison, avant le correctif. Un rouge
  pour une autre raison (un paramètre inutilisé, un service absent) n'en est pas un.
- **Une suite non jouée est rouge**. Un témoin sauté n'est pas vert. On lit « running N tests »
  avant d'annoncer.
- **L'identité git est l'adresse gmail de Lucie, avec zéro attribution** dans les commits. Pas de
  force push, pas de suppression de branche distante : une branche rebasée se pousse sous un
  nouveau nom.
- **Chaque commit se pousse sur sa branche dans la minute.** Un défaut connu non corrigé reçoit un
  ticket.
