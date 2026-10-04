//! « Indexer ce dépôt », côté outils : `EstimateNode` — dire ce que ça va
//! coûter avant d'indexer — et `IndexNode` — indexer en fond, et rendre un
//! reçu.
//!
//! # Le reçu est un journal
//!
//! `index` ne porte pas l'indexation dans un tour de parole : il la lance et
//! rend le chemin d'un journal, au même contrat que ceux de `run_bg`
//! (`run_nodes::dossier_journaux`). L'avancement s'y écrit en lignes ; `wait`
//! les attrape par son motif, et avec `timeout_s=0` les lit sans attendre. La
//! dernière ligne est [`DONE_LINE`], ou commence par [`FAILED_PREFIX`].
//!
//! # Ce que l'avancement dit, et ce qu'il ne promet pas
//!
//! Deux temps : le plein texte d'abord (`sync_source` en exigeant
//! `RECHERCHE_TEXTE`), les vecteurs ensuite, par passes. Mesuré le 3 octobre
//! 2026, le premier temps **croît plus vite que la taille** du dépôt —
//! l'insertion des relations par le moteur — : 17 s pour 122 fichiers, 30 min
//! pour 6 735. Le journal dit donc où l'on en est en fichiers, pas un temps
//! restant qu'il ne saurait pas tenir. Pour les vecteurs, le reste se dit en
//! temps : le débit est mesuré.
//!
//! Pendant le premier temps, le catalogue est tenu par la synchronisation ;
//! pendant le second, il est rendu entre deux passes, et une recherche par
//! mots passe.
//!
//! Le nœud ne fait que réunir ce que le service lui donne — la source de
//! fichiers, le catalogue — et appeler [`crate::estimate`]. Il **ne lit aucun
//! fichier pour compter** : des noms et des tailles. Il n'en lit quelques-uns
//! que pour sonder le débit de l'embarqueur, la première fois, quand le
//! catalogue n'en a pas encore noté.

use std::sync::{Arc, Mutex};

use super::node::{Node, NodeContext};
use super::node_registry::{Choices, ConfigParam, ConfigParamType, NodeFactory, NodeSchema};
use super::port::{PortDef, PortType, PortValue};
use crate::catalog::Catalog;
use crate::code_tools::{source_service, FileSource, ToolFormat, WorkingTree};
use crate::estimate::{code_policy, estimate_here, source_files, working_tree_files, Estimate, Kept, Rate};
use crate::generated::{GeneratedPolicy, GENERATED_POLICY_SERVICE};

/// Combien de morceaux la sonde embarque, et leur taille en caractères.
const PROBE_SAMPLES: usize = 64;
const PROBE_CHARS: usize = 1_200;

/// `estimate` : ce que la source contient, le modèle du premier index et
/// pourquoi, la durée prévue des vecteurs, et si `index` demandera une
/// confirmation. **Output** `result` (PortType::Map). Services :
/// `file_source` (requis), `catalog` (pour le débit ; sans lui, pas de durée).
pub struct EstimateNode {
    node_name: String,
    probe: bool,
    format: ToolFormat,
}

impl EstimateNode {
    pub fn new(name: &str) -> Self {
        Self { node_name: name.to_string(), probe: true, format: ToolFormat::Markdown }
    }
    /// Sonder le débit quand le catalogue n'en a pas noté (défaut : oui).
    pub fn with_probe(mut self, probe: bool) -> Self {
        self.probe = probe;
        self
    }
    pub fn with_format(mut self, format: ToolFormat) -> Self {
        self.format = format;
        self
    }
}

/// La politique des fichiers générés que le backend a rangée en service
/// (`workspace.generated` du manifeste) ; sans elle, les défauts.
fn generated_policy(ctx: &NodeContext) -> GeneratedPolicy {
    ctx.service::<GeneratedPolicy>(GENERATED_POLICY_SERVICE).cloned().unwrap_or_default()
}

