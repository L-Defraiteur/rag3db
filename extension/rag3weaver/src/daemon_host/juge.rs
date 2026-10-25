//! **Le juge de session** (page 04 du 25 octobre 2026) : un agent déclaré
//! (`templates/agents/juge/`) qui relit un run par son journal, rejoue un outil
//! qui ne fait que lire, ouvre le résultat, se compare aux bilans précédents de
//! la même tâche, puis rend son verdict dans une forme fixe — le dernier pas,
//! pas le juge. L'hôte range le verdict en fiche de bilan dans la mémoire que
//! le backend cible déclare (`bilans.memoire`) et propose les tickets.
//!
//! Le juge tourne sans bus de backend : ses runs ne sont jamais journalisés,
//! donc jamais cibles.

use super::*;
use crate::agent::ToolBox;
use crate::chat::{ChatConfig, ChatRunOptions, ChatSession, EventHandler};
use crate::llm::{GenOptions, ResponseFormat, StringSink, ToolCall, Turn};
use crate::tools::ToolDef;
use std::sync::atomic::AtomicBool;

/// **La file des bilans** : la fin d'un tour y dépose `(backend, run)` quand le
/// backend déclare `bilans` ; un fil de fond tenu par le serveur
/// ([`DaemonHost::servir_les_bilans`]) les juge un par un. Un bilan qu'il faut
/// demander n'existerait pas (c7).
pub struct FileDesBilans {
    envoi: std::sync::mpsc::Sender<(String, String)>,
    reception: Mutex<Option<std::sync::mpsc::Receiver<(String, String)>>>,
}

impl Default for FileDesBilans {
    fn default() -> Self {
        let (envoi, reception) = std::sync::mpsc::channel();
        Self { envoi, reception: Mutex::new(Some(reception)) }
    }
}

/// Ce qu'un outil du juge rend au plus au modèle.
const BORNE: usize = 24 * 1024;

fn borner(texte: String) -> String {
    if texte.len() <= BORNE {
        return texte;
    }
    let mut coupe = BORNE;
    while !texte.is_char_boundary(coupe) {
        coupe -= 1;
    }
    format!("{}… [{} octets de plus]", &texte[..coupe], texte.len() - coupe)
}

/// **Le nom d'une fiche de bilan**, stable (96) : `bilan : ` puis l'intitulé
/// en minuscules, espaces repliés, ponctuation de bord retirée. Une seule
/// fonction : deux formulations rangées par deux fonctions feraient deux fiches.
pub fn nom_de_tache(intitule: &str) -> String {
    let replie = intitule.to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ");
    let nu = replie.trim_matches(|c: char| !c.is_alphanumeric());
    format!("bilan : {nu}")
}

/// Le gabarit livré du juge : son `ChatConfig` et le schéma de son verdict.
fn gabarit() -> Result<(ChatConfig, Value), String> {
    let lire = |nom: &str| -> Result<Value, String> {
        let octets = crate::builtin_templates::get(nom).ok_or_else(|| format!("le gabarit livré {nom} manque"))?;
        serde_json::from_slice(octets).map_err(|e| format!("{nom} : {e}"))
    };
    let config: ChatConfig = serde_json::from_value(lire("agents/juge/juge.json")?).map_err(|e| format!("agents/juge/juge.json : {e}"))?;
    Ok((config, lire("agents/juge/bilan.schema.json")?))
}

/// **Les outils du juge** : sur le run cible, son backend et la mémoire.
struct OutilsDuJuge<'a> {
    host: &'a DaemonHost,
    cible: Arc<Hosted>,
    journal: &'a Value,
    memoire: Arc<Hosted>,
    fiche: &'a str,
}

