//! Mesure : **le graphe dépend-il de la taille du paquet ?** (3 octobre 2026).
//!
//! Le dépôt entier compte 819 808 relations indexé d'un seul tenant, contre
//! 438 104 par paquets de 64. Ici, la même source (`src/` de rag3weaver, qu'on
//! sait juger) est indexée des deux façons. L'écart est classé par relation,
//! dans le même fichier ou entre fichiers, et par nombre de définisseurs du
//! nom visé. Les arêtes de l'écart sont écrites dans
//! `~/.cache/rag3weaver-build/graphe-paquets-{seul,paquets}.tsv`, pour la
//! relecture.
//!
//! Affichée, pas affirmée : hors de la batterie.
//!
//! Run with: ./run_e2e.sh --test e2e_graphe_et_paquets
#![cfg(all(feature = "rag3db-native", feature = "code"))]

use std::collections::{BTreeMap, BTreeSet, HashMap};

use rag3weaver::code::{default_scope_chunking, register_code_schema};
use rag3weaver::code_sync::{sync_source, SourceSyncOptions};
use rag3weaver::code_tools::WorkingTree;
use rag3weaver::connection::{CypherValue, DbConnection};
use rag3weaver::disponibilite::Disponibilites;
use rag3weaver::embedder::HashEmbedder;
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

/// **Le chemin du moteur ne se dérive jamais du chemin du code.** `RAG3DB_ROOT`
/// d'abord, le manifeste en repli, et l'absence **dite** — la règle du journal
/// (« Méthode », piège 6), écrite après quatre sites trouvés le même jour.
///
/// Deux de plus le 10 octobre 2026 — `e2e_graphe_et_paquets` et
/// `sonde_sens_des_relations` —, trouvés en rejouant le grep du motif par le
/// **chemin** après un rebase. Le journal dit de le rejouer pour cette raison
/// exacte : un nouveau site arrive sur master sans rapport avec le précédent.
/// Depuis un worktree, le code vit ici et l'extension est bâtie dans l'arbre
/// qui a lancé cmake : un chemin dérivé de `CARGO_MANIFEST_DIR` cherche donc la
/// bibliothèque là où elle n'est pas.
fn extension_vectorielle() -> String {
    let racine = std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("racine du dépôt")
            .display()
            .to_string()
    });
    let chemin = format!("{racine}/extension/vector/build/libvector.rag3db_extension");
    assert!(
        std::path::Path::new(&chemin).exists(),
        "l'extension vectorielle est absente : {chemin}\n\
         Posez RAG3DB_ROOT sur l'arbre où le moteur est bâti (depuis un worktree, \
         c'est l'arbre principal), ou bâtissez-la ici — run_e2e.sh le fait et \
         exporte la variable."
    );
    chemin
}

fn catalogue() -> Catalog {
    let conn = Rag3dbConnection::in_memory().expect("base en mémoire");
    conn.execute(&format!("LOAD EXTENSION '{}'", extension_vectorielle())).unwrap();
    let config = CatalogConfig { name: Some("graphe-paquets".into()), embedding_dim: 64, ..Default::default() };
    let mut catalog = Catalog::new(Box::new(conn), Box::new(HashEmbedder::new(64)), config);
    catalog.initialize().unwrap();
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
    catalog
}

fn texte(v: &CypherValue) -> String {
    v.as_str().map(str::to_string).unwrap_or_else(|| format!("{v:?}"))
}

/// Les arêtes, par (relation, bout de départ, bout d'arrivée), avec leur
/// multiplicité.
fn aretes(catalog: &Catalog) -> BTreeMap<(String, String, String), usize> {
    let cle = |label: &str| match label {
        "Scope" => "key",
        "File" => "path",
        _ => "name",
    };
    let mut out = BTreeMap::new();
    for (rel, from, to) in rag3weaver::code::RELATIONS {
        let rows = catalog
            .execute_raw(&format!("MATCH (a:{from})-[r:{rel}]->(b:{to}) RETURN a.{}, b.{}", cle(from), cle(to)))
            .unwrap()
            .rows;
        for r in rows {
            *out.entry((rel.to_string(), texte(&r[0]), texte(&r[1]))).or_insert(0) += 1;
        }
    }
    out
}

/// Le fichier d'une clé de scope (`source#chemin#nom:genre`).
fn fichier(cle: &str) -> &str {
    cle.split('#').nth(1).unwrap_or("")
}

/// Le nom d'une clé de scope, sans parent ni genre (`Parent.nom:method` → `nom`).
fn nom(cle: &str) -> String {
    let dernier = cle.split('#').nth(2).unwrap_or("");
    let sans_genre = dernier.split(':').next().unwrap_or("");
    sans_genre.rsplit('.').next().unwrap_or("").to_string()
}

