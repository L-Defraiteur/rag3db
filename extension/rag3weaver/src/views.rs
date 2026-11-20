//! **Les vues** (2b′) : une vue se nomme `(couche, rang)`, et le chargeur lit sa chaîne de générations dans
//! `_generation` (la génération, sa parente épinglée, et ainsi de suite jusqu'à la racine) pour en faire la
//! **pile de lecture** : une description, sans aucune occurrence chargée. La composition se fait en
//! prédicat aux points de lecture d'aujourd'hui. Page :
//! `docs/arbre-principal/15-11-2026/01-2b-prime-le-chargeur-qui-compose-la-pile.md`.

use std::collections::BTreeSet;

use serde_json::Value;

/// The service under which a graph finds the [`ViewRequest`] of its call, when the call asked for one.
pub const REQUESTED_VIEW_SERVICE: &str = "requested_view";

/// Une vue demandée : la couche (son id stable) et le rang ; sans rang, la dernière génération publiée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewRequest {
    pub layer: String,
    pub rank: Option<i64>,
}

/// Un étage de la pile : une génération, et ce qu'elle retire de ses parentes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackLayer {
    pub layer: String,
    pub rank: i64,
    /// Les clés d'occurrence retirées par cet étage.
    pub removed_keys: BTreeSet<String>,
    /// Les fichiers retirés par cet étage.
    pub removed_files: BTreeSet<String>,
}

/// La pile de lecture d'une vue : la vue d'abord, la racine en dernier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadStack {
    pub layers: Vec<StackLayer>,
    /// La racine n'avait aucune parente écrite (une génération d'avant 2b′, ou une vraie racine) : dit.
    pub root_without_parent: bool,
}

/// Les réglages des vues, sous `tracked.views` du manifeste.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewSettings {
    /// La profondeur au-delà de laquelle une pile se refuse.
    pub max_depth: usize,
    /// Le temps qu'un chargement attend la table `_generation` sous une lecture.
    pub load_timeout_ms: u64,
}

impl Default for ViewSettings {
    fn default() -> Self {
        ViewSettings { max_depth: 16, load_timeout_ms: 2000 }
    }
}

/// **Les réglages des vues** depuis `tracked` (la clé `views`) ; absents, les défauts. Toute clé inconnue
/// est un refus qui la nomme.
pub fn view_settings(tracked: Option<&Value>) -> Result<ViewSettings, String> {
    let mut r = ViewSettings::default();
    let Some(vues) = tracked.and_then(|t| t.get("views")) else { return Ok(r) };
    let objet = vues.as_object().ok_or_else(|| format!("tracked.views : un objet {{max_depth?, load_timeout_ms?}} attendu, pas {vues}"))?;
    for (cle, v) in objet {
        let entier = v.as_u64().filter(|n| *n > 0);
        match (cle.as_str(), entier) {
            ("max_depth", Some(n)) => r.max_depth = n as usize,
            ("load_timeout_ms", Some(n)) => r.load_timeout_ms = n,
            ("max_depth" | "load_timeout_ms", None) => return Err(format!("tracked.views.{cle} : un entier positif attendu, pas {v}")),
            _ => return Err(format!("tracked.views : clé inconnue « {cle} » (max_depth, load_timeout_ms)")),
        }
    }
    Ok(r)
}

/// **The settings of the writes of layers** (2a′-3, lot 5), under `tracked.writes` of the manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteSettings {
    /// How many work layers are written at most; a new one beyond is refused, naming this setting.
    pub max_work_layers: usize,
    /// The period of the backend's sync of its tracked layers, in milliseconds; `None`: no sync runs (the default,
    /// a backend writes only its own layer).
    pub every_ms: Option<u64>,
}

impl Default for WriteSettings {
    fn default() -> Self {
        WriteSettings { max_work_layers: 32, every_ms: None }
    }
}

/// **The settings of the writes** from `tracked` (the key `writes`); absent, the defaults.
pub fn write_settings(tracked: Option<&Value>) -> Result<WriteSettings, String> {
    let mut r = WriteSettings::default();
    let Some(writes) = tracked.and_then(|t| t.get("writes")) else { return Ok(r) };
    let objet = writes.as_object().ok_or_else(|| format!("tracked.writes : un objet {{max_work_layers?}} attendu, pas {writes}"))?;
    for (cle, v) in objet {
        match (cle.as_str(), v.as_u64().filter(|n| *n > 0)) {
            ("max_work_layers", Some(n)) => r.max_work_layers = n as usize,
            ("every_ms", Some(n)) => r.every_ms = Some(n),
            ("max_work_layers" | "every_ms", None) => return Err(format!("tracked.writes.{cle} : un entier positif attendu, pas {v}")),
            _ => return Err(format!("tracked.writes : clé inconnue « {cle} » (max_work_layers, every_ms)")),
        }
    }
    Ok(r)
}

/// Les tombstones d'une génération : `[]` ou une liste de clés (l'écriture de la génération 0), ou
/// `{"keys": [...], "files": [...]}`.
pub(crate) fn read_tombstones(couche: &str, rang: i64, texte: &str) -> Result<(BTreeSet<String>, BTreeSet<String>), String> {
    let refus = || format!("vue {couche}:{rang} : tombstones illisibles ({texte})");
    let liste = |v: Option<&Value>| -> Result<BTreeSet<String>, String> {
        match v {
            None => Ok(BTreeSet::new()),
            Some(Value::Array(a)) => a.iter().map(|x| x.as_str().map(str::to_string).ok_or_else(refus)).collect(),
            Some(_) => Err(refus()),
        }
    };
    let v: Value = serde_json::from_str(texte).map_err(|_| refus())?;
    match &v {
        Value::Array(_) => Ok((liste(Some(&v))?, BTreeSet::new())),
        Value::Object(o) if o.keys().all(|k| k == "keys" || k == "files") => Ok((liste(o.get("keys"))?, liste(o.get("files"))?)),
        _ => Err(refus()),
    }
}

