//! **Sonde : que donne la réouverture d'une base laissée « à rouvrir » ?**
//!
//! Pas un test — une sonde, qui ne juge rien et imprime ce qu'elle trouve. Elle
//! existe parce qu'une base réelle dans cet état est ce que je n'ai pas su
//! fabriquer : trois conditions essayées (repos, charge d'une ingestion de
//! dépôt, seize instances simultanées) et aucune ne l'a produite ; un tampon
//! étroit produit une **autre** panne.
//!
//! La session embarquements en a laissé deux, gardées telles quelles après un
//! point de reprise échoué (« buffer pool is full… must be closed and reopened »).
//!
//! ```sh
//! SONDE_BASE=~/.cache/rag3weaver-build/copie-base-echouee-mem-tampon4g/estimate.rag3db \
//!   cargo test --features rag3db-native,… --test sonde_reouverture -- --include-ignored --nocapture
//! ```
//!
//! **Sur une copie, jamais sur l'originale** : la réouverture rejoue le journal,
//! donc elle écrit. La copie se fait par reflink (`cp --reflink=always`), qui ne
//! coûte ni temps ni place sur btrfs.
//!
//! Ce qu'elle imprime, dans l'ordre et sans s'arrêter au premier échec — chaque
//! étage dit quelque chose que le suivant ne dit pas :
//!
//! 1. `OUVERTURE=` — la connexion seule : le rejeu du journal passe-t-il ?
//! 2. `EXTENSION=` — le chargement de l'extension vectorielle ;
//! 3. `LIGNES=` — ce que la base contient, par le Cypher le plus nu ;
//! 4. `INDEX_VUS=` — ce que `SHOW_INDEXES` rend ;
//! 5. `CATALOGUE=` — le montage par le chemin du produit, avec sa réparation
//!    d'index détaché.

#![cfg(feature = "rag3db-native")]

use std::collections::HashMap;
use std::path::Path;

use rag3weaver::embedder::MockEmbedder;
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

fn racine_moteur() -> String {
    std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .expect("racine du dépôt")
            .display()
            .to_string()
    })
}

#[test]
#[ignore = "sonde : demande SONDE_BASE, et écrit dans la base désignée"]
fn que_donne_la_reouverture() {
    let Ok(base) = std::env::var("SONDE_BASE") else {
        println!("SONDE=non jouée — SONDE_BASE n'est pas posée (ce n'est pas « rien à signaler »)");
        return;
    };
    let base = shellexpand(&base);
    println!("▸ base : {base}");
    for suffixe in ["", ".wal", ".shadow"] {
        let chemin = format!("{base}{suffixe}");
        match std::fs::metadata(&chemin) {
            Ok(m) => println!("▸   {chemin} — {} octets", m.len()),
            Err(e) => println!("▸   {chemin} — absent ({e})"),
        }
    }

    let ext = format!("{}/extension/vector/build/libvector.rag3db_extension", racine_moteur());
    println!("▸ extension attendue : {ext} ({})", if Path::new(&ext).exists() { "présente" } else { "ABSENTE" });

    // ── 1. La connexion seule : le rejeu du journal ──────────────────────
    let conn = match std::panic::catch_unwind(|| Rag3dbConnection::new(&base)) {
        Ok(Ok(c)) => {
            println!("OUVERTURE=ok");
            c
        }
        Ok(Err(e)) => {
            println!("OUVERTURE=erreur: {e}");
            return;
        }
        Err(_) => {
            println!("OUVERTURE=panique");
            return;
        }
    };
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);

    // ── 2. L'extension ───────────────────────────────────────────────────
    match boxed.execute(&format!("LOAD EXTENSION '{ext}'")) {
        Ok(_) => println!("EXTENSION=ok"),
        Err(e) => println!("EXTENSION=erreur: {e}"),
    }

    // ── 3. Ce que la base contient, par le Cypher le plus nu ─────────────
    for entite in ["File", "Scope", "Symbol"] {
        match boxed.execute(&format!("MATCH (n:{entite}) RETURN count(n)")) {
            Ok(r) => println!(
                "LIGNES={entite} {}",
                r.rows.first().and_then(|l| l.first()).and_then(|v| v.as_i64()).unwrap_or(-1)
            ),
            Err(e) => println!("LIGNES={entite} erreur: {e}"),
        }
    }

    // ── 4. Les index au catalogue ────────────────────────────────────────
    match boxed.execute("CALL SHOW_INDEXES() RETURN *") {
        Ok(r) => {
            for l in &r.rows {
                let cells: Vec<String> = l
                    .iter()
                    .map(|c| c.as_str().map(|s| s.to_string()).unwrap_or_else(|| format!("{c:?}")))
                    .collect();
                println!("INDEX_VUS={}", cells.join("/"));
            }
            if r.rows.is_empty() {
                println!("INDEX_VUS=(aucun)");
            }
        }
        Err(e) => println!("INDEX_VUS=erreur: {e}"),
    }

    // ── 5. Le chemin du produit ──────────────────────────────────────────
    //
    // Une seconde connexion, parce que le catalogue prend la sienne. Et le
    // montage **écrit** : étapes 2 à 4 d'`initialize` reposent les DDL, et la
    // sonde d'index détaché rebâtit ce qu'elle trouve en retard. C'est bien ce
    // qu'on veut savoir — mais c'est pourquoi on travaille sur une copie.
    let montage = std::panic::catch_unwind(|| {
        let conn = Rag3dbConnection::new(&base).map_err(|e| e.to_string())?;
        let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
        boxed.execute(&format!("LOAD EXTENSION '{ext}'")).map_err(|e| e.to_string())?;
        let config = CatalogConfig {
            name: Some("sonde".into()),
            entities: HashMap::new(),
            relations: HashMap::new(),
            embedding_dim: 64,
            ..Default::default()
        };
        let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(64)), config);
        catalog.initialize().map_err(|e| e.to_string())?;
        Ok::<(), String>(())
    });
    match montage {
        Ok(Ok(())) => println!("CATALOGUE=ok"),
        Ok(Err(e)) => println!("CATALOGUE=erreur: {e}"),
        Err(_) => println!("CATALOGUE=panique"),
    }
}

/// `~` en tête, et rien d'autre : une sonde ne mérite pas une dépendance.
fn shellexpand(s: &str) -> String {
    match s.strip_prefix("~/") {
        Some(reste) => format!("{}/{reste}", std::env::var("HOME").unwrap_or_default()),
        None => s.to_string(),
    }
}