impl OutilsDuJuge<'_> {
    fn lire_le_run(&self, args: &Value) -> Result<Value, String> {
        let outil = args["outil"].as_str().filter(|s| !s.is_empty());
        let evenements: Vec<&Value> = self.journal["evenements"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|e| e["kind"] != "Token")
            .filter(|e| outil.is_none_or(|o| e["tool"].as_str() == Some(o) || e["tool"].is_null()))
            .collect();
        Ok(json!({
            "run": self.journal["run"],
            "debut": self.journal["debut"],
            "fin": self.journal["fin"],
            "evenements": evenements,
            "tours": self.journal["tours"],
            "appel": self.journal["appel"],
            "tache": self.journal["tache"],
            "fichiers": self.journal["fichiers"],
            "propositions": self.journal["propositions"],
        }))
    }

    /// Rejoue un appel du run, mêmes arguments, même backend — seulement un
    /// outil qui ne fait que lire.
    fn rejouer_un_outil(&self, args: &Value) -> Result<Value, String> {
        let id = args["call_id"].as_str().ok_or("call_id manque")?;
        let appel = self.journal["evenements"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|e| e["kind"] == "ToolCallStarted" && e["call_id"] == id)
            .ok_or_else(|| format!("aucun appel {id} dans ce run"))?;
        let outil = appel["tool"].as_str().unwrap_or_default().to_string();
        let arguments: Value = serde_json::from_str(appel["arguments"].as_str().unwrap_or("{}")).unwrap_or(json!({}));
        self.host
            .with_backend(&self.cible, |b| {
                let prepared = b.prepared();
                match prepared.manifest.tools.get(&outil) {
                    Some(t) if t.policy.write_files || t.policy.run_commands => {
                        Err(format!("« {outil} » écrit ou lance des commandes : le juge ne rejoue que ce qui lit"))
                    }
                    // Un outil des déclarations (propose, declare…) n'est pas au manifeste.
                    None => Err(format!("« {outil} » n'est pas un outil du backend : le juge ne le rejoue pas")),
                    Some(_) => b.call(&outil, arguments.clone()).map(|r| json!({ "outil": outil, "arguments": arguments, "rendu": r["result"] })),
                }
            })
            .and_then(|r| r)
    }

    fn ouvrir_le_resultat(&self) -> Result<Value, String> {
        let tache = &self.journal["tache"];
        let relecture = match (tache["parent"].as_str(), tache["nom"].as_str()) {
            (Some(parent), Some(nom)) => self.host.relecture(parent, nom).unwrap_or_else(|e| json!({ "erreur": e })),
            _ => Value::Null,
        };
        Ok(json!({
            "tache": tache,
            "relecture": relecture,
            "fichiers": self.journal["fichiers"],
            "propositions": self.journal["propositions"],
            "appel": self.journal["appel"],
        }))
    }

    fn bilans_precedents(&self) -> Result<Value, String> {
        let options = json!({ "limit": 5, "filter_condition": { "field": { "key": "fiche", "value": self.fiche } } });
        self.host
            .with_backend(&self.memoire, |b| b.call("recall_apports", json!({ "query": self.fiche, "options": options })))
            .and_then(|r| r)
            .map(|r| json!({ "fiche": self.fiche, "apports": r["result"] }))
    }
}

impl ToolBox for OutilsDuJuge<'_> {
    fn call(&self, call: &ToolCall) -> Turn {
        let args: Value = serde_json::from_str(&call.arguments).unwrap_or(json!({}));
        let rendu = match call.name.as_str() {
            "lire_le_run" => self.lire_le_run(&args),
            "rejouer_un_outil" => self.rejouer_un_outil(&args),
            "ouvrir_le_resultat" => self.ouvrir_le_resultat(),
            "bilans_precedents" => self.bilans_precedents(),
            autre => Err(format!("le juge n'a pas d'outil « {autre} »")),
        };
        let contenu = match rendu {
            Ok(v) => borner(v.to_string()),
            Err(e) => json!({ "ok": false, "error": e }).to_string(),
        };
        Turn::tool_result(&call.id, &call.name, contenu)
    }

    fn tool_defs(&self) -> Vec<ToolDef> {
        let def = |name: &str, description: &str, parameters: Value| ToolDef { name: name.into(), description: description.into(), parameters };
        let rien = json!({ "type": "object", "properties": {}, "additionalProperties": false });
        vec![
            def(
                "lire_le_run",
                "Le run cible : ses appels (arguments exacts), leurs fins (ok, durée, taille), les appels au modèle (jetons, reprises), les tours tels que l'agent les a reçus, la tâche, les fichiers écrits, les propositions. `outil` le restreint à un outil.",
                json!({ "type": "object", "properties": { "outil": { "type": "string" } }, "additionalProperties": false }),
            ),
            def(
                "rejouer_un_outil",
                "Rejoue un appel du run (par son call_id), mêmes arguments, même backend, pour voir ce que l'agent a reçu. Seulement un outil qui ne fait que lire.",
                json!({ "type": "object", "properties": { "call_id": { "type": "string" } }, "required": ["call_id"], "additionalProperties": false }),
            ),
            def("ouvrir_le_resultat", "Le résultat du run : la relecture de la tâche (fichiers, diff), les propositions, les fichiers écrits, ou ce qu'un appel a rendu.", rien.clone()),
            def("bilans_precedents", "Les bilans précédents de la même tâche, dans la mémoire, pour comparer.", rien),
        ]
    }
}