/// Les noms et les tailles de la source. Un dossier se liste sans rien lire,
/// exclusions comprises ; une autre source se lit — elle ne sait pas dire une
/// taille autrement.
fn files_of(source: &dyn FileSource) -> Result<(Vec<(String, u64)>, Vec<(String, &'static str)>), String> {
    match source.cursor().strip_prefix("worktree:") {
        Some(root) => working_tree_files(&WorkingTree::new(root)),
        None => Ok((source_files(source)?, Vec::new())),
    }
}

/// Des morceaux pris dans les premiers fichiers retenus : la matière de la sonde.
fn probe_samples(source: &dyn FileSource, files: &[(String, u64)], policy: &dyn Fn(&str, u64) -> Kept) -> Vec<String> {
    let mut samples = Vec::new();
    for (path, bytes) in files {
        if samples.len() >= PROBE_SAMPLES {
            break;
        }
        if !matches!(policy(path, *bytes), Kept::Yes(_)) {
            continue;
        }
        let Ok(Some(content)) = source.read(path) else { continue };
        let chars: Vec<char> = content.chars().collect();
        samples.extend(chars.chunks(PROBE_CHARS).take(PROBE_SAMPLES - samples.len()).map(|c| c.iter().collect::<String>()));
    }
    samples
}

impl Node for EstimateNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "EstimateNode"
    }
    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        Some(Box::new(serde_json::json!({ "probe": self.probe })))
    }
    fn outputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&EstimateNodeFactory).1
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let source = source_service(ctx).ok_or("EstimateNode: 'file_source' service not found")?;
        let catalog = ctx.service::<Arc<Mutex<Catalog>>>("catalog").cloned();
        let estimate =
            estimate_of(source.as_ref(), catalog.as_ref(), self.probe, &generated_policy(ctx)).map_err(|e| format!("EstimateNode: {e}"))?;
        ctx.metric("files", estimate.survey.files as f64);
        ctx.metric("bytes", estimate.survey.bytes as f64);
        ctx.metric("skipped", estimate.survey.skipped_files() as f64);
        if let Some(s) = estimate.vectors_seconds {
            ctx.metric("vectors_seconds", s as f64);
        }
        let value = match self.format {
            ToolFormat::Markdown => serde_json::Value::String(estimate.text()),
            ToolFormat::Json => serde_json::to_value(&estimate).map_err(|e| e.to_string())?,
        };
        ctx.set_output("result", PortValue::new(value));
        Ok(())
    }
}

/// L'estimation d'une source, pour les deux nœuds. Le débit est celui que le
/// catalogue a noté, sinon une sonde — notée à son tour, pour que
/// l'estimation suivante ne la refasse pas. Sans catalogue : pas de durée.
fn estimate_of(
    source: &dyn FileSource,
    catalog: Option<&Arc<Mutex<Catalog>>>,
    probe: bool,
    generated: &GeneratedPolicy,
) -> Result<Estimate, String> {
    let (files, excluded) = files_of(source)?;
    // Les générés sortent de la politique avec leur raison et leur poids :
    // ni comptés, ni sondés, ni indexés ensuite.
    let generated = crate::estimate::generated_among(source, &files, generated, code_policy);
    let policy = |path: &str, bytes: u64| match generated.get(path) {
        Some(reason) => Kept::No(reason),
        None => code_policy(path, bytes),
    };
    let (rate, remote) = match catalog {
        None => (None, false),
        Some(catalog) => {
            let guard = catalog.lock().unwrap();
            let rate = match guard.known_embedding_rate().map_err(|e| e.to_string())? {
                Some(rate) => Some(rate),
                None if probe => guard.probe_embedding_rate(&probe_samples(source, &files, &policy)).map_err(|e| format!("sonde : {e}"))?,
                None => None,
            };
            (rate, guard.embedder_is_remote())
        }
    };
    Ok(estimate_here(&files, &excluded, policy, rate, remote))
}

pub struct EstimateNodeFactory;

