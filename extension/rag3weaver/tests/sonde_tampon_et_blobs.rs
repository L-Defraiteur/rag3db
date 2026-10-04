//! Sonde — **un point de reprise qui ne tient plus dans le tampon du moteur**,
//! sans rag3weaver : la connexion seule et du Cypher brut.
//!
//! Le ticket `docs/tickets/2026-10-04-point-de-reprise-echoue-quand-le-tampon-est-petit.md` :
//! la première indexation du dépôt meurt sur « buffer pool is full … A
//! checkpoint of this database failed » sous 8 Gio de tampon, et les deux
//! morts observées suivent une écriture des blobs du plein texte. Cette sonde
//! refait ce que fait le magasin de blobs (`cypher_blob_store.rs`) et rien
//! d'autre : une table `(clé, BLOB)`, des lots `UNWIND … MERGE … SET` de
//! 32 Mo au plus, des générations de segments qui en remplacent d'autres, et
//! les octets des blobs supprimés vidés deux générations plus tard.
//!
//! Elle ne juge rien : elle dit à quelle génération le point de reprise
//! échoue, avec combien d'octets écrits, et la mémoire résidente au fil de
//! l'eau. Base sur disque, sous `~/.cache` ; le tampon et le reste se règlent
//! par l'environnement :
//!
//! - `RAG3DB_BUFFER_POOL_SIZE` : le tampon du moteur (octets) ;
//! - `SONDE_MODE` : `remplace` (défaut — les segments d'une génération
//!   remplacent ceux de la précédente, comme des fusions) ou `ajoute` (rien
//!   n'est jamais supprimé ni vidé) ;
//! - `SONDE_GO` : le total d'octets à écrire, en Go (défaut 8) ;
//! - `SONDE_SEGMENT_MO` : la taille d'un segment, en Mo (défaut 4).
//!
//! Run with: ./run_e2e.sh --test sonde_tampon_et_blobs
#![cfg(feature = "rag3db-native")]

use rag3weaver::connection::{CypherValue, DbConnection, QueryParam};
use rag3weaver::Rag3dbConnection;

fn env(name: &str, default: u64) -> u64 {
    std::env::var(name).ok().and_then(|v| v.trim().parse().ok()).unwrap_or(default)
}

/// Des octets incompressibles et reproductibles.
fn octets(graine: u64, taille: usize) -> Vec<u8> {
    let mut x = graine.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    (0..taille)
        .map(|_| {
            x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (x >> 33) as u8
        })
        .collect()
}

fn rss_mo() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| s.lines().find(|l| l.starts_with("VmRSS:")).and_then(|l| l.split_whitespace().nth(1).and_then(|v| v.parse::<u64>().ok())))
        .map(|kb| kb / 1024)
        .unwrap_or(0)
}

#[test]
#[ignore]
fn un_point_de_reprise_sous_des_blobs_qui_se_remplacent() {
    let mode = std::env::var("SONDE_MODE").unwrap_or_else(|_| "remplace".into());
    let total = env("SONDE_GO", 8) << 30;
    let segment = (env("SONDE_SEGMENT_MO", 4) << 20) as usize;
    const SEGMENTS_PAR_GENERATION: u64 = 16;
    const LOT: usize = 32 << 20;

    let dossier = std::path::PathBuf::from(std::env::var("HOME").unwrap()).join(".cache/rag3weaver-build/sonde-tampon");
    let _ = std::fs::remove_dir_all(&dossier);
    std::fs::create_dir_all(&dossier).unwrap();
    let conn = Rag3dbConnection::new(dossier.join("sonde.rag3db")).expect("base sur disque");
    eprintln!(
        "[sonde] tampon {:?}, mode {mode}, {} Go à écrire, segments de {} Mo, {SEGMENTS_PAR_GENERATION} par génération",
        std::env::var("RAG3DB_BUFFER_POOL_SIZE").ok(),
        total >> 30,
        segment >> 20
    );
    conn.execute("CREATE NODE TABLE Blobs (_key STRING, _data BLOB, _deleted_gen INT64, PRIMARY KEY(_key))").unwrap();

    let ecrire = "UNWIND $items AS item WITH item, item.key AS k MERGE (b:Blobs {_key: k}) SET b._data = item.data, b._deleted_gen = -1";
    let (mut ecrit, mut generation) = (0u64, 0i64);
    let debut = std::time::Instant::now();
    while ecrit < total {
        // Une génération de segments, en lots de 32 Mo au plus.
        let mut lot: Vec<CypherValue> = Vec::new();
        let mut octets_du_lot = 0usize;
        for s in 0..SEGMENTS_PAR_GENERATION {
            let cle = format!("seg-{generation}-{s}");
            let item = CypherValue::Map(std::collections::BTreeMap::from([
                ("key".to_string(), CypherValue::String(cle)),
                ("data".to_string(), CypherValue::Blob(octets(generation as u64 * 1000 + s, segment))),
            ]));
            if !lot.is_empty() && octets_du_lot + segment > LOT {
                if let Err(e) = conn.execute_with_params(ecrire, &[QueryParam::new("items", CypherValue::List(std::mem::take(&mut lot)))]) {
                    eprintln!("[sonde] ÉCHEC à la génération {generation}, après {} Mo écrits, {:.0} s, mémoire {} Mo : {e}", ecrit >> 20, debut.elapsed().as_secs_f64(), rss_mo());
                    return;
                }
                octets_du_lot = 0;
            }
            lot.push(item);
            octets_du_lot += segment;
            ecrit += segment as u64;
        }
        if let Err(e) = conn.execute_with_params(ecrire, &[QueryParam::new("items", CypherValue::List(lot))]) {
            eprintln!("[sonde] ÉCHEC à la génération {generation}, après {} Mo écrits, {:.0} s, mémoire {} Mo : {e}", ecrit >> 20, debut.elapsed().as_secs_f64(), rss_mo());
            return;
        }
        if mode == "remplace" && generation > 0 {
            // Les segments de la génération d'avant sont remplacés (une
            // fusion) ; ceux d'il y a deux générations rendent leurs octets.
            let suppression = conn.execute_with_params(
                "MATCH (b:Blobs) WHERE b._key STARTS WITH $prefixe SET b._deleted_gen = $gen",
                &[QueryParam::new("prefixe", CypherValue::String(format!("seg-{}-", generation - 1))), QueryParam::new("gen", CypherValue::Int(generation))],
            );
            let vidage = conn.execute_with_params(
                "MATCH (b:Blobs) WHERE b._deleted_gen >= 0 AND b._deleted_gen <= $limite AND octet_length(b._data) > 0 SET b._data = $vide",
                &[QueryParam::new("limite", CypherValue::Int(generation - 2)), QueryParam::new("vide", CypherValue::Blob(Vec::new()))],
            );
            if let Err(e) = suppression.and(vidage) {
                eprintln!("[sonde] ÉCHEC au remplacement de la génération {generation}, après {} Mo écrits, mémoire {} Mo : {e}", ecrit >> 20, rss_mo());
                return;
            }
        }
        if generation % 8 == 0 {
            eprintln!("[sonde] génération {generation} : {} Mo écrits, {:.0} s, mémoire {} Mo", ecrit >> 20, debut.elapsed().as_secs_f64(), rss_mo());
        }
        generation += 1;
    }
    eprintln!("[sonde] PASSÉ : {} Mo écrits en {generation} générations, {:.0} s, mémoire {} Mo", ecrit >> 20, debut.elapsed().as_secs_f64(), rss_mo());
}
