# lucivy — passation pour Codex, 16 novembre 2026

Cette page est écrite pour une session Codex neuve, qui n'a aucune mémoire de nos échanges, seulement les docs et les
tickets. Elle est demandée par Lucie le lundi 16 novembre 2026 au matin, et relayée par l'orchestration (c7). Elle
couvre le périmètre **lucivy** : le moteur de plein texte publié (dépôt `L-Defraiteur/lucivy`) et son pointeur
dans rag3weaver (`extension/rag3weaver/Cargo.toml` de rag3db).

Pour le reste du contexte :
- `docs/lucivy/15-11-2026/01-rapport-de-session.md` : versions 4.3.7 et 4.3.8, mesures, décisions, commandes de
  mesure ;
- `docs/lucivy/11-11-2026/02-knowledge-dump.md` : les règles du périmètre et ce qu'il faut savoir du moteur ;
- dans le dépôt lucivy, le `CLAUDE.md` : architecture, registres, historique des versions.

## 1. État au 16 novembre 2026, 10 h

### La version 4.3.9

- **Publiée** : tag `v4.3.9` sur `main` = `0825bcd`, posé au mot de Lucie (« oki tu peux poser le tag oui stp »).
  `release.yml` est vert du premier coup. Vérifié dans les registres : crates.io (`ld-lucivy`, `lucivy-core`,
  `sparse-vector`, `lucistore`, `luciole`), npm (`lucivy`, `lucivy-wasm`), PyPI (`lucivy`), tous en 4.3.9. Le
  `CLAUDE.md` de lucivy est à jour (main `a1c0dfd`).
- **Ce qu'elle apporte** : une recherche filtrée par `allowed_ids` prend la **longueur moyenne de ses admis**.
  - Jusqu'en 4.3.8, N et les fréquences de documents étaient ceux de la vue, mais la longueur moyenne d'un champ
    restait celle du corpus entier.
  - Une base qui garde ses versions fermées (les générations de rag3weaver) classait donc avec la longueur de toute
    son histoire.
  - Le code : `lucivy_core/src/bm25_global.rs` (`with_subset`) et `lucivy_core/src/search_dag.rs` (`view_tokens`).
    Les fieldnorms sont lus au milieu de leur palier et corrigés par le total exact du segment.
  - Le format ne change pas.
  - La conception : `docs/lucivy/16-11-2026/01-bm25-au-rang-visible.md`.
  - Le témoin : `lucivy_core/tests/test_filtered_scores_ignore_hidden_lengths.rs` (rangs exacts, scores à 5e-3
    relatif près).

### Le pointeur dans rag3weaver

- **master de rag3db est en 4.3.8** (depuis e95a1b8c6, avec la maintenance du plein texte par l'hôte).
- **La branche `lucivy-4-3-9`** (worktree `~/git_workspaces/rag3db-lucivy-439`, au portable et à luciepc) pointe
  aujourd'hui sur la révision `f7da3be` de lucivy (tête f5c2f3124d, rejeu vert de la nuit à luciepc,
  `lucivy-439/passe-20261116-013514`). La session lucivy y remplace les trois dépendances `git = …, rev = …` par
  `"4.3.9"`, rejoue une passe courte, puis le dit « prêt » à l'orchestration. Sa place dans la file : après le lot (5)
  de codeparsers s'il est prêt avant, avant la bascule de lib. **Pas de push sans le « à toi » de l'orchestration.**
- **Si ce pointeur n'est pas sur master quand Codex arrive** : l'entrée 1 de la partie 2 dit comment le reprendre.

### La maintenance nocturne

- Elle a tourné pour la première fois le 16 novembre 2026 à 03:30, sur les quatre backends ouverts, **sans aucun
  refus** (`daemon_status`, clé `fts_maintenance` de l'état d'index).
- Le ticket du dictionnaire des effacés est donc fermé
  (`docs/tickets/closed/2026-11-14-le-dictionnaire-garde-les-textes-des-documents-effaces.md`).
- La même lecture a fait voir un défaut neuf (entrée 2).

### Ce qu'il faut savoir pour ne rien casser

