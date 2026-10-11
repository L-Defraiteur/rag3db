//! E2E : **une base par arbre** — l'identité relative le permet (11 octobre
//! 2026, lot B4 de G, décision (a)).
//!
//! La forme voulue (orchestration, priorité de Lucie : « le cœur doit avoir son
//! worktree, n'importe lequel, indexé et à jour quand il édite ») : une base
//! par arbre, née d'un instantané reflink de la base de l'arbre principal,
//! puis tenue par le seul delta de la branche. Elle ne tient que si l'identité
//! d'un fichier est **la même dans deux copies du dépôt** : relative à la
//! racine, en « / » (`workspace_path`). Avec l'identité absolue d'avant, la
//! copie ouverte sur un autre arbre ne reconnaissait aucun fichier.
//!
//! Ce témoin fait le delta à la main : `list_files` avec l'état compare chaque
//! fichier de l'arbre à l'index par son identité, puis `reingest_file` reprend
//! les seuls fichiers qui diffèrent. La réconciliation **automatique** (la
//! montre qui rapproche le disque et l'index à l'ouverture) n'est pas encore
//! sur master : elle viendra par le proto de codeparsers et les routes de
//! l'hôte (arbre principal), qui attendent ce lot pour l'épreuve réelle.
//!
//! Run with: ./run_e2e.sh --test e2e_base_par_arbre
#![cfg(all(feature = "rag3db-native", feature = "code"))]

use std::path::{Path, PathBuf};

use rag3weaver::code::{default_scope_chunking, register_code_schema, FILE};
use rag3weaver::code_sync::{reingest_file, sync_source, SourceSyncOptions};
use rag3weaver::code_tools::{list_files, FileSource, WorkingTree};
use rag3weaver::embedder::HashEmbedder;
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

fn extension_vector() -> String {
    let root = std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        PathBuf::from(&manifest).join("../..").canonicalize().unwrap().to_string_lossy().to_string()
    });
    format!("{root}/extension/vector/build/libvector.rag3db_extension")
}

/// Une base de code **sur disque** (une copie reflink n'a de sens que là).
fn ouvrir(base: &Path) -> Catalog {
    let conn = Rag3dbConnection::new(base).expect("base sur disque");
    use rag3weaver::connection::DbConnection;
    conn.execute(&format!("LOAD EXTENSION '{}'", extension_vector())).unwrap();
    let config = CatalogConfig { name: Some("base-par-arbre".into()), embedding_dim: 64, ..Default::default() };
    let mut catalog = Catalog::new(Box::new(conn), Box::new(HashEmbedder::new(64)), config);
    catalog.initialize().unwrap();
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
    catalog
}

fn ecrire(racine: &Path, fichiers: &[(&str, &str)]) {
    for (chemin, texte) in fichiers {
        let p = racine.join(chemin);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, texte).unwrap();
    }
}

/// Les identités rangées des fichiers, triées : `(source, path)`.
fn identites(catalog: &Catalog) -> Vec<(String, String)> {
    let rows = catalog.execute_raw(&format!("MATCH (f:{FILE}) RETURN f.source, f.path")).unwrap();
    let mut v: Vec<(String, String)> = rows
        .rows
        .iter()
        .map(|r| (r[0].as_str().unwrap_or("").to_string(), r[1].as_str().unwrap_or("").to_string()))
        .collect();
    v.sort();
    v
}

