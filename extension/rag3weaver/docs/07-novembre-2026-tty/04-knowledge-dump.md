# Ce que la session tty a appris (7 novembre 2026)

- **Le flux de l'hôte** (`src/ide.rs`, et non `src/daemon_host/ide.rs`).
  - C'est un SSE brut, écrit par `request.into_writer()`, sans longueur ni découpage en morceaux. Les ids commencent à 1, le tampon garde 2048 événements, un battement part toutes les 15 s.
  - Sans curseur, l'abonnement commence à « maintenant ». Un client qui lance un run doit donc attendre l'en-tête 200 avant de poster : c'est le crochet `ouvert` de `flux::suivre`.
  - `Last-Event-ID: 0` rejoue tout le tampon.
- **Les droits d'un outil.** `ToolPolicy` (`src/backend_code.rs:233`) est sérialisable et lisible dans `describe()` par `self.manifest.tools[name].policy`.
- **Le bâti.**
  - La lib rag3weaver ne se bâtit pas sans `rag3db-native` (ticket du 7 nov.). Un client portable doit donc vivre dans un crate à part, en dépendance de test de rag3weaver pour ses e2e.
  - Un worktree neuf n'a pas `codeparsers` : il faut `git submodule update --init extension/rag3weaver/codeparsers` avant tout cargo.
  - La lib du magasin 04063ee06 est plus vieille que les sources du moteur : `run_e2e.sh` exige alors `RAG3WEAVER_MOTEUR_ANCIEN=1`. `code_tree::copy` est privée : un e2e recopie les gabarits lui-même.
- **Un faux équipier en e2e.** `DaemonHost::with_chat_model(...)` avec un `CallbackLlm`, et un `MockLlm` qui parle (un token par mot) et appelle `step_done`. Un seul pas suffit pour finir « fait sans plan ». `DaemonServer::new(server, n).serve(host, Some(jeton))` sert l'hôte sur un vrai port avec son jeton.
- **luciepc.** `~/.cache/rag3weaver-build/passe_epinglee.sh <worktree> <journaux> <filtre>` est la passe commune. Elle prend `SUITE_DU_TEMOIN`, `SUITES_EN_PLUS` et `CIBLE`, et sa dernière ligne de `resume.log` dit VERT ou ROUGE.
- **mosh ne passe aucune image** (kitty, sixel) et n'a la couleur vraie qu'à partir de la 1.4, des deux côtés. Tout ce qui compte se dessine donc en cellules.
