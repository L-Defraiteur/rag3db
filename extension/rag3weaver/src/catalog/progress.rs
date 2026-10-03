//! **Où en est l'index** — l'état d'avancement, lisible par un agent.
//!
//! Le compte de la dette existait, privé : il ne sortait que comme
//! avertissement quand une recherche vectorielle rendait zéro. Ici il devient
//! une lecture publique, par table, dans la forme d'une ligne de journal —
//! « plein texte prêt · vecteurs 8 200 sur 20 100 (40 %) » — que `wait`
//! attrape par son motif.
//!
//! C'est un **compte en base**, pas un événement : il vaut pour tous les
//! processus qui remboursent la dette, y compris celui qu'on n'est pas.
//!
//! Le débit d'embarquement mesuré vit ici aussi (`note_embedding_rate`) :
//! c'est lui qui change un reste en une durée, pour l'avancement comme pour
//! l'estimation (`crate::estimate`).

use serde::{Deserialize, Serialize};

use super::{Catalog, CatalogError};
use crate::connection::{CypherValue, QueryParam};
use crate::estimate::Rate;

/// Une table de morceaux et ce qu'il lui manque.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableProgress {
    pub table: String,
    /// Les morceaux écrits — tous cherchables par mots.
    pub chunks: usize,
    /// Ceux qui attendent leur vecteur dense, pour le modèle courant.
    pub dense_missing: usize,
    /// Ceux qui attendent leur vecteur creux ; `None` si la table n'en a pas.
    pub sparse_missing: Option<usize>,
}

/// L'avancement de l'index, toutes tables à vecteurs confondues.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexProgress {
    /// Le modèle dont on compte les vecteurs.
    pub model: String,
    pub tables: Vec<TableProgress>,
    /// Ce processus a encore des écritures en file : le plein texte de ces
    /// lignes-là n'est pas prêt.
    pub writes_pending: usize,
    /// Les relations qu'une synchronisation en masse n'a pas encore posées
    /// (`code_sync`, marque `relations_pending:…`). `None` : aucune
    /// synchronisation n'en retient — le graphe est celui des lignes.
    #[serde(default)]
    pub relations_pending: Option<usize>,
}

impl IndexProgress {
    pub fn chunks(&self) -> usize {
        self.tables.iter().map(|t| t.chunks).sum()
    }

    pub fn dense_missing(&self) -> usize {
        self.tables.iter().map(|t| t.dense_missing).sum()
    }

    pub fn sparse_missing(&self) -> usize {
        self.tables.iter().filter_map(|t| t.sparse_missing).sum()
    }

    /// Les vecteurs denses faits, en pourcentage. Un index vide est à jour.
    pub fn dense_percent(&self) -> u8 {
        match self.chunks() {
            0 => 100,
            n => (100 * (n - self.dense_missing().min(n)) / n) as u8,
        }
    }

    /// Tout ce qui est écrit est cherchable par tous les signaux.
    pub fn complete(&self) -> bool {
        self.writes_pending == 0 && self.relations_pending.is_none() && self.dense_missing() == 0 && self.sparse_missing() == 0
    }

    /// **La ligne qu'un agent lit**, et que `wait` attrape. `rate` : le débit
    /// connu de l'embarqueur, pour dire ce qu'il reste en temps.
    pub fn line(&self, rate: Option<Rate>, chars_per_chunk: usize) -> String {
        let text = match self.writes_pending {
            0 => "plein texte prêt".to_string(),
            n => format!("plein texte : {n} écritures en file"),
        };
        // Trois états, pas deux : les mots, puis les relations, puis les
        // vecteurs. Le segment n'apparaît que si des relations attendent.
        let text = match self.relations_pending {
            None => text,
            Some(n) => format!("{text} · relations : {n} liens à poser"),
        };
        let chunks = self.chunks();
        let missing = self.dense_missing();
        let mut line = if missing == 0 {
            format!("{text} · vecteurs prêts ({chunks} morceaux, {})", self.model)
        } else {
            format!("{text} · vecteurs {} sur {chunks} ({} %, {})", chunks - missing.min(chunks), self.dense_percent(), self.model)
        };
        if missing > 0 {
            if let Some(rate) = rate {
                let left = rate.duration_for((missing * chars_per_chunk) as u64).as_secs();
                line.push_str(&match left {
                    0..=89 => format!(" · reste environ {left} s"),
                    _ => format!(" · reste environ {} min", (left + 30) / 60),
                });
            }
        }
        match self.sparse_missing() {
            0 => {}
            n => line.push_str(&format!(" · creux : {n} à faire")),
        }
        line
    }
}

