//! **Un outil qui lit le catalogue ne rend jamais un vide sans dire d'où il
//! vient.**
//!
//! Trouvé par la passe « agent faible » (3 octobre 2026) : sur un index
//! jamais construit, `usages` rendait « rien trouvé », et l'agent en a conclu
//! que la fonction n'existait pas. Tout nœud qui lit le graphe passe donc par
//! [`read_catalog`], sur le modèle de la recherche adaptative
//! (`SearchSourceNode`) :
//! - le catalogue est **occupé** (une indexation tient le verrou) :
//!   `try_lock` échoue, et c'est une réponse, jamais une attente ni une
//!   erreur ;
//! - **jamais indexé** : un refus qui dit quoi faire ;
//! - indexation **en cours**, ou relations pas encore chargées : le résultat,
//!   précédé d'une ligne d'état — il est partiel ;
//! - **prêt** : le résultat seul ; « rien trouvé » y est une réponse.
//!
//! L'état se lit par [`Catalog::index_state_for`] **de l'entité lue** — le
//! pivot du gabarit —, jamais par l'état global : dans le chat réel, le
//! journal de la conversation s'écrit dans la même base dès le premier
//! message, l'état global quitte « jamais », et la porte ne refusait plus
//! (3 octobre 2026). Une lecture de méta, pas un comptage.

use std::sync::{Arc, Mutex, MutexGuard};

use crate::catalog::Level;
use crate::catalog::Catalog;

/// Ce que dit le catalogue avant qu'on le lise.
pub enum CatalogRead<'a> {
    /// On peut lire ; `status` est la ligne à mettre en tête quand le
    /// résultat est partiel.
    Ready { catalog: MutexGuard<'a, Catalog>, status: Option<String> },
    /// On lit par la vue de lecture, sans le mutex du catalogue
    /// (`crate::read_view`, lot L2).
    View { view: Arc<crate::read_view::ReadView>, status: Option<String> },
    /// On ne lit pas : la phrase à rendre à la place du résultat.
    Refused(String),
}

/// Combien une lecture attend un catalogue tenu avant de refuser : de quoi
/// laisser passer la réindexation d'un fichier, pas une indexation entière.
pub const READ_WAIT: std::time::Duration = std::time::Duration::from_millis(1000);

/// **Les lecteurs qui attendent un catalogue**, comptés **par catalogue** (la
/// clé est l'adresse de son `Arc`, que la lecture et la réaction partagent) :
/// une réaction qui enchaîne les fichiers (et reprend le verrou à chaque fois)
/// leur cède entre deux fichiers — le `Mutex` n'est pas équitable, et un
/// lecteur qui sonde perdrait sinon à chaque reprise. Par catalogue, pour qu'un
/// lecteur d'un backend ne ralentisse pas la réaction d'un autre.
static READERS_WAITING: std::sync::LazyLock<Mutex<std::collections::HashMap<usize, usize>>> =
    std::sync::LazyLock::new(Default::default);

fn catalog_key(catalog: &Arc<Mutex<Catalog>>) -> usize {
    Arc::as_ptr(catalog) as usize
}

/// Des lecteurs attendent-ils ce catalogue ?
pub fn readers_waiting(catalog: &Arc<Mutex<Catalog>>) -> bool {
    READERS_WAITING
        .lock() // L4 : pas le catalogue (les lecteurs en attente)
        .map(|m| m.get(&catalog_key(catalog)).copied().unwrap_or(0) > 0)
        .unwrap_or(false)
}

fn count_reader(key: usize, delta: isize) {
    if let Ok(mut m) = READERS_WAITING.lock() { // L4 : pas le catalogue (les lecteurs en attente)
        let n = m.entry(key).or_insert(0);
        *n = (*n as isize + delta).max(0) as usize;
        if *n == 0 {
            m.remove(&key);
        }
    }
}

/// Le catalogue est tenu par une indexation.
pub const BUSY: &str = "Catalogue occupé : une indexation le tient en ce moment. Réessayez dans un moment, ou prenez `search_code`, qui balaie les fichiers.";
/// Rien n'a jamais été indexé.
pub const NEVER_INDEXED: &str = "Aucun index : rien n'a encore été indexé, donc un nom introuvable ici ne veut rien dire. Lancez `index` d'abord, ou prenez `search_code`, qui balaie les fichiers.";

/// La ligne d'état d'un index partiel, `None` quand il est prêt.
pub fn partial_status(text: Level, relations: Level) -> Option<String> {
    match (text, relations) {
        (Level::Ready, Level::Ready) => None,
        (Level::Running, Level::Running) => Some("Index en cours, relations en cours de chargement : résultat partiel.".into()),
        (Level::Running, _) => Some("Index en cours : résultat partiel.".into()),
        (_, Level::Running) => Some("Relations en cours de chargement : résultat partiel.".into()),
        _ => None,
    }
}

