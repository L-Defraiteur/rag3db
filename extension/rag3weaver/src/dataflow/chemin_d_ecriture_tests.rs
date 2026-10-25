//! **Le résultat d'une indexation ne dépend pas du chemin d'écriture**
//! (ticket `docs/tickets/2026-10-18-les-relations-d-un-index-dependent-du-chemin-d-ecriture.md`,
//! 18 octobre 2026). Codeparsers a mesuré sur code-lourd : au seuil du COPY par
//! défaut, des `HAS_PARENT` (une méthode définie hors ligne vers sa classe) et
//! des `INHERITS_FROM` deviennent des `CONSUMES` sur le même couple ; sans
//! COPY, la relation fine est là. Ici le même petit dépôt, indexé deux fois :
//! tout par COPY (seuil 1) et tout par lots (seuil haut). Les arêtes, une à
//! une, doivent être les mêmes.

use crate::code::analyze;
use crate::embedder::HashEmbedder;
use crate::{Catalog, CatalogConfig, Rag3dbConnection};


fn depot() -> Vec<(String, String)> {
    vec![
        (
            "foo.h".into(),
            "class Base {\npublic:\n    virtual void run();\n};\n\nclass Foo : public Base {\npublic:\n    void bar();\n    int baz(int x);\n};\n\nclass Fille : public Foo {\npublic:\n    void bar();\n};\n".into(),
        ),
        (
            "foo.cpp".into(),
            "#include \"foo.h\"\n\nvoid Base::run() {}\n\nvoid Foo::bar() {\n    baz(1);\n}\n\nint Foo::baz(int x) {\n    return x + 1;\n}\n\nvoid Fille::bar() {\n    Foo::bar();\n}\n".into(),
        ),
    ]
}

/// Les vrais fichiers des paires perdues sur code-lourd (listes de
/// codeparsers) : `StructField.copy` → `StructField` (`HAS_PARENT`), et
/// `ErrorNode` → `TerminalNode` (`INHERITS_FROM`).
fn fichiers_reels() -> Vec<(String, String)> {
    let racine = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    fn parcourir(racine: &std::path::Path, dir: &str, out: &mut Vec<(String, String)>) {
        for e in std::fs::read_dir(racine.join(dir)).unwrap_or_else(|e| panic!("{dir} : {e}")) {
            let e = e.unwrap();
            let rel = format!("{dir}/{}", e.file_name().to_string_lossy());
            if e.file_type().unwrap().is_dir() {
                parcourir(racine, &rel, out);
            } else if rel.ends_with(".cpp") || rel.ends_with(".h") {
                out.push((rel.clone(), std::fs::read_to_string(racine.join(&rel)).unwrap_or_default()));
            }
        }
    }
    let mut out = Vec::new();
    for dir in ["src/common", "src/include/common", "third_party/antlr4_runtime/src/tree", "third_party/antlr4_runtime/src/atn"] {
        parcourir(&racine, dir, &mut out);
    }
    out.sort();
    out
}

/// Le premier index de ces fichiers par `sync_source` (le chemin de `index` :
/// Bulk, des paquets, les relations différées), un fichier par paquet.
fn aretes_par_la_synchronisation(seuil: usize) -> Vec<String> {
    let d = tempfile::tempdir().unwrap();
    let conn = Rag3dbConnection::new(&d.path().join("base.rag3db")).expect("la base");
    let mut cat = Catalog::new(
        Box::new(conn),
        Box::new(HashEmbedder::new(8)),
        CatalogConfig { allow_mock_embedder: true, embedding_dim: 8, ..Default::default() },
    );
    cat.initialize().unwrap();
    // Le seuil réglé sur ce catalogue, pas dans l'environnement du processus :
    // la variable faisait passer par lots les témoins voisins de la lib
    // (`copie_liens_tests`, faux vert du 19 octobre 2026).
    cat.set_link_copy_threshold(Some(seuil));
    crate::code::register_code_schema(&mut cat, crate::code::default_scope_chunking()).unwrap();
    let source = crate::code_tools::Snapshot::new("depot", fichiers_reels());
    let options = crate::code_sync::SourceSyncOptions { batch_files: 64, ..Default::default() };
    crate::code_sync::sync_source(&mut cat, &source, &options, &mut |_| {}).expect("la synchronisation");
    lire_les_aretes(&cat)
}

fn lire_les_aretes(cat: &Catalog) -> Vec<String> {
    let r = cat.execute_raw("MATCH (a)-[r]->(b) RETURN label(r), a._uuid, b._uuid").unwrap();
    let mut v: Vec<String> = r
        .rows
        .iter()
        .map(|l| format!("{} {} -> {}", l[0].as_str().unwrap_or("?"), l[1].as_str().unwrap_or("?"), l[2].as_str().unwrap_or("?")))
        .filter(|l| !l.contains("_Chunk") && !l.starts_with("Scope_CHUNKED_FROM"))
        .collect();
    v.sort();
    v
}

/// Ce qui n'est que d'un côté, en texte.
fn ecart(a: &[String], b: &[String]) -> (Vec<String>, Vec<String>) {
    (a.iter().filter(|x| !b.contains(x)).cloned().collect(), b.iter().filter(|x| !a.contains(x)).cloned().collect())
}

