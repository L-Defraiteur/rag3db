//! **Un saut d'appel autour des premiers rangs** (lot `call-hop`, 23 novembre 2026, essai 6 du banc).
//!
//! Un symptôme nomme ce que sa cause appelle, pas la cause : « save file fts flush sync » trouve
//! `flush_blob_store`, le défaut vit chez son appelant `commit_lot_after_native`, dont le texte ne porte aucun de ces
//! mots hors de l'appel. Le gabarit de recherche déclare donc un saut le long des appels : pour chacun des `from`
//! premiers résultats, ses appelants (la machinerie de `usages`, genre `call` seul), classés par recoupement avec la
//! requête, les `per_source` meilleurs posés juste après leur source, un epsilon sous son score, « via call ».
//!
//! **Le parseur dit la vérité, le gabarit compose** : aucune arête n'est déduite ici, le nœud lit les usages que le
//! graphe porte déjà, par les relations que le gabarit nomme (`pivot`, `defined_by`, `used_by`, `direct`, comme
//! `usages.mmd`). `from=0` est la valeur neutre : le nœud laisse passer les résultats tels quels, comme
//! `RerankNode(candidates=0)`. Rien n'est écrit.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, Mutex};

use super::node::{Node, NodeContext};
use super::node_registry::{ConfigParam, ConfigParamType, NodeFactory, NodeSchema};
use super::port::{take_or_clone, PortDef, PortType, PortValue, QueryPayload};
use super::resultat::UnifiedResult;
use super::services::ConnService;
use super::usage_nodes::{usages_config_from, usages_of_in, QueryScope, Usage, UsagesConfig};
use crate::catalog::Catalog;
use crate::connection::CypherValue;
use crate::read_view::ReadCatalog;
use crate::search::SearchResult;

/// Le genre d'usage suivi : un appel.
const CALL: &str = "call";

/// L'écart sous le score de la source : l'appelant la suit, devant la ligne d'après.
const EPSILON: f64 = 1e-9;

/// `CallersHopNode` : **inputs** `results` (Results), `query` (Query) ; **output** `results`. Services : `catalog`
/// (ou la vue de lecture), `conn`.
pub struct CallersHopNode {
    node_name: String,
    /// Les premiers résultats d'où l'on saute ; `0` : passe.
    from: usize,
    /// Appelants gardés au plus par source.
    per_source: usize,
    /// Les champs de l'appelant lus pour le recoupement avec la requête.
    text_fields: Vec<String>,
    /// La provenance posée sur un appelant (`relation`, rendue « via … »).
    relation: String,
    cfg: UsagesConfig,
    scope: QueryScope,
}

/// **Les mots de la requête** : en minuscules, sans doublon, d'au moins deux caractères.
pub fn query_words(query: &str) -> Vec<String> {
    let mut vus = HashSet::new();
    query
        .split(|c: char| !c.is_alphanumeric())
        .map(str::to_lowercase)
        .filter(|w| w.chars().count() >= 2)
        .filter(|w| vus.insert(w.clone()))
        .collect()
}

/// **Le recoupement** : combien de mots de la requête le texte porte, sans distinguer la casse.
pub fn overlap(words: &[String], text: &str) -> usize {
    let text = text.to_lowercase();
    words.iter().filter(|w| text.contains(w.as_str())).count()
}

/// Le texte d'un appelant : ses champs déclarés, bout à bout.
fn text_of(data: &BTreeMap<String, CypherValue>, fields: &[String]) -> String {
    fields.iter().filter_map(|f| data.get(f).and_then(|v| v.as_str())).collect::<Vec<_>>().join("\n")
}

