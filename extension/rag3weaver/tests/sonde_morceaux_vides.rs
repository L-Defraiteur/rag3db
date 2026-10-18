//! Sonde : **les morceaux de code écrits sans texte** (11 et 12 octobre 2026).
//! Ticket : `docs/tickets/2026-10-11-des-morceaux-de-code-sont-ecrits-sans-texte.md`.
//!
//! Chaque test compte les `Scope_Chunk` dont `_text` est nul ou vide (et ceux
//! qui gardent un `_text_hash`) après une histoire donnée : un gros fichier
//! seul, un dossier entier, une base sur disque rouverte, une masse arrêtée
//! puis reprise, un fichier ré-ingéré, le rattrapage des vecteurs. Aucun ne
//! les reproduit en une histoire propre ; `compter_une_base` est l'outil qui
//! dira l'origine sur une copie de base atteinte (`SONDE_BASE`).
//!
//! Run with: cargo test --features rag3db-native,code --test sonde_morceaux_vides -- --ignored --nocapture <test>
//! Variables : `RAG3DB_ROOT` (l'arbre, l'extension vector), `SONDE_LUCIVY` (le corpus),
//! `SONDE_FICHIER`,
//! `SONDE_DOSSIERS` (séparés par `:`), `SONDE_BASES` (dossier effacé puis
//! recréé), `SONDE_BASE`.
#![cfg(all(feature = "rag3db-native", feature = "code"))]

use rag3weaver::code::{analyze_source, default_scope_chunking, register_code_schema};
use rag3weaver::code_sync::{sync_source, SourceSyncOptions};
use rag3weaver::code_tools::{edit_file, EditOp, FileSource, Snapshot, WorkingTree};
use rag3weaver::connection::DbConnection;
use rag3weaver::disponibilite::Disponibilites;
use rag3weaver::embedder::HashEmbedder;
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

/// **Le corpus : les sources de lucivy.** `SONDE_LUCIVY` (le dossier d'un arbre de lucivy), sinon
/// la crate publiée du registre cargo, à la version que prend rag3weaver (4.3.0). Le sous-module
/// `extension/lucivy/ld-lucivy` qui servait ici est retiré (18 octobre 2026).
fn lucivy(crate_publiee: &str) -> String {
    if let Ok(d) = std::env::var("SONDE_LUCIVY") {
        return if crate_publiee == "lucivy-core" {
            format!("{d}/lucivy_core")
        } else {
            d
        };
    }
    let maison = std::env::var("CARGO_HOME")
        .unwrap_or_else(|_| format!("{}/.cargo", std::env::var("HOME").unwrap()));
    let index = std::fs::read_dir(format!("{maison}/registry/src")).expect("le registre cargo");
    index
        .filter_map(|e| e.ok())
        .map(|e| e.path().join(format!("{crate_publiee}-4.3.0")))
        .find(|d| d.is_dir())
        .unwrap_or_else(|| {
            panic!("{crate_publiee} 4.3.0 absente du registre cargo : posez SONDE_LUCIVY")
        })
        .to_string_lossy()
        .to_string()
}

fn catalogue() -> Catalog {
    catalogue_sur(Rag3dbConnection::in_memory().expect("base en mémoire"))
}

fn catalogue_sur(conn: Rag3dbConnection) -> Catalog {
    conn.execute(&format!("LOAD EXTENSION '{}'", rag3weaver::backend::vector_extension_of_this_process().expect("l'extension vector de ce processus").display())).unwrap();
    let config = CatalogConfig { name: Some("sonde-vides".into()), embedding_dim: 64, ..Default::default() };
    let mut catalog = Catalog::new(Box::new(conn), Box::new(HashEmbedder::new(64)), config);
    catalog.initialize().unwrap();
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
    catalog
}

fn un(catalog: &Catalog, q: &str) -> i64 {
    catalog.execute_raw(q).unwrap().rows[0][0].as_i64().unwrap()
}

