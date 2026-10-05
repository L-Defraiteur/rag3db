//! **Un COPY refusé par le moteur rend la base à rouvrir** (5 octobre 2026).
//!
//! Hors transaction, un COPY que le moteur refuse après l'avoir commencé se
//! repliait sur MERGE dans la même session. Le banc a montré qu'après un COPY
//! refusé ou annulé, le même COPY validé puis un point de reprise ne finit
//! pas ; et un index de clé primaire qui perd une entrée (défaut du moteur
//! trouvé le même jour) faisait sauter au MERGE l'arête en silence. Désormais
//! la base est empoisonnée, fermée sans point de reprise, et la cause est
//! dite.
//!
//! Le cas fabriqué, déterministe : une preuve d'existence périmée. Des nœuds
//! posés pendant la preuve (`begin_proving_presence`), puis l'un supprimé en
//! Cypher brut, ce qui ne le retire pas de la preuve ; le COPY des liens lui
//! fait confiance, et le moteur le refuse (« Unable to find primary key »).
//! Rouverte, sans preuve, la base vérifie les bouts, et la reprise va au bout.
//!
//! ```bash
//! ./run_e2e.sh --test e2e_copy_refuse_rouvre
//! ```
#![cfg(feature = "rag3db-native")]

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use rag3weaver::config::FieldType;
use rag3weaver::connection::CypherValue;
use rag3weaver::disponibilite::Disponibilites;
use rag3weaver::embedder::MockEmbedder;
use rag3weaver::records::RefOrUuid;
use rag3weaver::search::SearchSignals;
use rag3weaver::{Catalog, CatalogConfig, EntityConfig, Rag3dbConnection, SimpleFieldDef};

const ROLE: &str = "COPY_REFUSE_ROLE";
const BASE: &str = "COPY_REFUSE_BASE";

fn bout() -> EntityConfig {
    let mut fields = HashMap::new();
    fields.insert(
        "nom".to_string(),
        SimpleFieldDef { field_type: FieldType::String, is_title: true, is_content: true, ..Default::default() },
    );
    EntityConfig { fields, signals: SearchSignals::BM25, hashsafe: Some(vec!["nom".into()]), ..Default::default() }
}

fn ligne(nom: &str) -> BTreeMap<String, CypherValue> {
    BTreeMap::from([("nom".to_string(), CypherValue::String(nom.into()))])
}

fn catalogue(base: &str) -> Catalog {
    let conn = Rag3dbConnection::new(base).expect("ouvrir la base");
    let config = CatalogConfig { name: Some("copy-refuse".into()), embedding_dim: 4, ..Default::default() };
    let mut catalog = Catalog::new(Box::new(conn), Box::new(MockEmbedder::new(4)), config);
    catalog.initialize().expect("initialize");
    catalog.register_entity("Gauche", bout()).unwrap();
    catalog.register_entity("Droite", bout()).unwrap();
    catalog.register_relation_with("Lien", "Gauche", "Droite", HashMap::new()).unwrap();
    catalog
}

/// 30 × 30 liens, au-delà du seuil du COPY.
fn lier(catalog: &mut Catalog) -> String {
    for g in 0..30 {
        let ug = catalog.entity_uuid("Gauche", &ligne(&format!("g{g}"))).unwrap();
        for d in 0..30 {
            let ud = catalog.entity_uuid("Droite", &ligne(&format!("d{d}"))).unwrap();
            catalog
                .link_jusqu_a("Lien", RefOrUuid::Uuid(ug.clone()), RefOrUuid::Uuid(ud), BTreeMap::new(), Disponibilites::AUCUNE)
                .unwrap();
        }
    }
    let r = catalog.drain();
    format!("failed={} {:?}", r.failed, r.warnings).replace('\n', " ")
}

fn compte(catalog: &Catalog, q: &str) -> String {
    catalog.execute_raw(q).map(|r| format!("{:?}", r.rows.first())).unwrap_or_else(|e| format!("refusée : {e}"))
}

#[test]
#[ignore]
fn role_enfant() {
    let (Ok(role), Ok(base)) = (std::env::var(ROLE), std::env::var(BASE)) else { return };
    let mut catalog = catalogue(&base);
    if role == "refuser" {
        catalog.begin_proving_presence();
        catalog.ingest_entities("Gauche", (0..30).map(|i| ligne(&format!("g{i}"))).collect()).unwrap();
        catalog.ingest_entities("Droite", (0..30).map(|i| ligne(&format!("d{i}"))).collect()).unwrap();
        // Supprimé hors du drain : la preuve le croit encore là.
        catalog.execute_raw("MATCH (d:Droite {nom: 'd0'}) DETACH DELETE d").unwrap();
        println!("RESULTAT {}", lier(&mut catalog));
        println!("RAISON {}", catalog.must_reopen().unwrap_or_default().replace('\n', " "));
        println!("ENSUITE {}", compte(&catalog, "MATCH (d:Droite) RETURN count(d)"));
    } else {
        println!("REPRISE {}", lier(&mut catalog));
        println!("LIENS {}", compte(&catalog, "MATCH ()-[r:Lien]->() RETURN count(r)"));
    }
}

fn lancer(role: &str, base: &PathBuf) -> String {
    let s = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "role_enfant", "--nocapture", "--ignored"])
        .env(ROLE, role)
        .env(BASE, base)
        .output()
        .expect("lancer le fils");
    let texte = format!("{}{}", String::from_utf8_lossy(&s.stdout), String::from_utf8_lossy(&s.stderr));
    assert!(s.status.success(), "le fils « {role} » a échoué ({:?}) :\n{}", s.status, texte.chars().take(4000).collect::<String>());
    texte
}

fn ligne_de<'a>(texte: &'a str, prefixe: &str) -> &'a str {
    texte.lines().find_map(|l| l.strip_prefix(prefixe)).unwrap_or_else(|| panic!("pas de « {prefixe} » :\n{texte}"))
}

#[test]
#[ignore]
fn un_copy_refuse_par_le_moteur_empoisonne_la_base_sans_repli() {
    let dossier = PathBuf::from(std::env::var("HOME").unwrap()).join(format!(
        ".cache/rag3weaver-build/copy-refuse-{}-{}",
        std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    ));
    std::fs::create_dir_all(&dossier).unwrap();
    let base = dossier.join("base.rag3db");
    let refus = lancer("refuser", &base);
    let raison = ligne_de(&refus, "RAISON ");
    println!("▸ résultat : {}", ligne_de(&refus, "RESULTAT ").chars().take(300).collect::<String>());
    println!("▸ raison : {}", raison.chars().take(300).collect::<String>());
    assert!(
        raison.contains("Unable to find primary key") && raison.contains("le moteur a refusé le COPY"),
        "le refus du moteur, nommé, rend la base à rouvrir :\n{}",
        refus.chars().take(3000).collect::<String>()
    );
    assert!(ligne_de(&refus, "ENSUITE ").starts_with("refusée"), "la session refuse tout ensuite");
    assert!(!refus.contains("retour au chemin par lots"), "pas de repli dans la même session");

    let reprise = lancer("reprendre", &base);
    println!("▸ reprise : {} ; liens {}", ligne_de(&reprise, "REPRISE "), ligne_de(&reprise, "LIENS "));
    assert!(ligne_de(&reprise, "REPRISE ").starts_with("failed=0"), "la reprise va au bout");
    assert_eq!(ligne_de(&reprise, "LIENS "), "Some([Int(870)])", "30 × 29 liens : le bout supprimé écarté par la vérification");
    let _ = std::fs::remove_dir_all(&dossier);
}
