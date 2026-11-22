# Le jeu de questions de la recherche, v2

22 novembre 2026. **v1 plus cinq fautes de frappe sur des identifiants anglais** (q42 à q46), à la demande de lucivy :
les fautes françaises de v1 visent du code anglais et mesurent surtout le dense, pas le flou de lucivy. v1 reste figé
([questions-recherche-v1.md](questions-recherche-v1.md)).

46 questions, mêmes règles d'or que v1. Chaque or est présent dans l'arbre de la base de mesure
`base-mesure-master-0763672a95` (master du 22 nov., vérifié avant son index).

**Les classes de lucivy**, mesurées sur origin/master par les mots porteurs de chaque question retrouvés dans son or :
- 39 solides ;
- 4 fragiles (q03, q18, q24, q31) ;
- 3 atteignables par le dense seulement (q14, q15, q28), où les budgets ne se jugent jamais.

**La fuite** : les pages du banc et les e2e du banc citent les questions. Le juge (`juge_questions_v2.py`) ôte toute ligne
sous `docs/banc/` et sous `extension/rag3weaver/tests/e2e_banc_` avant de lire un rang, et compte ce qu'il ôte.

Les cinq lignes ajoutées à v1 :

```jsonl
{"id": "q42", "lang": "en", "kind": "typo", "query": "FuseResultNode", "gold": {"name": "FuseResultsNode", "path": "dataflow/generic_search_nodes.rs"}}
{"id": "q43", "lang": "en", "kind": "typo", "query": "sync_sorce", "gold": {"name": "sync_source", "path": "src/code_sync.rs"}}
{"id": "q44", "lang": "en", "kind": "typo", "query": "BM25SerchNode", "gold": {"name": "BM25SearchNode", "path": "dataflow/generic_search_nodes.rs"}}
{"id": "q45", "lang": "en", "kind": "typo", "query": "reingest_fiel", "gold": {"name": "reingest_file", "path": "src/code_sync.rs"}}
{"id": "q46", "lang": "en", "kind": "typo", "query": "SearchOptons", "gold": {"name": "SearchOptions", "path": "src/search.rs"}}
```
