//! **`ReindexFileNode`** : un fichier changé sous une montre est réindexé,
//! un fichier supprimé est retiré de l'index (W2,
//! `docs/11-octobre-2026-everything-declarative/04-surveiller-les-fichiers.md`).
//!
//! Il lit les `FileChanged` que `EventSourceNode` a tirés du sujet `files`, et
//! passe par les deux chemins qui existent déjà : `code_sync::reingest_file`
//! (le fichier est là) et `code_sync::remove_file` (il n'y est plus).
//! **C'est le disque qui décide, pas l'événement** : un « removed » arrivé
//! après la recréation du fichier (un `git checkout`, un balayage en retard),
//! ou forgé par autre chose que la montre, ne retire pas un fichier présent. Pendant une
//! indexation entière de la source, il ne la double pas : le fichier est inscrit
//! au registre de reprise (`code_sync::note_change`), comme une édition.
//!
//! **Réservé aux réactions** (`backend_code::REACTION_NODES`) : c'est une
//! capacité nommée — écrire l'index du code — et non la porte de fichiers ou de
//! commandes. Il ne lit que ce que la source donne, sous sa racine.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use super::node::{Node, NodeContext};
use super::node_registry::{ConfigParam, ConfigParamType, NodeFactory, NodeSchema};
use super::port::{PortDef, PortType, PortValue};
use crate::catalog::Catalog;
use crate::code_tools::{FileSource, FILE_SOURCE_SERVICE};

/// **Un lot réindexé dans une transaction** (L3, page
/// 12-octobre-2026-arbre-principal/02) : le DDL avant le BEGIN, le lot en
/// plein texte seul (aucun appel au service d'embarquement dans la
/// transaction), le plein texte en tampon jusqu'au COMMIT ; puis la dette
/// d'embarquement du lot, hors transaction. Un lecteur sur la connexion de
/// lecture voit le lot d'avant, entier, jusqu'au COMMIT. Un échec défait le
/// lot sans empoisonner le catalogue. Sans transaction possible (un dialecte
/// sans Cypher), le chemin d'avant, en ligne.
///
/// Deux crochets de témoin, par le nom d'un fichier du lot : `__pause_du_lot__`
/// tient la transaction ouverte 2 s avant le COMMIT ; `__echec_du_lot__` fait
/// échouer le lot avant le COMMIT.
fn reindexer_en_lot(
    cat: &mut Catalog,
    source: &dyn FileSource,
    fichiers: &[(String, Option<String>)],
) -> Result<crate::code_tools::ReingestReport, String> {
    use crate::disponibilite::Disponibilites;
    // Le lot s'écrit sans ses vecteurs : leur dette se paie après, hors du
    // verrou du catalogue (`Catalog::embarquer_le_retard_hors_verrou`), pour
    // que les lectures n'attendent pas le modèle.
    let en_lot = cat.begin_lot().map_err(|e| e.to_string())?;
    if !en_lot {
        return crate::code_sync::reingest_files(cat, source, fichiers, Disponibilites::RECHERCHE_TEXTE);
    }
    let r = crate::code_sync::reingest_files(cat, source, fichiers, Disponibilites::RECHERCHE_TEXTE).and_then(|r| {
        if fichiers.iter().any(|(p, _)| p.contains("__pause_du_lot__")) {
            std::thread::sleep(std::time::Duration::from_secs(2));
        }
        if fichiers.iter().any(|(p, _)| p.contains("__echec_du_lot__")) {
            return Err("échec du lot provoqué (crochet de témoin)".to_string());
        }
        Ok(r)
    });
    match r {
        Ok(rapport) => {
            cat.commit_lot().map_err(|e| e.to_string())?;
            Ok(rapport)
        }
        Err(e) => {
            cat.rollback_lot();
            Err(e)
        }
    }
}

pub struct ReindexFileNode {
    name: String,
    /// Seulement les événements de cette montre ; vide : toutes.
    watch: String,
    /// Au plus tant de fichiers réindexés ensemble, sous une prise du verrou.
    files_per_batch: usize,
    /// **Le budget de tenue d'un lot** : le lot suivant se dimensionne pour
    /// tenir le verrou à peu près ce temps, d'après le coût par fichier du lot
    /// précédent. Sous l'attente d'une lecture (`catalog_read`, 1 s) : les
    /// lecteurs passent entre deux lots sans être refusés.
    hold_ms: u64,
}