- **Un tag `v*` sur lucivy publie partout** : crates.io, npm et PyPI, par `release.yml`. Une version publiée ne se
  retire pas. **Jamais de tag sans le mot de Lucie**, dit dans la session même.
- **Dans lucivy, tout passe par une PR** vers `main`. Fusion par rebase **dès que la CI de la PR est verte** (règle
  de Lucie), puis le dire à l'orchestration. Codex ne pousse jamais directement sur `main`.
  - La CI : `.github/workflows/ci.yml` (lib par features, clippy `-D warnings`, `lucivy-core` et `lucivy-cpp`,
    ground-truth, audit de `thread::spawn`, Python, Node.js).
  - Un rouge de CI se lit, il ne se relance pas à l'aveugle. Un seuil ne se relâche pas sans la mesure qui le
    justifie (exemple : PR #41, une marge tirée de la structure plutôt qu'un pourcentage).
- **Les versions** : 4.3.x pour un correctif, 4.4 seulement pour un changement de format d'index ou d'API (Lucie).
  Un tag seulement quand la version apporte assez, ou pour un correctif obligatoire.
- **master de rag3db ne porte que des versions publiées de lucivy.** Un pointeur `git = …, rev = …` reste en branche.
- **Les mesures se font sur des copies**, jamais sur les bases de Lucie (`~/.local/share/rag3weaver/*.fts`).
  - Une copie : `cp -a --reflink=auto` de la table voulue, l'hôte à l'arrêt ou le backend fermé
    (`POST /backend/close`).
  - Ouvrir un index écrit dedans.
  - Pas de copie dans `/tmp`, qui est de la mémoire vive.
- **L'arbre `~/git_workspaces/lucivy` des deux postes est celui de Lucie** (sur une ancienne main, avec des
  modifications à elle) : on n'y commite rien et on n'y change pas de branche. On n'y fait que des `fetch` et des
  `worktree add`.

## 2. Travail pour Codex, dans l'ordre

Chaque entrée renvoie à son ticket. Un lot = une branche `cx-…` de lucivy, une PR, un témoin vu rouge avant le
correctif. Les commandes supposent un worktree à soi :

```bash
git -C ~/git_workspaces/lucivy fetch origin
git -C ~/git_workspaces/lucivy worktree add -b cx-<lot> ~/git_workspaces/cx-lucivy-<lot> origin/main
cd ~/git_workspaces/cx-lucivy-<lot>
export CARGO_TARGET_DIR=~/.cache/rag3weaver-build/cx-lucivy-target   # un target à Codex, jamais celui d'une autre session
J=$(( $(nproc) - 2 ))                                               # deux cœurs laissés libres, jamais -j$(nproc)
~/.cache/rag3weaver-build/poste lourd cargo test --release -j $J -p lucivy-core > ~/.cache/rag3weaver-build/cx-lucivy-logs/core.log 2>&1
~/.cache/rag3weaver-build/poste lourd cargo test --release -j $J --lib > ~/.cache/rag3weaver-build/cx-lucivy-logs/lib.log 2>&1
```

Une sortie s'écrit dans un fichier, puis se lit (`tail`, `grep`). On ne pipe pas une passe longue. Une suite qui ne
compile pas, ou qui saute, n'est pas verte.

### Entrée 1 : prouver le bm25 seul de la migration sous 4.3.9

- **Ticket** : `docs/lucivy/tickets/2026-11-16-le-bm25-seul-de-la-migration-reste-a-prouver-egal-sous-4-3-9.md`.
- **Prérequis, si le pointeur n'est pas sur master** (le demander à l'orchestration) :
  1. Dans `~/git_workspaces/rag3db-lucivy-439`, mettre `lucivy-core`, `ld-lucivy` et `sparse-vector` à `"4.3.9"`
     (crates.io) dans `extension/rag3weaver/Cargo.toml`, puis
     `cargo update -p lucivy-core -p ld-lucivy -p sparse-vector`.
  2. Rejouer la passe courte selon `docs/procedures/push-d-un-lot.md` : `fts_handle`, `fts_maintenance`,
     `check_features`, `e2e_code`, `e2e_lectures`, `e2e_demon_hote`. Lib épinglée `373b5477b`,
     `RAG3WEAVER_MOTEUR_ANCIEN=1` dit dans le résumé, `POSTE_MEM_MAX=32G`. Le `resume.log` est écrit par la passe :
     première ligne « commit <tête> », dernière ligne `VERT`.
  3. Dire « prêt » avec la tête, le résumé et le verdict. Pousser seulement au « à toi », par la garde.
