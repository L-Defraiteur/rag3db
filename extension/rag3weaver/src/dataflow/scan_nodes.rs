//! **Le balayage des fichiers** : la recherche quand l'index ne sait pas
//! (encore) répondre.
//!
//! Un workspace qui vient d'être ouvert n'a rien d'indexé, et la première
//! question d'un agent arrive avant la fin du premier `index`. Plutôt qu'un
//! zéro résultat qui se lit « ça n'existe pas », la source de fichiers
//! répond : mots exacts, classés, bornés — et la ligne d'état dit que c'est
//! un balayage, pas l'index.
//!
//! Ce nœud **se tait hors mode balayage** : la décision appartient à
//! `SearchSourceNode` (une fois, par l'état de l'index) et descend par la
//! requête. Deux décideurs divergeraient pendant une course et rendraient
//! des doublons index + balayage.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::code_tools::{probable_secret, FileSource, FILE_SOURCE_SERVICE};
use crate::connection::CypherValue;
use crate::search::ChunkInfo;
use crate::search_strategy::UnifiedResult;

use super::node::{Node, NodeContext};
use super::port::{take_or_clone, PortDef, PortValue, QueryPayload};

/// Au-delà, un fichier n'est pas balayé : il est compté dans la ligne de
/// méta. Un journal de 50 Mo ne doit pas coûter la première question.
const MAX_FILE_BYTES: usize = 512 * 1024;

/// L'extrait rendu par fichier, borné : la meilleure ligne, pas le fichier.
const MAX_EXCERPT_CHARS: usize = 240;

pub struct ScanFilesNode {
    node_name: String,
}

impl ScanFilesNode {
    pub fn new(name: &str) -> Self {
        Self { node_name: name.to_string() }
    }
}

/// Les mots de la requête : alphanumériques, minuscules, dédupliqués. Un mot
/// d'une lettre ne discrimine rien, il est ignoré.
fn query_words(query: &str) -> Vec<String> {
    let mut words: Vec<String> = query
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|w| w.chars().count() >= 2)
        .map(|w| w.to_lowercase())
        .collect();
    words.sort();
    words.dedup();
    words
}

/// Ce qu'un fichier balayé a donné : de quoi classer, et de quoi rendre.
struct FileHit {
    path: String,
    /// Combien de mots distincts de la requête ce fichier contient.
    distinct: usize,
    /// Combien de mots distincts la meilleure ligne contient — la proximité :
    /// des mots ensemble sur une ligne valent plus que dispersés.
    best_line_distinct: usize,
    total_occurrences: usize,
    best_line_no: usize,
    best_line: String,
}

fn scan_file(path: &str, content: &str, words: &[String]) -> Option<FileHit> {
    let mut seen = vec![false; words.len()];
    let mut total = 0usize;
    let mut best: (usize, usize, &str) = (0, 0, "");
    for (no, line) in content.lines().enumerate() {
        let lower = line.to_lowercase();
        let mut line_distinct = 0;
        for (i, w) in words.iter().enumerate() {
            let n = lower.matches(w.as_str()).count();
            if n > 0 {
                line_distinct += 1;
                seen[i] = true;
                total += n;
            }
        }
        if line_distinct > best.0 {
            best = (line_distinct, no + 1, line);
        }
    }
    let distinct = seen.iter().filter(|s| **s).count();
    (distinct > 0).then(|| FileHit {
        path: path.to_string(),
        distinct,
        best_line_distinct: best.0,
        total_occurrences: total,
        best_line_no: best.1,
        best_line: best.2.trim().chars().take(MAX_EXCERPT_CHARS).collect(),
    })
}

