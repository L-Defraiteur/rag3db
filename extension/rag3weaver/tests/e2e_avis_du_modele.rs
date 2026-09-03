//! **Demander son avis au modèle** — sur les outils qu'on lui donne.
//!
//! Lucie, deux fois : *« on pourrait demander à gemini : quel est ton avis sur
//! les outils disponibles, qu'est-ce que ces outils rendent possible, que te
//! manquerait-il pour pouvoir en faire plus… »*.
//!
//! # Pourquoi c'est un test et pas une conversation
//!
//! **La surface décrite doit être celle du moteur, pas la mienne.** Les fiches
//! d'outils envoyées au modèle sortent de `graph_tool_defs_openai` — la même
//! liste, au même format, que celle qu'un agent reçoit en production. Si je la
//! recopiais, je demanderais son avis sur ce que je crois avoir construit
//! plutôt que sur ce qui existe, et l'exercice ne vaudrait rien.
//!
//! Le document produit contient donc, dans l'ordre : les outils **tels que le
//! moteur les publie**, et la réponse **verbatim**. Rien de moi entre les deux.
//!
//! ```bash
//! GOOGLE_APPLICATION_CREDENTIALS=$PWD/../../.vault/vertex-sa.json \
//! GOOGLE_CLOUD_PROJECT=lr-hub-472010 \
//! cargo test --features code,daemon,openai-llm,rag3db-native \
//!   --test e2e_avis_du_modele -- --ignored --nocapture
//! ```

#![cfg(all(feature = "code", feature = "openai-llm", feature = "rag3db-native"))]

use rag3weaver::dataflow::builtin_graph_tools;
use rag3weaver::llm::{generate_to_string, GenOptions, Turn};
use rag3weaver::openai_llm::OpenAiLlm;

fn vertex() -> Option<OpenAiLlm> {
    rag3weaver::regime::modele_agentique("avis")
}

