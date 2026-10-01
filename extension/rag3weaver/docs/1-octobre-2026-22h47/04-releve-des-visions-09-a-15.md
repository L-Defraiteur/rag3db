# Relevé — docs de vision 09 à 15 et produits contre le code

**1er octobre 2026.** Relevé fait par un agent de repérage, en lecture seule, sur `mtg-experiments` (`3886a458e`). Annexe de [01 — la réconciliation des objectifs](01-reconciliation-des-objectifs.md). Chaque « absent » dit ce qui a été cherché. Ce relevé est une photographie : il n'est pas tenu à jour.

---

# Rapport de réconciliation : visions de fin août / début septembre 2026 face au code au 1er octobre 2026

J'ai lu les huit documents en entier. Toutes les vérifications sont faites dans `/home/lucied/git_workspaces/rag3db` (HEAD `3886a458e`, 2026-10-01). Chemins relatifs à `extension/rag3weaver/` sauf mention contraire.

**Ce qui se dégage :** la machinerie est en grande partie là : porte des commandes, `run`, `schema`, dialecte Postgres prouvé, cinq organes de backend, invariants de silence. Ce qui manque, c'est tout ce que les docs 09 et 10 appelaient « les rôles et ce que chacun possède », plus l'ingesteur (13) et le schéma comme artefact (14).

---

## Doc 09 — `docs/vision_roadmap_09_2026/09-trois-roles-et-une-seule-main.md` (30 août)

**Thèse.** Un humain intervient à trois moments : relever le niveau, valider un design, décider de ce qu'on garde. Ce sont ces moments qu'il faut transformer en rôles : design, contexte (« souffleur »), code. Seul le rôle code touche au code et porte les modes de permission, avec `auto` par défaut. La mémoire de l'agent de code est bornée « à la manche de design » pour qu'il ne redérive pas son objectif de ses propres traces.