/// **Prendre le catalogue pour lire**, en attendant un verrou court : la
/// réindexation d'un fichier le tient quelques dizaines de millisecondes. Le
/// lecteur se compte comme en attente le temps de la sonde, et la réaction lui
/// cède. `None` au-delà de [`READ_WAIT`] (une indexation entière) ou sur un
/// verrou empoisonné.
pub fn lock_for_read(catalog: &Arc<Mutex<Catalog>>) -> Option<std::sync::MutexGuard<'_, Catalog>> {
    let deadline = std::time::Instant::now() + READ_WAIT;
    // Compté comme lecteur en attente le temps de la sonde (rendu à la sortie).
    struct Attente(bool, usize);
    impl Drop for Attente {
        fn drop(&mut self) {
            if self.0 {
                count_reader(self.1, -1);
            }
        }
    }
    let mut attente = Attente(false, catalog_key(catalog));
    let cat = loop {
        match catalog.try_lock() {
            Ok(cat) => break cat,
            Err(std::sync::TryLockError::WouldBlock) if std::time::Instant::now() < deadline => {
                if !attente.0 {
                    count_reader(attente.1, 1);
                    attente.0 = true;
                }
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
            Err(_) => return None,
        }
    };
    drop(attente);
    Some(cat)
}

/// Prendre le catalogue pour lire `entity`, ou dire pourquoi pas. La
/// cellule du catalogue : voir [`read_catalog_in`].
pub fn read_catalog<'a>(catalog: &'a Arc<Mutex<Catalog>>, entity: &str) -> CatalogRead<'a> {
    read_catalog_in(catalog, entity, None)
}

/// La même, pour la cellule de l'appel (le service `scope`, que le backend
/// pose par requête ; `None` : celle du catalogue). **Le retard de l'index
/// sur le disque** de cette cellule s'ajoute à la ligne d'état : des
/// fichiers changés, pas encore réindexés, et le résultat peut dater.
pub fn read_catalog_in<'a>(
    catalog: &'a Arc<Mutex<Catalog>>,
    entity: &str,
    cell: Option<&crate::scope::Scope>,
) -> CatalogRead<'a> {
    // **Un verrou tenu brièvement s'attend** : la réindexation d'un fichier
    // (la réaction de la montre) le tient le temps d'une ingestion, quelques
    // dizaines de millisecondes. Seule une tenue longue — une indexation
    // entière — rend le refus ; la réponse, sinon, porte le retard dit.
    let Some(cat) = lock_for_read(catalog) else {
        return CatalogRead::Refused(BUSY.into());
    };
    match statut(&cat, entity, cell) {
        Ok(status) => CatalogRead::Ready { catalog: cat, status },
        Err(refus) => CatalogRead::Refused(refus),
    }
}

/// **La ligne d'état d'une lecture**, sur un catalogue déjà tenu : l'index
/// partiel, le retard sur le disque, l'arbre du dernier index, les morceaux
/// sans texte. `Err` : la phrase du refus (jamais indexé).
fn statut(cat: &Catalog, entity: &str, cell: Option<&crate::scope::Scope>) -> Result<Option<String>, String> {
    let status = match cat.index_state_for(entity) {
        Ok(state) if state.text == Level::Never => return Err(NEVER_INDEXED.into()),
        Ok(state) => partial_status(state.text, state.relations),
        // L'état ne se lit pas (entité inconnue du catalogue, méta
        // illisible) : on lit quand même, sans rien promettre.
        Err(_) => None,
    };
    let cell = cell.cloned().unwrap_or_else(|| cat.scope().clone());
    let lag = cat
        .file_lag()
        .summary_in(&cell)
        .map(|s| s.status_line(crate::catalog::retard::now_ms()));
    let status = match (status, lag) {
        (Some(partial), Some(lag)) => Some(format!("{partial} {lag}")),
        (partial, lag) => partial.or(lag),
    };
    // **Sur quel code on répond** : l'arbre et le commit du dernier index.
    let arbre = cat.indexed_tree_line(&cell, crate::catalog::retard::now_ms());
    // L'index est borgne sur ses morceaux sans texte : la réponse le dit.
    let vides = cat.chunks_sans_texte_notes();
    let arbre = match (arbre, vides) {
        (a, 0) => a,
        (Some(a), n) => Some(format!("{a} — {n} morceaux sans texte, ni cherchables ni embarqués (défaut connu)")),
        (None, n) => Some(format!("{n} morceaux sans texte, ni cherchables ni embarqués (défaut connu)")),
    };
    Ok(match (arbre, status) {
        (Some(a), Some(s)) => Some(format!("{a} — {s}")),
        (a, s) => a.or(s),
    })
}

