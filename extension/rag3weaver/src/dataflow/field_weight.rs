//! **La pondération par valeur de champ** — `FieldWeightNode` (pas C du
//! 2 octobre 2026). Un poids, jamais un filtre : le score se multiplie,
//! rien ne sort de la liste, elle se réordonne.
//!
//! Son module à lui, et pas un fichier existant : `e2e_code` indexe
//! `port.rs` comme corpus vivant et le banc indexe `src/` — on n'alourdit
//! pas un fichier-corpus (leçon du 18 septembre).
//!
//! **L'échelle du 2 octobre**, la même que pour la fusion, par champ :
//! l'appelant (`options.field_weights`) > le **choix** du graphe
//! (`field` + `weights`) > l'entité (`EntityConfig.field_weights`,
//! transporté par la cible sans aplatissement) > le **défaut** du gabarit
//! (`field` + `default_weights`) > neutre. La première déclaration d'un
//! champ gagne ; les champs des autres étages restent appliqués.
//!
//! **Trois absences, trois conduites — jamais de rétrécissement
//! silencieux** : une *valeur* hors table prend `default` ; un *champ*
//! hors du schéma de la cible est neutre et s'avoue dans la méta ; une
//! *donnée* absente (résultat non enrichi) est neutre et s'avoue aussi.
//! Le nœud est posé d'office dans `search_base.mmd` entre la fusion et le
//! rerank — le pool du cross-encoder doit déjà refléter la pondération, et
//! lui reste juge final — neutre à coût nul tant que personne ne déclare.

use std::sync::{Arc, Mutex};

use crate::catalog::Catalog;
use crate::search::FieldWeight;

use super::node::{Node, NodeContext};
use super::port::{take_or_clone, PortDef, PortValue, QueryPayload};
use super::resultat::UnifiedResult;

pub struct FieldWeightNode {
    node_name: String,
    /// Le **choix** du graphe : prime sur la déclaration de l'entité.
    choice: Option<FieldWeight>,
    /// Le **défaut** du gabarit : ne pèse que si personne ne déclare ce champ.
    fallback: Option<FieldWeight>,
}

impl FieldWeightNode {
    pub fn new(name: &str) -> Self {
        Self {
            node_name: name.to_string(),
            choice: None,
            fallback: None,
        }
    }

    /// Le choix du graphe : un champ, sa table, son défaut.
    pub fn with_choice(mut self, fw: FieldWeight) -> Self {
        self.choice = Some(fw);
        self
    }

    /// Le défaut du gabarit pour un champ.
    pub fn with_fallback(mut self, fw: FieldWeight) -> Self {
        self.fallback = Some(fw);
        self
    }
}

impl Node for FieldWeightNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "FieldWeightNode"
    }
    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        Some(Box::new(serde_json::json!({
            "choice": self.choice,
            "fallback": self.fallback,
        })))
    }
    fn inputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::node_factories::FieldWeightNodeFactory).0
    }
    fn outputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::node_factories::FieldWeightNodeFactory).1
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let start = std::time::Instant::now();
        let mut results = ctx
            .take_input("results")
            .and_then(|pv| take_or_clone::<Vec<UnifiedResult>>(pv))
            .unwrap_or_default();
        let qp = ctx.take_input("query").and_then(|pv| take_or_clone::<QueryPayload>(pv));
        let mut warnings: Vec<String> = Vec::new();

        self.apply(&mut results, qp.as_ref(), ctx, &mut warnings);

        let count = results.len();
        ctx.set_output("results", PortValue::new(results));
        ctx.set_output(
            "meta",
            PortValue::new(crate::search::SearchMeta {
                query: qp.as_ref().map(|q| q.query.clone()).unwrap_or_default(),
                target: qp.as_ref().map(|q| q.target_name.clone()).unwrap_or_default(),
                signals: crate::search::SearchSignals::NONE,
                consistency: qp.as_ref().map(|q| q.options.consistency).unwrap_or_default(),
                partial: false,
                pending_count: 0,
                vector_count: 0,
                bm25_count: 0,
                sparse_count: 0,
                fused_count: count,
                reranked_count: 0,
                warnings,
                search_time_ms: start.elapsed().as_millis() as u64,
                diagnostics: None,
            }),
        );
        Ok(())
    }
}