/// **Why a stack does not load**: one variant per refusal, so that a caller (the name fallback of 2b-C) tests the
/// kind instead of the text. `Display` gives the text of before, word for word.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StackError {
    /// The layer has no generation at all (rank omitted): « couche inconnue (couche sans génération, 2a′-3) ».
    UnknownLayer { layer: String, without_generation: bool },
    /// The layer is known but not at the requested rank.
    RankAbsent { layer: String, rank: i64 },
    /// A pinned parent is not in `_generation`.
    MissingParent { view: String, parent: String, rank: i64, child: String, child_rank: i64 },
    /// A parent chain comes back on itself.
    Cycle { view: String, layer: String, rank: i64 },
    /// The stack is deeper than `tracked.views.max_depth`.
    TooDeep { view: String, max_depth: usize },
    /// Loading took longer than `tracked.views.load_timeout_ms`.
    LoadTimeout { view: String, ms: u64 },
    /// A generation's tombstones do not parse.
    UnreadableTombstones { layer: String, rank: i64, text: String },
    /// The dialect has no generations.
    NoGenerations,
    /// The read of `_generation` (or the catalog) failed; the message as before.
    Db(String),
}

impl std::fmt::Display for StackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StackError::UnknownLayer { layer, without_generation: true } => write!(f, "vue « {layer} » : couche inconnue (couche sans génération, 2a′-3)"),
            StackError::UnknownLayer { layer, without_generation: false } => write!(f, "vue « {layer} » : couche inconnue"),
            StackError::RankAbsent { layer, rank } => write!(f, "vue {layer} : rang {rank} absent de _generation"),
            StackError::MissingParent { view, parent, rank, child, child_rank } => {
                write!(f, "vue {view} : parente manquante {parent}:{rank} (de {child}:{child_rank})")
            }
            StackError::Cycle { view, layer, rank } => write!(f, "vue {view} : cycle de parentes en {layer}:{rank}"),
            StackError::TooDeep { view, max_depth } => write!(f, "vue {view} : profondeur au-delà de {max_depth} étages (tracked.views.max_depth)"),
            StackError::LoadTimeout { view, ms } => write!(f, "vue {view} : chargement au-delà de {ms} ms (tracked.views.load_timeout_ms)"),
            StackError::UnreadableTombstones { layer, rank, text } => write!(f, "vue {layer}:{rank} : tombstones illisibles ({text})"),
            StackError::NoGenerations => write!(f, "vues : ce dialecte n'a pas de générations"),
            StackError::Db(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for StackError {}

/// **Charger la pile d'une vue** depuis `_generation`. Refus nommés : couche inconnue, rang absent, parente
/// manquante, cycle, profondeur au-delà du réglage.
pub fn load_stack(catalog: &crate::catalog::Catalog, vue: &ViewRequest, reglages: &ViewSettings) -> Result<ReadStack, StackError> {
    catalog.check_reopen().map_err(|e| StackError::Db(format!("vues : {e}")))?;
    load_stack_on(catalog.conn(), catalog.dialect_arc().as_ref(), vue, reglages)
}

/// **Le chargeur sur une connexion** : celle de l'instantané d'un appel d'outil, pour que la vue et
/// ses lignes viennent de la même lecture, sans le mutex du catalogue.
pub fn load_stack_on(
    conn: &dyn crate::connection::DbConnection,
    dialecte: &dyn crate::dialect::SchemaDialect,
    vue: &ViewRequest,
    reglages: &ViewSettings,
) -> Result<ReadStack, StackError> {
    use crate::connection::{CypherValue, QueryParam};
    let (Some(lire), Some(dernier)) = (dialecte.read_generation(), dialecte.latest_generation_rank()) else {
        return Err(StackError::NoGenerations);
    };
    let debut = std::time::Instant::now();
    let texte = |nom: &str, v: &str| QueryParam::new(nom, CypherValue::String(v.to_string()));
    let executer = |cypher: &str, params: &[QueryParam]| conn.execute_with_params(cypher, params).map_err(|e| StackError::Db(format!("vues : lecture de _generation : {e}")));
    let rang = match vue.rank {
        Some(r) => r,
        None => {
            let lu = executer(&dernier, &[texte("couche", &vue.layer)])?;
            match lu.rows.first().and_then(|r| r.first()) {
                Some(CypherValue::Int(r)) => *r,
                _ => return Err(StackError::UnknownLayer { layer: vue.layer.clone(), without_generation: true }),
            }
        }
    };
    let mut etages: Vec<StackLayer> = Vec::new();
    let mut vus = BTreeSet::new();
    let (mut couche, mut rang) = (vue.layer.clone(), rang);
    loop {
        if !vus.insert((couche.clone(), rang)) {
            return Err(StackError::Cycle { view: vue.layer.clone(), layer: couche, rank: rang });
        }
        if etages.len() >= reglages.max_depth {
            return Err(StackError::TooDeep { view: vue.layer.clone(), max_depth: reglages.max_depth });
        }
        if debut.elapsed().as_millis() as u64 > reglages.load_timeout_ms {
            return Err(StackError::LoadTimeout { view: vue.layer.clone(), ms: reglages.load_timeout_ms });
        }
        let lu = executer(&lire, &[texte("couche", &couche), QueryParam::new("rang", CypherValue::Int(rang))])?;
        let Some(ligne) = lu.rows.first() else {
            return Err(match etages.last() {
                None if vue.rank.is_some() => StackError::RankAbsent { layer: couche, rank: rang },
                None => StackError::UnknownLayer { layer: couche, without_generation: false },
                Some(enfant) => StackError::MissingParent {
                    view: vue.layer.clone(),
                    parent: couche,
                    rank: rang,
                    child: enfant.layer.clone(),
                    child_rank: enfant.rank,
                },
            });
        };
        let tombstones = match ligne.get(2) {
            Some(CypherValue::String(t)) => t.clone(),
            _ => "[]".to_string(),
        };
        let (removed_keys, removed_files) = read_tombstones(&couche, rang, &tombstones)
            .map_err(|_| StackError::UnreadableTombstones { layer: couche.clone(), rank: rang, text: tombstones.clone() })?;
        etages.push(StackLayer { layer: couche.clone(), rank: rang, removed_keys, removed_files });
        match (ligne.first(), ligne.get(1)) {
            (Some(CypherValue::String(pc)), Some(CypherValue::Int(pr))) if !pc.is_empty() => {
                couche = pc.clone();
                rang = *pr;
            }
            // Une génération sans parente écrite se relit comme racine (promesse 7).
            _ => return Ok(ReadStack { layers: etages, root_without_parent: true }),
        }
    }
}

/// **What a view knows of its own index** (the per-view index state, 18 November 2026): what it reads, and
/// whether its layer's index is behind — never another layer's. A read under view B is up to date while an
/// index writes layer A.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewState {
    /// The layer of the view (its stable id).
    pub layer: String,
    /// The rank the view reads: the requested one, or the last published generation of its layer; `None` when
    /// the layer has no generation yet (a worktree layer before 2a′-3, a base not yet published), which is not a
    /// refusal here.
    pub rank: Option<i64>,
    /// An index writes this layer right now (`Lag::is_writing`).
    pub indexing: bool,
    /// The files changed in this layer and not yet reindexed: as the watch noted them (`Lag`, its starting
    /// reconciliation included) when a watch serves the layer; otherwise the cold diff of its tree, once
    /// [`ViewState::with_cold_behind`] is applied.
    pub files_behind: usize,
    /// The oldest of those changes (Unix milliseconds), when there are some.
    pub behind_since_ms: Option<u64>,
    /// `!indexing && files_behind == 0`.
    pub up_to_date: bool,
    /// A watch serves this layer (`Lag::is_watched`): `files_behind` is its count; otherwise, the cold diff's
    /// ([`ViewState::with_cold_behind`]).
    pub watched: bool,
    /// **The seconds left to reindex the text of the files behind** (point 7 of the vision of 15 November): the files
    /// behind times the observed reindexing rate of the cell (`Lag::rate`, never a constant). `Some(0)` when nothing is
    /// behind; `None` when files are behind and no rate was observed yet.
    pub text_eta_s: Option<u64>,
    /// The observed reindexing rate of the cell (`Lag::rate`), rounded to the millisecond per file, `None` before a
    /// first index: what `text_eta_s` is computed with, kept for [`ViewState::with_cold_behind`].
    pub text_rate_ms_per_file: Option<u64>,
    /// **The seconds left to embed the view's chunks due**, set by [`ViewState::with_vectors_due`] from the chunks due,
    /// their mean size and the rate of the view's embedding service. `None` until it is set, or when no rate is known.
    pub vectors_eta_s: Option<u64>,
}

