# codeparsers — passation pour Codex (16 novembre 2026)

Pour une session Codex neuve, sans mémoire des échanges : tout ce qu'il faut est ici, dans les tickets et dans
les pages citées. Codeparsers est un **sous-module** de rag3db (`extension/rag3weaver/codeparsers`, dépôt à part
`L-Defraiteur/codeparsers`) : un lot touche presque toujours les deux dépôts, le parseur d'un côté, le rendez-vous
de rag3weaver (`extension/rag3weaver/src/code.rs`) de l'autre.

## 1. État

**Sur master** (rag3db `ab685ed779` et ses suivants ; pointeur codeparsers `bd13385`, aussi la tête de la
branche `master` du dépôt codeparsers) : `REGLES_DE_L_ANALYSE = 7` (`extension/rag3weaver/src/code.rs`). Les
sept montées, dans l'ordre (chacune a son ticket fermé sous `docs/tickets/closed/`) :

1. la base (l'écrivain des générations, ED) ;
2. la pile de la résolution d'arêtes : une variante d'enum lit son type par le chemin ; un appel de méthode
   std sur une locale sans type lu ne se relie pas par le nom entre fichiers (listes `data/std_methods/`,
   générées depuis le poste) ; une chaîne de méthodes du projet se type (`DeferredType::OfType`) ; un appel C
   vers l'API C se voit (les `#include` d'une source C, la garde `extern "C"` de rag3db.h) ;
3. une méthode héritée se trouve par le type dérivé (`base_types`, `choose_inherited`,
   `RAG3WEAVER_INHERITANCE_DEPTH`) ;
4. le membre d'une bibliothèque de provenance sûre (`builtin`, `dependency`) ne prend pas de rendez-vous ;
   un membre lu au niveau du fichier garde son propriétaire ;
5. une locale d'un motif de tuple et une locale redéclarée lisent leur type (`DeferredBindings`) ;
6. deux homonymes d'une même ligne gardent deux uuid et deux clés (les octets du nœud) ;
7. le retour d'une fonction définie plusieurs fois se lit par celle que le fichier importe (`ReturnOf.module`).

**En branche, non poussé** : les mots vides de Rust tirés de la doc de std (entrée 1 ci-dessous). Branche rag3db
`codeparsers/rust-stop-words`, tête `044c7d4250` ; branche codeparsers `rust-stop-words`, tête `b48da05`. Le
témoin rouge (`a_rust_call_named_like_a_javascript_keyword_is_a_reference`) est vu rouge puis vert, e2e_usages
38/38, e2e_code 57/57, codeparsers 68 jeux verts. **La sonde a trouvé un défaut que la liste masquait** (107
`std::env::var` reliés au `var` du projet) : à corriger avant le « prêt » (entrée 1).

**Pour ne pas casser** :

- **Le pointeur et le Cargo.lock** vont dans le même lot ; le sous-module se commite avec son dossier
  (`git commit -- extension/rag3weaver/codeparsers …`). Après chaque commit dans codeparsers, pousser sa
  branche **avant** le commit rag3db qui pointe dessus. Quand le lot atterrit, avancer la branche `master` de
  codeparsers jusqu'au pointeur, **en avance rapide seule** (`git push origin <sha>:refs/heads/master` après
  `git merge-base --is-ancestor origin/master <sha>`).
- **Un lot qui change une règle de l'analyse monte `REGLES_DE_L_ANALYSE`** de un, avec un commentaire daté au-dessus
  de la constante, et la nouveauté (`docs/nouveautes.md`) dit « Toutes les bases repassent une fois ». La garde
  `extension/rag3weaver/scripts/check_regles_de_l_analyse.py` le vérifie.
- **La sonde avant/après est la preuve d'une règle** : `extension/rag3weaver/tests/sonde_marque_nom.rs`
  (CONSUMES par marque, scopes sans DEFINED_IN, et un TSV par couple). Corpus : l'arbre figé
  `~/git_workspaces/rag3db-garde`, par racines de liens symboliques (`rag3weaver/src`, `src/` du moteur, et ce
  que le lot touche). « Avant » = codeparsers et `code.rs` de master, « après » = la branche ; comparer les TSV
  couple par couple et **lire à la main** ce qui part et ce qui naît (un échantillon de 15 à 25). Une règle qui
  ôte des liens justes se resserre avant le « prêt ». Plus de 2 000 fichiers en un lot dépasse l'enveloppe de
  32 Go : découper les racines.
