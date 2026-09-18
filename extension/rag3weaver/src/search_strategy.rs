//! Search strategy types: `SearchStrategy` config and expansion rules.
//! Used by `Catalog::search_with_strategy()`.

use serde::{Deserialize, Serialize};

use crate::search::{SearchMeta, SearchOptions};

// ─── L'infrastructure du graphe a déménagé ──────────────────────────────────
//
// `UnifiedResult`, `ChildSummary` et `source_info` sont le type de port et
// les aides de **toute** la chaîne composable, pas de la seule stratégie :
// ils vivent dans `dataflow::resultat` depuis le 18 septembre 2026 (pas D du
// doc 02). Le chemin `search_strategy::*` reste pour les appelants.
pub use crate::dataflow::resultat::{source_info, ChildSummary, UnifiedResult};

// ─── SearchStrategy ─────────────────────────────────────────────────────────

/// Strategy configuration for search with reactive expansion.
#[derive(Debug, Clone)]
pub struct SearchStrategy {
    /// Base search options (limit, consistency, signals, etc.).
    pub search: SearchOptions,
    /// Expansion rules: which relations to follow after search results.
    pub expansions: Vec<ExpansionRule>,
    /// Maximum processing rounds (safety guard).
    pub max_rounds: usize,
}

impl Default for SearchStrategy {
    fn default() -> Self {
        Self {
            search: SearchOptions::default(),
            expansions: vec![],
            max_rounds: 10,
        }
    }
}

/// A rule that triggers graph expansion for matching search results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpansionRule {
    /// The relation to traverse (e.g. "HAS_FILE", "PARENT_OF").
    pub relation: String,
    /// Direction: follow the relation outgoing or incoming.
    pub direction: ExpansionDirection,
    /// Only expand results whose source entity matches this type.
    /// `None` = expand all results.
    pub source_entity: Option<String>,
    /// Maximum children to fetch per parent.
    pub limit: usize,
}

impl Default for ExpansionRule {
    fn default() -> Self {
        Self {
            relation: String::new(),
            direction: ExpansionDirection::Outgoing,
            source_entity: None,
            limit: 50,
        }
    }
}

/// Direction of relation traversal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExpansionDirection {
    Outgoing,
    Incoming,
}

// ─── SearchStrategyResponse ─────────────────────────────────────────────────

/// Complete response from `search_with_strategy()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchStrategyResponse {
    pub results: Vec<UnifiedResult>,
    pub meta: SearchMeta,
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strategy_default() {
        let s = SearchStrategy::default();
        assert!(s.expansions.is_empty());
        assert_eq!(s.max_rounds, 10);
    }
}