/// **La fraîcheur d'une réponse** (la graine de l'hôte d'équipe) : sur quel commit et quelle
/// branche l'index a été bâti, s'il y avait des modifications non commitées, et combien de
/// fichiers la montre a vus changer sans qu'ils soient encore réindexés. `None` sans index noté.
/// Le champ `freshness` des outils de lecture (JSON), et leur ligne d'en-tête (markdown).
pub fn freshness_of(cat: &Catalog, cell: &crate::scope::Scope) -> Option<serde_json::Value> {
    let arbre = cat.indexed_tree(cell)?;
    let retard = cat.file_lag().summary_in(cell);
    Some(serde_json::json!({
        "root": arbre.get("root"),
        "commit": arbre.get("commit"),
        "branch": arbre.get("branch"),
        "dirty": arbre.get("dirty").and_then(|d| d.as_bool()).unwrap_or(false),
        "indexed_at_ms": arbre.get("at_ms"),
        "lag_files": retard.as_ref().map(|r| r.files).unwrap_or(0),
        "lag_since_ms": retard.as_ref().map(|r| r.since_ms),
    }))
}

/// La ligne de fraîcheur : l'arbre indexé, puis le retard s'il y en a un.
pub fn freshness_line(cat: &Catalog, cell: &crate::scope::Scope) -> Option<String> {
    let now = crate::catalog::retard::now_ms();
    let arbre = cat.indexed_tree_line(cell, now)?;
    Some(match cat.file_lag().summary_in(cell) {
        Some(r) => format!("{arbre} — {}", r.status_line(now)),
        None => format!("{arbre}, à jour"),
    })
}

/// Ce qu'un nœud de lecture rend : le refus, ou le résultat précédé de sa
/// ligne d'état. En JSON, `{"status": …, "result": …}`.
pub fn with_status(status: Option<&str>, result: serde_json::Value, json: bool) -> serde_json::Value {
    match (status, json) {
        (None, _) => result,
        (Some(s), false) => serde_json::Value::String(format!("> {s}\n\n{}", result.as_str().unwrap_or_default())),
        (Some(s), true) => serde_json::json!({ "status": s, "result": result }),
    }
}

/// Le refus, dans le format du nœud.
pub fn refusal(message: &str, json: bool) -> serde_json::Value {
    if json {
        serde_json::json!({ "status": message, "result": serde_json::Value::Null })
    } else {
        serde_json::Value::String(message.to_string())
    }
}

/// Ce qu'une lecture tient pour lire : le catalogue sous son mutex, ou la
/// vue de lecture.
pub enum ReadSource<'a> {
    Locked(MutexGuard<'a, Catalog>),
    View(Arc<crate::read_view::ReadView>),
}

impl ReadSource<'_> {
    pub fn get(&self) -> &dyn crate::read_view::ReadCatalog {
        match self {
            ReadSource::Locked(g) => &**g,
            ReadSource::View(v) => v.as_ref(),
        }
    }
}

impl<'a> CatalogRead<'a> {
    /// La source et la ligne d'état, ou la phrase du refus.
    pub fn into_reader(self) -> Result<(ReadSource<'a>, Option<String>), String> {
        match self {
            CatalogRead::Ready { catalog, status } => Ok((ReadSource::Locked(catalog), status)),
            CatalogRead::View { view, status } => Ok((ReadSource::View(view), status)),
            CatalogRead::Refused(m) => Err(m),
        }
    }
}

/// La ligne d'état d'une lecture par la vue quand un écrivain tient le
/// catalogue : la réponse est juste, mais d'avant ce qu'il écrit.
pub const WRITER_HOLDS: &str = "Une écriture de l'index est en cours : la réponse porte sur l'état validé, d'avant elle.";

/// **Lire pour un outil, sans attendre l'écrivain** (lot L2) : par la vue de
/// lecture quand elle est publiée et que l'appel vise la cellule du
/// catalogue. La ligne d'état se calcule si le catalogue est libre à
/// l'instant (un essai, jamais une attente) ; sinon elle dit qu'on lit l'état
/// validé. Sans vue, ou pour une autre cellule : [`read_catalog_in`], comme
/// avant.
pub fn read_for_tool<'a>(
    catalog: &'a Arc<Mutex<Catalog>>,
    view: Option<&Arc<crate::read_view::ReadViewCell>>,
    entity: &str,
    cell: Option<&crate::scope::Scope>,
) -> CatalogRead<'a> {
    let vue = view.and_then(|c| c.current());
    let Some(vue) = vue.filter(|v| cell.is_none_or(|c| *c == v.scope)) else {
        return read_catalog_in(catalog, entity, cell);
    };
    // Libre à l'instant : la ligne d'état complète, sous ce garde, rendu
    // aussitôt ; la lecture elle-même se fait par la vue.
    let essai = catalog.try_lock().ok().map(|cat| statut(&cat, entity, cell));
    // Un appel de lecture qui n'a pas eu d'instantané (pool plein, L6) le dit.
    let sans = |status: Option<String>| -> Option<String> {
        if !vue.sans_instantane {
            return status;
        }
        let ligne = crate::read_pool::SANS_INSTANTANE.to_string();
        Some(match status {
            Some(s) => format!("{s}\n{ligne}"),
            None => ligne,
        })
    };
    match essai {
        Some(Ok(status)) => CatalogRead::View { status: sans(status), view: vue },
        Some(Err(refus)) => CatalogRead::Refused(refus),
        None => CatalogRead::View { status: sans(Some(WRITER_HOLDS.to_string())), view: vue },
    }
}

