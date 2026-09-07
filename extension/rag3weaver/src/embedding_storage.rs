//! **Où vivent les vecteurs d'un modèle** — la colonne, le marqueur, l'index.
//!
//! Un index porte plusieurs modèles d'embarquement (7 septembre 2026). Chacun
//! a sa colonne de vecteurs, son marqueur de fraîcheur et son index HNSW, et
//! **aucun de ces noms n'est dérivé** : ils sont résolus depuis ce qu'un
//! modèle a déclaré en s'enregistrant dans `_catalog_meta`.
//!
//! # Pourquoi résoudre plutôt que dériver
//!
//! Un index d'avant porte `embedding`, `_embed_hash`, `{table}_vec`. On ne
//! les renomme pas : un `RENAME COLUMN` serait gratuit, mais l'index HNSW ne
//! suivrait pas, et le reconstruire sur un gros index coûte ce qu'on veut
//! précisément éviter. À la place, l'entrée du premier modèle dit
//! `storage: legacy`, et c'est [`VectorStorage::resolve`] qui sait ce que ça
//! veut dire — **le seul endroit** qui connaît les deux conventions.
//!
//! Le garde-fou « un modèle par index » est donc devenu un index à une seule
//! entrée `legacy` : le cas dégénéré, sans code spécial.
//!
//! # Le slug
//!
//! `Embedder::name()` rend `granite-278m`, `bge-m3`… Le slug en est une
//! fonction pure : minuscules, `[a-z0-9]`, tout le reste devient `_`, sans
//! doublon ni bord. C'est lui qui nomme le stockage. Le séparateur des noms
//! composés est `__`, et un slug qui en contiendrait un est **refusé à
//! l'enregistrement** — la coupe reste sans ambiguïté par construction.

use serde::{Deserialize, Serialize};

/// Préfixe des clés de `_catalog_meta` : une par modèle enregistré.
pub const META_PREFIX: &str = "embedding_model:";

/// Le séparateur entre un nom de colonne et le slug qui le qualifie.
pub const SEPARATOR: &str = "__";

/// Comment un modèle range ses vecteurs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageKind {
    /// Les noms d'avant : `embedding`, `_embed_hash`, `{table}_vec`. Un seul
    /// modèle par index pouvait les porter ; il les garde.
    Legacy,
    /// Les noms qualifiés par le slug : `embedding__{slug}`,
    /// `_embed_hash__{slug}`, `{table}_vec__{slug}`.
    Suffixed,
}

/// Ce qu'un modèle déclare en s'enregistrant. La valeur d'une clé
/// `embedding_model:{slug}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbeddingModelEntry {
    /// Le nom tel que l'embarqueur le rend — pour les journaux et les refus.
    pub name: String,
    /// La dimension de ses vecteurs, gravée dans le type de sa colonne.
    pub dim: usize,
    pub storage: StorageKind,
}

impl EmbeddingModelEntry {
    pub fn slug(&self) -> String {
        slug_of(&self.name)
    }

    /// La signature `nom:dim`, celle que l'index d'avant écrivait.
    pub fn signature(&self) -> String {
        format!("{}:{}", self.name, self.dim)
    }

    pub fn meta_key(&self) -> String {
        format!("{META_PREFIX}{}", self.slug())
    }

    /// Lit une signature `nom:dim` — la clé `embedding_model` d'un index v5 —
    /// et en fait l'entrée `legacy` que la v6 inscrit à sa place.
    pub fn from_legacy_signature(signature: &str) -> Option<Self> {
        let (name, dim) = signature.rsplit_once(':')?;
        let dim: usize = dim.parse().ok()?;
        if name.is_empty() || dim == 0 {
            return None;
        }
        Some(Self { name: name.to_string(), dim, storage: StorageKind::Legacy })
    }
}

/// Le slug d'un nom de modèle : minuscules, `[a-z0-9]`, le reste devient `_`,
/// sans doublon ni bord. `granite-278m` → `granite_278m`.
pub fn slug_of(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut last_underscore = true; // pour ne pas commencer par `_`
    for c in name.chars() {
        let c = c.to_ascii_lowercase();
        if c.is_ascii_alphanumeric() {
            out.push(c);
            last_underscore = false;
        } else if !last_underscore {
            out.push('_');
            last_underscore = true;
        }
    }
    while out.ends_with('_') {
        out.pop();
    }
    out
}

