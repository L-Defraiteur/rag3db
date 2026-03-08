# Doc 20 — Réflexion : Extensibilité — Nœuds custom et limites de Rhai

Date : 8 mars 2026

## 1. Le problème

Le Doc 19 design ScriptNode (Rhai) comme un "CypherNode programmable" pour les migrations. Mais les vrais use cases d'extensibilité vont bien au-delà :

| Use case | Besoin | Rhai seul ? |
|----------|--------|-------------|
| Filtre de résultats search (score, entity type) | Manipulation de `Vec<UnifiedResult>` | Oui — mais ports typés requis |
| Reranking custom | Idem | Oui |
| Normalisation LLM (noms, catégories, descriptions) | Appel HTTP vers OpenAI/Ollama | **Non** — pas de réseau |
| Connecteur Shopify / Prestashop | HTTP + auth OAuth/API key | **Non** |
| Connecteur Composio | HTTP + auth | **Non** |
| Import CSV/JSON externe | Lecture FS | **Non** — pas d'accès FS |
| Enrichissement via API tierce (geocoding, etc.) | HTTP | **Non** |
| Processing Python/Node.js (langchain, etc.) | Subprocess / FFI | **Non** |

**Constat** : Rhai couvre les transformations internes (données déjà dans le graph). Pour tout ce qui implique le monde extérieur (APIs, fichiers, LLM), il est insuffisant par design (sandbox).

---

## 2. Les trois niveaux d'extensibilité

### Niveau 1 — ScriptNode (Rhai) : transformations sandboxées

**Quoi** : Un nœud dont la logique `execute()` est un script Rhai au lieu de code Rust.

**Accès** : inputs (ports), Cypher (via ServiceRegistry), outputs (ports). Pas de réseau, pas de FS.

**Use cases** :
- Migrations complexes (normalisation, data cleanup, multi-step Cypher)
- Filtre / reranking de résultats search
- Transformation de données entre nœuds (reformatage, calcul de champs dérivés)
- Validation custom
- Orchestration de queries Cypher conditionnelles

**Limites** : tout ce qui est dans le tableau "Non" ci-dessus.

**Exemple — filtre search** :
```
graph LR
    qs["QuerySourceNode(kb_name='$kb', query='$q')"]
    ps["PrimarySearchNode"]
    filter["ScriptNode(script='...', in_results='Results', out_results='Results')"]
    compose["ComposeNode"]

    qs -->|query| ps
    ps -->|results| filter
    filter -->|results| compose
```

```javascript
// filter.rhai
let results = ctx.inputs.results;
let filtered = [];
for r in results {
    if r.score > 0.3 && r.entity != "Internal" {
        filtered.push(r);
    }
}
set_output("results", filtered);
```

**Exemple — migration normalization** :
```javascript
let rows = query_cypher("MATCH (n:Product) RETURN n._uuid, n.name");
for row in rows {
    let clean = row.name.trim().to_lower();
    if clean != row.name {
        query_cypher(`MATCH (n {_uuid: '${row._uuid}'}) SET n.name = '${clean}'`);
    }
}
set_output("done", ());
```

### Niveau 2 — HttpNode : appels HTTP déclaratifs

**Quoi** : Un nœud built-in Rust qui fait un appel HTTP. Pas de scripting — configuration déclarative (URL, method, headers, body template).

**Accès** : réseau HTTP/HTTPS. Inputs pour le body/params, outputs pour la réponse.

**Use cases** :
- Appel LLM (OpenAI, Anthropic, Ollama local)
- Appels REST simples (Shopify product API, webhook, geocoding)
- Ingestion depuis une API paginée (combiné avec un script de contrôle)

**Combiné avec ScriptNode** — le pattern typique est script→http→script :

```
graph LR
    prepare["ScriptNode(script='build LLM prompt from entities')"]
    llm["HttpNode(url='http://localhost:11434/api/generate', method='POST')"]
    parse["ScriptNode(script='extract and structure LLM response')"]
    insert["InsertRecordNode"]

    prepare -->|result:body| llm
    llm -->|response:trigger| parse
    parse -->|result:entities| insert
```