impl NodeFactory for EstimateNodeFactory {
    fn create(&self, name: &str, config: &serde_json::Value) -> Result<Box<dyn Node>, String> {
        let mut node = EstimateNode::new(name);
        if let Some(p) = config.get("probe").and_then(|v| v.as_bool()) {
            node = node.with_probe(p);
        }
        if let Some(f) = config.get("format").and_then(|v| v.as_str()) {
            node = node.with_format(ToolFormat::parse(f).map_err(|e| format!("EstimateNode: {e}"))?);
        }
        Ok(Box::new(node))
    }
    fn node_type(&self) -> &'static str {
        "EstimateNode"
    }
    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "EstimateNode",
            description: "Estimates what indexing the file source will cost: files kept and skipped (with reasons), first-index model and why, predicted vector time, and whether indexing will ask for confirmation. Writes nothing.",
            inputs: vec![],
            outputs: vec![PortDef { name: "result", port_type: PortType::Map, required: false }],
            config_params: vec![
                ConfigParam {
                    name: "probe",
                    param_type: ConfigParamType::Bool,
                    required: false,
                    default: Some(serde_json::json!(true)),
                    description: "Sonder le débit de l'embarqueur (une à deux secondes) quand aucun n'est noté ; sans sonde ni débit noté, la durée est dite inconnue",
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "format",
                    param_type: ConfigParamType::String,
                    required: false,
                    default: Some(serde_json::json!("markdown")),
                    description: "markdown (compact, pour le modèle) | json (structuré)",
                    choices: Some(Choices::fixed(["markdown", "json"])),
                    json_schema: None,
                },
            ],
        }
    }
}

// ─── IndexNode ───────────────────────────────────────────────────────────────

/// La dernière ligne d'un journal d'indexation qui a abouti. C'est le motif
/// que `wait` attend ; il ne change pas.
pub const DONE_LINE: &str = "indexation terminée";
/// Le début de la dernière ligne d'un journal d'indexation qui a échoué.
pub const FAILED_PREFIX: &str = "indexation échouée";

/// Un journal neuf, là où `wait` accepte d'attendre.
pub fn new_index_journal() -> std::io::Result<std::path::PathBuf> {
    let dir = super::run_nodes::dossier_journaux().join("index");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}-{}.out", crate::dataflow::checkpoint::timestamp_ms(), std::process::id()));
    std::fs::write(&path, "")?;
    Ok(path)
}

fn log(journal: &std::path::Path, line: &str) {
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new().append(true).create(true).open(journal) {
        let _ = writeln!(f, "{line}");
    }
}

/// **Indexer en fond.** Rend la main tout de suite ; le journal dit la suite.
/// `kept_bytes` : les octets retenus par l'estimation, pour dire le reste des
/// vecteurs en temps.
pub fn spawn_index(
    catalog: Arc<Mutex<Catalog>>,
    source: Arc<dyn FileSource>,
    journal: std::path::PathBuf,
    kept_bytes: u64,
    generated: GeneratedPolicy,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || match run_index(&catalog, source.as_ref(), &journal, kept_bytes, generated) {
        Ok(()) => log(&journal, DONE_LINE),
        Err(e) => log(&journal, &format!("{FAILED_PREFIX} : {e}")),
    })
}

