//! Catalog configuration structures.
//!
//! All structs support both camelCase and snake_case JSON keys for compatibility
//! with the TypeScript counterpart. Defaults are provided via `#[serde(default)]`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// ─── Field Types ─────────────────────────────────────────────────────────────

/// Type of a field in an entity definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    #[serde(alias = "String")]
    String,
    #[serde(alias = "Text")]
    Text,
    #[serde(alias = "Int64")]
    Int64,
    #[serde(alias = "integer", alias = "Integer")]
    Integer,
    #[serde(alias = "Double")]
    Double,
    #[serde(alias = "number", alias = "Number")]
    Number,
    #[serde(alias = "Boolean")]
    Boolean,
    #[serde(alias = "Timestamp")]
    Timestamp,
    #[serde(alias = "Json", alias = "JSON")]
    Json,
    #[serde(alias = "Tags")]
    Tags,
    #[serde(alias = "Choice")]
    Choice,
}

impl Default for FieldType {
    fn default() -> Self {
        Self::String
    }
}

// ─── Field Definition ────────────────────────────────────────────────────────

/// Definition of a single field within an entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldDef {
    #[serde(default, rename = "type", alias = "field_type", alias = "fieldType")]
    pub field_type: FieldType,

    #[serde(default, alias = "title_for")]
    pub title_for: Option<String>,

    #[serde(
        default,
        alias = "content_for",
        deserialize_with = "deserialize_string_or_vec"
    )]
    pub content_for: Option<Vec<String>>,

    /// **Accepté mais non appliqué** (vérifié le 25 août 2026) : aucun chemin
    /// de recherche ne lit ce champ — lucivy n'a pas de pondération par
    /// champ. Conservé pour la compatibilité des configs existantes.
    #[serde(default)]
    pub boost: Option<f64>,

    #[serde(default, rename = "default", alias = "default_value")]
    pub default_value: Option<serde_json::Value>,
}

impl FieldDef {
    /// A field is chunked if it is content for at least one knowledge base.
    pub fn is_chunked(&self) -> bool {
        self.content_for.is_some()
    }
}

fn deserialize_string_or_vec<'de, D>(deserializer: D) -> Result<Option<Vec<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de;

    struct StringOrVec;

    impl<'de> de::Visitor<'de> for StringOrVec {
        type Value = Option<Vec<String>>;

        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("null, a string, or a list of strings")
        }

        fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
            Ok(Some(vec![v.to_owned()]))
        }

        fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
            Ok(Some(vec![v]))
        }

        fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut v = Vec::new();
            while let Some(s) = seq.next_element::<String>()? {
                v.push(s);
            }
            Ok(Some(v))
        }
    }

    deserializer.deserialize_any(StringOrVec)
}

// ─── Entity Definition ──────────────────────────────────────────────────────

/// Definition of an entity type in the catalog.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityDef {
    #[serde(default)]
    pub fields: HashMap<String, FieldDef>,

    #[serde(default)]
    pub hashsafe: Option<Vec<String>>,
}

// ─── Relation Definition ────────────────────────────────────────────────────

/// Definition of a relation type between entities.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationDef {
    pub from: String,
    pub to: String,

    #[serde(default)]
    pub properties: Option<HashMap<String, FieldDef>>,
}

// ─── Search & KB Config ─────────────────────────────────────────────────────


/// Chunking strategy.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChunkStrategy {
    Semantic,
    Fixed,
    Sentence,
    /// Markdown-aware splitting (respects headers, code blocks, lists).
    Markdown,
}

impl Default for ChunkStrategy {
    fn default() -> Self {
        Self::Semantic
    }
}

/// Chunking configuration for a knowledge base.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ChunkingConfig {
    #[serde(alias = "max_size")]
    pub max_size: usize,

    pub overlap: usize,

    pub strategy: ChunkStrategy,

    #[serde(alias = "fulltext_on_chunks")]
    pub fulltext_on_chunks: bool,

    /// Maximum chars reserved for the title prefix in embed_text.
    /// Title is truncated to this limit before being prepended to chunk text.
    /// The effective chunk max_size is reduced by this amount + separator length.
    /// Set to 0 to disable title prefix in embeddings.
    #[serde(default = "default_title_max_chars", alias = "title_max_chars")]
    pub title_max_chars: usize,
}

fn default_title_max_chars() -> usize { 256 }

impl ChunkingConfig {
    /// Combien de chunks pour un contenu de `chars` caractères.
    ///
    /// Le titre est préfixé à chaque chunk pour l'embedding, donc il mange
    /// autant de place utile — c'est la partie qu'on oublie en estimant à la
    /// main, et elle compte : 256 caractères sur 1 500, c'est un sixième.
    ///
    /// Un contenu vide fait **un** chunk, pas zéro : la ligne existe.
    pub fn chunks_for(&self, chars: usize) -> usize {
        let effective = self.max_size.saturating_sub(self.title_max_chars).max(1);
        if chars <= effective {
            return 1;
        }
        // Chaque chunk suivant ne gagne que ce que le recouvrement ne reprend
        // pas. Un recouvrement >= à la taille utile n'avancerait jamais : on
        // le borne, sinon l'estimation partirait à l'infini au lieu de dire
        // que la configuration est absurde.
        let step = effective.saturating_sub(self.overlap).max(1);
        1 + (chars - effective).div_ceil(step)
    }
}

impl Default for ChunkingConfig {
    fn default() -> Self {
        Self {
            max_size: 1500,
            overlap: 200,
            strategy: ChunkStrategy::Semantic,
            fulltext_on_chunks: true,
            title_max_chars: default_title_max_chars(),
        }
    }
}

