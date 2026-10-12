//! **Le premier flux de jetons, mesuré et éprouvé sur un vrai modèle** (page
//! `docs/12-octobre-2026-recherche/01-le-premier-flux-de-jetons.md`, §4.5 et
//! §5). Deux tests `#[ignore]`, joués à la main :
//!
//! - `measure_token_overhead` : le surcoût par jeton du passage par le graphe,
//!   sur 100 000 fragments d'un `CallbackLlm` ;
//! - `local_model_tokens_reach_the_consumer_before_the_end` (feature
//!   `openai-llm`) : `llama-server` et qwen3-1.7b, lancés et arrêtés par le
//!   témoin, pour éprouver le protocole (le produit se juge avec Gemini).

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::graph::DataflowGraph;
use super::llm_nodes::{LlmNode, LLM_SERVICE};
use super::node::{Node, NodeContext};
use super::port::{take_or_clone, PortDef, PortType, PortValue, Posture};
use super::runtime::DataflowRuntime;
use super::services::ServiceRegistry;
use crate::llm::{Finish, Flow, GenOptions, Llm, TokenSink, Usage};

struct Prompt(String);

impl Node for Prompt {
    fn name(&self) -> &str {
        "source"
    }
    fn node_type(&self) -> &str {
        "Prompt"
    }
    fn outputs(&self) -> Vec<PortDef> {
        vec![PortDef::value("out", PortType::Text, false)]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        ctx.set_output("out", PortValue::new(self.0.clone()));
        Ok(())
    }
}

/// Lit `tokens` fragment par fragment et note l'instant de chacun.
struct Reader {
    seen: Arc<Mutex<Vec<(Instant, String)>>>,
}

impl Node for Reader {
    fn name(&self) -> &str {
        "reader"
    }
    fn node_type(&self) -> &str {
        "Reader"
    }
    fn inputs(&self) -> Vec<PortDef> {
        vec![PortDef::value("tokens", PortType::Text, true).posture(Posture::PerElement)]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        while let Some(v) = ctx.next("tokens").map_err(|e| format!("{e:?}"))? {
            let t = take_or_clone::<String>(v).ok_or("tokens : une chaîne attendue")?;
            self.seen.lock().unwrap().push((Instant::now(), t));
        }
        Ok(())
    }
}

/// `Prompt → LlmNode`, et `tokens → Reader` si `reader` est donné.
fn graph(prompt: &str, opts: GenOptions, reader: Option<Reader>) -> DataflowGraph {
    let mut g = DataflowGraph::new();
    g.add_node(Box::new(Prompt(prompt.into()))).unwrap();
    g.add_node(Box::new(LlmNode::new("llm").with_options(opts))).unwrap();
    g.connect("source", "out", "llm", "prompt").unwrap();
    if let Some(r) = reader {
        g.add_node(Box::new(r)).unwrap();
        g.connect("llm", "tokens", "reader", "tokens").unwrap();
    }
    g
}

fn run(llm: Arc<dyn Llm>, g: &mut DataflowGraph) -> String {
    let mut services = ServiceRegistry::new();
    services.register(LLM_SERVICE, llm);
    let out = DataflowRuntime::with_services(10, services).execute(g).expect("le run");
    out.get("llm", "text").cloned().and_then(take_or_clone::<String>).expect("text")
}

/// Un puits qui accumule, sans graphe : la référence du coût du modèle seul.
struct Plain(String);

impl TokenSink for Plain {
    fn on_token(&mut self, delta: &str) -> Flow {
        self.0.push_str(delta);
        Flow::Continue
    }
}