/// What a view owes in vectors, and how fast its service embeds: the input of [`ViewState::with_vectors_due`].
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct VectorsDue {
    /// The chunks of the view without a vector.
    pub chunks: usize,
    /// Their mean size, in characters.
    pub mean_chars: f64,
    /// The rate of the view's embedding service, in characters per second (an index's rate: the claim, the call and
    /// the write); `None` when no rate is known.
    pub chars_per_s: Option<f64>,
    /// Where the rate comes from, said in the answer, with embarquement's names (its page 05 of 19 November): « index »
    /// (the catalog's key of the service, an index's rate), « bench » (the bench's declared rate, halved), « unknown ».
    pub rate_from: Option<&'static str>,
}

/// **The files changed since a layer's index, without a watch** (ticket of 18 November 2026): its indexed tree
/// (`Catalog::indexed_tree_of_layer`) against its tip: the working tree of its root, or its ref for a history layer
/// (`ref` in the marker). Untracked files the index takes are counted too (`untracked_blobs`). Runs `git`: call it without the
/// catalog's lock. A path is behind when it differs from its indexed commit and was not indexed in that very state
/// (`dirty_blobs`), or when it was indexed modified and is no longer. `None` outside git, or when git refuses.
///
/// **Kept while the tip does not move**: the answer is cached under the tip's signature, its `HEAD` and the
/// modification time and size of each path `git status` names; only `git status` and `rev-parse` run again.
pub fn cold_files_behind(marque: &Value) -> Option<usize> {
    let racine = marque.get("root")?.as_str()?;
    let commit = marque.get("commit")?.as_str().filter(|c| !c.is_empty())?;
    let git = |args: &[&str]| -> Option<String> {
        let out = std::process::Command::new("git").arg("-C").arg(racine).args(args).output().ok()?;
        out.status.success().then(|| String::from_utf8_lossy(&out.stdout).to_string())
    };
    // A history layer's tip is its ref: the files changed between its indexed commit and the ref now.
    if let Some(reference) = marque.get("ref").and_then(Value::as_str) {
        let pointe = git(&["rev-parse", reference])?;
        return Some(git(&["diff", "--name-only", commit, pointe.trim()])?.lines().filter(|l| !l.is_empty()).count());
    }
    let tete = git(&["rev-parse", "HEAD"])?;
    // Untracked files included: the index takes them, so the tip's signature and the diff count them.
    let status = git(&["status", "--porcelain", "--untracked-files=all"])?;
    let mut signature = format!("{}\n{}\n", marque, tete.trim());
    // Tracked changes and untracked files alike: an edit of either moves the tip.
    for chemin in status_paths(&status).into_iter().chain(untracked_paths(racine)) {
        let m = std::fs::metadata(std::path::Path::new(racine).join(&chemin)).ok();
        let (mtime, taille) = m.map_or((0, 0), |m| {
            (m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_nanos()), m.len())
        });
        signature.push_str(&format!("{chemin}\t{mtime}\t{taille}\n"));
    }
    static GARDE: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<String, (String, usize)>>> = std::sync::OnceLock::new();
    let garde = GARDE.get_or_init(Default::default);
    let cle = format!("{racine}\n{}", marque.get("at_ms").and_then(Value::as_u64).unwrap_or(0));
    if let Some((sig, n)) = garde.lock().unwrap_or_else(|e| e.into_inner()).get(&cle) {
        if *sig == signature {
            return Some(*n);
        }
    }
    let changes: BTreeSet<String> = git(&["diff", "--name-only", commit])?.lines().filter(|l| !l.is_empty()).map(str::to_string).collect();
    let sales: std::collections::BTreeMap<String, String> = marque
        .get("dirty_blobs")
        .and_then(Value::as_object)
        .map(|m| m.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or("-").to_string())).collect())
        .unwrap_or_default();
    let blobs_de = |cle: &str| -> std::collections::BTreeMap<String, String> {
        marque
            .get(cle)
            .and_then(Value::as_object)
            .map(|m| m.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or("-").to_string())).collect())
            .unwrap_or_default()
    };
    let non_suivis_indexes = blobs_de("untracked_blobs");
    let non_suivis: BTreeSet<String> = untracked_paths(racine).into_iter().collect();
    let blob = |chemin: &str| git(&["hash-object", "--", chemin]).map(|h| h.trim().to_string()).unwrap_or_else(|| "-".to_string());
    let mut derriere = 0usize;
    // Untracked now: behind unless indexed untracked in this very state.
    for chemin in &non_suivis {
        if non_suivis_indexes.get(chemin) != Some(&blob(chemin)) {
            derriere += 1;
        }
    }
    // Untracked at the index and no longer: deleted, or tracked since (then counted by the diff below).
    derriere += non_suivis_indexes.keys().filter(|c| !non_suivis.contains(*c) && !changes.contains(*c)).count();
    for chemin in &changes {
        // Untracked at the index, tracked since, unchanged: indexed in this state.
        if non_suivis_indexes.get(chemin).is_some_and(|b| *b == blob(chemin)) {
            continue;
        }
        match sales.get(chemin) {
            // Indexed in this modified state: behind only if it moved since.
            Some(blob) => {
                let maintenant = git(&["hash-object", "--", chemin]).map(|h| h.trim().to_string()).unwrap_or_else(|| "-".to_string());
                if &maintenant != blob {
                    derriere += 1;
                }
            }
            None => derriere += 1,
        }
    }
    // Indexed modified, now back to its commit.
    derriere += sales.keys().filter(|c| !changes.contains(*c)).count();
    garde.lock().unwrap_or_else(|e| e.into_inner()).insert(cle, (signature, derriere));
    Some(derriere)
}