impl Node for ScanFilesNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "ScanFilesNode"
    }
    fn inputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&super::node_factories::ScanFilesNodeFactory).0
    }
    fn outputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&super::node_factories::ScanFilesNodeFactory).1
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let qp = ctx
            .take_input("query")
            .and_then(|pv| take_or_clone::<QueryPayload>(pv))
            .ok_or("ScanFilesNode: missing 'query' input")?;

        // Hors mode balayage, silence : zéro résultat, zéro méta. L'index
        // répond, ce nœud n'existe pas.
        if !qp.scan {
            ctx.set_output("results", PortValue::new(Vec::<UnifiedResult>::new()));
            return Ok(());
        }

        let empty = |ctx: &mut NodeContext, pourquoi: &str| {
            ctx.warn(pourquoi);
            ctx.set_output("results", PortValue::new(Vec::<UnifiedResult>::new()));
            Ok(())
        };

        let Some(source) = ctx.service::<Arc<dyn FileSource>>(FILE_SOURCE_SERVICE).cloned() else {
            return empty(ctx, "ScanFilesNode: aucune source de fichiers — rien à balayer");
        };
        let words = query_words(&qp.query);
        if words.is_empty() {
            return empty(ctx, "ScanFilesNode: aucun mot d'au moins deux lettres dans la requête");
        }

        // La liste de la source applique déjà les règles d'exclusion du
        // dossier ; la règle des secrets se réapplique ici parce qu'un
        // instantané chargé d'ailleurs peut ne pas l'avoir vue.
        let paths = source.list()?;
        let mut hits: Vec<FileHit> = Vec::new();
        let (mut scanned, mut skipped_size, mut skipped_secret) = (0usize, 0usize, 0usize);
        for path in &paths {
            if probable_secret(path).is_some() {
                skipped_secret += 1;
                continue;
            }
            let Ok(Some(content)) = source.read(path) else { continue };
            if content.len() > MAX_FILE_BYTES {
                skipped_size += 1;
                continue;
            }
            scanned += 1;
            if let Some(hit) = scan_file(path, &content, &words) {
                hits.push(hit);
            }
        }

        // Classement : les mots trouvés d'abord, la proximité ensuite (des
        // mots ensemble sur une ligne), le volume en dernier.
        hits.sort_by(|a, b| {
            (b.distinct, b.best_line_distinct, b.total_occurrences)
                .cmp(&(a.distinct, a.best_line_distinct, a.total_occurrences))
                .then_with(|| a.path.cmp(&b.path))
        });

        let limit = qp.options.limit.max(1);
        let total_hits = hits.len();
        let shown = hits.len().min(limit);
        let results: Vec<UnifiedResult> = hits
            .into_iter()
            .take(limit)
            .map(|h| {
                let name = h.path.rsplit('/').next().unwrap_or(&h.path).to_string();
                let mut data = BTreeMap::new();
                data.insert("name".to_string(), CypherValue::String(name));
                data.insert("file_path".to_string(), CypherValue::String(h.path.clone()));
                UnifiedResult {
                    uuid: format!("scan:{}", h.path),
                    // Le score dit le classement, pas une probabilité : il
                    // n'est pas comparable à un score d'index.
                    score: h.distinct as f64 + h.best_line_distinct as f64 / 100.0,
                    entity: Some(qp.target_name.clone()),
                    data: Some(data),
                    chunk: Some(ChunkInfo {
                        uuid: String::new(),
                        text: h.best_line,
                        index: 0,
                        score: h.distinct as f64,
                        start_line: h.best_line_no,
                        end_line: h.best_line_no,
                        start_char: 0,
                        end_char: 0,
                    }),
                    chunks: None,
                    relation: None,
                    matched_children: None,
                    other_children: None,
                    graph: None,
                    signal: Some("scan".to_string()),
                }
            })
            .collect();

        // « Combien d'autres » : le balayage est borné, et il le dit.
        let mut ligne = format!(
            "balayage : {scanned} fichiers lus, {total_hits} contiennent les mots"
        );
        if total_hits > shown {
            ligne.push_str(&format!(", {} au-delà de la limite", total_hits - shown));
        }
        if skipped_size > 0 {
            ligne.push_str(&format!(", {skipped_size} sautés (plus de 512 Ko)"));
        }
        if skipped_secret > 0 {
            ligne.push_str(&format!(", {skipped_secret} écartés (secrets probables)"));
        }
        ctx.metric("scanned", scanned as f64);
        ctx.metric("hits", total_hits as f64);
        ctx.set_output(
            "meta",
            PortValue::new(crate::search::SearchMeta {
                query: qp.query.clone(),
                target: qp.target_name.clone(),
                signals: crate::search::SearchSignals::NONE,
                consistency: qp.options.consistency,
                partial: false,
                pending_count: 0,
                vector_count: 0,
                bm25_count: 0,
                sparse_count: 0,
                fused_count: shown,
                reranked_count: 0,
                warnings: vec![ligne],
                search_time_ms: 0,
                diagnostics: None,
            }),
        );
        ctx.set_output("results", PortValue::new(results));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code_tools::Snapshot;
    use crate::dataflow::ServiceRegistry;
    use crate::search::SearchOptions;

    fn payload(query: &str, scan: bool, limit: usize) -> QueryPayload {
        QueryPayload {
            target_name: "Scope".into(),
            query: query.into(),
            options: SearchOptions { limit, ..Default::default() },
            target: None,
            embedding: None,
            sparse: None,
            scan,
        }
    }

    fn contexte(files: &[(&str, &str)]) -> NodeContext {
        let mut services = ServiceRegistry::new();
        let source: Arc<dyn FileSource> = Arc::new(Snapshot::new(
            "scan-test",
            files.iter().map(|(p, c)| (p.to_string(), c.to_string())),
        ));
        services.register(FILE_SOURCE_SERVICE, source);
        NodeContext::with_services(Arc::new(services))
    }

    fn sorties(ctx: &mut NodeContext) -> (Vec<UnifiedResult>, Option<crate::search::SearchMeta>) {
        let mut outs = ctx.drain_outputs();
        let results = outs
            .remove("results")
            .and_then(take_or_clone::<Vec<UnifiedResult>>)
            .unwrap_or_default();
        let meta = outs.remove("meta").and_then(take_or_clone::<crate::search::SearchMeta>);
        (results, meta)
    }

    /// **Hors mode balayage, silence.** Le nœud est présent dans le graphe en
    /// permanence : c'est la requête qui dit s'il agit.
    #[test]
    fn hors_mode_scan_le_noeud_se_tait() {
        let mut ctx = contexte(&[("a.rs", "la fonction depart existe")]);
        ctx.set_input("query", PortValue::new(payload("depart", false, 10)));
        ScanFilesNode::new("scan").execute(&mut ctx).unwrap();
        let (r, meta) = sorties(&mut ctx);
        assert!(r.is_empty());
        assert!(meta.is_none(), "pas de méta hors mode scan");
    }

    /// **Classement : les mots trouvés d'abord, la proximité ensuite.** Un
    /// fichier avec les deux mots bat un fichier avec un seul ; à mots
    /// égaux, les mots sur la même ligne battent les mots dispersés.
    #[test]
    fn le_classement_mots_trouves_puis_proximite() {
        let mut ctx = contexte(&[
            ("un_seul.rs", "le moteur tourne\n"),
            ("disperse.rs", "le moteur\n\nla recherche\n"),
            ("ensemble.rs", "le moteur de recherche\n"),
        ]);
        ctx.set_input("query", PortValue::new(payload("moteur recherche", true, 10)));
        ScanFilesNode::new("scan").execute(&mut ctx).unwrap();
        let (r, _) = sorties(&mut ctx);
        let chemins: Vec<&str> = r.iter().map(|u| u.uuid.as_str()).collect();
        assert_eq!(
            chemins,
            ["scan:ensemble.rs", "scan:disperse.rs", "scan:un_seul.rs"],
            "{chemins:?}"
        );
        // La forme est celle de la recherche : data, chunk, signal.
        let premier = &r[0];
        assert_eq!(premier.signal.as_deref(), Some("scan"));
        let chunk = premier.chunk.as_ref().unwrap();
        assert_eq!(chunk.start_line, 1);
        assert!(chunk.text.contains("moteur de recherche"), "{}", chunk.text);
    }

    /// **Borné, et il le dit** : la limite tronque, la méta compte le reste ;
    /// un secret probable n'est jamais balayé.
    #[test]
    fn le_balayage_est_borne_et_avoue() {
        let files: Vec<(String, String)> = (0..5)
            .map(|i| (format!("f{i}.rs"), "le mot cherche est là\n".to_string()))
            .chain([(".env".to_string(), "cherche SECRET=oui\n".to_string())])
            .collect();
        let refs: Vec<(&str, &str)> = files.iter().map(|(p, c)| (p.as_str(), c.as_str())).collect();
        let mut ctx = contexte(&refs);
        ctx.set_input("query", PortValue::new(payload("cherche", true, 2)));
        ScanFilesNode::new("scan").execute(&mut ctx).unwrap();
        let (r, meta) = sorties(&mut ctx);
        assert_eq!(r.len(), 2, "le budget de rendu borne");
        assert!(!r.iter().any(|u| u.uuid.contains(".env")), "les secrets restent dehors");
        let ligne = meta.expect("une méta de balayage").warnings.join(" ");
        assert!(ligne.contains("3 au-delà de la limite"), "{ligne}");
        assert!(ligne.contains("écartés (secrets probables)"), "{ligne}");
    }
}