/// Knowledge base configuration.
///
/// JSON examples:
/// - `{ "signals": ["bm25", "vector", "sparse"] }`
/// - `{ "signals": ["bm25", "vector"], "signalConfigs": { "bm25": { "weight": 0.3, "role": "boost" } } }`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct KBConfig {
    /// Active search signals: `["bm25", "vector", "sparse"]`.
    pub signals: crate::search::SearchSignals,

    /// Per-signal config (weights, roles, boost types).
    /// Each value is a number (= weight) or an object (full config).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signal_configs: Option<HashMap<String, crate::search::SignalConfig>>,

    /// Fusion strategy for "fuse" signals (default: rrf).
    #[serde(default)]
    pub fusion_strategy: crate::search::FusionStrategy,

    /// RRF k parameter (default: 60.0).
    #[serde(default = "default_rrf_k")]
    pub rrf_k: f64,

    /// BM25 weight in fusion (used when signal_configs is absent).
    #[serde(alias = "keyword_weight")]
    pub keyword_weight: f64,

    /// **Accepté mais non appliqué** (vérifié le 25 août 2026) : copié dans
    /// `KBMetadata`, jamais lu ensuite. Voir `docs/vision_roadmap_08_2026/06`.
    #[serde(alias = "title_boost")]
    pub title_boost: f64,

    /// **Accepté mais non appliqué** — même statut que `title_boost`.
    #[serde(alias = "content_boost")]
    pub content_boost: f64,

    pub chunking: ChunkingConfig,

    /// **Accepté mais non appliqué** (vérifié le 25 août 2026) : désérialisé
    /// et jamais lu. Emplacement prévu pour `grep` / `read` (feuille de route
    /// `docs/vision_roadmap_08_2026/06` §2.2).
    #[serde(default, alias = "special_ops")]
    pub special_ops: Option<HashMap<String, serde_json::Value>>,

    /// Sparse weight in fusion (used when signal_configs is absent).
    #[serde(default = "default_sparse_weight", alias = "sparse_weight")]
    pub sparse_weight: f64,
}

fn default_sparse_weight() -> f64 { 0.2 }
fn default_rrf_k() -> f64 { 60.0 }

impl KBConfig {
    /// Build a [`FusionConfig`](crate::search::FusionConfig) from this KB config.
    ///
    /// If `signal_configs` is present, use it directly.
    /// Otherwise, derive from `keyword_weight` / `sparse_weight`.
    pub fn fusion_config(&self) -> crate::search::FusionConfig {
        use crate::search::{FusionConfig, SignalConfig};
        if let Some(ref configs) = self.signal_configs {
            let get = |name: &str| configs.get(name).copied().unwrap_or_default();
            FusionConfig {
                strategy: self.fusion_strategy,
                rrf_k: self.rrf_k,
                bm25: get("bm25"),
                vector: get("vector"),
                sparse: get("sparse"),
            }
        } else {
            let sparse_w = if self.signals.sparse() { self.sparse_weight } else { 0.0 };
            let vector_w = (1.0 - self.keyword_weight - sparse_w).max(0.0);
            FusionConfig {
                strategy: self.fusion_strategy,
                rrf_k: self.rrf_k,
                bm25: SignalConfig { weight: self.keyword_weight, ..SignalConfig::default() },
                vector: SignalConfig { weight: vector_w, ..SignalConfig::default() },
                sparse: SignalConfig { weight: self.sparse_weight, ..SignalConfig::default() },
            }
        }
    }
}

impl Default for KBConfig {
    fn default() -> Self {
        use crate::search::SearchSignals;
        Self {
            signals: SearchSignals::HYBRID,
            signal_configs: None,
            fusion_strategy: Default::default(),
            rrf_k: default_rrf_k(),
            keyword_weight: 0.3,
            title_boost: 2.0,
            content_boost: 1.0,
            chunking: ChunkingConfig::default(),
            special_ops: None,
            sparse_weight: default_sparse_weight(),
        }
    }
}

// ─── Embedding Config ───────────────────────────────────────────────────────

/// Configuration for the embedding provider.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct EmbeddingConfig {
    pub provider: Option<String>,
    pub model: Option<String>,

    #[serde(alias = "max_input_tokens")]
    pub max_input_tokens: Option<usize>,
}

// ─── Simple Entity Config (registerEntity) ──────────────────────────────────

/// Field definition for a simple entity (registerEntity API).
/// Unlike `FieldDef`, uses `is_title`/`is_content` instead of KB references.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SimpleFieldDef {
    /// Type of the field (String, Text, Int64, Double, Boolean, Timestamp, etc.)
    #[serde(default, rename = "type", alias = "field_type", alias = "fieldType")]
    pub field_type: FieldType,

    /// Title field — provides context for chunks. At most one per entity.
    /// Shortcut for `title_for: "self"` (simple pipeline, no KB).
    #[serde(default, alias = "is_title")]
    pub is_title: bool,

    /// Content field — concatenated for chunking/embedding. Multiple allowed.
    /// Shortcut for `content_for: ["self"]` (simple pipeline, no KB).
    #[serde(default, alias = "is_content")]
    pub is_content: bool,

    /// Explicit KB title assignment. This field provides the title for the named KB.
    /// Mutually exclusive with `is_title`.
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "title_for")]
    pub title_for: Option<String>,

    /// Explicit KB content assignment. This field provides content for the named KBs.
    /// Mutually exclusive with `is_content`.
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "content_for")]
    pub content_for: Option<Vec<String>>,
}