#[test]
#[ignore]
fn le_graphe_selon_la_taille_du_paquet() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let indexer = |batch_files: usize| {
        let mut catalog = catalogue();
        let options = SourceSyncOptions { batch_files, exige: Disponibilites::RECHERCHE_TEXTE, ..Default::default() };
        let r = sync_source(&mut catalog, &WorkingTree::new(&src), &options, &mut |_| {}).unwrap();
        eprintln!("[paquets de {batch_files}] {} fichiers, {} scopes, {} relations au rapport", r.files_ingested, r.scopes_written, r.relations);
        catalog
    };
    let par_paquets = indexer(64);
    let seul = indexer(100_000);
    let (a, b) = (aretes(&par_paquets), aretes(&seul));

    // Les définisseurs de chaque nom, dans l'index d'un seul tenant.
    let mut definisseurs: HashMap<String, usize> = HashMap::new();
    for r in seul.execute_raw("MATCH (s:Scope) RETURN s.name").unwrap().rows {
        *definisseurs.entry(texte(&r[0])).or_insert(0) += 1;
    }

    let cache = std::path::PathBuf::from(std::env::var("HOME").unwrap()).join(".cache/rag3weaver-build");
    std::fs::create_dir_all(&cache).unwrap();
    for (nom_cote, ici, la) in [("seul", &b, &a), ("paquets", &a, &b)] {
        let mut classes: BTreeMap<(String, &'static str, &'static str), usize> = BTreeMap::new();
        let mut lignes = Vec::new();
        for ((rel, de, vers), n) in ici {
            let n_la = la.get(&(rel.clone(), de.clone(), vers.clone())).copied().unwrap_or(0);
            if *n <= n_la {
                continue;
            }
            let lieu = if !vers.contains('#') {
                "vers un nœud non-scope"
            } else if fichier(de) == fichier(vers) {
                "même fichier"
            } else {
                "entre fichiers"
            };
            let homonymes = match definisseurs.get(&nom(vers)).copied().unwrap_or(0) {
                _ if !vers.contains('#') => "-",
                0 | 1 => "nom unique",
                _ => "nom ambigu",
            };
            *classes.entry((rel.clone(), lieu, homonymes)).or_insert(0) += n - n_la;
            lignes.push(format!("{rel}\t{de}\t{vers}\t{lieu}\t{homonymes}\t{}", n - n_la));
        }
        let chemin = cache.join(format!("graphe-paquets-{nom_cote}.tsv"));
        std::fs::write(&chemin, lignes.join("\n")).unwrap();
        let total: usize = classes.values().sum();
        eprintln!("\nSEULEMENT EN « {nom_cote} » : {total} arêtes ({})", chemin.display());
        for ((rel, lieu, homonymes), n) in &classes {
            eprintln!("  {rel:<14} {lieu:<22} {homonymes:<11} {n}");
        }
    }
    let noms: BTreeSet<&str> = a.keys().map(|(r, _, _)| r.as_str()).collect();
    for rel in noms {
        let na: usize = a.iter().filter(|((r, _, _), _)| r == rel).map(|(_, n)| n).sum();
        let nb: usize = b.iter().filter(|((r, _, _), _)| r == rel).map(|(_, n)| n).sum();
        eprintln!("[{rel}] paquets {na} — seul {nb}");
    }
}

/// **Le graphe ne dépend pas de la taille du paquet** (décision du 4 octobre
/// 2026, journal : « un seul résolveur entre fichiers »). La même source,
/// indexée par paquets d'un fichier, de 64 et d'un seul tenant, donne les
/// mêmes arêtes, avec leur multiplicité. Tout lien entre fichiers passe par
/// la voie des `Symbol`, qui s'abstient sur un nom ambigu ; l'analyseur ne
/// résout que dans le fichier.
#[test]
#[ignore]
fn le_graphe_ne_depend_pas_de_la_taille_du_paquet() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/dataflow");
    let indexer = |batch_files: usize| {
        let mut catalog = catalogue();
        let options = SourceSyncOptions { batch_files, exige: Disponibilites::RECHERCHE_TEXTE, ..Default::default() };
        sync_source(&mut catalog, &WorkingTree::new(&src), &options, &mut |_| {}).unwrap();
        aretes(&catalog)
    };
    let un = indexer(1);
    for (taille, graphe) in [(64, indexer(64)), (100_000, indexer(100_000))] {
        let en_plus: Vec<_> = graphe.iter().filter(|(k, n)| un.get(*k) != Some(n)).take(10).collect();
        let en_moins: Vec<_> = un.iter().filter(|(k, n)| graphe.get(*k) != Some(n)).take(10).collect();
        assert!(
            en_plus.is_empty() && en_moins.is_empty(),
            "paquets de {taille} contre paquets d'un fichier : {} arêtes contre {}\nen plus : {en_plus:?}\nen moins : {en_moins:?}",
            graphe.values().sum::<usize>(),
            un.values().sum::<usize>()
        );
    }
}
