//! **Un arrêt brutal au milieu d'un paquet en transaction** (4 octobre 2026).
//!
//! La transaction par paquet (`RAG3WEAVER_TX_PAR_PAQUET=1`, mode Bulk) fait
//! tenir un paquet entier entre `BEGIN` et `COMMIT`. C'est la condition pour
//! l'allumer par défaut : un processus tué au milieu d'un paquet, base
//! ouverte, doit laisser une base qui rouvre, et la reprise de l'index doit
//! rendre les mêmes comptes qu'une passe sans arrêt.
//!
//! Trois processus, comme `e2e_arret_brutal` :
//! - **l'écrivain** indexe un corpus de 300 fichiers par paquets de 32, et se
//!   tue par SIGKILL au paquet 2, après son ingestion et avant sa validation
//!   (crochet `RAG3WEAVER_TEST_KILL_IN_BATCH`) ;
//! - **le repreneur**, un processus neuf, rouvre la base et relance la même
//!   synchronisation ;
//! - **le témoin** indexe le même corpus dans une base neuve, sans arrêt.
//!
//! Le journal est exigé non vide après la mort : les paquets 0 et 1, validés,
//! y sont encore (aucun point de reprise ne les a repliés sur un si petit
//! corpus). Un journal vide ne prouverait rien. Sur disque, sous `~/.cache` :
//! `/tmp` est de la mémoire vive ici, et la base reste examinable.
//!
//! ```bash
//! ./run_e2e.sh --test e2e_tx_par_paquet_arret
//! ```
#![cfg(all(feature = "rag3db-native", feature = "code"))]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use rag3weaver::code::{default_scope_chunking, register_code_schema};
use rag3weaver::code_sync::{sync_source, RelationsMode, SourceSyncOptions};
use rag3weaver::code_tools::Snapshot;
use rag3weaver::connection::{CypherValue, DbConnection, QueryParam};
use rag3weaver::disponibilite::Disponibilites;
use rag3weaver::embedder::HashEmbedder;
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

const ROLE: &str = "TX_ARRET_ROLE";
const BASE: &str = "TX_ARRET_BASE";
const PAQUET_TUE: usize = 2;

thread_local! {
    /// Les fils de ce fil de test tournent-ils en mode fichiers ?
    static FICHIERS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// La n-ième synchronisation du plein texte tue-t-elle le fils ?
    static TUER_AVANT_SYNC: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) };
    /// Le rôle « un-lot » meurt-il après son lot ?
    static TUER_APRES_LOT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// Mourir pendant la validation : (n-ième COMMIT, délai en µs).
    static TUER_EN_VALIDATION: std::cell::Cell<Option<(usize, u64)>> = const { std::cell::Cell::new(None) };
    /// Le corpus gros, et le rang du paquet où l'échoueur échoue.
    static GROS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static ECHEC_AU_PAQUET: std::cell::Cell<usize> = const { std::cell::Cell::new(6) };
    /// Le corpus de la fuite, et le COPY journalisé.
    static FUITE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
/// Le « rang » qui demande la mort juste avant la poussée finale du plein
/// texte, après tous les paquets et les relations.
const AVANT_LA_POUSSEE: usize = usize::MAX;

fn racine_moteur() -> String {
    std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap().display().to_string()
    })
}

