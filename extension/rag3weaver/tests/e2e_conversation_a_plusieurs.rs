//! **Une expérience, pas une validation.**
//!
//! On pose une question dans un fil où trois agents sont présents, chacun avec
//! un domaine différent et les mêmes outils sur notre propre code. Puis on
//! écrit ce qui s'est réellement passé — chaque tour, chaque appel d'outil avec
//! ses arguments entiers, chaque résultat, les postures et les messages.
//!
//! **Le livrable est la trace, pas le vert.** Si les agents se marchent dessus,
//! répondent tous la même chose, ou n'appellent jamais `pause_dialogue`, c'est
//! précisément ce qu'on veut lire — et c'est le genre de chose qu'aucun test
//! scripté ne dira, puisqu'un test scripté sait déjà ce qui va arriver.
//!
//! ```bash
//! RAG3WEAVER_LOCAL_LLM=http://127.0.0.1:8080/v1 RAG3WEAVER_LOCAL_MODEL=qwen3-coder-30b \
//!   ./run_e2e.sh --features openai-llm --test e2e_conversation_a_plusieurs
//! ```
//!
//! L'artefact est écrit dans `target/artefacts/`.
#![cfg(all(feature = "rag3db-native", feature = "openai-llm", feature = "code", feature = "burn-embedder"))]

use std::sync::{Arc, Mutex};
use std::time::Instant;

use rag3weaver::agent::{Agent, AgentLimits, GraphToolBox, ToolBox};
use rag3weaver::catalog::Catalog;
use rag3weaver::code::{analyze_source, default_scope_chunking, register_code_schema};
use rag3weaver::code_tools::{FileSource, WorkingTree, FILE_SOURCE_SERVICE};
use rag3weaver::dataflow::graph_tool::builtin_graph_tools;
use rag3weaver::dataflow::ServiceRegistry;
use rag3weaver::embedder::{DualEmbedder, Embedder, HashEmbedder};
use rag3weaver::events::{inbox_topic, EventBus};
use rag3weaver::llm::{GenOptions, StringSink, ToolChoice, Turn};
use rag3weaver::openai_llm::OpenAiLlm;
use rag3weaver::postures::Postures;
use rag3weaver::{CatalogConfig, Rag3dbConnection};

mod common;

fn rag3db_root() -> String {
    std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("../..")
            .canonicalize()
            .unwrap()
            .to_string_lossy()
            .to_string()
    })
}

/// **Cette suite était locale seulement**, et ce n'était pas un oubli : trois
/// agents qui se répondent font beaucoup d'appels, et les envoyer au nuage se
/// paie. La fabrique change cela, délibérément — sous `RAG3WEAVER_REGIME=confort`
/// c'est précisément ce qu'on demande, ne pas prendre la carte. Sous `plein`,
/// `RAG3WEAVER_LOCAL_LLM` continue de décider comme avant.
fn model() -> Option<OpenAiLlm> {
    rag3weaver::regime::modele_agentique("fil")
}

/// Notre propre `src/`, indexé en entier : les agents parlent de code réel.
///
/// **La racine doit contenir de quoi répondre.** Premier essai : `src/dataflow`
/// seul, avec une question qui porte sur le catalogue — les trois agents ont
/// passé leurs tours à chercher un `catalog.rs` qui n'existait pas dans leur
/// monde. Le montage contredisait sa propre question, et ça ne se voyait qu'en
/// lisant la trace.
fn setup() -> (Arc<ServiceRegistry>, Arc<dyn FileSource>) {
    let dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("src");
    let source: Arc<dyn FileSource> = Arc::new(WorkingTree::new(&dir));

    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    let ext = format!("{}/extension/vector/build/libvector.rag3db_extension", rag3db_root());
    boxed.execute(&format!("LOAD EXTENSION '{ext}'")).unwrap();
    // **Le vrai embedder, pas un factice.**
    //
    // Premiers essais avec `HashEmbedder` : les 34 résultats de `search`
    // étaient tous `bm25`, et une requête conceptuelle (« node failure »)
    // rendait zéro. Les agents ont donc préféré `grep`, et ils avaient
    // raison — de là où ils se tenaient, `search` était un grep avec des
    // étapes en plus. On ne peut pas reprocher à un agent de bouder une
    // moitié de moteur qu'on ne lui a pas donnée.
    let config = CatalogConfig { name: Some("fil".into()), embedding_dim: 1024, ..Default::default() };
    let mut catalog = Catalog::new(boxed, Box::new(HashEmbedder::new(1024)), config);
    catalog.initialize().unwrap();
    let bge: Arc<dyn DualEmbedder> = common::burn::BGE_M3.clone();
    catalog.set_dual_embedder(bge);
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
    let analysis = analyze_source(source.as_ref()).unwrap();
    let report = catalog.ingest_code(&analysis).unwrap();
    eprintln!("[fil] ingéré {report:?}");

    let mut services = ServiceRegistry::new();
    // Le catalogue monte lui-même la liste — une seule source.
    catalog.register_search_services(&mut services);
    // **L'embarqueur du graphe n'est pas celui du catalogue**, et c'est voulu :
    // le catalogue garde un `HashEmbedder` pour son ingestion, le graphe reçoit
    // le vrai BGE-M3. Enregistré **après** la liste commune, qui poserait sinon
    // celui du catalogue — un agent qui cherche sur des vecteurs de hachage
    // aurait raison de bouder la recherche.
    let embedder: Arc<dyn Embedder> = common::burn::BGE_M3.clone();
    services.register::<Arc<dyn Embedder>>("embedder", embedder);
    services.register("catalog", Arc::new(Mutex::new(catalog)));
    services.register::<Arc<dyn FileSource>>(FILE_SOURCE_SERVICE, source.clone());
    (Arc::new(services), source)
}