- **Ce qu'il faut faire** : quand le banc a rejoué le bm25 seul sur les copies A et B′
  (`docs/banc/16-11-2026/01-la-parite-de-la-migration.md`), lire son verdict.
  - Égalité de classement sur 30/30 : fermer le ticket (`git mv` vers `docs/tickets/closed/`, ligne **État** qui
    commence par « corrigé » ou « fermé », le résumé du banc cité).
  - Sinon : les étapes 1 à 4 du ticket, avec `test_compare_two_indexes`.
- **Taille** : 1 h pour lire et fermer ; une demi-journée si le départage est à faire.
- **Bloqué par** : le pointeur sur master (orchestration), puis la passe du banc.

### Entrée 2 : la compaction ne doit pas grossir un dictionnaire sans texte mort

- **Ticket** : `docs/lucivy/tickets/2026-11-16-la-compaction-du-dictionnaire-grossit-un-dictionnaire-sans-texte-mort.md`.
- **Ce qu'il faut faire** :
  1. Mesurer sur une copie de `Lucivy_Source` de la mémoire longue : `index_report` par fichier, avant et après
     `compact_dictionary`.
  2. Trouver d'où viennent les +3,75 % (les plages d'ids de `ids_section` dans `src/suffix_fst/termtexts_v3.rs`, ou
     la réécriture de générations).
  3. Corriger : ne rien réécrire quand aucun texte n'est mort, et ne pas morceler les plages.
  4. **Ne pas toucher** au format des fichiers, à l'API de `maintain` ni à la garde `max_purge_ratio`.
- **Comment le prouver** :
  - rouge d'abord, dans `lucivy_core/tests/test_maintain.rs` : `dictionary_bytes` après ≤ avant, sur un index sans
    texte mort ;
  - puis `-p lucivy-core` et `--lib` entiers ;
  - puis la mesure 1 rejouée.
- **Taille** : une demi-journée à une journée (la mesure d'abord ; la cause n'est pas connue).
- **Bloqué par** : rien. Une copie de la base mémoire demande le backend fermé : le demander à l'orchestration, qui
  sait si quelqu'un s'en sert.

### Entrée 3 : un ulp d'écart de score entre profils en `parse`

- **Ticket** : `docs/lucivy/tickets/2026-11-14-un-ulp-d-ecart-de-score-entre-profils-en-parse.md`.
- **Ce qu'il faut faire** : rendre la somme des clauses d'un OU indépendante de l'ordre de réponse des segments et des
  dictionnaires.
  - `parse` sur une valeur simple devient un OU de `contains` par mot et par champ.
  - Chercher où les scores des clauses s'additionnent : `src/query/boolean_query/` (`boolean_weight.rs`),
    `src/query/union/` (`buffered_union.rs`, `simple_union.rs`), et le chemin `parse` de `lucivy_core/src/query.rs`.
  - Corriger par une somme dans un ordre fixé par clause, ou en f64 puis arrondie une fois.
  - **Ne pas** changer la formule du BM25 ni l'ordre des résultats hors ex æquo.
- **Comment le prouver** :
  - rouge d'abord, un témoin neuf `lucivy_core/tests/test_parse_scores_identical_across_layouts.rs` :
    - les mêmes documents indexés dans deux dispositions : `shared_dictionary: true` contre `false`, ou un commit
      par document contre un seul commit ;
    - des requêtes `parse` de trois à cinq mots ;
    - exiger `score.to_bits()` égal document par document.
  - Le voir rouge sur main. **Si un petit corpus ne rougit pas**, le dire, et prendre pour témoin rouge le banc du
    profil léger sur une copie : `test_light_profile` sur main, `LUCIVY_PROFILE_VARIANTS=positions,segment_dictionary`,
    un fichier de requêtes réduit aux `parse`. La ligne `SCORES DIFFER … largest gap` doit disparaître.
  - Puis `-p lucivy-core`, `--lib`, et la ground-truth de la CI.