/// **The untracked files of `root` the index takes**: not tracked, not ignored (`git ls-files --others
/// --exclude-standard`).
pub fn untracked_paths(root: &str) -> Vec<String> {
    std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "--others", "--exclude-standard", "-z"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| o.stdout.split(|b| *b == 0).filter(|p| !p.is_empty()).map(|p| String::from_utf8_lossy(p).to_string()).collect())
        .unwrap_or_default()
}

/// The paths `git status --porcelain` names as changed (a rename names its new path); untracked lines (`??`) are not
/// changes of a tracked file.
pub fn status_paths(status: &str) -> Vec<String> {
    status
        .lines()
        .filter(|l| l.len() > 3 && !l.starts_with("??"))
        .map(|l| {
            let chemin = &l[3..];
            chemin.rsplit(" -> ").next().unwrap_or(chemin).trim_matches('"').to_string()
        })
        .collect()
}

impl ViewState {
    /// **A view without a watch counts the cold diff of its tree** as its files behind (the larger of it and what
    /// `Lag` holds); a watched view keeps the watch's count. `None` (outside git) changes nothing.
    pub fn with_cold_behind(self, files: Option<usize>) -> Self {
        let Some(froid) = files.filter(|_| !self.watched) else { return self };
        let files_behind = self.files_behind.max(froid);
        ViewState {
            files_behind,
            up_to_date: !self.indexing && files_behind == 0,
            text_eta_s: if files_behind == 0 {
                Some(0)
            } else {
                self.text_rate_ms_per_file.map(|ms| (files_behind as u64 * ms).div_ceil(1000))
            },
            ..self
        }
    }

    /// **The seconds left to embed** the chunks due: chunks × mean size ÷ rate, rounded up; `Some(0)` with nothing due,
    /// `None` without a rate.
    pub fn with_vectors_due(self, due: VectorsDue) -> Self {
        let vectors_eta_s = if due.chunks == 0 {
            Some(0)
        } else {
            due.chars_per_s.filter(|r| *r > 0.0).map(|r| (due.chunks as f64 * due.mean_chars / r).ceil() as u64)
        };
        ViewState { vectors_eta_s, ..self }
    }
}

