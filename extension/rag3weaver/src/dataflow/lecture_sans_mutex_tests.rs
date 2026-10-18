//! **La garde des lectures sans le mutex** (lot L4, 18 octobre 2026) : un nœud
//! de lecture ne prend pas le mutex du catalogue sans dire pourquoi.
//!
//! Le septième rejeu de codeparsers a montré qu'un seul `catalog.lock()` sur
//! le chemin d'une lecture (la cible de `search_code`) suffit à faire attendre
//! toute la tenue d'un écrivain, et, par le verrou du backend dans l'hôte, à
//! faire attendre tous les autres appels avec lui. Ce test relit les fichiers
//! des nœuds de lecture : toute prise d'un verrou (`.lock()`, hors `try_lock`)
//! porte sur sa ligne un marqueur `// L4 : <raison>`. Les raisons admises :
//! - **repli sans vue** : aucune vue de lecture n'est publiée, ou l'appel vise
//!   une autre cellule que la sienne ;
//! - **Strict, ou sans vue** : la consigne `Strict` attend l'écrivain, c'est son
//!   contrat ;
//! - **écriture** : un nœud d'écriture rangé dans le même fichier ;
//! - **pas le catalogue** : un autre verrou.
//!
//! **La liste blanche**, hors de ces fichiers, et pourquoi :
//! - `EstimateNode` (`index_nodes.rs`) : il embarque une sonde et lit le taux
//!   connu, l'état vivant de l'écrivain ; une estimation peut attendre ;
//! - `Backend::build_services` pour une cellule autre que celle de la vue
//!   (`backend.rs`) : il faudrait une vue par cellule ;
//! - `Backend::index_states` (l'état des index, `daemon_status`) : il lit l'état
//!   vivant de l'écrivain (la file, le rebâti en cours), absent de la vue ;
//!   `lock_for_read` le borne à 1 s et rend « occupé » ;
//! - la validation des arguments de la boucle d'agent (`graph_tool.rs`,
//!   `tools.rs`, `node_registry.rs`, `agent.rs`) : le chemin des agents en
//!   processus, pas celui de l'hôte ; à convertir avec le proto.

const FICHIERS: &[(&str, &str)] = &[
    ("generic_search_nodes.rs", include_str!("generic_search_nodes.rs")),
    ("field_weight.rs", include_str!("field_weight.rs")),
    ("search_nodes.rs", include_str!("search_nodes.rs")),
    ("composable_results.rs", include_str!("composable_results.rs")),
    ("schema_nodes.rs", include_str!("schema_nodes.rs")),
    ("code_nodes.rs", include_str!("code_nodes.rs")),
    ("usage_nodes.rs", include_str!("usage_nodes.rs")),
    ("neighborhood_nodes.rs", include_str!("neighborhood_nodes.rs")),
    ("links_nodes.rs", include_str!("links_nodes.rs")),
    ("catalog_read.rs", include_str!("catalog_read.rs")),
    ("backend_nodes.rs", include_str!("../backend_nodes.rs")),
];

/// Une prise de verrou sur la ligne, hors `try_lock`.
fn prend_un_verrou(ligne: &str) -> bool {
    let mut reste = ligne;
    while let Some(i) = reste.find("lock()") {
        if !reste[..i].ends_with("try_") {
            return true;
        }
        reste = &reste[i + "lock()".len()..];
    }
    false
}

#[test]
fn aucun_noeud_de_lecture_ne_prend_le_mutex_sans_dire_pourquoi() {
    let mut fautes = Vec::new();
    for (nom, texte) in FICHIERS {
        for (n, ligne) in texte.lines().enumerate() {
            let code = ligne.trim_start();
            if code.starts_with("//") || !prend_un_verrou(ligne) {
                continue;
            }
            if !ligne.contains("// L4 : ") {
                fautes.push(format!("{nom}:{} : {}", n + 1, code));
            }
        }
    }
    assert!(
        fautes.is_empty(),
        "une prise de verrou sans raison sur le chemin des lectures (marquer `// L4 : <raison>`, ou passer par la vue) :\n{}",
        fautes.join("\n")
    );
}

/// La garde voit bien une prise, et pas un `try_lock`.
#[test]
fn la_garde_reconnait_une_prise() {
    assert!(prend_un_verrou("let g = catalog.lock().unwrap();"));
    assert!(!prend_un_verrou("let g = catalog.try_lock().ok();"));
    assert!(prend_un_verrou("c.try_lock().ok().or_else(|| c.lock().ok())"));
}
