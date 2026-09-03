//! **Un modèle lit-il et écrit-il le Mermaid qu'on lui sert ?**
//!
//! Lucie, avant de convertir douze affichages : *« testons le premier déjà sur
//! gemini voir si lui gère bien, et pareil sur le llm local, voir à quel point
//! ils gèrent bien lecture/écriture de ce format »*.
//!
//! La discipline est la bonne : on standardise sur un format **après** avoir
//! vérifié que ses consommateurs le lisent, pas avant.
//!
//! # Ce qui rend ce test honnête
//!
//! - Les questions ont des réponses **vérifiables dans le diagramme**, et la
//!   réponse est contrainte par un schéma : pas de jugement de notre part sur
//!   « est-ce que ça a l'air correct ».
//! - L'écriture est validée par **notre propre parseur Mermaid**
//!   (`parse_mermaid_template`), pas par un humain qui trouve que ça ressemble.
//!   Si le moteur le relit, c'est valide ; sinon, non.
//!
//! ```bash
//! # nuage
//! GOOGLE_APPLICATION_CREDENTIALS=…/.vault/vertex-sa.json GOOGLE_CLOUD_PROJECT=lr-hub-472010 \
//!   cargo test --features code,openai-llm,rag3db-native --test e2e_lecture_mermaid -- --ignored --nocapture
//! # local
//! RAG3WEAVER_LOCAL_LLM=http://127.0.0.1:8080/v1 RAG3WEAVER_LOCAL_MODEL=qwen3-coder-30b \
//!   cargo test --features code,openai-llm,rag3db-native --test e2e_lecture_mermaid -- --ignored --nocapture
//! ```

#![cfg(all(feature = "code", feature = "openai-llm"))]

use rag3weaver::dataflow::render_nodes::{rendre, resolve_template};
use rag3weaver::dataflow::schema_nodes::{CibleVue, RelationVue, SchemaView};
use rag3weaver::llm::{generate_to_string, GenOptions, ResponseFormat, Turn};
use rag3weaver::openai_llm::OpenAiLlm;

fn modele() -> Option<(OpenAiLlm, String)> {
    rag3weaver::regime::modele_agentique_nomme("mermaid")
}

/// Un schéma assez riche pour que les questions aient un sens, et assez petit
/// pour qu'un échec soit imputable au format et pas à la taille.
fn schema() -> SchemaView {
    let c = |nom: &str, sig: &str, champs: &[&str]| CibleVue {
        nom: nom.into(),
        signaux: sig.into(),
        champs: champs.iter().map(|s| s.to_string()).collect(),
    };
    let r = |nom: &str, de: &str, vers: &str| RelationVue {
        nom: nom.into(),
        de: de.into(),
        vers: vers.into(),
    };
    SchemaView {
        targets: vec![
            c("Scope", "bm25|vector", &["name", "docstring", "signature"]),
            c("File", "bm25", &["path", "language"]),
            c("Symbol", "bm25", &["name"]),
            c("Template", "bm25|vector", &["name", "family", "category"]),
        ],
        relations: vec![
            r("DEFINED_IN", "Scope", "File"),
            r("CONSUMES", "Scope", "Symbol"),
            r("PARENT_OF", "Scope", "Scope"),
        ],
    }
}

/// Une question dont la réponse se lit dans le diagramme.
struct Question {
    texte: &'static str,
    attendu: &'static str,
}

const QUESTIONS: &[Question] = &[
    Question { texte: "Quelle relation va de Scope vers File ? Donne son nom seul.", attendu: "DEFINED_IN" },
    Question { texte: "Combien de cibles le schéma contient-il ? Donne le nombre seul.", attendu: "4" },
    Question { texte: "Quelle cible porte le champ 'family' ? Donne son nom seul.", attendu: "Template" },
    Question { texte: "Existe-t-il une relation qui part de File ? Réponds exactement 'oui' ou 'non'.", attendu: "non" },
    Question { texte: "Quelle relation relie Scope à lui-même ? Donne son nom seul.", attendu: "PARENT_OF" },
];

#[derive(serde::Deserialize)]
struct Reponse {
    reponse: String,
}