/// Qui est dans le fil. Le rôle est un **domaine**, pas un adjectif : ce que
/// chacun regarde, et donc ce qu'il ne regarde pas
/// (doc 09 §2). Aucune ligne de personnalité — c'est délibéré : si les
/// réponses se ressemblent, on saura que le rôle ne portait rien.
const QUI: [(&str, &str, &str); 3] = [
    (
        "alma",
        "dataflow",
        "Vous travaillez sur le runtime dataflow : les nœuds, les ports, l'exécution. \
         Ce qui touche au catalogue et au schéma n'est pas votre domaine.",
    ),
    (
        "zed",
        "catalogue",
        "Vous travaillez sur le catalogue : le schéma, l'identité des entités, les migrations. \
         Ce qui touche à l'ordonnancement des nœuds n'est pas votre domaine.",
    ),
    (
        "maurice",
        "recherche",
        "Vous travaillez sur la recherche : BM25, vecteurs, fusion, rerank. \
         Le reste n'est pas votre domaine.",
    ),
];

const SOCLE: &str = "Vous êtes dans un fil avec d'autres agents et une humaine, Lucie. \
Outils sur le code : `grep`, `read`, `search` (target=\"Scope\" ou \"File\" ; ajoutez `relation` pour suivre le graphe). \
Établissez les faits avec les outils avant d'affirmer, et citez fichier et lignes. \
Si la question ne relève pas de votre domaine, ou si un autre a déjà répondu ce que vous alliez dire, \
appelez `pause_dialogue` plutôt que de meubler. Répondez court.";

const QUESTION: &str = "Quand un nœud du dataflow échoue au milieu d'un graphe, qu'arrive-t-il aux \
écritures déjà faites dans le catalogue ? Est-ce qu'on peut se retrouver avec un index à moitié à jour ?";

/// Tout ce qu'on a vu d'un agent, gardé entier.
struct Passage {
    nom: String,
    domaine: String,
    turns: Vec<Turn>,
    stop: String,
    appels: usize,
    erreurs: usize,
    ms: u128,
    texte: String,
}

fn md_echappe(s: &str) -> String {
    s.replace("```", "ˋˋˋ")
}

/// L'artefact : **entier**. Pas de troncature, pas de résumé — c'est tout
/// l'objet. Un extrait de 400 caractères ne dit pas si l'agent a lu avant de
/// répondre.
///
/// **Et il s'écrit pendant, pas après.**
///
/// Il se construisait entièrement en mémoire et se posait sur le disque à la
/// toute fin. Une expérience qui casse au troisième agent ne laissait donc
/// rien — ni les sondes, ni les deux agents qui avaient fini. C'est la même
/// règle que pour les journaux de passe : **écrire d'abord**. Un artefact qui
/// n'existe que si tout s'est bien passé est un artefact qu'on n'a jamais
/// quand on en a besoin.
///
/// Effet de bord voulu : on peut faire un `tail -f` dessus pendant que les
/// agents parlent.
///
/// Le bilan passe donc **à la fin** — il ne peut pas être écrit avant que les
/// agents aient fini, et le mettre en tête obligerait à tout garder en
/// mémoire. On lit la conversation, puis ce qu'elle a coûté ; c'est l'ordre
/// naturel de toute façon.
struct Artefact {
    fichier: std::fs::File,
    chemin: std::path::PathBuf,
}