fn run_index(
    catalog: &Arc<Mutex<Catalog>>,
    source: &dyn FileSource,
    journal: &std::path::Path,
    kept_bytes: u64,
    generated: GeneratedPolicy,
) -> Result<(), String> {
    use crate::code_sync::{sync_source, SourceSyncOptions, SourceSyncProgress, SyncPhase};
    use crate::disponibilite::Disponibilites;

    // L'état que la recherche lit à chaque appel, **par entité** : celui de
    // `Scope` ne doit rien au journal d'une conversation rangé dans la même
    // base. Une réindexation ne fait pas régresser un niveau déjà prêt.
    const ENTITIES: [&str; 4] = [crate::code::FILE, crate::code::SCOPE, crate::code::LIBRARY, crate::code::SYMBOL];

    // Premier temps : les lignes et le plein texte, les vecteurs en dette.
    let options = SourceSyncOptions { exige: Disponibilites::RECHERCHE_TEXTE, generated, ..Default::default() };
    let report = {
        let mut guard = catalog.lock().map_err(|_| "catalogue empoisonné".to_string())?;
        guard.note_indexing_started(&ENTITIES).map_err(|e| e.to_string())?;
        sync_source(&mut guard, source, &options, &mut |p: SourceSyncProgress| match p.phase {
            // Les paquets : les mots. En masse, les liens s'accumulent en
            // file ; un fichier édité après son passage sera repris à la fin.
            SyncPhase::Nodes => {
                let mut line = format!("plein texte : {} fichiers sur {} ({} scopes)", p.files_done, p.files_total, p.scopes_written);
                if p.relations_pending > 0 {
                    line.push_str(&format!(" · {} liens en file", p.relations_pending));
                }
                if p.files_to_resume > 0 {
                    line.push_str(&format!(" — {} édités après leur passage, repris à la fin", p.files_to_resume));
                }
                log(journal, &line);
            }
            // Les mots sont là ; le graphe arrive d'un coup.
            SyncPhase::Relations => log(journal, &format!("plein texte prêt · relations : {} liens à poser", p.relations_pending)),
            // Ce qui a été édité pendant l'indexation, repris avant de finir.
            SyncPhase::Resume => log(
                journal,
                &format!("reprise : {} fichier(s) sur {} édités pendant l'indexation", p.files_resumed, p.files_resumed + p.files_to_resume),
            ),
            SyncPhase::Done => {}
        })?
    };
    log(
        journal,
        &format!("plein texte prêt : {} fichiers, {} scopes, {} relations posées", report.files_ingested, report.scopes_written, report.relations),
    );

    for (reason, n) in &report.files_set_aside {
        log(journal, &format!("{n} fichiers écartés : {reason}"));
    }

    // Second temps : la dette de vecteurs, par passes ; le catalogue est
    // rendu entre deux, une recherche par mots passe.
    let (rate, start) = {
        let guard = catalog.lock().map_err(|_| "catalogue empoisonné".to_string())?;
        let start = guard.index_progress().map_err(|e| e.to_string())?;
        (guard.known_embedding_rate().map_err(|e| e.to_string())?, start)
    };
    let chars_per_chunk = (kept_bytes as usize / start.chunks().max(1)).max(1);
    catalog
        .lock()
        .map_err(|_| "catalogue empoisonné".to_string())?
        .refresh_index_states(&ENTITIES, rate, chars_per_chunk, false)
        .map_err(|e| e.to_string())?;
    // Une ligne par changement : un journal qui se répète noie ce qu'il dit.
    let mut last_line = String::new();
    let mut say = |line: String| {
        if line != last_line {
            log(journal, &line);
            last_line = line;
        }
    };
    say(start.line(rate, chars_per_chunk));
    let t = std::time::Instant::now();
    let mut embedded = 0usize;
    let mut last_percent = start.dense_percent();
    loop {
        let (n, progress) = {
            let mut guard = catalog.lock().map_err(|_| "catalogue empoisonné".to_string())?;
            let n = guard.embarquer_le_retard(Disponibilites::TOUT, 512, None).map_err(|e| e.to_string())?;
            // Chaque passe rafraîchit les états : « en cours » reste cru tant
            // que quelqu'un travaille.
            let progress = guard.refresh_index_states(&ENTITIES, rate, chars_per_chunk, n > 0).map_err(|e| e.to_string())?;
            (n, progress)
        };
        if n == 0 {
            say(progress.line(rate, chars_per_chunk));
            break;
        }
        embedded += n;
        if progress.dense_percent() != last_percent {
            last_percent = progress.dense_percent();
            say(progress.line(rate, chars_per_chunk));
        }
    }
    // Ce que cette indexation a mesuré en vrai, écritures comprises : la
    // prochaine estimation prévoira mieux que la sonde.
    if let Some(measured) = Rate::from_sample(embedded * chars_per_chunk, t.elapsed()) {
        if embedded >= 512 {
            let guard = catalog.lock().map_err(|_| "catalogue empoisonné".to_string())?;
            guard.note_measured_embedding_rate(measured).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Le refus d'`index` quand l'estimation dépasse le seuil et que l'appelant
/// n'a pas confirmé : il dit ce que ça coûte et quoi faire.
pub fn confirmation_refusal(estimate: &Estimate, confirm: bool) -> Option<String> {
    (estimate.needs_confirmation && !confirm).then(|| {
        format!(
            "confirmation requise avant d'indexer — rien n'a été écrit.\n{}\nPour lancer quand même : index avec confirm=true.",
            estimate.text()
        )
    })
}

/// `index` : estime, refuse sans confirmation au-delà du seuil, sinon lance
/// l'indexation en fond et rend son journal. **Output** `result`
/// (PortType::Map). Services : `file_source` et `catalog` (requis).
pub struct IndexNode {
    node_name: String,
    confirm: bool,
    format: ToolFormat,
}

impl IndexNode {
    pub fn new(name: &str) -> Self {
        Self { node_name: name.to_string(), confirm: false, format: ToolFormat::Markdown }
    }
    pub fn with_confirm(mut self, confirm: bool) -> Self {
        self.confirm = confirm;
        self
    }
    pub fn with_format(mut self, format: ToolFormat) -> Self {
        self.format = format;
        self
    }
}

impl Node for IndexNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "IndexNode"
    }
    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        Some(Box::new(serde_json::json!({ "confirm": self.confirm })))
    }
    fn outputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&IndexNodeFactory).1
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let source = source_service(ctx).ok_or("IndexNode: 'file_source' service not found")?;
        let catalog = ctx.service::<Arc<Mutex<Catalog>>>("catalog").cloned().ok_or("IndexNode: 'catalog' service not found")?;
        let generated = generated_policy(ctx);
        let estimate = estimate_of(source.as_ref(), Some(&catalog), true, &generated).map_err(|e| format!("IndexNode: {e}"))?;
        // Le tampon d'abord : une confirmation n'y change rien, seul un
        // tampon plus grand le lève.
        if let Some(refusal) = estimate.buffer_refusal() {
            return Err(format!("index : {refusal}"));
        }
        if let Some(refusal) = confirmation_refusal(&estimate, self.confirm) {
            return Err(format!("index : {refusal}"));
        }
        let journal = new_index_journal().map_err(|e| format!("IndexNode: journal : {e}"))?;
        log(&journal, &format!("indexation lancée : {} fichiers, {}", estimate.survey.files, estimate.model));
        // Détaché : c'est le journal qui porte la suite, pas ce tour de parole.
        drop(spawn_index(catalog, source, journal.clone(), estimate.survey.bytes, generated));
        ctx.metric("files", estimate.survey.files as f64);
        let path = journal.to_string_lossy().to_string();
        let value = match self.format {
            ToolFormat::Markdown => serde_json::Value::String(format!(
                "indexation lancée en fond.\njournal : {path}\n{}\nSuivre : wait(journal, pattern='{DONE_LINE}|{FAILED_PREFIX}') — avec timeout_s=0 pour lire l'état sans attendre.",
                estimate.text()
            )),
            ToolFormat::Json => serde_json::json!({ "journal": path, "done": DONE_LINE, "failed": FAILED_PREFIX, "estimate": estimate }),
        };
        ctx.set_output("result", PortValue::new(value));
        Ok(())
    }
}

