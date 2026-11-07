//! **Sonde : le coût du plein texte selon le nombre de termes** (7 novembre 2026, ticket « Le plein
//! texte explose sur une requête longue »). Pas un test : une sonde, qui ne juge rien et imprime
//! une table, pour que la borne du nœud (K termes, L caractères) sorte d'une mesure.
//!
//! Pour N termes distincts (1 à 197, tirés d'une page de prose du dépôt), la durée de lucivy seul
//! sur la poignée plein texte de la cible, médiane de trois essais (un seul au-delà de 5 s) :
//! les mots comme le nœud les envoie (`Parse`), les mots un par un (`ContainsSplit`, distance 1
//! puis 0) et la requête entière (`Contains`, distance 1 puis 0).
//!
//! ```sh
//! cp -a --reflink=always <base>.rag3db <copie>.rag3db
//! SONDE_CONFIG=~/.config/rag3weaver/<backend> SONDE_GABARIT=code SONDE_ENTITE=Scope \
//!   [SONDE_MANIFESTE=<manifeste de l'hôte, s'il n'est pas sous templates/backends/<gabarit>/>] \
//!   SONDE_BASE=<copie>.rag3db poste mesure scripts/cargo_magasin.sh test \
//!   --features rag3db-native,code,daemon --test sonde_requete_longue -- --include-ignored --nocapture
//! ```
//!
//! **Sur une copie, jamais sur l'originale** : l'ouverture écrit.

#![cfg(all(feature = "rag3db-native", feature = "daemon"))]

use std::path::{Path, PathBuf};
use std::time::Instant;

use rag3weaver::backend::PreparedBackend;
use rag3weaver::search::{build_bm25_query, BM25Mode};
use rag3weaver::Rag3dbConnection;
use serde_json::{json, Value};

/// Une page de prose du dépôt : des mots de prompt, pas des identifiants.
const PROSE: &str = include_str!("../docs/06-novembre-2026-orchestration/01-plan-du-week-end.md");
const PALIERS: &[usize] = &[1, 4, 8, 16, 32, 64, 128, 197];

fn copier(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).unwrap();
    for e in std::fs::read_dir(src).unwrap() {
        let e = e.unwrap();
        let cible = dst.join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            copier(&e.path(), &cible);
        } else {
            std::fs::copy(e.path(), &cible).unwrap();
        }
    }
}

/// Les mots distincts de la prose, de trois lettres au moins, dans leur ordre d'apparition.
fn termes() -> Vec<String> {
    let mut vus = std::collections::HashSet::new();
    PROSE
        .split(|c: char| !c.is_alphanumeric())
        .filter(|m| m.chars().count() >= 3)
        .map(str::to_lowercase)
        .filter(|m| vus.insert(m.clone()))
        .collect()
}