**Config HttpNode** :
```
http["HttpNode(url='https://api.example.com/v1/data', method='POST', content_type='application/json', headers='Authorization: Bearer $token')"]
```

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `trigger` | in | Empty | Optionnel, séquençage |
| `body` | in | Map | Corps de la requête (sérialisé en JSON) |
| `response` | out | Map | Corps de la réponse (parsé depuis JSON) |
| `status` | out | Map | `{ status_code, headers }` |
| `done` | out | Empty | Signal de complétion |

**Limites** :
- Pas de flow OAuth interactif (pas de redirect URI, pas de browser)
- Pas de streaming (response complète en mémoire)
- Pas de WebSocket
- Un seul appel par exécution (pas de pagination automatique — il faut boucler via le graph ou un script)

**Note implémentation** : `reqwest` est déjà en dev-dependency. Le passer en dependency optionnelle (feature `http-node`) est trivial.

### Niveau 3 — ProcessNode : subprocess externe

**Quoi** : Un nœud qui spawn un process externe, envoie les inputs sur stdin (JSON), lit la réponse sur stdout (JSON).

**Accès** : tout ce que le process externe peut faire (réseau, FS, libs Python/Node.js, GPU).

**Use cases** :
- Flows OAuth complexes (token refresh, multi-step auth)
- Utilisation de libs Python (langchain, pandas, scikit-learn)
- Utilisation de libs Node.js (SDK Shopify, SDK Stripe)
- Processing GPU custom (modèle ML spécifique)
- Tout ce qui dépasse HTTP simple

**Exemple** :
```
graph LR
    source["QuerySourceNode(kb_name='Products', query='*')"]
    search["PrimarySearchNode"]
    enrich["ProcessNode(cmd='python3 scripts/enrich_products.py')"]
    insert["InsertRecordNode"]

    source -->|query| search
    search -->|results:stdin| enrich
    enrich -->|stdout:entities| insert
```

Le script Python reçoit les résultats sur stdin, fait ce qu'il veut (appels API, ML, etc.), et écrit les entités enrichies sur stdout.

**Protocole stdin/stdout** :
```json
// stdin (inputs)
{"results": [...], "config": {"api_key": "..."}}

// stdout (outputs)
{"entities": [...]}
```

**Limites** :
- Sécurité : le process a les droits du user qui lance rag3weaver
- Debugging : erreurs dans le process → stderr capturé dans le rapport d'exécution
- Performance : overhead de spawn + sérialisation JSON
- Dépendances : l'utilisateur doit installer Python/Node.js/etc. et les packages requis

---

## 3. Comparaison des niveaux

| | ScriptNode (Rhai) | HttpNode | ProcessNode |
|---|---|---|---|
| **Complexité implémentation** | Moyenne (~400 loc) | Faible (~200 loc) | Moyenne (~300 loc) |
| **Sécurité** | Sandbox complet | Réseau seulement | Aucune (droits du user) |
| **Dépendance runtime** | Aucune (embarqué) | Aucune | Python/Node/etc. |
| **Debugging** | Bon (erreurs Rhai claires) | Bon (HTTP status/body) | Variable (stderr) |
| **Latence** | ~1ms (CPU local) | ~10-500ms (réseau) | ~100ms+ (spawn + réseau) |
| **Feature flag** | `rhai-script` (~2MB) | `http-node` (~1MB reqwest) | Aucun (juste std::process) |
| **Use cases** | Transforms internes | APIs REST, LLM | Tout le reste |
| **WASM compatible** | Oui (rhai supporte WASM) | Non (pas de réseau en WASM) | Non (pas de subprocess en WASM) |

### Combinaisons puissantes

Les niveaux se composent. Un pipeline réel pourrait mixer les trois :

```
graph LR
    %% 1. Lire les produits du graph
    source["CypherNode(query='MATCH (p:Product) WHERE p.description IS NULL RETURN p._uuid, p.name')"]

    %% 2. Préparer le prompt LLM (Rhai)
    prepare["ScriptNode(script='build prompt batch for products')"]

    %% 3. Appeler le LLM (HTTP)
    llm["HttpNode(url='http://localhost:11434/api/generate', method='POST')"]

    %% 4. Parser la réponse LLM (Rhai)
    parse["ScriptNode(script='extract descriptions from LLM response')"]

    %% 5. Mettre à jour le graph (Cypher)
    update["ScriptNode(script='SET descriptions on Product nodes')"]

    source -->|done:trigger| prepare
    source -->|result:trigger| prepare
    prepare -->|result:body| llm
    llm -->|response:trigger| parse
    parse -->|done:trigger| update
```

