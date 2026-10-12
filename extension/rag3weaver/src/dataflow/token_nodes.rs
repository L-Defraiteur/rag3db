//! **Les consommateurs des jetons** (page
//! `docs/12-octobre-2026-recherche/01-le-premier-flux-de-jetons.md`, §2.3) :
//! `TokenEvents` montre les fragments au fil de l'eau, `Spot` les classe au
//! fil de l'eau. Tous deux lisent un flux de textes (la sortie `tokens` de
//! `LlmNode`) élément par élément.

use std::collections::BTreeMap;
use std::sync::Arc;

use serde_json::{json, Value};

use super::node::{Node, NodeContext};
use super::node_registry::{ConfigParam, ConfigParamType, NodeFactory, NodeSchema};
use super::port::{take_or_clone, PortDef, PortKind, PortType, PortValue, Posture};

// ─── TokenEvents ─────────────────────────────────────────────────────────────

/// **Les fragments sur le bus, au fil de l'eau** : chaque élément de `tokens`
/// devient un `Event::Token` (sujet `tokens`, et `run.<id>`), avec le run, le
/// nom du flux et son rang. Une interface ou un agent appelant voit les jetons
/// d'un graphe comme ceux d'une conversation. Sans bus, le nœud lit sans rien
/// dire : brancher le flux suffit à garder le modèle en vie.
///
/// **Des paquets** (`batch_ms`, 0 par défaut : un événement par fragment) : les
/// fragments s'accumulent, et le paquet part avec le fragment qui dépasse le
/// délai depuis le premier du paquet, ou à la fin du flux. Le bus écrase les
/// plus anciens au-delà de sa capacité (64) : un écran plus lent que le modèle
/// se règle par son graphe, pas par le bus de tout le monde. Un écran qui reçoit
/// malgré tout `Overflowed(n)` le dit (« n fragments perdus, le texte entier
/// suit dans `text` ») : il ne le tait pas.
pub struct TokenEvents {
    name: String,
    stream: String,
    batch: std::time::Duration,
}

impl Node for TokenEvents {
    fn name(&self) -> &str {
        &self.name
    }
    fn node_type(&self) -> &str {
        "TokenEvents"
    }
    fn inputs(&self) -> Vec<PortDef> {
        super::node_registry::ports_declares(&TokenEventsFactory).0
    }
    fn outputs(&self) -> Vec<PortDef> {
        super::node_registry::ports_declares(&TokenEventsFactory).1
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let bus = ctx.service::<Arc<crate::events::EventBus>>("event_bus").cloned();
        let run = ctx.run_id().to_string();
        let stream = self.stream.clone();
        let emettre = |premier: u64, text: String, count: u64| {
            if let Some(bus) = &bus {
                bus.emit(crate::events::Event::Token { run: run.clone(), node: stream.clone(), seq: premier, text, count });
            }
        };
        let mut seq = 0u64;
        // Le paquet en cours : son premier rang, son texte, son compte, son départ.
        let mut paquet: Option<(u64, String, u64, std::time::Instant)> = None;
        while let Some(v) = ctx.next("tokens").map_err(|e| format!("TokenEvents : {e:?}"))? {
            let text = take_or_clone::<String>(v).ok_or("TokenEvents : 'tokens' porte des textes")?;
            if self.batch.is_zero() {
                emettre(seq, text, 1);
            } else {
                let p = paquet.get_or_insert_with(|| (seq, String::new(), 0, std::time::Instant::now()));
                p.1.push_str(&text);
                p.2 += 1;
                if p.3.elapsed() >= self.batch {
                    let (premier, texte, compte, _) = paquet.take().expect("posé plus haut");
                    emettre(premier, texte, compte);
                }
            }
            seq += 1;
        }
        if let Some((premier, texte, compte, _)) = paquet.take() {
            emettre(premier, texte, compte);
        }
        ctx.metric("tokens", seq as f64);
        ctx.set_output("report", PortValue::new(json!({"tokens": seq})));
        Ok(())
    }
}

pub struct TokenEventsFactory;