/// La surface, telle que le moteur la publie — **catalogue branché**.
///
/// Le premier essai a interrogé le modèle sans catalogue, et sa deuxième
/// critique portait précisément là-dessus : *« le paramètre `target` est une
/// boîte noire, comment suis-je censé deviner la liste des cibles valides ? »*.
/// Elle était juste sur ce qu'il voyait, et fausse sur ce qui existe :
/// `SearchSourceNode` déclare `Choices::Targets`, que `tool_def_with(catalog)`
/// résout en énumération des cibles réelles.
///
/// Évaluer une surface plus pauvre que la vraie, c'est récolter des critiques
/// qui portent sur le montage. On branche donc un catalogue, comme en
/// production.
fn surface() -> (Vec<serde_json::Value>, usize) {
    let (_, tools) = builtin_graph_tools().expect("outils fournis");

    let conn = rag3weaver::Rag3dbConnection::in_memory().expect("base en mémoire");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    // Un embarqueur factice assumé : on ne cherche rien, on lit des noms de
    // cibles. Le drapeau existe pour ce cas et le rend visible.
    let config = rag3weaver::CatalogConfig {
        name: Some("avis".into()),
        embedding_dim: 8,
        allow_mock_embedder: true,
        ..Default::default()
    };
    let mut catalog = rag3weaver::Catalog::new(
        boxed,
        Box::new(rag3weaver::embedder::MockEmbedder::new(8)),
        config,
    );
    catalog.initialize().expect("initialisation");
    rag3weaver::code::register_code_schema(&mut catalog, rag3weaver::code::default_scope_chunking())
        .expect("schéma de code");
    rag3weaver::template::register_template_schema(&mut catalog).expect("schéma des gabarits");

    let defs = rag3weaver::tools::graph_tool_defs_with(&tools, Some(&catalog));
    let json: Vec<serde_json::Value> = defs.iter().map(|d| d.to_openai_json()).collect();
    let cibles = json
        .iter()
        .find(|t| t["function"]["name"] == "search")
        .and_then(|t| t["function"]["parameters"]["properties"]["target"]["enum"].as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    (json, cibles)
}

const QUESTION: &str = "Voici la liste exacte des outils dont tu disposerais, au format que tu reçois \
d'habitude. Ce moteur indexe du code et de la connaissance dans une base de graphe, et sert des agents \
qui construisent des applications.\n\n\
Trois questions, dans l'ordre, en français :\n\n\
1. **Qu'est-ce que ces outils rendent possible ?** Sois concret : quelles tâches tu mènerais à bien \
avec ça, et lesquelles tu abandonnerais.\n\
2. **Qu'est-ce qui te gêne dans ce qui existe ?** Une description ambiguë, un paramètre dont tu ne \
saurais pas quoi faire, deux outils que tu confondrais, un défaut qui te ferait perdre des tours.\n\
3. **Que te manque-t-il ?** Nomme les outils absents par ordre d'importance, et pour chacun dis ce que \
tu ne peux pas faire sans lui.\n\n\
Ne sois pas poli : les réponses complaisantes ne servent à rien. Si quelque chose est mal conçu, \
dis-le et dis pourquoi.";

#[test]
#[ignore = "appelle un modèle distant"]
fn le_modele_donne_son_avis_sur_nos_outils() {
    let Some(llm) = vertex() else {
        eprintln!("[avis] pas d'identifiants — test ignoré");
        return;
    };

    let (outils, cibles) = surface();
    let liste = serde_json::to_string_pretty(&outils).expect("sérialisation");
    eprintln!(
        "[avis] {} outils publiés, {} caractères de fiches, {cibles} cibles énumérées pour search",
        outils.len(),
        liste.len()
    );
    assert!(cibles > 0, "sans cibles énumérées, on évalue une surface plus pauvre que la vraie");

    let turns = vec![
        Turn::system(
            "Tu es un agent de code expérimenté. Tu évalues une boîte à outils avant de t'en servir. \
             Ton lecteur est l'équipe qui l'a construite : elle veut des critiques utilisables, pas \
             des compliments.",
        ),
        Turn::user(format!("{QUESTION}\n\n```json\n{liste}\n```")),
    ];

    let t0 = std::time::Instant::now();
    // **Un budget large, et c'est structurel.** `GenOptions::default()` donne
    // 512 jetons, et un modèle qui raisonne les dépense d'abord en réflexion :
    // le premier essai a rendu 86 caractères qui étaient la *queue* du
    // raisonnement, ni le début ni la réponse. Un budget qui compte la
    // réflexion doit être dimensionné pour elle.
    let opts = GenOptions::default().with_max_tokens(16_000);
    let sortie = generate_to_string(&llm, &turns, &opts).expect("génération");
    let duree = t0.elapsed();
    eprintln!(
        "[avis] {} caractères en {duree:.1?} · fin={:?} · jetons prompt={} sortie={}",
        sortie.text.len(),
        sortie.finish,
        sortie.usage.prompt_tokens,
        sortie.usage.completion_tokens
    );
    assert!(!sortie.text.trim().is_empty(), "réponse vide");

    // ── Le document, fait par le moteur ──────────────────────────────────
    //
    // Les fiches viennent de `graph_tool_defs_openai`, la réponse est
    // verbatim, et l'en-tête dit d'où vient chaque morceau. Un lecteur doit
    // pouvoir refaire l'expérience sans me croire sur parole.
    let mut doc = String::new();
    doc.push_str("# L'avis d'un modèle sur nos outils\n\n");
    doc.push_str(&format!(
        "**Produit par `tests/e2e_avis_du_modele.rs`**, modèle `{}`, {} outils publiés, \
         {cibles} cibles énumérées, réponse en {duree:.1?}.\n\n",
        std::env::var("VERTEX_MODEL").unwrap_or_else(|_| "google/gemini-3.5-flash".into()),
        outils.len()
    ));
    doc.push_str(
        "Les fiches ci-dessous ne sont pas recopiées : elles sortent de \
         `tools::graph_tool_defs_openai`, la liste que reçoit un agent en production. \
         La réponse est verbatim.\n\n---\n\n## Les outils, tels que le moteur les publie\n\n",
    );
    for t in &outils {
        let f = &t["function"];
        doc.push_str(&format!(
            "### `{}`\n\n{}\n\nParamètres : `{}`\n\n",
            f["name"].as_str().unwrap_or("?"),
            f["description"].as_str().unwrap_or(""),
            f["parameters"]["properties"]
                .as_object()
                .map(|o| o.keys().cloned().collect::<Vec<_>>().join("`, `"))
                .unwrap_or_default()
        ));
    }
    doc.push_str("---\n\n## La question posée\n\n");
    doc.push_str(&format!("> {}\n\n", QUESTION.replace('\n', "\n> ")));
    doc.push_str("---\n\n## La réponse, verbatim\n\n");
    doc.push_str(&sortie.text);
    doc.push('\n');

    let sortie_path = std::env::var("RAG3WEAVER_AVIS_OUT")
        .unwrap_or_else(|_| "target/avis-du-modele.md".into());
    std::fs::write(&sortie_path, &doc).expect("écriture du document");
    eprintln!("[avis] document écrit dans {sortie_path}");
}