- **Taille** : une demi-journée à une journée.
- **Bloqué par** : rien.

### Entrée 4 : `indexed: false` ignoré pour un champ texte ou `string`

- **Ticket** : `docs/lucivy/tickets/2026-11-16-indexed-false-est-ignore-pour-un-champ-texte.md`.
- **Ce qu'il faut faire** :
  - un avertissement et la doc, pas davantage ;
  - une fonction pure `schema_warnings(&SchemaConfig) -> Vec<String>` (anglais), appelée par `build_schema`
    (`lucivy_core/src/handle.rs`) ;
  - la doc du champ `indexed` (`lucivy_core/src/query.rs:93`) et le README de `lucivy_core`.
  - Honorer `indexed: false` est un choix de Lucie, hors de ce lot.
- **Comment le prouver** : témoin `lucivy_core/tests/test_text_field_indexed_false.rs`, rouge tant que la fonction
  ne rend rien, puis `-p lucivy-core`.
- **Taille** : 2 à 3 h.
- **Bloqué par** : rien.

### Entrée 5 : préparer la 4.3.10, sans la taguer

- **Quand** : quand au moins les entrées 2 et 3 sont fusionnées dans `main`.
- **Ce qu'il faut faire**, sur le modèle du commit de version 4.3.9 (`f7da3be`, à lire avec `git show --stat f7da3be`),
  dans une PR `cx-lucivy-4-3-10` :
  - le numéro dans les neuf `Cargo.toml` et leurs dépendances internes, les sept `package.json`, le `pyproject.toml` ;
  - le `CHANGELOG.md` daté ;
  - « What's new in 4.3.10 » dans les cinq README.
  - Vérifier par `git grep -n '4\.3\.9'` qu'aucun numéro n'est oublié (hors CHANGELOG et historique).
- **Comment le prouver** : la CI de la PR, verte. Puis la fusion.
- **Taille** : 1 à 2 h.
- **Bloqué par** : **le tag est à Lucie** (partie 3). La PR fusionnée, Codex dit à l'orchestration « 4.3.10 prête
  sur main <sha>, CI verte », et s'arrête là.

### Entrée 6 : le chemin sans positions relit tout pour une requête longue (lot long)

- **Ticket** : `docs/lucivy/tickets/2026-11-14-le-chemin-sans-positions-relit-tout-pour-une-requete-longue.md`.
- **Pourquoi en dernier** : aucun index de rag3weaver n'est sans positions aujourd'hui (décision du 14 novembre :
  on garde les positions). C'est ce qui interdit le profil léger un jour.
- **Ce qu'il faut faire** : avant la vérification sur le texte stocké, réduire les candidats d'une requête de
  plusieurs tokens par l'intersection des postings de ses tokens les plus rares. Couper la vérification d'un document
  dès qu'un token exigé y manque. Les réponses ne changent pas : ce sont les mêmes documents, spans et scores qu'avec
  positions.
- **Comment le prouver** :
  - rouge d'abord, par un compteur des documents relus sur le texte stocké (un compteur de test, exposé seulement
    sous `cfg(test)` ou par une stat de requête existante, si une stat convient) : une requête longue de dix mots
    dont un seul est rare doit relire au plus les documents de ce mot ;
  - puis l'égalité des réponses avec et sans positions, par `test_light_profile` sur une copie
    (`LUCIVY_PROFILE_VARIANTS=positions,no_positions`), et le temps de la requête longue de l'archiviste (60 s
    aujourd'hui) relevé avant et après, sous `poste mesure` ;
  - puis `-p lucivy-core`, `--lib`, ground-truth.
- **Taille** : deux à trois jours.
- **Bloqué par** : rien. Prévenir l'orchestration avant de commencer, à cause de la place que prend la copie et du
  temps de mesure.

### Ce que Codex ne fait pas

- **Le pré-passage des ids vivants**
  (`docs/lucivy/tickets/2026-11-14-le-pre-passage-des-ids-vivants-relit-les-postings.md`) : l'orchestration a
  décidé de le garder tel quel le 14 novembre. Rien à faire sans un mot neuf.
