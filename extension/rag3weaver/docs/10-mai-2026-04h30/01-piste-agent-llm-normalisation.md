# Piste : Normalisation agentique LLM dans rag3weaver

## Contexte

On veut permettre des pipelines d'ingestion où certains champs (xlsx, CSV, etc.) passent par un LLM pour normalisation/enrichissement avant indexation. Exemple : un champ "adresse" libre → extraction structurée (rue, ville, code postal), ou un champ "description produit" → catégorisation + mots-clés.

## Écosystème Rust agent (mai 2026)

| Lib | License | Approche | Actor runtime | Tool calling | Structured output | Multi-provider |
|-----|---------|----------|---------------|-------------|-------------------|----------------|
| **Rig** | MIT/Apache 2.0 | Modulaire, bas niveau | Aucun | Trait `Tool` | Serde extraction | OpenAI, Anthropic, Ollama, Groq, Cohere |
| **AutoAgents** | MIT/Apache 2.0 | Framework complet | Ractor (Erlang-like) | `#[tool]` macro | JSON schema typé | OpenAI, Anthropic, Ollama, DeepSeek, xAI |
| **rs-agent** | MIT | Batteries included | Aucun (orchestrateur) | UTCP | Oui | Gemini, Ollama, Anthropic, OpenAI |
| **ai-agents** | MIT | State machine | Aucun | Skill system | Oui | 12 providers |

## Architecture rag3weaver actuelle

Rag3weaver a déjà un framework dataflow complet :

- `Node` trait : `name()`, `inputs()`, `outputs()`, `execute(&mut self, ctx)`
- `DataflowGraph` : DAG typé avec ports
- `DataflowRuntime` : exécution topologique, checkpoint, observability
- `ServiceRegistry` : injection de services (DB, embedder, etc.)
- `NodeRegistry` + `NodeFactory` : instanciation dynamique de nœuds
- Nœuds existants : `InsertRecordNode`, `EmbedNode`, `ChunkRecordNode`, `FlushNode`, etc.

Le pipeline d'ingestion actuel :
```
records → InsertRecord → Chunk → Embed → KBGather → KBUpdate → Flush
```

## Intégration proposée : LlmNormalizeNode

### Option A : Intégré dans rag3weaver (recommandé)

Un nouveau nœud `LlmNormalizeNode` dans le dataflow existant :

```
xlsx_parse → LlmNormalize → InsertRecord → Chunk → Embed → Flush
```

```rust
pub struct LlmNormalizeNode {
    name: String,
    provider: Arc<dyn LlmProvider>,  // trait abstrait
    prompt_template: String,         // "Normalise cette adresse: {input}"
    output_schema: JsonSchema,       // schema structuré attendu
    field_mapping: HashMap<String, String>,  // champ source → champ cible
    batch_size: usize,               // pour batching des appels LLM
}

impl Node for LlmNormalizeNode {
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let records = ctx.read_input::<BatchPayload>("records")?;
        // Pour chaque record, appeler le LLM avec le prompt + champs sources
        // Parser la réponse structurée
        // Écrire les champs normalisés dans le record
        ctx.write_output("records", normalized_records);
        Ok(())
    }
}
```

Le `LlmProvider` trait :
```rust
pub trait LlmProvider: Send + Sync {
    fn complete(&self, prompt: &str, schema: &JsonSchema) -> Result<serde_json::Value, String>;
    fn complete_batch(&self, prompts: &[String], schema: &JsonSchema) -> Result<Vec<serde_json::Value>, String>;
}
```

### Option B : Crate séparé (rag3weaver-agent)

Si la complexité agent grandit (multi-step reasoning, agent-as-tool, memory), un crate séparé :

```
rag3weaver-agent/
├── src/
│   ├── lib.rs
│   ├── provider/        # LLM providers (OpenAI, Anthropic, Ollama)
│   ├── tool/            # Tool trait + registry
│   ├── normalize_node.rs  # LlmNormalizeNode (implémente dataflow::Node)
│   ├── classify_node.rs   # LlmClassifyNode
│   └── extract_node.rs    # LlmExtractNode (extraction d'entités)
└── Cargo.toml           # dépend de rag3weaver (pour Node trait) + rig ou autoagents-core
```

## Quelle lib LLM utiliser ?

### Rig seul (recommandé pour commencer)

- On utilise Rig comme couche LLM (providers, structured output)
- Le dataflow rag3weaver reste l'orchestrateur
- Pas de conflit de runtime
- ~50 lignes pour implémenter `LlmProvider` au-dessus de Rig

```rust
// Implémentation avec Rig
use rig::providers::openai;

struct RigProvider {
    client: openai::Client,
    model: String,
}

impl LlmProvider for RigProvider {
    fn complete(&self, prompt: &str, schema: &JsonSchema) -> Result<Value, String> {
        let agent = self.client.agent(&self.model).build();
        // Rig gère le structured output nativement
        let result = tokio::runtime::Runtime::new().unwrap()
            .block_on(agent.extract::<Value>(prompt));
        result.map_err(|e| e.to_string())
    }
}
```

### Fork AutoAgents (si besoin multi-agent plus tard)

- Prendre `autoagents-core` (macros `#[tool]`, `#[agent]`)
- Prendre `autoagents-llm` (providers)
- Remplacer Ractor par le trait `Node` de rag3weaver dataflow
- Un `AgentNode` qui encapsule un agent AutoAgents dans un nœud dataflow

Avantages :
- Macros `#[tool]` → zéro boilerplate pour définir des tools
- Sandbox WASM pour tool execution (sécurité)
- Structured output déjà typé

Inconvénients :
- Plus de code à maintenir (fork)
- AutoAgents est encore jeune (v0.2.x)

## Décision recommandée

**Phase 1** : `LlmNormalizeNode` intégré dans rag3weaver, basé sur Rig.
- Un seul nœud, un trait `LlmProvider`, 2-3 implémentations (OpenAI, Anthropic, Ollama)
- Pas de fork, pas de dépendance lourde
- Validé sur le use case xlsx normalisation

**Phase 2** (si besoin) : Extraire vers `rag3weaver-agent` avec plus de nœuds :
- `LlmClassifyNode` (catégorisation)
- `LlmExtractNode` (NER / extraction d'entités)
- `LlmValidateNode` (validation qualité des données)
- Tool calling pour enrichissement (API externes, DB lookups)

**Phase 3** (si multi-agent nécessaire) : Fork AutoAgents, adapter sur dataflow.

## Configuration dans le schema rag3weaver

```json
{
  "entities": [{
    "name": "Product",
    "fields": [
      {"name": "raw_description", "type": "text"},
      {"name": "category", "type": "text"},
      {"name": "keywords", "type": "text[]"}
    ],
    "normalize": {
      "provider": "openai",
      "model": "gpt-4o-mini",
      "rules": [
        {
          "input": "raw_description",
          "output": ["category", "keywords"],
          "prompt": "Extract category and keywords from this product description"
        }
      ]
    }
  }]
}
```

## Liens

- Rig : https://github.com/0xPlaygrounds/rig (MIT/Apache 2.0)
- AutoAgents : https://github.com/liquidos-ai/AutoAgents (MIT/Apache 2.0)
- rs-agent : https://crates.io/crates/rs-agent (MIT)
- UTCP : https://github.com/universal-tool-calling-protocol/rs-utcp
