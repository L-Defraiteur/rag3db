//! **Le code sur la synchronisation déclarée** (3 octobre 2026).
//!
//! L'entité `Scope` déclare son périmètre — la source — et un grain fin — le
//! fichier (`SnapshotConfig { scope: [source], fine_scope: [file_path] }`). Ce
//! module en fait les verbes du code ; le mécanisme, lui, ne connaît aucun nom
//! de champ ([`crate::catalog`], `synchronisation.rs`).
//!
//! - [`reingest_file`] : une édition connaît tout le contenu de son fichier.
//!   C'est une **fin immédiate** sur le grain fin — ouvrir, marquer les scopes
//!   que l'analyse produit encore, finir —, **avant** `ingest_code` : un scope
//!   déplacé dans le même fichier (même nom, autre parent) aurait sinon deux
//!   définisseurs au moment de la résolution par `Symbol`, qui s'abstient sur
//!   un nom ambigu. `allowEmpty` (un fichier vidé de ses scopes est légitime)
//!   et `force` (renommer un scope sur deux est une édition ordinaire) : le
//!   contenu entier est connu, les garde-fous d'un instantané partiel n'ont
//!   rien à protéger ici.
//! - Si une synchronisation de la source entière tient le périmètre large,
//!   le grain fin est refusé : l'édition **écrit simplement**, la marque à
//!   l'écriture la compte pour la session large, et les scopes disparus du
//!   fichier partent à la fin de celle-ci.
//!
//! - [`sync_source`] : la **synchronisation d'une source entière**. Elle ouvre
//!   une session sur le grain large de `Scope` et de `File`, ingère la source
//!   par paquets de fichiers en marquant ce qu'elle porte, puis finit : ce qui
//!   a disparu de la source — scopes, et fichiers supprimés — part, par la
//!   mise de côté. C'est la brique que les produits appellent pour « indexer
//!   ce dépôt » et le tenir à jour.
//!
//! - **L'édition pendant l'indexation** : une synchronisation tient le
//!   catalogue de bout en bout, et une édition n'attend jamais son verrou. Le
//!   [registre](note_change) hors du catalogue dit, pour un fichier édité,
//!   s'il sera lu à son paquet (pas encore passé : il le sera dans son état
//!   édité) ou s'il est à reprendre (déjà passé) ; la synchronisation reprend
//!   ces derniers à sa toute fin, par [`reingest_file`], et le compte dans
//!   son avancement.
//!
//! Le module est neuf exprès : `code.rs` et `code_tools.rs` sont indexés par
//! le banc de recherche (corpus vivant).

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::{LazyLock, Mutex};

use serde::{Deserialize, Serialize};

use crate::catalog::{Catalog, CatalogError, SnapshotFinish, SnapshotFinishOptions};
use crate::code::{FILE, SCOPE};
use crate::code_tools::{FileSource, ReingestReport};
use crate::connection::CypherValue;
use crate::disponibilite::Disponibilites;

/// Le grain fin d'un fichier : sa source et son nom indexé.
fn grain_du_fichier(source_id: &str, indexed_path: &str) -> BTreeMap<String, CypherValue> {
    BTreeMap::from([
        ("source".to_string(), CypherValue::String(source_id.to_string())),
        ("file_path".to_string(), CypherValue::String(indexed_path.to_string())),
    ])
}

/// Une synchronisation en cours, vue du registre : les fichiers déjà lus,
/// et ceux qu'une édition a touchés après leur passage.
#[derive(Default)]
struct EnCours {
    /// Les synchronisations de cette source en cours dans le processus.
    tenants: usize,
    lus: BTreeSet<String>,
    a_reprendre: BTreeSet<String>,
}

/// **Le registre « à reprendre »**, hors du catalogue : une édition le
/// consulte sans attendre le verrou que la synchronisation tient. Par
/// identité de source ; les chemins sont ceux de la source.
static EN_COURS: LazyLock<Mutex<HashMap<String, EnCours>>> = LazyLock::new(Default::default);

/// Ce qu'une indexation en cours fait d'un fichier qui vient de changer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ChangeDuringIndexing {
    /// Aucune indexation de cette source en cours.
    NotIndexing,
    /// Pas encore passé : son paquet le lira dans son état présent.
    ReadAtItsBatch,
    /// Déjà passé : repris à la fin de l'indexation en cours.
    QueuedForResume,
}

/// **Un fichier de la source vient de changer** (écrit, ou supprimé) : à
/// appeler *après* l'écriture. Si une indexation de la source est en cours,
/// le fichier est soit lu plus tard à son paquet, soit mis à reprendre.
///
/// L'ordre fait la justesse : la synchronisation inscrit un chemin comme lu
/// *avant* de le lire, l'édition écrit *avant* de consulter ; un chemin
/// trouvé non lu le sera donc après l'écriture.
pub fn note_change(source_id: &str, path: &str) -> ChangeDuringIndexing {
    let mut registre = EN_COURS.lock().unwrap();
    let Some(en_cours) = registre.get_mut(source_id) else {
        return ChangeDuringIndexing::NotIndexing;
    };
    if en_cours.lus.contains(path) {
        en_cours.a_reprendre.insert(path.to_string());
        ChangeDuringIndexing::QueuedForResume
    } else {
        ChangeDuringIndexing::ReadAtItsBatch
    }
}