/// Où en est un niveau de l'index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    /// Rien n'a été indexé à ce niveau.
    Never,
    /// Il y a de quoi chercher, mais pas tout : une indexation est en cours,
    /// ou une dette reste à solder.
    Running,
    /// Tout ce qui est écrit est cherchable à ce niveau.
    Ready,
}

/// **L'état de l'index, lisible à chaque recherche.** Une seule lecture de
/// méta — pas un comptage : c'est ce qu'une recherche consulte pour choisir
/// son mode (grep avant, plein texte pendant, fusion après).
///
/// L'indexation l'écrit à chaque changement ([`Catalog::note_index_state`]).
/// **Limite** : une écriture qui ne passe pas par elle — l'édition d'un
/// fichier, un lot — ne le met pas à jour ; l'état « prêt » veut dire « prêt à
/// la dernière indexation ». Les comptes ([`Catalog::index_progress`]) disent
/// toujours vrai, au prix d'un comptage.
/// Un état « en cours » que plus personne ne rafraîchit — un processus tué —
/// est reconnu à son âge et recompté ([`IndexState::is_stale`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexState {
    pub text: Level,
    pub vectors: Level,
    /// Les relations : `running` tant qu'une synchronisation en masse en
    /// retient — le graphe n'est pas complet, « rien trouvé » en suivant un
    /// lien ne veut pas dire « rien ».
    #[serde(default = "ready")]
    pub relations: Level,
    /// Les vecteurs faits, en pourcentage.
    pub vectors_percent: u8,
    /// Ce qu'il reste de vecteurs, en secondes, quand un débit est connu.
    #[serde(default)]
    pub vectors_seconds_left: Option<u64>,
    /// Quand cet état a été écrit (millisecondes Unix).
    pub updated_ms: u64,
}

fn ready() -> Level {
    Level::Ready
}

/// Au-delà, un état « en cours » sans nouvelles n'est plus cru.
pub const STATE_STALE_MS: u64 = 10 * 60 * 1_000;

impl IndexState {
    /// L'état que disent les comptes.
    pub fn from_progress(progress: &IndexProgress, now_ms: u64) -> Self {
        let chunks = progress.chunks();
        let level = |missing: usize| match (chunks, missing) {
            (0, _) => Level::Never,
            (_, 0) => Level::Ready,
            (n, m) if m >= n => Level::Never,
            _ => Level::Running,
        };
        Self {
            text: match (chunks, progress.writes_pending) {
                (0, 0) => Level::Never,
                (_, 0) => Level::Ready,
                _ => Level::Running,
            },
            relations: match (chunks, progress.relations_pending) {
                (0, None) => Level::Never,
                (_, None) => Level::Ready,
                (_, Some(_)) => Level::Running,
            },
            vectors: level(progress.dense_missing()),
            vectors_percent: if chunks == 0 { 0 } else { progress.dense_percent() },
            vectors_seconds_left: None,
            updated_ms: now_ms,
        }
    }