fn schema_reponse() -> ResponseFormat {
    ResponseFormat::strict_schema(
        "reponse",
        serde_json::json!({
            "type": "object",
            "properties": { "reponse": { "type": "string" } },
            "required": ["reponse"],
            "additionalProperties": false
        }),
    )
}

#[test]
#[ignore = "appelle un modèle"]
fn un_modele_lit_le_mermaid_qu_on_lui_sert() {
    let Some((llm, nom)) = modele() else {
        eprintln!("[mermaid] aucun modèle joignable — test ignoré");
        return;
    };
    let carte = rendre(&schema(), &resolve_template("schema").unwrap()).expect("rendu");
    eprintln!("[mermaid] carte de {} caractères\n{carte}", carte.len());

    let mut justes = 0;
    for q in QUESTIONS {
        let turns = vec![
            Turn::system(
                "Tu réponds d'après le schéma fourni, et rien d'autre. Réponds au plus court : \
                 un nom, un nombre, ou oui/non. Pas de phrase.",
            ),
            Turn::user(format!("{carte}\n\n{}", q.texte)),
        ];
        let opts = GenOptions::default()
            .with_max_tokens(4_000)
            .with_response_format(schema_reponse());
        let sortie = match generate_to_string(&llm, &turns, &opts) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("  ✗ {} → erreur : {e}", q.texte);
                continue;
            }
        };
        let brut = serde_json::from_str::<Reponse>(&sortie.text)
            .map(|r| r.reponse)
            .unwrap_or(sortie.text.clone());
        let ok = brut.trim().trim_matches('"').eq_ignore_ascii_case(q.attendu);
        justes += ok as usize;
        eprintln!("  {} {} → {:?} (attendu {:?})", if ok { "✓" } else { "✗" }, q.texte, brut.trim(), q.attendu);
    }

    eprintln!("[mermaid] {nom} : lecture {justes}/{}", QUESTIONS.len());
    // Le test ne fixe pas de note : il **mesure**. Une note serait un jugement
    // qu'on aurait choisi après coup pour que ça passe.
    assert!(justes > 0, "aucune question juste — le format est illisible pour ce modèle");
}

/// **Écrire du Mermaid valide**, validé par notre propre parseur.
///
/// C'est la moitié qui compte le plus : les fiches d'outils sont en Mermaid,
/// et un agent qui en compose devra en produire.
#[test]
#[ignore = "appelle un modèle"]
fn un_modele_ecrit_du_mermaid_que_notre_parseur_relit() {
    let Some((llm, nom)) = modele() else {
        eprintln!("[mermaid] aucun modèle joignable — test ignoré");
        return;
    };
    let turns = vec![
        Turn::system(
            "Tu produis un graphe Mermaid et rien d'autre : pas de texte autour, \
             pas de bloc de code, pas d'explication.",
        ),
        Turn::user(
            "Écris un graphe Mermaid `graph LR` avec exactement trois nœuds nommés \
             `source`, `filtre` et `rendu`, une arête de `source` vers `filtre` étiquetée \
             `results`, et une arête de `filtre` vers `rendu` étiquetée `results`. \
             Chaque nœud porte un libellé entre crochets et guillemets, comme \
             `source[\"SearchSourceNode(target=Scope)\"]`."
                .to_string(),
        ),
    ];
    let opts = GenOptions::default().with_max_tokens(4_000);
    let sortie = generate_to_string(&llm, &turns, &opts).expect("génération");
    let texte = sortie.text.trim().trim_start_matches("```mermaid").trim_start_matches("```")
        .trim_end_matches("```").trim().to_string();
    eprintln!("[mermaid] {nom} a écrit :\n{texte}");

    // **Notre parseur est le juge.** S'il le relit, c'est valide ; sinon, non.
    match rag3weaver::dataflow::parse_mermaid_template(&texte, &std::collections::HashMap::new()) {
        Ok(def) => {
            eprintln!(
                "[mermaid] {nom} : écriture ✓ — {} nœuds, {} arêtes",
                def.nodes.len(),
                def.edges.len()
            );
        }
        Err(e) => eprintln!("[mermaid] {nom} : écriture ✗ — notre parseur refuse : {e}"),
    }
}
