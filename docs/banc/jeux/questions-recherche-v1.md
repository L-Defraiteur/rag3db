# Le jeu de questions de la recherche, v1

22 novembre 2026. Jeu durable du banc, au oui de Lucie (11 h 50), pour mesurer la recherche de code de rag3weaver et
les choix de lucivy. Il est versionné : une question ne change jamais dans une version, on en crée une suivante.

**41 questions**, chacune avec son or :

| | nombre |
|---|---|
| français / anglais | 26 / 15 |
| phrases | 23 |
| noms | 2 |
| fautes de frappe | 5 |
| questions longues | 3 |
| pages | 6 |

- **L'or d'une définition** : son nom et la fin de son chemin. Le juge accepte le premier résultat dont le `name` est ce nom
  et le chemin finit ainsi.
- **L'or d'une page** : le fichier. Le juge accepte tout résultat de ce fichier.

Chaque or existe sur master au 20 novembre 2026 à 6 h (`c94bf03b6e`), donc dans la base de mesure du banc, indexée ce
jour-là.

Le juge est `juge_questions.py`, et la comparaison `compare_questions.py`, à luciepc sous `~/.cache/rag3weaver-build/`.
Ils donnent le rang de l'or sous la recherche complète et sous le BM25 seul, le rappel@10 par langue et par sorte, la
part des pages entières en tête du BM25 seul, et l'avertissement de lucivy.

Les lignes ci-dessous, à copier dans un fichier `.jsonl` :