fn compter(catalog: &Catalog, chemin: &str) {
    let total = un(catalog, "MATCH (c:Scope_Chunk) RETURN count(c)");
    let nuls = un(catalog, "MATCH (c:Scope_Chunk) WHERE c._text IS NULL RETURN count(c)");
    let vides = un(catalog, "MATCH (c:Scope_Chunk) WHERE c._text = '' RETURN count(c)");
    let haches = un(
        catalog,
        "MATCH (c:Scope_Chunk) WHERE (c._text IS NULL OR c._text = '') AND c._text_hash IS NOT NULL AND c._text_hash <> '' RETURN count(c)",
    );
    println!("[{chemin}] Scope_Chunk {total}, _text nul {nuls}, _text vide {vides}, vides avec hash {haches}");
    let r = catalog
        .execute_raw(
            "MATCH (c:Scope_Chunk) WHERE c._text IS NULL OR c._text = '' \
             RETURN c._parent_uuid, c._parent_field, c._text_hash, c._uuid LIMIT 3",
        )
        .unwrap();
    for l in &r.rows {
        println!("  vide : {l:?}");
    }
    // Les pleins qui contiennent une barre oblique inverse, pour comparer.
    let barres = un(catalog, "MATCH (c:Scope_Chunk) WHERE c._text CONTAINS '\\\\' RETURN count(c)");
    println!("  pleins contenant une barre oblique inverse : {barres}");
    if nuls + vides > 0 {
        let r = catalog
            .execute_raw(
                "MATCH (c:Scope_Chunk), (s:Scope) WHERE (c._text IS NULL OR c._text = '') AND s._uuid = c._parent_uuid \
                 RETURN s.file_path, count(c) AS n ORDER BY n DESC LIMIT 20",
            )
            .unwrap();
        for l in &r.rows {
            println!("  par fichier : {l:?}");
        }
    }
}

/// Le même compte à l'échelle : `SONDE_DOSSIERS`, des dossiers séparés par
/// `:`, chacun synchronisé seul sur une base neuve (chemin de masse).
#[test]
#[ignore]
fn les_morceaux_vides_d_un_dossier() {
    let dossiers = std::env::var("SONDE_DOSSIERS").expect("SONDE_DOSSIERS");
    for dossier in dossiers.split(':') {
        let mut masse = catalogue();
        let options = SourceSyncOptions { exige: Disponibilites::RECHERCHE_TEXTE, ..Default::default() };
        let t0 = std::time::Instant::now();
        let rapport = sync_source(&mut masse, &WorkingTree::new(dossier), &options, &mut |_| {}).unwrap();
        println!("{dossier} : {} fichier(s) listé(s), {} ingéré(s), {:.1} s", rapport.files_listed, rapport.files_ingested, t0.elapsed().as_secs_f64());
        compter(&masse, dossier);
    }
}

#[test]
#[ignore]
fn les_morceaux_vides_d_un_gros_fichier() {
    let fichier = std::env::var("SONDE_FICHIER").unwrap_or_else(|_| {
        format!("{}/src/sharded_handle.rs", lucivy("lucivy-core"))
    });
    let texte = std::fs::read_to_string(&fichier).unwrap();
    let nom = std::path::Path::new(&fichier).file_name().unwrap().to_string_lossy().to_string();
    println!("fichier {fichier} : {} octets, {} lignes", texte.len(), texte.lines().count());
    let snapshot = Snapshot::new("sonde", [(nom, texte)]);

    let mut masse = catalogue();
    let options = SourceSyncOptions { exige: Disponibilites::RECHERCHE_TEXTE, ..Default::default() };
    let rapport = sync_source(&mut masse, &snapshot, &options, &mut |_| {}).unwrap();
    println!("sync_source : {} fichier(s) ingéré(s)", rapport.files_ingested);
    compter(&masse, "sync_source, base vide");

    let mut paquet = catalogue();
    let analyse = analyze_source(&snapshot).unwrap();
    paquet.ingest_code(&analyse).unwrap();
    compter(&paquet, "ingest_code");
}

/// Sur disque : synchroniser (avec ou sans vecteurs), compter, fermer,
/// rouvrir, recompter. `SONDE_DOSSIERS` comme plus haut ; `SONDE_BASES`, le
/// dossier des bases (effacé et recréé).
#[test]
#[ignore]
fn les_morceaux_vides_sur_disque() {
    let dossiers = std::env::var("SONDE_DOSSIERS").expect("SONDE_DOSSIERS");
    let bases = std::path::PathBuf::from(std::env::var("SONDE_BASES").expect("SONDE_BASES"));
    let _ = std::fs::remove_dir_all(&bases);
    std::fs::create_dir_all(&bases).unwrap();
    for (i, dossier) in dossiers.split(':').enumerate() {
        for (variante, exige) in [("sans vecteurs", Disponibilites::RECHERCHE_TEXTE), ("avec vecteurs", Disponibilites::TOUT)] {
            let chemin = bases.join(format!("b{i}-{}.rag3db", variante.replace(' ', "-")));
            {
                let mut cat = catalogue_sur(Rag3dbConnection::new(&chemin).unwrap());
                let options = SourceSyncOptions { exige, ..Default::default() };
                let t0 = std::time::Instant::now();
                let rapport = sync_source(&mut cat, &WorkingTree::new(dossier), &options, &mut |_| {}).unwrap();
                println!("{dossier} [{variante}] : {} ingéré(s), {:.1} s", rapport.files_ingested, t0.elapsed().as_secs_f64());
                let storage = cat.vector_storage("Scope_Chunk").unwrap();
                let v = un(&cat, &format!("MATCH (c:Scope_Chunk) WHERE c.{} IS NOT NULL RETURN count(c)", storage.column));
                println!("  vecteurs posés : {v}");
                compter(&cat, &format!("{variante}, avant fermeture"));
            }
            let cat = catalogue_sur(Rag3dbConnection::new(&chemin).unwrap());
            compter(&cat, &format!("{variante}, après réouverture"));
        }
    }
}