/// Le plafond par défaut de `files_per_batch`.
pub const FILES_PER_BATCH: usize = 32;
/// Le budget de tenue par défaut d'un lot, en millisecondes.
pub const HOLD_MS: u64 = 400;
/// La taille du premier lot, avant toute mesure.
const FIRST_BATCH: usize = 4;

/// Ce qui est arrivé à un chemin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Outcome {
    Reindexed,
    Removed,
    /// Une indexation entière tient la source : repris à sa fin.
    Deferred,
}

/// Les étapes qui en contiennent d'autres : elles compteraient deux fois.
const AGREGATS: &[&str] = &[
    "entités · TOTAL des appels",
    "entités · exécuter le graphe (nœuds + points de reprise)",
    "code · ingérer File, Scope, Library",
    "code · symboles (ingestion Symbol + rendez-vous)",
    "sync · paquet entier, de bout en bout",
    "analyse · parse_project (codeparsers, par fichier en parallèle)",
    "symboles · ingestion",
    "symboles · drain des rendez-vous",
    "symboles · drain des arêtes résolues",
    // Elle contient la lecture des DEFINES et des MENTIONS, les types
    // différés, les parents et fichiers, le choix et le lien.
    "symboles · relecture et mise en file des arêtes",
    "drain · exécution",
    "instantané · drain",
];