impl DaemonHost {
    /// **Juge un run** d'un backend hébergé (page 04) : rend la fiche, l'apport
    /// et le verdict, et les tickets proposés. Refusé, en le disant, si le
    /// backend ne déclare pas `bilans` ou si le run n'a pas de journal.
    pub fn juger(&self, backend_id: &str, run: &str) -> Result<Value, String> {
        // Le juge ne juge jamais ses propres runs (ils n'ont pas de journal ; et
        // un appel qui le demanderait est refusé en le disant).
        if run.starts_with("juge-") {
            return Err(format!("{run} est un run du juge : il ne se juge pas lui-même"));
        }
        let cible = self.hote_par_id(backend_id)?;
        let (memoire_manifeste, modele) = self.with_backend(&cible, |b| {
            let p = b.prepared();
            let memoire = p.manifest.bilans.as_ref().map(|d| if d.memoire.is_absolute() { d.memoire.clone() } else { p.directory.join(&d.memoire) });
            (memoire, (self.juge_model)(b))
        })?;
        // Le manifeste jugé, par son nom de fichier dans son dossier : ce que
        // ses tickets proposent de changer.
        let propre = cible.manifest.file_name().map(|f| f.to_string_lossy().to_string());
        let memoire_manifeste = memoire_manifeste.ok_or("ce backend ne déclare pas `bilans` : rien ne range ses bilans")?;
        let llm = modele.map_err(|why| format!("le juge n'a pas de modèle : {why}"))?;
        let chemin = self.journal.fichier(backend_id, run).ok_or_else(|| format!("le run {run} n'a pas de journal (pas fini, ou effacé)"))?;
        let journal: Value = serde_json::from_slice(&std::fs::read(&chemin).map_err(|e| format!("{} : {e}", chemin.display()))?).map_err(|e| e.to_string())?;
        let memoire = self.hosted(&memoire_manifeste)?;

        // La tâche ciblee : le premier message de l'agent, sinon l'outil appelé.
        let intitule = journal["tours"]
            .as_array()
            .and_then(|t| t.iter().find(|t| t["role"] == "user"))
            .and_then(|t| t["content"].as_str())
            .map(str::to_string)
            .or_else(|| journal["appel"]["outil"].as_str().map(|o| format!("outil {o}")))
            .unwrap_or_else(|| format!("run {run}"));
        let fiche = nom_de_tache(&intitule);

        // L'enquête : la boucle de l'agent juge, sur ses outils.
        let (mut config, schema) = gabarit()?;
        let state_dir = self.chats_root.join("juge");
        std::fs::create_dir_all(&state_dir).map_err(|e| format!("{} : {e}", state_dir.display()))?;
        config.state_dir = state_dir;
        let outils = OutilsDuJuge { host: self, cible: cible.clone(), journal: &journal, memoire: memoire.clone(), fiche: &fiche };
        // Le message système vit dans la session (comme pour le chat de l'hôte) :
        // `run_with` ne le prend pas de la config.
        let mut session = ChatSession::default();
        session.turns.insert(0, Turn::system(&config.system_prompt));
        let debut = std::time::Instant::now();
        let silencieux: EventHandler = Arc::new(|_| {});
        let enquete = session.run_with(
            &format!("Juge ce run : {run}, sur la tâche « {intitule} ». Enquête, puis dis en une ligne que tu as assez."),
            llm.as_ref(),
            &outils,
            &config,
            silencieux,
            Arc::new(AtomicBool::new(false)),
            ChatRunOptions { bus: None, run_id: Some(format!("juge-{run}")), meter: Some(self.chat_meter.clone()), stopped_by: None },
        )?;

        // Le dernier pas : le verdict, dans la forme fixe du schéma.
        // La session ne garde pas le message système : le verdict le reprend, pour
        // que le modèle le rende en juge.
        let mut tours = session.turns.clone();
        if tours.first().is_none_or(|t| t.role != "system") {
            tours.insert(0, Turn::system(&config.system_prompt));
        }
        tours.push(Turn::user("Rends maintenant ton verdict, entier, dans la forme demandée."));
        let opts = GenOptions {
            max_tokens: config.max_output_tokens,
            response_format: Some(ResponseFormat::strict_schema("bilan", schema)),
            ..Default::default()
        };
        let mut sink = StringSink::default();
        llm.generate(&tours, &opts, &mut sink).map_err(|e| format!("le verdict du juge : {e:?}"))?;
        let verdict: Value = serde_json::from_str(sink.text.trim()).map_err(|e| format!("le verdict n'est pas du JSON : {e} — {}", sink.text))?;
        let juge_ms = debut.elapsed().as_millis() as u64;

        let cout = cout_du_run(&journal);
        let texte = rendre_le_bilan(&verdict, &cout, run, juge_ms);
        let ecrit = self.ecrire_le_bilan(&memoire, &fiche, &intitule, run, &journal, &texte, &verdict)?;
        let tickets = self.proposer_les_tickets(&cible, propre.as_deref(), &verdict, &fiche);
        Ok(json!({
            "fiche": fiche,
            "intitule": intitule,
            "source": ecrit["source"],
            "verdict": verdict,
            "cout": cout,
            "tickets": tickets,
            "enquete": { "iterations": enquete.iterations, "ms": juge_ms },
        }))
    }

