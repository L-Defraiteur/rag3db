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
        self.writes_pending == 0 && self.dense_missing() == 0 && self.sparse_missing() == 0
    }

    /// **La ligne qu'un agent lit**, et que `wait` attrape. `rate` : le débit
    /// connu de l'embarqueur, pour dire ce qu'il reste en temps.
    pub fn line(&self, rate: Option<Rate>, chars_per_chunk: usize) -> String {
        let text = match self.writes_pending {
            0 => "plein texte prêt".to_string(),
            n => format!("plein texte : {n} écritures en file"),
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

/// Le débit noté pour un modèle et une origine (`local`, ou l'adresse d'un
/// service).
fn rate_key(model: &str, origin: &str) -> String {
    format!("embedding_rate:{model}:{origin}")
}

impl Catalog {
    /// **Où en est l'index**, compté en base. Lecture seule.
    pub fn index_progress(&self) -> Result<IndexProgress, CatalogError> {
        self.check_initialized()?;
        let mut tables = Vec::new();
        for table in self.vector_tables() {
            let chunks = self
                .conn
                .execute(&self.dialect.count_rows(&table))
                .ok()
                .and_then(|r| r.rows.first().and_then(|l| l.first()).and_then(|v| v.as_i64()))
                .unwrap_or(0) as usize;
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
        Ok(IndexProgress { model: self.current_embedding_entry().name.clone(), tables, writes_pending: self.pending.total_count() })
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
}