---

## 4. Problème transversal : ports typés et Deserialize

### Le problème

Quel que soit le niveau, dès qu'un nœud custom s'insère dans un pipeline search, il doit manipuler des types comme `Results`, `Children`, `Meta`. Actuellement :

- Ces types (`UnifiedResult`, `ChildSummary`, `ChunkInfo`, `SearchMeta`, etc.) n'ont que `Serialize`, pas `Deserialize`
- Le checkpoint system ne sait pas non plus les restaurer (`deserialize_non_batch_port_value` = stub)
- Un ScriptNode qui filtre des Results ne peut pas les reconstruire après transformation

### La solution

Ajouter `Deserialize` aux types search. C'est mécanique — tous les champs sous-jacents sont déjà deserializables :

| Type | Champs | Bloquant ? |
|------|--------|-----------|
| `CypherValue` | primitifs + Vec + BTreeMap | **Déjà Deserialize** |
| `ChunkInfo` | String, usize, f64 | Non |
| `AttributedChunk` | idem + String | Non |
| `SearchMeta` | String, usize, enums Serialize | Non (ajouter Deserialize aux enums) |
| `ExploreGraph` | Vec<GraphNode>, Vec<GraphEdge> | Non |
| `ChildSummary` | String, BTreeMap<String, CypherValue> | Non |
| `UnifiedResult` | tout ci-dessus + Option<Vec<...>> récursif | Non |

**Impact positif collatéral** : ça débloque aussi `port_value_from_checkpoint` pour les types search, rendant le checkpoint complet (pas seulement Batch + Empty).

**Note `camelCase`** : ces types utilisent `#[serde(rename_all = "camelCase")]`. Le round-trip JSON fonctionne tant qu'on passe par serde (les clés JSON sont en camelCase, serde sait les mapper). Les scripts Rhai manipulent donc des clés camelCase (`r.startLine` pas `r.start_line`).

### Ports du ScriptNode

Avec Deserialize disponible, le ScriptNode peut avoir des ports configurables :

```
filter["ScriptNode(file='filter.rhai', in_results='Results', out_results='Results')"]
```

La factory :
1. Parse `in_*` / `out_*` dans la config → crée les PortDef correspondants
2. Si aucun `in_*`/`out_*` → ports par défaut (trigger/result/done)
3. À l'exécution :
   - Input : `PortValue::Results(vec)` → `serde_json::to_value` → `json_to_rhai` → Rhai Array
   - Output : Rhai → `rhai_to_json` → `serde_json::from_value::<Vec<UnifiedResult>>` → `PortValue::Results(vec)`

Le type déclaré dans la config guide la désérialisation de sortie.

---

## 5. Questions ouvertes

### Q1 — Scope Phase 5 : quels niveaux implémenter ?

**Option A** — ScriptNode seul
- +Focalisé, testable, ~400 loc
- -Les use cases excitants (LLM, APIs) restent bloqués

**Option B** — ScriptNode + HttpNode
- +Couvre 80% des cas réels
- +HttpNode est simple (~200 loc, reqwest)
- -Deux nœuds à implémenter + tester

**Option C** — ScriptNode + HttpNode + ProcessNode
- +Extensibilité complète
- -Surface large, plus de tests, sécurité ProcessNode

### Q2 — Rhai est-il le bon langage ?

Rhai est bien pour un DSL embarqué léger. Mais pour des scripts plus complexes :
- Pas de destructuring, pas de closures avancées
- Syntax non familière (ni JS, ni Python, ni Lua)
- Pas d'écosystème (pas de packages, pas de linter, pas d'IDE support)

**Alternatives** :
- **Lua** (mlua) — plus familier pour les gamedevs, écosystème plus large, mais même limitations sandbox
- **JavaScript** (QuickJS via rquickjs) — syntax universellement connue, JSON natif, mais plus lourd (~5MB)
- **Starlark** (starlark-rust) — langage de build Google (Bazel), Python-like, sandbox par design

