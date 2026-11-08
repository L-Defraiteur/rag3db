//! E2E : **le poids des positions du creux, par le service d'embarquement**
//! (`POST /embed_sparse` avec `"spans": true`). Un client sans le modèle
//! surligne en creux avec ce que le service a déjà calculé : la plage
//! d'octets de chaque jeton et son poids avant la fusion au maximum.
//!
//! Joue contre le service bge-m3 de `RAG3WEAVER_EMBED_SERVICE` (luciepc,
//! 127.0.0.1:7879 là-bas, le tunnel 7980 ici) ; sans service bge-m3, il
//! rougit en le disant — il ne se saute pas.
//!
//! ```bash
//! ./run_e2e.sh --test e2e_service_spans
//! ```

#![cfg(feature = "daemon")]

use rag3weaver::daemon::DaemonEmbedder;
use rag3weaver::embedder::SparseEmbedder;

#[test]
#[ignore = "joue contre le service d'embarquement bge-m3"]
fn le_service_rend_le_poids_de_chaque_position() {
    let service = DaemonEmbedder::from_service("bge-m3")
        .expect("RAG3WEAVER_EMBED_SERVICE doit nommer un service bge-m3 : ce témoin ne se saute pas")
        .unwrap_or_else(|e| panic!("{e}"));
    let textes = vec![
        "Le chat dort sur la base de données.".to_string(),
        "fn run_hop(catalog: &Catalog) -> Result<QueryResult, String>".to_string(),
    ];
    let lignes = service
        .embed_sparse_spans(&textes)
        .unwrap_or_else(|e| panic!("le service {} ne rend pas les positions : {e}", service.identite().executable));
    assert_eq!(lignes.len(), textes.len());
    let sans = service.embed_sparse(&textes).expect("le creux seul");
    for ((texte, (creux, spans)), seul) in textes.iter().zip(&lignes).zip(&sans) {
        assert_eq!(creux, seul, "le même creux, avec ou sans positions");
        assert!(!spans.is_empty(), "des positions pour {texte:?}");
        // Des plages d'octets du texte reçu, dans l'ordre, sur des frontières de caractère.
        for s in spans {
            assert!(s.start < s.end && s.end <= texte.len(), "plage hors du texte : {s:?} dans {texte:?}");
            assert!(texte.is_char_boundary(s.start) && texte.is_char_boundary(s.end), "plage au milieu d'un caractère : {s:?}");
            assert!(s.weight >= 0.0, "un poids après ReLU n'est pas négatif : {s:?}");
        }
        assert!(spans.windows(2).all(|w| w[0].start <= w[1].start), "positions dans l'ordre du texte");
        // Chaque mot est couvert par au moins une position.
        for mot in texte.split_whitespace() {
            let debut = mot.as_ptr() as usize - texte.as_ptr() as usize;
            let fin = debut + mot.len();
            assert!(
                spans.iter().any(|s| s.start < fin && s.end > debut),
                "le mot {mot:?} n'a aucune position dans {texte:?} : {spans:?}"
            );
        }
        // Cohérent avec le creux : chaque valeur est le poids d'une position,
        // et le plus grand poids est la plus grande valeur.
        assert!(
            creux.values.iter().all(|v| spans.iter().any(|s| s.weight == *v)),
            "une valeur du creux sans position de ce poids"
        );
        let max_spans = spans.iter().map(|s| s.weight).fold(0.0f32, f32::max);
        let max_creux = creux.values.iter().copied().fold(0.0f32, f32::max);
        assert_eq!(max_spans, max_creux, "le plus grand poids des positions est la plus grande valeur du creux");
        let surligne: Vec<String> = spans.iter().filter(|s| s.weight > 0.0).map(|s| format!("{}={:.3}", &texte[s.start..s.end], s.weight)).collect();
        println!("▸ {} positions, {} valeurs : {}", spans.len(), creux.nnz(), surligne.join(" "));
    }
}