/// Une indexation de cette source est-elle en cours dans le processus ?
pub fn indexing_in_progress(source_id: &str) -> bool {
    EN_COURS.lock().unwrap().contains_key(source_id)
}

fn inscrire(source_id: &str) {
    EN_COURS.lock().unwrap().entry(source_id.to_string()).or_default().tenants += 1;
}

fn marquer_lus(source_id: &str, paths: &[String]) {
    if let Some(en_cours) = EN_COURS.lock().unwrap().get_mut(source_id) {
        en_cours.lus.extend(paths.iter().cloned());
    }
}

fn nombre_a_reprendre(source_id: &str) -> usize {
    EN_COURS.lock().unwrap().get(source_id).map_or(0, |e| e.a_reprendre.len())
}

/// Les fichiers à reprendre — ou, s'il n'y en a plus, la fin de
/// l'inscription, **sous le même verrou** : une édition qui arrive ensuite
/// trouve la source hors indexation, au lieu d'une file que personne ne
/// viderait plus.
fn reprendre_ou_sortir(source_id: &str) -> Option<Vec<String>> {
    let mut registre = EN_COURS.lock().unwrap();
    let en_cours = registre.get_mut(source_id)?;
    if en_cours.a_reprendre.is_empty() {
        en_cours.tenants -= 1;
        if en_cours.tenants == 0 {
            registre.remove(source_id);
        }
        return None;
    }
    Some(std::mem::take(&mut en_cours.a_reprendre).into_iter().collect())
}

/// **Éditer sans attendre le catalogue** : le chemin des outils, où le
/// catalogue est partagé. S'il est libre, l'édition le prend et ré-ingère
/// tout de suite ([`edit_file`](crate::code_tools::edit_file)). S'il est
/// tenu par une indexation de la source, l'édition écrit le fichier et
/// s'inscrit au registre — lu à son paquet, ou repris à la fin — sans
/// toucher l'index maintenant. Tenu par autre chose, on attend, comme avant.
pub fn edit_file_shared(
    source: &dyn FileSource,
    catalog: &Mutex<Catalog>,
    path: &str,
    op: &crate::code_tools::EditOp,
) -> Result<crate::code_tools::EditResult, String> {
    let (source_id, _) = crate::code_tools::indexed_name(source, path);
    loop {
        match catalog.try_lock() {
            Ok(mut guard) => return crate::code_tools::edit_file(source, Some(&mut guard), path, op),
            Err(std::sync::TryLockError::Poisoned(e)) => return Err(format!("catalogue empoisonné : {e}")),
            Err(std::sync::TryLockError::WouldBlock) if indexing_in_progress(&source_id) => break,
            // Tenu, mais pas par une indexation de cette source — ou pas
            // encore inscrite : on repasse dans un instant.
            Err(std::sync::TryLockError::WouldBlock) => std::thread::sleep(std::time::Duration::from_millis(5)),
        }
    }
    let (mut result, after_text) = crate::code_tools::write_edit(source, path, op)?;
    match note_change(&source_id, path) {
        // L'indexation a fini entre-temps : le catalogue se libère, on ré-ingère.
        ChangeDuringIndexing::NotIndexing => {
            let mut guard = catalog.lock().map_err(|e| format!("catalogue empoisonné : {e}"))?;
            match crate::code_tools::reingest_file(&mut guard, source, path, &after_text) {
                Ok(r) => result.reingest = Some(r),
                Err(e) => result.index_pending = Some(e),
            }
        }
        suite => result.during_indexing = Some(suite),
    }
    Ok(result)
}