    /// Le reste des vecteurs en temps, quand on connaît le débit.
    pub fn with_time_left(mut self, progress: &IndexProgress, rate: Option<Rate>, chars_per_chunk: usize) -> Self {
        self.vectors_seconds_left = match (progress.dense_missing(), rate) {
            (0, _) | (_, None) => None,
            (missing, Some(rate)) => Some(rate.duration_for((missing * chars_per_chunk) as u64).as_secs()),
        };
        self
    }

    pub fn running(&self) -> bool {
        self.text == Level::Running || self.relations == Level::Running || self.vectors == Level::Running
    }

    /// Un état « en cours » trop vieux pour être cru.
    pub fn is_stale(&self, now_ms: u64) -> bool {
        self.running() && now_ms.saturating_sub(self.updated_ms) > STATE_STALE_MS
    }
}

const INDEX_STATE_KEY: &str = "index_state";

/// La clé de méta de l'état d'une entité.
fn entity_state_key(entity: &str) -> String {
    format!("{INDEX_STATE_KEY}:{entity}")
}

/// Le débit noté pour un modèle et une origine (`local`, ou l'adresse d'un
/// service).
fn rate_key(model: &str, origin: &str) -> String {
    format!("embedding_rate:{model}:{origin}")
}

impl Catalog {
    /// **Où en est l'index**, compté en base, toutes entités confondues.
    /// Lecture seule.
    pub fn index_progress(&self) -> Result<IndexProgress, CatalogError> {
        self.check_initialized()?;
        self.progress_of(self.vector_tables())
    }

    /// **Où en est l'index de cette entité** — ses morceaux à elle, pas ceux
    /// des autres. Une entité sans vecteurs (plein texte seul) est comptée sur
    /// ses lignes : cherchable par mots dès qu'elle en a, jamais par vecteurs.
    pub fn index_progress_for(&self, entity: &str) -> Result<IndexProgress, CatalogError> {
        self.check_initialized()?;
        if !self.entity_configs.contains_key(entity) {
            return Err(CatalogError::UnknownEntity(entity.to_string()));
        }
        let chunks = format!("{entity}_Chunk");
        if self.vector_tables().contains(&chunks) {
            return self.progress_of(vec![chunks]);
        }
        let rows = self.count_rows_of(entity);
        Ok(IndexProgress {
            model: self.current_embedding_entry().name.clone(),
            tables: vec![TableProgress { table: entity.to_string(), chunks: rows, dense_missing: rows, sparse_missing: None }],
            writes_pending: self.pending.total_count(),
            relations_pending: self.relations_pending()?,
        })
    }

    fn count_rows_of(&self, table: &str) -> usize {
        self.conn
            .execute(&self.dialect.count_rows(table))
            .ok()
            .and_then(|r| r.rows.first().and_then(|l| l.first()).and_then(|v| v.as_i64()))
            .unwrap_or(0) as usize
    }

    fn progress_of(&self, vector_tables: Vec<String>) -> Result<IndexProgress, CatalogError> {
        let mut tables = Vec::new();
        for table in vector_tables {
            let chunks = self.count_rows_of(&table);
            let entity = table.strip_suffix("_Chunk").unwrap_or(&table);
            let sparse = self.entity_configs.get(entity).is_some_and(|c| c.signals.sparse());
            tables.push(TableProgress {
                // Un modèle que cet index ne porte pas encore — rien n'a été
                // écrit sous lui — doit tout : chaque morceau attend son vecteur.
                dense_missing: match self.vector_storage(&table) {
                    Ok(storage) => self.count_marqueur_manquant(&table, &storage.marker),
                    Err(CatalogError::EmbeddingModelUnavailable(_)) => chunks,
                    Err(e) => return Err(e),
                },
                sparse_missing: sparse.then(|| self.count_marqueur_manquant(&table, "_sparse_hash")),
                chunks,
                table,
            });
        }
        Ok(IndexProgress {
            model: self.current_embedding_entry().name.clone(),
            tables,
            writes_pending: self.pending.total_count(),
            relations_pending: self.relations_pending()?,
        })
    }