/// **Le surcoût par jeton du passage par le graphe** (§5) : 100 000
/// fragments, émis (1) dans un puits nu, (2) par `LlmNode` sans lecteur de
/// `tokens` (les éléments sont jetés, l'exécuteur par niveaux), (3) par
/// `LlmNode` vers un lecteur `PerElement` (l'exécuteur concurrent, le canal
/// de 64). Un jeton de modèle arrive toutes les dizaines de millisecondes : la
/// borne, large, est 20 µs par jeton pour le passage par le canal.
#[test]
#[ignore = "mesure, jouée à la main"]
fn measure_token_overhead() {
    const N: usize = 100_000;
    let llm: Arc<dyn Llm> = Arc::new(crate::llm::CallbackLlm::new("cent-mille", 1 << 20, |_, _, sink| {
        for _ in 0..N {
            if sink.on_token("x ") == Flow::Stop {
                return Ok((Finish::cancelled(), Usage::default()));
            }
        }
        Ok((Finish::eos(), Usage { completion_tokens: N, ..Default::default() }))
    }));
    let opts = GenOptions { max_tokens: N, ..Default::default() };
    let per = |d: Duration| d.as_nanos() as f64 / N as f64;

    let t = Instant::now();
    let mut plain = Plain(String::new());
    llm.generate(&[crate::llm::Turn::user("x")], &opts, &mut plain).unwrap();
    let nu = t.elapsed();
    assert_eq!(plain.0.len(), 2 * N);

    let mut g = graph("x", opts.clone(), None);
    assert!(!g.has_streams().unwrap());
    let t = Instant::now();
    let text = run(llm.clone(), &mut g);
    let sans_lecteur = t.elapsed();
    assert_eq!(text.len(), 2 * N);

    let seen = Arc::new(Mutex::new(Vec::with_capacity(N)));
    let mut g = graph("x", opts, Some(Reader { seen: seen.clone() }));
    assert!(g.has_streams().unwrap());
    let t = Instant::now();
    let text = run(llm, &mut g);
    let avec_lecteur = t.elapsed();
    assert_eq!(seen.lock().unwrap().len(), N, "chaque fragment lu");
    assert_eq!(text.len(), 2 * N);

    let canal = per(avec_lecteur) - per(sans_lecteur);
    eprintln!(
        "[mesure] {N} fragments, profil {} : puits nu {:.0} ns/jeton ; LlmNode sans lecteur {:.0} ns/jeton ; \
         avec lecteur PerElement {:.0} ns/jeton ; le canal coûte {:.0} ns/jeton",
        if cfg!(debug_assertions) { "debug" } else { "release" },
        per(nu),
        per(sans_lecteur),
        per(avec_lecteur),
        canal
    );
    assert!(per(avec_lecteur) < 20_000.0, "{:.0} ns par jeton avec lecteur", per(avec_lecteur));
}

#[cfg(feature = "openai-llm")]
pub(crate) mod local {
    use super::*;
    use std::io::{Read, Write};
    use std::process::{Child, Command, Stdio};

    /// Le serveur lancé par le témoin, arrêté par lui (son propre pid, jamais
    /// par nom), même sur un échec.
    pub(crate) struct Server(Child);

    impl Drop for Server {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    fn health(port: u16) -> bool {
        let Ok(mut s) = std::net::TcpStream::connect(("127.0.0.1", port)) else { return false };
        let _ = s.set_read_timeout(Some(Duration::from_secs(2)));
        if s.write_all(b"GET /health HTTP/1.0\r\nHost: 127.0.0.1\r\n\r\n").is_err() {
            return false;
        }
        let mut r = String::new();
        let _ = s.read_to_string(&mut r);
        r.starts_with("HTTP/1.1 200") || r.starts_with("HTTP/1.0 200")
    }

    /// **Lance `llama-server`** avec qwen3-1.7b (ou `RAG3WEAVER_TEMOIN_MODELE`),
    /// sur le processeur, et l'attend : le serveur, arrêté à sa chute, et son
    /// port. Partagé par les témoins à vrai modèle.
    pub(crate) fn start_llama() -> (Server, u16) {
        let model = std::env::var("RAG3WEAVER_TEMOIN_MODELE").unwrap_or_else(|_| {
            format!("{}/models/qwen3-1.7b/Qwen3-1.7B-Q8_0.gguf", std::env::var("HOME").unwrap())
        });
        assert!(std::path::Path::new(&model).exists(), "pas de modèle à {model}");
        let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
        let log = std::env::temp_dir().join(format!("temoin-llama-{port}.log"));
        let child = Command::new("llama-server")
            .args(["-m", &model, "--host", "127.0.0.1", "--port", &port.to_string()])
            .args(["-c", "4096", "-ngl", "0", "-t", "6", "--reasoning-budget", "0"])
            .stdout(Stdio::null())
            .stderr(std::fs::File::create(&log).unwrap())
            .spawn()
            .expect("llama-server introuvable");
        let server = Server(child);
        let depart = Instant::now();
        while !health(port) {
            assert!(depart.elapsed() < Duration::from_secs(180), "llama-server pas prêt en 180 s ({})", log.display());
            std::thread::sleep(Duration::from_millis(250));
        }
        eprintln!("[témoin] llama-server prêt en {:?}", depart.elapsed());
        (server, port)
    }