impl Artefact {
    fn ouvrir(chemin: &std::path::Path, fil: &str, sondes: &[(&str, &str, String)]) -> Self {
        use std::io::Write;
        std::fs::create_dir_all(chemin.parent().unwrap()).ok();
        let mut fichier = std::fs::File::create(chemin).expect("ouvrir l'artefact");

        let mut m = String::new();
        m.push_str(&format!("# Fil « {fil} » — trace complète\n\n"));
        m.push_str("Écrit par le moteur, **au fur et à mesure**. Chaque tour, chaque appel\nd'outil avec ses arguments entiers, chaque résultat.\n\n");
        m.push_str(&format!("**Question posée à tous** — `broadcast`, fil `{fil}` :\n\n> {QUESTION}\n\n"));
        if std::env::var("RAG3WEAVER_TEMOIN").is_ok() {
            m.push_str("**Mode témoin** : aucune phrase de domaine dans l'invite. Les trois agents\nreçoivent exactement le même système. Toute divergence ici est du tirage, pas du rôle.\n\n");
        }
        m.push_str("## Ce que `search` rend, appelé directement\n\n");
        for (cible, requete, res) in sondes {
            m.push_str(&format!(
                "**`search(target=\"{cible}\", query=\"{requete}\")`**\n\n```\n{}\n```\n\n",
                md_echappe(res)
            ));
        }
        fichier.write_all(m.as_bytes()).expect("écrire l'en-tête");
        fichier.flush().ok();
        eprintln!("[fil] artefact ouvert : {} (tail -f pour suivre)", chemin.display());
        Self { fichier, chemin: chemin.to_path_buf() }
    }

    /// Un agent vient de finir : sa conversation entière part sur le disque
    /// **maintenant**, pas à la fin de l'expérience.
    fn ajouter(&mut self, p: &Passage) {
        use std::io::Write;
        let mut m = String::new();
        m.push_str(&format!("\n---\n\n## {} · domaine `{}`\n\n", p.nom, p.domaine));
        m.push_str(&format!(
            "_`{}` · {} appels ({} erreurs) · {} ms_\n\n",
            p.stop, p.appels, p.erreurs, p.ms
        ));
        for (i, t) in p.turns.iter().enumerate() {
            m.push_str(&format!("### [{i}] {}\n\n", t.role));
            if let Some(n) = &t.tool_name {
                m.push_str(&format!("_résultat de_ `{n}`"));
                if let Some(id) = &t.tool_call_id {
                    m.push_str(&format!(" _(appel `{id}`)_"));
                }
                m.push_str("\n\n");
            }
            if !t.content.is_empty() {
                m.push_str(&format!("```\n{}\n```\n\n", md_echappe(&t.content)));
            }
            for c in &t.tool_calls {
                m.push_str(&format!("**appelle** `{}` — `{}`\n\n```json\n{}\n```\n\n", c.name, c.id, md_echappe(&c.arguments)));
            }
        }
        m.push_str(&format!("**Réponse finale**\n\n> {}\n", p.texte.replace('\n', "\n> ")));
        self.fichier.write_all(m.as_bytes()).expect("écrire un passage");
        self.fichier.flush().ok();
    }

    fn terminer(mut self, passages: &[Passage], postures: &Postures) {
        use std::io::Write;
        let mut m = String::new();
        m.push_str("\n---\n\n## Bilan\n\n| agent | domaine | arrêt | appels | erreurs | ms |\n|---|---|---|---|---|---|\n");
        for p in passages {
            m.push_str(&format!(
                "| {} | {} | `{}` | {} | {} | {} |\n",
                p.nom, p.domaine, p.stop, p.appels, p.erreurs, p.ms
            ));
        }
        let post = postures.all();
        m.push_str("\n## Postures à la fin\n\n");
        if post.is_empty() {
            m.push_str("_Aucune._ Personne ne s'est tu explicitement — à lire comme un résultat, pas comme un succès.\n");
        } else {
            for (qui, p) in &post {
                m.push_str(&format!("- **{qui}** — `{:?}`, envers « {} » : {}\n", p.kind, p.with, p.reason));
            }
        }
        self.fichier.write_all(m.as_bytes()).expect("écrire le bilan");
        self.fichier.flush().ok();
        eprintln!("[fil] artefact : {}", self.chemin.display());
    }
}