pub struct IndexNodeFactory;

impl NodeFactory for IndexNodeFactory {
    fn create(&self, name: &str, config: &serde_json::Value) -> Result<Box<dyn Node>, String> {
        let mut node = IndexNode::new(name);
        if let Some(c) = config.get("confirm").and_then(|v| v.as_bool()) {
            node = node.with_confirm(c);
        }
        if let Some(f) = config.get("format").and_then(|v| v.as_str()) {
            node = node.with_format(ToolFormat::parse(f).map_err(|e| format!("IndexNode: {e}"))?);
        }
        Ok(Box::new(node))
    }
    fn node_type(&self) -> &'static str {
        "IndexNode"
    }
    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "IndexNode",
            description: "Indexes the file source in the background: full text first, vectors afterwards. Refuses without confirm=true when the estimate exceeds the declared threshold. Returns the path of a journal that `wait` can follow.",
            inputs: vec![],
            outputs: vec![PortDef { name: "result", port_type: PortType::Map, required: false }],
            config_params: vec![
                ConfigParam {
                    name: "confirm",
                    param_type: ConfigParamType::Bool,
                    required: false,
                    default: Some(serde_json::json!(false)),
                    description: "Lancer même si l'estimation dépasse le seuil de confirmation",
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "format",
                    param_type: ConfigParamType::String,
                    required: false,
                    default: Some(serde_json::json!("markdown")),
                    description: "markdown (compact, pour le modèle) | json (structuré)",
                    choices: Some(Choices::fixed(["markdown", "json"])),
                    json_schema: None,
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code_tools::{Snapshot, FILE_SOURCE_SERVICE};
    use crate::dataflow::services::ServiceRegistry;

    fn result(ctx: &mut NodeContext) -> serde_json::Value {
        ctx.drain_outputs().remove("result").and_then(super::super::port::take_or_clone::<serde_json::Value>).expect("un résultat")
    }

    fn context(source: Arc<dyn FileSource>) -> NodeContext {
        let mut services = ServiceRegistry::new();
        services.register(FILE_SOURCE_SERVICE, source);
        NodeContext::with_services(Arc::new(services))
    }

    /// Sans catalogue : les comptes et le modèle, et une durée dite inconnue
    /// plutôt qu'inventée.
    #[test]
    fn l_outil_compte_la_source_et_dit_ce_qu_il_ne_sait_pas() {
        let source: Arc<dyn FileSource> = Arc::new(Snapshot::new(
            "depot",
            [("src/lib.rs".to_string(), "pub fn a() {}\n".repeat(40)), ("data/big.csv".to_string(), "1,2,3\n".repeat(100))],
        ));
        let mut ctx = context(source);
        EstimateNode::new("estimate").with_format(ToolFormat::Json).execute(&mut ctx).expect("estimate");
        let v = result(&mut ctx);
        assert_eq!(v["survey"]["files"], 1, "{v}");
        assert_eq!(v["vectors_seconds"], serde_json::Value::Null, "{v}");
        assert_eq!(v["needs_confirmation"], false, "{v}");
        assert!(v["survey"]["skipped"].as_object().is_some_and(|s| !s.is_empty()), "le csv est écarté avec sa raison : {v}");
        assert!(!v["model"].as_str().unwrap_or_default().is_empty(), "{v}");
    }

    /// Un dossier se liste sans rien lire, et ce que la source écarte
    /// d'elle-même — un secret probable — est compté avec sa raison.
    #[test]
    fn un_dossier_s_estime_avec_ce_que_la_source_ecarte() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/main.rs"), "fn main() {}\n").unwrap();
        std::fs::write(dir.path().join(".env"), "TOKEN=abc\n").unwrap();
        let source: Arc<dyn FileSource> = Arc::new(WorkingTree::new(dir.path()));
        let mut ctx = context(source);
        EstimateNode::new("estimate").execute(&mut ctx).expect("estimate");
        let text = result(&mut ctx).as_str().unwrap_or_default().to_string();
        assert!(text.starts_with("1 fichiers retenus"), "{text}");
        assert!(text.contains("1 écartés"), "le .env est écarté par la source, et ça se dit : {text}");
        assert!(text.contains("durée inconnue"), "{text}");
    }

    /// Un fichier généré est écarté avec sa raison et son poids, dans un
    /// dossier comme dans une source en mémoire ; la politique du manifeste,
    /// rangée en service, lève la règle.
    #[test]
    fn les_fichiers_generes_sont_ecartes_avec_leur_raison_et_la_regle_se_leve() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join("src/generated")).unwrap();
        std::fs::write(dir.path().join("src/main.rs"), "fn main() {}\n").unwrap();
        std::fs::write(dir.path().join("src/parser.rs"), "// Generated from Cypher.g4 by ANTLR 4.13.1\nfn p() {}\n").unwrap();
        std::fs::write(dir.path().join("src/generated/model.rs"), "pub fn model() {}\n").unwrap();
        let tree: Arc<dyn FileSource> = Arc::new(WorkingTree::new(dir.path()));
        let snapshot: Arc<dyn FileSource> = Arc::new(Snapshot::new(
            "depot",
            [
                ("src/main.rs".to_string(), "fn main() {}\n".to_string()),
                ("src/parser.rs".to_string(), "// Generated from Cypher.g4 by ANTLR 4.13.1\nfn p() {}\n".to_string()),
                ("src/generated/model.rs".to_string(), "pub fn model() {}\n".to_string()),
            ],
        ));
        for source in [tree, snapshot] {
            let mut ctx = context(source.clone());
            EstimateNode::new("estimate").execute(&mut ctx).expect("estimate");
            let text = result(&mut ctx).as_str().unwrap_or_default().to_string();
            assert!(text.starts_with("1 fichiers retenus"), "{text}");
            assert!(text.contains(crate::generated::REASON_MARKER), "{text}");
            assert!(text.contains(crate::generated::REASON_DIR), "{text}");

            let mut services = ServiceRegistry::new();
            services.register(FILE_SOURCE_SERVICE, source);
            services.register(GENERATED_POLICY_SERVICE, GeneratedPolicy::off());
            let mut ctx = NodeContext::with_services(Arc::new(services));
            EstimateNode::new("estimate").execute(&mut ctx).expect("estimate");
            let text = result(&mut ctx).as_str().unwrap_or_default().to_string();
            assert!(text.starts_with("3 fichiers retenus"), "règle levée : {text}");
            assert!(!text.contains("généré"), "{text}");
        }
    }

    #[test]
    fn le_format_inconnu_est_refuse_a_la_fabrique() {
        assert!(EstimateNodeFactory.create("e", &serde_json::json!({"format": "yaml"})).is_err());
        assert!(EstimateNodeFactory.create("e", &serde_json::json!({"probe": false})).is_ok());
    }

    /// **Le refus dit ce que ça coûte et quoi faire**, et ne refuse que ce
    /// qui dépasse le seuil sans confirmation.
    #[test]
    fn index_refuse_sans_confirmation_au_dela_du_seuil() {
        use crate::embedding_choice::Choice;
        use crate::estimate::{survey, CONFIRM_ABOVE};
        let s = survey([("a.rs", 35_000_000u64)], code_policy);
        let choice = Choice { model: "granite-278m".into(), reason: "le défaut".into() };
        let slow = Estimate::new(s.clone(), choice.clone(), Some(Rate { chars_per_second: 20_000.0 }), CONFIRM_ABOVE);
        let refusal = confirmation_refusal(&slow, false).expect("au-delà du seuil, sans confirmation");
        assert!(refusal.contains("rien n'a été écrit") && refusal.contains("environ 29 min") && refusal.contains("confirm=true"), "{refusal}");
        assert_eq!(confirmation_refusal(&slow, true), None, "confirmé : on lance");
        let fast = Estimate::new(s.clone(), choice.clone(), Some(Rate { chars_per_second: 500_000.0 }), CONFIRM_ABOVE);
        assert_eq!(confirmation_refusal(&fast, false), None, "sous le seuil : rien à confirmer");
        let unknown = Estimate::new(s, choice, None, CONFIRM_ABOVE);
        assert_eq!(confirmation_refusal(&unknown, false), None, "on ne bloque pas sur une durée inconnue");
    }

    #[test]
    fn index_exige_un_catalogue_et_son_journal_est_attendable() {
        let source: Arc<dyn FileSource> = Arc::new(Snapshot::new("depot", [("a.rs".to_string(), "fn a() {}\n".to_string())]));
        let mut ctx = context(source);
        let e = IndexNode::new("index").execute(&mut ctx).expect_err("pas de catalogue");
        assert!(e.contains("catalog"), "{e}");
        // Le journal naît là où `wait` accepte d'attendre.
        let journal = new_index_journal().expect("journal");
        assert!(journal.starts_with(crate::dataflow::run_nodes::dossier_journaux()), "{}", journal.display());
        let _ = std::fs::remove_file(&journal);
        assert!(IndexNodeFactory.create("i", &serde_json::json!({"confirm": true})).is_ok());
    }
}