#[test]
#[ignore = "sonde : demande SONDE_CONFIG, SONDE_GABARIT, SONDE_ENTITE et SONDE_BASE (une copie)"]
fn le_cout_du_plein_texte_selon_le_nombre_de_termes() {
    let config = PathBuf::from(std::env::var("SONDE_CONFIG").expect("SONDE_CONFIG : le dossier de config du backend"));
    let base = PathBuf::from(std::env::var("SONDE_BASE").expect("SONDE_BASE : une copie de sa base"));
    let gabarit = std::env::var("SONDE_GABARIT").unwrap_or_else(|_| "code".into());
    let entite = std::env::var("SONDE_ENTITE").unwrap_or_else(|_| "Scope".into());

    // Les gabarits de cette branche ; de l'hôte, ce qui désigne sa base.
    let tmp = tempfile::tempdir().unwrap();
    let gabarits = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates");
    copier(&gabarits.join("tools"), &tmp.path().join("templates/tools"));
    // Tous les gabarits de backend : l'un emprunte les graphes d'un autre (memory lit
    // `../notebook/graphs/`).
    copier(&gabarits.join("backends"), &tmp.path().join("templates/backends"));
    // Le manifeste de l'hôte : nommé (`SONDE_MANIFESTE`, la mémoire longue range le sien sous
    // `backends/memory/`), sinon cherché sous `templates/backends/<gabarit>/`.
    let fichier_hote = match std::env::var("SONDE_MANIFESTE") {
        Ok(chemin) => PathBuf::from(chemin),
        Err(_) => {
            let dossier_hote = config.join("templates/backends").join(&gabarit);
            ["rag3db.json", "backend.json"]
                .iter()
                .map(|n| dossier_hote.join(n))
                .find(|p| p.is_file())
                .unwrap_or_else(|| panic!("le manifeste de l'hôte sous {}", dossier_hote.display()))
        }
    };
    let hote: Value = serde_json::from_slice(&std::fs::read(&fichier_hote).unwrap()).unwrap();
    let manifeste = tmp.path().join("templates/backends").join(&gabarit).join("backend.json");
    let mut m: Value = serde_json::from_slice(&std::fs::read(&manifeste).unwrap()).unwrap();
    for cle in ["name", "workspace", "embeddings", "vector_extension"] {
        if !hote[cle].is_null() {
            m[cle] = hote[cle].clone();
        }
    }
    if m["workspace"].is_object() {
        m["workspace"]["commands"] = json!("off");
    }
    // La mesure ne lit que le plein texte : pas de service creux à joindre.
    // `get_mut`, pas `m["models"]` : l'index mutable de serde_json pose `null` sur une clé absente.
    if let Some(modeles) = m.get_mut("models").and_then(Value::as_object_mut) {
        modeles.remove("sparse");
    }
    m["database"] = json!(base.to_string_lossy());
    if let Some(outils) = m.get_mut("tools").and_then(Value::as_object_mut) {
        outils.retain(|_, t| t["policy"]["write_files"] != json!(true) && t["policy"]["run_commands"] != json!(true));
    }
    std::fs::write(&manifeste, serde_json::to_vec_pretty(&m).unwrap()).unwrap();

    let prepared = PreparedBackend::load(&manifeste).expect("le manifeste charge");
    let (embarqueur, raison) = prepared.embedder_or_absent();
    let backend = prepared
        .open_with_reason(Box::new(Rag3dbConnection::new(&base).expect("la copie de la base")), embarqueur, raison)
        .expect("l'ouverture");

    let tous = termes();
    assert!(tous.len() >= 197, "la prose a {} termes distincts, il en faut 197", tous.len());
    let mut cat = backend.catalog.lock().unwrap();
    let cible = cat.resolve_search_target(&entite).unwrap_or_else(|e| panic!("la cible {entite} : {e}"));
    let poignee = cat
        .ensure_fts_handle(&cible.parent_table, &cible.bm25_fields, &rag3weaver::scope::fts_filter_fields())
        .unwrap_or_else(|| panic!("l'index plein texte de {}", cible.parent_table));
    let documents = poignee.num_docs();
    drop(cat);

    let mesurer = |requete: &str, mode: BM25Mode, distance: u8| -> String {
        let json = build_bm25_query(requete, &cible.bm25_fields, mode, distance);
        let config: lucivy_core::query::QueryConfig = serde_json::from_str(&json).expect("la requête lucivy");
        let mut durees = Vec::new();
        for _ in 0..3 {
            let t = Instant::now();
            rag3weaver::fts_handle::search_hits(&poignee, &config, 30, None).expect("lucivy");
            durees.push(t.elapsed().as_secs_f64() * 1000.0);
            if durees[0] > 5000.0 {
                break;
            }
        }
        durees.sort_by(|a, b| a.partial_cmp(b).unwrap());
        format!("{:.0}", durees[durees.len() / 2])
    };

    println!("base {} — cible {entite} ({}), {documents} documents", base.display(), cible.parent_table);
    println!("| termes | caractères | mots (Parse, le nœud) | mots un par un d=1 | mots un par un d=0 | entière d=1 (le nœud) | entière d=0 |");
    println!("|---|---|---|---|---|---|---|");
    for &n in PALIERS {
        let requete = tous[..n].join(" ");
        println!(
            "| {n} | {} | {} | {} | {} | {} | {} |",
            requete.chars().count(),
            mesurer(&requete, BM25Mode::Parse, 1),
            mesurer(&requete, BM25Mode::ContainsSplit, 1),
            mesurer(&requete, BM25Mode::ContainsSplit, 0),
            mesurer(&requete, BM25Mode::Contains, 1),
            mesurer(&requete, BM25Mode::Contains, 0),
        );
    }
}