/// Configuration for a simple entity (registerEntity API).
/// Self-contained: declares fields, types, and search signals in one call.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct EntityConfig {
    pub fields: HashMap<String, SimpleFieldDef>,

    /// Search signals (default: Hybrid = BM25 + Vector).
    pub signals: crate::search::SearchSignals,

    /// Chunking configuration (default: Semantic, 1500 chars, 200 overlap).
    pub chunking: ChunkingConfig,

    /// Champs dont les valeurs, concaténées, déterminent l'`_uuid` de la
    /// ligne (`hashsafe_uuid(entity, values)`). Sans cette liste, l'uuid est
    /// dérivé de **tous** les champs : une ligne dont le contenu change est
    /// une autre ligne. Avec — `["path"]` pour un fichier, `["key"]` pour un
    /// scope de code — l'identité survit au changement de contenu, et une
    /// ré-ingestion met à jour au lieu de dupliquer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hashsafe: Option<Vec<String>>,

    /// Champs rendus par une recherche **en plus** du titre et des contenus.
    /// Un `Scope` de code trouvé par `search` doit dire son fichier et ses
    /// lignes, sinon le modèle ne peut pas le lire (25 août 2026, l'agent
    /// cloud a erré faute de `file_path` dans les résultats).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub return_fields: Option<Vec<String>>,

    /// `Some(false)` : cette entité **n'a pas de chunks**. Elle est écrite,
    /// indexée en plein texte et cherchable, mais sans ligne dans
    /// `{Entity}_Chunk` ni lien `CHUNKED_FROM`.
    ///
    /// Pour quoi faire : une entité dont le contenu *est* son titre — un
    /// nom de symbole de vingt caractères — paie sinon deux écritures pour
    /// rien. L'index plein texte, lui, vit sur la table **parente** : le
    /// BM25 ne perd rien (26 août 2026).
    ///
    /// Ce que ça coûte : pas d'extrait ni de lignes dans les résultats, et
    /// **aucune recherche vectorielle ni sparse** — la colonne `embedding`
    /// et l'index HNSW vivent sur la table de chunks. C'est pourquoi
    /// [`Self::validate`] refuse `Some(false)` avec un signal vecteur ou
    /// sparse : mieux vaut une erreur de configuration qu'une entité
    /// silencieusement introuvable.
    ///
    /// `None` (défaut) : comme avant, des chunks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chunked: Option<bool>,

    /// L'état de cette entité et les passages autorisés, s'il y en a.
    ///
    /// `None` (défaut) : l'entité n'a pas d'état, et rien ne change.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lifecycle: Option<Lifecycle>,
}

impl Default for EntityConfig {
    fn default() -> Self {
        Self {
            fields: HashMap::new(),
            signals: crate::search::SearchSignals::HYBRID,
            chunking: ChunkingConfig::default(),
            hashsafe: None,
            return_fields: None,
            chunked: None,
            lifecycle: None,
        }
    }
}

/// **Un passage d'un état à un autre**, nommé.
///
/// Le nom compte : c'est lui qui apparaît dans une trace, dans un événement et
/// dans une politique. `confirmed -> cancelled` ne dit pas s'il s'agit d'une
/// annulation par le client ou par le praticien ; `cancel_by_customer` le dit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transition {
    pub name: String,
    pub from: String,
    pub to: String,
}

/// **L'état d'une entité, et les passages autorisés.**
///
/// C'est la première tranche de comportement que la déclaration sait décrire
/// ([doc 07](../docs/27-aout-2026-13h01/07-le-langage-de-declaration.md) §4).
/// Jusqu'ici, `EntityConfig` décrivait une **forme** et des **signaux** ;
/// rien de ce qu'une ligne a le droit de devenir.
///
/// Deux propriétés en font autre chose qu'un champ `status` avec une
/// convention :
///
/// 1. **Elle est vérifiable statiquement.** Un état inatteignable, une
///    transition vers un état qui n'existe pas, deux transitions du même nom :
///    tout ça se voit sans exécuter quoi que ce soit, donc avant d'écrire une
///    seule ligne. C'est exactement ce qu'on demande à une représentation
///    intermédiaire produite par un modèle.
/// 2. **Les états ne sont pas listés à part.** Ils se déduisent de `initial`
///    et des transitions — une seule source, donc rien à garder en accord.
///    Un état qu'aucune transition ne mentionne n'existe pas, ce qui est la
///    bonne réponse : personne ne pourrait l'atteindre.
///
/// **Ce que ce type ne fait pas encore** : l'empêcher au moment d'écrire. Le
/// bon endroit est le drain, où l'**ancien** état est lu — `UpdateRecordNode`
/// lit déjà la ligne existante. Vérifier plus tôt demanderait une lecture de
/// plus et mentirait sur les mises à jour déjà en attente.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lifecycle {
    /// Le champ qui porte l'état. Doit exister, et être une chaîne.
    pub field: String,
    /// L'état d'une ligne qui vient d'être écrite.
    pub initial: String,
    pub transitions: Vec<Transition>,
}

impl Lifecycle {
    /// Tous les états, déduits — `initial` et les deux bouts de chaque
    /// transition. Triés, pour qu'un message d'erreur soit toujours le même.
    pub fn states(&self) -> Vec<&str> {
        let mut out: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
        out.insert(self.initial.as_str());
        for t in &self.transitions {
            out.insert(t.from.as_str());
            out.insert(t.to.as_str());
        }
        out.into_iter().collect()
    }

    /// La transition qui mène de `from` à `to`, s'il y en a une.
    pub fn allows(&self, from: &str, to: &str) -> Option<&Transition> {
        self.transitions.iter().find(|t| t.from == from && t.to == to)
    }

    /// Ce qu'une ligne dans l'état `from` peut devenir.
    pub fn next_from(&self, from: &str) -> Vec<&Transition> {
        self.transitions.iter().filter(|t| t.from == from).collect()
    }

    /// Les états atteignables depuis `initial`.
    fn reachable(&self) -> std::collections::BTreeSet<&str> {
        let mut seen: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
        let mut front = vec![self.initial.as_str()];
        seen.insert(self.initial.as_str());
        while let Some(at) = front.pop() {
            for t in self.transitions.iter().filter(|t| t.from == at) {
                if seen.insert(t.to.as_str()) {
                    front.push(t.to.as_str());
                }
            }
        }
        seen
    }

