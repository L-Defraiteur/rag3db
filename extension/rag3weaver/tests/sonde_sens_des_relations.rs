//! **La sonde des deux sens voit-elle la perte de relations au point de
//! reprise ?** (4 octobre 2026). C'est la recette minimale de la session du
//! banc, en Cypher brut : deux régions de 1 024 nœuds, une relation sortante
//! dans chacune, un CHECKPOINT, la relation de la première région supprimée,
//! un second CHECKPOINT. Le moteur ne compte que les régions modifiées ;
//! trouvant zéro, il libère tout le groupe dans le sens direct, et la
//! relation de l'autre région disparaît de ce sens seulement.
//!
//! **Hors de la batterie** (le fichier ne commence pas par `e2e_`) : il est
//! rouge tant que le moteur n'est pas corrigé, et en deviendra le témoin. Il
//! affirme le comportement juste (les deux sens égaux), et imprime ce que la
//! sonde voit.
//!
//! Run with: ./run_e2e.sh --test sonde_sens_des_relations
#![cfg(feature = "rag3db-native")]

use rag3weaver::connection::DbConnection;
use rag3weaver::relation_directions::count_both_directions;
use rag3weaver::Rag3dbConnection;

#[test]
#[ignore]
fn la_recette_du_banc_garde_les_deux_sens_egaux() {
    let dir = std::path::PathBuf::from(std::env::var("HOME").unwrap())
        .join(format!(".cache/rag3weaver-build/sonde-sens-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let conn = Rag3dbConnection::new(&dir).expect("base sur disque");
    for q in [
        "CALL auto_checkpoint=false",
        "CREATE NODE TABLE person(id INT64 PRIMARY KEY)",
        "CREATE REL TABLE knows(FROM person TO person, MANY_MANY)",
        "UNWIND range(0,1024) AS id CREATE (:person {id:id})",
        "MATCH (a:person),(b:person) WHERE a.id IN [0,1024] AND b.id=1 CREATE (a)-[:knows]->(b)",
        "CHECKPOINT",
    ] {
        conn.execute(q).unwrap_or_else(|e| panic!("{q} : {e}"));
    }
    let avant = count_both_directions(&conn).unwrap();
    eprintln!("[SONDE avant la suppression] {avant:?}");
    for q in ["MATCH (a:person)-[r:knows]->(:person) WHERE a.id=0 DELETE r", "CHECKPOINT"] {
        conn.execute(q).unwrap_or_else(|e| panic!("{q} : {e}"));
    }
    let apres = count_both_directions(&conn).unwrap();
    eprintln!("[SONDE après le second point de reprise] {apres:?}");
    // L'étalon : la seule forme qui force le sens, un nœud lié par sa clé.
    let un = |q: &str| conn.execute(q).unwrap().rows[0][0].clone();
    eprintln!(
        "[SONDE étalon par clé] direct depuis 1024 : {:?} ; inverse vers 1 : {:?}",
        un("MATCH (a:person {id: 1024})-[r:knows]->() RETURN count(r)"),
        un("MATCH (b:person {id: 1})<-[r:knows]-() RETURN count(r)")
    );
    drop(conn);
    let rouverte = Rag3dbConnection::new(&dir).expect("réouverture");
    let rouvert = count_both_directions(&rouverte).unwrap();
    eprintln!("[SONDE après réouverture] {rouvert:?}");
    drop(rouverte);
    let _ = std::fs::remove_dir_all(&dir);
    assert!(avant.iter().all(|d| d.is_symmetric() && d.forward == 2), "deux relations dans chaque sens : {avant:?}");
    assert!(apres.iter().all(|d| d.is_symmetric() && d.forward == 1), "une relation reste, dans chaque sens : {apres:?}");
    assert!(rouvert.iter().all(|d| d.is_symmetric() && d.forward == 1), "et après réouverture : {rouvert:?}");
}

/// **Le même défaut par le chemin du produit.** Une source de 1 100 fichiers
/// (plus de 1 024 scopes : deux régions du groupe), dont seuls le premier
/// et le dernier importent une bibliothèque (`USES_LIBRARY`, une relation
/// clairsemée). Une resynchronisation retire le premier fichier (ses scopes
/// partent, et l'arête de la première région avec eux), puis un CHECKPOINT.
/// L'arête du dernier fichier, dans l'autre région, doit rester lisible dans
/// les deux sens, et l'index valoir un index bâti à neuf.
#[test]
#[ignore]
fn un_fichier_retire_ne_fait_pas_perdre_les_aretes_des_autres_regions() {
    use rag3weaver::code::{default_scope_chunking, register_code_schema};
    use rag3weaver::code_sync::{sync_source, SourceSyncOptions};
    use rag3weaver::code_tools::Snapshot;
    use rag3weaver::disponibilite::Disponibilites;
    use rag3weaver::embedder::HashEmbedder;
    use rag3weaver::{Catalog, CatalogConfig};

    let dir = std::path::PathBuf::from(std::env::var("HOME").unwrap())
        .join(format!(".cache/rag3weaver-build/sonde-produit-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let fichiers = |sans_le_premier: bool| -> Vec<(String, String)> {
        (0..1100)
            .filter(|i| !(sans_le_premier && *i == 0))
            .map(|i| {
                let import = if i == 0 || i == 1099 { "use serde::Serialize;\n\n" } else { "" };
                (format!("f{i:03}.rs"), format!("{import}pub fn f{i}() -> u32 {{ {i} }}\n"))
            })
            .collect()
    };
    let racine = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
    let conn = Rag3dbConnection::new(&dir).expect("base sur disque");
    conn.execute(&format!("LOAD EXTENSION '{}/extension/vector/build/libvector.rag3db_extension'", racine.display())).unwrap();
    let config = CatalogConfig { name: Some("sonde-produit".into()), embedding_dim: 16, ..Default::default() };
    let mut catalog = Catalog::new(Box::new(conn), Box::new(HashEmbedder::new(16)), config);
    catalog.initialize().unwrap();
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
    let options = SourceSyncOptions { batch_files: 64, exige: Disponibilites::RECHERCHE_TEXTE, force: true, ..Default::default() };
    sync_source(&mut catalog, &Snapshot::new("regions", fichiers(false)), &options, &mut |_| {}).unwrap();
    catalog.execute_raw("CHECKPOINT").unwrap();
    let decalages = catalog
        .execute_raw("MATCH (s:Scope)-[:USES_LIBRARY]->(:Library) RETURN s.file_path, offset(id(s))")
        .map(|r| format!("{:?}", r.rows))
        .unwrap_or_else(|e| e.to_string());
    eprintln!("[SONDE produit] décalages des scopes qui importent (il faut un < 1 024 et un ≥ 1 024) : {decalages}");
    let avant = count_both_directions(catalog.conn()).unwrap();
    let scopes = catalog.execute_raw("MATCH (s:Scope) RETURN count(s)").unwrap().rows[0][0].clone();
    eprintln!("[SONDE produit] {scopes:?} scopes (il en faut plus de 1 024 : deux régions)");
    // `MESURE_GESTE=edition` : le premier fichier perd son import par une
    // édition (`reingest_file` retire ses arêtes par `DELETE r`, l'ingestion
    // n'en repose aucune) ; sinon, une resynchronisation le retire (ses scopes
    // partent par `DETACH DELETE`).
    let source = Snapshot::new("regions", fichiers(false));
    if std::env::var("MESURE_GESTE").as_deref() == Ok("edition") {
        let r = rag3weaver::code_tools::edit_file(
            &source,
            Some(&mut catalog),
            "f000.rs",
            &rag3weaver::code_tools::EditOp::Replace { old: "use serde::Serialize;\n\n".into(), new: String::new() },
        )
        .unwrap();
        assert!(r.reingest.is_some(), "{r:?}");
    } else {
        let r = sync_source(&mut catalog, &Snapshot::new("regions", fichiers(true)), &options, &mut |_| {}).unwrap();
        assert_eq!(r.files.removed.len(), 1, "le premier fichier part : {r:?}");
    }
    catalog.execute_raw("CHECKPOINT").unwrap();
    let apres = count_both_directions(catalog.conn()).unwrap();
    let usages = |v: &[rag3weaver::relation_directions::DirectionCount]| {
        v.iter().filter(|d| d.table == "USES_LIBRARY").map(|d| (d.forward, d.backward)).collect::<Vec<_>>()
    };
    eprintln!("[SONDE produit] USES_LIBRARY avant {:?}, après {:?}", usages(&avant), usages(&apres));
    let fautes: Vec<_> = apres.iter().filter(|d| !d.is_symmetric()).collect();
    drop(catalog);
    let _ = std::fs::remove_dir_all(&dir);
    assert!(fautes.is_empty(), "les deux sens doivent rester égaux : {fautes:?}");
    assert_eq!(usages(&apres), vec![(1, 1)], "l'import du dernier fichier reste");
}