    /// **Les bilans d'un backend**, pour la vue « bilans » de l'IDE : ses fiches
    /// (`via: juge`) dans la mémoire qu'il déclare, chacune avec ses apports dans
    /// l'ordre et la note de chacun (lue dans son bloc json), d'où la tendance.
    pub fn bilans_de(&self, backend_id: &str) -> Result<Value, String> {
        let cible = self.hote_par_id(backend_id)?;
        let memoire = self.with_backend(&cible, |b| {
            let p = b.prepared();
            p.manifest.bilans.as_ref().map(|d| if d.memoire.is_absolute() { d.memoire.clone() } else { p.directory.join(&d.memoire) })
        })?;
        let Some(memoire) = memoire else {
            return Ok(json!({ "declare": false, "fiches": [] }));
        };
        let memoire = self.hosted(&memoire)?;
        let appeler = |outil: &str, args: Value| -> Result<Value, String> { self.with_backend(&memoire, |b| b.call(outil, args)).and_then(|r| r) };
        let lignes = |r: &Value| -> Vec<Value> {
            match &r["result"] {
                Value::Array(a) => a.clone(),
                Value::Object(o) => o.get("results").and_then(Value::as_array).cloned().unwrap_or_default(),
                _ => Vec::new(),
            }
        };
        let fiches = appeler("recall_fiches", json!({ "query": "bilan", "options": { "limit": 100, "filter_condition": { "field": { "key": "via", "value": "juge" } } } }))?;
        let mut sortie = Vec::new();
        for f in lignes(&fiches) {
            let data = if f["data"].is_object() { f["data"].clone() } else { f.clone() };
            let Some(nom) = data["name"].as_str().map(str::to_string) else { continue };
            let apports = appeler("recall_apports", json!({ "query": "bilan", "options": { "limit": 100, "filter_condition": { "field": { "key": "fiche", "value": nom } } } }))?;
            let mut liste: Vec<Value> = lignes(&apports)
                .into_iter()
                .map(|a| if a["data"].is_object() { a["data"].clone() } else { a })
                .map(|a| {
                    let texte = a["texte"].as_str().unwrap_or_default().to_string();
                    let verdict = texte
                        .rsplit_once("```json\n")
                        .and_then(|(_, j)| j.split_once("\n```"))
                        .and_then(|(j, _)| serde_json::from_str::<Value>(j).ok())
                        .unwrap_or(Value::Null);
                    json!({ "source": a["source"], "creee": a["created_at"], "texte": texte, "note": verdict["note"], "comparaison": verdict["comparaison"] })
                })
                .collect();
            liste.sort_by_key(|a| a["creee"].as_i64().unwrap_or(0));
            sortie.push(json!({ "fiche": nom, "about": data["about"], "apports": liste }));
        }
        Ok(json!({ "declare": true, "fiches": sortie }))
    }

