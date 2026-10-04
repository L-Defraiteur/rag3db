//! **La transaction par paquet rend le même graphe, ligne à ligne** (4 octobre
//! 2026). Condition de l'allumage par défaut.
//!
//! Des comptes égaux ne verraient pas une propriété posée sur la mauvaise
//! ligne — le défaut des nœuds écrits hors COPY puis par COPY dans une même
//! transaction le faisait, en silence (session cœur C++). Ici, chaque table
//! de l'utilisateur est vidée en entier et comparée :
//! - nœuds : l'ensemble des lignes, toutes propriétés, hors marques de
//!   session (`_snapshot`, `_absent_since`) qui diffèrent légitimement ;
//! - relations : l'ensemble (uuid du départ, uuid de l'arrivée, propriétés) ;
//! - plein texte : le nombre de documents qui contiennent un mot donné.
//!
//! Deux bras, chacun avec et sans `RAG3WEAVER_TX_PAR_PAQUET` (quatre paquets
//! par validation), chaque passe dans un processus neuf :
//! - **premier index** : 300 fichiers de 8 fonctions, paquets de 32 — environ
//!   256 scopes par paquet, assez pour que les liens de morceaux partent par
//!   COPY ;
//! - **reprise** : le même index posé sans transaction, puis une seconde
//!   synchronisation (fichiers changés, ajoutés, retirés) en mode Bulk — les
//!   nœuds passent par MERGE et les liens de morceaux par COPY dans le même
//!   groupe, la forme que le moteur n'avait pas éprouvée.
//!
//! Un repli de COPY au milieu d'un groupe ne se fabrique pas avec du code (tous
//! ses champs s'écrivent en CSV) : `e2e_tx_merge_puis_copy` le couvre.
//!
//! ```bash
//! ./run_e2e.sh --test e2e_tx_ligne_a_ligne
//! ```
#![cfg(all(feature = "rag3db-native", feature = "code"))]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use rag3weaver::code::{default_scope_chunking, register_code_schema};
use rag3weaver::code_sync::{sync_source, RelationsMode, SourceSyncOptions};
use rag3weaver::code_tools::Snapshot;
use rag3weaver::connection::{CypherValue, DbConnection};
use rag3weaver::disponibilite::Disponibilites;
use rag3weaver::embedder::HashEmbedder;
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

const ROLE: &str = "TX_LIGNES_ROLE";
const BASE: &str = "TX_LIGNES_BASE";

fn racine_moteur() -> String {
    std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap().display().to_string()
    })
}