/// Les deux histoires d'une base vivante, sur disque (11 octobre 2026, à la
/// demande de l'arbre principal) : (1) une synchronisation en masse arrêtée
/// entre deux paquets puis reprise ; (2) un gros fichier modifié puis
/// ré-ingéré, par `edit_file` puis par une resynchronisation de la source.
#[test]
#[ignore]
fn les_morceaux_vides_apres_histoires() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    let bases = std::path::PathBuf::from(std::env::var("SONDE_BASES").expect("SONDE_BASES"));
    let _ = std::fs::remove_dir_all(&bases);
    std::fs::create_dir_all(&bases).unwrap();

    // (1) La masse interrompue puis reprise.
    let dossier = lucivy("ld-lucivy");
    let chemin = bases.join("reprise.rag3db");
    {
        let mut cat = catalogue_sur(Rag3dbConnection::new(&chemin).unwrap());
        let stop = Arc::new(AtomicBool::new(false));
        let options = SourceSyncOptions {
            exige: Disponibilites::RECHERCHE_TEXTE,
            batch_files: 64,
            stop: Some(stop.clone()),
            ..Default::default()
        };
        let mut paquets = 0;
        let rendu = sync_source(&mut cat, &WorkingTree::new(&dossier), &options, &mut |p| {
            paquets += 1;
            if p.files_done >= 300 {
                stop.store(true, Ordering::SeqCst);
            }
        });
        println!("(1) première passe : {paquets} avancées, {:?}", rendu.as_ref().map(|r| r.files_ingested).map_err(|e| e.chars().take(200).collect::<String>()));
        compter(&cat, "(1) après l'arrêt");
    }
    {
        let mut cat = catalogue_sur(Rag3dbConnection::new(&chemin).unwrap());
        compter(&cat, "(1) arrêtée, rouverte");
        let options = SourceSyncOptions { exige: Disponibilites::RECHERCHE_TEXTE, batch_files: 64, ..Default::default() };
        let rendu = sync_source(&mut cat, &WorkingTree::new(&dossier), &options, &mut |_| {}).unwrap();
        println!("(1) reprise : {} ingéré(s)", rendu.files_ingested);
        compter(&cat, "(1) après la reprise");
    }
    let cat = catalogue_sur(Rag3dbConnection::new(&chemin).unwrap());
    compter(&cat, "(1) reprise, rouverte");
    drop(cat);

    // (2) Le gros fichier modifié puis ré-ingéré.
    let core = lucivy("lucivy-core");
    let arbre = WorkingTree::new(&core);
    let (liste, _) = arbre.list_with_exclusions().unwrap();
    let fichiers: Vec<(String, String)> = liste
        .iter()
        .filter_map(|p| std::fs::read_to_string(std::path::Path::new(&core).join(p)).ok().map(|t| (p.clone(), t)))
        .collect();
    let gros = fichiers.iter().find(|(p, _)| p.ends_with("sharded_handle.rs")).expect("sharded_handle.rs").clone();
    println!("(2) {} fichiers, le gros : {}", fichiers.len(), gros.0);
    let snapshot = Snapshot::new("sonde", fichiers);
    let chemin = bases.join("reingestion.rag3db");
    let mut cat = catalogue_sur(Rag3dbConnection::new(&chemin).unwrap());
    let options = SourceSyncOptions { exige: Disponibilites::RECHERCHE_TEXTE, ..Default::default() };
    sync_source(&mut cat, &snapshot, &options, &mut |_| {}).unwrap();
    compter(&cat, "(2) indexé");

    // Même longueur : une ligne du milieu, réécrite caractère pour caractère.
    let lignes: Vec<&str> = gros.1.lines().collect();
    let milieu = lignes[lignes.len() / 2..]
        .iter()
        .find(|l| l.trim().len() > 20 && gros.1.matches(*l).count() == 1)
        .unwrap()
        .to_string();
    let remplace: String = milieu.chars().map(|c| if c.is_alphanumeric() { 'x' } else { c }).collect();
    let r1 = edit_file(&snapshot, Some(&mut cat), &gros.0, &EditOp::Replace { old: milieu, new: remplace }).unwrap();
    println!("(2) édition même longueur : {:?}", r1.reingest.as_ref().map(|x| (x.scopes_deleted,)));
    compter(&cat, "(2) après édition même longueur");

    // Une ligne insérée en tête : tous les décalages bougent.
    let actuel = snapshot.read(&gros.0).unwrap().unwrap();
    let r2 = edit_file(&snapshot, Some(&mut cat), &gros.0, &EditOp::Write { content: format!("// sonde\n{actuel}") }).unwrap();
    println!("(2) insertion en tête : {:?}", r2.reingest.as_ref().map(|x| (x.scopes_deleted,)));
    compter(&cat, "(2) après insertion en tête");

    // La resynchronisation de la source entière (la montre).
    sync_source(&mut cat, &snapshot, &options, &mut |_| {}).unwrap();
    compter(&cat, "(2) après resynchronisation");
    drop(cat);
    let cat = catalogue_sur(Rag3dbConnection::new(&chemin).unwrap());
    compter(&cat, "(2) rouverte");
}