Pour le MVP, Rhai reste un choix raisonnable. La question se pose surtout si on veut que des utilisateurs non-Rust écrivent beaucoup de scripts. À réévaluer après les premiers retours.

### Q3 — HttpNode : authentification

Pour les APIs qui nécessitent auth :
- **API key** : header statique `Authorization: Bearer $token` → facile via la config
- **OAuth2 client_credentials** : pourrait être un builtin du HttpNode (token refresh automatique)
- **OAuth2 authorization_code** : interactif, nécessite un browser → hors scope

Le MVP HttpNode pourrait supporter headers statiques + variables template. OAuth client_credentials en extension future.

### Q4 — Batching et pagination

Un HttpNode fait **un seul appel**. Pour itérer sur des pages ou batcher des appels LLM :
- **Option A** : boucle dans un ScriptNode qui appelle HttpNode via un builtin `http_call()` — mais ça casse le sandbox
- **Option B** : répétition via le graph (un nœud de contrôle qui re-trigger le HttpNode) — complexe
- **Option C** : un HttpBatchNode spécialisé qui itère sur un Array d'inputs — simple mais limité
- **Option D** : le ScriptNode prépare N appels, le HttpNode supporte un mode batch

À creuser. Pour le MVP, un seul appel par HttpNode suffit (l'utilisateur chunk ses données en amont si nécessaire).

### Q5 — Sécurité des credentials

Les URLs et headers contiennent potentiellement des secrets (API keys, tokens). Comment les gérer ?
- **Variables template** : `$OPENAI_KEY` résolu depuis les variables d'environnement ou un vault
- **Pas de log des headers auth** dans le rapport d'exécution
- **Feature flag** : `http-node` désactivé par défaut en WASM (pas de sens)

### Q6 — Et l'option "pas de Rhai du tout" ?

Si on a HttpNode + ProcessNode, est-ce que ScriptNode (Rhai) est vraiment nécessaire ?

**Oui**, pour les raisons suivantes :
- Les transformations simples (filtre, reformat) ne justifient pas un process externe
- Le round-trip serde (PortValue → JSON → process → JSON → PortValue) est lent pour des opérations triviales
- Les migrations Cypher multi-step sont le cas n°1 et Rhai y excelle
- Rhai tourne en WASM, ProcessNode non

ScriptNode et ProcessNode ne sont pas redondants — ils couvrent des échelles de complexité différentes.

---

## 6. Proposition de phasage

### Phase 5a — Pré-requis : Deserialize sur types search (~50 loc)
- Ajouter `Deserialize` à UnifiedResult, ChildSummary, ChunkInfo, AttributedChunk, SearchMeta, ExploreGraph, et leurs sous-types
- Implémenter `deserialize_non_batch_port_value` (le stub actuel)
- Tests round-trip : serialize → deserialize = identique
- **Bénéfice immédiat** : checkpoint complet pour les pipelines search

### Phase 5b — ScriptNode (Rhai) (~400 loc, ~15 tests)
- Comme décrit dans le Doc 19, mais avec ports configurables (in_*/out_*)
- Ports par défaut (trigger/result/done) si pas de config explicite
- Feature flag `rhai-script`

### Phase 5c — HttpNode (~200 loc, ~10 tests)
- URL, method, headers (avec variables template)
- Content-Type: application/json par défaut
- Ports : trigger(in), body(in, Map), response(out, Map), status(out, Map), done(out, Empty)
- Timeout configurable
- Feature flag `http-node` (reqwest en optional dependency)

### Phase 5d (futur) — ProcessNode
- Hors scope immédiat
- À implémenter quand un use case concret le justifie

---

## 7. Résumé

Le ScriptNode (Rhai) du Doc 19 est nécessaire mais insuffisant pour une vraie extensibilité. Les use cases les plus intéressants (LLM, connecteurs API) nécessitent l'accès réseau que Rhai interdit par design.

La solution est une approche multi-niveaux : **ScriptNode** pour les transformations internes, **HttpNode** pour les appels externes, et éventuellement **ProcessNode** pour le reste. Ces niveaux se composent naturellement dans le graph (script→http→script).

Le pré-requis transversal est l'ajout de `Deserialize` aux types search, qui débloque à la fois les ports typés pour ScriptNode et le checkpoint complet.