/// **Un slug est refusé s'il ne peut pas nommer une colonne sans ambiguïté.**
///
/// Vide, un caractère hors `[a-z0-9_]`, ou le séparateur `__` lui-même : dans
/// les trois cas, le nom composé ne se recouperait plus, ou pas du tout.
pub fn validate_slug(slug: &str) -> Result<(), String> {
    if slug.is_empty() {
        return Err("slug vide — l'embarqueur n'a pas de nom utilisable".into());
    }
    if slug.contains(SEPARATOR) {
        return Err(format!("slug `{slug}` contient le séparateur `{SEPARATOR}`"));
    }
    if let Some(c) = slug.chars().find(|c| !(c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '_')) {
        return Err(format!("slug `{slug}` contient `{c}`, hors [a-z0-9_]"));
    }
    Ok(())
}

/// Les trois noms d'un modèle sur une table, résolus. Ce que tout consommateur
/// prend en entrée au lieu de reconstruire un `format!`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VectorStorage {
    pub column: String,
    pub marker: String,
    pub index: String,
    pub dim: usize,
}

impl VectorStorage {
    /// Résout les noms pour `table` — `{Entity}_Chunk`, ou une table de base
    /// de connaissances (`{KB}_Index`, `{KB}_Index_Chunk`).
    ///
    /// **Une table de base de connaissances garde ses noms d'avant, quel que
    /// soit le modèle.** Sa chaîne (`KBEmbedNode`, les DDL `_Index*`) n'a pas
    /// appris les modèles multiples — elle meurt avec le repli des KB en
    /// entités dérivées (chantier du 7 septembre 2026) — et elle écrit
    /// `{kb}_embedding` / `_embed_hash` / `{table}_vec`. Résoudre autre chose
    /// pour elle, c'est chercher dans un index qui n'existe pas
    /// (trouvé par `e2e_phase0b` le 8 septembre).
    pub fn resolve(table: &str, entry: &EmbeddingModelEntry) -> Self {
        let kb = table.strip_suffix("_Index_Chunk").or_else(|| table.strip_suffix("_Index"));
        let base_column = match kb {
            Some(kb) => format!("{kb}_embedding"),
            None => "embedding".to_string(),
        };
        let storage = if kb.is_some() { StorageKind::Legacy } else { entry.storage };
        match storage {
            StorageKind::Legacy => Self {
                column: base_column,
                marker: "_embed_hash".into(),
                index: format!("{table}_vec"),
                dim: entry.dim,
            },
            StorageKind::Suffixed => {
                let slug = entry.slug();
                Self {
                    column: format!("{base_column}{SEPARATOR}{slug}"),
                    marker: format!("_embed_hash{SEPARATOR}{slug}"),
                    index: format!("{table}_vec{SEPARATOR}{slug}"),
                    dim: entry.dim,
                }
            }
        }
    }
}