/// **Ré-ingère un seul fichier**, par la synchronisation déclarée : analyse
/// seule (références locales et `DEFINED_IN` ; l'inter-fichiers attend la
/// résolution contre la base), retrait des scopes disparus par une fin
/// immédiate sur le grain du fichier, puis upsert du reste.
pub fn reingest_file(
    catalog: &mut Catalog,
    source: &dyn FileSource,
    path: &str,
    content: &str,
    exige: Disponibilites,
) -> Result<ReingestReport, String> {
    let cursor = source.cursor();
    let (root, virtual_source) = match cursor.strip_prefix("worktree:") {
        Some(root) => (root.to_string(), false),
        None => ("/".to_string(), true),
    };
    // Les chemins du projet : un import vers un autre fichier n'est pas une
    // bibliothèque. Une source qui ne sait pas se lister analyse sans eux.
    let projet = source.list().ok();
    let mut analysis =
        crate::code::analyze_in_project(&root, vec![(path.to_string(), content.to_string())], &cursor, projet.as_deref());
    for f in &mut analysis.files {
        f.cursor = cursor.clone();
        if virtual_source {
            f.absolute_path.clear();
        }
    }
    let (source_id, indexed_path) = crate::code_tools::indexed_name(source, path);
    let grain = grain_du_fichier(&source_id, &indexed_path);

    // La fin immédiate sur le grain du fichier — ou rien, si la source entière
    // est en cours de synchronisation : alors elle s'en charge.
    let mut deleted = 0usize;
    let mut deferred_to = None;
    let mut noms_retires = Vec::new();
    match catalog.begin_snapshot(SCOPE, &grain, true) {
        Ok(open) => {
            let uuids = analysis
                .scopes
                .iter()
                .map(|s| {
                    catalog.entity_uuid(SCOPE, &BTreeMap::from([("key".to_string(), CypherValue::String(s.key.clone()))]))
                })
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            catalog.mark_snapshot(SCOPE, &grain, &open.session, &uuids).map_err(|e| e.to_string())?;
            let plan = catalog
                .plan_snapshot_finish(SCOPE, &grain, &open.session, SnapshotFinishOptions { allow_empty: true, force: true })
                .map_err(|e| e.to_string())?;
            noms_retires = noms_des_scopes(catalog, &plan.removed)?;
            let fin = catalog.apply_snapshot_finish(plan).map_err(|e| e.to_string())?;
            deleted = fin.removed.len();
        }
        Err(CatalogError::SnapshotRefused(raison)) => deferred_to = Some(raison),
        Err(e) => return Err(e.to_string()),
    }
    oublier_les_aretes_du_fichier(catalog, &source_id, &indexed_path)?;
    let report = catalog.ingest_code_jusqu_a(&analysis, exige).map_err(|e| e.to_string())?;
    // Un nom dont un définisseur vient de partir a pu redevenir unique.
    catalog.resoudre_les_symboles(&noms_retires, exige).map_err(|e| e.to_string())?;
    Ok(ReingestReport {
        scopes_upserted: report.scopes,
        scopes_deleted: deleted,
        relations: report.relations,
        failed: report.failed,
        deferred_to,
    })
}

/// **Ce que les scopes gardés d'un fichier affirmaient**, retiré avant de le
/// ré-ingérer : un scope qui reste (même clé) mais n'appelle plus `f` garderait
/// sinon son arête vers `f`. Ce sont les arêtes **sortantes** des scopes du
/// fichier — l'ingestion les repose toutes : relations de l'analyse,
/// rendez-vous, arêtes résolues de ses mentionneurs — et le miroir
/// `CONSUMED_BY` de ses `CONSUMES`. Les arêtes **entrantes** venues d'autres
/// fichiers restent : leurs appelants n'ont pas changé.
fn oublier_les_aretes_du_fichier(catalog: &mut Catalog, source_id: &str, indexed_path: &str) -> Result<(), String> {
    let params = [
        crate::connection::QueryParam::new("source", CypherValue::String(source_id.to_string())),
        crate::connection::QueryParam::new("path", CypherValue::String(indexed_path.to_string())),
    ];
    let filtre = "WHERE s.source = $source AND s.file_path = $path DELETE r";
    for (rel, from, _) in crate::code::RELATIONS {
        if from == SCOPE && rel != "CONSUMED_BY" {
            catalog
                .execute_raw_with_params(&format!("MATCH (s:{SCOPE})-[r:{rel}]->() {filtre}"), &params)
                .map_err(|e| e.to_string())?;
        }
    }
    catalog
        .execute_raw_with_params(&format!("MATCH ()-[r:CONSUMED_BY]->(s:{SCOPE}) {filtre}"), &params)
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// **Retire un fichier supprimé de la source** : ses scopes par une fin
/// immédiate sur son grain, où rien n'est marqué, puis sa ligne `File`. Le
/// pendant de [`reingest_file`] pour un fichier qui n'existe plus.
pub fn remove_file(catalog: &mut Catalog, source: &dyn FileSource, path: &str, exige: Disponibilites) -> Result<ReingestReport, String> {
    let (source_id, indexed_path) = crate::code_tools::indexed_name(source, path);
    let grain = grain_du_fichier(&source_id, &indexed_path);
    let open = catalog.begin_snapshot(SCOPE, &grain, true).map_err(|e| e.to_string())?;
    let plan = catalog
        .plan_snapshot_finish(SCOPE, &grain, &open.session, SnapshotFinishOptions { allow_empty: true, force: true })
        .map_err(|e| e.to_string())?;
    let noms_retires = noms_des_scopes(catalog, &plan.removed)?;
    let fin = catalog.apply_snapshot_finish(plan).map_err(|e| e.to_string())?;
    let fichier = BTreeMap::from([
        ("source".to_string(), CypherValue::String(source_id)),
        ("path".to_string(), CypherValue::String(indexed_path)),
    ]);
    let uuid = catalog.entity_uuid(FILE, &fichier).map_err(|e| e.to_string())?;
    if !catalog.get_many(FILE, std::slice::from_ref(&uuid)).map_err(|e| e.to_string())?.is_empty() {
        catalog.delete_jusqu_a(FILE, &uuid, exige).map_err(|e| e.to_string())?;
    }
    catalog.resoudre_les_symboles(&noms_retires, exige).map_err(|e| e.to_string())?;
    Ok(ReingestReport { scopes_deleted: fin.removed.len(), ..Default::default() })
}

/// Ce qu'on demande à une synchronisation de source.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SourceSyncOptions {
    /// Fichiers par paquet d'ingestion.
    pub batch_files: usize,
    /// S'arrêter au plan : la source est ingérée (c'est l'avancement), mais
    /// rien n'est retiré et les sessions sont abandonnées. Le rapport dit ce
    /// qui le serait — l'estimation avant de confirmer.
    pub plan_only: bool,
    /// Reprendre une synchronisation abandonnée de la même source.
    pub takeover: bool,
    /// Les échappatoires des garde-fous de la fin, explicites.
    pub allow_empty: bool,
    pub force: bool,
    /// Ce qui doit être prêt quand un paquet rend (`RECHERCHE_TEXTE` laisse
    /// la dette de vecteurs en base).
    #[serde(skip, default = "tout")]
    pub exige: Disponibilites,
    /// Comment poser les relations. `None` : en masse
    /// ([`RelationsMode::Bulk`]) quand la source n'a encore rien en base,
    /// paquet par paquet sinon.
    pub relations: Option<RelationsMode>,
    /// Les fichiers générés, écartés avant l'analyse et comptés au rapport
    /// (`SourceSyncReport::files_set_aside`). Les défauts écartent ;
    /// `GeneratedPolicy::off()` lève la règle.
    pub generated: crate::generated::GeneratedPolicy,
}