    /// Ce qui se voit **sans rien exécuter**.
    ///
    /// `fields` sert à vérifier que le champ d'état existe et porte bien une
    /// chaîne : une machine à états sur un entier serait acceptée par le type
    /// et fausse à l'écriture.
    pub fn validate(&self, fields: &HashMap<String, SimpleFieldDef>) -> Result<(), String> {
        if self.field.is_empty() || self.initial.is_empty() {
            return Err("lifecycle : `field` et `initial` sont obligatoires".into());
        }
        match fields.get(&self.field) {
            None => return Err(format!("lifecycle : '{}' n'est pas un champ de cette entité", self.field)),
            Some(f) if f.field_type != FieldType::String => {
                return Err(format!(
                    "lifecycle : le champ '{}' est {:?}, il doit être String — un état est un nom",
                    self.field, f.field_type
                ))
            }
            Some(_) => {}
        }
        let mut noms: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
        for t in &self.transitions {
            if t.name.is_empty() || t.from.is_empty() || t.to.is_empty() {
                return Err("lifecycle : une transition veut un nom, un départ et une arrivée".into());
            }
            if !noms.insert(t.name.as_str()) {
                return Err(format!("lifecycle : deux transitions nommées '{}'", t.name));
            }
        }
        // Un état qu'on ne peut pas atteindre est une déclaration morte : soit
        // il manque une transition, soit l'état est de trop. Les deux méritent
        // d'être dits maintenant plutôt que découverts en production.
        let atteignables = self.reachable();
        let orphelins: Vec<&str> = self.states().into_iter().filter(|s| !atteignables.contains(s)).collect();
        if !orphelins.is_empty() {
            return Err(format!(
                "lifecycle : état(s) inatteignable(s) depuis '{}' : {}",
                self.initial,
                orphelins.join(", ")
            ));
        }
        Ok(())
    }
}

/// **Ce qu'un schéma coûtera, avant qu'on le paie.**
///
/// Un coût qu'on ne voit pas est un coût qu'on ne discute pas. Celui-ci est
/// le plus facile à laisser filer, parce que rien ne casse : déclarer
/// « cherchable par le sens » est gratuit à écrire, et se paie en calcul,
/// en disque et en temps d'ingestion à chaque ligne, pour toujours.
///
/// **Ce sont des ordres de grandeur, pas des garanties** : le découpage
/// sémantique suit les frontières du texte, donc le nombre réel de chunks
/// s'écarte de ce calcul. Ça suffit pour la seule question qui compte au
/// moment de déclarer — *est-ce qu'on parle de mille ou de cent mille ?*
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SchemaCost {
    /// Les lignes de l'entité elle-même.
    pub rows: usize,
    /// Les lignes écrites dans `{Entity}_Chunk`.
    pub chunks: usize,
    /// Les vecteurs denses calculés **et** stockés.
    pub embeddings: usize,
    /// Les vecteurs creux.
    pub sparse_vectors: usize,
    /// Les documents indexés en plein texte — sur la table parente, et sur
    /// les chunks si `fulltext_on_chunks`.
    pub fulltext_documents: usize,
}

impl SchemaCost {
    /// Une ligne lisible, pour être **dite** — c'est tout l'objet.
    pub fn describe(&self, entity: &str) -> String {
        let mut parts = vec![format!("{} lignes", self.rows)];
        if self.chunks > 0 {
            parts.push(format!("{} chunks", self.chunks));
        }
        if self.embeddings > 0 {
            parts.push(format!("{} embeddings", self.embeddings));
        }
        if self.sparse_vectors > 0 {
            parts.push(format!("{} vecteurs creux", self.sparse_vectors));
        }
        parts.push(format!("{} documents plein texte", self.fulltext_documents));
        format!("{entity} : {}", parts.join(", "))
    }
}

impl EntityConfig {
    /// **Le point de départ d'un schéma déclaré par un modèle.**
    ///
    /// [`Self::default`] vaut `HYBRID` — BM25 **et** vecteur — ce qui est un
    /// défaut raisonnable pour du code écrit à la main par quelqu'un qui sait
    /// ce qu'il déclare. Ça ne l'est pas du tout pour une déclaration produite
    /// par un modèle, qui cochera tout ce qui est cochable : dire
    /// « cherchable » est gratuit à écrire et cher à exécuter.
    ///
    /// **Ce n'est pas une inquiétude théorique.** Le défaut a déjà mordu, chez
    /// nous, dans du code écrit par des gens qui connaissent la maison : il
    /// faisait calculer et stocker un embedding pour chacun des 3 275 symboles
    /// de `src/dataflow`, que personne n'avait voulus (`code.rs`,
    /// `symbol_config`).
    ///
    /// Donc ici : **BM25, et rien d'autre**. Un vecteur se demande.
    pub fn declared() -> Self {
        Self { signals: crate::search::SearchSignals::BM25, ..Self::default() }
    }

    /// Ce que coûtera cette entité pour `rows` lignes dont le contenu fait en
    /// moyenne `avg_content_chars` caractères.
    ///
    /// Voir [`SchemaCost`] pour ce que ce chiffre vaut, et ne vaut pas.
    pub fn cost_for(&self, rows: usize, avg_content_chars: usize) -> SchemaCost {
        let chunks_per_row = if self.chunked == Some(false) {
            0
        } else {
            self.chunking.chunks_for(avg_content_chars)
        };
        let chunks = rows * chunks_per_row;
        SchemaCost {
            rows,
            chunks,
            embeddings: if self.signals.vector() { chunks } else { 0 },
            sparse_vectors: if self.signals.sparse() { chunks } else { 0 },
            // Le plein texte vit sur la table **parente** ; les chunks ne s'y
            // ajoutent que si on l'a demandé.
            fulltext_documents: if self.signals.bm25() {
                rows + if self.chunking.fulltext_on_chunks { chunks } else { 0 }
            } else {
                0
            },
        }
    }

    /// Get the title field name (first field with is_title=true or title_for="self").
    pub fn title_field(&self) -> Option<&str> {
        self.fields.iter()
            .find(|(_, f)| f.is_title || f.title_for.as_deref() == Some("self"))
            .map(|(name, _)| name.as_str())
    }

    /// Get content field names (fields with is_content=true or content_for containing "self"), sorted.
    pub fn content_fields(&self) -> Vec<&str> {
        let mut fields: Vec<&str> = self.fields.iter()
            .filter(|(_, f)| f.is_content || f.content_for.as_ref().map_or(false, |v| v.iter().any(|s| s == "self")))
            .map(|(name, _)| name.as_str())
            .collect();
        fields.sort();
        fields
    }