- **Le poids du plein texte du dépôt**
  (`docs/lucivy/tickets/2026-11-13-le-plein-texte-du-depot-pese-cent-fois-ses-sources.md`) : la part lucivy est
  faite (4.3.8, maintenance). Les exclusions de fichiers sont chez recherche. La compression du texte stocké et le
  zstd à dictionnaire entraîné changent le format : une 4.4, au choix de Lucie.
- **Les mots vides par langue** : lucivy n'en a pas. `RAW_TOKENIZER` découpe le camelCase et met en minuscules, sans
  liste de mots vides. Le ticket est chez codeparsers
  (`docs/codeparsers/tickets/2026-11-15-la-liste-des-mots-vides-de-rust-est-une-copie-de-celle-de-javascript.md`).
- **Les `cx-lucivy-*` du 13 novembre** (barrière de shutdown B et C) : leur passation est
  `docs/lucivy/13-11-2026/99-passation-week-end.md`. Rien n'a bougé depuis : C n'est ni compilé ni mesuré. Pour le
  reprendre, il faut un mot neuf de l'orchestration.

## 3. Ce qui attend Lucie (Codex n'y touche pas)

- **Tout tag de lucivy**, donc toute publication : la 4.3.10 de l'entrée 5. Codex prépare et dit « prête ». Le tag
  se pose seulement sur le mot de Lucie, dit dans la session.
- **Honorer `indexed: false`** sur un champ texte (entrée 4) : un choix d'API.
- **Toute 4.4** : compression du texte stocké, zstd à dictionnaire entraîné, changement de format.
- **Le tri des worktrees et branches anciennes**, aux deux postes : la liste est dans
  `docs/lucivy/15-11-2026/01-rapport-de-session.md`, § 7. Le target `target-lucivy-session` de luciepc (26 Go) est
  gardé jusqu'à son mot.

## 4. Règles du périmètre

- **Anglais pour tout code neuf** : identifiants, options, clés de config et de JSON, noms de tests, messages
  d'erreur et d'avertissement. **Français** pour la prose des docs, les commits, les commentaires et les tickets.
  L'existant ne se renomme pas en lot : on ajoute un alias quand on y touche.
- **Un worktree et un target à Codex**, branches préfixées `cx-` dans lucivy comme dans rag3db. Jamais l'arbre
  de Lucie, jamais le target d'une autre session.
- **Côté rag3weaver**, la passe et le push suivent `docs/procedures/push-d-un-lot.md` :
  - par `poste push` et la garde ;
  - **jamais sans le « à toi » de l'orchestration** ;
  - le résumé est écrit par la passe elle-même, jamais à la main.
  - Des docs seules se poussent sans passe, mais jamais entre un « à toi » et son « poussé ».
- **Rouge d'abord** : chaque témoin est vu rouge avant le correctif, et son rouge est dit dans le commit ou la PR.
  **Une suite non jouée est rouge**, et un test qui saute n'est pas vert.
- **Un défaut connu non corrigé reçoit un ticket** sous `docs/lucivy/tickets/`, pas une ligne dans un message.
  Fermer un ticket, c'est `git mv` vers `docs/tickets/closed/`. Ensuite
  `python3 extension/rag3weaver/scripts/check_tickets.py` doit passer.
- **L'identité git** : Lucie Defraiteur, adresse gmail personnelle, jamais l'adresse pro. Aucune mention d'un
  assistant dans les commits ni dans les PR.
- **`gh`** : le compte `L-Defraiteur`. Si `gh auth status` montre le compte pro actif en semaine, ne pas l'employer
  et ne pas basculer sans le mot de Lucie : les pushes par ssh suffisent, et la PR se demande à l'orchestration.
- **npm** : l'organisation est `luciformresearch`. lucivy publie par la CI du tag, jamais à la main.
- **Pas de force push**, pas d'effacement de branche distante. Une branche rebasée se pousse sous un nouveau nom.
- **Les lourds** passent par `~/.cache/rag3weaver-build/poste lourd`, et une mesure par `poste mesure`. Toujours
  laisser deux cœurs libres.