/// **Le chemin du vrai `index`** (à la demande de l'arbre principal, nuit du
/// 11 au 12 octobre 2026) : `sync_source` en RECHERCHE_TEXTE, puis le
/// rattrapage par passes de 512 — `embarquer_le_retard` (réclamation puis
/// vecteur) ou la seule `reclamer_le_retard` —, puis réouverture.
#[test]
#[ignore]
fn les_morceaux_vides_apres_rattrapage() {
    let bases = std::path::PathBuf::from(std::env::var("SONDE_BASES").expect("SONDE_BASES"));
    let _ = std::fs::remove_dir_all(&bases);
    std::fs::create_dir_all(&bases).unwrap();
    let dossier = lucivy("ld-lucivy");
    for variante in ["embarquer", "reclamer"] {
        let chemin = bases.join(format!("{variante}.rag3db"));
        {
            let mut cat = catalogue_sur(Rag3dbConnection::new(&chemin).unwrap());
            let options = SourceSyncOptions { exige: Disponibilites::RECHERCHE_TEXTE, ..Default::default() };
            let rendu = sync_source(&mut cat, &WorkingTree::new(&dossier), &options, &mut |_| {}).unwrap();
            println!("[{variante}] sync_source : {} ingéré(s)", rendu.files_ingested);
            compter(&cat, &format!("{variante}, avant rattrapage"));
            let t0 = std::time::Instant::now();
            let (mut passes, mut lignes) = (0usize, 0usize);
            loop {
                let n = if variante == "embarquer" {
                    cat.embarquer_le_retard(Disponibilites::TOUT, 512, None).unwrap()
                } else {
                    let marqueur = cat.vector_storage("Scope_Chunk").unwrap().marker;
                    cat.reclamer_le_retard("Scope_Chunk", &marqueur, 512).unwrap().len()
                };
                if n == 0 || passes >= 1000 {
                    break;
                }
                passes += 1;
                lignes += n;
            }
            println!("[{variante}] rattrapage : {passes} passes, {lignes} lignes, {:.1} s", t0.elapsed().as_secs_f64());
            let storage = cat.vector_storage("Scope_Chunk").unwrap();
            let v = un(&cat, &format!("MATCH (c:Scope_Chunk) WHERE c.{} IS NOT NULL RETURN count(c)", storage.column));
            println!("  vecteurs posés : {v}");
            compter(&cat, &format!("{variante}, après rattrapage"));
        }
        let cat = catalogue_sur(Rag3dbConnection::new(&chemin).unwrap());
        compter(&cat, &format!("{variante}, rouverte"));
    }
}

/// Compter les morceaux sans texte d'une base existante (`SONDE_BASE`), sans
/// catalogue : la connexion seule.
#[test]
#[ignore]
fn compter_une_base() {
    let chemin = std::env::var("SONDE_BASE").expect("SONDE_BASE");
    let conn = Rag3dbConnection::new(&chemin).unwrap();
    conn.execute(&format!("LOAD EXTENSION '{}'", rag3weaver::backend::vector_extension_of_this_process().expect("l'extension vector de ce processus").display())).unwrap();
    for q in [
        "MATCH (c:Scope_Chunk) RETURN count(c)",
        "MATCH (c:Scope_Chunk) WHERE c._text IS NULL RETURN count(c)",
        "MATCH (c:Scope_Chunk) WHERE c._text = '' RETURN count(c)",
        "MATCH (c:File_Chunk) RETURN count(c)",
        "MATCH (c:File_Chunk) WHERE c._text IS NULL OR c._text = '' RETURN count(c)",
    ] {
        let r = conn.execute(q).unwrap();
        println!("{q} → {:?}", r.rows.first().and_then(|l| l.first()));
    }
}
