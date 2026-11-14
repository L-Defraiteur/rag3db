//! La maintenance du plein texte d'une table par l'hôte : le relevé de ses
//! documents vivants avec un jeton, puis la purge d'une liste et la compaction
//! (`lucivy_core::maintenance`), refusée si la table a changé depuis le relevé.
//!
//! **L'ordre que l'appelant doit tenir** (14 novembre 2026, avec hotfixs) :
//! 1. `POST /backend/fts-live-ids` : les vivants et le jeton ;
//! 2. la lecture du graphe, **après** le relevé : les orphelins sont les vivants
//!    qu'aucune ligne du graphe ne porte ;
//! 3. `POST /backend/maintain` avec ce jeton.
//!
//! Dans cet ordre, aucun document vivant n'est effacé :
//! - un document posé après le relevé change le jeton, et la purge est refusée ;
//! - une ligne posée avant le relevé est vue par le graphe lu après.
//!
//! Dans l'ordre inverse (le graphe lu d'abord), une ligne posée entre les deux
//! lectures aurait son document parmi les vivants et sous le jeton : c'est une
//! règle de l'appelant, que la route ne peut pas vérifier, puisque la lecture du
//! graphe ne passe pas par elle.
//!
//! **Le jeton** : pour chaque shard, les segments que voit son lecteur, avec
//! leur `max_doc` et leurs supprimés. Un commit qui ajoute, efface ou fusionne
//! le change. Il est pris sur le même lecteur que les vivants, et relu après
//! eux : un commit entre les deux recommence le relevé.

use std::sync::Arc;

use lucivy_core::sharded_handle::ShardedHandle;

type Handle = Arc<ShardedHandle>;

/// Le jeton d'un shard, sur un lecteur donné.
fn shard_token(searcher: &ld_lucivy::Searcher) -> String {
    let mut segments: Vec<String> = searcher.segment_readers().iter()
        .map(|s| format!("{}:{}:{}", s.segment_id().uuid_string(), s.max_doc(), s.num_deleted_docs()))
        .collect();
    segments.sort();
    segments.join(",")
}

/// Le jeton de la table : celui de chaque shard, sur son lecteur courant.
pub fn token(handle: &Handle) -> Result<String, String> {
    let mut parts = Vec::with_capacity(handle.num_shards());
    for i in 0..handle.num_shards() {
        let shard = handle.shard(i).ok_or_else(|| format!("shard {i} absent"))?;
        parts.push(shard_token(&shard.reader.searcher()));
    }
    Ok(parts.join("|"))
}

/// Les `_node_id` vivants de la table, triés, et le jeton du même lecteur.
/// Relu tant qu'un commit tombe entre la lecture des vivants et celle du jeton
/// (au plus cinq fois, puis un refus).
pub fn live_ids(handle: &Handle) -> Result<(Vec<u64>, String), String> {
    for _ in 0..5 {
        let before = token(handle)?;
        let mut ids = Vec::new();
        let mut parts = Vec::with_capacity(handle.num_shards());
        for i in 0..handle.num_shards() {
            let shard = handle.shard(i).ok_or_else(|| format!("shard {i} absent"))?;
            let searcher = shard.reader.searcher();
            for segment in searcher.segment_readers() {
                let column = segment.fast_fields().u64(lucivy_core::handle::NODE_ID_FIELD)
                    .map_err(|e| format!("colonne _node_id : {e}"))?;
                let alive = segment.alive_bitset();
                for doc in 0..segment.max_doc() {
                    if alive.is_some_and(|a| !a.is_alive(doc)) {
                        continue;
                    }
                    if let Some(id) = column.first(doc) {
                        ids.push(id);
                    }
                }
            }
            parts.push(shard_token(&searcher));
        }
        let read = parts.join("|");
        if read == before && token(handle)? == read {
            ids.sort_unstable();
            return Ok((ids, read));
        }
    }
    Err("la table change sans cesse pendant le relevé : réessayer dans une fenêtre calme".into())
}

/// Ce qu'on demande à `maintain`.
#[derive(Debug, Clone)]
pub struct Demande {
    pub purge: Vec<u64>,
    pub compact: bool,
    pub max_purge_ratio: Option<f64>,
    /// Le jeton rendu par `live_ids`.
    pub jeton: String,
}