impl NodeFactory for TokenEventsFactory {
    fn create(&self, name: &str, config: &Value) -> Result<Box<dyn Node>, String> {
        let stream = match config.get("stream") {
            None | Some(Value::Null) => name.to_string(),
            Some(Value::String(s)) => s.clone(),
            Some(other) => return Err(format!("TokenEvents : 'stream' est un nom, pas {other}")),
        };
        let batch_ms = match config.get("batch_ms") {
            None | Some(Value::Null) => 0,
            Some(v) => v.as_u64().ok_or_else(|| format!("TokenEvents : 'batch_ms' est un entier, pas {v}"))?,
        };
        Ok(Box::new(TokenEvents { name: name.to_string(), stream, batch: std::time::Duration::from_millis(batch_ms) }))
    }
    fn node_type(&self) -> &str {
        "TokenEvents"
    }
    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "TokenEvents".into(),
            description: "Publie chaque fragment d'un flux de textes (la sortie `tokens` d'un LlmNode) sur le bus, \
                          sujet `tokens`, au format des jetons d'une conversation, avec le run, le flux et le rang"
                .into(),
            inputs: vec![PortDef::value("tokens", PortType::Text, true).posture(Posture::PerElement)],
            outputs: vec![PortDef::value("report", PortType::Map, false)],
            config_params: vec![
                ConfigParam {
                    name: "stream".into(),
                    param_type: ConfigParamType::String,
                    required: false,
                    default: None,
                    description: "le nom du flux dans les événements ; absent : le nom du nœud".into(),
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "batch_ms".into(),
                    param_type: ConfigParamType::Int,
                    required: false,
                    default: Some(json!(0)),
                    description: "réunir les fragments reçus dans ce délai en un seul événement (texte concaténé, \
                                  rang du premier, compte) ; 0 : un événement par fragment"
                        .into(),
                    choices: None,
                    json_schema: None,
                },
            ],
        }
    }
}

// ─── Spot ────────────────────────────────────────────────────────────────────

/// Un motif : un texte, ou une expression régulière (`re:…`).
enum Motif {
    Texte(String),
    #[cfg(feature = "code")]
    Regex(regex::Regex),
}

impl Motif {
    fn lire(label: &str, source: &str) -> Result<Self, String> {
        match source.strip_prefix("re:") {
            None if source.is_empty() => Err(format!("Spot : le motif de « {label} » est vide")),
            None => Ok(Motif::Texte(source.to_string())),
            #[cfg(feature = "code")]
            Some(re) => regex::Regex::new(re).map(Motif::Regex).map_err(|e| format!("Spot : le motif de « {label} » : {e}")),
            #[cfg(not(feature = "code"))]
            Some(_) => Err(format!("Spot : le motif de « {label} » est une expression régulière, qui demande la feature `code`")),
        }
    }

    /// Les occurrences (début, fin, en octets) dans `texte`, à partir de `depuis`.
    fn occurrences(&self, texte: &str, depuis: usize) -> Vec<(usize, usize)> {
        let reste = texte.get(depuis..).unwrap_or("");
        match self {
            Motif::Texte(t) => reste.match_indices(t.as_str()).map(|(i, m)| (depuis + i, depuis + i + m.len())).collect(),
            #[cfg(feature = "code")]
            Motif::Regex(r) => r.find_iter(reste).map(|m| (depuis + m.start(), depuis + m.end())).collect(),
        }
    }

    fn longueur(&self) -> usize {
        match self {
            Motif::Texte(t) => t.len(),
            #[cfg(feature = "code")]
            Motif::Regex(_) => 0,
        }
    }
}

/// **Classer au fil de l'eau** : une fenêtre glissante du texte reçu, des motifs
/// à étiquettes, et une étiquette émise sur `labels` (un flux) dès qu'un motif
/// apparaît — à cheval sur deux fragments compris, chaque occurrence une fois.
/// Une étiquette de `stop_on` fait **lâcher l'entrée** : avec `stop_when_unread`
/// sur le `LlmNode`, le modèle s'arrête alors, si personne d'autre ne lit ses
/// jetons. C'est le classement en cours d'arrivée qui ne coûte rien.
pub struct Spot {
    name: String,
    motifs: Vec<(String, Motif)>,
    window: usize,
    stop_on: Vec<String>,
}