    /// Les liens qu'une synchronisation en masse retient encore, toutes
    /// sources confondues. La marque vaut `{session}|{liens}` ; vide, la
    /// synchronisation est finie (ou a échoué, et l'a effacée).
    fn relations_pending(&self) -> Result<Option<usize>, CatalogError> {
        const PREFIX: &str = "relations_pending:";
        let stmt = self.dialect.load_meta_by_prefix("prefix");
        let result = self
            .conn
            .execute_with_params(&stmt, &[QueryParam::new("prefix", CypherValue::String(PREFIX.into()))])
            .map_err(|e| CatalogError::DbError(e.to_string()))?;
        let mut pending = None;
        for row in &result.rows {
            let (Some(CypherValue::String(k)), Some(CypherValue::String(v))) = (row.get(0), row.get(1)) else { continue };
            if !k.starts_with(PREFIX) || v.trim().is_empty() {
                continue;
            }
            let links = v.rsplit('|').next().and_then(|n| n.trim().parse::<usize>().ok()).unwrap_or(0);
            pending = Some(pending.unwrap_or(0) + links);
        }
        Ok(pending)
    }

    /// **L'état de l'index, à bas coût**, toutes entités confondues : l'état
    /// noté par l'indexation, en une lecture de méta. Sans état noté — un
    /// index rempli par un autre chemin — ou devant un état « en cours »
    /// périmé, on compte ([`Self::index_progress`]) et on note le résultat,
    /// pour que l'appel suivant ne recompte pas.
    ///
    /// **Il ne répond pas à « cette entité est-elle indexée ? ».** Tout ce que
    /// la base contient y compte — le journal d'une conversation, écrit dès le
    /// premier message, le fait quitter `never` avant qu'aucun code ne soit
    /// indexé (trouvé le 3 octobre 2026 dans le chat réel). Pour une entité,
    /// [`Self::index_state_for`].
    pub fn index_state(&self) -> Result<IndexState, CatalogError> {
        self.state_at(INDEX_STATE_KEY, || self.index_progress())
    }

    /// **L'état de l'index de cette entité**, au même prix : une lecture de
    /// méta, sa propre clé. C'est lui qu'une recherche ou un outil consulte
    /// pour sa cible.
    pub fn index_state_for(&self, entity: &str) -> Result<IndexState, CatalogError> {
        if !self.entity_configs.contains_key(entity) {
            return Err(CatalogError::UnknownEntity(entity.to_string()));
        }
        self.state_at(&entity_state_key(entity), || self.index_progress_for(entity))
    }

    fn state_at(&self, key: &str, count: impl FnOnce() -> Result<IndexProgress, CatalogError>) -> Result<IndexState, CatalogError> {
        let now = crate::dataflow::checkpoint::timestamp_ms();
        if let Some(noted) = self.read_meta_key(key)?.and_then(|v| serde_json::from_str::<IndexState>(&v).ok()) {
            // Un « jamais » noté n'est pas cru sur parole : la table a pu être
            // remplie depuis par un autre chemin que l'indexation, et rien ne
            // viendrait le corriger — un outil refuserait pour toujours un
            // index qui existe. Compter une table vide ne coûte rien.
            if !noted.is_stale(now) && noted.text != Level::Never {
                // Les relations se lisent à leur marque, que la
                // synchronisation tient elle-même à jour : une lecture de
                // méta de plus, toujours pas un comptage.
                let relations = match (noted.text, self.relations_pending()?) {
                    (Level::Never, None) => Level::Never,
                    (_, None) => Level::Ready,
                    (_, Some(_)) => Level::Running,
                };
                return Ok(IndexState { relations, ..noted });
            }
        }
        let counted = IndexState::from_progress(&count()?, now);
        self.note_state_at(key, counted)?;
        Ok(counted)
    }