/// **Comment une synchronisation pose les relations.**
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RelationsMode {
    /// Avec chaque paquet : les liens sont là au fil de l'eau. Pour une
    /// source déjà indexée, où seuls les fichiers changés en apportent.
    PerBatch,
    /// À la fin, en une fois : les paquets ne posent que les nœuds et le plein
    /// texte ; les relations et les rendez-vous de la résolution attendent en
    /// file — vidée par COPY au-delà de [`BULK_QUEUE_LIMIT`] liens, pour borner
    /// la mémoire — et la résolution par `Symbol` tourne une fois, sur toute
    /// la source. Pour une première indexation.
    Bulk,
}

/// Au-delà de ce nombre de liens en file, le chemin de masse les pose (par
/// COPY) sans attendre la fin : la mémoire reste bornée sur un poste modeste
/// — de l'ordre de la centaine de Mo pour 200 000 liens.
pub const BULK_QUEUE_LIMIT: usize = 200_000;

/// Où en est une synchronisation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SyncPhase {
    /// Les paquets : nœuds et plein texte (et relations, en `PerBatch`).
    #[default]
    Nodes,
    /// Le chargement final des relations et la résolution (`Bulk`).
    Relations,
    /// La reprise des fichiers édités après leur passage.
    Resume,
    /// Tout est posé.
    Done,
}

fn tout() -> Disponibilites {
    Disponibilites::TOUT
}

impl Default for SourceSyncOptions {
    fn default() -> Self {
        Self {
            batch_files: 64,
            plan_only: false,
            takeover: false,
            allow_empty: false,
            force: false,
            exige: Disponibilites::TOUT,
            relations: None,
            generated: Default::default(),
        }
    }
}

/// L'avancement d'une synchronisation, rendu après chaque paquet.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceSyncProgress {
    pub files_done: usize,
    pub files_total: usize,
    pub scopes_written: usize,
    /// Les liens en file, pas encore posés (`Bulk`).
    pub relations_pending: usize,
    /// Les fichiers édités après leur passage, que la fin reprendra.
    pub files_to_resume: usize,
    pub files_resumed: usize,
    pub phase: SyncPhase,
}

/// Ce qu'une synchronisation de source a fait.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceSyncReport {
    pub source: String,
    /// Fichiers listés par la source, et ceux retenus pour l'analyse.
    pub files_listed: usize,
    pub files_ingested: usize,
    pub scopes_written: usize,
    pub relations: usize,
    pub failed: usize,
    /// Le mode qui a servi, et la durée du chargement final en `Bulk`.
    pub relations_mode: Option<RelationsMode>,
    pub relations_bulk_ms: u128,
    /// La fin, par entité : plan seul si `plan_only`, appliquée sinon.
    pub scopes: SnapshotFinish,
    pub files: SnapshotFinish,
    /// Les fichiers édités pendant l'indexation, après leur passage, et
    /// repris à la fin.
    pub files_resumed: Vec<String>,
    /// **Les chargements en masse refusés** pendant cette synchronisation,
    /// repris ligne à ligne, et leur cause. Vide d'ordinaire : un repli est
    /// juste mais lent, et il ne doit plus passer sans un mot.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bulk_load_refused: Vec<String>,
    /// **Le tampon du moteur** contre lequel cette synchronisation a tourné,
    /// et sa source (variable, manifeste, règle, défaut du moteur). Un échec
    /// de mémoire dit ainsi contre quoi il a été obtenu.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub buffer_pool: Option<String>,
    /// **Les fichiers écartés par une règle déclarée**, par raison — les
    /// générés (`SourceSyncOptions::generated`). Ils ne sont pas analysés.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub files_set_aside: BTreeMap<String, usize>,
}

