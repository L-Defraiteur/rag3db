# Backend déclaratif : carnet

Exemple générique de persistance rag3db, recherche lucivy + dense local et outils MCP. Les états et dates sont déclarés dans `backend.json` ; aucun vocabulaire métier n'est intégré au runtime.

- `schemas/` : contrats des données stockées (JSON Schema).
- `config.fields` : annotations de rôles ; les autres types viennent du schéma.
- `writes` : dates serveur en millisecondes UTC, révisions attendues et immutabilité.
- `graphs/` : opérations Mermaid réutilisables, avec entité liée par le manifeste.
- `tools` : seule surface exposée. Les bindings ne peuvent pas être remplacés par un appelant. `input_payloads` dérive une vue d'identité ou de champs éditables du schéma.

Le service d'embeddings déclaré doit être démarré et annoncer le modèle/dimension attendus. Le chemin de la bibliothèque vectorielle et celui de la base sont relatifs au manifeste. `data/` est ignoré par git.

Depuis la racine du dépôt, compilation adaptée à l'environnement de validation :

```bash
RAG3DB_SHARED=1 \
RAG3DB_LIBRARY_DIR="$PWD/build/lecteurs-csv/src" \
RAG3DB_INCLUDE_DIR="$PWD/build/lecteurs-csv/src" \
cargo build --offline --manifest-path extension/rag3weaver/Cargo.toml \
  --features daemon,rag3db-native --bin rag3weaver-backend -j 2
```

Décrire le contrat sans ouvrir la base ni contacter les embeddings :

```bash
LD_LIBRARY_PATH="$PWD/build/lecteurs-csv/src" \
extension/rag3weaver/target/debug/rag3weaver-backend \
  extension/rag3weaver/templates/backends/notebook/backend.json --describe
```

Exposer le MCP stdio (Python avec `scripts/requirements-mcp.txt` installé ; le venv expérimental existant convient) :

```bash
LD_LIBRARY_PATH="$PWD/build/lecteurs-csv/src" \
RAG3DB_BUFFER_POOL_SIZE=268435456 RAG3DB_MAX_DB_SIZE=2147483648 \
experiments/mtga/.venv/bin/python extension/rag3weaver/scripts/serve_backend_mcp.py \
  extension/rag3weaver/templates/backends/notebook/backend.json
```

Exemple d'arguments pour `put_note` :

```json
{"record":{"key":"one","text":"Flying creature","labels":["Blue"],"stage":"working"}}
```

Pour modifier : fournir le payload éditable complet et `expected_revision: 1` à côté de `record`. Pour lire : `get_note` avec `{"record":{"key":"one"}}`. Les dates et la révision stockée sont produites par le serveur.

Les snapshots sont immuables ; leur rejeu identique est accepté. Leur création n'est pas une transaction couplée à la modification d'une note. Voir le [rapport de périmètre et validation](../../../docs/19-09-2026/03-backends-declaratifs-et-persistance.md).