#[test]
#[ignore]
fn trois_agents_un_fil_une_question() {
    let Some(llm) = model() else {
        eprintln!("skipped: aucun modèle — ni RAG3WEAVER_LOCAL_LLM, ni identifiants Vertex");
        return;
    };
    let (services, _source) = setup();
    let (nodes, graph_tools) = builtin_graph_tools().unwrap();
    let toolbox = GraphToolBox::new(&graph_tools, &nodes, services);

    let bus = EventBus::new(512);
    let postures = Arc::new(Postures::new());
    let fil = "revue-du-27";

    // Les boîtes existent avant qu'on parle : un message envoyé à une boîte
    // jamais ouverte est perdu, et c'est le genre de perte silencieuse qu'on
    // ne veut pas confondre avec « il n'a rien dit ».
    for (nom, _, _) in QUI {
        bus.cursor(&inbox_topic(nom), rag3weaver::agent::AGENT_INBOX_CURSOR);
    }
    let destinataires: Vec<&str> = QUI.iter().map(|(n, _, _)| *n).collect();
    bus.broadcast("lucie", "lucie", &destinataires, QUESTION, fil);

    // **Ce que `search` rend vraiment**, avant d'accuser les agents de le bouder.
    // Zéro appel sur trente-neuf : soit ils l'ignorent, soit il ne sert à rien.
    // La différence se mesure, elle ne se suppose pas.
    let mut sondes = Vec::new();
    for (cible, requete) in [
        ("File", "catalog"),
        ("Scope", "drain"),
        ("Scope", "flush_insertions"),
        ("Scope", "node failure"),
    ] {
        let call = rag3weaver::llm::ToolCall::new(
            format!("sonde-{cible}-{requete}"),
            "search",
            serde_json::json!({ "target": cible, "query": requete }).to_string(),
        );
        let t = toolbox.call(&call);
        sondes.push((cible, requete, t.content));
    }

    // **Horodaté, en plus du nom.** Un artefact est une trace d'expérience :
    // deux manches du même montage sont deux résultats, pas un remplacement.
    // Sans la date on écrasait la précédente en silence, et il fallait penser
    // à changer `RAG3WEAVER_ARTEFACT` à chaque fois — donc on oubliait.
    // L'horodatage est automatique ; le nom reste à qui lance, pour dire ce
    // qu'on essayait.
    let quand = rag3weaver::dataflow::horodatage();
    let nom = std::env::var("RAG3WEAVER_ARTEFACT").unwrap_or_else(|_| "a-plusieurs".into());
    let out = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
        .join(format!("target/artefacts/fil-{nom}-{quand}.md"));
    let mut artefact = Artefact::ouvrir(&out, fil, &sondes);

    let mut passages = Vec::new();
    for (nom, domaine, cadre) in QUI {
        let opts = GenOptions::default()
            .with_max_tokens(1200)
            .with_tools(toolbox.tool_defs())
            .with_tool_choice(ToolChoice::Auto);
        let agent = Agent::new(&llm, &toolbox)
            .with_gen_options(opts)
            .with_name(nom)
            .with_run_id(nom)
            .with_domain(domaine)
            .with_events(bus.clone())
            .with_postures(postures.clone())
            .with_inbox()
            // Six tours ne suffisaient pas : deux agents s'y sont arrêtés en
            // plein milieu d'une recherche, sans jamais conclure. Un agent qui
            // butte sur `MaxIterations` ne dit pas qu'il a fini, il dit qu'on
            // l'a coupé — et on ne peut rien lire de sa réponse.
            .with_limits(AgentLimits { max_iterations: 14, ..Default::default() });

        // **Témoin** : les mêmes agents, sans leur phrase de domaine. Si les
        // réponses divergent autant qu'avec, le rôle ne portait rien — trois
        // tirages du même modèle divergent tout seuls, et attribuer cette
        // divergence au rôle serait exactement l'explication convaincante
        // contre laquelle on se prémunit.
        let temoin = std::env::var("RAG3WEAVER_TEMOIN").is_ok();
        let systeme = if temoin { SOCLE.to_string() } else { format!("{SOCLE}\n\n{cadre}") };
        let mut turns = vec![Turn::system(systeme), Turn::user(QUESTION)];
        let mut sink = StringSink::default();
        let t = Instant::now();
        let run = agent.run(&mut turns, &mut sink);
        let ms = t.elapsed().as_millis();

        let passage = match run {
            Ok(r) => {
                eprintln!("[fil] {nom} : {:?}, {} appels", r.stop, r.tool_calls);
                Passage {
                    nom: nom.into(), domaine: domaine.into(), turns,
                    stop: format!("{:?}", r.stop), appels: r.tool_calls,
                    erreurs: r.tool_errors, ms, texte: r.text,
                }
            }
            Err(e) => {
                // Un échec est un résultat : on l'écrit et on continue.
                eprintln!("[fil] {nom} : ERREUR {e}");
                Passage {
                    nom: nom.into(), domaine: domaine.into(), turns,
                    stop: format!("Err({e})"), appels: 0, erreurs: 0, ms,
                    texte: String::new(),
                }
            }
        };
        // Sur le disque **maintenant** : si le suivant fait tout tomber, ce
        // qui vient de se passer est déjà lisible.
        artefact.ajouter(&passage);
        passages.push(passage);
    }

    artefact.terminer(&passages, &postures);

    // La seule assertion, et elle est faible exprès : l'expérience doit avoir
    // eu lieu. Ce qu'elle raconte se lit, ça ne s'assure pas.
    assert!(!passages.is_empty());
}