| élément | § | état | preuve |
|---|---|---|---|
| Notion de **rôle** (design / contexte / code, + vision) | 2, 7 | absent | rien pour `enum Role`, `design`, `vision`, `persona` dans `src/agent.rs`, `src/chat.rs`, `src/tools.rs`, `src/bin/rag3weaver-chat.rs`. Le seul « role » est celui des tours (system/user/assistant). |
| Modes de permission réservés au rôle code | 2 | partiel | `src/commande.rs` : `Mode`, `Garde`. Ils s'appliquent à l'outil `run`, faute de notion de rôle. |
| `auto` comme mode par défaut | 2 | fait | `src/commande.rs` : `Mode` porte `#[default] Auto`, test `Mode::default() == Mode::Auto`. Monté avec `Mode::Auto` dans `src/agent.rs:225` (`mount_agent_services_on`). |
| Les rôles sans écriture n'ont pas d'outil qui écrit (propriété tenue par la surface d'outils) | 2 | partiel | le mécanisme existe : `NodeTypePolicy::Only` dans `src/dataflow/graph_tool.rs:464`. Aucune surface par rôle n'est définie. |
| Tempéraments (vision / design / code / contexte) dans les invites système | 2 bis | absent | rien pour `tempérament`, `obéissant`, `méfiant`, `souffleur` dans src, scripts ou templates |
| « Obéissant sur le but, pas sur les faits » écrit dans l'invite du code | 2 bis | absent | aucune invite par rôle |
| Boîte de réception entre deux tours | 3, 7 | fait | `src/agent.rs:941` `Agent::read_inbox` ; `events.rs::inbox_topic` |
| Trace cherchable (`search(target="Trace")`) | 3, 7 | fait | `src/dataflow/trace_nodes.rs:52` `TRACE_ENTITY = "Trace"` |
| Fraîcheur de l'index par fichier (`stale`) | 3, 7 | fait | `src/code_tools.rs:286` et `:525` (`ReadResult.stale`) |
| `agentCanAnswer` / `PeutRepondre` / `enum Manque` (`DejaDit`, `DansLeCode`, `DejaTente`, `Dehors`) | 3 | absent | rien pour `PeutRepondre`, `agentCanAnswer`, `agent_can_answer`, `enum Manque` |
| Jugement en sortie structurée, sans outil (`response_format`) | 3 | partiel | le mécanisme existe : `src/llm.rs:596` `GenOptions::with_response_format`. Aucun appelant pour ce jugement. |
| Pause du code par posture `WaitingForPeer`, interblocage détecté | 3, 7 | fait | `src/agent.rs:427` `PauseKind::WaitingForPeer` ; `src/postures.rs:95` `Postures::deadlocks` |
| Défaut « oui » de la porte ; plafond de 3 pauses puis escalade au design | 3 | absent | pas de porte, donc ni plafond ni escalade |
| Mesurer si le tour suivant cite ce qui a été injecté | 6 | absent | rien trouvé |
| `Absorb::Stale` et `Session::recall` | 4, 7 | fait | `src/session.rs:51` (`Stale { max_chars, after_turns }`), `:154` `recall`. Note : le défaut d'`Absorb` reste `Whole` (`session.rs:54`). |
| `Absorb::DernieresManches { n: 2 }` (coupe à la manche de design) | 4, 7 | absent | rien pour `DernieresManches` ni `Manche` |
| Paroles de design épinglées, jamais réduites | 4 | absent | rien trouvé |
| Artefacts par rôle (document de design, verdict « je valide », résumés, journal) | 5 | absent ; journal partiel | seul le journal des commandes existe : `run_nodes.rs::dossier_journaux`, `Atelier.journaux` |
| Gabarits de fin de session (`rapport-de-session`, etc.) | 5, 7 | absent | les familles sous `templates/` sont entities, patterns, queries, apps, backends ; rien pour `rapport-de-session` |
| Outil de recherche web | 3, 7 | absent | rien pour `web_search` ou `WebSearch` |
| Refus du design plus long que l'approbation | 6 | absent | pas de rôle design |

**Décisions de Lucie citées :**
- « nous on veut des agents qui se parlent, qui savent prendre des décisions »
- « c'est l'agent qui touche réellement à l'écriture du code qui est en auto / edit / standard ; l'agent de contexte ne fait que discuter avec l'agent de code ; l'agent de design sert d'user »
- tempéraments : vision « ambitieux · visionnaire · observateur » ; design « abstractionniste · optimiste · volontaire » ; code « attentif · minutieux · obéissant ». Celui du contexte est proposé par le doc, pas par elle.
- « il fait constamment quelques recherches dans les sessions, rappelle des choses relevantes à l'agent de code, ou fait des recherches dans le code indexé si c'est prêt, ou même deux ou trois grep, écoute l'agent parler et injecte au bon moment »
- « il pourrait jauger si le contexte semble suffisant pour répondre, et préférer mettre en pause l'agent de code pour relire / chercher l'historique des conversations ou chercher dans le code. Un outil `agentCanAnswer`, true/false + reason ; si false on le relance lui-même avec l'objectif de chercher du contexte. »
- « au moment où il décide, peut-être qu'il n'a que ce tool-là — sinon il va déjà chercher le contexte pour répondre, il le cherchera deux fois »
- « peut-être même pas un tool à ce moment-là, il a juste une sortie structurée possible ; c'est après, quand on le relance, qu'il a tous les tools de recherche »
- « Il a accès à toutes les conversations de chaque agent, connaît toute la hiérarchie, peut faire les outils de lecture qu'il veut. »
- « l'agent de code n'a que peu de vrais tours complets en mémoire, les anciens il voit un résumé assez court qu'il peut quand même consulter sur commande »
- « on peut se permettre les deux dernières paroles design — enfin les deux derniers vrais tours où il y a eu : parole design, vraies actions entreprises par l'agent code (donc tool calls), et réponse à nouveau design »
- « ça reste BEAUCOUP plus léger que ce que font actuellement les agents populaires — attendre 1 M de contexte par exemple »

---

## Doc 10 — `10-l-agent-vision-et-la-distance.md` (30 août)

**Thèse.** Un quatrième rôle, la vision, tient le *pourquoi*. Il stocke mot pour mot l'intention initiale de l'utilisateur et ne la perd jamais. Il découpe en tickets pour le design et tient un **registre de la distance** : ce qui répond, ce qui ne répond pas encore, ce qui s'est éloigné. Il est le seul à parler à l'humain. Le rôle est aujourd'hui tenu à la main (`docs/vision_roadmap_*`, `docs/issues/<date>/`).

| élément | § | état | preuve |
|---|---|---|---|
| Rôle vision (seul interlocuteur de l'humain) | 2 | absent | aucune notion de rôle (voir doc 09) |
| Vision initiale stockée mot pour mot, jamais réduite | 3, 4 | absent | rien trouvé |
| Registre de la distance (répond / pas encore / s'est éloigné, et quand) | 4, 7 | absent | rien trouvé |
| Découpage de la vision en tickets pour le design | 5 | absent | aucun outil ni type « issue » ou « ticket » dans `src/agent.rs`, `src/chat.rs`, `scripts/chat_app.py` |
| Dépassement explicite d'une vision, daté, les deux visions conservées | 6, 7 | absent | rien trouvé |
| Propositions « et si on faisait carrément ça ? » au retour des agents seulement | 6 | absent | rien trouvé |
| Contrepoids : la vision ne valide pas le code, n'écrit pas les designs | 6 | non vérifiable | pas de rôle |
| Écouter les conversations sans les interrompre (bus, curseurs par sujet) | 7 | fait | `src/events.rs` : `EventBus`, `run_topic`, `inbox_topic`, `subscribe_full` |
| Chercher ce qui a été tenté | 7 | fait | `trace_nodes.rs` (`Trace`) |
| Gabarits d'artefacts (`place` / `adopt`) | 7 | fait | `templates/tools/place.mmd`, `adopt.mmd` ; `src/template.rs` |

**Décision de Lucie citée :** « Un agent vision : il classe, crée des issues vision, et il stocke l'intention première de l'utilisateur. L'utilisateur dit un truc, vision stocke ça comme vision initiale — irréalisable honnêtement direct par l'agent de code, car souvent les utilisateurs veulent des trucs compliqués. Il perd jamais la vision initiale utilisateur. Il écoute les conversations et il note ce qui commence à répondre à la vision, et comment. Il découpe les features en tickets pour l'agent design. Il note aussi les nouvelles bonnes idées à proposer à l'utilisateur — « et si on faisait carrément ça ? » — au retour de tous les agents. »

---

## Doc 11 — `11-donner-des-commandes-a-un-agent.md` (30 août)

**Thèse.** Donner l'exécution de commandes à l'agent sans lui donner la machine. Une commande s'exécute par son argv, jamais par un shell. Il y a trois modes, une sentinelle enfichable qui rend un verdict structuré (décision, portée, fondement, faits, motif), et une liste blanche que la session se construit sans jamais l'élargir seule. L'irréversible n'est jamais mis en liste blanche.

| élément | § | état | preuve |
|---|---|---|---|
| Exécution par argv, pas de shell ; `sh -c` toujours explicite | 1 | fait | `src/commande.rs` : `Commande { programme, args, tuyau_entrant }` ; `observer()` lève `shell` |
| Ligne de commande réduite en argv par `codeparsers::shell` | en-tête | fait | `commande.rs:525` `Garde::juger_ligne` ; `codeparsers::shell::decomposer` |
| Modes `standard` / `approbation` / `auto` | 2 | fait | `commande.rs:386` `enum Mode` ; `Garde::juger` |
| Refus en `standard` « avec la liste » | 2 | fait | `Garde::juger` (`Mode::Standard`) joint `LECTURE_SEULE` |
| `trait Sentinelle { juger }` | 3 | fait | `commande.rs:247` ; `Garde::avec_sentinelle` |
| `Verdict { decision, portee, fondement, faits, motif }` | 3 | fait | `commande.rs:200` |
| `Decision` : Autorise / Demande / Refuse | 3 | fait | `commande.rs:158` |
| `Portee` : CetteFois / CetteCommande / CetteFamille / Toujours | 3 | fait | `commande.rs:169` |
| Portée dérivée du fondement (`JugeeInoffensive` plafonnée à `CetteCommande`) | 3 | fait | `Verdict::borner` |
| `Fondement` : Configuration / UtilisateurExplicite / DejaAccorde / JugeeInoffensive | 3 | fait | `commande.rs:187` |
| `Faits { ecrit, reseau, hors_domaine, irreversible, eleve, shell }` | 3 | partiel | `commande.rs:99` : il manque **`hors_domaine`**. Le domaine est contrôlé ailleurs, au cwd de l'`Atelier` (`ExecErreur::Atelier`, « sort du domaine ») ; `Contexte.domaine` est passé mais pas lu par `SentinelleDeBase`. |
| Session d'autorisations consultée avant la sentinelle | 4 | fait | `Autorisations::{retenir, couvre}` ; `Garde::juger` rend `DejaAccorde` |
| Famille = programme + premier sous-verbe | 4 | fait | `Commande::famille`, `MULTI_VERBES` |
| Irréversible, élevé ou shell restent toujours `CetteFois` | 4 | fait | `Verdict::borner` |
| Autorisations qui ne survivent pas à la session | 4 | fait | en mémoire seulement (`Mutex<BTreeSet>`) |
| Sentinelle de base, sans modèle | 5 | fait | `SentinelleDeBase` |
| Sentinelle à modèle (mode `auto`), contrainte par la même structure | 5 | absent | seule `SentinelleDeBase` implémente `Sentinelle` |
| `Toujours` écrit dans une configuration | 4, 6 | absent | `Toujours` ne fait que remplir l'ensemble des familles de la session ; aucun fichier de configuration (point marqué « à trancher ») |
| Demande à l'humain en mode `approbation` | 2, 6 | absent | `run_nodes.rs:94` passe toujours `accorde_par_l_utilisateur: false`, et un `Demande` rend « Commande en attente » sans canal de réponse. `UtilisateurExplicite` n'est atteignable par aucun chemin. |
| Délai d'expiration d'une portée | 6 | absent | le doc dit « aucune pour l'instant » |

**Décisions de Lucie citées :**
- « faut trouver maintenant comment lui donner accès aux commandes système, peut-être avec une allowlist de trucs qui n'ont pas besoin d'humain »
- « ne pas redemander 50 fois à l'user si ok pour lire la bdd si donné une fois »
- « peut-être en fait il crée sa propre allowlist, le classifieur, durant une session »
- Note d'en-tête : `auto` devient le mode de premier rang. Le code la cite sous la forme « auto serait le mode first class par défaut, de nos jours tout le monde fait ça » (`commande.rs`, doc de `Mode::Auto`).

---

## Doc 12 — `12-ce-qui-manque-a-l-agent-de-code.md` (30 août)

**Thèse.** C'est la feuille de route écrite par le modèle lui-même, dans cet ordre : exécuter, supprimer/déplacer un fichier, voir le schéma du graphe, inspecter un gabarit avant de le poser, corriger deux défauts de forme (`edit` sans `oneOf`, pied de page de `read` à décoder). Deux choses sont gardées malgré la critique : `grep` avec ses paramètres de graphe, `rerank` en entier.

| élément | § | état | preuve |
|---|---|---|---|
| 1. Exécuter (le verbe) | 1 | fait | `templates/tools/run.mmd`, `run_bg.mmd`, `wait.mmd` ; `src/dataflow/run_nodes.rs` `RunCommandNode` ; journal complet gardé |
| 2. Supprimer un fichier | 2 | absent | les outils publiés sont grep, edit, adopt, link_snapshot, search*, select_structured, ingest_snapshot, wait, list, place, read, run, run_bg, schema. Rien pour `delete_file`, `remove_file`, `DeleteFile`. Seul contournement : `rm` via `run`, sans désindexation. |
| 2. Déplacer ou renommer un fichier | 2 | absent | même recherche (`move_file`, `rename_file`) |
| 3. Voir le schéma du graphe | 3 | fait | `templates/tools/schema.mmd` (`schema(target)`, rendu Mermaid) |
| 4. Inspecter un gabarit avant de le poser | 4 | partiel | `src/template.rs:424` `prepare_entity` (« Ce qu'on va poser, avant de le poser ») et `template::read` existent, mais aucun outil ne les expose. `place` rend toujours le résultat après la pose. |
| 5a. Exclusivité `content` / `old`+`new` dans le schéma de `edit` | 5 | partiel | contrôle à l'exécution dans `code_nodes.rs:643` (`EditFileNodeFactory`). Le schéma n'a toujours pas de `oneOf` (l'exclusivité n'est écrite que dans la description de `edit.mmd`). Par ailleurs `openai_llm.rs:809` rejette `oneOf` côté Vertex. |
| 5b. `read` : `has_more` / `next_offset` structurés | 5 | partiel | `ReadResult.has_more` existe et passe en JSON avec `ToolFormat::Json`. Pas de `next_offset`. Le format par défaut (`Markdown`) rend toujours le pied de page texte « Use offset=… » (`code_tools.rs` `to_markdown`). |
| Garder `grep` avec ses paramètres de graphe | 6 | fait | `grep.mmd` : `relation`, `relation_limit` |
| Raison de `rerank` en entier visible du modèle | 6 | partiel | la description envoyée (`search.mmd:6`) explique quand l'utiliser ; la justification « entier plutôt que booléen » reste dans les commentaires `%%` |

**Décisions de Lucie citées :** aucune dans ce doc. Les citations sont du modèle (« Je code à l'aveugle », etc.).

---

## Doc 13 — `13-avaler-une-base-existante.md` (30 août)

**Thèse.** Un ingesteur lit une base étrangère (`information_schema`, clés primaires et étrangères) et en déduit les tables de liaison. Il **propose** un brouillon de gabarits d'entités où chaque décision porte sa `Provenance` (Declaree / Deduite / Devinee). C'est la rampe d'accès des projets existants. Ordre : prouver Postgres, puis lire et proposer, puis chercher dans la base d'origine, puis la boucle sur un domaine existant.

| élément | § | état | preuve |
|---|---|---|---|
| Prérequis : chemin Postgres **éprouvé** (le doc constatait « zéro test E2E ») | 7 | fait | `tests/e2e_postgres.rs` : environ 21 tests (schéma, ingestion, vecteur, plein texte, hybride, relations, cellules, filtre, checkpoints, marque d'eau, confiance…) |
| Lire tables, colonnes et types (`information_schema`) | 2 | absent | `information_schema` n'apparaît que dans un test (`e2e_postgres.rs:237`) ; aucun lecteur de schéma étranger dans src |
| Lire clés primaires et étrangères | 2 | absent | rien pour `FOREIGN KEY` ou `foreign_key` dans src |
| Règle de la table de liaison (deux clés étrangères = arête) | 2 | absent | rien pour `junction` ou « table de liaison » |
| Inférence des colonnes de texte cherchable (nom, type, longueur) | 3 | absent | rien trouvé |
| Inférence de clés étrangères par le nommage (`user_id` → `users.id`) | 3 | absent | rien trouvé |
| `enum Provenance { Declaree, Deduite, Devinee }` | 4 | absent | rien pour `Declaree`, `Devinee`, `enum Provenance` |
| Sortie en gabarits d'entités du catalogue | 5 | absent pour l'ingesteur | l'aval existe : famille `entity`, `place`, `adopt` |
| Ingestion sélective par table, incrémentale | 6 | absent | pas d'ingesteur |
| Connexion en lecture seule au niveau du compte | 6 | partiel | `Rag3dbConnection::read_only` (`rag3db_connection.rs:72`). Rien de tel côté `postgres_connection.rs`. |
| Secrets hors de l'agent et des traces | 6 | non vérifiable | pas d'ingesteur |
| Seuil ou catégorie « ignoré » pour les tables-flux | 6 | absent | rien trouvé |
| Étage 2 : recherche hybride sur la base d'origine via `DbConnection` | 7 | partiel | la recherche hybride Postgres marche sur des tables créées par rag3weaver (`PostgresSearchBackend`, `e2e_postgres::l_hybride_fusionne`), pas sur un schéma étranger |
| Étage 3 : boucle étrange sur un domaine existant | 7 | absent | rien trouvé |

**Décisions de Lucie citées :**
- « un genre d'ingesteur qui convertit une base étrangère en compat rag3weaver, parmi les backends supportés, détecte les tables de liaison etc., et tente de reconstruire un graphe »
- « une boucle étrange qui ne sert que kuzu, ça ne marche pas pour les projets en prod réels »

---

## Doc 14 — `14-le-schema-comme-artefact.md` (30 août)

**Thèse.** Les migrations déclarées (`dataflow/migrations.rs`, Mermaid) sont déjà des artefacts rejouables. Le chemin induit ne laisse aucune trace : `register_entity` (avec ses `ALTER`), `migrate_scope_columns`, `poser_index`. Il faut capturer l'**intention** (et non le SQL) au passage, en Mermaid, pour rejouer, relire avant, et constater l'écart entre configuration et base. Un dialecte doit aussi savoir décrire fonctions RPC, politiques RLS et droits (Supabase).

| élément | § | état | preuve |
|---|---|---|---|
| Lanceur de migrations : status, pending, dry_run, rollback, check_reversible | 1 | fait (existait déjà) | `src/dataflow/migrations.rs:322`, `:363`, `:381` (`dry_run`, `DryRunPlan`), `:555`, `:645` |
| Chemin induit toujours sans trace | 2 | constat toujours vrai | `catalog.rs:1086` `register_entity`, `:1488-1498` `ALTER TABLE`, `:3604` `migrate_scope_columns` (`eprintln!`), `:1257` `poser_index` |
| Capturer l'intention (`ColumnDef` / `ColumnType`) au passage, en artefact Mermaid | 3, 7 | absent | rien pour une capture, un journal de DDL ou d'intention dans `catalog.rs`, `dialect.rs`, `schema.rs`, `migrations.rs` |
| Plan sans exécution pour `register_entity` | 4 | absent | pas de `fn plan` ni d'équivalent `dry_run` sur le chemin induit |
| Constater l'écart configuration ↔ base | 4 | absent | rien pour `drift`, `SchemaDiff`, `ecart` dans le schéma |
| Fonctions RPC émises par le dialecte (ex. `rag3weaver.chercher(vector, k)`) | 5 | absent | seule fonction SQL émise : l'auxiliaire `rag3weaver.sans_accents` (`dialect.rs:1411`), pas une RPC de recherche |
| Politiques RLS et droits émis par le dialecte | 5 | absent | rien pour `ROW LEVEL SECURITY`, `CREATE POLICY`, `GRANT` |
| Méthode de dialecte « de quoi est fait un schéma » (tables, index, fonctions, politiques, droits) | 5, 7 | absent | `SchemaDialect` (`dialect.rs:87`) a `alter_add_column*` etc., rien de cet ordre |
| « Supabase-compatible » : aspiration et non fait | 5 | constat inchangé | `dialect.rs:1366` porte toujours ce libellé |
| Points non tranchés : qui écrit l'artefact, rattrapage, granularité, langage des fonctions | 6 | non tranchés | rien dans le code |

**Décisions de Lucie citées (sous forme de questions) :**
- « les migrations devraient peut-être produire des artefacts rejouables qu'on puisse commit »
- « pas de RPC à créer automatiquement ? »

---

## Doc 15 — `15-le-moteur-cesse-d-etre-mono-backend.md` (3 septembre)

**Thèse.** Le catalogue repose sur quatre organes (`DbConnection`, `SchemaDialect`, `SearchBackend`, `BlobStore`) plus un magasin de checkpoints. Les cinq existent pour rag3db et pour Postgres. Prouver Postgres a montré que les fuites venaient de kuzu, d'où « Neo4j en dernier ». Deux invariants : « le silence est le défaut, pas l'erreur », et « une recherche doit pouvoir dire qu'elle ne trouve rien de probant ». En concurrence : plusieurs lecteurs oui, plusieurs processus écrivains non.

| élément | § | état | preuve |
|---|---|---|---|
| `DbConnection` rag3db + Postgres | 1 | fait | `rag3db_connection.rs:244`, `postgres_connection.rs:339` |
| `SchemaDialect` rag3db + Postgres | 1 | fait | `dialect.rs:725` `Rag3dbDialect`, `:1369` `PostgresDialect` |
| `SearchBackend`, plein texte facultatif | 1 | fait | `search_backend.rs:116` (`sert_le_plein_texte`, `honore_le_filtre`, `text_search`) ; impl. `rag3db_search_backend.rs:28`, `postgres_search_backend.rs:65` |
| Choix du moteur de plein texte (`MoteurTexte` Auto / Lucivy / Natif) | 1, 3 | fait | `search_backend.rs:78` ; `Catalog::set_moteur_texte`, appelé dans `backend.rs:513` |
| `BlobStore` rag3db + Postgres | 1 | fait | `cypher_blob_store.rs:107`, `postgres_blob_store.rs:28` (trait `lucivy_core::blob_store`) |
| Magasin de checkpoints rag3db + Postgres | 1 | fait | `dataflow/checkpoint_store.rs:218` `CypherCheckpointStore`, `:639` `PostgresCheckpointStore` ; tests `la_reprise_apres_incident_tient_sur_postgres`, `le_catalogue_monte_le_magasin_de_checkpoints` |
| Neo4j en dernier | 2 | conforme (absent) | aucun backend Neo4j dans le crate ; une mention en commentaire (`checkpoint_store.rs:1007`) |
| « Une absence se nomme » : backend qui déclare ce qu'il ne garantit pas | 3 | fait (partiel en couverture) | `honore_le_filtre` par défaut `false`, `MoteurTexte::Natif` avec erreur nommée, `acces.rs` « jamais de repli silencieux » ; test `une_recherche_stricte_dit_ce_quelle_ne_peut_pas_tenir` |
| Marque « rien de probant » qui nomme son signal | 3 | fait | `search.rs:1975-1986` `marquer_la_confiance` ; test `la_marque_de_confiance_nomme_son_signal` |
| Seuil mesuré (et non copié de ragkit) | 3 | fait (test de mesure) | `e2e_postgres::ou_vit_la_frontiere_entre_le_vrai_et_le_bruit` |
| Plusieurs lecteurs : marque d'eau d'ingestion | 4 | fait | `catalog.rs:223-231` (`marque_posee`, dette publiée) ; test `la_marque_deau_traverse_la_frontiere` |
| Reprise sur refus transitoire | 4 | fait | `Rag3dbConnection::read_only_patient` (`rag3db_connection.rs:86`) |
| Chemin de lecture explicite direct / relais, sans repli silencieux | 4 | fait | `src/acces.rs` : `Acces { Direct, Demon, Auto }`, `Chemin`, `Lecteur.par` |
| Plusieurs processus **écrivains** (verrous inter-processus) | 4 | absent (annoncé ouvert) | rien pour `flock` ou inter-processus. `rag3daemon` reste l'écrivain unique (`bin/rag3daemon.rs`, `F_WRLCK`). |

**Décision de Lucie citée :** « Une boucle étrange qui ne sert que kuzu ne sert pas les projets réels. »

---

## Doc racine — `docs/vision_roadmap_09_2026/09-les-produits-et-leur-empaquetage.md` (18 septembre)

**Thèse.** Quatre produits : l'agent local avec MCP (« la boucle étrange »), le moteur de workflows cloud multi-locataire, le plan local gratuit, et la bibliothèque (crate, roue Python, paquet npm). Le produit 1 tire les autres. Empaquetage « tout dans le binaire » : extensions liées statiquement, registres comme installeur, carte détectée au premier lancement, poids téléchargés avec consentement depuis HF. Ordre de Lucie : replier les KB en entités dérivées, puis l'heuristique du premier index, puis l'entrée produit.

| élément | § | état | preuve |
|---|---|---|---|
| P1 : agent local + démon d'embarquement partagé + serveur MCP | Produits | partiel | agent `src/agent.rs`, chat `src/chat.rs` et `bin/rag3weaver-chat.rs`, démon `bin/rag3weaver-embeddings.rs` et `src/daemon/embeddings.rs`. MCP = pont **Python** `scripts/serve_backend_mcp.py`, qui expose les outils déclarés d'un backend via le SDK MCP officiel. Pas de serveur MCP natif dans le binaire. |
| P2 : serveur cloud multi-locataire avec API | Produits | partiel | cellules `_org` / `_project` (`scope.rs`, `migrate_scope_columns`), `rag3daemon` (relais HTTP de la base, `--exposer`), `bin/rag3weaver-backend.rs` (hôte persistant). Pas d'API de service multi-locataire identifiée. |
| P3 : plan local gratuit (binaire du 1 avec API) | Produits | non vérifiable / absent | pas d'API produit distincte trouvée |
| P4 : bibliothèque crate / roue / npm | Produits | partiel | crate Rust (`Cargo.toml`, `crate-type = ["lib","staticlib"]`). Aucune liaison pyo3 ou napi pour rag3weaver. `tools/python_api` et `tools/nodejs_api` sont ceux du moteur ; maturin et napi n'apparaissent que dans `extension/lucivy`. |
| Extensions `vector`, `geo` liées statiquement en natif | Empaquetage | partiel | le mécanisme CMake existe (`extension/CMakeLists.txt`, `EXTENSION_STATIC_LINK_LIST`), mais la liste est vide par défaut (`extension/extension_config.cmake` : `#set(EXTENSION_STATIC_LINK_LIST fts)`, `BUILD_EXTENSIONS` vaut `vector;geo` en dynamique). Le `build.rs` de rag3weaver ajoute `-rdynamic` pour les extensions chargées. |
| Roues PyPI (maturin, 4 plateformes), npm façon esbuild + napi-rs, crates.io + `cargo binstall`, intégration continue | Empaquetage | absent | rien pour maturin, napi-rs ou binstall hors lucivy ; aucune CI de publication trouvée pour rag3weaver |
| Carte détectée au premier lancement ; heuristique 107m / 278m (~50 000 documents ou VRAM < 4 Go) | Empaquetage | partiel | `src/embedding_choice.rs` (`choose`, `recommended_model`, `FILES_THRESHOLD = 50_000`, `VRAM_FLOOR_BYTES = 4 Gio`), `regime::card_class` (`/sys/class/drm`), `Catalog::note_embedding_choice`. Mais `EmbeddingDaemon::serveur_for_repository` (`daemon/embeddings.rs:346`), seul appelant de `choose`, n'a **aucun appelant**, donc pas de branchement sur un chemin produit. `note_embedding_choice` n'est appelée que par des tests. |
| Poids hors paquet, consentement explicite, téléchargement HF dans `~/.cache/huggingface`, `HF_ENDPOINT` respecté, refus = plein texte seul | Empaquetage | absent | les poids granite se localisent par variables (`RAG3WEAVER_GRANITE_107M` / `_278M`, `bin/rag3weaver-embeddings.rs:63`) ; rien pour `HF_ENDPOINT` ou un consentement. `hf-hub` ne sert qu'à `candle_embedder` (feature optionnelle). |
| Pas de dépôt d'extensions ; `INSTALL` sans dépôt refuse et nomme `RAG3DB_EXTENSION_REPO` (cœur C++) | Ce qui en découle | fait | `src/include/extension/extension.h:65` (racine), `src/binder/bind/bind_extension.cpp:27-36` ; rien pour `extension.rag3db.com` dans `src/` ni `extension/` |
| Ordre 1 : replier les KB en entités dérivées | Non ouvert | fait | `src/derived_kb.rs` (`EntityConfig { derived }`) |
| Ordre 2 : heuristique du premier index | Non ouvert | partiel | voir plus haut : écrite et testée, non branchée |
| Ordre 3 : entrée produit « indexer ce dépôt » | Non ouvert | absent | rien pour `index_repo`, `index_repository` ou « indexer ce dépôt » ; `Catalog::ingest_code` (`code.rs:1252`) existe comme brique |
| Miroir des modèles sur luciformresearch.com | Empaquetage | absent | rien trouvé |

**Décisions de Lucie :** le doc ne cite pas de phrase entre guillemets. Il rapporte des « conclusions d'une discussion avec Lucie », dont « l'ordre de Lucie tient : replier les KB en entités dérivées, l'heuristique du premier index, puis l'entrée produit ». Le code cite par ailleurs son arbitrage du 7 septembre (`embedding_choice.rs`) : 278m par défaut, 107m si plus de ~50 000 documents ou carte faible ou absente.

---

## Ce qui était voulu et reste entièrement absent

Classement par importance apparente dans les docs.

1. **La notion de rôle** (vision / design / contexte / code), avec ses invites et tempéraments, et la surface d'outils propre à chaque rôle. Docs 09 §2, §7 et 10 §2 : « ce qui manque, ce sont les rôles ».
2. **`Absorb::DernieresManches`** (coupe de la mémoire à la manche de design) et **l'épinglage des paroles de design**. Doc 09 §4, présenté comme « l'idée la plus fine ».
3. **Le souffleur et `agentCanAnswer` / `PeutRepondre` / `Manque`**, avec défaut « oui », plafond de pauses et escalade au design. Doc 09 §3.
4. **Le registre de la distance** et **la vision initiale stockée mot pour mot**, avec **dépassement explicite daté**. Doc 10 §4, §6.
5. **L'ingesteur de base étrangère** : `information_schema`, clés, tables de liaison, `Provenance` Declaree / Deduite / Devinee, brouillon de gabarits, sélection par table. Doc 13 §2-§7 (étage 1).
6. **Le schéma comme artefact** : capture de l'intention du chemin induit, plan sans exécution de `register_entity`, constat d'écart configuration ↔ base. Doc 14 §3-§4.
7. **Supprimer / déplacer un fichier** comme outils de l'agent. Doc 12 §2, n° 2 de l'ordre du modèle.
8. **Les fonctions RPC, politiques RLS et droits émis par le dialecte Postgres** (Supabase). Doc 14 §5.
9. **Les gabarits de fin de session** (`rapport-de-session`, etc.). Doc 09 §5.
10. **Les tickets et issues produits par l'agent vision**, et les propositions « au retour des agents ». Doc 10 §5-§6.
11. **La sentinelle à modèle** pour le mode `auto`, et le **canal de demande à l'humain** (mode `approbation`, `UtilisateurExplicite`), ainsi que la **persistance de `Toujours` en configuration**. Doc 11 §4-§6.
12. **L'empaquetage** : roues PyPI, npm/napi-rs, crates.io + binstall, CI multi-plateforme, consentement et téléchargement HF des poids, `HF_ENDPOINT`, miroir. Doc racine 09.
13. **L'entrée produit « indexer ce dépôt »**. Doc racine 09, ordre n° 3.
14. **L'outil de recherche web**. Doc 09 §3, §7, « nouvelle dépendance ».
15. **Plusieurs processus écrivains** (verrous inter-processus). Doc 15 §4, annoncé comme restant ouvert.
16. **Le champ `Faits.hors_domaine`**. Doc 11 §3, seul champ du verdict non implémenté.
17. **La mesure « le tour suivant cite-t-il l'injection »**. Doc 09 §6.