- **`tests/probe_unique_definer_misses.rs`** liste les mentions C/C++ à définisseur unique restées sans arête
  (pour la définition liée à l'en-tête).
- Le reste (les commandes, les pièges de zsh, les bancs A/B à luciepc) : le rapport
  `docs/codeparsers/15-11-2026/01-rapport-du-15-novembre.md` et la procédure `docs/procedures/push-d-un-lot.md`.

## 2. Travail pour Codex, dans l'ordre

Chaque entrée : un ticket réel, ce qu'il faut faire, la preuve, une taille, ce qui bloque. La commande de la
passe est la même pour toutes (section 4).

### Entrée 1 — Finir les mots vides de Rust (2 à 3 h)

- **Tickets** : `docs/codeparsers/tickets/2026-11-15-la-liste-des-mots-vides-de-rust-est-une-copie-de-celle-de-javascript.md`
  et `docs/codeparsers/tickets/2026-11-16-un-chemin-de-la-bibliotheque-standard-se-relie-a-un-homonyme.md`.
- **Où** : reprendre les branches `codeparsers/rust-stop-words` (rag3db) et `rust-stop-words` (codeparsers),
  rebasées sur master sous un nom neuf (`cx-rust-stop-words`). Ce qui est fait : `data/stop_words/generer.py`
  (les pages `keyword.*` de la doc de std 1.98.1, sans le mot-clé faible `union`, plus `Some`/`None`/`Ok`/`Err`
  des pages d'`Option` et de `Result`), `data/stop_words/rust.toml` (43 noms), le chargeur
  `standard::stop_words_of`, et `rust_scope_extraction_parser.rs` qui le lit.
- **Ce qui reste** : le second ticket. Dans `code.rs`, à la collecte du rendez-vous, à côté de la règle
  `externes_sures` (chercher « A member of an external library is not a project homonym ») : une référence
  qualifiée par un **chemin** dont le premier segment est une bibliothèque de provenance sûre ne prend pas de
  rendez-vous. Témoin rouge dans `tests/e2e_usages.rs` (`std::env::var("X")` contre un `pub fn var()` ailleurs).
- **La preuve** : la sonde sur `rag3weaver/src` et le moteur. Avant le correctif du chemin, elle donnait +389
  couples dont 107 `var` faux ; après, les `default` (221, `GenOptions::default()`… vers les `impl Default`) et
  `from` (41) restent, les `var` partent. Puis la nouveauté, `REGLES_DE_L_ANALYSE = 8`, la passe.
- **Ne pas toucher** : `RUST_BUILTIN_IDENTIFIERS` (une autre liste).

### Entrée 2 — Les mots vides de Python et de C (3 h)

- **Ticket** : le même que l'entrée 1, « Le même relevé pour Python et C ».
- **Quoi** : `IDENTIFIER_STOP_WORDS` de `python_scope_extraction_parser.rs` et `C_STOP_WORDS` de
  `c_scope_extraction_parser.rs` reprennent la liste de JavaScript. En données, comme Rust : Python depuis
  `keyword.kwlist` et `keyword.softkwlist` de l'interpréteur (les mots-clés doux `match`, `case`, `type`, `_`
  sont des identifiants légaux : les écarter, comme `union` en Rust) ; C depuis les jetons anonymes alphabétiques
  de `node-types.json` de tree-sitter-c (`~/.cargo/registry/src/*/tree-sitter-c-0.24.*/src/node-types.json`).
  Étendre `generer.py` et `stop_words_of`.
- **La preuve** : un témoin par langue (un scope nommé d'un mot JavaScript, `default` ou `from`, est référencé),
  la sonde sur `tools/python_api` et les scripts Python de `experiments/`, et sur `src/` pour C.

### Entrée 3 — Une arête qui dit comment elle a été résolue : les réexportations (4 h)

- **Ticket** : `docs/codeparsers/tickets/2026-10-04-une-arete-ne-dit-pas-comment-elle-a-ete-resolue.md`.
- **Quoi** : la marque `resolution` existe et `usages`/`impact` filtrent ; restent les réexportations et les
  mesures à rejouer (le ticket le dit). Lire le ticket en entier, rejouer ses mesures avec la sonde, écrire le
  reste.

### Entrée 4 — Les receveurs sans type : le reste (1 jour)

- **Ticket** : `docs/codeparsers/tickets/2026-10-04-des-receveurs-restent-sans-type.md`, section « Reste ».
- **Quoi** : les paramètres de fermeture (`v.iter().map(|x| x.f())`), l'élément d'un itérateur (garder
  `Iterator<T>` dans le pelage, `receveur::methode`), C++ `for (auto& x : v)`. Côté codeparsers :
  `scope_extraction/receveur.rs` (`typed_bindings_full`, `methode`), `usage.rs`. Une règle par lot, chacune avec
  son témoin et sa sonde ; garder la règle du 11 octobre (une locale à différé non lu n'est jamais reliée).

### Entrée 5 — La définition liée à l'en-tête : ses limites (1 jour)

- **Ticket** : `docs/codeparsers/tickets/2026-10-25-une-definition-cpp-se-lie-a-sa-declaration-d-en-tete.md`.
- **Quoi** : la mesure des manques à définisseur unique et la recherche par les bases sont faites (sections du
  16 novembre). Restent : le conteneur comparé par son dernier segment (deux `Transformer` de deux namespaces),
  les surcharges non distinguées, et **la mesure à l'oracle des arêtes « entete »** (`scripts/oracle_lsp.py cpp`,
  clangd, à luciepc). Commencer par l'oracle : il dit si la règle « entete » doit partir.
- **Bloque** : clangd et `compile_commands.json` à luciepc (le ticket dit lesquels).

### Entrée 6 — L'oracle qui cherche la cible au lieu du nom mentionné (4 h)

- **Ticket** : `docs/codeparsers/tickets/2026-11-11-l-oracle-cherche-la-cible-au-lieu-du-nom-mentionne.md`.
- **Quoi** : la branche rag3db `codeparsers/oracle-mention` (985d91d88) fait interroger le nom mentionné ;
  restent les témoins « alias » et « instance » et la ligne agrégée des sites. Reposer la branche sur master
  sous `cx-…`, écrire les deux témoins, rejouer les 415 liaisons.

### Entrée 7 — Relire les manifestes quand l'un d'eux change (4 h)

- **Ticket** : `docs/codeparsers/tickets/2026-11-14-relire-les-manifestes-quand-l-un-d-eux-change.md`.
- **Quoi** : `reingest_file_retenu` / `reingest_files_retenu` (`src/code_sync.rs`) relisent la note
  `manifests_read` au lieu des manifestes : quand le fichier réindexé **est** un manifeste (`Cargo.toml`,
  `package.json`, `pyproject.toml`, `requirements*.txt`, `vcpkg.json`, `go.mod`…), relire les manifestes et
  réécrire la note. Témoin e2e : ajouter une dépendance à un `pyproject.toml`, réindexer ce seul fichier, la
  provenance d'un import passe de `inconnue` à `dependency`.

### Entrée 8 — Une crate dont la bibliothèque porte un autre nom (3 h)

- **Ticket** : `docs/codeparsers/tickets/2026-11-14-une-crate-dont-la-bibliotheque-porte-un-autre-nom.md`.
- **Quoi** : la table des renommages en données (comme pour Python) : `rust-crypto` → `crypto`. Côté codeparsers,
  `data/imports/rust.toml` ; côté rag3weaver, `src/manifestes.rs`. Témoin : `use crypto::…` avec `rust-crypto`
  déclaré → `dependency`.

### Entrée 9 — Rapprocher un port vcpkg de son `find_package` (3 h)

- **Ticket** : `docs/codeparsers/tickets/2026-11-15-rapprocher-un-port-vcpkg-de-son-find-package.md`.
- **Quoi** : un petit script qui tire des `ports/<port>/usage` du registre `microsoft/vcpkg` la table port →
  paquet CMake, en données avec sa source, son commit et sa date en tête ; `manifestes.rs` la lit.
- **Ne pas** : deviner le dossier d'en-têtes d'un port (ticket suivant).

### Entrée 10 — Un fichier Rust qui s'analyse lentement (1 jour, mesure)

- **Ticket** : `docs/codeparsers/tickets/2026-10-11-un-fichier-rust-s-analyse-plus-lentement-a-la-ligne.md`.
- **Quoi** : mesurer un second gros fichier Rust avant de conclure ; suspects nommés dans le ticket
  (`typed_bindings_full`, `deferred_bindings` — **ce dernier est devenu situé le 16 novembre**, à remesurer).
  Pas de perf/valgrind sur les postes : échantillonner par `eu-stack` (voir le ticket).

### Entrée 11 — Une classe derrière une macro d'export (4 h, basse priorité)

- **Ticket** : `docs/codeparsers/tickets/2026-11-15-une-classe-derriere-une-macro-d-export-n-a-pas-de-scope.md`.
- **Quoi** : aucune classe du moteur n'est touchée (le compte est au ticket) ; la seule perdue est
  `nlohmann::basic_json` (third_party). La voie : lire la tête (`class <nom>` après une macro seule sur sa ligne)
  sous l'arbre fautif, dans `cpp_scope_extraction_parser.rs`. Témoin sur la fixture `tests/fixtures/fmt/format.h`.

### Entrée 12 — Le nettoyage du parent qui invalide un worktree de sous-module (2 h, procédure)

- **Ticket** : `docs/codeparsers/tickets/2026-11-11-le-nettoyage-du-parent-invalide-un-worktree-de-sous-module.md`.
- **Quoi** : une garde dans la procédure de nettoyage (avant `git worktree remove` d'un parent, vérifier qu'aucun
  worktree de sous-module n'y est rattaché) ; à écrire dans `docs/procedures/` et à proposer à l'orchestration.

### Bloqué, à ne pas commencer

- **Le dossier d'en-têtes d'un port vcpkg** (`docs/codeparsers/tickets/2026-11-15-le-dossier-d-en-tetes-d-un-port-vcpkg-n-est-ecrit-nulle-part.md`) :
  aucune source ne le dit ; une table faite à la main a été refusée. Attendre une source.
- **Le genre d'une méthode définie dans un autre fichier**
  (`docs/codeparsers/tickets/2026-11-15-le-genre-d-une-methode-definie-dans-un-autre-fichier.md`) : il se décide
  à la publication, par l'écrivain des générations ; c'est la file d'ED, pas celle-ci.

### Après chaque montée de `REGLES_DE_L_ANALYSE`

Demander au banc la mesure sur la vraie base du dépôt (deux index neufs du même arbre, avant/après, sans
embarqueur) : scopes sans DEFINED_IN, CONSUMES par `resolution`. Ce qu'elle révèle devient un ticket ici.

## 3. Ce qui attend Lucie

**L'import ES externe** (`docs/codeparsers/tickets/2026-11-11-une-mention-d-un-import-es-externe-n-a-pas-de-cible-a-elle.md`) :
le compte est fait (+6,5 à +9,2 % de scopes pour une sorte `import_binding`) ; la décision est à Lucie. Codex n'y
touche pas.

## 4. Les règles du périmètre

- **Identifiants, options, clés, noms de tests et de sondes en anglais** pour tout code neuf ; la prose, les
  commits et les tickets en français. Pas de renommage de l'existant.
- **Un worktree et un target à soi** (`~/.cache/rag3weaver-build/<nom>`), jamais l'arbre de Lucie
  (`~/git_workspaces/rag3db`) ; des branches `cx-…` dans les deux dépôts ; jamais de force push, jamais de
  branche distante effacée.
- **Rouge d'abord** : chaque correctif a un témoin vu rouge sur le code d'avant (retirer la correction le temps
  d'un passage si besoin), puis vert. **Une suite non jouée est rouge**, et un test sauté n'est pas vert.
- **La passe** : `~/.cache/rag3weaver-build/passe_epinglee.sh <worktree relatif à ~/git_workspaces>
  <dossier de journaux, un nom> <filtre du témoin>`, sous
  `POSTE_MEM_MAX=32G ~/.cache/rag3weaver-build/poste lourd python3 scripts/confiner_validation.py lancer
  --delai 14400 -- bash …`, avec `CIBLE=<son target>`, `SUITE_DU_TEMOIN=e2e_usages`,
  `SUITES_EN_PLUS=e2e_postgres` ; elle ajoute d'elle-même `e2e_code_sync` quand `code.rs` change. Dernière ligne
  du `resume.log` : `VERT`.
- **Le push** : par `docs/procedures/push-d-un-lot.md`, jamais sans le « à toi » de l'orchestration : la garde
  `~/.cache/rag3weaver-build/push_garde.sh` sous `poste push <session> --`, dans un `if tail -1 … | grep -qx
  VERT`, en mode pile (`PILE_AVANT=<la base>`) pour tolérer des docs arrivées ; puis les trois contrôles
  (status vide, aucun rebase, `FETCH_HEAD` = `HEAD`) et « poussé » avec le hash.
- **Identité** : `luciedefraiteur@gmail.com` dans les deux dépôts (le sous-module a sa propre config : la
  poser), jamais l'adresse pro ; aucune attribution d'assistant dans les commits.
- **Un défaut connu non corrigé a un ticket** sous `docs/codeparsers/tickets/` (famille « produit » ou « moteur »,
  `extension/rag3weaver/scripts/check_tickets.py --lot origin/master` vert).