    /// Note l'instant du premier fragment reçu du serveur, avant le graphe.
    struct Timed {
        inner: Arc<dyn Llm>,
        first: Arc<Mutex<Option<Instant>>>,
    }

    struct TimedSink<'a> {
        inner: &'a mut dyn TokenSink,
        first: Arc<Mutex<Option<Instant>>>,
    }

    impl TokenSink for TimedSink<'_> {
        fn on_token(&mut self, delta: &str) -> Flow {
            self.first.lock().unwrap().get_or_insert_with(Instant::now);
            self.inner.on_token(delta)
        }
        fn on_reasoning(&mut self, delta: &str) -> Flow {
            self.inner.on_reasoning(delta)
        }
        fn on_finish(&mut self, reason: &Finish) {
            self.inner.on_finish(reason)
        }
    }

    impl Llm for Timed {
        fn generate(
            &self,
            turns: &[crate::llm::Turn],
            opts: &GenOptions,
            sink: &mut dyn TokenSink,
        ) -> Result<(Finish, Usage), crate::llm::LlmError> {
            self.inner.generate(turns, opts, &mut TimedSink { inner: sink, first: self.first.clone() })
        }
        fn context_len(&self) -> usize {
            self.inner.context_len()
        }
        fn name(&self) -> &str {
            self.inner.name()
        }
    }

    /// **Le vrai modèle** (témoin 5) : qwen3-1.7b sous `llama-server`, sur le
    /// processeur (`-ngl 0`, six fils : la carte reste à Lucie), une réponse de
    /// quelques centaines de jetons. Le premier jeton arrive au consommateur
    /// bien avant la fin de la génération, à quelques millisecondes de son
    /// arrivée du serveur, et `text` est la concaténation exacte des jetons lus.
    #[test]
    #[ignore = "lance llama-server et qwen3-1.7b"]
    fn local_model_tokens_reach_the_consumer_before_the_end() {
        let (_server, port) = start_llama();

        let first = Arc::new(Mutex::new(None));
        let llm: Arc<dyn Llm> = Arc::new(Timed {
            inner: Arc::new(crate::openai_llm::OpenAiLlm::new(format!("http://127.0.0.1:{port}/v1"), "qwen3")),
            first: first.clone(),
        });
        let seen = Arc::new(Mutex::new(Vec::new()));
        let opts = GenOptions { max_tokens: 400, ..Default::default() };
        let mut g = graph(
            "Écris un texte d'environ deux cent cinquante mots sur les rivières de France. /no_think",
            opts,
            Some(Reader { seen: seen.clone() }),
        );
        let t0 = Instant::now();
        let text = run(llm, &mut g);
        let fin = t0.elapsed();

        let seen = seen.lock().unwrap();
        let n = seen.len();
        assert!(n >= 100, "{n} jetons seulement : la réponse est trop courte pour juger");
        let lu: String = seen.iter().map(|(_, t)| t.as_str()).collect();
        assert_eq!(text, lu, "text est la concaténation des jetons lus");
        let premier_serveur = first.lock().unwrap().expect("un premier jeton du serveur") - t0;
        let premier_lu = seen[0].0 - t0;
        let dernier_lu = seen[n - 1].0 - t0;
        let ecart = premier_lu.saturating_sub(premier_serveur);
        eprintln!(
            "[témoin] {n} jetons ; premier reçu du serveur {premier_serveur:?}, premier lu par le consommateur \
             {premier_lu:?} (écart {ecart:?}) ; dernier lu {dernier_lu:?} ; run {fin:?}"
        );
        assert!(premier_lu * 4 < dernier_lu, "le premier jeton n'arrive pas bien avant la fin : {premier_lu:?} sur {dernier_lu:?}");
        assert!(ecart < Duration::from_millis(50), "le graphe retient le premier jeton {ecart:?}");
    }
}