impl Node for Spot {
    fn name(&self) -> &str {
        &self.name
    }
    fn node_type(&self) -> &str {
        "Spot"
    }
    fn inputs(&self) -> Vec<PortDef> {
        super::node_registry::ports_declares(&SpotFactory).0
    }
    fn outputs(&self) -> Vec<PortDef> {
        super::node_registry::ports_declares(&SpotFactory).1
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        // La fenêtre, et ce qu'elle a laissé tomber devant elle (en octets) :
        // une position absolue = `debut` + une position dans la fenêtre.
        let mut fenetre = String::new();
        let mut debut = 0usize;
        // La fin de la dernière occurrence dite, par étiquette : une même
        // occurrence ne se redit pas quand la fenêtre la garde.
        let mut fin_dite: BTreeMap<String, usize> = BTreeMap::new();
        let garde = self.motifs.iter().map(|(_, m)| m.longueur()).max().unwrap_or(0).max(self.window);
        while let Some(v) = ctx.next("tokens").map_err(|e| format!("Spot : {e:?}"))? {
            let fragment = take_or_clone::<String>(v).ok_or("Spot : 'tokens' porte des textes")?;
            fenetre.push_str(&fragment);
            let mut dites: Vec<(usize, String, usize, String)> = Vec::new();
            for (label, motif) in &self.motifs {
                let deja = fin_dite.get(label).copied().unwrap_or(0).saturating_sub(debut);
                for (a, b) in motif.occurrences(&fenetre, deja.min(fenetre.len())) {
                    dites.push((debut + a, label.clone(), debut + b, fenetre[a..b].to_string()));
                }
            }
            // Dans l'ordre du texte, quelle que soit l'étiquette.
            dites.sort();
            for (at, label, fin, texte) in dites {
                if fin <= fin_dite.get(&label).copied().unwrap_or(0) {
                    continue;
                }
                fin_dite.insert(label.clone(), fin);
                if ctx.emit("labels", PortValue::new(json!({"label": label, "at": at, "match": texte}))).is_err() {
                    // Plus personne ne lit les étiquettes : rien de plus à dire.
                    return Ok(());
                }
                if self.stop_on.contains(&label) {
                    // Lâcher l'entrée : le producteur voit qu'on ne lit plus.
                    return Ok(());
                }
            }
            // La fenêtre ne garde que sa queue, à une frontière de caractère.
            if fenetre.len() > garde {
                let mut coupe = fenetre.len() - garde;
                while !fenetre.is_char_boundary(coupe) {
                    coupe += 1;
                }
                fenetre.drain(..coupe);
                debut += coupe;
            }
        }
        Ok(())
    }
}

pub struct SpotFactory;