/// **Une base copiée par reflink, ouverte sur un autre arbre du même dépôt,
/// rend les mêmes identités ; le delta ne touche que les fichiers qui
/// diffèrent.**
#[test]
#[ignore]
fn une_base_copiee_et_ouverte_sur_un_autre_arbre_garde_ses_identites_et_ne_reprend_que_le_delta() {
    let tmp = tempfile::tempdir().unwrap();
    let (arbre_a, arbre_b) = (tmp.path().join("principal"), tmp.path().join("branche"));
    let (dossier_a, dossier_b) = (tmp.path().join("base-principal"), tmp.path().join("base-branche"));
    std::fs::create_dir_all(&dossier_a).unwrap();
    let fichiers = [
        ("src/a.rs", "pub fn alpha() -> u32 {\n    1\n}\n"),
        ("src/b.rs", "pub fn beta() -> u32 {\n    2\n}\n"),
        ("src/sous/c.rs", "pub fn gamma() -> u32 {\n    3\n}\n"),
    ];
    ecrire(&arbre_a, &fichiers);

    // 1. La base de l'arbre principal.
    let avant = {
        let mut catalog = ouvrir(&dossier_a.join("code.rag3db"));
        sync_source(&mut catalog, &WorkingTree::new(&arbre_a), &SourceSyncOptions::default(), &mut |_| {}).unwrap();
        identites(&catalog)
    };
    eprintln!("[principal] {avant:?}");
    assert_eq!(
        avant,
        vec![
            ("file".to_string(), "src/a.rs".to_string()),
            ("file".to_string(), "src/b.rs".to_string()),
            ("file".to_string(), "src/sous/c.rs".to_string()),
        ],
        "relatives à la racine, en « / »"
    );

    // 2. L'instantané : une copie reflink du dossier de la base (une copie
    // ordinaire là où le système de fichiers ne sait pas).
    let cp = std::process::Command::new("cp")
        .arg("-a")
        .arg("--reflink=auto")
        .arg(&dossier_a)
        .arg(&dossier_b)
        .status()
        .expect("cp");
    assert!(cp.success(), "copie de la base");

    // 3. L'arbre de la branche : le même dépôt ailleurs, un fichier changé,
    // un fichier ajouté.
    ecrire(&arbre_b, &fichiers);
    ecrire(
        &arbre_b,
        &[("src/b.rs", "pub fn beta() -> u32 {\n    20\n}\n"), ("src/d.rs", "pub fn delta() -> u32 {\n    4\n}\n")],
    );

    // 4. La copie, ouverte sur l'autre racine : les mêmes identités.
    let mut catalog = ouvrir(&dossier_b.join("code.rag3db"));
    assert_eq!(identites(&catalog), avant, "la copie reconnaît ses fichiers sous une autre racine");
    let branche = WorkingTree::new(&arbre_b);

    // 5. Le delta, par l'état de chaque fichier contre l'index.
    let etat = list_files(&branche, Some(&catalog), None, 0, true).unwrap();
    let de = |p: &str| etat.entries.iter().find(|e| e.path == p).unwrap_or_else(|| panic!("{p} absent : {etat:?}")).clone();
    eprintln!("[état] {:?}", etat.entries.iter().map(|e| (&e.path, e.indexed, e.stale)).collect::<Vec<_>>());
    assert_eq!((de("src/a.rs").indexed, de("src/a.rs").stale), (Some(true), Some(false)), "inchangé, reconnu");
    assert_eq!((de("src/sous/c.rs").indexed, de("src/sous/c.rs").stale), (Some(true), Some(false)));
    assert_eq!((de("src/b.rs").indexed, de("src/b.rs").stale), (Some(true), Some(true)), "changé : périmé");
    assert_eq!(de("src/d.rs").indexed, Some(false), "ajouté : inconnu");
    let delta: Vec<String> = etat
        .entries
        .iter()
        .filter(|e| e.indexed != Some(true) || e.stale == Some(true))
        .map(|e| e.path.clone())
        .collect();
    assert_eq!(delta, vec!["src/b.rs".to_string(), "src/d.rs".to_string()], "le delta, et lui seul");

    // 6. Le delta repris : aucune identité en double, l'ajout reconnu.
    for p in &delta {
        let texte = branche.read(p).unwrap().unwrap();
        reingest_file(&mut catalog, &branche, p, &texte, Default::default()).unwrap();
    }
    let apres = identites(&catalog);
    eprintln!("[branche] {apres:?}");
    assert_eq!(apres.len(), 4, "trois reconnus, un ajouté, aucun doublon : {apres:?}");
    assert!(apres.contains(&("file".to_string(), "src/d.rs".to_string())));
    let etat = list_files(&branche, Some(&catalog), None, 0, true).unwrap();
    assert!(etat.entries.iter().all(|e| e.indexed == Some(true) && e.stale == Some(false)), "{etat:?}");
}

/// **Une base d'avant la règle se dit, et ne reçoit pas d'identités
/// relatives à côté des absolues.** Simulée : un `File` absolu, sans le
/// marqueur `code_identity`.
#[test]
#[ignore]
fn une_base_d_avant_la_regle_refuse_la_synchro_en_le_disant() {
    let tmp = tempfile::tempdir().unwrap();
    let arbre = tmp.path().join("arbre");
    ecrire(&arbre, &[("a.rs", "pub fn a() {}\n")]);
    let mut catalog = ouvrir(&tmp.path().join("code.rag3db"));
    catalog
        .execute_raw(&format!("CREATE (:{FILE} {{_uuid: 'ancien', source: 'file', path: '/ancien/a.rs'}})"))
        .unwrap();
    assert!(rag3weaver::code_sync::avant_la_regle(&catalog).unwrap());
    let refus = sync_source(&mut catalog, &WorkingTree::new(&arbre), &SourceSyncOptions::default(), &mut |_| {})
        .expect_err("une base d'avant la règle refuse");
    assert!(refus.contains("avant la règle") && refus.contains("réindexer"), "{refus}");
}
