//! **Sonde : les arêtes d'usage d'un arbre, par marque, et chacune en TSV** (15 novembre 2026),
//! pour mesurer avant et après une règle de résolution (ticket « la marque nom relie un appel de
//! méthode standard à un homonyme »).
//!
//! `SONDE_DIRS=<d1>,<d2>` : chaque dossier est indexé seul, en mémoire (aucune base de service
//! touchée), avec la porte de l'index. Rend, par dossier, les CONSUMES par marque et le compte des
//! appels sur un receveur non typé (rendez-vous visibles, jamais reliés). `SONDE_TSV=<préfixe>`
//! écrit en plus `<préfixe>-<n>.tsv`, une ligne par CONSUMES : fichier et ligne du mentionneur, son
//! nom, la cible, son fichier, la marque — de quoi faire un `comm` entre deux pointeurs.
//!
//! Run with: SONDE_DIRS=… cargo test --features rag3db-native,code --test sonde_marque_nom -- --ignored --nocapture

#![cfg(all(feature = "rag3db-native", feature = "code"))]

use rag3weaver::code::{admission, analyze, default_scope_chunking, register_code_schema};
use rag3weaver::embedder::HashEmbedder;
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

fn setup() -> Catalog {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    let ext = rag3weaver::backend::vector_extension_of_this_process()
        .expect("l'extension vector de ce processus")
        .display()
        .to_string();
    boxed.execute(&format!("LOAD EXTENSION '{ext}'")).unwrap();
    let config = CatalogConfig {
        name: Some("sonde-marque-nom".into()),
        embedding_dim: 64,
        ..Default::default()
    };
    let mut catalog = Catalog::new(boxed, Box::new(HashEmbedder::new(64)), config);
    catalog.initialize().unwrap();
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
    catalog
}

fn fichiers(d: &std::path::Path, v: &mut Vec<std::path::PathBuf>) {
    let Ok(lu) = std::fs::read_dir(d) else { return };
    for e in lu.flatten() {
        let p = e.path();
        let nom = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if nom.starts_with('.') || nom == "target" || nom == "node_modules" || nom == "build" {
            continue;
        }
        if p.is_dir() {
            fichiers(&p, v)
        } else if let Some(x) = p.extension().and_then(|x| x.to_str()) {
            if [
                "rs", "c", "h", "cpp", "cc", "hpp", "py", "ts", "js", "go", "cs",
            ]
            .contains(&x)
            {
                v.push(p)
            }
        }
    }
}

fn texte(v: Option<&rag3weaver::connection::CypherValue>) -> String {
    v.map(|v| {
        v.as_str()
            .map(str::to_string)
            .unwrap_or_else(|| v.as_i64().map(|n| n.to_string()).unwrap_or_default())
    })
    .unwrap_or_default()
}