impl FieldWeightNode {
    /// L'échelle du 2 octobre, par champ — la première déclaration d'un
    /// champ gagne : l'appelant, le choix du graphe, l'entité, le défaut du
    /// gabarit. Puis le produit des poids s'applique à chaque résultat et la
    /// liste se réordonne. Les trois absences s'avouent (doc de module).
    fn apply(
        &self,
        results: &mut [UnifiedResult],
        qp: Option<&QueryPayload>,
        ctx: &mut NodeContext,
        warnings: &mut Vec<String>,
    ) {
        let empty: Vec<FieldWeight> = Vec::new();
        let caller = qp.map(|q| &q.options.field_weights).unwrap_or(&empty);
        let entity = qp
            .and_then(|q| q.target.as_ref())
            .map(|t| &t.field_weights)
            .unwrap_or(&empty);

        let mut effective: Vec<&FieldWeight> = Vec::new();
        let mut taken: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for fw in caller
            .iter()
            .chain(self.choice.iter())
            .chain(entity.iter())
            .chain(self.fallback.iter())
        {
            if taken.insert(fw.field.as_str()) {
                effective.push(fw);
            }
        }
        if effective.is_empty() {
            // Neutre à coût nul : le cas de `search_base` sans déclaration.
            return;
        }

        // Un champ hors du schéma de la cible est neutre, et s'avoue — le
        // catalogue connaît les champs ; sans lui (graphe nu de test), la
        // conduite « donnée absente » suffit.
        if let (Some(qp), Some(catalog)) =
            (qp, ctx.service::<Arc<Mutex<Catalog>>>("catalog").cloned())
        {
            let known_fields = {
                let cat = catalog.lock().unwrap();
                cat.entity_configs()
                    .get(&qp.target_name)
                    .map(|c| c.fields.keys().cloned().collect::<std::collections::HashSet<_>>())
            };
            if let Some(known_fields) = known_fields {
                effective.retain(|fw| {
                    let known = known_fields.contains(&fw.field);
                    if !known {
                        warnings.push(format!(
                            "FieldWeightNode: le champ « {} » n'existe pas sur {} — pondération neutre",
                            fw.field, qp.target_name
                        ));
                    }
                    known
                });
            }
        }

        let mut missing_data: std::collections::BTreeSet<String> = Default::default();
        for r in results.iter_mut() {
            for fw in &effective {
                let weight = match r.data.as_ref().and_then(|d| d.get(&fw.field)) {
                    Some(v) => match v.as_str() {
                        Some(value) => fw.weights.get(value).copied().unwrap_or(fw.default),
                        // Une valeur qui n'est pas du texte : hors table, le défaut.
                        None => fw.default,
                    },
                    None => {
                        missing_data.insert(fw.field.clone());
                        1.0
                    }
                };
                r.score *= weight;
            }
        }
        for field in missing_data {
            warnings.push(format!(
                "FieldWeightNode: le champ « {field} » absent des données d'au moins un résultat — pondération neutre pour ceux-là"
            ));
        }
        results.sort_by(|a, b| b.score.total_cmp(&a.score).then(a.uuid.cmp(&b.uuid)));
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use crate::connection::CypherValue;
    use crate::search::{SearchMeta, SearchOptions};

    fn fw(field: &str, paires: &[(&str, f64)], default: f64) -> FieldWeight {
        FieldWeight {
            field: field.to_string(),
            weights: paires.iter().map(|(v, w)| (v.to_string(), *w)).collect(),
            default,
        }
    }

    fn make_result(uuid: &str, score: f64, kind: Option<&str>) -> UnifiedResult {
        let mut r = UnifiedResult {
            uuid: uuid.into(),
            score,
            entity: Some("T".into()),
            data: None,
            chunk: None,
            chunks: None,
            relation: None,
            matched_children: None,
            other_children: None,
            graph: None,
            signal: None,
        };
        if let Some(k) = kind {
            let mut data = BTreeMap::new();
            data.insert("kind".to_string(), CypherValue::String(k.to_string()));
            r.data = Some(data);
        }
        r
    }

    fn target_with(field_weights: Vec<FieldWeight>) -> crate::search::SearchTarget {
        crate::search::SearchTarget {
            name: "T".into(),
            parent_table: "T".into(),
            chunk_table: "T_Chunk".into(),
            chunk_rel: "T_CHUNKED_FROM".into(),
            chunk_rel_fwd: false,
            bm25_fields: vec![],
            enrich_fields: vec![],
            default_signals: crate::search::SearchSignals::HYBRID,
            default_fusion: None,
            has_source_refs: false,
            filter_indirection: None,
            field_weights,
        }
    }

    fn run_node(
        node: FieldWeightNode,
        results: Vec<UnifiedResult>,
        qp: Option<QueryPayload>,
    ) -> (Vec<UnifiedResult>, SearchMeta) {
        let mut node = node;
        let mut ctx = NodeContext::new();
        ctx.set_input("results", PortValue::new(results));
        if let Some(qp) = qp {
            ctx.set_input("query", PortValue::new(qp));
        }
        node.execute(&mut ctx).unwrap();
        let mut outputs = ctx.drain_outputs();
        let out = outputs
            .remove("results")
            .and_then(|pv| take_or_clone::<Vec<UnifiedResult>>(pv))
            .expect("results en sortie");
        let meta = outputs
            .remove("meta")
            .and_then(|pv| take_or_clone::<SearchMeta>(pv))
            .expect("meta en sortie");
        (out, meta)
    }

    fn payload(target: Option<crate::search::SearchTarget>, options: SearchOptions) -> QueryPayload {
        QueryPayload {
            target_name: "T".into(),
            query: "q".into(),
            options,
            target,
            embedding: None,
            sparse: None,
            scan: false,
        }
    }

    /// La table multiplie et réordonne ; rien ne sort de la liste.
    #[test]
    fn table_multiplies_and_reorders() {
        let node = FieldWeightNode::new("weigh").with_choice(fw("kind", &[("file", 0.5)], 1.0));
        let (out, _) = run_node(
            node,
            vec![make_result("a", 0.9, Some("file")), make_result("b", 0.8, Some("class"))],
            None,
        );
        assert_eq!(out.len(), 2, "un poids, jamais un filtre");
        assert_eq!(out[0].uuid, "b", "le genre dévalué passe derrière");
        assert!((out[0].score - 0.8).abs() < 1e-12);
        assert!((out[1].score - 0.45).abs() < 1e-12);
    }

    /// Une valeur hors table prend `default`.
    #[test]
    fn value_outside_table_takes_default() {
        let node = FieldWeightNode::new("weigh").with_choice(fw("kind", &[("file", 1.0)], 0.1));
        let (out, _) = run_node(
            node,
            vec![make_result("a", 0.9, Some("class")), make_result("b", 0.5, Some("file"))],
            None,
        );
        assert_eq!(out[0].uuid, "b", "la valeur hors table a pris 0,1");
        assert!((out[1].score - 0.09).abs() < 1e-12);
    }

    /// Une donnée absente est neutre, et s'avoue dans la méta.
    #[test]
    fn missing_data_is_neutral_and_admitted() {
        let node = FieldWeightNode::new("weigh").with_choice(fw("kind", &[("file", 0.1)], 0.1));
        let (out, meta) = run_node(
            node,
            vec![make_result("a", 0.9, None), make_result("b", 0.5, Some("file"))],
            None,
        );
        assert_eq!(out[0].uuid, "a");
        assert!((out[0].score - 0.9).abs() < 1e-12, "sans donnée, neutre");
        assert!(
            meta.warnings.iter().any(|w| w.contains("kind")),
            "l'absence s'avoue : {:?}",
            meta.warnings
        );
    }

    /// **L'échelle par champ** : l'appelant > le choix du graphe > l'entité
    /// > le défaut du gabarit ; la première déclaration d'un champ gagne.
    #[test]
    fn per_field_precedence_ladder() {
        let two = || vec![make_result("a", 0.9, Some("file")), make_result("b", 0.8, Some("class"))];
        let first = |node: FieldWeightNode, qp: Option<QueryPayload>| -> String {
            run_node(node, two(), qp).0[0].uuid.clone()
        };

        // Le défaut du gabarit pèse seul.
        let n = FieldWeightNode::new("weigh").with_fallback(fw("kind", &[("file", 0.1)], 1.0));
        assert_eq!(first(n, None), "b", "le défaut du gabarit pèse sans déclaration");

        // L'entité bat le défaut du gabarit.
        let n = FieldWeightNode::new("weigh").with_fallback(fw("kind", &[("file", 0.1)], 1.0));
        let qp = payload(Some(target_with(vec![fw("kind", &[("class", 0.1)], 1.0)])), SearchOptions::default());
        assert_eq!(first(n, Some(qp)), "a", "l'entité bat le défaut du gabarit");

        // Le choix du graphe bat l'entité.
        let n = FieldWeightNode::new("weigh").with_choice(fw("kind", &[("file", 0.1)], 1.0));
        let qp = payload(Some(target_with(vec![fw("kind", &[("class", 0.1)], 1.0)])), SearchOptions::default());
        assert_eq!(first(n, Some(qp)), "b", "le choix du graphe bat l'entité");

        // L'appelant bat le choix du graphe.
        let n = FieldWeightNode::new("weigh").with_choice(fw("kind", &[("file", 0.1)], 1.0));
        let mut options = SearchOptions::default();
        options.field_weights = vec![fw("kind", &[("class", 0.1)], 1.0)];
        let qp = payload(Some(target_with(vec![])), options);
        assert_eq!(first(n, Some(qp)), "a", "l'appelant bat le choix du graphe");
    }

    /// Un champ que l'entité déclare s'applique même quand le nœud en
    /// choisit un autre : les étages se superposent par champ.
    #[test]
    fn fields_from_different_tiers_stack() {
        let mut r1 = make_result("a", 0.9, Some("file"));
        r1.data.as_mut().unwrap().insert("lang".into(), CypherValue::String("rust".into()));
        let mut r2 = make_result("b", 0.8, Some("class"));
        r2.data.as_mut().unwrap().insert("lang".into(), CypherValue::String("python".into()));

        // L'entité dévalue python ; le nœud dévalue file : les deux pèsent.
        let node = FieldWeightNode::new("weigh").with_choice(fw("kind", &[("file", 0.5)], 1.0));
        let qp = payload(Some(target_with(vec![fw("lang", &[("python", 0.1)], 1.0)])), SearchOptions::default());
        let (out, _) = run_node(node, vec![r1, r2], Some(qp));
        assert!((out.iter().find(|r| r.uuid == "a").unwrap().score - 0.45).abs() < 1e-12);
        assert!((out.iter().find(|r| r.uuid == "b").unwrap().score - 0.08).abs() < 1e-12);
    }
}