/// **Synchroniser une source entière** : voir le module. Les sessions sont
/// abandonnées si quoi que ce soit échoue avant la fin — rien n'est retiré
/// d'une synchronisation incomplète.
pub fn sync_source(
    catalog: &mut Catalog,
    source: &dyn FileSource,
    options: &SourceSyncOptions,
    progress: &mut dyn FnMut(SourceSyncProgress),
) -> Result<SourceSyncReport, String> {
    let cursor = source.cursor();
    let source_id = crate::code::source_id(&cursor);
    let mut inscription = Inscription::new(&source_id);
    // Les replis comptés sont ceux de cette synchronisation.
    let _ = catalog.take_bulk_load_refusals();
    let (mut report, mut avancement) = synchroniser_la_source(catalog, source, options, progress, &source_id)?;
    // **La reprise**, à la toute fin, sessions closes : chaque fichier édité
    // après son passage est ré-ingéré sur son grain, ou retiré s'il n'existe
    // plus. Une édition qui arrive pendant la reprise s'ajoute à la file.
    while let Some(paths) = inscription.reprendre_ou_sortir() {
        avancement.phase = SyncPhase::Resume;
        avancement.files_to_resume = paths.len();
        progress(avancement);
        for path in paths {
            let r = match source.read(&path)? {
                Some(content) => reingest_file(catalog, source, &path, &content, options.exige)?,
                None => remove_file(catalog, source, &path, options.exige)?,
            };
            report.failed += r.failed;
            report.files_resumed.push(path);
            avancement.files_resumed += 1;
            avancement.files_to_resume -= 1;
            progress(avancement);
        }
    }
    report.bulk_load_refused = catalog.take_bulk_load_refusals();
    avancement.phase = SyncPhase::Done;
    progress(avancement);
    Ok(report)
}

/// L'inscription d'une synchronisation au registre, rendue quoi qu'il
/// arrive : la file d'un échec est abandonnée — une synchronisation qui
/// n'est pas allée au bout se relance, et relit le disque.
struct Inscription<'a> {
    source_id: &'a str,
    sortie: bool,
}

impl<'a> Inscription<'a> {
    fn new(source_id: &'a str) -> Self {
        inscrire(source_id);
        Self { source_id, sortie: false }
    }
    fn reprendre_ou_sortir(&mut self) -> Option<Vec<String>> {
        let paths = reprendre_ou_sortir(self.source_id);
        self.sortie = paths.is_none();
        paths
    }
}

impl Drop for Inscription<'_> {
    fn drop(&mut self) {
        if !self.sortie {
            let mut registre = EN_COURS.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(en_cours) = registre.get_mut(self.source_id) {
                en_cours.tenants -= 1;
                if en_cours.tenants == 0 {
                    registre.remove(self.source_id);
                }
            }
        }
    }
}

fn synchroniser_la_source(
    catalog: &mut Catalog,
    source: &dyn FileSource,
    options: &SourceSyncOptions,
    progress: &mut dyn FnMut(SourceSyncProgress),
    source_id: &str,
) -> Result<(SourceSyncReport, SourceSyncProgress), String> {
    let source_id = source_id.to_string();
    let grain = BTreeMap::from([("source".to_string(), CypherValue::String(source_id.clone()))]);
    let s_scopes = catalog.begin_snapshot(SCOPE, &grain, options.takeover).map_err(|e| e.to_string())?.session;
    let s_files = match catalog.begin_snapshot(FILE, &grain, options.takeover) {
        Ok(open) => open.session,
        Err(e) => {
            let _ = catalog.abort_snapshot(SCOPE, &grain, &s_scopes);
            return Err(e.to_string());
        }
    };
    let mode = match options.relations {
        Some(m) => m,
        None if source_deja_indexee(catalog, &source_id)? => RelationsMode::PerBatch,
        None => RelationsMode::Bulk,
    };
    // **La marque « relations en cours »**, posée dès le début du mode de
    // masse : un lecteur (l'avancement, la recherche, un autre processus)
    // sait que les relations de cette source ne sont pas encore là, au lieu
    // de lire un graphe vide. Sa valeur porte la session, pour qu'une marque
    // laissée par un processus tué se reconnaisse.
    let marque = format!("relations_pending:{}/{}:{source_id}", catalog.scope().org, catalog.scope().project);
    if mode == RelationsMode::Bulk {
        catalog.persist_meta_key(&marque, &format!("{s_scopes}|0")).map_err(|e| e.to_string())?;
    }
    let resultat = synchroniser(catalog, source, options, mode, &marque, progress, &grain, &s_scopes, &s_files, source_id);
    if mode == RelationsMode::Bulk {
        let _ = catalog.persist_meta_key(&marque, "");
    }
    if resultat.is_err() || options.plan_only {
        let _ = catalog.abort_snapshot(SCOPE, &grain, &s_scopes);
        let _ = catalog.abort_snapshot(FILE, &grain, &s_files);
    }
    resultat
}