/// Le message d'un refus : le modèle demandé, et ceux que l'index sait servir.
/// Jamais zéro résultat en silence — la règle du dépôt, distinguer « ça
/// n'existe pas » de « je ne te le montre pas ».
pub fn unavailable_message(slug: &str, available: &[EmbeddingModelEntry]) -> String {
    let mut liste: Vec<String> = available
        .iter()
        .map(|e| match e.storage {
            StorageKind::Legacy => format!("legacy={}", e.name),
            StorageKind::Suffixed => e.slug(),
        })
        .collect();
    liste.sort();
    let liste = if liste.is_empty() { "aucun".to_string() } else { liste.join(", ") };
    format!("modèle `{slug}` non disponible sur cet index ; disponibles : {liste}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_slug_est_une_fonction_pure_du_nom() {
        assert_eq!(slug_of("granite-278m"), "granite_278m");
        assert_eq!(slug_of("bge-m3"), "bge_m3");
        assert_eq!(slug_of("MockEmbedder"), "mockembedder");
        assert_eq!(slug_of("ibm-granite/granite-embedding-107m (burn)"), "ibm_granite_granite_embedding_107m_burn");
        assert_eq!(slug_of("--a--b--"), "a_b", "ni doublon ni bord");
        assert_eq!(slug_of("?"), "", "le nom anonyme ne donne rien — et c'est refusé plus loin");
    }

    #[test]
    fn un_slug_qui_ne_se_recoupe_pas_est_refuse() {
        assert!(validate_slug("granite_278m").is_ok());
        assert!(validate_slug("").is_err(), "vide");
        assert!(validate_slug("a__b").is_err(), "le séparateur lui-même");
        assert!(validate_slug("Granite").is_err(), "majuscule");
        assert!(validate_slug("bge-m3").is_err(), "le tiret n'est pas dans l'alphabet du slug");
    }

    #[test]
    fn legacy_garde_les_noms_d_avant_et_suffixed_les_qualifie() {
        let legacy = EmbeddingModelEntry { name: "bge-m3".into(), dim: 1024, storage: StorageKind::Legacy };
        let s = VectorStorage::resolve("Scope_Chunk", &legacy);
        assert_eq!((s.column.as_str(), s.marker.as_str(), s.index.as_str(), s.dim), ("embedding", "_embed_hash", "Scope_Chunk_vec", 1024));

        let suffixed = EmbeddingModelEntry { name: "granite-278m".into(), dim: 768, storage: StorageKind::Suffixed };
        let s = VectorStorage::resolve("Scope_Chunk", &suffixed);
        assert_eq!(s.column, "embedding__granite_278m");
        assert_eq!(s.marker, "_embed_hash__granite_278m");
        assert_eq!(s.index, "Scope_Chunk_vec__granite_278m");
        assert_eq!(s.dim, 768);
    }

    /// Les tables d'une base de connaissances portaient `{kb}_embedding`, et
    /// **elles le gardent quel que soit le modèle** : leur chaîne n'écrit rien
    /// d'autre, et résoudre un nom suffixé pour elles cherchait un index
    /// inexistant.
    #[test]
    fn une_table_de_kb_garde_ses_noms_d_avant_quel_que_soit_le_modele() {
        let legacy = EmbeddingModelEntry { name: "bge-m3".into(), dim: 1024, storage: StorageKind::Legacy };
        assert_eq!(VectorStorage::resolve("docs_Index_Chunk", &legacy).column, "docs_embedding");
        assert_eq!(VectorStorage::resolve("docs_Index", &legacy).column, "docs_embedding");
        let suffixed = EmbeddingModelEntry { name: "granite-278m".into(), dim: 768, storage: StorageKind::Suffixed };
        let s = VectorStorage::resolve("docs_Index_Chunk", &suffixed);
        assert_eq!((s.column.as_str(), s.marker.as_str(), s.index.as_str()), ("docs_embedding", "_embed_hash", "docs_Index_Chunk_vec"));
    }

    #[test]
    fn la_signature_d_avant_devient_une_entree_legacy() {
        let e = EmbeddingModelEntry::from_legacy_signature("bge-m3:1024").expect("signature lisible");
        assert_eq!(e, EmbeddingModelEntry { name: "bge-m3".into(), dim: 1024, storage: StorageKind::Legacy });
        assert_eq!(e.meta_key(), "embedding_model:bge_m3");
        assert_eq!(e.signature(), "bge-m3:1024");
        // Un nom qui contient lui-même un `:` : on coupe au dernier.
        assert_eq!(EmbeddingModelEntry::from_legacy_signature("a:b:4").unwrap().name, "a:b");
        assert!(EmbeddingModelEntry::from_legacy_signature("sans-dim").is_none());
        assert!(EmbeddingModelEntry::from_legacy_signature(":4").is_none());
    }

    #[test]
    fn le_refus_nomme_ce_qui_est_disponible() {
        let dispo = vec![
            EmbeddingModelEntry { name: "granite-278m".into(), dim: 768, storage: StorageKind::Suffixed },
            EmbeddingModelEntry { name: "bge-m3".into(), dim: 1024, storage: StorageKind::Legacy },
        ];
        assert_eq!(
            unavailable_message("granite_107m", &dispo),
            "modèle `granite_107m` non disponible sur cet index ; disponibles : granite_278m, legacy=bge-m3"
        );
        assert!(unavailable_message("x", &[]).ends_with("disponibles : aucun"));
    }
}