#[test]
#[ignore]
fn aretes_par_marque_en_tsv() {
    let dirs = std::env::var("SONDE_DIRS").expect("SONDE_DIRS");
    let tsv = std::env::var("SONDE_TSV").ok();
    // Profil strict optionnel : l'ancien appelant garde sa forme, le pilote frais l'exige.
    let probe_profile = std::env::var("RUST_PROBE_PROFILE").ok().map(|path| {
        serde_json::from_str::<serde_json::Value>(
            &std::fs::read_to_string(path).expect("probe profile readable"),
        )
        .expect("probe profile valid JSON")
    });
    if let Some(profile) = &probe_profile {
        assert_eq!(
            profile["roots"],
            serde_json::json!(dirs.split(',').collect::<Vec<_>>())
        );
    }
    for (n, racine) in dirs.split(',').enumerate() {
        let mut v = Vec::new();
        fichiers(std::path::Path::new(racine), &mut v);
        v.sort();
        let porte = rag3weaver::generated::GeneratedPolicy::default();
        let mut sources = Vec::new();
        let mut input_rejections = Vec::new();
        for path in &v {
            let relative = path
                .strip_prefix(racine)
                .expect("discovered path below root")
                .display()
                .to_string();
            let content = match std::fs::read_to_string(path) {
                Ok(content) => content,
                Err(error) => {
                    input_rejections.push((relative, format!("read_error: {error}")));
                    continue;
                }
            };
            if content.len() > 1_000_000 {
                input_rejections.push((relative, "size_limit".to_string()));
                continue;
            }
            if let Err(error) = admission(&relative, Some(&content), &porte) {
                input_rejections.push((relative, format!("admission: {error:?}")));
                continue;
            }
            sources.push((relative, content));
        }
        if let Some(prefix) = &tsv {
            std::fs::write(
                format!("{prefix}-{n}.admission.json"),
                serde_json::to_vec_pretty(&serde_json::json!({
                    "discovered":v.len(),"admitted":sources.len(),"rejected":input_rejections,
                    "admitted_paths":sources.iter().map(|(path,_)|path).collect::<Vec<_>>(),
                }))
                .unwrap(),
            )
            .unwrap();
        }
        let admis = sources.len();
        if let Some(profile) = &probe_profile {
            assert_eq!(
                v.len() as u64,
                profile["expected_counts"][n]
                    .as_u64()
                    .expect("expected file count")
            );
            assert_eq!(
                admis,
                v.len(),
                "no unreadable, oversized or unadmitted source may disappear"
            );
        }
        let analysis = analyze(racine, sources);
        if let Some(prefix) = &tsv {
            std::fs::write(
                format!("{prefix}-{n}.analysis.json"),
                serde_json::to_vec_pretty(&serde_json::json!({
                    "files":analysis.files.len(),"scopes":analysis.scopes.len(),
                    "skipped":analysis.skipped,"relations_dropped":analysis.relations_dropped,
                }))
                .unwrap(),
            )
            .unwrap();
        }
        if probe_profile.is_some() {
            assert!(
                analysis.skipped.is_empty(),
                "source analysis skipped files: {:?}",
                analysis.skipped
            );
            assert_eq!(
                analysis.files.len(),
                admis,
                "analysis must preserve every admitted file"
            );
        }
        let mut cat = setup();
        let observed_build_id = if let Some(profile) = &probe_profile {
            let result = cat
                .execute_raw("CALL BUILD_ID()")
                .expect("runtime BUILD_ID query");
            let actual = result
                .rows
                .first()
                .and_then(|row| row.first())
                .and_then(|value| value.as_str())
                .expect("BUILD_ID yields a string")
                .to_string();
            assert_eq!(
                Some(actual.as_str()),
                profile["expected_build_id"].as_str(),
                "actual native SDK"
            );
            Some(actual)
        } else {
            None
        };
        let report = cat.ingest_code(&analysis).unwrap();
        if probe_profile.is_some() {
            assert_eq!(report.failed, 0, "ingestion failures must not be hidden");
            assert_eq!(report.files, admis);
        }
        let r = cat
            .execute_raw("MATCH (:Scope)-[r:CONSUMES]->(:Scope) RETURN r.resolution, count(*)")
            .unwrap();
        let mut marques: Vec<(String, i64)> = r
            .rows
            .iter()
            .map(|x| {
                (
                    texte(x.first()),
                    x.get(1).and_then(|v| v.as_i64()).unwrap_or(0),
                )
            })
            .collect();
        marques.sort();
        let non_types = cat.execute_raw("MATCH (:Scope)-[m:MENTIONS]->(:Symbol) WHERE m.untyped_receiver = 'oui' RETURN count(*)").unwrap();
        let scopes = cat.execute_raw("MATCH (s:Scope) RETURN count(*)").unwrap();
        let orphans = cat.execute_raw("MATCH (s:Scope) WHERE NOT EXISTS { MATCH (s)-[:DEFINED_IN]->(:File) } RETURN count(*)").unwrap();
        let stored_files = cat.execute_raw("MATCH (f:File) RETURN count(*)").unwrap();
        if probe_profile.is_some() {
            assert_eq!(stored_files.rows[0][0].as_i64(), Some(admis as i64));
            assert_eq!(
                orphans.rows[0][0].as_i64(),
                Some(0),
                "every scope must retain its file"
            );
        }
        if let Some(prefix) = &tsv {
            std::fs::write(format!("{prefix}-{n}.meta.json"), serde_json::to_vec_pretty(&serde_json::json!({
                "root":racine,"discovered":v.len(),"admitted":admis,"analysis_files":analysis.files.len(),
                "analysis_skipped":analysis.skipped,"relations_dropped":analysis.relations_dropped,
                "ingest_report":report,"stored_files":stored_files.rows[0][0].as_i64(),
                "scope_count":scopes.rows[0][0].as_i64(),"orphan_scopes":orphans.rows[0][0].as_i64(),
                "resolution_counts":marques,"untyped_receiver_mentions":non_types.rows[0][0].as_i64(),
                "actual_build_id":observed_build_id,"strict_profile":probe_profile,
            })).unwrap()).unwrap();
        }
        println!(
            "SONDE {racine} : fichiers {} ; admis {admis} ; scopes {} (sans DEFINED_IN {}) ; CONSUMES {:?} ; rendez-vous non typés {}",
            v.len(),
            texte(scopes.rows[0].first()),
            texte(orphans.rows[0].first()),
            marques,
            texte(non_types.rows[0].first())
        );
        if let Some(prefixe) = &tsv {
            let r = cat
                .execute_raw("MATCH (a:Scope)-[r:CONSUMES]->(b:Scope) RETURN a.file_path, r.line, a.name, b.name, b.file_path, r.resolution, a.key, b.key, a._uuid, b._uuid, a.start_line, a.end_line, b.start_line, b.end_line, a.scope_type, b.scope_type, a.start_byte, a.end_byte, b.start_byte, b.end_byte")
                .unwrap();
            // Identité native et occurrences restent distinctes des six colonnes historiques.
            let mut occurrence_rows: Vec<serde_json::Value> = r
                .rows
                .iter()
                .map(|row| {
                    serde_json::json!({
                        "caller_path":texte(row.first()),"line":row.get(1).and_then(|v|v.as_i64()),
                        "caller_name":texte(row.get(2)),"target_name":texte(row.get(3)),
                        "target_path":texte(row.get(4)),"resolution":texte(row.get(5)),
                        "caller_key":texte(row.get(6)),"target_key":texte(row.get(7)),
                        "caller_uuid":texte(row.get(8)),"target_uuid":texte(row.get(9)),
                        "caller_start_line":row.get(10).and_then(|v|v.as_i64()),
                        "caller_end_line":row.get(11).and_then(|v|v.as_i64()),
                        "target_start_line":row.get(12).and_then(|v|v.as_i64()),
                        "target_end_line":row.get(13).and_then(|v|v.as_i64()),
                        "caller_type":texte(row.get(14)),"target_type":texte(row.get(15)),
                "caller_start_byte":row.get(16).and_then(|v|v.as_i64()),
                "caller_end_byte":row.get(17).and_then(|v|v.as_i64()),
                "target_start_byte":row.get(18).and_then(|v|v.as_i64()),
                "target_end_byte":row.get(19).and_then(|v|v.as_i64()),
                    })
                })
                .collect();
            occurrence_rows.sort_by_cached_key(|row| serde_json::to_string(row).unwrap());
            let jsonl = occurrence_rows
                .iter()
                .map(|row| serde_json::to_string(row).unwrap())
                .collect::<Vec<_>>()
                .join("\n")
                + "\n";
            std::fs::write(format!("{prefixe}-{n}.jsonl"), jsonl).unwrap();
            let edge_rows: Vec<Vec<String>> = r
                .rows
                .iter()
                .map(|row| (0..6).map(|i| texte(row.get(i))).collect())
                .collect();
            let mut lignes: Vec<String> = edge_rows.iter().map(|row| row.join("\t")).collect();
            lignes.sort();
            std::fs::write(format!("{prefixe}-{n}.tsv"), lignes.join("\n") + "\n").unwrap();
        }
    }
}