#[allow(clippy::too_many_arguments)]
fn synchroniser(
    catalog: &mut Catalog,
    source: &dyn FileSource,
    options: &SourceSyncOptions,
    mode: RelationsMode,
    marque: &str,
    progress: &mut dyn FnMut(SourceSyncProgress),
    grain: &BTreeMap<String, CypherValue>,
    s_scopes: &str,
    s_files: &str,
    source_id: String,
) -> Result<(SourceSyncReport, SourceSyncProgress), String> {
    let cursor = source.cursor();
    let (root, virtual_source) = match cursor.strip_prefix("worktree:") {
        Some(root) => (root.to_string(), false),
        None => ("/".to_string(), true),
    };
    // `RAG3WEAVER_INGEST_PROFILE` : ce que la synchronisation fait autour de
    // l'ingestion — lister, lire, analyser, marquer, finir — n'était dans
    // aucune ligne de profil (180 s sur 643, mesuré le 3 octobre 2026). Des
    // temps cumulés, publiés à la fin ; aucun changement de comportement.
    let mut profil = SyncProfile::default();
    let t = std::time::Instant::now();
    let listed = source.list()?;
    profil.add("lister la source", t);
    // Le tri au nom seul, comme `analyze_source` : ne pas lire un gros
    // fichier binaire pour l'écarter ensuite.
    let retenus: Vec<String> = listed
        .iter()
        .filter(|p| !matches!(crate::code::verdict(p, 0), crate::code::Verdict::Ecarte(_)))
        .cloned()
        .collect();
    let mut report =
        SourceSyncReport {
            source: source_id,
            files_listed: listed.len(),
            relations_mode: Some(mode),
            buffer_pool: catalog.conn().buffer_pool().map(crate::connection::describe_buffer_pool),
            ..Default::default()
        };
    let mut noms_differes = std::collections::BTreeSet::new();
    let mut avancement = SourceSyncProgress { files_total: retenus.len(), ..Default::default() };
    let par_transaction = mode == RelationsMode::Bulk && transaction_par_paquet();
    if par_transaction {
        // Le schéma que le premier paquet créerait à la volée, posé avant :
        // dans la transaction, une annulation l'emporterait avec les lignes.
        catalog.prepare_schema_for_ingest().map_err(|e| e.to_string())?;
    }
    for (rang_du_paquet, paquet) in retenus.chunks(options.batch_files.max(1)).enumerate() {
        // Lus, avant de les lire : une édition d'un de ces fichiers, à partir
        // d'ici, est à reprendre.
        marquer_lus(&report.source, paquet);
        let t = std::time::Instant::now();
        let mut sources = Vec::with_capacity(paquet.len());
        for path in paquet {
            if let Some(content) = source.read(path)? {
                // Un fichier généré n'entre pas dans l'analyse, et ça se
                // compte : la fin de session retire ce qu'il avait laissé.
                if let Some(reason) = options.generated.reason(path, &content) {
                    *report.files_set_aside.entry(reason.to_string()).or_default() += 1;
                    continue;
                }
                sources.push((path.clone(), content));
            }
        }
        profil.add("lire les fichiers", t);
        let t = std::time::Instant::now();
        let mut analysis = crate::code::analyze_in_project(&root, sources, &cursor, Some(&retenus));
        profil.add("analyser (codeparsers)", t);
        for f in &mut analysis.files {
            f.cursor = cursor.clone();
            if virtual_source {
                f.absolute_path.clear();
            }
        }
        // **Le paquet dans une transaction** (prototype, derrière
        // `RAG3WEAVER_TX_PAR_PAQUET=1`, en mode Bulk) : chaque COPY pose son
        // propre point de reprise à la validation, sauf dans une transaction
        // explicite, où un seul suffit au COMMIT (session cœur C++). Voir
        // [`terminer`].
        let tx = par_transaction;
        let t_paquet = std::time::Instant::now();
        if tx {
            commencer(catalog)?;
        }
        let resultat: Result<(), String> = (|| {
        let t = std::time::Instant::now();
        let ingere = match mode {
            RelationsMode::PerBatch => catalog.ingest_code_jusqu_a(&analysis, options.exige),
            RelationsMode::Bulk => catalog.ingest_code_differe(&analysis, options.exige, &mut noms_differes),
        }
        .map_err(|e| e.to_string())?;
        profil.add("ingérer le paquet (détail : [ingest-profile])", t);
        let t = std::time::Instant::now();
        // Ce que le paquet porte, marqué de la session : vu, pas seulement
        // écrit pendant elle.
        let uuids_scopes = analysis
            .scopes
            .iter()
            .map(|s| catalog.entity_uuid(SCOPE, &BTreeMap::from([("key".to_string(), CypherValue::String(s.key.clone()))])))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        let uuids_files = analysis
            .files
            .iter()
            .map(|f| catalog.entity_uuid(FILE, &f.data()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        profil.add("calculer les identifiants du paquet", t);
        let t = std::time::Instant::now();
        catalog.mark_snapshot(SCOPE, grain, s_scopes, &uuids_scopes).map_err(|e| e.to_string())?;
        catalog.mark_snapshot(FILE, grain, s_files, &uuids_files).map_err(|e| e.to_string())?;
        profil.add("marquer la session (mark_snapshot)", t);
        report.files_ingested += analysis.files.len();
        report.scopes_written += ingere.scopes;
        report.failed += ingere.failed;
        // Les relations de l'analyse, dans les deux modes : posées (par
        // paquet) ou mises en file (en masse) — le même compte.
        report.relations += ingere.relations;
        tuer_dans_le_paquet(rang_du_paquet);
        if mode == RelationsMode::Bulk && catalog.pending_work().relations.len() > BULK_QUEUE_LIMIT {
            // La mémoire bornée : la file est posée par COPY sans attendre.
            let t = std::time::Instant::now();
            let pose = catalog.drain_jusqu_a(options.exige);
            profil.add("vider la file des liens en route", t);
            report.failed += pose.failed;
        }
        Ok(())
        })();
        if tx {
            terminer(catalog, resultat)?;
        } else {
            resultat?;
        }
        crate::ingest_profile::add("sync · paquet entier, de bout en bout", t_paquet);
        avancement.files_done += paquet.len();
        avancement.scopes_written = report.scopes_written;
        avancement.relations_pending = catalog.pending_work().relations.len();
        avancement.files_to_resume = nombre_a_reprendre(&report.source);
        if mode == RelationsMode::Bulk {
            let _ = catalog.persist_meta_key(marque, &format!("{s_scopes}|{}", avancement.relations_pending));
        }
        progress(avancement);
    }
    if mode == RelationsMode::Bulk {
        avancement.phase = SyncPhase::Relations;
        progress(avancement);
        let debut = std::time::Instant::now();
        let tx = par_transaction;
        if tx {
            commencer(catalog)?;
        }
        let fin = catalog.finir_les_relations_differees(&noms_differes, options.exige).map_err(|e| e.to_string());
        let fin = if tx {
            let ok = fin.as_ref().map(|_| ()).map_err(Clone::clone);
            terminer(catalog, ok)?;
            fin?
        } else {
            fin?
        };
        report.failed += fin.failed;
        report.relations_bulk_ms = debut.elapsed().as_millis();
        profil.add("charger les relations à la fin", debut);
        avancement.relations_pending = 0;
    }
    let garde = SnapshotFinishOptions { allow_empty: options.allow_empty, force: options.force };
    let t = std::time::Instant::now();
    let plan_scopes = catalog.plan_snapshot_finish(SCOPE, grain, s_scopes, garde).map_err(|e| e.to_string())?;
    let plan_files = catalog.plan_snapshot_finish(FILE, grain, s_files, garde).map_err(|e| e.to_string())?;
    profil.add("planifier la fin (ce qui a disparu)", t);
    if options.plan_only {
        report.scopes = plan_scopes;
        report.files = plan_files;
        profil.publish();
        return Ok((report, avancement));
    }
    let t = std::time::Instant::now();
    // Les scopes d'abord : un fichier supprimé emporte ses `DEFINED_IN`, ses
    // scopes sont déjà partis.
    let noms_retires = noms_des_scopes(catalog, &plan_scopes.removed)?;
    report.scopes = catalog.apply_snapshot_finish(plan_scopes).map_err(|e| e.to_string())?;
    report.files = catalog.apply_snapshot_finish(plan_files).map_err(|e| e.to_string())?;
    // Un nom dont un définisseur vient de partir a pu redevenir unique : ses
    // mentionneurs, qu'aucun paquet ne repasse, gagnent leur arête.
    let resolu = catalog.resoudre_les_symboles(&noms_retires, options.exige).map_err(|e| e.to_string())?;
    report.relations += resolu.linked_across_batches;
    profil.add("appliquer la fin et résoudre les symboles", t);
    profil.publish();
    Ok((report, avancement))
}

/// Les temps de la synchronisation hors ingestion, cumulés sur toute la
/// source et publiés une fois (`RAG3WEAVER_INGEST_PROFILE`).
#[derive(Default)]
struct SyncProfile {
    stages: Vec<(&'static str, std::time::Duration)>,
}

impl SyncProfile {
    fn add(&mut self, stage: &'static str, since: std::time::Instant) {
        let took = since.elapsed();
        match self.stages.iter_mut().find(|(s, _)| *s == stage) {
            Some((_, total)) => *total += took,
            None => self.stages.push((stage, took)),
        }
    }

    fn publish(&self) {
        if std::env::var_os("RAG3WEAVER_INGEST_PROFILE").is_none() {
            return;
        }
        for (stage, total) in &self.stages {
            eprintln!("[sync-profile] {:>7} ms  {stage}", total.as_millis());
        }
        // Le détail de l'ingestion, cumulé sur toute la source.
        crate::ingest_profile::publish();
    }
}

/// Les noms des scopes `uuids`, relus avant leur retrait.
fn noms_des_scopes(catalog: &Catalog, uuids: &[String]) -> Result<Vec<String>, String> {
    if uuids.is_empty() {
        return Ok(Vec::new());
    }
    let mut noms: Vec<String> = catalog
        .get_many(SCOPE, uuids)
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter_map(|r| r.get("name").and_then(|v| v.as_str()).map(str::to_string))
        .collect();
    noms.sort();
    noms.dedup();
    Ok(noms)
}

/// La source a-t-elle déjà des scopes en base ? Sinon, c'est une première
/// indexation : le chemin de masse.
fn source_deja_indexee(catalog: &Catalog, source_id: &str) -> Result<bool, String> {
    let res = catalog
        .execute_raw_with_params(
            "MATCH (s:Scope) WHERE s.source = $source RETURN s._uuid LIMIT 1",
            &[crate::connection::QueryParam::new("source", CypherValue::String(source_id.to_string()))],
        )
        .map_err(|e| e.to_string())?;
    Ok(!res.rows.is_empty())
}

/// Le prototype de la transaction par paquet est-il demandé ?
fn transaction_par_paquet() -> bool {
    std::env::var("RAG3WEAVER_TX_PAR_PAQUET").as_deref() == Ok("1")
}

/// **Ouvrir la transaction d'un paquet** : toutes les écritures du catalogue
/// passent par la même connexion du moteur, donc par elle. Le catalogue
/// n'émet plus de DDL tant qu'elle est ouverte.
fn commencer(catalog: &mut Catalog) -> Result<(), String> {
    catalog.conn().execute("BEGIN TRANSACTION").map(|_| ()).map_err(|e| format!("ouvrir la transaction du paquet : {e}"))?;
    catalog.set_in_transaction(true);
    Ok(())
}

/// **Valider le paquet, ou le défaire.** Sur un échec, `ROLLBACK`, puis le
/// catalogue est empoisonné comme après un point de reprise échoué
/// ([`Catalog::poison`]) : ses caches (identifiants de nœuds, modèles
/// enregistrés, index plein texte ouverts, file vidée) gardent des écritures
/// défaites, et seule une réouverture les remet d'accord avec la base. Un
/// COMMIT refusé vaut la même chose. La reprise de l'index refait le paquet.
fn terminer(catalog: &mut Catalog, resultat: Result<(), String>) -> Result<(), String> {
    catalog.set_in_transaction(false);
    let echec = match resultat {
        Ok(()) => {
            // Le COMMIT porte le point de reprise que les COPY du paquet ont
            // demandé : on ne sait pas séparer les deux d'ici (minuterie de
            // rag3db-eb).
            let t = std::time::Instant::now();
            let r = catalog.conn().execute("COMMIT");
            crate::ingest_profile::add("sync · COMMIT du paquet (et son point de reprise)", t);
            match r {
                Ok(_) => return Ok(()),
                Err(e) => format!("valider le paquet : {e}"),
            }
        }
        Err(cause) => {
            let defait = catalog.conn().execute("ROLLBACK").map_err(|e| e.to_string());
            format!(
                "{cause} — le paquet est défait ({})",
                match defait {
                    Ok(_) => "ROLLBACK".to_string(),
                    Err(e) => format!("ROLLBACK refusé : {e}"),
                }
            )
        }
    };
    catalog.poison(&echec);
    Err(format!("{echec} : le catalogue doit être rouvert, et l'index repris"))
}

/// **Crochet de test** (`RAG3WEAVER_TEST_KILL_IN_BATCH=<rang>`) : le processus
/// se tue par SIGKILL au milieu du paquet de ce rang, après son ingestion et
/// avant sa validation — une mort base ouverte, transaction en cours
/// (`tests/e2e_tx_par_paquet_arret.rs`).
#[doc(hidden)]
fn tuer_dans_le_paquet(rang: usize) {
    if std::env::var("RAG3WEAVER_TEST_KILL_IN_BATCH").ok().and_then(|v| v.parse::<usize>().ok()) == Some(rang) {
        eprintln!("[rag3weaver] crochet de test : SIGKILL au paquet {rang}, transaction ouverte");
        let _ = std::process::Command::new("kill").args(["-KILL", &std::process::id().to_string()]).status();
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }
}