```jsonl
{"id": "q01", "lang": "fr", "kind": "phrase", "query": "fusionner les résultats de plusieurs signaux de recherche", "gold": {"name": "FuseResultsNode", "path": "dataflow/generic_search_nodes.rs"}}
{"id": "q02", "lang": "en", "kind": "phrase", "query": "node that fuses result lists from several search signals", "gold": {"name": "FuseResultsNode", "path": "dataflow/generic_search_nodes.rs"}}
{"id": "q03", "lang": "fr", "kind": "typo", "query": "fusionage des resultats de plusieur signaux", "gold": {"name": "FuseResultsNode", "path": "dataflow/generic_search_nodes.rs"}}
{"id": "q04", "lang": "en", "kind": "name", "query": "FuseResultsNode", "gold": {"name": "FuseResultsNode", "path": "dataflow/generic_search_nodes.rs"}}
{"id": "q05", "lang": "fr", "kind": "phrase", "query": "le nœud de recherche plein texte BM25", "gold": {"name": "BM25SearchNode", "path": "dataflow/generic_search_nodes.rs"}}
{"id": "q06", "lang": "en", "kind": "phrase", "query": "BM25 full text search node over the index fields", "gold": {"name": "BM25SearchNode", "path": "dataflow/generic_search_nodes.rs"}}
{"id": "q07", "lang": "fr", "kind": "phrase", "query": "le nœud de recherche vectorielle", "gold": {"name": "VectorSearchNode", "path": "dataflow/generic_search_nodes.rs"}}
{"id": "q08", "lang": "en", "kind": "typo", "query": "vectr search nod", "gold": {"name": "VectorSearchNode", "path": "dataflow/generic_search_nodes.rs"}}
{"id": "q09", "lang": "fr", "kind": "phrase", "query": "la recherche par vecteurs creux", "gold": {"name": "SparseSearchNode", "path": "dataflow/generic_search_nodes.rs"}}
{"id": "q10", "lang": "fr", "kind": "long", "query": "comment le moteur décide-t-il de chercher dans l'index ou de balayer les fichiers quand l'index n'est pas prêt", "gold": {"name": "adaptive_search_decision", "path": "dataflow/generic_search_nodes.rs"}}
{"id": "q11", "lang": "en", "kind": "phrase", "query": "embed the query text before vector search", "gold": {"name": "embed_query", "path": "src/search.rs"}}
{"id": "q12", "lang": "fr", "kind": "phrase", "query": "les options d'une recherche, limite, filtres et signaux", "gold": {"name": "SearchOptions", "path": "src/search.rs"}}
{"id": "q13", "lang": "en", "kind": "name", "query": "SearchOptions", "gold": {"name": "SearchOptions", "path": "src/search.rs"}}
{"id": "q14", "lang": "fr", "kind": "phrase", "query": "fusionner les surlignages d'un résultat", "gold": {"name": "merge_highlights", "path": "src/search.rs"}}
{"id": "q15", "lang": "en", "kind": "phrase", "query": "diagnostics of a search explaining why a chunk was missed", "gold": {"name": "SearchDiagnostics", "path": "src/search.rs"}}
{"id": "q16", "lang": "fr", "kind": "phrase", "query": "le mode de la requête plein texte, contains, mots ou symbole", "gold": {"name": "BM25Mode", "path": "src/search.rs"}}
{"id": "q17", "lang": "fr", "kind": "phrase", "query": "balayer et supprimer les caches d'index morts", "gold": {"name": "sweep_dead_index_caches", "path": "src/catalog.rs"}}
{"id": "q18", "lang": "en", "kind": "phrase", "query": "statistics of a reindex", "gold": {"name": "ReindexStats", "path": "src/catalog.rs"}}
{"id": "q19", "lang": "fr", "kind": "phrase", "query": "vérifier les comptes du plein texte à l'ouverture", "gold": {"name": "check_fts_counts", "path": "src/catalog.rs"}}
{"id": "q20", "lang": "en", "kind": "phrase", "query": "open the full text index files", "gold": {"name": "open_fts_files", "path": "src/catalog.rs"}}
{"id": "q21", "lang": "fr", "kind": "phrase", "query": "la politique de maintenance du plein texte d'un backend", "gold": {"name": "FtsMaintenancePolicy", "path": "src/backend.rs"}}
{"id": "q22", "lang": "en", "kind": "phrase", "query": "the backend manifest", "gold": {"name": "BackendManifest", "path": "src/backend.rs"}}
{"id": "q23", "lang": "fr", "kind": "typo", "query": "manifeste du backnd", "gold": {"name": "BackendManifest", "path": "src/backend.rs"}}
{"id": "q24", "lang": "fr", "kind": "phrase", "query": "les réglages de recherche d'un backend", "gold": {"name": "SearchSettings", "path": "src/backend.rs"}}
{"id": "q25", "lang": "fr", "kind": "phrase", "query": "compiler le programme de recherche d'un backend", "gold": {"name": "compile_search_program", "path": "src/backend.rs"}}
{"id": "q26", "lang": "fr", "kind": "phrase", "query": "fermer les sessions mortes de l'hôte", "gold": {"name": "reap_dead_sessions", "path": "src/daemon_host.rs"}}
{"id": "q27", "lang": "en", "kind": "phrase", "query": "open a client session on the daemon host", "gold": {"name": "open_session", "path": "src/daemon_host.rs"}}
{"id": "q28", "lang": "fr", "kind": "phrase", "query": "les recherches récentes de l'hôte", "gold": {"name": "recent_searches", "path": "src/daemon_host.rs"}}
{"id": "q29", "lang": "en", "kind": "phrase", "query": "stop a running run", "gold": {"name": "stop_run", "path": "src/daemon_host.rs"}}
{"id": "q30", "lang": "fr", "kind": "long", "query": "où l'hôte arrête-t-il proprement ses bases avant de s'éteindre", "gold": {"name": "shutdown", "path": "src/daemon_host.rs"}}
{"id": "q31", "lang": "fr", "kind": "phrase", "query": "réingérer un fichier modifié dans l'index", "gold": {"name": "reingest_file", "path": "src/code_sync.rs"}}
{"id": "q32", "lang": "en", "kind": "phrase", "query": "remove a deleted file from the code index", "gold": {"name": "remove_file", "path": "src/code_sync.rs"}}
{"id": "q33", "lang": "fr", "kind": "phrase", "query": "synchroniser une source de code", "gold": {"name": "sync_source", "path": "src/code_sync.rs"}}
{"id": "q34", "lang": "fr", "kind": "typo", "query": "sincroniser la source de code", "gold": {"name": "sync_source", "path": "src/code_sync.rs"}}
{"id": "q35", "lang": "en", "kind": "phrase", "query": "LLM node with a response schema", "gold": {"name": "LlmNode", "path": "dataflow/llm_nodes.rs"}}
{"id": "q36", "lang": "fr", "kind": "page", "query": "le journal des chantiers", "gold": {"path": "docs/journal-des-chantiers.md", "page": true}}
{"id": "q37", "lang": "fr", "kind": "page", "query": "comment un lot est testé et poussé", "gold": {"path": "docs/procedures/push-d-un-lot.md", "page": true}}
{"id": "q38", "lang": "en", "kind": "page", "query": "instructions for coding agents in this repository", "gold": {"path": "AGENTS.md", "page": true}}
{"id": "q39", "lang": "fr", "kind": "page", "query": "les nouveautés, l'inverse d'une roadmap", "gold": {"path": "docs/nouveautes.md", "page": true}}
{"id": "q40", "lang": "en", "kind": "long", "query": "what does rag3weaver do and how do I start using it", "gold": {"path": "extension/rag3weaver/README.md", "page": true}}
{"id": "q41", "lang": "fr", "kind": "page", "query": "comment sont organisés les docs, un dossier par session de travail", "gold": {"path": "docs/README.md", "page": true}}
```