/// La purge et la compaction, si le jeton est encore celui de la table.
/// L'appelant tient ce qui exclut les écrivains de la table (le slot du
/// backend en écriture, puis le catalogue) : entre la comparaison du jeton et
/// l'effacement, rien ne passe.
pub fn maintain(handle: &Handle, demande: &Demande) -> Result<serde_json::Value, String> {
    let live_docs = handle.num_docs();
    let courant = token(handle)?;
    if courant != demande.jeton {
        return Err(format!(
            "la table a changé depuis le relevé ({live_docs} documents vivants) : rien n'est effacé ; refaire fts-live-ids, \
             puis la lecture du graphe, puis maintain"
        ));
    }
    let mut request = lucivy_core::maintenance::MaintenanceRequest {
        purge_node_ids: demande.purge.clone(),
        compact: demande.compact,
        ..Default::default()
    };
    if let Some(ratio) = demande.max_purge_ratio {
        request.max_purge_ratio = ratio;
    }
    let report = handle.maintain(&request).map_err(|e| format!("{e} ({live_docs} documents vivants)"))?;
    let rapport = |r: &lucivy_core::index_report::IndexReport| {
        let (written, deleted) = r.documents();
        serde_json::json!({
            "index_bytes": r.index_bytes,
            "bytes_by_kind": r.bytes_by_kind,
            "documents": written - deleted,
            "deleted": deleted,
            "segments": r.segments.len(),
        })
    };
    Ok(serde_json::json!({
        "live_docs": live_docs,
        "requested": report.requested,
        "purged": report.purged,
        "absent": report.absent,
        "merge_rounds": report.merge_rounds,
        "dictionary_texts": [report.dictionary_texts.0, report.dictionary_texts.1],
        "purge_ms": report.purge_ms,
        "compact_ms": report.compact_ms,
        "before": rapport(&report.before),
        "after": rapport(&report.after),
        "summary": report.summary(),
    }))
}

/// Les documents vivants que dit un jeton (somme des `max_doc − supprimés`).
pub fn live_count_of(jeton: &str) -> u64 {
    jeton.split(['|', ','])
        .filter_map(|seg| {
            let mut parts = seg.rsplitn(3, ':');
            let deleted: u64 = parts.next()?.parse().ok()?;
            let max_doc: u64 = parts.next()?.parse().ok()?;
            Some(max_doc - deleted)
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index(dir: &std::path::Path) -> Handle {
        let config: lucivy_core::query::SchemaConfig = serde_json::from_value(serde_json::json!({
            "fields": [{ "name": "content", "type": "text", "stored": true }],
            "shards": 2,
            "sfx_version": 4
        })).unwrap();
        Arc::new(ShardedHandle::create(dir.to_str().unwrap(), &config).unwrap())
    }

    /// Le cas 3 d'hotfixs : un commit pendant le relevé. Les vivants et le jeton
    /// rendus viennent du même lecteur : autant d'ids que le jeton compte de
    /// documents, à chaque relevé, pendant qu'un autre fil ajoute et commite.
    #[test]
    fn le_releve_et_son_jeton_viennent_du_meme_lecteur() {
        let dir = tempfile::tempdir().unwrap();
        let handle = index(dir.path());
        for i in 0..50u64 {
            handle.add_document_json(i, &serde_json::json!({ "content": format!("doc {i}") })).unwrap();
        }
        handle.commit().unwrap();
        let ecrivain = {
            let handle = handle.clone();
            std::thread::spawn(move || {
                for i in 50..250u64 {
                    handle.add_document_json(i, &serde_json::json!({ "content": format!("doc {i}") })).unwrap();
                    if i % 5 == 0 { handle.commit().unwrap(); }
                }
                handle.commit().unwrap();
            })
        };
        let mut releves = 0;
        while !ecrivain.is_finished() {
            if let Ok((ids, jeton)) = live_ids(&handle) {
                assert_eq!(ids.len() as u64, live_count_of(&jeton), "[check: vivants-et-jeton-du-meme-lecteur] {jeton}");
                releves += 1;
            }
        }
        ecrivain.join().unwrap();
        let (ids, jeton) = live_ids(&handle).unwrap();
        assert_eq!(ids.len(), 250);
        assert_eq!(live_count_of(&jeton), 250);
        assert!(releves > 0, "au moins un relevé pendant les commits");
    }

    /// Le jeton change quand la table change, et pas autrement.
    #[test]
    fn le_jeton_suit_les_commits() {
        let dir = tempfile::tempdir().unwrap();
        let handle = index(dir.path());
        handle.add_document_json(1, &serde_json::json!({ "content": "un" })).unwrap();
        handle.commit().unwrap();
        let (_, jeton) = live_ids(&handle).unwrap();
        assert_eq!(token(&handle).unwrap(), jeton, "[check: stable-sans-commit]");
        handle.add_document_json(2, &serde_json::json!({ "content": "deux" })).unwrap();
        handle.commit().unwrap();
        assert_ne!(token(&handle).unwrap(), jeton, "[check: un-ajout-change-le-jeton]");
        let (_, jeton) = live_ids(&handle).unwrap();
        handle.delete_by_node_id(1).unwrap();
        handle.commit().unwrap();
        assert_ne!(token(&handle).unwrap(), jeton, "[check: une-suppression-change-le-jeton]");
        let refus = maintain(&handle, &Demande { purge: vec![2], compact: false, max_purge_ratio: Some(1.0), jeton }).unwrap_err();
        assert!(refus.contains("a changé"), "[check: maintain-refuse-un-jeton-perime] {refus}");
        assert_eq!(handle.num_docs(), 1, "[check: rien-n-est-efface]");
    }
}