    /// Returns true if this entity has simple pipeline content fields (is_content or content_for="self").
    pub fn has_simple_pipeline(&self) -> bool {
        !self.content_fields().is_empty()
    }

    /// Returns true if any field participates in a KB (title_for or content_for pointing to a KB name, not "self").
    pub fn has_kb_participation(&self) -> bool {
        self.fields.values().any(|f| {
            f.title_for.as_ref().map_or(false, |v| v != "self")
                || f.content_for.as_ref().map_or(false, |v| v.iter().any(|s| s != "self"))
        })
    }

    /// Validate field definitions (mutual exclusivity of is_title/title_for, is_content/content_for).
    pub fn validate(&self) -> Result<(), String> {
        if let Some(hs) = &self.hashsafe {
            if hs.is_empty() {
                return Err("hashsafe: empty list (omit it to hash all fields)".into());
            }
            if let Some(unknown) = hs.iter().find(|f| !self.fields.contains_key(*f)) {
                return Err(format!("hashsafe: '{unknown}' is not a field of this entity"));
            }
        }
        if let Some(rf) = &self.return_fields {
            if let Some(unknown) = rf.iter().find(|f| !self.fields.contains_key(*f)) {
                return Err(format!("return_fields: '{unknown}' is not a field of this entity"));
            }
        }
        if self.chunked == Some(false) {
            // Le vecteur et le sparse vivent sur la table de chunks : sans
            // chunk, l'entité serait invisible pour eux — en silence. On
            // refuse plutôt que de le laisser arriver.
            let mut absent = Vec::new();
            if self.signals.vector() {
                absent.push("vector");
            }
            if self.signals.sparse() {
                absent.push("sparse");
            }
            if !absent.is_empty() {
                return Err(format!(
                    "chunked = false est incompatible avec le(s) signal(aux) {} : \
                     leur index vit sur la table de chunks, l'entité serait introuvable. \
                     Garder les chunks, ou déclarer `signals: BM25`.",
                    absent.join(", ")
                ));
            }
        }
        if (self.signals.vector() || self.signals.sparse()) && !self.has_simple_pipeline() {
            // Même famille que le refus ci-dessus, et même conséquence : les
            // chunks seraient vides, donc l'entité introuvable par ces
            // signaux — en silence. Une erreur de configuration vaut mieux.
            return Err(
                "un signal vecteur ou sparse sans champ de contenu \
                 (`is_content` / `content_for: [\"self\"]`) : il n'y aurait rien à \
                 embarquer, et l'entité serait introuvable par ces signaux. \
                 Déclarer un champ de contenu, ou `signals: BM25`."
                    .into(),
            );
        }
        if let Some(lc) = &self.lifecycle {
            lc.validate(&self.fields)?;
        }
        for (name, f) in &self.fields {
            if crate::scope::is_scope_column(name) {
                return Err(format!("Field '{name}': nom réservé au scope (org/project)"));
            }
            if f.is_title && f.title_for.is_some() {
                return Err(format!("Field '{name}': is_title and title_for are mutually exclusive"));
            }
            if f.is_content && f.content_for.is_some() {
                return Err(format!("Field '{name}': is_content and content_for are mutually exclusive"));
            }
        }
        Ok(())
    }
}

// ─── Flush Config ───────────────────────────────────────────────────────────

/// Configuration for the auto-flush pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FlushConfig {
    #[serde(alias = "auto_flush")]
    pub auto_flush: bool,

    #[serde(alias = "max_count")]
    pub max_count: usize,

    #[serde(alias = "max_delay_ms")]
    pub max_delay_ms: u64,

    #[serde(alias = "completed_retention_ms")]
    pub completed_retention_ms: u64,

    #[serde(alias = "embed_batch_size")]
    pub embed_batch_size: usize,
}

impl Default for FlushConfig {
    fn default() -> Self {
        Self {
            auto_flush: true,
            max_count: 50,
            max_delay_ms: 100,
            completed_retention_ms: 3_600_000,
            embed_batch_size: 32,
        }
    }
}

// ─── Main Catalog Config ────────────────────────────────────────────────────

/// Top-level catalog configuration.
///
/// Defines the entity types, relation types, knowledge bases,
/// and embedding parameters for a rag3weaver instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CatalogConfig {
    pub name: Option<String>,

    pub entities: HashMap<String, EntityDef>,

    pub relations: HashMap<String, RelationDef>,

    #[serde(alias = "knowledge_bases")]
    pub knowledge_bases: HashMap<String, KBConfig>,

    #[serde(alias = "embedding_dim")]
    pub embedding_dim: usize,

    pub embedding: Option<EmbeddingConfig>,

    pub flush: FlushConfig,
}