#[test]
fn les_aretes_du_premier_index_ne_dependent_pas_du_chemin_d_ecriture() {
    let par_lots = aretes_par_la_synchronisation(1_000_000_000);
    let par_copy = aretes_par_la_synchronisation(1);
    let compte = |v: &[String]| {
        let mut m = std::collections::BTreeMap::<String, usize>::new();
        for a in v {
            *m.entry(a.split(' ').next().unwrap_or("").to_string()).or_default() += 1;
        }
        m
    };
    eprintln!("[chemin] par lots : {:?}", compte(&par_lots));
    eprintln!("[chemin] par COPY : {:?}", compte(&par_copy));
    let (lots, copy) = ecart(&par_lots, &par_copy);
    assert!(lots.is_empty() && copy.is_empty(), "par lots seulement :\n{}\npar COPY seulement :\n{}", lots.join("\n"), copy.join("\n"));
    // Les arêtes se lisent par uuid : la présence des relations fines suffit
    // ici, l'égalité arête par arête est le témoin.
    assert!(par_lots.iter().any(|a| a.starts_with("HAS_PARENT ")), "des HAS_PARENT");
    assert!(par_lots.iter().any(|a| a.starts_with("INHERITS_FROM ")), "des INHERITS_FROM");
}

/// Les arêtes de la base, `relation de -> vers`, triées.
fn aretes(seuil: usize) -> Vec<String> {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let mut cat = Catalog::new(
        Box::new(conn),
        Box::new(HashEmbedder::new(8)),
        CatalogConfig { allow_mock_embedder: true, embedding_dim: 8, ..Default::default() },
    );
    cat.initialize().unwrap();
    // Le seuil réglé sur ce catalogue, pas dans l'environnement du processus :
    // la variable faisait passer par lots les témoins voisins de la lib
    // (`copie_liens_tests`, faux vert du 19 octobre 2026).
    cat.set_link_copy_threshold(Some(seuil));
    crate::code::register_code_schema(&mut cat, crate::code::default_scope_chunking()).unwrap();
    cat.ingest_code(&analyze("/depot", depot())).unwrap();
    let r = cat.execute_raw("MATCH (a)-[r]->(b) RETURN label(r), a._uuid, b._uuid").unwrap();
    let mut v: Vec<String> = r
        .rows
        .iter()
        .map(|l| format!("{} {} -> {}", l[0].as_str().unwrap_or("?"), l[1].as_str().unwrap_or("?"), l[2].as_str().unwrap_or("?")))
        .filter(|l| !l.contains("_Chunk") && !l.starts_with("Scope_CHUNKED_FROM"))
        .collect();
    v.sort();
    v
}

#[test]
fn les_aretes_d_un_index_ne_dependent_pas_du_chemin_d_ecriture() {
    let par_lots = aretes(1_000_000_000);
    let par_copy = aretes(1);
    let seulement_lots: Vec<&String> = par_lots.iter().filter(|a| !par_copy.contains(a)).collect();
    let seulement_copy: Vec<&String> = par_copy.iter().filter(|a| !par_lots.contains(a)).collect();
    assert!(
        seulement_lots.is_empty() && seulement_copy.is_empty(),
        "par lots seulement :\n{}\npar COPY seulement :\n{}",
        seulement_lots.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n"),
        seulement_copy.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n")
    );
    assert!(par_lots.iter().any(|a| a.starts_with("HAS_PARENT")), "le dépôt a bien des HAS_PARENT : {par_lots:#?}");
    assert!(par_lots.iter().any(|a| a.starts_with("INHERITS_FROM")), "et des INHERITS_FROM : {par_lots:#?}");
}

/// **Une paire, un genre, le plus fin** : une méthode hors ligne dont le corps
/// nomme aussi sa classe pousse `CONSUMES` et `HAS_PARENT` pour le même
/// couple ; une classe qui nomme sa base dans son corps, `CONSUMES` et
/// `INHERITS_FROM`. L'analyse n'en garde qu'un, la relation fine.
#[test]
fn une_paire_ne_garde_que_son_genre_le_plus_fin() {
    let a = analyze(
        "/depot",
        vec![
            ("types.h".into(), "class Base {};\nclass StructField : public Base {\npublic:\n    StructField copy() const;\n    Base* base;\n};\n".into()),
            ("types.cpp".into(), "#include \"types.h\"\n\nStructField StructField::copy() const {\n    return StructField();\n}\n".into()),
        ],
    );
    let mut par_paire = std::collections::BTreeMap::<(String, String), Vec<String>>::new();
    for (scope, nom, genre) in &a.pending {
        par_paire.entry((scope.clone(), nom.clone())).or_default().push(genre.clone());
    }
    assert!(par_paire.values().all(|g| g.len() == 1), "un genre par paire : {par_paire:#?}");
    let genre_de = |scope: &str, nom: &str| {
        par_paire.iter().find(|((s, n), _)| s.contains(scope) && n == nom).map(|(_, g)| g[0].clone())
    };
    assert_eq!(genre_de("copy", "StructField").as_deref(), Some("HAS_PARENT"), "{par_paire:#?}");
    assert_eq!(genre_de("StructField", "Base").as_deref(), Some("INHERITS_FROM"), "{par_paire:#?}");
}