fn dossier_sur_disque(cas: &str) -> PathBuf {
    let d = PathBuf::from(std::env::var("HOME").unwrap()).join(".cache/rag3weaver-build/tx-par-paquet-arret").join(format!(
        "{cas}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    ));
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// 300 fichiers : chacun définit une fonction et appelle la précédente ; un
/// sur dix importe une bibliothèque. Des arêtes entre paquets, donc.
/// Le corpus « gros » (`TX_ARRET_GROS`, transmis aux fils) : 400 fichiers
/// de 250 fonctions, 100 000 scopes, en paquets de 200 fichiers — un paquet
/// porte 50 000 scopes et autant de morceaux, la taille où le banc voit le
/// point de reprise après un COPY annulé ne pas finir.
fn gros() -> bool {
    std::env::var_os("TX_ARRET_GROS").is_some()
}

/// Le corpus de la fuite (`TX_ARRET_FUITE`, transmis aux fils) : 600
/// fichiers de 250 fonctions, puis 10 petits. Paquets de 600 fichiers : le
/// premier porte 150 000 scopes, au-delà des 131 072 lignes d'un groupe
/// plein — la taille où une mort pendant un COPY journalisé perdait ses
/// pages dans le fichier (ticket des pages d'un COPY journalisé, corrigé par
/// `0aed3c4b5`). Le second, petit, est celui où l'écrivain meurt : le premier
/// est validé, au journal, et la réouverture le rejoue.
fn fuite() -> bool {
    std::env::var_os("TX_ARRET_FUITE").is_some()
}

fn paquet() -> usize {
    if fuite() {
        600
    } else if gros() {
        200
    } else {
        32
    }
}

fn corpus() -> Vec<(String, String)> {
    if fuite() {
        let gros_fichiers = (0..600).map(|i| {
            let corps: String = (0..250).map(|j| format!("pub fn g{i}_{j}() -> u32 {{ {j} }}\n")).collect();
            (format!("src/g{i:03}.rs"), corps)
        });
        let petits = (0..10).map(|i| (format!("src/p{i:02}.rs"), format!("pub fn p{i}() -> u32 {{ {i} }}\n")));
        return gros_fichiers.chain(petits).collect();
    }
    if gros() {
        return (0..400)
            .map(|i| {
                let corps: String = (0..250).map(|j| format!("pub fn f{i}_{j}() -> u32 {{ {j} }}\n")).collect();
                (format!("src/g{i:03}.rs"), corps)
            })
            .collect();
    }
    (0..300)
        .map(|i| {
            let import = if i % 10 == 0 { "use serde::Serialize;\n\n" } else { "" };
            let appel = if i > 0 { format!("f{}() + ", i - 1) } else { String::new() };
            (format!("src/f{i:03}.rs"), format!("{import}pub fn f{i}() -> u32 {{ {appel}{i} }}\n"))
        })
        .collect()
}

fn catalogue(base: &Path) -> Catalog {
    catalogue_en(base, false)
}

fn catalogue_en(base: &Path, lecture: bool) -> Catalog {
    let conn = Rag3dbConnection::new(base).expect("ouvrir la base");
    conn.execute(&format!("LOAD EXTENSION '{}/extension/vector/build/libvector.rag3db_extension'", racine_moteur()))
        .expect("extension vecteur");
    let config = CatalogConfig { name: Some("tx-arret".into()), embedding_dim: 16, ..Default::default() };
    let mut catalog = if lecture {
        Catalog::ouvrir_en_lecture(Box::new(conn), Box::new(HashEmbedder::new(16)), config)
    } else {
        Catalog::new(Box::new(conn), Box::new(HashEmbedder::new(16)), config)
    };
    // `TX_ARRET_FTS=fichiers` : le plein texte dans des fichiers à côté de la
    // base (`FtsStorage::Files`), dont la génération est validée avec les
    // lignes.
    if std::env::var("TX_ARRET_FTS").as_deref() == Ok("fichiers") {
        let dossier = format!("{}.fts", base.display());
        catalog.set_fts_storage(rag3weaver::fts_handle::FtsStorage::Files { base_path: dossier });
    } else {
        // Les blobs, posés : une base neuve sur disque prend les fichiers par
        // défaut, et ces cas-là éprouvent le stockage dans la base.
        catalog.set_fts_storage(rag3weaver::fts_handle::FtsStorage::BlobBacked { lazy: false });
    }
    catalog.initialize().expect("initialize");
    register_code_schema(&mut catalog, default_scope_chunking()).expect("schéma du code");
    catalog
}

/// `reprendre` : la session laissée ouverte par un processus mort se reprend
/// (`takeover`) — sans quoi la synchronisation refuse, une seule session à la
/// fois par périmètre. C'est la voie que le produit donne aujourd'hui après
/// un arrêt brutal.
fn synchroniser(catalog: &mut Catalog, reprendre: bool) {
    let options = SourceSyncOptions {
        batch_files: paquet(),
        relations: Some(RelationsMode::Bulk),
        // La fuite se cherche aussi par vecteur : les vecteurs s'écrivent.
        exige: if fuite() { Disponibilites::TOUT } else { Disponibilites::RECHERCHE_TEXTE },
        force: true,
        takeover: reprendre,
        ..Default::default()
    };
    let r = sync_source(catalog, &Snapshot::new("depot", corpus()), &options, &mut |_| {}).expect("synchroniser");
    assert_eq!(r.failed, 0, "aucun échec : {r:?}");
    println!("REPLIS_EN_MASSE {} {:?}", r.bulk_load_refused.len(), r.bulk_load_refused.first());
}

/// Les comptes de chaque table de l'utilisateur (nœuds, et arêtes lues dans
/// les deux sens). Les tables internes (`_…`) en sont exclues : le magasin
/// de blobs et les marques de session diffèrent légitimement entre une base
/// reprise et une base neuve.
fn comptes(catalog: &Catalog) -> BTreeMap<String, i64> {
    let conn = catalog.conn();
    let tables = conn.execute("CALL show_tables() RETURN *").unwrap();
    let mut out = BTreeMap::new();
    for r in &tables.rows {
        let Some(nom) = r.get(1).and_then(|v| v.as_str()) else { continue };
        if nom.starts_with('_') {
            continue;
        }
        if r.iter().any(|v| v.as_str() == Some("NODE")) {
            let n = conn.execute(&format!("MATCH (n:{nom}) RETURN count(n)")).unwrap().rows[0][0].as_i64().unwrap();
            out.insert(format!("nœuds {nom}"), n);
        }
    }
    // Le plein texte aussi : un commit lucivy oublié ou défait se verrait
    // ici, pas dans les tables. Chaque scope du corpus dit « pub » ; chaque
    // chemin de fichier, « src » (File n'indexe que son chemin).
    for (entite, champ, mot) in [("Scope", "content", "pub"), ("File", "path", "src")] {
        let handle = catalog.fts_handle(entite).unwrap_or_else(|| panic!("index plein texte de {entite}"));
        let requete: lucivy_core::query::QueryConfig =
            serde_json::from_value(serde_json::json!({"type": "contains", "field": champ, "value": mot, "distance": 0}))
                .unwrap();
        let n = handle.search(&requete, 1_000_000, None).unwrap().len() as i64;
        out.insert(format!("plein texte {entite} « {mot} »"), n);
    }
    for d in rag3weaver::relation_directions::count_both_directions(conn).unwrap() {
        if d.from.starts_with('_') || d.to.starts_with('_') {
            continue;
        }
        out.insert(format!("arêtes {} {}→{} direct", d.table, d.from, d.to), d.forward);
        out.insert(format!("arêtes {} {}→{} inverse", d.table, d.from, d.to), d.backward);
    }
    out
}

/// Une valeur, écrite de façon stable (les flottants par leurs bits), sans
/// les champs de session qui diffèrent légitimement entre une base reprise
/// et une base neuve (repris de `e2e_tx_ligne_a_ligne`). Sans les vecteurs
/// et leurs marqueurs non plus : la synchronisation exige le plein texte
/// seul, et le rattrapage opportuniste de la dette d'embarquement (512
/// lignes par table et par passe, dans l'ordre du parcours) en pose déjà
/// une partie à la reprise — l'état de la dette, pas les données.
fn canonique(v: &CypherValue) -> String {
    match v {
        CypherValue::Map(m) => format!(
            "{{{}}}",
            m.iter()
                .filter(|(k, _)| !matches!(k.as_str(), "_snapshot" | "_absent_since" | "_id" | "_label" | "_ID" | "_LABEL" | "_src" | "_dst" | "_embed_claim"))
                .filter(|(k, _)| !k.starts_with("embedding") && !k.starts_with("_embed_hash"))
                .map(|(k, v)| format!("{k}={}", canonique(v)))
                .collect::<Vec<_>>()
                .join(",")
        ),
        CypherValue::List(l) => format!("[{}]", l.iter().map(canonique).collect::<Vec<_>>().join(",")),
        CypherValue::Float(f) => format!("f{:x}", f.to_bits()),
        autre => format!("{autre:?}"),
    }
}

/// **L'empreinte ligne à ligne** du graphe : par table de nœuds et par table
/// d'arêtes (lues dans les deux sens), le compte et le hachage de l'ensemble
/// trié des lignes canoniques — propriétés comprises. Et, pour chaque uuid
/// de chaque table de nœuds, sa recherche **par la clé** (l'index de clé
/// primaire) : c'est ce contrôle qui aurait vu la clé perdue d'eb2d78e46.
/// Des empreintes, pas des textes : deux grands ensembles ne se diffent pas.
fn empreinte(catalog: &Catalog, depot: Option<&Path>) -> (BTreeMap<String, String>, usize) {
    use std::hash::{Hash, Hasher};
    let conn = catalog.conn();
    let mut out = BTreeMap::new();
    let mut introuvables = 0usize;
    // Les lignes de chaque table, triées, déposées dans un fichier : en cas
    // d'écart, le parent en lit les premières différences, bornées.
    let resume = |table: &str, lignes: std::collections::BTreeSet<String>| {
        if let Some(d) = depot {
            let _ = std::fs::create_dir_all(d);
            let nom: String = table.chars().map(|c| if c.is_alphanumeric() { c } else { '_' }).collect();
            let _ = std::fs::write(d.join(nom), lignes.iter().cloned().collect::<Vec<_>>().join("\n"));
        }
        let mut h = std::collections::hash_map::DefaultHasher::new();
        lignes.hash(&mut h);
        format!("{}:{:x}", lignes.len(), h.finish())
    };
    let tables = conn.execute("CALL show_tables() RETURN *").unwrap();
    for r in &tables.rows {
        let Some(nom) = r.get(1).and_then(|v| v.as_str()) else { continue };
        if nom.starts_with('_') || !r.iter().any(|v| v.as_str() == Some("NODE")) {
            continue;
        }
        let lignes = conn.execute(&format!("MATCH (n:{nom}) RETURN n, n._uuid")).unwrap().rows;
        let uuids: Vec<CypherValue> = lignes.iter().map(|l| l[1].clone()).collect();
        let cle = format!("nœuds {nom}");
        let v = resume(&cle, lignes.iter().map(|l| canonique(&l[0])).collect());
        out.insert(cle, v);
        for tranche in uuids.chunks(5_000) {
            let trouves = conn
                .execute_with_params(
                    &format!("UNWIND $u AS u MATCH (n:{nom} {{_uuid: u}}) RETURN count(n)"),
                    &[QueryParam::new("u", CypherValue::List(tranche.to_vec()))],
                )
                .unwrap()
                .rows[0][0]
                .as_i64()
                .unwrap() as usize;
            introuvables += tranche.len() - trouves;
        }
    }
    for d in rag3weaver::relation_directions::count_both_directions(conn).unwrap() {
        if d.from.starts_with('_') || d.to.starts_with('_') {
            continue;
        }
        for (sens, q) in [
            ("direct", format!("MATCH (a:{}) WITH a MATCH (a)-[r:{}]->(b:{}) RETURN a._uuid, b._uuid, r", d.from, d.table, d.to)),
            ("inverse", format!("MATCH (b:{}) WITH b MATCH (b)<-[r:{}]-(a:{}) RETURN a._uuid, b._uuid, r", d.to, d.table, d.from)),
        ] {
            let lignes = conn.execute(&q).unwrap().rows;
            let cle = format!("arêtes {} {}→{} ({sens})", d.table, d.from, d.to);
            let v = resume(&cle, lignes.iter().map(|l| format!("{}→{} {}", canonique(&l[0]), canonique(&l[1]), canonique(&l[2]))).collect());
            out.insert(cle, v);
        }
    }
    (out, introuvables)
}

/// **Le rôle d'un processus fils** : sans variable, rien (il ne tourne que
/// lancé par le test ci-dessous).
#[test]
#[ignore]
fn role_enfant() {
    let (Ok(role), Ok(base)) = (std::env::var(ROLE), std::env::var(BASE)) else { return };
    let base = PathBuf::from(base);
    if role == "lecteur-fts" {
        // Un lecteur seul : il calcule l'état, ne jette ni n'écrit rien.
        let mut catalog = catalogue_en(&base, true);
        catalog.open_fts_files();
        let etat = catalog.index_state_for("Scope").expect("état");
        println!("ETAT {:?} {:?}", etat.text, etat.text_percent);
        println!("DOSSIER {}", Path::new(&format!("{}.fts", base.display())).exists());
        return;
    }
    if role == "constat" {
        // Juste après une mort, avant toute reprise : un groupe est là en
        // entier ou pas du tout. Ni morceau sans son arête, ni parent marqué
        // découpé sans morceau.
        // En écriture, sans initialiser le catalogue : une ouverture en
        // lecture seule ne rejouerait pas les pages fantômes d'un point de
        // reprise interrompu ; seules des lectures suivent.
        let conn = rag3weaver::Rag3dbConnection::new(&base).expect("rouvrir la base");
        let compte = |q: &str| rag3weaver::connection::DbConnection::execute(&conn, q).unwrap().rows[0][0].as_i64().unwrap();
        let fichiers = compte("MATCH (f:File) RETURN count(f)");
        let mut orphelins = 0;
        let mut sans_morceau = 0;
        for e in ["File", "Scope"] {
            orphelins += compte(&format!("MATCH (c:{e}_Chunk) WHERE NOT EXISTS {{ MATCH (c)-[:{e}_CHUNKED_FROM]->(:{e}) }} RETURN count(c)"));
            sans_morceau += compte(&format!(
                "MATCH (p:{e}) WHERE p._chunked_hash IS NOT NULL AND p._chunked_hash <> '' \
                 AND NOT EXISTS {{ MATCH (:{e}_Chunk)-[:{e}_CHUNKED_FROM]->(p) }} RETURN count(p)"
            ));
        }
        println!("CONSTAT fichiers={fichiers} morceaux_sans_arete={orphelins} parents_sans_morceau={sans_morceau}");
        return;
    }
    let mut catalog = catalogue(&base);
    if role == "un-lot" {
        // Un écrivain qui ne fait qu'un lot du rebâti ; avec
        // `TX_ARRET_TUER_APRES_LOT`, il meurt ensuite, rebâti interrompu.
        catalog.open_fts_files();
        let reste = catalog.rebuild_fts_step(20).expect("un lot");
        // Le premier lot va à la première table dans l'ordre (File).
        let etat = catalog.index_state_for("File").expect("état");
        println!("ETAT {:?} {:?} reste={reste}", etat.text, etat.text_percent);
        if std::env::var_os("TX_ARRET_TUER_APRES_LOT").is_some() {
            let _ = std::process::Command::new("kill").args(["-KILL", &std::process::id().to_string()]).status();
            loop {
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        }
        return;
    }
    if role == "lecteur" {
        // Ce qu'une recherche verrait, sans synchroniser : l'état des mots.
        let etat = catalog.index_state_for("Scope").expect("état d'index");
        println!("ETAT mots {:?}", etat.text);
        return;
    }
    if role == "chercheur" {
        // Une recherche seule, sans synchronisation : la voie de la recherche
        // doit rebâtir un dossier en avance, pas chercher dans un index vide.
        let catalog = std::sync::Arc::new(std::sync::Mutex::new(catalog));
        let trouves = Catalog::rechercher(&catalog, rag3weaver::code::SCOPE, "f010", rag3weaver::search::SearchOptions {
            consistency: rag3weaver::search::Consistency::Immediate,
            signals: Some(rag3weaver::search::SearchSignals::BM25),
            ..Default::default()
        })
        .expect("recherche");
        println!("PREMIERE {}", trouves.results.len());
        // Le rebâti tourne en fond : attendre qu'il ait fini, puis chercher.
        for _ in 0..600 {
            let fini = catalog.lock().unwrap().index_state_for("Scope").map(|e| e.text_percent.is_none()).unwrap_or(false);
            if fini {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        let trouves = Catalog::rechercher(&catalog, rag3weaver::code::SCOPE, "f010", rag3weaver::search::SearchOptions {
            consistency: rag3weaver::search::Consistency::Immediate,
            signals: Some(rag3weaver::search::SearchSignals::BM25),
            ..Default::default()
        })
        .expect("recherche");
        println!("TROUVES {}", trouves.results.len());
        return;
    }
    if role == "echoueur" {
        // Le chemin du ROLLBACK : le paquet échoue, la base est empoisonnée,
        // puis lâchée (sa fermeture fait un point de reprise après
        // l'annulation). La reprise se prouve dans un processus neuf.
        let options = SourceSyncOptions {
            batch_files: paquet(),
            relations: Some(RelationsMode::Bulk),
            exige: Disponibilites::RECHERCHE_TEXTE,
            force: true,
            ..Default::default()
        };
        let erreur = sync_source(&mut catalog, &Snapshot::new("depot", corpus()), &options, &mut |_| {})
            .expect_err("le paquet piégé échoue");
        println!("ECHEC {erreur}");
        assert!(catalog.must_reopen().is_some(), "le catalogue est empoisonné après le ROLLBACK : {erreur}");
        drop(catalog);
        println!("FERME");
        return;
    }
    synchroniser(&mut catalog, role == "repreneur");
    // L'écrivain n'arrive jamais ici : le crochet le tue au paquet tué.
    println!("COMPTES {role} {}", serde_json::to_string(&comptes(&catalog)).unwrap());
    if gros() {
        let depot = PathBuf::from(format!("{}.lignes-{role}", base.display()));
        let (e, introuvables) = empreinte(&catalog, Some(&depot));
        println!("EMPREINTE {role} {}", serde_json::to_string(&e).unwrap());
        println!("INTROUVABLES_PAR_CLE {introuvables}");
    }
    if fuite() {
        // Les COPY journalisés repliés sur un point de reprise : zéro, sinon
        // rien n'a été journalisé et le cas ne prouve rien.
        let replis = catalog
            .conn()
            .execute("CALL current_setting('copy_journal_fallbacks') RETURN *")
            .map(|r| format!("{:?}", r.rows.first()))
            .unwrap_or_else(|e| format!("illisible : {e}"));
        println!("REPLIS_DU_JOURNAL {role} {replis}");
        // Une fonction du premier paquet, celui que la réouverture a rejoué
        // du journal, cherchée par son nom et par son texte.
        let partage = std::sync::Arc::new(std::sync::Mutex::new(catalog));
        let cherche = |requete: &str, signaux| {
            Catalog::rechercher(&partage, rag3weaver::code::SCOPE, requete, rag3weaver::search::SearchOptions {
                signals: Some(signaux),
                ..Default::default()
            })
            .expect("recherche")
            .results
            .iter()
            .any(|x| format!("{x:?}").contains("g005.rs"))
        };
        println!(
            "TROUVE {role} mot={} vecteur={}",
            cherche("g5_7", rag3weaver::search::SearchSignals::BM25),
            cherche("pub fn g5_7() -> u32 { 7 }", rag3weaver::search::SearchSignals::VECTOR)
        );
        catalog = std::sync::Arc::try_unwrap(partage).ok().unwrap().into_inner().unwrap();
    }
    if role == "repreneur" {
        // Un point de reprise explicite à la fin de la reprise : sur l'état
        // d'après un COPY annulé, il ne finit pas (le banc) ; ici il doit
        // finir, et vite.
        let t = std::time::Instant::now();
        catalog.conn().execute("CHECKPOINT").expect("point de reprise à la fin de la reprise");
        println!("POINT_DE_REPRISE {}", t.elapsed().as_millis());
    }
}

fn lancer(role: &str, base: &Path, tuer: bool) -> (std::process::ExitStatus, String) {
    lancer_avec(role, base, tuer.then_some(PAQUET_TUE), true, 1)
}

/// `tuer` : le rang du paquet où mourir ; `par_validation` : K, les paquets
/// par validation (`RAG3WEAVER_TX_PAQUETS_PAR_VALIDATION`).
fn lancer_avec(role: &str, base: &Path, tuer: Option<usize>, transaction: bool, par_validation: usize) -> (std::process::ExitStatus, String) {
    let mut cmd = std::process::Command::new(std::env::current_exe().unwrap());
    if FICHIERS.with(|f| f.get()) {
        cmd.env("TX_ARRET_FTS", "fichiers");
    } else {
        cmd.env_remove("TX_ARRET_FTS");
    }
    cmd.args(["--exact", "role_enfant", "--nocapture", "--ignored"])
        .env(ROLE, role)
        .env(BASE, base)
        .env("RAG3WEAVER_TX_PAQUETS_PAR_VALIDATION", par_validation.to_string())
        .env("RAG3WEAVER_TX_PAR_PAQUET", "0")
        .env_remove("RAG3WEAVER_TEST_KILL_IN_BATCH")
        .env_remove("RAG3WEAVER_TEST_KILL_BEFORE_BLOB_PUSH")
        .env_remove("RAG3WEAVER_TX_POUSSEE_A_LA_FIN")
        .env_remove("RAG3WEAVER_TEST_FAIL_IN_BATCH")
        .env_remove("RAG3WEAVER_TEST_KILL_IN_COMMIT");
    if let Some((n, delai)) = TUER_EN_VALIDATION.with(|f| f.get()) {
        cmd.env("RAG3WEAVER_TEST_KILL_IN_COMMIT", format!("{n}:{delai}"));
    }
    if transaction {
        cmd.env("RAG3WEAVER_TX_PAR_PAQUET", "1");
    }
    if let Some(rang) = tuer {
        if rang == AVANT_LA_POUSSEE {
            // La poussée unique est une option : l'écrivain la demande.
            cmd.env("RAG3WEAVER_TX_POUSSEE_A_LA_FIN", "1");
            cmd.env("RAG3WEAVER_TEST_KILL_BEFORE_BLOB_PUSH", "1");
        } else {
            cmd.env("RAG3WEAVER_TEST_KILL_IN_BATCH", rang.to_string());
        }
    }
    if role == "echoueur" {
        cmd.env("RAG3WEAVER_TEST_FAIL_IN_BATCH", ECHEC_AU_PAQUET.with(|f| f.get()).to_string());
    }
    if GROS.with(|f| f.get()) {
        cmd.env("TX_ARRET_GROS", "1");
    } else {
        cmd.env_remove("TX_ARRET_GROS");
    }
    if FUITE.with(|f| f.get()) {
        cmd.env("TX_ARRET_FUITE", "1").env("RAG3WEAVER_TEST_COPY_JOURNALISE", "1");
    } else {
        cmd.env_remove("TX_ARRET_FUITE").env_remove("RAG3WEAVER_TEST_COPY_JOURNALISE");
    }
    match TUER_AVANT_SYNC.with(|f| f.get()) {
        Some(n) => {
            cmd.env("RAG3WEAVER_TEST_KILL_BEFORE_FTS_SYNC", n.to_string());
        }
        None => {
            cmd.env_remove("RAG3WEAVER_TEST_KILL_BEFORE_FTS_SYNC");
        }
    }
    if TUER_APRES_LOT.with(|f| f.get()) {
        cmd.env("TX_ARRET_TUER_APRES_LOT", "1");
    } else {
        cmd.env_remove("TX_ARRET_TUER_APRES_LOT");
    }
    let sortie = cmd.output().expect("lancer le fils");
    let texte = format!("{}{}", String::from_utf8_lossy(&sortie.stdout), String::from_utf8_lossy(&sortie.stderr));
    (sortie.status, texte)
}

fn comptes_rendus(role: &str, sortie: &str) -> BTreeMap<String, i64> {
    let ligne = sortie
        .lines()
        .find_map(|l| l.strip_prefix(&format!("COMPTES {role} ")))
        .unwrap_or_else(|| panic!("le {role} n'a pas rendu ses comptes :\n{sortie}"));
    serde_json::from_str(ligne).unwrap()
}

/// Les fichiers du journal à côté de la base, et leur taille.
fn journal(dossier: &Path) -> Vec<(String, u64)> {
    std::fs::read_dir(dossier)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().contains("wal"))
        .map(|e| (e.file_name().to_string_lossy().to_string(), e.metadata().map(|m| m.len()).unwrap_or(0)))
        .collect()
}

#[test]
#[ignore]
fn un_arret_au_milieu_d_un_paquet_se_reprend_aux_comptes_d_une_passe_sans_arret() {
    let dossier = dossier_sur_disque("repris");
    let base = dossier.join("base.rag3db");

    let (statut, sortie) = lancer("ecrivain", &base, true);
    println!("▸ écrivain : {statut:?}");
    use std::os::unix::process::ExitStatusExt;
    assert_eq!(statut.signal(), Some(9), "l'écrivain doit mourir par SIGKILL au paquet {PAQUET_TUE} :\n{sortie}");
    assert!(sortie.contains(&format!("SIGKILL au paquet {PAQUET_TUE}")), "mort au crochet, pas ailleurs :\n{sortie}");
    let wal = journal(&dossier);
    println!("▸ journal après la mort : {wal:?}");
    assert!(wal.iter().any(|(_, t)| *t > 0), "journal vide ou absent : la mort ne prouverait rien ({wal:?})");

    let (statut, sortie) = lancer("repreneur", &base, false);
    assert!(statut.success(), "la base rouvre, et la reprise va au bout :\n{sortie}");
    let repris = comptes_rendus("repreneur", &sortie);

    let temoin_dossier = dossier_sur_disque("temoin");
    let (statut, sortie) = lancer("temoin", &temoin_dossier.join("base.rag3db"), false);
    assert!(statut.success(), "le témoin va au bout :\n{sortie}");
    let temoin = comptes_rendus("temoin", &sortie);

    println!("▸ comptes repris : {repris:?}");
    assert!(temoin.get("nœuds Scope").copied().unwrap_or(0) >= 300, "le témoin a bien indexé le corpus : {temoin:?}");
    assert_eq!(repris, temoin, "la reprise rend les comptes d'une passe sans arrêt");
    let _ = std::fs::remove_dir_all(&dossier);
    let _ = std::fs::remove_dir_all(&temoin_dossier);
}

/// **La transaction par paquet rend le graphe du chemin ordinaire** : même
/// corpus, une première indexation avec la variable (transaction, naissances
/// par COPY à chaque paquet, marque de session dans la ligne) et une sans ;
/// les comptes de nœuds et d'arêtes, dans les deux sens, sont égaux.
#[test]
#[ignore]
fn la_transaction_par_paquet_rend_les_comptes_du_chemin_ordinaire() {
    let avec = dossier_sur_disque("avec");
    let sans = dossier_sur_disque("sans");
    let (statut, sortie) = lancer_avec("avec", &avec.join("base.rag3db"), None, true, 1);
    assert!(statut.success(), "avec la transaction :\n{sortie}");
    let comptes_avec = comptes_rendus("avec", &sortie);
    let (statut, sortie) = lancer_avec("sans", &sans.join("base.rag3db"), None, false, 1);
    assert!(statut.success(), "sans la transaction :\n{sortie}");
    let comptes_sans = comptes_rendus("sans", &sortie);
    assert!(comptes_sans.get("nœuds Scope").copied().unwrap_or(0) >= 300, "{comptes_sans:?}");
    assert_eq!(comptes_avec, comptes_sans, "mêmes comptes avec et sans la transaction par paquet");
    let _ = std::fs::remove_dir_all(&avec);
    let _ = std::fs::remove_dir_all(&sans);
}

/// **Deux paquets par validation** (`RAG3WEAVER_TX_PAQUETS_PAR_VALIDATION=2`) :
/// la mort au paquet 3 défait les paquets 2 et 3, ouverts ensemble ; la
/// reprise les refait, et les comptes sont ceux d'une passe sans arrêt —
/// celle-ci en une seule validation, pour couvrir aussi « tous ».
#[test]
#[ignore]
fn un_arret_a_deux_paquets_par_validation_se_reprend_aux_memes_comptes() {
    let dossier = dossier_sur_disque("repris-k2");
    let base = dossier.join("base.rag3db");
    let (statut, sortie) = lancer_avec("ecrivain", &base, Some(3), true, 2);
    use std::os::unix::process::ExitStatusExt;
    assert_eq!(statut.signal(), Some(9), "mort par SIGKILL au paquet 3 :\n{sortie}");
    assert!(journal(&dossier).iter().any(|(_, t)| *t > 0), "journal vide : la mort ne prouverait rien");
    let (statut, sortie) = lancer_avec("repreneur", &base, None, true, 2);
    assert!(statut.success(), "la base rouvre, et la reprise va au bout :\n{sortie}");
    let repris = comptes_rendus("repreneur", &sortie);
    let temoin_dossier = dossier_sur_disque("temoin-tous");
    let (statut, sortie) = lancer_avec("temoin", &temoin_dossier.join("base.rag3db"), None, true, 0);
    assert!(statut.success(), "le témoin, en une seule validation, va au bout :\n{sortie}");
    let temoin = comptes_rendus("temoin", &sortie);
    assert!(temoin.get("nœuds Scope").copied().unwrap_or(0) >= 300, "{temoin:?}");
    assert_eq!(repris, temoin, "à K = 2 comme à une seule validation, les comptes d'une passe sans arrêt");
    let _ = std::fs::remove_dir_all(&dossier);
    let _ = std::fs::remove_dir_all(&temoin_dossier);
}

/// **Au réglage proposé : quatre paquets par validation.** Dix paquets de 32
/// fichiers : les groupes 0–3 et 4–7, puis 8–9. La mort au paquet 6, au
/// milieu du deuxième groupe, défait les paquets 4 à 6 et garde les 0 à 3,
/// validés ; la reprise rend les comptes d'une passe sans arrêt au même
/// réglage.
#[test]
#[ignore]
fn un_arret_au_milieu_d_un_groupe_de_quatre_paquets_se_reprend_aux_memes_comptes() {
    let dossier = dossier_sur_disque("repris-k4");
    let base = dossier.join("base.rag3db");
    let (statut, sortie) = lancer_avec("ecrivain", &base, Some(6), true, 4);
    use std::os::unix::process::ExitStatusExt;
    assert_eq!(statut.signal(), Some(9), "mort par SIGKILL au paquet 6 :\n{sortie}");
    let wal = journal(&dossier);
    println!("▸ journal après la mort, K = 4 : {wal:?}");
    assert!(wal.iter().any(|(_, t)| *t > 0), "journal vide : la mort ne prouverait rien ({wal:?})");
    let (statut, sortie) = lancer_avec("repreneur", &base, None, true, 4);
    assert!(statut.success(), "la base rouvre, et la reprise va au bout :\n{sortie}");
    let repris = comptes_rendus("repreneur", &sortie);
    let temoin_dossier = dossier_sur_disque("temoin-k4");
    let (statut, sortie) = lancer_avec("temoin", &temoin_dossier.join("base.rag3db"), None, true, 4);
    assert!(statut.success(), "le témoin va au bout :\n{sortie}");
    let temoin = comptes_rendus("temoin", &sortie);
    assert!(temoin.get("nœuds Scope").copied().unwrap_or(0) >= 300, "{temoin:?}");
    assert_eq!(repris, temoin, "à K = 4, la reprise rend les comptes d'une passe sans arrêt");
    let _ = std::fs::remove_dir_all(&dossier);
    let _ = std::fs::remove_dir_all(&temoin_dossier);
}

/// **Une mort pendant la validation d'un groupe** (5 octobre 2026). Un
/// groupe de quatre paquets mêle des écritures ordinaires (MERGE, SET) et des
/// COPY ; sa validation force un point de reprise. Le moteur, avant
/// `37608cf4b`, rejouait par le journal les écritures ordinaires d'un groupe
/// mort pendant ce point de reprise, sans les lignes du COPY : un groupe à
/// moitié, des morceaux sans leur arête que la reprise ne refaisait jamais.
/// Ici, un autre fil tue l'écrivain un délai donné après le début de la
/// deuxième validation (`RAG3WEAVER_TEST_KILL_IN_COMMIT`), balayé pour
/// tomber avant, pendant et après le point de reprise. Juste après chaque
/// mort, en lecture : 128, 256 ou 300 fichiers (un groupe entier ou pas du
/// tout), aucun morceau sans arête, aucun parent découpé sans morceau ; puis
/// la reprise rend les comptes d'une passe sans arrêt.
#[test]
#[ignore]
fn une_mort_pendant_la_validation_d_un_groupe_le_laisse_entier_ou_absent() {
    let temoin_dossier = dossier_sur_disque("temoin-validation");
    let (statut, sortie) = lancer_avec("temoin", &temoin_dossier.join("base.rag3db"), None, true, 4);
    assert!(statut.success(), "le témoin va au bout :\n{sortie}");
    let temoin = comptes_rendus("temoin", &sortie);
    let mut constats = Vec::new();
    // Le point de reprise de la validation tombe, sur ce corpus, entre 100 et
    // 200 ms : le pas s'y resserre.
    let delais = [0u64, 10_000, 50_000, 100_000, 110_000, 120_000, 130_000, 140_000, 150_000, 160_000, 170_000, 180_000, 190_000, 200_000, 400_000];
    for delai in delais {
        let dossier = dossier_sur_disque(&format!("validation-{delai}"));
        let base = dossier.join("base.rag3db");
        TUER_EN_VALIDATION.with(|f| f.set(Some((2, delai))));
        let (statut, sortie) = lancer_avec("ecrivain", &base, None, true, 4);
        TUER_EN_VALIDATION.with(|f| f.set(None));
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(statut.signal(), Some(9), "mort par SIGKILL pendant la deuxième validation ({delai} µs) :\n{sortie}");
        let (statut, sortie) = lancer_avec("constat", &base, None, true, 4);
        assert!(statut.success(), "la base rouvre après la mort ({delai} µs) :\n{sortie}");
        let constat = ligne(&sortie, "CONSTAT ");
        println!("▸ {delai:>7} µs : {constat}");
        let lire = |cle: &str| -> i64 {
            constat.split_whitespace().find_map(|m| m.strip_prefix(cle)).and_then(|v| v.parse().ok()).unwrap()
        };
        assert!([128, 256, 300].contains(&lire("fichiers=")), "un groupe à moitié ({delai} µs) : {constat}");
        assert_eq!(lire("morceaux_sans_arete="), 0, "des morceaux sans leur arête ({delai} µs) : {constat}");
        assert_eq!(lire("parents_sans_morceau="), 0, "des parents découpés sans morceau ({delai} µs) : {constat}");
        constats.push(lire("fichiers="));
        let (statut, sortie) = lancer_avec("repreneur", &base, None, true, 4);
        assert!(statut.success(), "la reprise va au bout ({delai} µs) :\n{sortie}");
        let repris = comptes_rendus("repreneur", &sortie);
        assert_eq!(repris, temoin, "après une mort pendant la validation ({delai} µs), les comptes d'une passe sans arrêt");
        let _ = std::fs::remove_dir_all(&dossier);
    }
    let absent = constats.iter().filter(|&&n| n == 128).count();
    let entier = constats.iter().filter(|&&n| n == 256).count();
    println!("▸ {} morts : groupe absent {absent}, groupe entier {entier}, plus loin {}", constats.len(), constats.len() - absent - entier);
    let _ = std::fs::remove_dir_all(&temoin_dossier);
}

/// **Le chemin du ROLLBACK** (K = 4) : le paquet 6 échoue au milieu du
/// deuxième groupe ; ROLLBACK, catalogue empoisonné, puis la base lâchée —
/// sa fermeture fait un point de reprise sur l'état d'après l'annulation,
/// sûr depuis 836edfc29 (le compte de l'index de clé recule) ; puis la base
/// rouverte dans un processus neuf, l'index repris, et un point de reprise
/// explicite à la fin qui finit en temps borné. Les comptes sont
/// ceux d'une passe sans échec.
#[test]
#[ignore]
fn un_paquet_qui_echoue_est_defait_et_la_reprise_rend_les_memes_comptes() {
    let dossier = dossier_sur_disque("rollback");
    let base = dossier.join("base.rag3db");
    let (statut, sortie) = lancer_avec("echoueur", &base, None, true, 4);
    assert!(statut.success(), "échec, ROLLBACK, empoisonnement et fermeture :\n{sortie}");
    assert!(sortie.contains("échec au paquet 6") && sortie.contains("FERME"), "le paquet piégé a échoué, la base est fermée :\n{sortie}");
    let (statut, sortie) = lancer_avec("repreneur", &base, None, true, 4);
    assert!(statut.success(), "la base rouvre dans un processus neuf, et la reprise va au bout :\n{sortie}");
    let repris = comptes_rendus("repreneur", &sortie);
    let sortie_reprise = sortie.clone();
    let temoin_dossier = dossier_sur_disque("temoin-rollback");
    let (statut, sortie) = lancer_avec("temoin", &temoin_dossier.join("base.rag3db"), None, true, 4);
    assert!(statut.success(), "le témoin va au bout :\n{sortie}");
    let temoin = comptes_rendus("temoin", &sortie);
    assert!(temoin.get("nœuds Scope").copied().unwrap_or(0) >= 300, "{temoin:?}");
    assert_eq!(repris, temoin, "après un ROLLBACK et une reprise, les comptes d'une passe sans échec");
    let duree = point_de_reprise_final(&sortie_reprise);
    assert!(duree < 60_000, "le point de reprise final finit en temps borné : {duree} ms");
    let _ = std::fs::remove_dir_all(&dossier);
    let _ = std::fs::remove_dir_all(&temoin_dossier);
}

/// Le temps du point de reprise explicite à la fin d'une reprise.
fn point_de_reprise_final(sortie: &str) -> u128 {
    ligne(sortie, "POINT_DE_REPRISE ").trim().parse().unwrap()
}

/// **Un gros paquet défait** (5 octobre 2026), à la taille du banc : le
/// deuxième paquet (200 fichiers, 50 000 scopes et autant de morceaux, par
/// COPY) échoue ; ROLLBACK, base fermée avec son point de reprise ; la reprise,
/// dans un processus neuf, refait le même COPY et va au bout aux comptes
/// d'une passe sans échec, puis un point de reprise explicite finit en temps
/// borné — la recette du banc (COPY annulé, même COPY validé, point de
/// reprise) ne se rejoue pas chez nous à travers la réouverture.
#[test]
#[ignore]
fn un_gros_paquet_defait_se_reprend_et_son_point_de_reprise_finit() {
    // Rouge jusqu'à eb2d78e46 (ticket
    // `2026-10-05-cle-perdue-par-l-index-de-cle-primaire`) : une clé sortait
    // de l'index de clé primaire après deux COPY de 50 000 lignes, et le COPY
    // des arêtes DEFINES était refusé avant le paquet piégé.
    GROS.with(|f| f.set(true));
    ECHEC_AU_PAQUET.with(|f| f.set(1));
    let dossier = dossier_sur_disque("gros-rollback");
    let base = dossier.join("base.rag3db");
    let (statut, sortie) = lancer_avec("echoueur", &base, None, true, 1);
    assert!(statut.success(), "échec, ROLLBACK, empoisonnement et fermeture :\n{}", sortie.chars().take(3000).collect::<String>());
    assert!(
        sortie.contains("échec au paquet 1") && sortie.contains("FERME"),
        "le gros paquet a échoué, la base est fermée :\n{}",
        sortie.lines().filter(|l| l.starts_with("ECHEC") || l.starts_with("FERME") || l.contains("panicked") || l.contains("rror")).take(20).collect::<Vec<_>>().join("\n")
    );
    let (statut, sortie) = lancer_avec("repreneur", &base, None, true, 1);
    assert!(statut.success(), "la reprise va au bout ({:?}) :\n{}", statut, sortie.chars().take(3000).collect::<String>());
    let repris = comptes_rendus("repreneur", &sortie);
    let duree = point_de_reprise_final(&sortie);
    let replis_reprise = ligne(&sortie, "REPLIS_EN_MASSE ");
    let empreinte_reprise = ligne(&sortie, "EMPREINTE repreneur ");
    let introuvables_reprise = ligne(&sortie, "INTROUVABLES_PAR_CLE ");
    let temoin_dossier = dossier_sur_disque("gros-temoin");
    let (statut, sortie) = lancer_avec("temoin", &temoin_dossier.join("base.rag3db"), None, true, 1);
    GROS.with(|f| f.set(false));
    ECHEC_AU_PAQUET.with(|f| f.set(6));
    assert!(statut.success(), "le témoin va au bout");
    let temoin = comptes_rendus("temoin", &sortie);
    println!(
        "▸ gros paquet défait : point de reprise final en {duree} ms ; scopes {:?} ; replis en masse : reprise {replis_reprise}, témoin {}",
        repris.get("nœuds Scope"),
        ligne(&sortie, "REPLIS_EN_MASSE ")
    );
    assert!(temoin.get("nœuds Scope").copied().unwrap_or(0) >= 100_000, "{temoin:?}");
    assert_eq!(repris, temoin, "après un gros paquet défait et une reprise, les comptes d'une passe sans échec");
    let empreinte_temoin = ligne(&sortie, "EMPREINTE temoin ");
    let ecarts: Vec<String> = {
        let a: BTreeMap<String, String> = serde_json::from_str(&empreinte_reprise).unwrap();
        let b: BTreeMap<String, String> = serde_json::from_str(&empreinte_temoin).unwrap();
        a.keys().chain(b.keys()).collect::<std::collections::BTreeSet<_>>().into_iter()
            .filter(|k| a.get(*k) != b.get(*k))
            .map(|k| format!("{k} : reprise {:?}, témoin {:?}", a.get(k), b.get(k)))
            .collect()
    };
    println!("▸ ligne à ligne : {} tables, {} écarts ; introuvables par la clé : reprise {introuvables_reprise}, témoin {}", serde_json::from_str::<BTreeMap<String, String>>(&empreinte_temoin).unwrap().len(), ecarts.len(), ligne(&sortie, "INTROUVABLES_PAR_CLE "));
    if !ecarts.is_empty() {
        // Les trois premières différences de chaque table en écart, bornées.
        let a: BTreeMap<String, String> = serde_json::from_str(&empreinte_reprise).unwrap();
        let b: BTreeMap<String, String> = serde_json::from_str(&empreinte_temoin).unwrap();
        for k in a.keys().filter(|k| a.get(*k) != b.get(*k)) {
            let nom: String = k.chars().map(|c| if c.is_alphanumeric() { c } else { '_' }).collect();
            let lire = |d: &Path| std::fs::read_to_string(d.join(&nom)).unwrap_or_default();
            let ra = lire(Path::new(&format!("{}.lignes-repreneur", base.display())));
            let rb = lire(Path::new(&format!("{}.lignes-temoin", temoin_dossier.join("base.rag3db").display())));
            let mut vues = 0;
            for (x, y) in ra.lines().zip(rb.lines()) {
                if x != y {
                    println!("▸ {k}\n    reprise : {}\n    témoin  : {}", x.chars().take(600).collect::<String>(), y.chars().take(600).collect::<String>());
                    vues += 1;
                    if vues == 3 {
                        break;
                    }
                }
            }
        }
    }
    assert!(ecarts.is_empty(), "la reprise et le témoin, ligne à ligne (propriétés et arêtes) : {ecarts:?}");
    assert_eq!(introuvables_reprise.trim(), "0", "chaque uuid de la reprise se trouve par sa clé");
    assert_eq!(ligne(&sortie, "INTROUVABLES_PAR_CLE ").trim(), "0", "chaque uuid du témoin se trouve par sa clé");
    assert!(duree < 60_000, "le point de reprise final finit en temps borné : {duree} ms");
    let _ = std::fs::remove_dir_all(&dossier);
    let _ = std::fs::remove_dir_all(&temoin_dossier);
}

/// **Une mort juste avant la poussée finale du plein texte** (K = 4) : tous
/// les paquets et les relations sont validés, mais les blobs du plein texte,
/// retenus, ne sont pas en base. Une lecture de l'état dit « mots : en
/// cours » (pas « prêt » sur un index vide) ; la reprise rebâtit le plein
/// texte depuis les lignes, et les comptes — plein texte compris — sont ceux
/// d'une passe sans arrêt.
#[test]
#[ignore]
fn une_mort_avant_la_poussee_du_plein_texte_se_reprend_aux_memes_comptes() {
    let dossier = dossier_sur_disque("avant-poussee");
    let base = dossier.join("base.rag3db");
    let (statut, sortie) = lancer_avec("ecrivain", &base, Some(AVANT_LA_POUSSEE), true, 4);
    use std::os::unix::process::ExitStatusExt;
    assert_eq!(statut.signal(), Some(9), "mort par SIGKILL avant la poussée :\n{sortie}");
    assert!(sortie.contains("SIGKILL avant la poussée du plein texte"), "mort au crochet :\n{sortie}");
    let (statut, sortie) = lancer_avec("lecteur", &base, None, true, 4);
    assert!(statut.success(), "{sortie}");
    assert!(sortie.contains("ETAT mots Running"), "les mots sont « en cours », pas « prêts » :\n{sortie}");
    let (statut, sortie) = lancer_avec("repreneur", &base, None, true, 4);
    assert!(statut.success(), "la reprise rebâtit le plein texte et va au bout :\n{sortie}");
    assert!(sortie.contains("reconstruction depuis les lignes"), "le plein texte a été rebâti :\n{sortie}");
    let repris = comptes_rendus("repreneur", &sortie);
    let temoin_dossier = dossier_sur_disque("temoin-avant-poussee");
    let (statut, sortie) = lancer_avec("temoin", &temoin_dossier.join("base.rag3db"), None, true, 4);
    assert!(statut.success(), "le témoin va au bout :\n{sortie}");
    let temoin = comptes_rendus("temoin", &sortie);
    assert!(temoin.get("plein texte Scope « pub »").copied().unwrap_or(0) >= 300, "{temoin:?}");
    assert_eq!(repris, temoin, "après la reconstruction, les comptes d'une passe sans arrêt");
    let _ = std::fs::remove_dir_all(&dossier);
    let _ = std::fs::remove_dir_all(&temoin_dossier);
}

/// **Le plein texte en fichiers, après un arrêt entre les fichiers et la
/// validation** : l'écrivain meurt au paquet 2, ses fichiers du plein texte
/// déjà synchronisés et marqués, la transaction des lignes pas encore validée.
/// Le repreneur trouve un dossier en avance sur la base, le jette, le rebâtit
/// depuis les lignes, finit la synchronisation ; ses comptes — plein texte
/// compris — sont ceux d'un témoin sans arrêt, lui aussi en fichiers.
#[test]
#[ignore]
fn en_fichiers_un_arret_entre_les_fichiers_et_la_validation_se_rebatit_aux_bons_comptes() {
    FICHIERS.with(|f| f.set(true));
    let dossier = dossier_sur_disque("fichiers-repris");
    let base = dossier.join("base.rag3db");
    let (statut, sortie) = lancer("ecrivain", &base, true);
    use std::os::unix::process::ExitStatusExt;
    assert_eq!(statut.signal(), Some(9), "mort par SIGKILL au paquet {PAQUET_TUE} :\n{sortie}");
    assert!(journal(&dossier).iter().any(|(_, t)| *t > 0), "journal vide : la mort ne prouverait rien");
    assert!(dossier.join("base.rag3db.fts").exists(), "le dossier du plein texte existe après la mort");

    let (statut, sortie) = lancer("repreneur", &base, false);
    assert!(statut.success(), "la base rouvre, et la reprise va au bout :\n{sortie}");
    assert!(sortie.contains("dossier jeté, rebâti depuis les lignes"), "le dossier en avance est reconnu :\n{sortie}");
    let repris = comptes_rendus("repreneur", &sortie);

    let temoin_dossier = dossier_sur_disque("fichiers-temoin");
    let (statut, sortie) = lancer("temoin", &temoin_dossier.join("base.rag3db"), false);
    assert!(statut.success(), "le témoin va au bout :\n{sortie}");
    let temoin = comptes_rendus("temoin", &sortie);
    println!("▸ comptes repris : {repris:?}");
    assert!(temoin.get("nœuds Scope").copied().unwrap_or(0) >= 300, "{temoin:?}");
    assert_eq!(repris, temoin, "en fichiers, la reprise rend les comptes d'une passe sans arrêt, plein texte compris");
    let _ = std::fs::remove_dir_all(&dossier);
    let _ = std::fs::remove_dir_all(&temoin_dossier);
}

/// **Sans arrêt, rien n'est rebâti** : une passe complète en fichiers, puis
/// un processus neuf qui rouvre et resynchronise — la génération du dossier
/// est celle de la base, aucun rebâti, mêmes comptes.
#[test]
#[ignore]
fn en_fichiers_une_base_saine_se_rouvre_sans_rebatir() {
    FICHIERS.with(|f| f.set(true));
    let dossier = dossier_sur_disque("fichiers-sain");
    let base = dossier.join("base.rag3db");
    let (statut, sortie) = lancer("premier", &base, false);
    assert!(statut.success(), "{sortie}");
    let premier = comptes_rendus("premier", &sortie);
    let (statut, sortie) = lancer("second", &base, false);
    assert!(statut.success(), "{sortie}");
    assert!(!sortie.contains("rebâti depuis les lignes"), "une base saine ne se rebâtit pas :\n{sortie}");
    assert_eq!(comptes_rendus("second", &sortie), premier, "mêmes comptes après réouverture");
    let _ = std::fs::remove_dir_all(&dossier);
}

/// **La recherche aussi rebâtit** : après la même mort, un processus neuf qui
/// ne fait que chercher trouve un dossier en avance sur la base ; il le
/// rebâtit avant de répondre, et trouve ce que les paquets validés ont écrit.
#[test]
#[ignore]
fn en_fichiers_une_recherche_apres_un_arret_rebatit_avant_de_repondre() {
    FICHIERS.with(|f| f.set(true));
    let dossier = dossier_sur_disque("fichiers-chercheur");
    let base = dossier.join("base.rag3db");
    let (statut, sortie) = lancer("ecrivain", &base, true);
    use std::os::unix::process::ExitStatusExt;
    assert_eq!(statut.signal(), Some(9), "mort par SIGKILL au paquet {PAQUET_TUE} :\n{sortie}");
    let (statut, sortie) = lancer("chercheur", &base, false);
    assert!(statut.success(), "{sortie}");
    assert!(sortie.contains("rebâti depuis les lignes"), "la recherche rebâtit le dossier en avance :\n{sortie}");
    let trouves: usize = sortie.lines().find_map(|l| l.strip_prefix("TROUVES ")).and_then(|n| n.trim().parse().ok()).unwrap_or(0);
    assert!(trouves > 0, "f010 est dans un paquet validé (paquets 0 et 1 : f000 à f063) : au moins un résultat :\n{sortie}");
    let _ = std::fs::remove_dir_all(&dossier);
}

fn ligne(sortie: &str, prefixe: &str) -> String {
    sortie.lines().find_map(|l| l.strip_prefix(prefixe)).map(str::to_string).unwrap_or_else(|| panic!("pas de ligne « {prefixe} » :\n{sortie}"))
}

/// **Les trois états du rebâti, et son interruption.** Après la mort de
/// l'écrivain : un lecteur seul voit « à rebâtir » (en cours, 0 %) sans rien
/// jeter ; un écrivain qui fait un lot voit « en cours » avec un avancement
/// entre 0 et 100 % ; il meurt après ce lot (rebâti interrompu) ; un repreneur
/// recommence le rebâti, va au bout, et ses comptes sont ceux d'un témoin ;
/// un lecteur voit alors « prêt ».
#[test]
#[ignore]
fn en_fichiers_le_rebati_dit_son_etat_et_survit_a_son_interruption() {
    FICHIERS.with(|f| f.set(true));
    let dossier = dossier_sur_disque("fichiers-etats");
    let base = dossier.join("base.rag3db");
    use std::os::unix::process::ExitStatusExt;
    let (statut, sortie) = lancer("ecrivain", &base, true);
    assert_eq!(statut.signal(), Some(9), "{sortie}");

    // À rebâtir, vu d'un lecteur seul.
    let (statut, sortie) = lancer("lecteur-fts", &base, false);
    assert!(statut.success(), "{sortie}");
    assert_eq!(ligne(&sortie, "ETAT "), "Running Some(0)", "le lecteur calcule « à rebâtir » :\n{sortie}");
    assert_eq!(ligne(&sortie, "DOSSIER "), "true", "le lecteur n'a rien jeté");

    // En cours, avec avancement ; puis l'écrivain meurt après son lot.
    TUER_APRES_LOT.with(|f| f.set(true));
    let (statut, sortie) = lancer("un-lot", &base, false);
    TUER_APRES_LOT.with(|f| f.set(false));
    assert_eq!(statut.signal(), Some(9), "{sortie}");
    let etat = ligne(&sortie, "ETAT ");
    let pourcent: u8 = etat.split("Some(").nth(1).and_then(|r| r.split(')').next()).and_then(|n| n.parse().ok()).unwrap_or(0);
    assert!(etat.starts_with("Running") && pourcent > 0 && pourcent < 100, "en cours, entre 0 et 100 % : {etat}\n{sortie}");

    // Le repreneur recommence le rebâti interrompu et va au bout.
    let (statut, sortie) = lancer("repreneur", &base, false);
    assert!(statut.success(), "{sortie}");
    assert!(sortie.contains("rebâti interrompu"), "le rebâti interrompu est reconnu :\n{sortie}");
    let repris = comptes_rendus("repreneur", &sortie);
    let temoin_dossier = dossier_sur_disque("fichiers-etats-temoin");
    let (statut, sortie) = lancer("temoin", &temoin_dossier.join("base.rag3db"), false);
    assert!(statut.success(), "{sortie}");
    assert_eq!(repris, comptes_rendus("temoin", &sortie), "comptes d'une passe sans arrêt, plein texte compris");

    // Prêt.
    let (statut, sortie) = lancer("lecteur-fts", &base, false);
    assert!(statut.success(), "{sortie}");
    assert!(ligne(&sortie, "ETAT ").ends_with("None"), "plus rien à rebâtir :\n{sortie}");
    assert!(!ligne(&sortie, "ETAT ").starts_with("Running"), "le plein texte est prêt :\n{sortie}");
    let _ = std::fs::remove_dir_all(&dossier);
    let _ = std::fs::remove_dir_all(&temoin_dossier);
}

/// **Hors de la transaction par paquet, un arrêt entre les lignes et le plein
/// texte se détecte** (la génération promise) : l'écrivain, sans transaction,
/// meurt avant la dixième synchronisation du plein texte — des lignes déjà
/// validées, leur plein texte pas encore durable. Le repreneur trouve la
/// promesse non tenue, rebâtit, et ses comptes — plein texte compris — sont
/// ceux d'un témoin sans arrêt.
#[test]
#[ignore]
fn en_fichiers_hors_transaction_un_arret_entre_lignes_et_plein_texte_se_detecte() {
    FICHIERS.with(|f| f.set(true));
    let dossier = dossier_sur_disque("fichiers-promesse");
    let base = dossier.join("base.rag3db");
    TUER_AVANT_SYNC.with(|f| f.set(Some(10)));
    let (statut, sortie) = lancer_avec("ecrivain", &base, None, false, 1);
    TUER_AVANT_SYNC.with(|f| f.set(None));
    use std::os::unix::process::ExitStatusExt;
    assert_eq!(statut.signal(), Some(9), "mort au crochet :\n{sortie}");
    assert!(sortie.contains("SIGKILL avant la synchronisation 10"), "{sortie}");
    let (statut, sortie) = lancer_avec("repreneur", &base, None, false, 1);
    assert!(statut.success(), "{sortie}");
    assert!(sortie.contains("une génération promise n'a pas été tenue"), "la promesse non tenue est reconnue :\n{sortie}");
    let repris = comptes_rendus("repreneur", &sortie);
    let temoin_dossier = dossier_sur_disque("fichiers-promesse-temoin");
    let (statut, sortie) = lancer_avec("temoin", &temoin_dossier.join("base.rag3db"), None, false, 1);
    assert!(statut.success(), "{sortie}");
    assert_eq!(repris, comptes_rendus("temoin", &sortie), "hors transaction, la reprise rend les comptes d'une passe sans arrêt");
    let _ = std::fs::remove_dir_all(&dossier);
    let _ = std::fs::remove_dir_all(&temoin_dossier);
}

/// Le nombre de résultats d'une recherche par mots à `k`, par le chemin du
/// produit (résolution des décalages en lignes comprise).
fn rendus(catalog: &std::sync::Arc<std::sync::Mutex<Catalog>>, mot: &str, k: usize) -> usize {
    Catalog::rechercher(catalog, rag3weaver::code::SCOPE, mot, rag3weaver::search::SearchOptions {
        consistency: rag3weaver::search::Consistency::Immediate,
        signals: Some(rag3weaver::search::SearchSignals::BM25),
        limit: k,
        ..Default::default()
    })
    .expect("recherche")
    .results
    .len()
}

fn documents_et_lignes(catalog: &Catalog) -> (u64, i64) {
    let docs = catalog.fts_handle("Scope").expect("index des scopes").num_docs();
    let lignes = catalog.conn().execute("MATCH (n:Scope) RETURN count(n)").unwrap().rows[0][0].as_i64().unwrap();
    (docs, lignes)
}

/// **La garde des comptes** : un document du plein texte dont la ligne est
/// absente (fabriqué ici, à un décalage qu'aucune ligne ne porte) prend une
/// place dans les premiers résultats sans être rendu ; la synchronisation
/// suivante le voit (documents ≠ lignes), rebâtit l'index depuis les lignes,
/// et une recherche à k rend k. Dans les deux modes du plein texte.
fn la_garde_retire_un_document_sans_ligne(fichiers: bool) {
    FICHIERS.with(|f| f.set(fichiers));
    let dossier = dossier_sur_disque(if fichiers { "garde-fichiers" } else { "garde-blobs" });
    let base = dossier.join("base.rag3db");
    // En processus : la variable du mode passe par l'environnement du fils
    // seulement ; ici le catalogue se monte à la main.
    let mut catalog = catalogue(&base);
    if fichiers {
        drop(catalog);
        let conn = Rag3dbConnection::new(&base).expect("base");
        conn.execute(&format!("LOAD EXTENSION '{}/extension/vector/build/libvector.rag3db_extension'", racine_moteur())).unwrap();
        let config = CatalogConfig { name: Some("tx-arret".into()), embedding_dim: 16, ..Default::default() };
        catalog = Catalog::new(Box::new(conn), Box::new(HashEmbedder::new(16)), config);
        catalog.set_fts_storage(rag3weaver::fts_handle::FtsStorage::Files { base_path: format!("{}.fts", base.display()) });
        catalog.initialize().unwrap();
        register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
    }
    synchroniser(&mut catalog, false);
    let (docs, lignes) = documents_et_lignes(&catalog);
    assert_eq!(docs as i64, lignes, "avant le fantôme, un document par ligne");
    {
        let handle = catalog.fts_handle("Scope").unwrap();
        let texte = "pub ".repeat(200) + "fantomatique";
        rag3weaver::fts_handle::index_document(&handle, &[("content".to_string(), texte)], 9_000_000).unwrap();
        handle.commit().unwrap();
    }
    assert_eq!(documents_et_lignes(&catalog).0 as i64, lignes + 1, "le fantôme compte");
    let catalog = std::sync::Arc::new(std::sync::Mutex::new(catalog));
    let avant = rendus(&catalog, "pub", 10);
    println!("▸ {} : « pub » à k = 10 avec le fantôme : {avant} rendus", if fichiers { "fichiers" } else { "blobs" });
    assert_eq!(rendus(&catalog, "fantomatique", 10), 0, "un document sans ligne n'est jamais rendu");

    synchroniser(&mut catalog.lock().unwrap(), false);
    let (docs, lignes) = documents_et_lignes(&catalog.lock().unwrap());
    assert_eq!(docs as i64, lignes, "la garde a rebâti l'index : un document par ligne");
    assert_eq!(rendus(&catalog, "pub", 10), 10, "une recherche à k rend k");
    assert_eq!(rendus(&catalog, "fantomatique", 10), 0);
    drop(catalog);
    let _ = std::fs::remove_dir_all(&dossier);
}

#[test]
#[ignore]
fn la_garde_des_comptes_retire_un_document_sans_ligne_en_fichiers() {
    la_garde_retire_un_document_sans_ligne(true);
}

#[test]
#[ignore]
fn la_garde_des_comptes_retire_un_document_sans_ligne_en_blobs() {
    la_garde_retire_un_document_sans_ligne(false);
}

/// **À la relance, une ligne refaite remplace le document de son décalage**
/// (`upsert_document`) au lieu d'en ajouter un second : un fantôme posé au
/// décalage que prendra la prochaine ligne est remplacé par elle, sans que la
/// garde intervienne (ingestion directe, hors synchronisation).
#[test]
#[ignore]
fn a_la_relance_la_ligne_refaite_remplace_le_document_de_son_decalage() {
    let dossier = dossier_sur_disque("relance-decalage");
    let base = dossier.join("base.rag3db");
    let mut catalog = catalogue(&base);
    synchroniser(&mut catalog, false);
    let (_, lignes) = documents_et_lignes(&catalog);
    {
        let handle = catalog.fts_handle("Scope").unwrap();
        rag3weaver::fts_handle::index_document(&handle, &[("content".to_string(), "fantomatique".to_string())], lignes as u64).unwrap();
        handle.commit().unwrap();
    }
    let analyse = rag3weaver::code::analyze("/neuf", vec![("src/neuf.rs".to_string(), "pub fn neuf_unique() -> u32 { 1 }\n".to_string())]);
    catalog.ingest_code(&analyse).expect("une ligne neuve");
    let decalage: i64 = catalog
        .conn()
        .execute("MATCH (n:Scope) WHERE n.name = 'neuf_unique' RETURN OFFSET(id(n))")
        .unwrap()
        .rows[0][0]
        .as_i64()
        .unwrap();
    let (docs, lignes) = documents_et_lignes(&catalog);
    println!("▸ la ligne neuve a pris le décalage {decalage} (le fantôme était au décalage {}) ; {docs} documents, {lignes} lignes", lignes - 1);
    let catalog = std::sync::Arc::new(std::sync::Mutex::new(catalog));
    assert_eq!(rendus(&catalog, "fantomatique", 10), 0, "le fantôme n'est plus là");
    assert_eq!(docs as i64, lignes, "un document par ligne : remplacé, pas ajouté");
    drop(catalog);
    let _ = std::fs::remove_dir_all(&dossier);
}

fn catalogue_fichiers(base: &Path) -> Catalog {
    let conn = Rag3dbConnection::new(base).expect("base");
    conn.execute(&format!("LOAD EXTENSION '{}/extension/vector/build/libvector.rag3db_extension'", racine_moteur())).unwrap();
    let config = CatalogConfig { name: Some("tx-arret".into()), embedding_dim: 16, ..Default::default() };
    let mut catalog = Catalog::new(Box::new(conn), Box::new(HashEmbedder::new(16)), config);
    catalog.set_fts_storage(rag3weaver::fts_handle::FtsStorage::Files { base_path: format!("{}.fts", base.display()) });
    catalog.initialize().unwrap();
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
    catalog
}

/// **Une marque qui ne s'écrit pas ne laisse jamais un index disparu** : la
/// base refuse toute écriture (catalogue empoisonné) ; la garde des comptes
/// trouve un écart, échoue à poser la marque, le dit, et ne touche pas à
/// l'index ; l'ouverture d'un dossier en désaccord échoue à marquer, n'ouvre
/// pas l'index et garde le dossier.
#[test]
#[ignore]
fn une_marque_qui_echoue_ne_laisse_jamais_un_index_sans_marque() {
    let dossier = dossier_sur_disque("marque-echoue");
    let base = dossier.join("base.rag3db");
    let fts = PathBuf::from(format!("{}.fts", base.display()));
    {
        let mut catalog = catalogue_fichiers(&base);
        synchroniser(&mut catalog, false);
        let handle = catalog.fts_handle("Scope").unwrap();
        rag3weaver::fts_handle::index_document(&handle, &[("content".to_string(), "fantomatique".to_string())], 9_000_000).unwrap();
        handle.commit().unwrap();
        drop(handle);
        // La garde, base empoisonnée : erreur dite, index gardé.
        catalog.poison("test : la base refuse toute écriture");
        let garde = catalog.check_fts_counts();
        assert!(garde.is_err(), "la marque n'a pas pu s'écrire : la garde échoue au lieu de jeter l'index : {garde:?}");
        assert!(catalog.fts_handle("Scope").is_some(), "l'index n'a pas été retiré");
        assert!(fts.exists(), "le dossier du plein texte est là");
    }
    // L'ouverture d'un dossier en désaccord, base empoisonnée.
    let dir_scope = std::fs::read_dir(&fts).unwrap().flatten().find(|e| e.file_name().to_string_lossy().contains("Scope")).expect("dossier de Scope").path();
    std::fs::remove_file(dir_scope.join("_generation")).expect("la génération du dossier");
    let mut catalog = catalogue_fichiers(&base);
    catalog.poison("test : la base refuse toute écriture");
    catalog.open_fts_files();
    assert!(catalog.fts_handle("Scope").is_none(), "sans marque posée, l'index n'est pas ouvert");
    assert!(dir_scope.exists(), "sans marque posée, le dossier n'est pas jeté");
    drop(catalog);
    let _ = std::fs::remove_dir_all(&dossier);
}

/// **Le témoin produit de la fuite de pages** (10 octobre 2026) : une mort
/// pendant un COPY journalisé, puis la réouverture qui le rejoue, puis la
/// recherche par mot et par vecteur.
///
/// Le crochet `RAG3WEAVER_TEST_COPY_JOURNALISE` coupe le point de reprise
/// forcé dans les deux stockages, et lève le seuil du journal. En fichiers,
/// le COPY journalisé est le défaut du moteur depuis ff9bad960 ; avant, sans
/// le crochet, rien n'était journalisé. En blobs, le produit force toujours
/// le point de reprise : ce cas éprouve le moteur hors du chemin du produit.
///
/// L'écrivain valide le premier paquet (150 000 scopes, un groupe plein,
/// au journal) et meurt au second. Le repreneur rouvre, rejoue, va au bout ;
/// ses comptes sont ceux d'un témoin sans arrêt, et la fonction `g5_7` du
/// premier paquet se retrouve par son nom et par son texte. La taille des
/// deux bases est dite, pas jugée : 559 pages perdues pour 200 000 lignes
/// avant le correctif, soit quelques Mio sur une base bien plus grosse.
fn mort_pendant_un_copy_journalise(fichiers: bool) {
    FICHIERS.with(|f| f.set(fichiers));
    FUITE.with(|f| f.set(true));
    let cas = if fichiers { "fuite-fichiers" } else { "fuite-blobs" };
    let dossier = dossier_sur_disque(cas);
    let base = dossier.join("base.rag3db");
    let t = std::time::Instant::now();
    let (statut, sortie) = lancer_avec("ecrivain", &base, Some(1), true, 1);
    use std::os::unix::process::ExitStatusExt;
    assert_eq!(statut.signal(), Some(9), "l'écrivain meurt au paquet 1 :\n{}", sortie.chars().take(3000).collect::<String>());
    let wal = journal(&dossier);
    println!("▸ {cas} : écrivain mort en {:.0?}, journal {wal:?}", t.elapsed());
    // Le premier paquet (150 000 lignes) doit être au journal pour que la
    // réouverture le rejoue : des dizaines de Mio. Un journal de quelques
    // centaines d'octets veut dire qu'un point de reprise l'a replié, et le
    // test ne prouverait rien (151 octets au premier essai, le 10 octobre).
    let octets: u64 = wal.iter().map(|(_, t)| *t).sum();
    assert!(octets >= 10 << 20, "journal de {octets} octets : le premier paquet n'y est pas, le test ne prouverait rien ({wal:?})");

    let (statut, sortie) = lancer_avec("repreneur", &base, None, true, 1);
    assert!(statut.success(), "la base rouvre et la reprise va au bout :\n{}", sortie.chars().take(3000).collect::<String>());
    let repris = comptes_rendus("repreneur", &sortie);
    let trouve = ligne(&sortie, "TROUVE repreneur ");

    let temoin_dossier = dossier_sur_disque(&format!("{cas}-temoin"));
    let temoin_base = temoin_dossier.join("base.rag3db");
    let (statut, sortie) = lancer_avec("temoin", &temoin_base, None, true, 1);
    assert!(statut.success(), "le témoin va au bout :\n{}", sortie.chars().take(3000).collect::<String>());
    let temoin = comptes_rendus("temoin", &sortie);
    let replis = ligne(&sortie, "REPLIS_DU_JOURNAL temoin ");

    let taille = |p: &Path| std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);
    println!(
        "▸ {cas} : {:.0?} en tout ; base reprise {} octets, témoin {} octets ; {trouve} ; replis du témoin {replis}",
        t.elapsed(),
        taille(&base),
        taille(&temoin_base)
    );
    assert!(temoin.get("nœuds Scope").copied().unwrap_or(0) >= 150_000, "le témoin a indexé le corpus : {temoin:?}");
    assert!(replis.contains("\"0\"") || replis.contains("(0)"), "aucun COPY du témoin ne s'est replié sur un point de reprise : {replis}");
    assert_eq!(repris, temoin, "la reprise rend les comptes d'une passe sans arrêt");
    assert_eq!(trouve, "mot=true vecteur=true", "g5_7, du paquet rejoué, se retrouve par mot et par vecteur");
    let _ = std::fs::remove_dir_all(&dossier);
    let _ = std::fs::remove_dir_all(&temoin_dossier);
}

#[test]
#[ignore]
fn en_fichiers_une_mort_pendant_un_copy_journalise_se_rejoue_et_se_retrouve() {
    mort_pendant_un_copy_journalise(true);
}

#[test]
#[ignore]
fn en_blobs_une_mort_pendant_un_copy_journalise_se_rejoue_et_se_retrouve() {
    mort_pendant_un_copy_journalise(false);
}
