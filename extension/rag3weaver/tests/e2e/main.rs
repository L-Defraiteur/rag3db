//! **Le binaire unique des suites e2e** (11 octobre 2026, demande de Lucie :
//! « bâtis une seule fois avec le même cœur, pas instanciés 50 fois »).
//!
//! Chaque suite reste un fichier `tests/e2e_*.rs`, monté ici comme module :
//! le crate de test se compile et se relie UNE fois au lieu d'une fois par
//! suite. `run_e2e.sh` lance ce binaire UNE FOIS PAR SUITE, filtré sur
//! `<suite>::` — chaque suite garde donc son processus à elle (ses
//! `set_var`, ses `OnceLock`, ses statics, son démon), comme avant.
//!
//! Lancé à la main sans filtre (`cargo test --test e2e`), il joue TOUTES les
//! suites dans un seul processus : l'isolation entre suites n'y est plus
//! garantie. Passer par `run_e2e.sh`, ou filtrer : `cargo test --test e2e
//! e2e_code::`.
//!
//! **Fichier généré** par `tests/e2e/generer.py` — ne pas l'éditer à la main.

#[allow(dead_code, unused_imports)]
#[path = "../common/mod.rs"]
mod common;

#[path = "../e2e_agent_loop.rs"]
mod e2e_agent_loop;
#[path = "../e2e_arret_brutal.rs"]
mod e2e_arret_brutal;
#[path = "../e2e_arret_propre.rs"]
mod e2e_arret_propre;
#[path = "../e2e_avis_du_modele.rs"]
mod e2e_avis_du_modele;
#[path = "../e2e_banc_bge_m3.rs"]
mod e2e_banc_bge_m3;
#[path = "../e2e_banc_cohesion.rs"]
mod e2e_banc_cohesion;
#[path = "../e2e_banc_etage.rs"]
mod e2e_banc_etage;
#[path = "../e2e_banc_fiches_classees.rs"]
mod e2e_banc_fiches_classees;
#[path = "../e2e_banc_hnsw.rs"]
mod e2e_banc_hnsw;
#[path = "../e2e_banc_liens.rs"]
mod e2e_banc_liens;
#[path = "../e2e_banc_qualite.rs"]
mod e2e_banc_qualite;
#[path = "../e2e_banc_relations.rs"]
mod e2e_banc_relations;
#[path = "../e2e_banc_texte_brut.rs"]
mod e2e_banc_texte_brut;
#[path = "../e2e_banc_vertex_embedding.rs"]
mod e2e_banc_vertex_embedding;
#[path = "../e2e_batch_observe.rs"]
mod e2e_batch_observe;
#[path = "../e2e_burn_attention.rs"]
mod e2e_burn_attention;
#[path = "../e2e_burn_embedder.rs"]
mod e2e_burn_embedder;
#[path = "../e2e_burn_fusion.rs"]
mod e2e_burn_fusion;
#[path = "../e2e_burn_granite.rs"]
mod e2e_burn_granite;
#[path = "../e2e_burn_minilm.rs"]
mod e2e_burn_minilm;
#[path = "../e2e_burn_multilingual_minilm.rs"]
mod e2e_burn_multilingual_minilm;
#[path = "../e2e_burn_ocr.rs"]
mod e2e_burn_ocr;
#[path = "../e2e_burn_reranker.rs"]
mod e2e_burn_reranker;
#[path = "../e2e_burn_xlmr_reranker.rs"]
mod e2e_burn_xlmr_reranker;
#[path = "../e2e_catalogue_gabarits.rs"]
mod e2e_catalogue_gabarits;
#[path = "../e2e_chainage.rs"]
mod e2e_chainage;
#[path = "../e2e_charge_ingestion.rs"]
mod e2e_charge_ingestion;
#[path = "../e2e_checkpoint.rs"]
mod e2e_checkpoint;
#[path = "../e2e_chemin_de_masse.rs"]
mod e2e_chemin_de_masse;
#[path = "../e2e_cloud_code_agent.rs"]
mod e2e_cloud_code_agent;
#[path = "../e2e_cloud_schema_probe.rs"]
mod e2e_cloud_schema_probe;
#[path = "../e2e_code.rs"]
mod e2e_code;
#[path = "../e2e_code_sync.rs"]
mod e2e_code_sync;
#[path = "../e2e_contrat_transactions.rs"]
mod e2e_contrat_transactions;
#[path = "../e2e_conversation_a_plusieurs.rs"]
mod e2e_conversation_a_plusieurs;
#[path = "../e2e_copy_liens.rs"]
mod e2e_copy_liens;
#[path = "../e2e_copy_refuse_rouvre.rs"]
mod e2e_copy_refuse_rouvre;
#[path = "../e2e_dataflow_observe.rs"]
mod e2e_dataflow_observe;
#[path = "../e2e_demon_embeddings.rs"]
mod e2e_demon_embeddings;
#[path = "../e2e_demon_hote.rs"]
mod e2e_demon_hote;
#[path = "../e2e_drain_unified.rs"]
mod e2e_drain_unified;
#[path = "../e2e_embarqueur_absent.rs"]
mod e2e_embarqueur_absent;
#[path = "../e2e_entites_derivees.rs"]
mod e2e_entites_derivees;
#[path = "../e2e_estimate.rs"]
mod e2e_estimate;
#[path = "../e2e_etat_non_declare.rs"]
mod e2e_etat_non_declare;
#[path = "../e2e_garde_des_liens.rs"]
mod e2e_garde_des_liens;
#[path = "../e2e_generic_search.rs"]
mod e2e_generic_search;
#[path = "../e2e_graph_tool.rs"]
mod e2e_graph_tool;
#[path = "../e2e_graphe_et_paquets.rs"]
mod e2e_graphe_et_paquets;
#[path = "../e2e_highlight_long_text.rs"]
mod e2e_highlight_long_text;
#[path = "../e2e_hnsw_scale.rs"]
mod e2e_hnsw_scale;
#[path = "../e2e_idempotent_registration.rs"]
mod e2e_idempotent_registration;
#[path = "../e2e_impact.rs"]
mod e2e_impact;
#[path = "../e2e_impact_fichier.rs"]
mod e2e_impact_fichier;
#[path = "../e2e_invariant_des_vecteurs.rs"]
mod e2e_invariant_des_vecteurs;
#[path = "../e2e_journal_de_conversation.rs"]
mod e2e_journal_de_conversation;
#[path = "../e2e_lecture_du_catalogue.rs"]
mod e2e_lecture_du_catalogue;
#[path = "../e2e_lecture_mermaid.rs"]
mod e2e_lecture_mermaid;
#[path = "../e2e_lecture_seule_apres_fermeture_sans_point_de_reprise.rs"]
mod e2e_lecture_seule_apres_fermeture_sans_point_de_reprise;
#[path = "../e2e_liens.rs"]
mod e2e_liens;
#[path = "../e2e_mcp_sigterm.rs"]
mod e2e_mcp_sigterm;
#[path = "../e2e_mcp_stdio.rs"]
mod e2e_mcp_stdio;
#[path = "../e2e_mesure_ingestion_code.rs"]
mod e2e_mesure_ingestion_code;
#[path = "../e2e_mesure_joignabilite.rs"]
mod e2e_mesure_joignabilite;
#[path = "../e2e_mesure_sync_source.rs"]
mod e2e_mesure_sync_source;
#[path = "../e2e_native.rs"]
mod e2e_native;
#[path = "../e2e_nom_reserve.rs"]
mod e2e_nom_reserve;
#[path = "../e2e_outils_disent_ce_qui_reste.rs"]
mod e2e_outils_disent_ce_qui_reste;
#[path = "../e2e_phase0b.rs"]
mod e2e_phase0b;
#[path = "../e2e_plans_par_lot.rs"]
mod e2e_plans_par_lot;
#[path = "../e2e_points_de_reprise_nettoyes.rs"]
mod e2e_points_de_reprise_nettoyes;
#[path = "../e2e_postgres.rs"]
mod e2e_postgres;
#[path = "../e2e_preuve_d_existence.rs"]
mod e2e_preuve_d_existence;
#[path = "../e2e_prise_atomique.rs"]
mod e2e_prise_atomique;
#[path = "../e2e_profile_overhead.rs"]
mod e2e_profile_overhead;
#[path = "../e2e_proto_boutique.rs"]
mod e2e_proto_boutique;
#[path = "../e2e_rag3daemon.rs"]
mod e2e_rag3daemon;
#[path = "../e2e_reacteur.rs"]
mod e2e_reacteur;
#[path = "../e2e_reaction_montee.rs"]
mod e2e_reaction_montee;
#[path = "../e2e_recherche_dense_apres_suppressions.rs"]
mod e2e_recherche_dense_apres_suppressions;
#[path = "../e2e_repli_de_branche.rs"]
mod e2e_repli_de_branche;
#[path = "../e2e_reprise_durcie.rs"]
mod e2e_reprise_durcie;
#[path = "../e2e_rerank.rs"]
mod e2e_rerank;
#[path = "../e2e_result_mode.rs"]
mod e2e_result_mode;
#[path = "../e2e_rouvrir.rs"]
mod e2e_rouvrir;
#[path = "../e2e_scope.rs"]
mod e2e_scope;
#[path = "../e2e_search.rs"]
mod e2e_search;
#[path = "../e2e_search_queue.rs"]
mod e2e_search_queue;
#[path = "../e2e_simple_entity.rs"]
mod e2e_simple_entity;
#[path = "../e2e_sparse_dump.rs"]
mod e2e_sparse_dump;
#[path = "../e2e_stockage_du_plein_texte.rs"]
mod e2e_stockage_du_plein_texte;
#[path = "../e2e_symbol_search.rs"]
mod e2e_symbol_search;
#[path = "../e2e_synchronisation.rs"]
mod e2e_synchronisation;
#[path = "../e2e_tampon_plein.rs"]
mod e2e_tampon_plein;
#[path = "../e2e_titre_indexe.rs"]
mod e2e_titre_indexe;
#[path = "../e2e_tx_ligne_a_ligne.rs"]
mod e2e_tx_ligne_a_ligne;
#[path = "../e2e_tx_merge_puis_copy.rs"]
mod e2e_tx_merge_puis_copy;
#[path = "../e2e_tx_par_paquet_arret.rs"]
mod e2e_tx_par_paquet_arret;
#[path = "../e2e_tx_vecteurs_apres_rollback.rs"]
mod e2e_tx_vecteurs_apres_rollback;
#[path = "../e2e_types_differes.rs"]
mod e2e_types_differes;
#[path = "../e2e_undo.rs"]
mod e2e_undo;
#[path = "../e2e_usages.rs"]
mod e2e_usages;
#[path = "../e2e_verrous.rs"]
mod e2e_verrous;
#[path = "../e2e_working_tree.rs"]
mod e2e_working_tree;