/// **The state of a view**, read on `conn` (the snapshot of a tool call, like [`load_stack_on`]) and in `lag`
/// for the layer of the view in `cell`. A failed read of `_generation` is a refusal; an unknown layer is not.
pub fn view_state(
    conn: &dyn crate::connection::DbConnection,
    dialecte: &dyn crate::dialect::SchemaDialect,
    vue: &ViewRequest,
    lag: &crate::catalog::retard::Lag,
    cell: &crate::scope::Scope,
) -> Result<ViewState, StackError> {
    use crate::catalog::retard::{LagKey, LayerId};
    use crate::connection::{CypherValue, QueryParam};
    let rank = match (vue.rank, dialecte.latest_generation_rank()) {
        (Some(r), _) => Some(r),
        (None, None) => None,
        (None, Some(dernier)) => {
            let lu = conn
                .execute_with_params(&dernier, &[QueryParam::new("couche", CypherValue::String(vue.layer.clone()))])
                .map_err(|e| StackError::Db(format!("vues : lecture de _generation : {e}")))?;
            match lu.rows.first().and_then(|r| r.first()) {
                Some(CypherValue::Int(r)) => Some(*r),
                _ => None,
            }
        }
    };
    let key = LagKey { cell: cell.clone(), layer: LayerId(vue.layer.clone()) };
    let indexing = lag.is_writing(&key);
    let behind = lag.summary_of(&key);
    let files_behind = behind.map_or(0, |b| b.files);
    Ok(ViewState {
        layer: vue.layer.clone(),
        rank,
        indexing,
        files_behind,
        behind_since_ms: behind.map(|b| b.since_ms),
        up_to_date: !indexing && files_behind == 0,
        watched: lag.is_watched(&key),
        text_eta_s: if files_behind == 0 {
            Some(0)
        } else {
            lag.rate(cell).map(|ms_par_fichier| (files_behind as f64 * ms_par_fichier / 1000.0).ceil() as u64)
        },
        text_rate_ms_per_file: lag.rate(cell).map(|ms| ms.round() as u64),
        vectors_eta_s: None,
    })
}

#[cfg(all(test, feature = "rag3db-native"))]
mod tests {
    use super::*;
    use serde_json::json;

    fn catalogue() -> crate::Catalog {
        let conn = crate::Rag3dbConnection::in_memory().unwrap();
        let mut cat = crate::Catalog::without_embedding_model(Box::new(conn), crate::CatalogConfig::default());
        cat.initialize().unwrap();
        let ddl = cat.dialect_arc().create_generation_table().expect("le dialecte sait les générations");
        cat.execute_raw(&ddl).unwrap();
        cat
    }

    fn poser(cat: &crate::Catalog, couche: &str, rang: i64, parente: Option<(&str, i64)>, tombstones: &str) {
        let (pc, pr) = match parente {
            Some((c, r)) => (format!("'{c}'"), r.to_string()),
            None => ("NULL".to_string(), "NULL".to_string()),
        };
        let t = tombstones.replace('\'', "\\'");
        cat.execute_raw(&format!(
            "CREATE (:_generation {{_key: '{couche}:{rang}', couche: '{couche}', rang: {rang}, commit: '', base: '', recette: '', \
             parente_couche: {pc}, parente_rang: {pr}, tombstones: '{t}', publiee_le: 0}})"
        ))
        .unwrap();
    }

    fn vue(couche: &str, rang: Option<i64>) -> ViewRequest {
        ViewRequest { layer: couche.into(), rank: rang }
    }

