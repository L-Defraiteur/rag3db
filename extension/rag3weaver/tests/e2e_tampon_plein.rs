//! **Le tampon du moteur plein empoisonne la base** (5 octobre 2026).
//!
//! Un COPY refusé par « The buffer pool is full » après avoir réservé ses
//! lignes laisse la table fausse en mémoire jusqu'à la réouverture (défaut
//! du moteur, cœur C++). Le repli sur MERGE y aurait cherché ses clés et pu
//! écrire des doublons. Désormais ce refus empoisonne la base, comme un point
//! de reprise échoué : le message dit la cause et le réglage, la session
//! refuse tout ensuite, la fermeture se fait sans point de reprise, et une
//! réouverture avec un tampon suffisant reprend aux bons comptes.
//!
//! Un fils ouvre la base avec un tampon minuscule (`RAG3DB_BUFFER_POOL_SIZE`)
//! et charge par COPY plus de texte que le tampon n'en tient ; un second fils
//! la rouvre avec le tampon par défaut et refait le même chargement.
//!
//! ```bash
//! ./run_e2e.sh --test e2e_tampon_plein
//! ```
#![cfg(feature = "rag3db-native")]

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use rag3weaver::config::FieldType;
use rag3weaver::connection::CypherValue;
use rag3weaver::disponibilite::Disponibilites;
use rag3weaver::embedder::MockEmbedder;
use rag3weaver::search::SearchSignals;
use rag3weaver::{Catalog, CatalogConfig, EntityConfig, Rag3dbConnection, SimpleFieldDef};

const ROLE: &str = "TAMPON_PLEIN_ROLE";
const BASE: &str = "TAMPON_PLEIN_BASE";
const LIGNES: usize = 20_000;
const TAMPON_MINUSCULE: &str = "33554432"; // 32 Mio

fn doc() -> EntityConfig {
    let mut fields = HashMap::new();
    fields.insert("nom".to_string(), SimpleFieldDef { field_type: FieldType::String, is_title: true, ..Default::default() });
    fields.insert("texte".to_string(), SimpleFieldDef { field_type: FieldType::String, is_content: true, ..Default::default() });
    EntityConfig { fields, signals: SearchSignals::BM25, hashsafe: Some(vec!["nom".into()]), chunked: Some(false), ..Default::default() }
}

fn lignes() -> Vec<BTreeMap<String, CypherValue>> {
    (0..LIGNES)
        .map(|i| {
            BTreeMap::from([
                ("nom".to_string(), CypherValue::String(format!("d{i}"))),
                ("texte".to_string(), CypherValue::String(format!("{i} ").repeat(800))),
            ])
        })
        .collect()
}

fn catalogue(base: &str) -> Catalog {
    let conn = Rag3dbConnection::new(base).expect("ouvrir la base");
    let config = CatalogConfig { name: Some("tampon-plein".into()), embedding_dim: 4, ..Default::default() };
    let mut catalog = Catalog::new(Box::new(conn), Box::new(MockEmbedder::new(4)), config);
    catalog.initialize().expect("initialize");
    catalog.register_entity("Doc", doc()).expect("entité");
    catalog
}

#[test]
#[ignore]
fn role_enfant() {
    let (Ok(role), Ok(base)) = (std::env::var(ROLE), std::env::var(BASE)) else { return };
    let mut catalog = catalogue(&base);
    let r = catalog.ingest_entities_jusqu_a("Doc", lignes(), Disponibilites::RECHERCHE_TEXTE);
    match role.as_str() {
        "plein" => {
            println!("RESULTAT {:?}", r.as_ref().map(|r| (r.processed, r.failed, r.warnings.len())));
            if let Ok(r) = &r {
                for w in r.warnings.iter().take(5) {
                    println!("AVERTISSEMENT {w}");
                }
            }
            println!("RAISON {}", catalog.must_reopen().unwrap_or_default().replace('\n', " "));
            let ensuite = catalog.execute_raw("MATCH (d:Doc) RETURN count(d)");
            println!("ENSUITE {}", match ensuite {
                Ok(_) => "servie".to_string(),
                Err(e) => format!("refusée : {e}"),
            });
        }
        _ => {
            let r = r.expect("la reprise ingère");
            assert_eq!(r.failed, 0, "{:?}", r.warnings);
            let n = catalog.execute_raw("MATCH (d:Doc) RETURN count(d), count(DISTINCT d._uuid)").unwrap().rows;
            println!("COMPTES {:?}", n[0]);
        }
    }
}

fn lancer(role: &str, base: &PathBuf, tampon: Option<&str>) -> String {
    let mut cmd = std::process::Command::new(std::env::current_exe().unwrap());
    cmd.args(["--exact", "role_enfant", "--nocapture", "--ignored"]).env(ROLE, role).env(BASE, base);
    match tampon {
        Some(t) => cmd.env("RAG3DB_BUFFER_POOL_SIZE", t),
        None => cmd.env_remove("RAG3DB_BUFFER_POOL_SIZE"),
    };
    let s = cmd.output().expect("lancer le fils");
    let texte = format!("{}{}", String::from_utf8_lossy(&s.stdout), String::from_utf8_lossy(&s.stderr));
    assert!(s.status.success(), "le fils « {role} » a échoué ({:?}) :\n{}", s.status, texte.chars().take(4000).collect::<String>());
    texte
}

fn ligne<'a>(texte: &'a str, prefixe: &str) -> &'a str {
    texte.lines().find_map(|l| l.strip_prefix(prefixe)).unwrap_or_else(|| panic!("pas de « {prefixe} »"))
}

#[test]
#[ignore]
fn un_tampon_plein_empoisonne_la_base_et_la_reprise_rend_les_bons_comptes() {
    let dossier = PathBuf::from(std::env::var("HOME").unwrap()).join(format!(
        ".cache/rag3weaver-build/tampon-plein-{}-{}",
        std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    ));
    std::fs::create_dir_all(&dossier).unwrap();
    let base = dossier.join("base.rag3db");

    let plein = lancer("plein", &base, Some(TAMPON_MINUSCULE));
    let raison = ligne(&plein, "RAISON ");
    let ensuite = ligne(&plein, "ENSUITE ");
    println!("▸ résultat : {}", ligne(&plein, "RESULTAT "));
    println!("▸ raison : {}", raison.chars().take(400).collect::<String>());
    println!("▸ ensuite : {}", ensuite.chars().take(200).collect::<String>());
    assert!(
        raison.contains("The buffer pool is full"),
        "le refus du tampon plein n'a pas été fabriqué (32 Mio, {LIGNES} lignes) — le test ne prouverait rien :\n{}",
        plein.chars().take(4000).collect::<String>()
    );
    assert!(raison.contains("tampon du moteur est plein") && raison.contains("RAG3DB_BUFFER_POOL_SIZE"), "la cause et le réglage : {raison}");
    assert!(ensuite.starts_with("refusée") && ensuite.contains("must reopen"), "la session refuse tout ensuite : {ensuite}");
    assert!(!plein.contains("retour au MERGE"), "le repli sur MERGE n'a pas eu lieu");

    let reprise = lancer("reprise", &base, None);
    let comptes = ligne(&reprise, "COMPTES ");
    println!("▸ après réouverture avec le tampon par défaut : {comptes}");
    assert_eq!(comptes, format!("{:?}", vec![CypherValue::Int(LIGNES as i64), CypherValue::Int(LIGNES as i64)]), "chaque ligne une fois");
    let _ = std::fs::remove_dir_all(&dossier);
}