fn dossier(cas: &str) -> PathBuf {
    let d = PathBuf::from(std::env::var("HOME").unwrap()).join(".cache/rag3weaver-build/tx-ligne-a-ligne").join(format!(
        "{cas}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    ));
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// La version 1 : 300 fichiers de 8 fonctions qui s'appellent, un fichier
/// sur dix importe une bibliothèque. La version 2 change un fichier sur
/// cinq, en retire dix et en ajoute vingt.
fn corpus(version: u8) -> Vec<(String, String)> {
    let fichier = |i: usize, change: bool| {
        let import = if i % 10 == 0 { "use serde::Serialize;\n\n" } else { "" };
        let corps: String = (0..8)
            .map(|j| {
                let appel = if j > 0 { format!("f{i}_{}() + ", j - 1) } else if i > 0 { format!("f{}_7() + ", i - 1) } else { String::new() };
                let val = if change { i * 10 + j + 1 } else { i * 10 + j };
                format!("pub fn f{i}_{j}() -> usize {{ {appel}{val} }}\n")
            })
            .collect();
        (format!("src/f{i:03}.rs"), format!("{import}{corps}"))
    };
    match version {
        1 => (0..300).map(|i| fichier(i, false)).collect(),
        _ => (0..320).filter(|i| !(100..110).contains(i)).map(|i| fichier(i, i % 5 == 0)).collect(),
    }
}

fn catalogue(base: &Path) -> Catalog {
    let conn = Rag3dbConnection::new(base).expect("ouvrir la base");
    conn.execute(&format!("LOAD EXTENSION '{}/extension/vector/build/libvector.rag3db_extension'", racine_moteur()))
        .expect("extension vecteur");
    let config = CatalogConfig { name: Some("tx-lignes".into()), embedding_dim: 16, ..Default::default() };
    let mut catalog = Catalog::new(Box::new(conn), Box::new(HashEmbedder::new(16)), config);
    catalog.initialize().expect("initialize");
    register_code_schema(&mut catalog, default_scope_chunking()).expect("schéma du code");
    catalog
}

fn synchroniser(catalog: &mut Catalog, version: u8) {
    let options = SourceSyncOptions {
        batch_files: 32,
        relations: Some(RelationsMode::Bulk),
        exige: Disponibilites::RECHERCHE_TEXTE,
        force: true,
        ..Default::default()
    };
    let r = sync_source(catalog, &Snapshot::new("depot", corpus(version)), &options, &mut |_| {}).expect("synchroniser");
    assert_eq!(r.failed, 0, "aucun échec : {r:?}");
}

/// Une valeur, écrite de façon stable (les flottants par leurs bits).
fn canonique(v: &CypherValue) -> String {
    match v {
        CypherValue::Map(m) => format!(
            "{{{}}}",
            m.iter()
                .filter(|(k, _)| !matches!(k.as_str(), "_snapshot" | "_absent_since" | "_id" | "_label" | "_ID" | "_LABEL" | "_src" | "_dst" | "_embed_claim"))
                .map(|(k, v)| format!("{k}={}", canonique(v)))
                .collect::<Vec<_>>()
                .join(",")
        ),
        CypherValue::List(l) => format!("[{}]", l.iter().map(canonique).collect::<Vec<_>>().join(",")),
        CypherValue::Float(f) => format!("f{:x}", f.to_bits()),
        autre => format!("{autre:?}"),
    }
}

/// Le graphe entier, table par table, en lignes canoniques.
fn vider(catalog: &Catalog) -> BTreeMap<String, BTreeSet<String>> {
    let conn = catalog.conn();
    let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let tables = conn.execute("CALL show_tables() RETURN *").unwrap();
    for r in &tables.rows {
        let Some(nom) = r.get(1).and_then(|v| v.as_str()) else { continue };
        if nom.starts_with('_') || !r.iter().any(|v| v.as_str() == Some("NODE")) {
            continue;
        }
        let lignes = conn.execute(&format!("MATCH (n:{nom}) RETURN n")).unwrap();
        out.entry(format!("nœuds {nom}")).or_default().extend(lignes.rows.iter().map(|l| canonique(&l[0])));
    }
    for d in rag3weaver::relation_directions::count_both_directions(conn).unwrap() {
        if d.from.starts_with('_') || d.to.starts_with('_') {
            continue;
        }
        // Les deux sens : depuis chaque départ, puis depuis chaque arrivée. Une
        // propriété écrite dans un sens seulement (vu le 4 octobre 2026 : 140
        // arêtes « nom » d'un côté, « fichier » de l'autre) se voit ici, dans
        // une seule base.
        for (sens, q) in [
            ("direct", format!("MATCH (a:{}) WITH a MATCH (a)-[r:{}]->(b:{}) RETURN a._uuid, b._uuid, r", d.from, d.table, d.to)),
            ("inverse", format!("MATCH (b:{}) WITH b MATCH (b)<-[r:{}]-(a:{}) RETURN a._uuid, b._uuid, r", d.to, d.table, d.from)),
        ] {
            let lignes = conn.execute(&q).unwrap();
            out.entry(format!("arêtes {} {}→{} ({sens})", d.table, d.from, d.to))
                .or_default()
                .extend(lignes.rows.iter().map(|l| format!("{}→{} {}", canonique(&l[0]), canonique(&l[1]), canonique(&l[2]))));
        }
    }
    for (entite, champ, mot) in [("Scope", "content", "pub"), ("File", "path", "src")] {
        let handle = catalog.fts_handle(entite).unwrap_or_else(|| panic!("index plein texte de {entite}"));
        let requete: lucivy_core::query::QueryConfig =
            serde_json::from_value(serde_json::json!({"type": "contains", "field": champ, "value": mot, "distance": 0})).unwrap();
        let n = handle.search(&requete, 1_000_000, None).unwrap().len();
        out.entry(format!("plein texte {entite} « {mot} »")).or_default().insert(n.to_string());
    }
    out
}

/// Le rôle du fils : `premier` (une synchronisation de la version 1) ou
/// `reprise` (la version 2, sur une base où le parent a posé la version 1
/// sans transaction), sous le réglage du fils. Le graphe est écrit dans
/// `<base>.lignes.json`.
#[test]
#[ignore]
fn role_enfant() {
    let (Ok(role), Ok(base)) = (std::env::var(ROLE), std::env::var(BASE)) else { return };
    let base = PathBuf::from(base);
    let mut catalog = catalogue(&base);
    synchroniser(&mut catalog, if role == "reprise" { 2 } else { 1 });
    let graphe = vider(&catalog);
    let fichier = PathBuf::from(format!("{}.lignes.json", base.display()));
    std::fs::write(&fichier, serde_json::to_string(&graphe).unwrap()).unwrap();
    println!("VIDE {}", fichier.display());
}

fn lancer(role: &str, base: &Path, transaction: bool) -> std::process::Output {
    let mut cmd = std::process::Command::new(std::env::current_exe().unwrap());
    cmd.args(["--exact", "role_enfant", "--nocapture", "--ignored"])
        .env(ROLE, role)
        .env(BASE, base)
        .env("RAG3WEAVER_TX_PAQUETS_PAR_VALIDATION", "4")
        .env_remove("RAG3WEAVER_TX_PAR_PAQUET");
    if transaction {
        cmd.env("RAG3WEAVER_TX_PAR_PAQUET", "1");
    }
    cmd.output().expect("lancer le fils")
}

fn graphe(base: &Path, sortie: &std::process::Output) -> BTreeMap<String, BTreeSet<String>> {
    let texte = format!("{}{}", String::from_utf8_lossy(&sortie.stdout), String::from_utf8_lossy(&sortie.stderr));
    // Toujours gardée à côté de la base : une trace doit survivre à un succès.
    let _ = std::fs::write(format!("{}.sortie.txt", base.display()), &texte);
    // Et à un endroit stable, que l'effacement des dossiers d'un succès épargne.
    if let (Some(dossier), Some(parent)) = (base.parent(), base.parent().and_then(Path::parent)) {
        let bras: String = dossier.file_name().unwrap_or_default().to_string_lossy().splitn(3, '-').take(2).collect::<Vec<_>>().join("-");
        let _ = std::fs::write(parent.join(format!("derniere-sortie-{bras}.txt")), &texte);
    }
    assert!(sortie.status.success(), "le fils a échoué ({:?}) :\n{texte}", sortie.status);
    let fichier = PathBuf::from(format!("{}.lignes.json", base.display()));
    serde_json::from_str(&std::fs::read_to_string(&fichier).unwrap()).unwrap()
}

/// **Les deux sens d'une même base** : chaque table de relations lue depuis
/// ses départs et depuis ses arrivées doit rendre les mêmes lignes.
fn sens_divergents(g: &BTreeMap<String, BTreeSet<String>>) -> Vec<String> {
    let mut out = Vec::new();
    for (table, direct) in g.iter().filter(|(t, _)| t.ends_with("(direct)")) {
        let inverse_cle = table.replace("(direct)", "(inverse)");
        let vide = BTreeSet::new();
        let inverse = g.get(&inverse_cle).unwrap_or(&vide);
        if direct != inverse {
            let d: Vec<_> = direct.difference(inverse).take(3).collect();
            let i: Vec<_> = inverse.difference(direct).take(3).collect();
            out.push(format!("{table} : {} lignes contre {} lues depuis l'arrivée ; direct seulement {d:?} ; inverse seulement {i:?}", direct.len(), inverse.len()));
        }
    }
    out
}

/// Les différences, table par table, en clair (les cinq premières lignes).
fn ecarts(a: &BTreeMap<String, BTreeSet<String>>, b: &BTreeMap<String, BTreeSet<String>>) -> Vec<String> {
    let mut out = Vec::new();
    for table in a.keys().chain(b.keys()).collect::<BTreeSet<_>>() {
        let vide = BTreeSet::new();
        let (x, y) = (a.get(table).unwrap_or(&vide), b.get(table).unwrap_or(&vide));
        if x != y {
            let seulement_a: Vec<_> = x.difference(y).take(5).collect();
            let seulement_b: Vec<_> = y.difference(x).take(5).collect();
            out.push(format!(
                "{table} : {} contre {} lignes ; avec seulement {seulement_a:?} ; sans seulement {seulement_b:?}",
                x.len(),
                y.len()
            ));
        }
    }
    out
}

/// Pose la version 1 sans transaction, par un fils.
fn poser_la_version_1(base: &Path) {
    let sortie = lancer("premier", base, false);
    graphe(base, &sortie);
}

#[test]
#[ignore]
fn un_premier_index_sous_transaction_rend_le_meme_graphe_ligne_a_ligne() {
    let (avec, sans) = (dossier("premier-avec").join("base.rag3db"), dossier("premier-sans").join("base.rag3db"));
    let g_avec = graphe(&avec, &lancer("premier", &avec, true));
    let g_sans = graphe(&sans, &lancer("premier", &sans, false));
    let lignes: usize = g_sans.values().map(BTreeSet::len).sum();
    println!("▸ premier index : {} tables, {lignes} lignes comparées", g_sans.len());
    assert!(g_sans.get("nœuds Scope").map_or(0, BTreeSet::len) >= 2400, "le corpus est bien indexé");
    for (nom, g, base) in [("avec", &g_avec, &avec), ("sans", &g_sans, &sans)] {
        let d = sens_divergents(g);
        assert!(d.is_empty(), "les deux sens divergent dans la base « {nom} » ({}) :\n{}", base.display(), d.join("\n"));
    }
    let e = ecarts(&g_avec, &g_sans);
    assert!(e.is_empty(), "le graphe diffère sous transaction (bases gardées : {} et {}) :\n{}", avec.display(), sans.display(), e.join("\n"));
    let _ = std::fs::remove_dir_all(avec.parent().unwrap());
    let _ = std::fs::remove_dir_all(sans.parent().unwrap());
}

#[test]
#[ignore]
fn une_reprise_sous_transaction_rend_le_meme_graphe_ligne_a_ligne() {
    let (avec, sans) = (dossier("reprise-avec").join("base.rag3db"), dossier("reprise-sans").join("base.rag3db"));
    poser_la_version_1(&avec);
    poser_la_version_1(&sans);
    let g_avec = graphe(&avec, &lancer("reprise", &avec, true));
    let g_sans = graphe(&sans, &lancer("reprise", &sans, false));
    let lignes: usize = g_sans.values().map(BTreeSet::len).sum();
    println!("▸ reprise : {} tables, {lignes} lignes comparées", g_sans.len());
    for (nom, g, base) in [("avec", &g_avec, &avec), ("sans", &g_sans, &sans)] {
        let d = sens_divergents(g);
        assert!(d.is_empty(), "les deux sens divergent dans la base « {nom} » ({}) :\n{}", base.display(), d.join("\n"));
    }
    let e = ecarts(&g_avec, &g_sans);
    assert!(e.is_empty(), "le graphe diffère sous transaction (bases gardées : {} et {}) :\n{}", avec.display(), sans.display(), e.join("\n"));
    let _ = std::fs::remove_dir_all(avec.parent().unwrap());
    let _ = std::fs::remove_dir_all(sans.parent().unwrap());
}