/// Les `tests/e2e_*.rs` suivis par git (index compris) — un brouillon non
/// suivi d'une autre session ne compte pas. Sans git, tous les présents.
fn suites_suivies() -> Vec<String> {
    let crate_ = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let par_git = std::process::Command::new("git")
        .args(["ls-files", "tests/e2e_*.rs"])
        .current_dir(crate_)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter_map(|l| l.rsplit('/').next().map(str::to_string))
                .collect::<Vec<_>>()
        });
    par_git.unwrap_or_else(|| {
        std::fs::read_dir(crate_.join("tests"))
            .map(|d| {
                d.flatten()
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .filter(|n| n.starts_with("e2e_") && n.ends_with(".rs"))
                    .collect()
            })
            .unwrap_or_default()
    })
}

/// **Chaque suite est montée.** Sans ce filet, une suite ajoutée sous
/// `tests/` sans être montée ici ne serait plus bâtie du tout (`autotests =
/// false`) — un saut en silence, le pire des défauts d'une batterie.
#[test]
fn chaque_suite_est_montee() {
    let ici = include_str!("main.rs");
    let mut manquantes = Vec::new();
    for nom in suites_suivies() {
        if !ici.contains(&format!("#[path = \"../{nom}\"]")) {
            manquantes.push(nom);
        }
    }
    assert!(
        manquantes.is_empty(),
        "suites non montées dans tests/e2e/main.rs : {manquantes:?} — \
         rejouer `python3 tests/e2e/generer.py`"
    );
}

/// **Aucune suite ne se relance par le nom court d'un de ses tests.** Dans ce
/// binaire, un test s'appelle `<suite>::<test>` : un fils lancé par
/// `current_exe()` avec `--exact <test>` ne jouerait rien, sortirait avec 0,
/// et le parent conclurait sur un fils vide — un vert vide sur les suites
/// d'arrêt brutal (relecture de rag3db-73, 11 octobre 2026). La forme juste :
/// `&crate::common::test_de_cette_suite!("role_enfant")`.
#[test]
fn aucune_suite_ne_se_relance_par_son_nom_court() {
    let dossier = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut fautes = Vec::new();
    for nom in suites_suivies() {
        let texte = std::fs::read_to_string(dossier.join(&nom)).unwrap_or_default();
        let compact: String = texte.split_whitespace().collect::<Vec<_>>().join(" ");
        if compact.contains("\"--exact\", \"") {
            fautes.push(nom);
        }
    }
    assert!(
        fautes.is_empty(),
        "suites qui se relancent par un nom court (`--exact \"test\"`), qui ne désigne \
         rien dans le binaire unique : {fautes:?} — employer \
         `crate::common::test_de_cette_suite!(\"test\")`"
    );
}