    /// Note l'état global de l'index. Appelé par l'indexation à chaque changement.
    pub fn note_index_state(&self, state: IndexState) -> Result<(), CatalogError> {
        self.note_state_at(INDEX_STATE_KEY, state)
    }

    /// Note l'état de l'index d'une entité.
    pub fn note_index_state_for(&self, entity: &str, state: IndexState) -> Result<(), CatalogError> {
        self.note_state_at(&entity_state_key(entity), state)
    }

    fn note_state_at(&self, key: &str, state: IndexState) -> Result<(), CatalogError> {
        if self.lecture_seule {
            return Ok(());
        }
        let json = serde_json::to_string(&state).map_err(|e| CatalogError::DbError(e.to_string()))?;
        self.persist_meta_key(key, &json)
    }

    /// **Une indexation commence** sur ces entités : leur texte passe « en
    /// cours », sauf s'il était prêt — une réindexation ne fait pas régresser
    /// un niveau, l'ancien contenu reste cherchable. L'état global suit.
    pub fn note_indexing_started(&self, entities: &[&str]) -> Result<(), CatalogError> {
        let now = crate::dataflow::checkpoint::timestamp_ms();
        let started = |before: IndexState| IndexState { text: if before.text == Level::Ready { Level::Ready } else { Level::Running }, updated_ms: now, ..before };
        self.note_index_state(started(self.index_state()?))?;
        for entity in entities {
            self.note_index_state_for(entity, started(self.index_state_for(entity)?))?;
        }
        Ok(())
    }

    /// **Compte et note** l'état global et celui de chaque entité. `working` :
    /// une passe de vecteurs vient d'avancer — « jamais » devient « en cours ».
    /// Rend l'avancement global, pour la ligne de journal.
    pub fn refresh_index_states(&self, entities: &[&str], rate: Option<Rate>, chars_per_chunk: usize, working: bool) -> Result<IndexProgress, CatalogError> {
        let now = crate::dataflow::checkpoint::timestamp_ms();
        let state_of = |progress: &IndexProgress| {
            let mut state = IndexState::from_progress(progress, now).with_time_left(progress, rate, chars_per_chunk);
            if working && state.vectors == Level::Never && progress.chunks() > 0 && progress.dense_missing() < progress.chunks() {
                state.vectors = Level::Running;
            }
            state
        };
        for entity in entities {
            self.note_index_state_for(entity, state_of(&self.index_progress_for(entity)?))?;
        }
        let global = self.index_progress()?;
        self.note_index_state(state_of(&global))?;
        Ok(global)
    }

    /// L'embarqueur de ce catalogue vit-il ailleurs (un démon, un service) ?
    pub fn embedder_is_remote(&self) -> bool {
        self.embedder.distant()
    }