/// **Les appelants gardés d'une source** : ceux qui recoupent la requête (un recoupement nul écarte), du meilleur
/// au moindre (à égalité, l'ordre d'arrivée), `per_source` au plus.
pub fn keep_callers(words: &[String], candidates: Vec<SearchResult>, fields: &[String], per_source: usize) -> Vec<SearchResult> {
    let mut notes: Vec<(usize, usize, SearchResult)> = candidates
        .into_iter()
        .enumerate()
        .filter_map(|(i, c)| {
            let n = c.data.as_ref().map(|d| overlap(words, &text_of(d, fields))).unwrap_or(0);
            (n > 0).then_some((n, i, c))
        })
        .collect();
    notes.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    notes.into_iter().take(per_source).map(|(_, _, c)| c).collect()
}

/// **Placer les appelants** : chacun juste après sa source, s'il n'est pas déjà devant ; plus bas dans la liste, il
/// est remonté (sa ligne d'origine, re-notée). Son score : celui de la source, un epsilon en dessous.
pub fn place_callers(results: Vec<UnifiedResult>, callers: &HashMap<usize, Vec<UnifiedResult>>, relation: &str) -> Vec<UnifiedResult> {
    let mut restants: Vec<Option<UnifiedResult>> = results.into_iter().map(Some).collect();
    let rang: HashMap<String, usize> = restants.iter().enumerate().rev().filter_map(|(i, r)| r.as_ref().map(|r| (r.uuid.clone(), i))).collect();
    let mut out: Vec<UnifiedResult> = Vec::with_capacity(restants.len());
    let mut places: HashSet<String> = HashSet::new();
    for i in 0..restants.len() {
        let Some(r) = restants[i].take() else { continue };
        if !places.insert(r.uuid.clone()) {
            continue;
        }
        let score = r.score;
        out.push(r);
        for (k, appelant) in callers.get(&i).into_iter().flatten().enumerate() {
            if places.contains(&appelant.uuid) {
                continue;
            }
            // Plus bas dans la liste : sa ligne remonte, avec ce que les signaux en ont dit.
            let mut ligne = match rang.get(&appelant.uuid).filter(|j| **j > i) {
                Some(j) => restants[*j].take().unwrap_or_else(|| appelant.clone()),
                None => appelant.clone(),
            };
            ligne.score = score - EPSILON * (k + 1) as f64;
            ligne.relation = Some(relation.to_string());
            places.insert(ligne.uuid.clone());
            out.push(ligne);
        }
    }
    out
}

impl CallersHopNode {
    /// Les appelants d'une source, par les usages : genre `call`, attribués à cette définition.
    fn callers_of(&self, cat: &dyn ReadCatalog, name: &str, source_uuid: &str) -> Result<Vec<String>, String> {
        let report = usages_of_in(cat, &self.cfg, name, "", &self.scope)?;
        let mut vus = HashSet::new();
        Ok(report
            .usages
            .iter()
            .filter(|u: &&Usage| u.usage == CALL && u.definition.as_deref() == Some(source_uuid))
            .map(|u| u.user.uuid.clone())
            .filter(|u| u != source_uuid && vus.insert(u.clone()))
            .collect())
    }
}