    fn etage(couche: &str, rang: i64, cles: &[&str], fichiers: &[&str]) -> StackLayer {
        StackLayer {
            layer: couche.into(),
            rank: rang,
            removed_keys: cles.iter().map(|s| s.to_string()).collect(),
            removed_files: fichiers.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// **Une pile d'un étage** : la génération 0 de `backend`, sans parente écrite, se relit comme racine ;
    /// un rang omis prend la dernière génération publiée.
    #[test]
    fn a_one_layer_stack_and_its_root_without_parent() {
        let cat = catalogue();
        poser(&cat, "backend", 0, None, "[]");
        let p = load_stack(&cat, &vue("backend", None), &ViewSettings::default()).unwrap();
        assert_eq!(p, ReadStack { layers: vec![etage("backend", 0, &[], &[])], root_without_parent: true });
        assert_eq!(load_stack(&cat, &vue("backend", Some(0)), &ViewSettings::default()).unwrap(), p);
    }

    /// **Une pile de deux étages** : l'enfant sur sa parente épinglée, ses tombstones lus ; un rang omis
    /// prend le dernier rang de l'enfant.
    #[test]
    fn a_two_layer_stack_with_its_tombstones() {
        let cat = catalogue();
        poser(&cat, "backend", 0, None, "[]");
        poser(&cat, "backend", 1, None, "[]");
        poser(&cat, "enfant", 1, Some(("backend", 0)), r#"{"keys": ["k-f"], "files": ["src/vieux.rs"]}"#);
        poser(&cat, "enfant", 2, Some(("backend", 0)), r#"{"keys": ["k-f", "k-g"], "files": []}"#);
        let p = load_stack(&cat, &vue("enfant", None), &ViewSettings::default()).unwrap();
        assert_eq!(
            p,
            ReadStack { layers: vec![etage("enfant", 2, &["k-f", "k-g"], &[]), etage("backend", 0, &[], &[])], root_without_parent: true }
        );
        let p1 = load_stack(&cat, &vue("enfant", Some(1)), &ViewSettings::default()).unwrap();
        assert_eq!(p1.layers[0], etage("enfant", 1, &["k-f"], &["src/vieux.rs"]));
        assert_eq!(p1.layers[1], etage("backend", 0, &[], &[]), "la parente reste épinglée à son rang, pas au dernier");
    }

    /// **La publication écrit une racine sans parente** (promesse 6, `backend` seul) : la génération 0
    /// posée par `poser_les_generations`, par le vrai `merge_generation`, se relit comme racine.
    #[test]
    fn generation_0_published_reads_back_as_a_root() {
        let conn = crate::Rag3dbConnection::in_memory().unwrap();
        let mut cat = crate::Catalog::without_embedding_model(Box::new(conn), crate::CatalogConfig::default());
        cat.initialize().unwrap();
        cat.poser_les_generations(&[], "backend").unwrap();
        let p = load_stack(&cat, &vue("backend", None), &ViewSettings::default()).unwrap();
        assert_eq!(p, ReadStack { layers: vec![etage("backend", 0, &[], &[])], root_without_parent: true });
        let lu = cat.execute_raw("MATCH (g:_generation) RETURN g.parente_couche IS NULL, g.parente_rang IS NULL").unwrap();
        assert_eq!(lu.rows.len(), 1);
        assert!(lu.rows[0].iter().all(|v| matches!(v, crate::connection::CypherValue::Bool(true))), "la racine porte une parente : {:?}", lu.rows);
    }

    /// **Le chargeur sur connexion** rend la même pile que par le catalogue (l'instantané d'un appel).
    #[test]
    fn the_loader_on_a_connection_gives_the_same_stack() {
        let cat = catalogue();
        poser(&cat, "backend", 0, None, "[]");
        poser(&cat, "enfant", 1, Some(("backend", 0)), r#"{"keys": ["k"], "files": []}"#);
        let r = ViewSettings::default();
        let par_le_catalogue = load_stack(&cat, &vue("enfant", None), &r).unwrap();
        let sur_connexion = load_stack_on(cat.conn(), cat.dialect_arc().as_ref(), &vue("enfant", None), &r).unwrap();
        assert_eq!(sur_connexion, par_le_catalogue);
    }

    /// **Les refus nommés** : une couche inconnue, un rang absent, une parente manquante, un cycle, une
    /// profondeur au-delà du réglage, des tombstones illisibles.
    #[test]
    fn the_loader_refusals_name_themselves() {
        let cat = catalogue();
        poser(&cat, "backend", 0, None, "[]");
        poser(&cat, "orpheline", 1, Some(("disparue", 3)), "[]");
        poser(&cat, "a", 1, Some(("b", 1)), "[]");
        poser(&cat, "b", 1, Some(("a", 1)), "[]");
        poser(&cat, "c1", 1, Some(("backend", 0)), "[]");
        poser(&cat, "c2", 1, Some(("c1", 1)), "[]");
        poser(&cat, "illisible", 1, None, "pas du json");
        let r = ViewSettings::default();
        for (v, mot) in [
            (vue("inconnue", None), "inconnue"),
            (vue("backend", Some(7)), "7"),
            (vue("orpheline", None), "disparue:3"),
            (vue("a", None), "cycle"),
            (vue("illisible", None), "tombstones"),
        ] {
            let refus = load_stack(&cat, &v, &r).expect_err(&format!("{v:?} se charge")).to_string();
            assert!(refus.contains(mot), "le refus de {v:?} ne nomme pas « {mot} » : {refus}");
        }
        let court = ViewSettings { max_depth: 2, ..r };
        let refus = load_stack(&cat, &vue("c2", None), &court).expect_err("trois étages sous une profondeur de 2").to_string();
        assert!(refus.contains("profondeur"), "{refus}");
        assert!(load_stack(&cat, &vue("c2", None), &ViewSettings::default()).is_ok());
    }

    /// **Each refusal has its kind** (2b-C's name fallback tests the kind, not the text), and `Display` keeps the
    /// text of before.
    #[test]
    fn each_refusal_has_its_kind() {
        let cat = catalogue();
        poser(&cat, "backend", 0, None, "[]");
        poser(&cat, "orpheline", 1, Some(("disparue", 3)), "[]");
        poser(&cat, "a", 1, Some(("b", 1)), "[]");
        poser(&cat, "b", 1, Some(("a", 1)), "[]");
        poser(&cat, "illisible", 1, None, "pas du json");
        let r = ViewSettings::default();
        let e = |v: ViewRequest| load_stack(&cat, &v, &r).expect_err("loads");
        assert_eq!(e(vue("inconnue", None)), StackError::UnknownLayer { layer: "inconnue".into(), without_generation: true });
        assert_eq!(e(vue("inconnue", None)).to_string(), "vue « inconnue » : couche inconnue (couche sans génération, 2a′-3)");
        assert_eq!(e(vue("backend", Some(7))), StackError::RankAbsent { layer: "backend".into(), rank: 7 });
        assert!(
            matches!(e(vue("orpheline", None)), StackError::MissingParent { ref parent, rank: 3, .. } if parent == "disparue"),
            "{:?}",
            e(vue("orpheline", None))
        );
        assert!(matches!(e(vue("a", None)), StackError::Cycle { .. }));
        assert!(matches!(e(vue("illisible", None)), StackError::UnreadableTombstones { rank: 1, .. }));
        assert!(matches!(load_stack(&cat, &vue("a", None), &ViewSettings { max_depth: 1, ..r }).unwrap_err(), StackError::TooDeep { max_depth: 1, .. }));
    }

    /// **A view is up to date while another layer is indexed** (Lucie, 18 November 2026: another worktree must not
    /// read « indexation en cours » when its own is up to date). Two layers by hand: an index writes layer `a` and
    /// the watch noted two files in it; view `b` is up to date, not indexing, nothing behind; `a` says it.
    #[test]
    fn a_view_is_up_to_date_while_another_layer_is_indexed() {
        use crate::catalog::retard::{LagKey, LayerId};
        let cat = catalogue();
        poser(&cat, "backend", 0, None, "[]");
        poser(&cat, "a", 1, Some(("backend", 0)), "[]");
        poser(&cat, "b", 1, Some(("backend", 0)), "[]");
        let lag = std::sync::Arc::new(crate::catalog::retard::Lag::new());
        let cell = cat.scope().clone();
        let a = LagKey { cell: cell.clone(), layer: LayerId("a".into()) };
        let _ecrit = lag.writing_guard(a.clone());
        lag.note_in(&a, "src/x.rs", 1_000);
        lag.note_in(&a, "src/y.rs", 2_000);
        let etat = |v: &str| view_state(cat.conn(), cat.dialect_arc().as_ref(), &vue(v, None), &lag, &cell).unwrap();
        let b = etat("b");
        assert_eq!(
            b,
            ViewState { layer: "b".into(), rank: Some(1), indexing: false, files_behind: 0, behind_since_ms: None, up_to_date: true, watched: false, text_eta_s: Some(0), text_rate_ms_per_file: None, vectors_eta_s: None },
            "view b is held behind layer a's index"
        );
        let a_ = etat("a");
        assert!(a_.indexing && !a_.up_to_date, "layer a, being written, says it is up to date: {a_:?}");
        assert_eq!((a_.files_behind, a_.behind_since_ms), (2, Some(1_000)), "{a_:?}");
        assert!(etat("backend").up_to_date, "the backend's layer is held behind layer a's index");
    }

    /// **The estimates of a view follow the rates** (point 7 of the vision of 15 November, page
    /// `docs/arbre-principal/19-11-2026/05-l-estimation-du-reste-par-vue.md`). Layer a has 4 files behind and an observed
    /// reindexing rate of 500 ms per file: `text_eta_s` is 2 s, and 4 s at 1 000 ms. 1 000 chunks due of 600 characters
    /// under a rate of 200 000 characters per second: `vectors_eta_s` is 3 s. Without a rate, `None`. View b, with
    /// nothing behind and nothing due: 0 and 0.
    #[test]
    fn the_estimates_of_a_view_follow_the_rates() {
        use crate::catalog::retard::{LagKey, LayerId};
        let cat = catalogue();
        poser(&cat, "backend", 0, None, "[]");
        poser(&cat, "a", 1, Some(("backend", 0)), "[]");
        poser(&cat, "b", 1, Some(("backend", 0)), "[]");
        let lag = crate::catalog::retard::Lag::new();
        let cell = cat.scope().clone();
        let a = LagKey { cell: cell.clone(), layer: LayerId("a".into()) };
        for f in ["w.rs", "x.rs", "y.rs", "z.rs"] {
            lag.note_in(&a, f, 1_000);
        }
        let etat = |v: &str| view_state(cat.conn(), cat.dialect_arc().as_ref(), &vue(v, None), &lag, &cell).unwrap();
        assert_eq!(etat("a").text_eta_s, None, "files behind without an observed rate have an estimate");
        lag.set_rate(&cell, 500.0);
        assert_eq!(etat("a").text_eta_s, Some(2), "4 files at 500 ms per file are not 2 s");
        lag.set_rate(&cell, 1_000.0);
        assert_eq!(etat("a").text_eta_s, Some(4), "the estimate does not follow the rate");
        let du = VectorsDue { chunks: 1_000, mean_chars: 600.0, chars_per_s: Some(200_000.0), rate_from: Some("index") };
        assert_eq!(etat("a").with_vectors_due(du).vectors_eta_s, Some(3), "1 000 chunks of 600 characters at 200 000 c/s are not 3 s");
        assert_eq!(etat("a").with_vectors_due(VectorsDue { chars_per_s: None, ..du }).vectors_eta_s, None, "an unknown rate gives an estimate");
        let b = etat("b").with_vectors_due(VectorsDue { chunks: 0, ..du });
        assert_eq!((b.text_eta_s, b.vectors_eta_s), (Some(0), Some(0)), "a view with nothing due is not at 0 and 0");
    }

    /// **A view without a watch sees its lag** (ticket `2026-11-18-l-etat-d-une-vue-sans-montre-ne-voit-pas-son-retard`):
    /// a git tree, `g.rs` already modified when the layer is indexed. Right after the index, nothing is behind (the dirty
    /// `g.rs` was indexed as it is). `f.rs` changed: 1 behind, not up to date. `g.rs` brought back to its commit: 2 behind
    /// (the index holds its modified text). A watch on the layer: its count (0 here) is the count, not the cold diff.
    #[test]
    fn a_view_without_a_watch_sees_its_lag() {
        use crate::catalog::retard::{LagKey, LayerId};
        let d = tempfile::tempdir().unwrap();
        let racine = d.path();
        let git = |args: &[&str]| {
            let o = std::process::Command::new("git").arg("-C").arg(racine).args(args).output().unwrap();
            assert!(o.status.success(), "git {args:?} : {}", String::from_utf8_lossy(&o.stderr));
        };
        git(&["init", "-q"]);
        std::fs::write(racine.join("f.rs"), "pub fn f() {}\n").unwrap();
        std::fs::write(racine.join("g.rs"), "pub fn g() {}\n").unwrap();
        git(&["add", "."]);
        git(&["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "-m", "c1"]);
        std::fs::write(racine.join("g.rs"), "pub fn g() -> u8 { 1 }\n").unwrap();
        let cat = catalogue();
        poser(&cat, "backend", 0, None, "[]");
        let cell = cat.scope().clone();
        cat.note_indexed_tree_of_layer(&cell, "backend", &crate::catalog::indexed_tree_of(&racine.to_string_lossy())).unwrap();
        let lag = std::sync::Arc::new(crate::catalog::retard::Lag::new());
        let etat = || {
            let marque = cat.indexed_tree_of_layer(&cell, "backend").expect("the layer's indexed tree");
            view_state(cat.conn(), cat.dialect_arc().as_ref(), &vue("backend", None), &lag, &cell).unwrap().with_cold_behind(cold_files_behind(&marque))
        };
        let s = etat();
        assert!(s.up_to_date && s.files_behind == 0, "right after its index, the layer is behind: {s:?}");
        std::fs::write(racine.join("f.rs"), "pub fn f() -> u8 { 2 }\n").unwrap();
        let s = etat();
        assert!(!s.up_to_date && s.files_behind == 1, "a file changed since the index, the view without a watch says it is up to date: {s:?}");
        git(&["checkout", "-q", "--", "g.rs"]);
        let s = etat();
        assert_eq!(s.files_behind, 2, "a file indexed modified, then brought back, is not behind: {s:?}");
        let _montre = lag.watching_guard(LagKey { cell: cell.clone(), layer: LayerId("backend".into()) });
        let s = etat();
        assert!(s.watched && s.up_to_date && s.files_behind == 0, "a watched layer takes the cold diff instead of its watch's count: {s:?}");
    }

    /// **A view without a watch sees its untracked files** (the rest of the ticket of 18 November 2026): the index takes
    /// the files git does not track, so the cold diff counts them too. `n.rs` untracked at the index and unchanged is not
    /// behind; edited, it is; `m.rs`, new after the index, is; `o.rs`, untracked at the index then deleted, is.
    #[test]
    fn a_view_without_a_watch_sees_its_untracked_files() {
        let d = tempfile::tempdir().unwrap();
        let racine = d.path();
        let git = |args: &[&str]| {
            let o = std::process::Command::new("git").arg("-C").arg(racine).args(args).output().unwrap();
            assert!(o.status.success(), "git {args:?} : {}", String::from_utf8_lossy(&o.stderr));
        };
        git(&["init", "-q"]);
        std::fs::write(racine.join("f.rs"), "pub fn f() {}\n").unwrap();
        git(&["add", "."]);
        git(&["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "-m", "c1"]);
        std::fs::write(racine.join("n.rs"), "pub fn n() {}\n").unwrap();
        std::fs::write(racine.join("o.rs"), "pub fn o() {}\n").unwrap();
        let cat = catalogue();
        poser(&cat, "backend", 0, None, "[]");
        let cell = cat.scope().clone();
        cat.note_indexed_tree_of_layer(&cell, "backend", &crate::catalog::indexed_tree_of(&racine.to_string_lossy())).unwrap();
        let lag = std::sync::Arc::new(crate::catalog::retard::Lag::new());
        let etat = || {
            let marque = cat.indexed_tree_of_layer(&cell, "backend").expect("the layer's indexed tree");
            view_state(cat.conn(), cat.dialect_arc().as_ref(), &vue("backend", None), &lag, &cell).unwrap().with_cold_behind(cold_files_behind(&marque))
        };
        let s = etat();
        assert!(s.up_to_date && s.files_behind == 0, "right after its index, untracked files indexed as they are are behind: {s:?}");
        std::fs::write(racine.join("n.rs"), "pub fn n() -> u8 { 1 }\n").unwrap();
        assert_eq!(etat().files_behind, 1, "an untracked file edited since the index is not behind");
        std::fs::write(racine.join("m.rs"), "pub fn m() {}\n").unwrap();
        assert_eq!(etat().files_behind, 2, "an untracked file new since the index is not behind");
        std::fs::remove_file(racine.join("o.rs")).unwrap();
        let s = etat();
        assert!(!s.up_to_date && s.files_behind == 3, "an untracked file indexed then deleted is not behind: {s:?}");
    }

    /// **An unknown layer is a state, not a refusal**: a worktree layer before 2a′-3 has no generation; its view
    /// has no rank, and its lag is still read.
    #[test]
    fn an_unknown_layer_has_a_state_without_rank() {
        let cat = catalogue();
        let lag = crate::catalog::retard::Lag::new();
        let s = view_state(cat.conn(), cat.dialect_arc().as_ref(), &vue("sans-generation", None), &lag, cat.scope()).unwrap();
        assert_eq!(s.rank, None);
        assert!(s.up_to_date, "{s:?}");
        let s7 = view_state(cat.conn(), cat.dialect_arc().as_ref(), &vue("sans-generation", Some(7)), &lag, cat.scope()).unwrap();
        assert_eq!(s7.rank, Some(7), "a requested rank is the rank read");
    }

    /// **Les réglages des vues** : les défauts sans `views`, les valeurs déclarées, et une clé inconnue
    /// refusée en la nommant ; `tracked` accepte la clé `views`.
    #[test]
    fn the_view_settings_and_their_refusals() {
        assert_eq!(view_settings(None).unwrap(), ViewSettings::default());
        assert_eq!(view_settings(Some(&json!({"sources": []}))).unwrap(), ViewSettings::default());
        assert_eq!(
            view_settings(Some(&json!({"views": {"max_depth": 4, "load_timeout_ms": 500}}))).unwrap(),
            ViewSettings { max_depth: 4, load_timeout_ms: 500 }
        );
        for (mauvais, mot) in [(json!({"views": {"profondeur": 4}}), "profondeur"), (json!({"views": {"max_depth": 0}}), "max_depth"), (json!({"views": 3}), "views")] {
            let refus = view_settings(Some(&mauvais)).expect_err(&format!("{mauvais} est accepté"));
            assert!(refus.contains(mot), "le refus de {mauvais} ne nomme pas « {mot} » : {refus}");
        }
        crate::suivies::declaration_du_manifeste(Some(&json!({"views": {"max_depth": 4}, "sources": []})))
            .expect("tracked refuse la clé views");
    }
}
