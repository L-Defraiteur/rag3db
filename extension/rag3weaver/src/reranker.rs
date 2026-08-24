//! Reranking : un score (requête, passage) par paire, après la fusion des
//! signaux et avant la pagination (doc 29, chantier 3).
//!
//! Même doctrine que les embedders : le trait est la seule surface que le
//! `Catalog` voit ; le produit est un cross-encoder sur burn
//! (`BurnMiniLmReranker`), candle sert d'oracle de parité.

use std::sync::Arc;

use crate::embedder::EmbedError;

/// Scoreur de paires (requête, passage). Les scores sont des logits ou des
/// probabilités selon le modèle ; seul leur **ordre** est contractuel.
pub trait Reranker: Send + Sync {
    /// Un score par passage, dans l'ordre des passages.
    fn rerank(&self, query: &str, passages: &[String]) -> Result<Vec<f32>, EmbedError>;

    /// Nom lisible (modèle), pour les diagnostics.
    fn name(&self) -> &str {
        "reranker"
    }
}

impl<T: Reranker + ?Sized> Reranker for Arc<T> {
    fn rerank(&self, query: &str, passages: &[String]) -> Result<Vec<f32>, EmbedError> {
        (**self).rerank(query, passages)
    }
    fn name(&self) -> &str {
        (**self).name()
    }
}

/// Reranker de test : score = recouvrement lexical (mots de la requête présents
/// dans le passage, insensible à la casse), ce qui rend l'ordre prévisible.
#[derive(Debug, Default, Clone)]
pub struct MockReranker;

impl Reranker for MockReranker {
    fn rerank(&self, query: &str, passages: &[String]) -> Result<Vec<f32>, EmbedError> {
        let terms: Vec<String> = query
            .split(|c: char| !c.is_alphanumeric())
            .filter(|t| !t.is_empty())
            .map(|t| t.to_lowercase())
            .collect();
        Ok(passages
            .iter()
            .map(|p| {
                if terms.is_empty() {
                    return 0.0;
                }
                let lower = p.to_lowercase();
                let hits = terms.iter().filter(|t| lower.contains(t.as_str())).count();
                hits as f32 / terms.len() as f32
            })
            .collect())
    }
    fn name(&self) -> &str {
        "mock-overlap"
    }
}

/// Reranker par fermeture (hôtes non-Rust, tests).
pub struct CallbackReranker {
    name: String,
    f: Box<dyn Fn(&str, &[String]) -> Result<Vec<f32>, EmbedError> + Send + Sync>,
}

impl CallbackReranker {
    pub fn new(
        name: impl Into<String>,
        f: impl Fn(&str, &[String]) -> Result<Vec<f32>, EmbedError> + Send + Sync + 'static,
    ) -> Self {
        Self { name: name.into(), f: Box::new(f) }
    }
}

impl Reranker for CallbackReranker {
    fn rerank(&self, query: &str, passages: &[String]) -> Result<Vec<f32>, EmbedError> {
        (self.f)(query, passages)
    }
    fn name(&self) -> &str {
        &self.name
    }
}

/// Texte d'un résultat à soumettre au reranker : le chunk retrouvé d'abord
/// (c'est l'extrait réel), sinon `_content` s'il a été enrichi, sinon vide.
pub fn passage_text(r: &crate::search::SearchResult) -> String {
    if let Some(c) = r.chunk.as_ref() {
        if !c.text.is_empty() {
            return c.text.clone();
        }
    }
    let Some(d) = r.data.as_ref() else { return String::new() };
    if let Some(t) = d.get("_content").or_else(|| d.get("_text")).and_then(|v| v.as_str()) {
        if !t.is_empty() {
            return t.to_string();
        }
    }
    // Entité simple enrichie : ses champs utilisateur (titre, contenu…), dans
    // l'ordre des clés — le même texte que celui qui a été indexé, à peu près.
    d.iter()
        .filter(|(k, _)| !k.starts_with('_'))
        .filter_map(|(_, v)| v.as_str().filter(|s| !s.is_empty()))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_scores_by_overlap() {
        let s = MockReranker
            .rerank("kernel scheduler", &["the kernel scheduler".into(), "cooking".into(), "kernel".into()])
            .unwrap();
        assert!(s[0] > s[2] && s[2] > s[1]);
        assert_eq!(s[1], 0.0);
    }

    #[test]
    fn callback_forwards() {
        let r = CallbackReranker::new("cb", |_q, p| Ok(p.iter().map(|x| x.len() as f32).collect()));
        assert_eq!(r.rerank("q", &["ab".into(), "abcd".into()]).unwrap(), vec![2.0, 4.0]);
        assert_eq!(r.name(), "cb");
    }
}