    /// D'où vient le calcul, pour ranger un débit : `service` ou `local`.
    fn embedder_origin(&self) -> &'static str {
        if self.embedder.distant() {
            "service"
        } else {
            "local"
        }
    }

    /// Le débit noté pour l'embarqueur de ce catalogue, s'il y en a un.
    pub fn known_embedding_rate(&self) -> Result<Option<Rate>, CatalogError> {
        self.embedding_rate(self.embedder.name(), self.embedder_origin())
    }

    /// Note le débit **constaté** de l'embarqueur de ce catalogue — ce qu'une
    /// indexation vient de mesurer en vrai, écritures comprises.
    pub fn note_measured_embedding_rate(&self, rate: Rate) -> Result<(), CatalogError> {
        self.note_embedding_rate(self.embedder.name(), self.embedder_origin(), rate)
    }

    /// **Sonde** l'embarqueur de ce catalogue sur `samples`, et note le débit.
    pub fn probe_embedding_rate(&self, samples: &[String]) -> Result<Option<Rate>, CatalogError> {
        let rate = crate::estimate::probe_rate(self.embedder.as_ref(), samples).map_err(CatalogError::DbError)?;
        if let Some(rate) = rate {
            self.note_embedding_rate(self.embedder.name(), self.embedder_origin(), rate)?;
        }
        Ok(rate)
    }

    /// Note le débit mesuré d'un embarqueur. Le dernier mesuré remplace le
    /// précédent : c'est le poste d'aujourd'hui qu'on veut prévoir.
    pub fn note_embedding_rate(&self, model: &str, origin: &str, rate: Rate) -> Result<(), CatalogError> {
        if self.lecture_seule {
            return Ok(());
        }
        self.persist_meta_key(&rate_key(model, origin), &format!("{:.0}", rate.chars_per_second))
    }

    /// Le débit noté, s'il y en a un.
    pub fn embedding_rate(&self, model: &str, origin: &str) -> Result<Option<Rate>, CatalogError> {
        Ok(self
            .read_meta_key(&rate_key(model, origin))?
            .and_then(|v| v.trim().parse::<f64>().ok())
            .filter(|v| *v > 0.0)
            .map(|chars_per_second| Rate { chars_per_second }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn progress(chunks: usize, dense_missing: usize, sparse_missing: Option<usize>, writes_pending: usize) -> IndexProgress {
        IndexProgress {
            model: "granite-278m".into(),
            tables: vec![TableProgress { table: "Scope_Chunk".into(), chunks, dense_missing, sparse_missing }],
            writes_pending,
            relations_pending: None,
        }
    }

    #[test]
    fn la_ligne_dit_ce_qui_est_pret_et_ce_qui_reste() {
        let p = progress(20_100, 11_900, None, 0);
        assert_eq!(p.dense_percent(), 40);
        assert!(!p.complete());
        assert_eq!(p.line(None, 500), "plein texte prêt · vecteurs 8200 sur 20100 (40 %, granite-278m)");
        // Avec un débit connu, le reste se dit en temps.
        let r = Rate { chars_per_second: 30_000.0 };
        assert_eq!(p.line(Some(r), 500), "plein texte prêt · vecteurs 8200 sur 20100 (40 %, granite-278m) · reste environ 3 min");
    }

    #[test]
    fn un_index_a_jour_le_dit_et_un_index_vide_aussi() {
        let fini = progress(20_100, 0, Some(0), 0);
        assert!(fini.complete());
        assert_eq!(fini.line(None, 500), "plein texte prêt · vecteurs prêts (20100 morceaux, granite-278m)");
        let vide = progress(0, 0, None, 0);
        assert_eq!(vide.dense_percent(), 100);
        assert!(vide.complete());
    }

    #[test]
    fn les_ecritures_en_file_et_le_creux_se_disent() {
        let p = progress(100, 100, Some(40), 12);
        assert!(!p.complete());
        let l = p.line(None, 500);
        assert!(l.starts_with("plein texte : 12 écritures en file · vecteurs 0 sur 100 (0 %"), "{l}");
        assert!(l.ends_with("· creux : 40 à faire"), "{l}");
    }

    #[test]
    fn plusieurs_tables_s_additionnent() {
        let mut p = progress(100, 50, None, 0);
        p.tables.push(TableProgress { table: "Note_Chunk".into(), chunks: 100, dense_missing: 0, sparse_missing: Some(10) });
        assert_eq!((p.chunks(), p.dense_missing(), p.sparse_missing(), p.dense_percent()), (200, 50, 10, 75));
    }

    #[test]
    fn l_etat_distingue_jamais_en_cours_et_pret_par_niveau() {
        let jamais = IndexState::from_progress(&progress(0, 0, None, 0), 1);
        assert_eq!((jamais.text, jamais.vectors, jamais.vectors_percent), (Level::Never, Level::Never, 0));
        // Les lignes sont là, aucun vecteur : cherchable par mots seulement.
        let mots = IndexState::from_progress(&progress(1_000, 1_000, None, 0), 1);
        assert_eq!((mots.text, mots.vectors, mots.vectors_percent), (Level::Ready, Level::Never, 0));
        let pendant = IndexState::from_progress(&progress(1_000, 600, None, 0), 1);
        assert_eq!((pendant.text, pendant.vectors, pendant.vectors_percent), (Level::Ready, Level::Running, 40));
        let pret = IndexState::from_progress(&progress(1_000, 0, None, 0), 1);
        assert_eq!((pret.text, pret.vectors, pret.vectors_percent), (Level::Ready, Level::Ready, 100));
        // Des écritures en file : le plein texte n'est pas complet.
        assert_eq!(IndexState::from_progress(&progress(1_000, 0, None, 5), 1).text, Level::Running);
    }

    #[test]
    fn un_etat_en_cours_sans_nouvelles_n_est_plus_cru() {
        let en_cours = IndexState { text: Level::Ready, relations: Level::Ready, vectors: Level::Running, vectors_percent: 40, vectors_seconds_left: None, updated_ms: 1_000 };
        assert!(!en_cours.is_stale(1_000 + STATE_STALE_MS));
        assert!(en_cours.is_stale(1_001 + STATE_STALE_MS), "un processus tué ne laisse pas « en cours » pour toujours");
        // Un état abouti ne périme pas : rien ne le rafraîchit, et c'est normal.
        let pret = IndexState { text: Level::Ready, relations: Level::Ready, vectors: Level::Ready, vectors_percent: 100, vectors_seconds_left: None, updated_ms: 1_000 };
        assert!(!pret.is_stale(u64::MAX));
        // Et il se lit tel qu'il s'écrit.
        let json = serde_json::to_string(&en_cours).unwrap();
        assert!(json.contains("\"vectors\":\"running\""), "{json}");
        assert_eq!(serde_json::from_str::<IndexState>(&json).unwrap(), en_cours);
    }

    /// **Trois états** : les mots, les relations, les vecteurs. Pendant un
    /// chargement en masse, la ligne et l'état disent que le graphe n'est
    /// pas encore complet.
    #[test]
    fn les_relations_en_attente_se_disent_dans_la_ligne_et_dans_l_etat() {
        let mut p = progress(20_100, 20_100, None, 0);
        p.relations_pending = Some(418_761);
        assert!(!p.complete());
        assert_eq!(p.line(None, 500), "plein texte prêt · relations : 418761 liens à poser · vecteurs 0 sur 20100 (0 %, granite-278m)");
        let state = IndexState::from_progress(&p, 1);
        assert_eq!((state.text, state.relations, state.vectors), (Level::Ready, Level::Running, Level::Never));
        assert!(state.running());
        // Une fois posées, le segment disparaît et le niveau est prêt.
        p.relations_pending = None;
        assert_eq!(IndexState::from_progress(&p, 1).relations, Level::Ready);
        assert!(!p.line(None, 500).contains("relations"));
        // Un état noté avant ce niveau se relit : les relations y sont prêtes.
        let ancien: IndexState = serde_json::from_str(r#"{"text":"ready","vectors":"running","vectors_percent":40,"updated_ms":5}"#).unwrap();
        assert_eq!((ancien.relations, ancien.vectors_seconds_left), (Level::Ready, None));
    }

    #[test]
    fn le_reste_des_vecteurs_se_dit_en_secondes_quand_le_debit_est_connu() {
        let p = progress(20_100, 11_900, None, 0);
        let rate = Rate { chars_per_second: 30_000.0 };
        assert_eq!(IndexState::from_progress(&p, 1).with_time_left(&p, Some(rate), 500).vectors_seconds_left, Some(198));
        assert_eq!(IndexState::from_progress(&p, 1).with_time_left(&p, None, 500).vectors_seconds_left, None);
        let fini = progress(20_100, 0, None, 0);
        assert_eq!(IndexState::from_progress(&fini, 1).with_time_left(&fini, Some(rate), 500).vectors_seconds_left, None);
    }
}