impl Node for CallersHopNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "CallersHopNode"
    }
    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        Some(Box::new(serde_json::json!({ "from": self.from, "per_source": self.per_source, "relation": self.relation })))
    }
    fn inputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&CallersHopNodeFactory).0
    }
    fn outputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&CallersHopNodeFactory).1
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let mut results = ctx
            .take_input("results")
            .and_then(|pv| take_or_clone::<Vec<UnifiedResult>>(pv))
            .ok_or("CallersHopNode: missing 'results' input")?;
        let qp = ctx.take_input("query").and_then(|pv| take_or_clone::<QueryPayload>(pv)).ok_or("CallersHopNode: missing 'query' input")?;
        let words = query_words(&qp.query);
        // La valeur neutre, le balayage, une requête sans mot : on passe, sans rien dire.
        let (Some(target), false, false, false) = (qp.target.as_ref(), self.from == 0, qp.scan, words.is_empty() || results.is_empty()) else {
            ctx.set_output("results", PortValue::new(results));
            return Ok(());
        };
        // L'entité des lignes rendues et leurs champs, comme `ResolveParentNode`.
        let (entity, fields) = match &target.occurrence_projection {
            Some(p) => (p.entity.clone(), p.fields.clone()),
            None => (target.parent_table.clone(), target.enrich_fields.clone()),
        };
        let mut fields = fields;
        for f in self.text_fields.iter().chain(std::iter::once(&self.cfg.title)) {
            if !fields.contains(f) {
                fields.push(f.clone());
            }
        }
        let Some(conn) = ctx.service::<ConnService>("conn").map(|c| c.0.clone()) else {
            ctx.set_output("results", PortValue::new(results));
            return Ok(());
        };
        let backend = super::generic_search_nodes::moteur_de_recherche(ctx, &conn);
        let tete = self.from.min(results.len());

        // Le nom des sources : leur ligne, lue quand la tête n'a pas encore ses données.
        let mut noms: Vec<Option<String>> = results[..tete].iter().map(|r| nom_de(r, &self.cfg.title)).collect();
        if noms.iter().any(Option::is_none) {
            let mut plats: Vec<SearchResult> = results[..tete].iter().cloned().map(SearchResult::from).collect();
            crate::search::enrich_results_with_data_via_backend(backend.as_ref(), &entity, &fields, &mut plats)
                .map_err(|e| format!("CallersHopNode: {e}"))?;
            for (i, p) in plats.into_iter().enumerate() {
                if noms[i].is_none() {
                    noms[i] = p.data.as_ref().and_then(|d| d.get(&self.cfg.title)).and_then(|v| v.as_str()).map(str::to_string);
                    if results[i].data.is_none() {
                        results[i].data = p.data;
                    }
                }
            }
        }

        // Les appelants de chaque source, par la vue de lecture (sinon le catalogue, le temps d'une lecture).
        let vue = super::generic_search_nodes::vue_de(ctx);
        let catalog = ctx.service::<Arc<Mutex<Catalog>>>("catalog").cloned();
        let par_source: Vec<(usize, Vec<String>)> = {
            let garde;
            let cat: &dyn ReadCatalog = match (&vue, &catalog) {
                (Some(v), _) => &**v,
                (None, Some(c)) => match super::catalog_read::lock_for_read(c) {
                    Some(g) => {
                        garde = g;
                        &*garde
                    }
                    None => {
                        ctx.warn("CallersHopNode: catalogue tenu — résultats rendus sans le saut d'appel");
                        ctx.set_output("results", PortValue::new(results));
                        return Ok(());
                    }
                },
                (None, None) => {
                    ctx.set_output("results", PortValue::new(results));
                    return Ok(());
                }
            };
            // Un catalogue sans le pivot déclaré (une autre forme que le code) : rien à suivre.
            if cat.entity_config(&self.cfg.pivot).is_none() {
                ctx.set_output("results", PortValue::new(results));
                return Ok(());
            }
            let mut v = Vec::new();
            for (i, nom) in noms.iter().enumerate() {
                let Some(nom) = nom.as_deref().filter(|n| !n.is_empty()) else { continue };
                let appelants = self.callers_of(cat, nom, &results[i].uuid)?;
                if !appelants.is_empty() {
                    v.push((i, appelants));
                }
            }
            v
        };

        // Leurs lignes, du même grain que les résultats, puis les meilleures par recoupement.
        let mut callers: HashMap<usize, Vec<UnifiedResult>> = HashMap::new();
        let mut trouves = 0usize;
        for (i, uuids) in par_source {
            let mut plats: Vec<SearchResult> = uuids
                .into_iter()
                .map(|uuid| SearchResult { uuid, score: 0.0, entity: Some(entity.clone()), data: None, chunk: None, chunks: None, sans_vecteur: false })
                .collect();
            crate::search::enrich_results_with_data_via_backend(backend.as_ref(), &entity, &fields, &mut plats)
                .map_err(|e| format!("CallersHopNode: {e}"))?;
            let gardes = keep_callers(&words, plats, &self.text_fields, self.per_source);
            trouves += gardes.len();
            if !gardes.is_empty() {
                callers.insert(i, gardes.into_iter().map(UnifiedResult::from).collect());
            }
        }
        ctx.metric("callers", trouves as f64);
        ctx.set_output("results", PortValue::new(place_callers(results, &callers, &self.relation)));
        Ok(())
    }
}