    /// Le modèle du juge (un faux pour les témoins).
    pub fn with_juge_model(mut self, model: chat::ChatModel) -> Self {
        self.juge_model = model;
        self
    }

    /// Dépose un bilan à faire, si le backend déclare `bilans` (la fin d'un tour).
    pub(super) fn demander_un_bilan(&self, hosted: &Arc<Hosted>, run: &str) {
        let declare = self.with_backend(hosted, |b| b.prepared().manifest.bilans.is_some()).unwrap_or(false);
        if declare {
            let _ = self.bilans.envoi.send((hosted.id.clone(), run.to_string()));
        }
    }

    /// **Le fil des bilans**, lancé une fois par le serveur : il attend que le
    /// journal du run soit complet (ses tours ajoutés), puis le juge ; un refus
    /// ou une panne se disent, et le fil continue. Il finit avec l'hôte.
    pub fn servir_les_bilans(self: &Arc<Self>) {
        let Some(reception) = self.bilans.reception.lock().unwrap_or_else(|p| p.into_inner()).take() else { return };
        let hote = Arc::downgrade(self);
        let _ = std::thread::Builder::new().name("juge".into()).spawn(move || {
            for (backend, run) in reception {
                let Some(h) = hote.upgrade() else { break };
                let fin = std::time::Instant::now() + std::time::Duration::from_secs(30);
                while !h
                    .journal
                    .fichier(&backend, &run)
                    .and_then(|p| std::fs::read(p).ok())
                    .and_then(|o| serde_json::from_slice::<Value>(&o).ok())
                    .is_some_and(|v| v["tours"].is_array() || v["appel"].is_object())
                {
                    if std::time::Instant::now() > fin {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                match h.juger(&backend, &run) {
                    Ok(v) => eprintln!("[juge] {backend} {run} : {} (note {})", v["fiche"], v["verdict"]["note"]),
                    Err(e) => eprintln!("[juge] {backend} {run} : {e}"),
                }
            }
        });
    }

    /// La fiche (créée si besoin, `via: juge`), le L0 (le journal, au chemin),
    /// le L1 de section `bilan`, son lien, et une ancre par outil appelé (96).
    fn ecrire_le_bilan(&self, memoire: &Arc<Hosted>, fiche: &str, intitule: &str, run: &str, journal: &Value, texte: &str, verdict: &Value) -> Result<Value, String> {
        let appeler = |outil: &str, args: Value| -> Result<Value, String> { self.with_backend(memoire, |b| b.call(outil, args)).and_then(|r| r) };
        // `get_fiche` rend une réponse même quand la fiche n'existe pas : elle
        // existe si la réponse porte son nom.
        let nom = json!({ "name": fiche }).to_string();
        let nom = &nom[1..nom.len() - 1];
        let existe = appeler("get_fiche", json!({ "record": { "name": fiche } })).is_ok_and(|r| r["result"].to_string().contains(nom));
        if !existe {
            appeler("put_fiche", json!({ "record": { "name": fiche, "about": intitule, "state": "current", "via": "juge" } }))?;
        }
        let jour = chrono_jour();
        let chemin = format!("sources/{jour}-{}.md", run.replace(|c: char| !c.is_ascii_alphanumeric() && c != '-', "-"));
        let contenu = format!("# Journal du run {run}\n\n```json\n{}\n```\n", serde_json::to_string_pretty(journal).unwrap_or_default());
        appeler(
            "put_source",
            json!({ "record": { "chemin": chemin, "titre": format!("le run {run}"), "contenu": contenu, "origine": "envoi", "session": run, "via": "juge" } }),
        )?;
        let texte_l1 = format!("{texte}\n\n```json\n{}\n```\n", serde_json::to_string_pretty(verdict).unwrap_or_default());
        appeler(
            "put_apport",
            json!({ "record": { "fiche": fiche, "source": chemin, "section": "bilan", "texte": texte_l1, "niveau": "L1", "state": "current", "via": "juge" } }),
        )?;
        appeler("relier_apport", json!({ "links": [{ "from": { "name": fiche }, "to": { "fiche": fiche, "source": chemin, "section": "bilan" } }] }))?;
        // Une ancre par outil appelé : `cited_by(value: "outil:<nom>")` rend ses bilans.
        let _ = appeler("create_ref_type", json!({ "genre": "outil" }));
        let mut outils: Vec<String> = journal["evenements"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|e| e["kind"] == "ToolCallStarted")
            .filter_map(|e| e["tool"].as_str().map(str::to_string))
            .chain(journal["appel"]["outil"].as_str().map(str::to_string))
            .collect();
        outils.sort();
        outils.dedup();
        for o in &outils {
            if let Err(e) = appeler("ancrer_fiche", json!({ "genre": "outil", "valeur": format!("outil:{o}"), "fiche": { "name": fiche } })) {
                eprintln!("[juge] l'ancre outil:{o} de « {fiche} » : {e}");
            }
        }
        Ok(json!({ "source": chemin, "ancres": outils }))
    }

    /// Un ticket par description à corriger : une proposition sur la clé
    /// `description` de l'outil dans le manifeste cible (les `.mmd` partagés
    /// sont hors de son dossier), appliquée ou rejetée comme les autres.
    fn proposer_les_tickets(&self, cible: &Arc<Hosted>, manifeste: Option<&str>, verdict: &Value, fiche: &str) -> Vec<Value> {
        let Some(manifeste) = manifeste else { return Vec::new() };
        let mut tickets = Vec::new();
        for a in verdict["a_corriger"].as_array().into_iter().flatten() {
            let (Some(outil), Some(proposee)) = (a["outil"].as_str(), a["proposee"].as_str()) else { continue };
            let fait = self.with_backend(cible, |b| {
                let p = b.prepared();
                if !p.manifest.tools.contains_key(outil) {
                    return Err(format!("« {outil} » n'est pas un outil de ce manifeste"));
                }
                let lu = std::fs::read_to_string(p.directory.join(manifeste)).map_err(|e| e.to_string())?;
                let mut m: Value = serde_json::from_str(&lu).map_err(|e| e.to_string())?;
                m["tools"][outil]["description"] = json!(proposee);
                let texte = serde_json::to_string_pretty(&m).map_err(|e| e.to_string())? + "\n";
                let raison = format!("le juge ({fiche}) : « {} » a trompé l'agent", a["phrase"].as_str().unwrap_or(""));
                b.proposer(&[(manifeste.to_string(), texte)], &raison, "juge").map(|p| json!({ "outil": outil, "proposition": p.id }))
            });
            match fait.and_then(|r| r) {
                Ok(t) => tickets.push(t),
                Err(e) => tickets.push(json!({ "outil": outil, "refus": e })),
            }
        }
        tickets
    }
}

/// Le coût du run, par slug et par unité, lu dans ses `LlmCall` et `Consumed`.
fn cout_du_run(journal: &Value) -> Value {
    let mut cout: std::collections::BTreeMap<String, i64> = std::collections::BTreeMap::new();
    for e in journal["evenements"].as_array().into_iter().flatten() {
        if e["kind"] == "LlmCall" {
            let modele = e["model"].as_str().unwrap_or("?");
            for unite in ["prompt_tokens", "cached_prompt_tokens", "completion_tokens"] {
                *cout.entry(format!("{modele} {unite}")).or_default() += e[unite].as_i64().unwrap_or(0);
            }
        }
    }
    json!(cout)
}

fn chrono_jour() -> String {
    let s = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let (j, _) = (s / 86_400, s % 86_400);
    // Le jour civil (algorithme de Howard Hinnant), sans dépendance.
    let z = j as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Le bilan lisible, tel que Lucie le lit dans la fiche : les rubriques en
/// prose courte.
fn rendre_le_bilan(v: &Value, cout: &Value, run: &str, juge_ms: u64) -> String {
    let liste = |cle: &str| -> String {
        let items: Vec<String> = v[cle].as_array().into_iter().flatten().filter_map(|x| x.as_str().map(|s| format!("- {s}"))).collect();
        if items.is_empty() { "- rien".into() } else { items.join("\n") }
    };
    let faits: Vec<String> = v["faits"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|f| format!("- ({}) {}", f["nature"].as_str().unwrap_or("?"), f["texte"].as_str().unwrap_or("")))
        .collect();
    let outils: Vec<String> = v["outils"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|o| {
            let ambigus: Vec<String> = o["ambigus"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|a| format!("\n  - retour ambigu : « {} » — l'agent a cru : {}", a["retour"].as_str().unwrap_or(""), a["cru"].as_str().unwrap_or("")))
                .collect();
            format!(
                "- `{}` : {} appel(s), {} échec(s), {} inutile(s){}",
                o["outil"].as_str().unwrap_or("?"),
                o["appels"],
                o["echecs"],
                o["inutiles"],
                ambigus.concat()
            )
        })
        .collect();
    let corriger: Vec<String> = v["a_corriger"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|a| format!("- `{}` : « {} » → « {} »", a["outil"].as_str().unwrap_or("?"), a["phrase"].as_str().unwrap_or(""), a["proposee"].as_str().unwrap_or("")))
        .collect();
    format!(
        "**Note {}/10** (confiance {}), objectif {}. {}\n\n\
         Comparaison : {} — {}\n\n\
         Faits :\n{}\n\nOutils :\n{}\n\nFrictions :\n{}\n\nAttentes :\n{}\n\nCe qui manquait :\n{}\n\nÀ corriger :\n{}\n\n\
         Coût : {}\n\n_Run {run} ; cible en {juge_ms} ms._",
        v["note"],
        v["confiance"].as_str().unwrap_or("?"),
        v["objectif"].as_str().unwrap_or("?"),
        v["resume"].as_str().unwrap_or(""),
        v["comparaison"]["verdict"].as_str().unwrap_or("?"),
        v["comparaison"]["sur"].as_str().unwrap_or(""),
        if faits.is_empty() { "- rien".into() } else { faits.join("\n") },
        if outils.is_empty() { "- aucun".into() } else { outils.join("\n") },
        liste("frictions"),
        liste("attentes"),
        liste("manquait"),
        if corriger.is_empty() { "- rien".into() } else { corriger.join("\n") },
        cout,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Le nom d'une fiche de bilan : une seule fonction (96).
    #[test]
    fn the_task_name_is_stable() {
        assert_eq!(nom_de_tache("  Dis SALUT  au lieu\nde bonjour ! "), "bilan : dis salut au lieu de bonjour");
        assert_eq!(nom_de_tache("« Écris une note. »"), "bilan : écris une note");
    }

    /// Le gabarit livré du juge se lit, et son schéma est strict.
    #[test]
    fn the_judge_template_reads() {
        let (config, schema) = gabarit().unwrap();
        assert_eq!(config.name, "juge");
        assert!(config.system_prompt.contains("rejouer_un_outil"));
        assert_eq!(schema["additionalProperties"], false);
        assert!(schema["required"].as_array().unwrap().iter().any(|r| r == "comparaison"));
    }
}