impl NodeFactory for SpotFactory {
    fn create(&self, name: &str, config: &Value) -> Result<Box<dyn Node>, String> {
        let motifs = match config.get("patterns") {
            Some(Value::Object(m)) if !m.is_empty() => m
                .iter()
                .map(|(label, v)| {
                    let source = v.as_str().ok_or_else(|| format!("Spot : le motif de « {label} » est un texte"))?;
                    Ok((label.clone(), Motif::lire(label, source)?))
                })
                .collect::<Result<Vec<_>, String>>()?,
            _ => return Err("Spot : 'patterns' est un objet non vide { étiquette: motif }".into()),
        };
        let window = match config.get("window") {
            None | Some(Value::Null) => 256,
            Some(v) => v.as_u64().filter(|n| *n > 0).ok_or("Spot : 'window' est un entier positif")? as usize,
        };
        let stop_on: Vec<String> = match config.get("stop_on") {
            None | Some(Value::Null) => Vec::new(),
            Some(Value::Array(a)) => a
                .iter()
                .map(|v| v.as_str().map(str::to_string).ok_or("Spot : 'stop_on' est une liste d'étiquettes"))
                .collect::<Result<_, _>>()?,
            Some(_) => return Err("Spot : 'stop_on' est une liste d'étiquettes".into()),
        };
        if let Some(inconnue) = stop_on.iter().find(|l| !motifs.iter().any(|(m, _)| m == *l)) {
            return Err(format!("Spot : 'stop_on' cite « {inconnue} », qui n'est pas une étiquette de 'patterns'"));
        }
        Ok(Box::new(Spot { name: name.to_string(), motifs, window, stop_on }))
    }
    fn node_type(&self) -> &str {
        "Spot"
    }
    fn schema(&self) -> NodeSchema {
        let p = |name: &'static str, param_type: ConfigParamType, description: &'static str| ConfigParam {
            name: name.into(),
            param_type,
            required: name == "patterns",
            default: None,
            description: description.into(),
            choices: None,
            json_schema: None,
        };
        NodeSchema {
            node_type: "Spot".into(),
            description: "Repère des motifs dans un flux de textes au fil de l'eau (à cheval sur deux fragments \
                          compris) et émet une étiquette par occurrence ; une étiquette de stop_on lâche l'entrée"
                .into(),
            inputs: vec![PortDef::value("tokens", PortType::Text, true).posture(Posture::PerElement)],
            outputs: vec![PortDef::value("labels", PortType::Map, false).stream(PortKind::Finite)],
            config_params: vec![
                p("patterns", ConfigParamType::Json, "{ étiquette: motif } ; un motif est un texte, ou `re:` suivi d'une expression régulière"),
                p("window", ConfigParamType::Int, "la fenêtre glissante, en octets (256) ; au moins le plus long motif texte"),
                p("stop_on", ConfigParamType::Json, "des étiquettes qui font lâcher l'entrée dès qu'elles sont dites"),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataflow::port::{take_or_clone, PortKind, PortType, PortValue, Posture};
    use crate::dataflow::services::ServiceRegistry;
    use crate::dataflow::{DataflowGraph, DataflowRuntime};
    use crate::events::{topic, Event, EventBus};

    /// Un producteur de fragments : émet chaque texte sur `tokens`, un flux fini.
    struct Fragments {
        texts: Vec<&'static str>,
    }
    impl Node for Fragments {
        fn name(&self) -> &str {
            "llm"
        }
        fn node_type(&self) -> &str {
            "Fragments"
        }
        fn inputs(&self) -> Vec<PortDef> {
            vec![]
        }
        fn outputs(&self) -> Vec<PortDef> {
            vec![PortDef::value("tokens", PortType::Text, true).stream(PortKind::Finite)]
        }
        fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
            for t in &self.texts {
                ctx.emit("tokens", PortValue::new(t.to_string())).map_err(|e| format!("{e:?}"))?;
            }
            Ok(())
        }
    }

    /// Lit les étiquettes de `Spot`, élément par élément.
    struct Labels {
        seen: Arc<std::sync::Mutex<Vec<Value>>>,
    }
    impl Node for Labels {
        fn name(&self) -> &str {
            "étiquettes"
        }
        fn node_type(&self) -> &str {
            "Labels"
        }
        fn inputs(&self) -> Vec<PortDef> {
            vec![PortDef::value("labels", PortType::Map, true).posture(Posture::PerElement)]
        }
        fn outputs(&self) -> Vec<PortDef> {
            vec![]
        }
        fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
            while let Some(v) = ctx.next("labels").map_err(|e| format!("{e:?}"))? {
                self.seen.lock().unwrap().push(take_or_clone::<Value>(v).ok_or("une étiquette en JSON")?);
            }
            Ok(())
        }
    }

    fn services(bus: &Arc<EventBus>) -> ServiceRegistry {
        let mut s = ServiceRegistry::new();
        s.register("event_bus", bus.clone());
        s
    }

    /// **Chaque fragment sur le bus** : sur le sujet `tokens`, dans l'ordre
    /// (`seq`), avec le run et le nom du flux, au format des jetons d'une
    /// conversation (`event: token`).
    #[test]
    fn token_events_publishes_each_fragment_on_the_tokens_topic() {
        let bus = Arc::new(EventBus::new(256));
        let mut rx = bus.subscribe(topic::TOKENS);
        let mut g = DataflowGraph::new();
        g.add_node(Box::new(Fragments { texts: vec!["bon", "jour", " !"] })).unwrap();
        g.add_node(TokenEventsFactory.create("montrer", &json!({"stream": "réponse"})).unwrap()).unwrap();
        g.connect("llm", "tokens", "montrer", "tokens").unwrap();
        DataflowRuntime::with_services(10, services(&bus)).execute(&mut g).unwrap();
        let mut recus = Vec::new();
        while let Ok(e) = rx.try_recv() {
            if let Event::Token { node, seq, text, count, .. } = &e {
                recus.push((node.clone(), *seq, text.clone()));
                assert_eq!(e.to_json()["event"], "token");
                assert_eq!(*count, 1, "sans batch_ms, un événement par fragment");
            }
        }
        assert_eq!(
            recus,
            [("réponse".to_string(), 0, "bon".to_string()), ("réponse".to_string(), 1, "jour".to_string()), ("réponse".to_string(), 2, " !".to_string())]
        );
    }

    /// **Des paquets** (`batch_ms`) : les fragments reçus dans le délai partent
    /// en un seul événement, le texte concaténé, le rang du premier et le
    /// compte. Pour un écran plus lent que le modèle, que le bus (64)
    /// écraserait sinon (mesure de la recherche, page 01 §8).
    #[test]
    fn token_events_batches_fragments_within_batch_ms() {
        let bus = Arc::new(EventBus::new(256));
        let mut rx = bus.subscribe(topic::TOKENS);
        let mut g = DataflowGraph::new();
        g.add_node(Box::new(Fragments { texts: vec!["bon", "jour", " !"] })).unwrap();
        g.add_node(TokenEventsFactory.create("montrer", &json!({"stream": "réponse", "batch_ms": 60000})).unwrap()).unwrap();
        g.connect("llm", "tokens", "montrer", "tokens").unwrap();
        DataflowRuntime::with_services(10, services(&bus)).execute(&mut g).unwrap();
        let mut recus = Vec::new();
        while let Ok(e) = rx.try_recv() {
            if let Event::Token { seq, text, count, .. } = &e {
                recus.push((*seq, text.clone(), *count));
                assert_eq!(e.to_json()["count"], 3);
            }
        }
        assert_eq!(recus, [(0, "bonjour !".to_string(), 3)]);
    }

    /// **Un motif à cheval sur deux fragments est vu, une occurrence dite une
    /// fois** : « jour » arrive en « jo » + « ur », puis revient plus loin.
    #[test]
    fn spot_sees_a_pattern_across_fragments_once_per_occurrence() {
        let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut g = DataflowGraph::new();
        g.add_node(Box::new(Fragments { texts: vec!["bon", "jo", "ur à tous, ", "bonjour encore"] })).unwrap();
        g.add_node(SpotFactory.create("repérer", &json!({"patterns": {"salut": "jour"}})).unwrap()).unwrap();
        g.add_node(Box::new(Labels { seen: seen.clone() })).unwrap();
        g.connect("llm", "tokens", "repérer", "tokens").unwrap();
        g.connect("repérer", "labels", "étiquettes", "labels").unwrap();
        DataflowRuntime::new(10).execute(&mut g).unwrap();
        let labels: Vec<String> = seen.lock().unwrap().iter().map(|v| v["label"].as_str().unwrap().to_string()).collect();
        assert_eq!(labels, ["salut", "salut"], "{:?}", seen.lock().unwrap());
    }

    /// **`stop_on` lâche l'entrée, et le modèle s'arrête** (témoin 4 de la
    /// page) : avec `stop_when_unread`, `LlmNode` cesse de générer quand plus
    /// personne ne lit ses jetons. Un `TokenEvents` branché en parallèle garde
    /// le modèle en vie jusqu'au bout.
    #[test]
    fn spot_stop_on_lets_go_and_stops_the_model_unless_another_reader_stays() {
        for avec_un_autre_lecteur in [false, true] {
            let produits = Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let compte = produits.clone();
            let llm = crate::llm::CallbackLlm::new("cent", 4096, move |_, _, sink| {
                for i in 0..100 {
                    compte.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    let t = if i == 5 { "FIN ".to_string() } else { format!("t{i} ") };
                    if sink.on_token(&t) == crate::llm::Flow::Stop {
                        return Ok((crate::llm::Finish::cancelled(), Default::default()));
                    }
                    std::thread::sleep(std::time::Duration::from_millis(2));
                }
                Ok((crate::llm::Finish::eos(), Default::default()))
            });
            let bus = Arc::new(EventBus::new(1024));
            let mut s = services(&bus);
            s.register(crate::dataflow::llm_nodes::LLM_SERVICE, Arc::new(llm) as Arc<dyn crate::llm::Llm>);
            let mut g = DataflowGraph::new();
            g.add_node(Box::new(Prompt)).unwrap();
            g.add_node(crate::dataflow::llm_nodes::LlmNodeFactory.create("llm", &json!({"stop_when_unread": true})).unwrap()).unwrap();
            g.add_node(SpotFactory.create("repérer", &json!({"patterns": {"fin": "FIN"}, "stop_on": ["fin"]})).unwrap()).unwrap();
            g.connect("prompt", "out", "llm", "prompt").unwrap();
            g.connect("llm", "tokens", "repérer", "tokens").unwrap();
            if avec_un_autre_lecteur {
                g.add_node(TokenEventsFactory.create("montrer", &json!({})).unwrap()).unwrap();
                g.connect("llm", "tokens", "montrer", "tokens").unwrap();
            }
            DataflowRuntime::with_services(10, s).execute(&mut g).unwrap();
            let n = produits.load(std::sync::atomic::Ordering::SeqCst);
            if avec_un_autre_lecteur {
                assert_eq!(n, 100, "TokenEvents lit toujours : le modèle va au bout");
            } else {
                assert!(n < 100, "Spot a lâché après FIN : le modèle s'arrête ({n} produits)");
            }
        }
    }

    struct Prompt;
    impl Node for Prompt {
        fn name(&self) -> &str {
            "prompt"
        }
        fn node_type(&self) -> &str {
            "Prompt"
        }
        fn inputs(&self) -> Vec<PortDef> {
            vec![]
        }
        fn outputs(&self) -> Vec<PortDef> {
            vec![PortDef::value("out", PortType::Text, true)]
        }
        fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
            ctx.set_output("out", PortValue::new("parle".to_string()));
            Ok(())
        }
    }
}