/// Le nom d'un résultat, quand ses données le portent.
fn nom_de(r: &UnifiedResult, title: &str) -> Option<String> {
    r.data.as_ref().and_then(|d| d.get(title)).and_then(|v| v.as_str()).filter(|n| !n.is_empty()).map(str::to_string)
}

/// Factory de [`CallersHopNode`].
pub struct CallersHopNodeFactory;

impl NodeFactory for CallersHopNodeFactory {
    // L6 : ne fait que lire.
    fn snapshot_safe(&self, _config: &serde_json::Value) -> bool {
        true
    }
    fn create(&self, name: &str, config: &serde_json::Value) -> Result<Box<dyn Node>, String> {
        let s = |k: &str| config.get(k).and_then(|v| v.as_str()).map(str::to_string).unwrap_or_default();
        let entier = |k: &str, defaut: u64| -> Result<usize, String> {
            match config.get(k) {
                None | Some(serde_json::Value::Null) => Ok(defaut as usize),
                Some(v) => v.as_u64().map(|n| n as usize).ok_or_else(|| format!("CallersHopNode: « {k} » doit être un entier positif")),
            }
        };
        let text_fields: Vec<String> = {
            let t = s("text_fields");
            let t = if t.trim().is_empty() { "name|docstring|content".to_string() } else { t };
            t.split(['|', ',']).map(str::trim).filter(|x| !x.is_empty()).map(String::from).collect()
        };
        let relation = Some(s("relation")).filter(|r| !r.is_empty()).unwrap_or_else(|| CALL.to_string());
        Ok(Box::new(CallersHopNode {
            node_name: name.to_string(),
            from: entier("from", 3)?,
            per_source: entier("per_source", 2)?,
            text_fields,
            relation,
            cfg: usages_config_from(config, "CallersHopNode")?,
            scope: QueryScope::parse(&s("scope"), &s("language")),
        }))
    }
    fn node_type(&self) -> &'static str {
        "CallersHopNode"
    }
    fn schema(&self) -> NodeSchema {
        let p = |name: &'static str, param_type, required, default: Option<serde_json::Value>, description: &'static str| ConfigParam {
            name: name.into(),
            param_type,
            required,
            default,
            description: description.into(),
            choices: None,
            json_schema: None,
        };
        use ConfigParamType::{Int, String as S};
        // La déclaration des usages, celle de `UsagesNode` : le gabarit dit comment trouver les appelants.
        const DECLARATION: [&str; 15] = [
            "pivot", "key", "defined_by", "used_by", "direct", "group_by", "usages_field", "line", "title", "kind_field",
            "path_fields", "line_field", "edge_field", "edge_guessed", "edge_doubtful",
        ];
        let mut params = vec![
            p("from", Int, false, Some(serde_json::json!(3)), "Les premiers résultats d'où l'on saute ; 0 = passe-plat exact"),
            p("per_source", Int, false, Some(serde_json::json!(2)), "Appelants gardés au plus par source, par recoupement avec la requête"),
            p("text_fields", S, false, Some(serde_json::json!("name|docstring|content")), "Champs de l'appelant lus pour le recoupement, séparés par |"),
            p("relation", S, false, Some(serde_json::json!(CALL)), "La provenance posée sur un appelant (rendue « via … »)"),
            p("qualifier_field", S, false, Some(serde_json::json!("")), "Champ de la définition qui porte son conteneur (nom qualifié)"),
            p("scope", S, false, Some(serde_json::json!("")), super::usage_nodes::SCOPE_HELP),
            p("language", S, false, Some(serde_json::json!("")), super::usage_nodes::LANGUAGE_HELP),
        ];
        params.extend(super::usage_nodes::UsagesNodeFactory.schema().config_params.into_iter().filter(|c| DECLARATION.contains(&c.name.as_ref())));
        NodeSchema {
            node_type: "CallersHopNode".into(),
            description: "One hop along the calls around the first ranks: the callers of each of the first `from` results (the usages of kind call, through the declared pivot and relations), ranked by how many query words they carry, the best `per_source` placed right after their source, just under its score, marked « via call ». from=0 passes the results through.".into(),
            inputs: vec![PortDef::value("results", PortType::Results, true), PortDef::value("query", PortType::Query, true)],
            outputs: vec![PortDef::value("results", PortType::Results, false)],
            config_params: params,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::SearchOptions;

    fn ligne(uuid: &str, score: f64, name: &str, content: &str) -> UnifiedResult {
        let data = BTreeMap::from([
            ("name".to_string(), CypherValue::String(name.into())),
            ("content".to_string(), CypherValue::String(content.into())),
        ]);
        UnifiedResult::from(SearchResult { uuid: uuid.into(), score, entity: Some("Scope".into()), data: Some(data), chunk: None, chunks: None, sans_vecteur: false })
    }

    /// `from=0` est la valeur neutre : les résultats passent tels quels, sans qu'aucun service soit consulté.
    #[test]
    fn from_zero_passes_the_results_through() {
        let mut node = CallersHopNodeFactory
            .create("hop", &serde_json::json!({"from": 0, "pivot": "Symbol", "key": "name", "defined_by": "DEFINES", "used_by": "MENTIONS"}))
            .unwrap();
        let mut ctx = NodeContext::new();
        ctx.set_input("results", PortValue::new(vec![ligne("a", 0.9, "flush_blob_store", ""), ligne("b", 0.5, "save", "")]));
        ctx.set_input(
            "query",
            PortValue::new(QueryPayload {
                target_name: "Scope".into(),
                query: "save file fts flush sync".into(),
                options: SearchOptions::default(),
                target: None,
                embedding: None,
                sparse: None,
                scan: false,
            }),
        );
        node.execute(&mut ctx).unwrap();
        let out = ctx.drain_outputs().get("results").and_then(|pv| pv.downcast::<Vec<UnifiedResult>>()).cloned().expect("results");
        assert_eq!(out.iter().map(|r| (r.uuid.as_str(), r.score)).collect::<Vec<_>>(), vec![("a", 0.9), ("b", 0.5)]);
        assert!(out.iter().all(|r| r.relation.is_none()));
    }

    /// Un appelant qui ne porte aucun mot de la requête est écarté ; les autres sont rangés par recoupement.
    #[test]
    fn a_caller_without_overlap_is_dropped() {
        let words = query_words("save file fts flush sync");
        let plat = |u: &str, name: &str, content: &str| SearchResult::from(ligne(u, 0.0, name, content));
        let fields = vec!["name".to_string(), "content".to_string()];
        let gardes = keep_callers(
            &words,
            vec![plat("x", "unrelated", "let a = b;"), plat("y", "commit_lot", "flush_blob_store(lot)"), plat("z", "save_and_sync", "flush()")],
            &fields,
            2,
        );
        assert_eq!(gardes.iter().map(|c| c.uuid.as_str()).collect::<Vec<_>>(), vec!["z", "y"]);
    }

    /// L'appelant se place juste après sa source, un epsilon sous elle ; plus bas dans la liste, il est remonté.
    #[test]
    fn a_caller_lower_in_the_list_is_raised_after_its_source() {
        let results = vec![ligne("a", 0.9, "callee", ""), ligne("b", 0.8, "other", ""), ligne("c", 0.1, "caller", "")];
        let callers = HashMap::from([(0usize, vec![ligne("c", 0.0, "caller", "")])]);
        let out = place_callers(results, &callers, "call");
        assert_eq!(out.iter().map(|r| r.uuid.as_str()).collect::<Vec<_>>(), vec!["a", "c", "b"]);
        assert!(out[1].score < 0.9 && out[1].score > 0.8);
        assert_eq!(out[1].relation.as_deref(), Some("call"));
    }
}