/// **La ligne de journal d'un fichier** : le verrou du catalogue attendu, puis
/// tenu ; puis chaque phase de la réindexation (ou du retrait), de la plus
/// longue à la plus courte, avec entre crochets ce qui s'y passe, instruction
/// par instruction, et ce qui n'est pas détaillé (la phase moins ses étapes).
///
/// Une phase s'enregistre **à sa fin** : les étapes enregistrées avant elle,
/// depuis la phase précédente, sont les siennes. Seules les feuilles comptent
/// (les [`AGREGATS`] en contiennent d'autres). Ce qui suit la dernière phase
/// se dit « hors phase ». Seules les étapes vues sur le fil de la réaction y
/// sont.
pub(crate) fn journal_line(
    path: &str,
    outcome: &Result<Outcome, String>,
    waited: std::time::Duration,
    held: std::time::Duration,
    steps: &[(&'static str, std::time::Duration)],
) -> String {
    use std::collections::BTreeMap;
    use std::time::Duration;
    let nom_court = |stage: &'static str| -> &'static str {
        match stage {
            "nœud embed" => "embarquer",
            s => s.strip_prefix("entités · ").unwrap_or(s),
        }
    };
    fn rang<'a>(m: &BTreeMap<&'a str, Duration>) -> Vec<(&'a str, Duration)> {
        let mut v: Vec<_> = m.iter().map(|(n, d)| (*n, *d)).collect();
        v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
        v
    }
    // Les phases, chacune avec ses étapes intérieures.
    let mut phases: BTreeMap<&str, (Duration, BTreeMap<&str, Duration>)> = Default::default();
    let mut en_cours: BTreeMap<&str, Duration> = Default::default();
    for (stage, took) in steps {
        if let Some(phase) = stage.strip_prefix("réindexer · ").or_else(|| stage.strip_prefix("retirer · ")) {
            let entree = phases.entry(phase).or_default();
            entree.0 += *took;
            for (n, d) in std::mem::take(&mut en_cours) {
                *entree.1.entry(n).or_default() += d;
            }
        } else if !AGREGATS.contains(stage) {
            *en_cours.entry(nom_court(stage)).or_default() += *took;
        }
    }
    let mut ordre: Vec<_> = phases.iter().map(|(n, (d, _))| (*n, *d)).collect();
    ordre.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    let corps = ordre
        .iter()
        .map(|(nom, duree)| {
            let interieur = &phases[nom].1;
            if interieur.is_empty() {
                return format!("{nom} {} ms", duree.as_millis());
            }
            let detaille: Duration = interieur.values().sum();
            // Les étapes sous la milliseconde comptent dans le détaillé, sans s'écrire.
            let mut morceaux: Vec<String> = rang(interieur)
                .iter()
                .filter(|(_, d)| d.as_millis() > 0)
                .map(|(n, d)| format!("{n} {} ms", d.as_millis()))
                .collect();
            morceaux.push(format!("non détaillé {} ms", duree.saturating_sub(detaille).as_millis()));
            format!("{nom} {} ms [{}]", duree.as_millis(), morceaux.join(", "))
        })
        .collect::<Vec<_>>()
        .join(", ");
    let hors = match rang(&en_cours) {
        v if v.is_empty() => String::new(),
        v => format!(" ; hors phase : {}", v.iter().map(|(n, d)| format!("{n} {} ms", d.as_millis())).collect::<Vec<_>>().join(", ")),
    };
    let verbe = match outcome {
        Ok(Outcome::Reindexed) => "réindexé",
        Ok(Outcome::Removed) => "retiré",
        Ok(Outcome::Deferred) => "remis à l'indexation entière",
        Err(_) => "en erreur",
    };
    format!(
        "[montre] {path} {verbe} : verrou attendu {} ms, tenu {} ms — {corps}{hors}",
        waited.as_millis(),
        held.as_millis()
    )
}

impl ReindexFileNode {
    /// Les chemins désignés, dédoublonnés, dans l'ordre. L'événement ne dit
    /// que **quel** fichier regarder ; ce qu'on en fait se lit sur le disque.
    fn todo(&self, events: &Value) -> BTreeSet<String> {
        events
            .as_array()
            .into_iter()
            .flatten()
            .filter(|e| e["kind"] == "FileChanged" && (self.watch.is_empty() || e["watch"] == self.watch.as_str()))
            .filter_map(|e| e["path"].as_str().map(str::to_string))
            .collect()
    }
}

impl Node for ReindexFileNode {
    fn node_type(&self) -> &str {
        "ReindexFileNode"
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn inputs(&self) -> Vec<PortDef> {
        ReindexFileNodeFactory.schema().inputs
    }
    fn outputs(&self) -> Vec<PortDef> {
        ReindexFileNodeFactory.schema().outputs
    }
    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        Some(Box::new(json!({"watch": self.watch})))
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let events = ctx
            .take_input("events")
            .and_then(|v| v.downcast::<Value>().map(|v| (*v).clone()))
            .unwrap_or(Value::Array(Vec::new()));
        let todo = self.todo(&events);
        let catalog = ctx
            .service::<Arc<Mutex<Catalog>>>("catalog")
            .cloned()
            .ok_or("ReindexFileNode : pas de service 'catalog'")?;
        let source = ctx
            .service::<Arc<dyn FileSource>>(FILE_SOURCE_SERVICE)
            .cloned()
            .ok_or("ReindexFileNode : pas de source de fichiers (le backend n'a pas de workspace)")?;

        // Le registre du retard, et la cellule : celle de l'appel (le service
        // `scope`), sinon celle du catalogue — comme la lecture
        // (`read_catalog_in`), pour effacer là où la montre a noté.
        let call_cell = ctx.service::<crate::scope::Scope>("scope").cloned();
        let (lag, cell) = {
            let cat = catalog.lock().map_err(|e| format!("catalogue empoisonné : {e}"))?;
            (cat.file_lag(), call_cell.unwrap_or_else(|| cat.scope().clone()))
        };
        let (mut reindexed, mut removed, mut deferred) = (Vec::new(), Vec::new(), Vec::new());
        let mut errors = Vec::new();
        let stop = ctx.service::<Arc<std::sync::atomic::AtomicBool>>(super::REACTION_STOP_SERVICE).cloned();
        let mut interrupted = 0usize;
        let source_id = crate::code_sync::registry_key(source.as_ref());
        // Une indexation entière tient la source : les fichiers sont lus à
        // leur paquet, ou repris à sa fin — on ne la double pas. Si elle démarre
        // juste après ce regard, un fichier peut passer deux fois (ici puis à
        // son paquet) : sans dommage, la réingestion d'un même contenu est
        // idempotente (l'upsert inchangé est court-circuité).
        let todo: Vec<String> = if crate::code_sync::indexing_in_progress(&source_id) {
            // En reprise, **sans effacer le retard** : rien n'est relu ici.
            for path in &todo {
                crate::code_sync::note_change(&source_id, path);
            }
            deferred.extend(todo);
            Vec::new()
        } else {
            todo.into_iter().collect()
        };
        let total = todo.len();
        let mut faits = 0usize;
        // La porte de l'index, avec la politique des générés du manifeste.
        let politique = ctx
            .service::<crate::generated::GeneratedPolicy>(crate::generated::GENERATED_POLICY_SERVICE)
            .cloned()
            .unwrap_or_default();
        let mut hors_perimetre = 0usize;
        let plafond = self.files_per_batch.max(1);
        let mut taille = FIRST_BATCH.min(plafond);
        while faits < total {
            let lot = &todo[faits..(faits + taille).min(total)];
            // **L'arrêt passe entre deux lots** : ce qui est fait reste, le
            // reste garde son retard, et la réconciliation le reprendra.
            if stop.as_ref().is_some_and(|s| s.load(std::sync::atomic::Ordering::SeqCst)) {
                interrupted = total - faits;
                break;
            }
            // **Céder aux lecteurs** entre deux lots : un lecteur qui attend le
            // catalogue passe avant le lot suivant (au plus 100 ms d'attente,
            // pour ne pas affamer la réaction sous des lectures continues).
            let cede = std::time::Instant::now();
            while super::catalog_read::readers_waiting(&catalog)
                && cede.elapsed() < std::time::Duration::from_millis(100)
            {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            let attente = std::time::Instant::now();
            let mut cat = catalog.lock().map_err(|e| format!("catalogue empoisonné : {e}"))?;
            let waited = attente.elapsed();
            let tenue = std::time::Instant::now();
            // L'instant de la lecture : un changement venu après garde son retard.
            let read_at = crate::catalog::retard::now_ms();
            // Le disque décide, fichier par fichier : réindexer ou retirer.
            // Un contenu que la porte de l'index refuse (généré, binaire) est
            // traité comme absent, comme le ferait la fin d'une indexation
            // entière : s'il était indexé, il en sort.
            let mut fichiers: Vec<(String, Option<String>)> = Vec::new();
            for path in lot {
                match source.read(path) {
                    Ok(Some(contenu)) if crate::code::admission(path, Some(&contenu), &politique).is_err() => {
                        hors_perimetre += 1;
                        fichiers.push((path.clone(), None));
                    }
                    Ok(contenu) => fichiers.push((path.clone(), contenu)),
                    Err(e) => {
                        ctx.warn(&format!("ReindexFileNode : {path} : {e}"));
                        errors.push(json!({"path": path, "error": e}));
                    }
                }
            }
            let (outcome, steps) = crate::ingest_profile::record(|| reindexer_en_lot(&mut cat, source.as_ref(), &fichiers));
            // Un lot en échec ne dit pas quel fichier : on les refait un à un
            // (idempotent), et chacun rend son verdict.
            let verdicts: Vec<(String, bool, Result<Outcome, String>)> = match outcome {
                Ok(r) => {
                    let o = if r.deferred_to.is_some() { Outcome::Deferred } else { Outcome::Reindexed };
                    fichiers.iter().map(|(p, c)| (p.clone(), c.is_some(), Ok(o))).collect()
                }
                Err(e) => {
                    ctx.warn(&format!("ReindexFileNode : le lot de {} fichier(s) a échoué ({e}) ; repris un à un", fichiers.len()));
                    fichiers
                        .iter()
                        .map(|(p, c)| {
                            // Chacun dans son lot : la même transaction, le
                            // même tampon, à l'échelle d'un fichier.
                            let r = reindexer_en_lot(&mut cat, source.as_ref(), &[(p.clone(), c.clone())]).map(|r| match c {
                                None => Outcome::Removed,
                                Some(_) if r.deferred_to.is_some() => Outcome::Deferred,
                                Some(_) => Outcome::Reindexed,
                            });
                            (p.clone(), c.is_some(), r)
                        })
                        .collect()
                }
            };
            drop(cat);
            let tenu = tenue.elapsed();
            faits += lot.len();
            // Le lot suivant, à la mesure de celui-ci : tant de fichiers qu'il
            // en tient dans le budget, entre un et le plafond.
            let par_fichier = tenu.as_millis() as u64 / lot.len().max(1) as u64;
            taille = (self.hold_ms / par_fichier.max(1)).clamp(1, plafond as u64) as usize;
            let etiquette = match fichiers.as_slice() {
                [(p, _)] => p.clone(),
                l => format!("{} fichiers ({})", l.len(), l.iter().map(|(p, _)| p.as_str()).collect::<Vec<_>>().join(", ")),
            };
            let resume = if verdicts.iter().all(|(_, _, r)| r.is_ok()) {
                Ok(if verdicts.iter().any(|(_, _, r)| matches!(r, Ok(Outcome::Deferred))) {
                    Outcome::Deferred
                } else if verdicts.iter().all(|(_, present, _)| !present) {
                    Outcome::Removed
                } else {
                    Outcome::Reindexed
                })
            } else {
                Err("au moins un fichier en échec".to_string())
            };
            eprintln!("{}", journal_line(&etiquette, &resume, waited, tenu, &steps));
            for (path, present, verdict) in verdicts {
                let verdict = match verdict {
                    // Un supprimé retiré se dit retiré, même passé par le lot.
                    Ok(Outcome::Reindexed) if !present => Ok(Outcome::Removed),
                    v => v,
                };
                if matches!(verdict, Ok(Outcome::Reindexed | Outcome::Removed)) {
                    lag.done(&cell, &path, read_at);
                }
                match verdict {
                    Ok(Outcome::Reindexed) => reindexed.push(path),
                    Ok(Outcome::Removed) => removed.push(path),
                    Ok(Outcome::Deferred) => {
                        crate::code_sync::note_change(&source_id, &path);
                        deferred.push(path);
                    }
                    Err(e) => {
                        ctx.warn(&format!("ReindexFileNode : {path} : {e}"));
                        errors.push(json!({"path": path, "error": e}));
                    }
                }
            }
            // **La dette du lot, hors du verrou.** Le lot est écrit, le
            // catalogue rendu, et le retard des fichiers effacé : ils sont à
            // jour pour le texte. Leur dette de vecteurs se réclame puis
            // s'écrit sous le verrou, en deux prises courtes, et le modèle est
            // appelé sans lui. Les morceaux neufs manquent à la recherche
            // vectorielle le temps de cet appel.
            let embarque = std::time::Instant::now();
            match Catalog::embarquer_le_retard_hors_verrou(
                &catalog,
                crate::disponibilite::Disponibilites::TOUT,
                crate::catalog::RATTRAPAGE_PAR_PASSE,
                None,
            ) {
                Ok(n) => eprintln!("[montre] puis {n} morceau(x) embarqué(s) hors du verrou : {} ms", embarque.elapsed().as_millis()),
                Err(e) => ctx.warn(&format!("ReindexFileNode : l'embarquement du lot reste dû : {e}")),
            }
        }
        // L'arbre indexé a avancé : son commit (et ses modifications) sont
        // ceux de maintenant.
        if !reindexed.is_empty() || !removed.is_empty() {
            if let Some(root) = crate::workspace_path::worktree_root(&source.cursor()) {
                // `git` d'abord, hors du verrou ; le verrou pour l'écriture seule.
                let v = crate::catalog::indexed_tree_of(root);
                if let Ok(cat) = catalog.lock() {
                    if let Err(e) = cat.note_indexed_tree(&cell, &v) {
                        ctx.warn(&format!("ReindexFileNode : l'arbre indexé n'a pas pu être noté : {e}"));
                    }
                }
            }
        }
        if hors_perimetre > 0 {
            let ligne = format!("{total} fichier(s) vu(s), {hors_perimetre} hors du périmètre de l'index (retirés s'ils y étaient)");
            eprintln!("[montre] {ligne}");
            // Aussi dans le journal de la réaction, que l'hôte lit.
            ctx.info(&format!("ReindexFileNode : {ligne}"));
        }
        ctx.info(&format!(
            "ReindexFileNode : {} réindexé(s), {} retiré(s), {} repris plus tard, {} en échec",
            reindexed.len(),
            removed.len(),
            deferred.len(),
            errors.len()
        ));
        ctx.set_output(
            "report",
            PortValue::new(json!({"reindexed": reindexed, "removed": removed, "deferred": deferred, "errors": errors, "interrupted": interrupted, "excluded": hors_perimetre})),
        );
        Ok(())
    }
}

pub struct ReindexFileNodeFactory;

impl NodeFactory for ReindexFileNodeFactory {
    fn create(&self, name: &str, config: &Value) -> Result<Box<dyn Node>, String> {
        let watch = match config.get("watch") {
            None | Some(Value::Null) => String::new(),
            Some(Value::String(s)) => s.clone(),
            Some(other) => return Err(format!("'watch' : une chaîne, pas {other}")),
        };
        let files_per_batch = match config.get("files_per_batch") {
            None | Some(Value::Null) => FILES_PER_BATCH,
            Some(v) => v
                .as_u64()
                .filter(|n| *n > 0)
                .ok_or_else(|| format!("'files_per_batch' : un entier positif, pas {v}"))? as usize,
        };
        let hold_ms = match config.get("hold_ms") {
            None | Some(Value::Null) => HOLD_MS,
            Some(v) => v.as_u64().filter(|n| *n > 0).ok_or_else(|| format!("'hold_ms' : un entier positif, pas {v}"))?,
        };
        Ok(Box::new(ReindexFileNode { name: name.to_string(), watch, files_per_batch, hold_ms }))
    }
    fn node_type(&self) -> &str {
        "ReindexFileNode"
    }
    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "ReindexFileNode".into(),
            description: "Réindexe les fichiers changés sous une montre (FileChanged du sujet 'files') et \
                          retire les supprimés ; pendant une indexation entière, les met en reprise. \
                          Réservé aux réactions."
                .into(),
            inputs: vec![PortDef::value("events", PortType::Map, false)],
            outputs: vec![PortDef::value("report", PortType::Map, true)],
            config_params: vec![
                ConfigParam {
                    name: "watch".into(),
                    param_type: ConfigParamType::String,
                    required: false,
                    default: Some(json!("")),
                    description: "seulement les événements de cette montre ; vide : toutes".into(),
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "files_per_batch".into(),
                    param_type: ConfigParamType::Int,
                    required: false,
                    default: Some(json!(FILES_PER_BATCH)),
                    description: "au plus tant de fichiers réindexés ensemble, en un passage et sous une prise \
                                  du verrou ; les lecteurs passent entre deux lots"
                        .into(),
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "hold_ms".into(),
                    param_type: ConfigParamType::Int,
                    required: false,
                    default: Some(json!(HOLD_MS)),
                    description: "le temps de tenue visé pour un lot : le suivant se dimensionne d'après le coût \
                                  par fichier du précédent (au moins un fichier)"
                        .into(),
                    choices: None,
                    json_schema: None,
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_journal_line_tells_the_lock_and_the_phases() {
        let ms = std::time::Duration::from_millis;
        // Dans l'ordre où elles s'enregistrent : une phase se clôt à sa fin, et
        // les étapes enregistrées avant elle sont les siennes.
        let steps = [
            ("réindexer · analyser le fichier", ms(12)),
            ("drain · suppressions", ms(40)),
            ("instantané · mettre de côté", ms(20)),
            ("instantané · drain", ms(70)),
            ("réindexer · fin d'instantané sur le grain du fichier", ms(100)),
            ("nœud embed", ms(900)),
            ("entités · gardes", ms(3)),
            // Les agrégats ne s'ajoutent pas : ils contiennent d'autres étapes.
            ("entités · TOTAL des appels", ms(1400)),
            ("code · ingérer File, Scope, Library", ms(1450)),
            ("nœud embed", ms(100)),
            ("symboles · drain des rendez-vous", ms(50)),
            ("drain · liens", ms(30)),
            ("réindexer · ingérer (scopes, morceaux, vecteurs, liens)", ms(1500)),
            ("drain · liens", ms(5)),
        ];
        let line = super::journal_line("arbre/scope.rs", &Ok(super::Outcome::Reindexed), ms(4), ms(1700), &steps);
        assert_eq!(
            line,
            "[montre] arbre/scope.rs réindexé : verrou attendu 4 ms, tenu 1700 ms — \
             ingérer (scopes, morceaux, vecteurs, liens) 1500 ms [embarquer 1000 ms, drain · liens 30 ms, gardes 3 ms, non détaillé 467 ms], \
             fin d'instantané sur le grain du fichier 100 ms [drain · suppressions 40 ms, instantané · mettre de côté 20 ms, non détaillé 40 ms], \
             analyser le fichier 12 ms ; hors phase : drain · liens 5 ms"
        );
    }
}