impl Default for CatalogConfig {
    fn default() -> Self {
        Self {
            name: None,
            entities: HashMap::new(),
            relations: HashMap::new(),
            knowledge_bases: HashMap::new(),
            embedding_dim: 384,
            embedding: None,
            flush: FlushConfig::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_chunkless_entity_must_not_claim_a_vector_signal() {
        use crate::search::SearchSignals;
        let base = |signals: SearchSignals, chunked: Option<bool>| {
            let mut fields = HashMap::new();
            fields.insert(
                "name".to_string(),
                SimpleFieldDef { field_type: FieldType::String, is_title: true, is_content: true, ..Default::default() },
            );
            EntityConfig { fields, signals, chunked, ..Default::default() }
        };

        // Sans chunk et sans vecteur : le cas voulu (un nom de symbole).
        assert!(base(SearchSignals::BM25, Some(false)).validate().is_ok());
        // Avec chunks, tout est permis comme avant.
        assert!(base(SearchSignals::HYBRID, None).validate().is_ok());
        assert!(base(SearchSignals::HYBRID, Some(true)).validate().is_ok());

        // Sans chunk **et** avec un signal dont l'index vit sur les chunks :
        // refusé, avec la raison et le remède.
        let e = base(SearchSignals::HYBRID, Some(false)).validate().unwrap_err();
        assert!(e.contains("vector") && e.contains("introuvable") && e.contains("BM25"), "{e}");
        let e = base(SearchSignals::BM25 | SearchSignals::SPARSE, Some(false)).validate().unwrap_err();
        assert!(e.contains("sparse"), "{e}");
    }

    #[test]
    fn default_catalog_config() {
        let config = CatalogConfig::default();
        assert_eq!(config.embedding_dim, 384);
        assert!(config.entities.is_empty());
        assert!(config.knowledge_bases.is_empty());
        assert!(config.flush.auto_flush);
        assert_eq!(config.flush.max_count, 50);
    }

    #[test]
    fn serde_roundtrip() {
        let json_str = r#"{
            "name": "test-catalog",
            "entities": {
                "Document": {
                    "fields": {
                        "title": { "type": "text", "titleFor": "main", "boost": 2.0 },
                        "body": { "type": "text", "contentFor": "main" },
                        "page_count": { "type": "int64" }
                    },
                    "hashsafe": ["title"]
                }
            },
            "relations": {
                "REFERENCES": { "from": "Document", "to": "Document" }
            },
            "knowledgeBases": {
                "main": {
                    "search": "hybrid",
                    "keywordWeight": 0.4,
                    "chunking": { "maxSize": 2000, "overlap": 300 }
                }
            },
            "embeddingDim": 768
        }"#;

        let config: CatalogConfig = serde_json::from_str(json_str).unwrap();
        assert_eq!(config.name.as_deref(), Some("test-catalog"));
        assert_eq!(config.embedding_dim, 768);

        let doc = &config.entities["Document"];
        assert_eq!(doc.hashsafe.as_deref(), Some(&["title".to_string()][..]));

        let title = &doc.fields["title"];
        assert_eq!(title.field_type, FieldType::Text);
        assert_eq!(title.title_for.as_deref(), Some("main"));
        assert_eq!(title.boost, Some(2.0));

        let body = &doc.fields["body"];
        assert!(body.is_chunked());
        assert_eq!(
            body.content_for.as_deref(),
            Some(&["main".to_string()][..])
        );

        let kb = &config.knowledge_bases["main"];
        assert_eq!(kb.signals, crate::search::SearchSignals::HYBRID);
        assert_eq!(kb.keyword_weight, 0.4);
        assert_eq!(kb.chunking.max_size, 2000);
        assert_eq!(kb.chunking.overlap, 300);

        // Roundtrip
        let serialized = serde_json::to_string(&config).unwrap();
        let config2: CatalogConfig = serde_json::from_str(&serialized).unwrap();
        assert_eq!(config2.name, config.name);
        assert_eq!(config2.embedding_dim, config.embedding_dim);
    }

    #[test]
    fn snake_case_keys() {
        let json_str = r#"{
            "knowledge_bases": {
                "kb1": {
                    "keyword_weight": 0.5,
                    "title_boost": 3.0,
                    "content_boost": 1.5
                }
            },
            "embedding_dim": 512
        }"#;

        let config: CatalogConfig = serde_json::from_str(json_str).unwrap();
        assert_eq!(config.embedding_dim, 512);

        let kb = &config.knowledge_bases["kb1"];
        assert_eq!(kb.keyword_weight, 0.5);
        assert_eq!(kb.title_boost, 3.0);
        assert_eq!(kb.content_boost, 1.5);
    }

    #[test]
    fn content_for_single_string() {
        let json_str = r#"{ "type": "text", "contentFor": "main" }"#;
        let field: FieldDef = serde_json::from_str(json_str).unwrap();
        assert_eq!(field.content_for, Some(vec!["main".to_string()]));
    }

    #[test]
    fn content_for_array() {
        let json_str = r#"{ "type": "text", "contentFor": ["main", "summary"] }"#;
        let field: FieldDef = serde_json::from_str(json_str).unwrap();
        assert_eq!(
            field.content_for,
            Some(vec!["main".to_string(), "summary".to_string()])
        );
    }

    #[test]
    fn content_for_absent() {
        let json_str = r#"{ "type": "text" }"#;
        let field: FieldDef = serde_json::from_str(json_str).unwrap();
        assert_eq!(field.content_for, None);
    }

    #[test]
    fn defaults_fill_in() {
        let config: CatalogConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(config.embedding_dim, 384);
        assert!(config.name.is_none());
        assert!(config.entities.is_empty());
        assert!(config.flush.auto_flush);
        assert_eq!(config.flush.embed_batch_size, 32);
    }

    #[test]
    fn field_type_enum_values() {
        for (json_val, expected) in [
            ("\"string\"", FieldType::String),
            ("\"text\"", FieldType::Text),
            ("\"int64\"", FieldType::Int64),
            ("\"double\"", FieldType::Double),
            ("\"boolean\"", FieldType::Boolean),
            ("\"timestamp\"", FieldType::Timestamp),
            ("\"json\"", FieldType::Json),
            ("\"tags\"", FieldType::Tags),
            ("\"choice\"", FieldType::Choice),
        ] {
            let ft: FieldType = serde_json::from_str(json_val).unwrap();
            assert_eq!(ft, expected, "failed for {json_val}");
        }
    }

    #[test]
    fn field_type_pascal_case() {
        for (json_val, expected) in [
            ("\"String\"", FieldType::String),
            ("\"Text\"", FieldType::Text),
            ("\"Int64\"", FieldType::Int64),
            ("\"Integer\"", FieldType::Integer),
            ("\"Double\"", FieldType::Double),
            ("\"Number\"", FieldType::Number),
            ("\"Boolean\"", FieldType::Boolean),
            ("\"Timestamp\"", FieldType::Timestamp),
            ("\"Json\"", FieldType::Json),
            ("\"JSON\"", FieldType::Json),
            ("\"Tags\"", FieldType::Tags),
            ("\"Choice\"", FieldType::Choice),
        ] {
            let ft: FieldType = serde_json::from_str(json_val).unwrap();
            assert_eq!(ft, expected, "failed for {json_val}");
        }
    }

    /// Reproduces the WASM test config exactly as JS sends it (camelCase keys,
    /// PascalCase FieldType values). This was the root cause of the Lucivy
    /// schema panic: "fieldType" key was not recognized, defaulting to String.
    #[test]
    fn js_style_config_deserialization() {
        let json_str = r#"{
            "name": "test-weaver",
            "entities": {
                "Document": {
                    "fields": {
                        "title": { "fieldType": "Text", "titleFor": "main" },
                        "body": { "fieldType": "Text" }
                    }
                }
            },
            "relations": {
                "REFERENCES": { "from": "Document", "to": "Document" }
            },
            "knowledgeBases": { "main": {} },
            "embeddingDim": 4
        }"#;

        let config: CatalogConfig = serde_json::from_str(json_str).unwrap();
        let doc = &config.entities["Document"];
        assert_eq!(doc.fields["title"].field_type, FieldType::Text,
            "title should be Text, not {:?}", doc.fields["title"].field_type);
        assert_eq!(doc.fields["body"].field_type, FieldType::Text,
            "body should be Text, not {:?}", doc.fields["body"].field_type);
    }

    #[test]
    fn chunking_defaults() {
        let c = ChunkingConfig::default();
        assert_eq!(c.max_size, 1500);
        assert_eq!(c.overlap, 200);
        assert_eq!(c.strategy, ChunkStrategy::Semantic);
        assert!(c.fulltext_on_chunks);
    }

    #[test]
    fn flush_config_snake_case() {
        let json_str = r#"{
            "auto_flush": false,
            "max_count": 100,
            "max_delay_ms": 500,
            "embed_batch_size": 64
        }"#;
        let fc: FlushConfig = serde_json::from_str(json_str).unwrap();
        assert!(!fc.auto_flush);
        assert_eq!(fc.max_count, 100);
        assert_eq!(fc.max_delay_ms, 500);
        assert_eq!(fc.embed_batch_size, 64);
    }

    #[test]
    fn relation_with_properties() {
        let json_str = r#"{
            "from": "Author",
            "to": "Book",
            "properties": {
                "role": { "type": "string" }
            }
        }"#;
        let rel: RelationDef = serde_json::from_str(json_str).unwrap();
        assert_eq!(rel.from, "Author");
        assert_eq!(rel.to, "Book");
        let props = rel.properties.unwrap();
        assert!(props.contains_key("role"));
        assert_eq!(props["role"].field_type, FieldType::String);
    }

    // ── Ce que le langage encourage (doc 07 §3) ─────────────────────────

    use crate::search::SearchSignals;

    fn with_content() -> EntityConfig {
        let mut fields = HashMap::new();
        fields.insert("name".to_string(), SimpleFieldDef { is_title: true, ..Default::default() });
        fields.insert("body".to_string(), SimpleFieldDef { is_content: true, ..Default::default() });
        EntityConfig { fields, ..Default::default() }
    }

    /// Même famille que `chunked = false` + vecteur : les chunks seraient
    /// vides, donc l'entité introuvable — en silence.
    #[test]
    fn un_signal_semantique_sans_contenu_est_refuse() {
        let mut fields = HashMap::new();
        fields.insert("name".to_string(), SimpleFieldDef { is_title: true, ..Default::default() });
        let c = EntityConfig { fields, ..Default::default() }; // HYBRID par défaut
        let err = c.validate().unwrap_err();
        assert!(err.contains("vecteur ou sparse"), "{err}");

        // Le même, en BM25 : parfaitement légitime.
        let mut fields = HashMap::new();
        fields.insert("name".to_string(), SimpleFieldDef { is_title: true, ..Default::default() });
        let c = EntityConfig { fields, signals: SearchSignals::BM25, ..Default::default() };
        assert!(c.validate().is_ok());
    }

    #[test]
    fn le_defaut_declare_par_un_modele_est_bm25_seul() {
        let d = EntityConfig::declared();
        assert!(d.signals.bm25());
        assert!(!d.signals.vector(), "un vecteur se demande, il ne s'hérite pas");
        assert!(!d.signals.sparse());
        // Et le défaut de la maison n'a pas changé : le casser casserait les
        // appelants, et il reste raisonnable pour du code écrit à la main.
        assert!(EntityConfig::default().signals.vector());
    }

    #[test]
    fn un_contenu_court_fait_un_chunk_pas_zero() {
        let c = ChunkingConfig::default();
        assert_eq!(c.chunks_for(0), 1);
        assert_eq!(c.chunks_for(10), 1);
        // 1500 - 256 = 1244 caractères utiles.
        assert_eq!(c.chunks_for(1_244), 1);
        assert_eq!(c.chunks_for(1_245), 2);
    }

    #[test]
    fn un_recouvrement_absurde_ne_part_pas_a_l_infini() {
        let c = ChunkingConfig { max_size: 500, overlap: 5_000, title_max_chars: 0, ..Default::default() };
        // Le pas est borné à 1 : le chiffre est énorme, et c'est exactement ce
        // qu'il faut voir avant de payer.
        assert_eq!(c.chunks_for(1_000), 1 + 500);
    }

    /// **Le chiffre du doc 07 §3.1, rendu exécutable.**
    ///
    /// 3 275 symboles, dont le contenu *est* le nom — une vingtaine de
    /// caractères. Sous le défaut de la maison, c'est 3 275 embeddings ;
    /// sous celui d'un schéma déclaré, zéro. Personne n'avait voulu les
    /// premiers.
    #[test]
    fn le_defaut_hybrid_aurait_coute_3275_embeddings() {
        let hybrid = with_content(); // HYBRID
        let cout = hybrid.cost_for(3_275, 20);
        assert_eq!(cout.embeddings, 3_275);

        let declare = EntityConfig { signals: SearchSignals::BM25, ..with_content() };
        assert_eq!(declare.cost_for(3_275, 20).embeddings, 0);

        // Et ce que `Symbol` déclare vraiment : sans chunks du tout.
        let symbole = EntityConfig { signals: SearchSignals::BM25, chunked: Some(false), ..with_content() };
        let c = symbole.cost_for(3_275, 20);
        assert_eq!((c.chunks, c.embeddings), (0, 0));
        assert_eq!(c.fulltext_documents, 3_275, "le plein texte vit sur la table parente");
        eprintln!("[coût] {}", cout.describe("Symbol (HYBRID)"));
        eprintln!("[coût] {}", c.describe("Symbol (déclaré)"));
    }

    /// Le plein texte vit sur la table **parente** — les chunks ne s'y
    /// ajoutent que si on l'a demandé. C'est le piège qui a coûté une nuit.
    #[test]
    fn le_plein_texte_compte_la_table_parente() {
        let mut c = with_content();
        c.chunking.fulltext_on_chunks = false;
        assert_eq!(c.cost_for(100, 10_000).fulltext_documents, 100);
        c.chunking.fulltext_on_chunks = true;
        let cout = c.cost_for(100, 10_000);
        assert_eq!(cout.fulltext_documents, 100 + cout.chunks);
    }

    // ── L'état et ses transitions (doc 07 §4) ───────────────────────────

    fn t(name: &str, from: &str, to: &str) -> Transition {
        Transition { name: name.into(), from: from.into(), to: to.into() }
    }

    /// Le premier usage sera le nôtre : un outil qui passe de brouillon à
    /// promu sur preuve (doc 49, doc 05).
    fn promotion() -> Lifecycle {
        Lifecycle {
            field: "status".into(),
            initial: "draft".into(),
            transitions: vec![
                t("promote", "draft", "promoted"),
                t("demote", "promoted", "draft"),
                t("archive", "promoted", "archived"),
            ],
        }
    }

    fn avec_status() -> HashMap<String, SimpleFieldDef> {
        let mut fields = HashMap::new();
        fields.insert("name".to_string(), SimpleFieldDef { is_title: true, ..Default::default() });
        fields.insert("status".to_string(), SimpleFieldDef::default());
        fields
    }

    #[test]
    fn les_etats_se_deduisent_des_transitions() {
        // Une seule source de vérité : rien à garder en accord.
        assert_eq!(promotion().states(), vec!["archived", "draft", "promoted"]);
    }

    #[test]
    fn une_machine_coherente_passe() {
        assert!(promotion().validate(&avec_status()).is_ok());
    }

    #[test]
    fn un_etat_inatteignable_est_refuse() {
        // Le cas qui ne se voit jamais à la lecture et jamais à l'exécution :
        // la ligne n'y arrive tout simplement pas.
        let mut lc = promotion();
        lc.transitions.push(t("purge", "archived", "purged"));
        lc.transitions.push(t("revive", "limbo", "draft"));
        let err = lc.validate(&avec_status()).unwrap_err();
        assert!(err.contains("inatteignable"), "{err}");
        assert!(err.contains("limbo"), "{err}");
        assert!(!err.contains("purged"), "purged s'atteint par archived : {err}");
    }

    #[test]
    fn le_champ_d_etat_doit_exister_et_etre_une_chaine() {
        let mut fields = avec_status();
        let err = promotion().validate(&HashMap::new()).unwrap_err();
        assert!(err.contains("n'est pas un champ"), "{err}");

        // Une machine à états sur un entier passe le typage et serait fausse
        // à l'écriture.
        fields.insert("status".to_string(), SimpleFieldDef { field_type: FieldType::Integer, ..Default::default() });
        let err = promotion().validate(&fields).unwrap_err();
        assert!(err.contains("String"), "{err}");
    }

    #[test]
    fn deux_transitions_du_meme_nom_sont_refusees() {
        // Le nom est ce qui apparaîtra dans une trace : deux identiques et on
        // ne sait plus laquelle a eu lieu.
        let mut lc = promotion();
        lc.transitions.push(t("promote", "archived", "promoted"));
        let err = lc.validate(&avec_status()).unwrap_err();
        assert!(err.contains("deux transitions"), "{err}");
    }

    #[test]
    fn ce_qu_une_ligne_peut_devenir() {
        let lc = promotion();
        assert!(lc.allows("draft", "promoted").is_some());
        assert!(lc.allows("draft", "archived").is_none(), "on n'archive pas un brouillon");
        assert_eq!(lc.allows("promoted", "archived").map(|t| t.name.as_str()), Some("archive"));
        let depuis_promu: Vec<&str> = lc.next_from("promoted").iter().map(|t| t.name.as_str()).collect();
        assert_eq!(depuis_promu, vec!["demote", "archive"]);
        assert!(lc.next_from("archived").is_empty(), "un état terminal");
    }

    #[test]
    fn une_entite_sans_etat_reste_valide() {
        // La promesse d'usage : ajouter ce champ ne change rien à ce qui existe.
        assert_eq!(EntityConfig::default().lifecycle, None);
        let c = EntityConfig { fields: avec_status(), signals: SearchSignals::BM25, ..Default::default() };
        assert!(c.validate().is_ok());
    }

    #[test]
    fn une_machine_incoherente_fait_echouer_l_entite() {
        // La garde est bien branchée sur `EntityConfig::validate`, pas
        // seulement disponible à côté.
        let mut lc = promotion();
        lc.transitions.push(t("revive", "limbo", "draft"));
        let c = EntityConfig {
            fields: avec_status(),
            signals: SearchSignals::BM25,
            lifecycle: Some(lc),
            ..Default::default()
        };
        assert!(c.validate().unwrap_err().contains("inatteignable"));
    }

    #[test]
    fn une_machine_survit_a_un_aller_retour_json() {
        let c = EntityConfig {
            fields: avec_status(),
            signals: SearchSignals::BM25,
            lifecycle: Some(promotion()),
            ..Default::default()
        };
        let json = serde_json::to_string(&c).unwrap();
        let relu: EntityConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(relu.lifecycle, c.lifecycle);
        // Et une entité sans machine n'écrit pas la clé.
        let nu = EntityConfig { fields: avec_status(), signals: SearchSignals::BM25, ..Default::default() };
        assert!(!serde_json::to_string(&nu).unwrap().contains("lifecycle"));
    }
}
