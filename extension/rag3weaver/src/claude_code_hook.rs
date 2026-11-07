//! **La commande mince des hooks de Claude Code** (`rag3weaver-backend hook`,
//! page `docs/07-novembre-2026-hooks-claude-code/01`, lot 1).
//!
//! Claude Code lance la commande à cinq moments (`SessionStart`,
//! `UserPromptSubmit`, `PreCompact`, `PostCompact`, `Stop`) avec un JSON sur
//! son entrée. La commande lit ce JSON, parle à l'hôte 7970 par le backend
//! mémoire déclaré, rend son bloc sur la sortie standard, et **sort toujours
//! en 0** : rien de ce qui se passe ici ne doit arrêter une session.
//!
//! Trois règles, toutes ici et aucune dans l'hôte :
//!
//! - **jamais bloquant** : pas de code 2, pas de `decision: block`, un délai
//!   propre plus court que celui de Claude Code ; au-delà, le bloc est vide et
//!   la raison va au journal ;
//! - **sans hôte, elle se tait** : l'hôte injoignable, le jeton absent, le
//!   backend qui refuse — rien sur la sortie, une ligne au journal
//!   (`~/.local/state/rag3weaver/hooks.log`). Elle ne démarre **jamais**
//!   d'hôte (c'est l'unité systemd qui le fait, avec le binaire installé) ;
//! - **vide quand il n'y a rien** : un bloc toujours présent apprend au
//!   modèle à ne plus le lire.
//!
//! Ce lot pose la mécanique : l'événement lu, le réglage, le client de l'hôte
//! borné, le journal, le silence. Les blocs rendus (lot 2) et les écritures
//! (lots 3 et 4) viennent dans [`repondre`], événement par événement.

use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Les événements que la commande connaît. Un autre est accepté et journalisé,
/// jamais une erreur : Claude Code en ajoutera.
pub const EVENEMENTS: [&str; 5] = ["SessionStart", "UserPromptSubmit", "PreCompact", "PostCompact", "Stop"];

/// Ce que Claude Code donne à la commande, dans la forme relevée le 7 novembre
/// 2026 (page 01 §1). Les champs qu'un événement n'a pas restent vides.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Evenement {
    pub nom: String,
    pub session_id: String,
    pub transcript_path: Option<PathBuf>,
    pub cwd: Option<PathBuf>,
    pub prompt_id: Option<String>,
    /// `SessionStart` : `startup`, `resume`, `clear`, `compact`, `fork`.
    pub source: Option<String>,
    /// `PreCompact` et `PostCompact` : `manual` ou `auto`.
    pub trigger: Option<String>,
    /// `UserPromptSubmit`.
    pub prompt: Option<String>,
    /// `PostCompact`.
    pub compact_summary: Option<String>,
    /// `Stop`.
    pub last_assistant_message: Option<String>,
    pub stop_hook_active: bool,
    /// `SessionStart` en `resume` ou `fork` : le contexte réel.
    pub context_tokens: Option<u64>,
}

impl Evenement {
    /// Lit le JSON du hook. Un JSON sans `hook_event_name` est refusé en le
    /// disant ; tout le reste est facultatif.
    pub fn lire(texte: &str) -> Result<Self, String> {
        let v: Value = serde_json::from_str(texte).map_err(|e| format!("l'entrée du hook n'est pas du JSON : {e}"))?;
        let nom = v["hook_event_name"].as_str().ok_or("l'entrée du hook n'a pas de `hook_event_name`")?.to_string();
        let chaine = |k: &str| v[k].as_str().filter(|s| !s.is_empty()).map(str::to_string);
        Ok(Self {
            session_id: chaine("session_id").unwrap_or_default(),
            transcript_path: chaine("transcript_path").map(PathBuf::from),
            cwd: chaine("cwd").map(PathBuf::from),
            prompt_id: chaine("prompt_id"),
            source: chaine("source"),
            trigger: chaine("trigger"),
            prompt: chaine("prompt"),
            compact_summary: chaine("compact_summary"),
            last_assistant_message: chaine("last_assistant_message"),
            stop_hook_active: v["stop_hook_active"].as_bool().unwrap_or(false),
            context_tokens: v["context_tokens"].as_u64(),
            nom,
        })
    }

    pub fn connu(&self) -> bool {
        EVENEMENTS.contains(&self.nom.as_str())
    }
}

/// **Le réglage** (`~/.config/rag3weaver/hooks.json`), posé par
/// `scripts/hooks_claude_code.sh`, lisible et modifiable à la main. Un champ
/// absent prend son défaut ; un fichier absent, tous les défauts.
#[derive(Debug, Clone, PartialEq)]
pub struct Reglage {
    /// Le manifeste du backend mémoire, tel que `~/.claude.json` le déclare.
    pub manifest: PathBuf,
    /// Les clés de la session auprès de l'hôte (`--keys` du client MCP).
    pub keys: Vec<String>,
    /// Le plafond de contexte en jetons (`RAG3WEAVER_HOOK_CONTEXT_TOKENS`).
    pub context_tokens: u64,
    /// La part du plafond où l'on suggère de ranger, une fois.
    pub suggest_ratio: f64,
    /// Les bornes des blocs, en caractères.
    pub prompt_block_chars: usize,
    pub session_block_chars: usize,
    /// Les délais propres, en millisecondes : `UserPromptSubmit`, les autres.
    pub prompt_timeout_ms: u64,
    pub timeout_ms: u64,
    pub journal: PathBuf,
}

impl Reglage {
    pub fn defaut() -> Self {
        let maison = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
        Self {
            manifest: maison.join(".config/rag3weaver/memoire-longue/backends/memory/backend.json"),
            keys: vec!["dev".into()],
            context_tokens: 1_000_000,
            suggest_ratio: 0.7,
            prompt_block_chars: 1_500,
            session_block_chars: 3_000,
            prompt_timeout_ms: 5_000,
            timeout_ms: 20_000,
            journal: maison.join(".local/state/rag3weaver/hooks.log"),
        }
    }

    /// Le chemin par défaut du réglage.
    pub fn chemin_par_defaut() -> PathBuf {
        std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(".")).join(".config/rag3weaver/hooks.json")
    }

    /// Lit un réglage ; les champs absents gardent leur défaut, un champ
    /// inconnu est une erreur dite (une faute de frappe ne doit pas se taire).
    pub fn lire(texte: &str) -> Result<Self, String> {
        let v: Value = serde_json::from_str(texte).map_err(|e| format!("hooks.json : {e}"))?;
        let o = v.as_object().ok_or("hooks.json : un objet attendu")?;
        let mut r = Self::defaut();
        for (k, val) in o {
            match k.as_str() {
                "manifest" => r.manifest = PathBuf::from(val.as_str().ok_or("hooks.json : `manifest` est un chemin")?),
                "keys" => r.keys = val.as_array().ok_or("hooks.json : `keys` est une liste")?.iter().filter_map(Value::as_str).map(str::to_string).collect(),
                "context_tokens" => r.context_tokens = val.as_u64().ok_or("hooks.json : `context_tokens` est un entier")?,
                "suggest_ratio" => r.suggest_ratio = val.as_f64().ok_or("hooks.json : `suggest_ratio` est un nombre")?,
                "prompt_block_chars" => r.prompt_block_chars = val.as_u64().ok_or("hooks.json : `prompt_block_chars` est un entier")? as usize,
                "session_block_chars" => r.session_block_chars = val.as_u64().ok_or("hooks.json : `session_block_chars` est un entier")? as usize,
                "prompt_timeout_ms" => r.prompt_timeout_ms = val.as_u64().ok_or("hooks.json : `prompt_timeout_ms` est un entier")?,
                "timeout_ms" => r.timeout_ms = val.as_u64().ok_or("hooks.json : `timeout_ms` est un entier")?,
                "journal" => r.journal = PathBuf::from(val.as_str().ok_or("hooks.json : `journal` est un chemin")?),
                autre => return Err(format!("hooks.json : champ inconnu `{autre}` (connus : manifest, keys, context_tokens, suggest_ratio, prompt_block_chars, session_block_chars, prompt_timeout_ms, timeout_ms, journal)")),
            }
        }
        if let Ok(v) = std::env::var("RAG3WEAVER_HOOK_CONTEXT_TOKENS") {
            r.context_tokens = v.trim().parse().map_err(|_| "RAG3WEAVER_HOOK_CONTEXT_TOKENS : un entier attendu".to_string())?;
        }
        Ok(r)
    }

    /// Lit le fichier ; absent, les défauts.
    pub fn charger(chemin: &Path) -> Result<Self, String> {
        match std::fs::read_to_string(chemin) {
            Ok(t) => Self::lire(&t),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Self::lire("{}"),
            Err(e) => Err(format!("{} : {e}", chemin.display())),
        }
    }

    /// Le JSON que le script d'installation pose : les défauts, écrits.
    pub fn json(&self) -> Value {
        json!({
            "manifest": self.manifest,
            "keys": self.keys,
            "context_tokens": self.context_tokens,
            "suggest_ratio": self.suggest_ratio,
            "prompt_block_chars": self.prompt_block_chars,
            "session_block_chars": self.session_block_chars,
            "prompt_timeout_ms": self.prompt_timeout_ms,
            "timeout_ms": self.timeout_ms,
            "journal": self.journal,
        })
    }

    pub fn delai(&self, evenement: &str) -> Duration {
        Duration::from_millis(if evenement == "UserPromptSubmit" { self.prompt_timeout_ms } else { self.timeout_ms })
    }
}

/// **Ce que la commande attend de l'hôte** : appeler un outil du backend
/// mémoire. Un trait pour que les témoins jouent sans réseau.
pub trait Hote {
    /// Le nom du backend ouvert (ce que `/open` a rendu).
    fn nom(&self) -> &str;
    fn appeler(&self, outil: &str, arguments: Value) -> Result<Value, String>;
}

/// **Le client de l'hôte, borné** : `POST /session`, `POST /open`, puis des
/// `POST /call`, et `DELETE /session` à la fin ; chaque requête sous le délai.
/// Il ne relance jamais l'hôte et ne relit pas le jeton : un hôte qui ne
/// répond pas est une raison de se taire, pas de s'acharner.
pub struct ClientHote {
    agent: ureq::Agent,
    base: String,
    token: Option<String>,
    session: String,
    manifest: PathBuf,
    nom: String,
}

/// Jamais le jeton : un secret ne doit pas pouvoir arriver dans un journal.
impl std::fmt::Debug for ClientHote {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientHote").field("base", &self.base).field("token", &self.token.as_ref().map(|_| "<redacted>")).field("nom", &self.nom).finish()
    }
}

impl ClientHote {
    /// L'adresse et le jeton de `daemon.json` ; l'hôte absent est dit.
    pub fn depuis_daemon_json(manifest: &Path, keys: &[String], delai: Duration) -> Result<Self, String> {
        let fiche = crate::daemon_lifecycle::read().ok_or("aucun hôte connu (daemon.json absent) : la commande ne le lance pas")?;
        Self::ouvrir(&fiche.address, Some(&fiche.token), manifest, keys, delai)
    }

    pub fn ouvrir(adresse: &str, token: Option<&str>, manifest: &Path, keys: &[String], delai: Duration) -> Result<Self, String> {
        let agent = crate::daemon::agent_with_timeout(delai);
        let base = crate::daemon::base_url(adresse);
        let mut me = Self { agent, base, token: token.map(str::to_string), session: String::new(), manifest: manifest.to_path_buf(), nom: String::new() };
        let corps = json!({ "client": "claude-code-hook", "keys": keys, "pid": std::process::id() });
        let v = me.poster("/session", corps, false)?;
        me.session = v["session"].as_str().ok_or("l'hôte n'a pas rendu de session")?.to_string();
        let ouvert = me.poster("/open", json!({ "manifest": me.manifest }), true)?;
        if ouvert["ok"] == json!(false) {
            return Err(format!("l'hôte n'ouvre pas le backend mémoire : {}", ouvert["error"].as_str().unwrap_or("?")));
        }
        me.nom = ouvert["describe"]["name"].as_str().unwrap_or("").to_string();
        Ok(me)
    }

    fn poster(&self, route: &str, corps: Value, avec_session: bool) -> Result<Value, String> {
        let mut requete = self.agent.post(format!("{}{route}", self.base)).header("content-type", "application/json");
        if let Some(t) = &self.token {
            requete = requete.header("Authorization", &format!("Bearer {t}"));
        }
        if avec_session {
            requete = requete.header(crate::daemon_host::SESSION_HEADER, &self.session);
        }
        let mut reponse = requete.send(corps.to_string()).map_err(|e| format!("{route} : l'hôte {} ne répond pas : {e}", self.base))?;
        let statut = reponse.status().as_u16();
        let texte = reponse.body_mut().read_to_string().map_err(|e| format!("{route} : {e}"))?;
        let v: Value = serde_json::from_str(&texte).unwrap_or_else(|_| json!({ "ok": false, "error": texte }));
        if statut != 200 {
            return Err(format!("{route} : refus {statut} — {}", v["error"].as_str().unwrap_or(&v.to_string())));
        }
        Ok(v)
    }

    /// Ferme la session ; une erreur ici n'intéresse personne.
    pub fn fermer(&self) {
        let mut requete = self.agent.delete(format!("{}/session", self.base)).header(crate::daemon_host::SESSION_HEADER, &self.session);
        if let Some(t) = &self.token {
            requete = requete.header("Authorization", &format!("Bearer {t}"));
        }
        let _ = requete.call();
    }
}

impl Hote for ClientHote {
    fn nom(&self) -> &str {
        &self.nom
    }
    fn appeler(&self, outil: &str, arguments: Value) -> Result<Value, String> {
        let v = self.poster("/call", json!({ "manifest": self.manifest, "name": outil, "arguments": arguments }), true)?;
        if v["ok"] == json!(false) {
            return Err(format!("{outil} : {}", v["error"].as_str().unwrap_or("?")));
        }
        Ok(v)
    }
}

/// Ce que la commande rend : le bloc (vide s'il n'y a rien) et les lignes du
/// journal.
#[derive(Debug, Default, PartialEq)]
pub struct Reponse {
    pub bloc: String,
    pub journal: Vec<String>,
}

/// **Répond à un événement**, l'hôte joint ou non. Lot 2 : les deux blocs de
/// lecture, `SessionStart` et `UserPromptSubmit` ; les autres événements
/// journalisent et ne rendent rien encore (lots 3 et 4). Le bloc est vide
/// quand il n'y a rien à dire, borné sinon.
pub fn repondre(e: &Evenement, reglage: &Reglage, hote: Result<&dyn Hote, String>) -> Reponse {
    let mut r = Reponse::default();
    let session = if e.session_id.is_empty() { "?" } else { e.session_id.as_str() };
    if !e.connu() {
        r.journal.push(format!("{} {session} : événement inconnu, ignoré", e.nom));
        return r;
    }
    let h = match hote {
        Ok(h) => h,
        Err(raison) => {
            r.journal.push(format!("{} {session} : silence — {raison}", e.nom));
            return r;
        }
    };
    let detail = match e.nom.as_str() {
        "SessionStart" => format!("source {}", e.source.as_deref().unwrap_or("?")),
        "PreCompact" | "PostCompact" => format!("trigger {}", e.trigger.as_deref().unwrap_or("?")),
        "UserPromptSubmit" => format!("prompt de {} caractères", e.prompt.as_deref().map(|p| p.chars().count()).unwrap_or(0)),
        _ => String::new(),
    };
    r.journal.push(format!("{} {session} : hôte joint, backend « {} »{}{}", e.nom, h.nom(), if detail.is_empty() { "" } else { ", " }, detail));
    let (bloc, borne) = match e.nom.as_str() {
        "SessionStart" => (bloc_de_session(e, reglage, h, &mut r.journal), reglage.session_block_chars),
        "UserPromptSubmit" => (bloc_de_prompt(e, reglage, h, &mut r.journal), reglage.prompt_block_chars),
        _ => (String::new(), 0),
    };
    if !bloc.trim().is_empty() {
        r.bloc = borner(&format!("[mémoire]\n{}", bloc.trim_end()), borne);
        r.journal.push(format!("{} {session} : bloc de {} caractères", e.nom, r.bloc.chars().count()));
    }
    r
}

/// Coupe à `max` caractères, sur une frontière de caractère, et le dit.
fn borner(texte: &str, max: usize) -> String {
    if max == 0 || texte.chars().count() <= max {
        return texte.to_string();
    }
    let garde = max.saturating_sub(12);
    let mut coupe: String = texte.chars().take(garde).collect();
    coupe.push_str("\n[… borné]");
    coupe
}

// ─── Ce qu'on lit dans la mémoire ────────────────────────────────────────────

/// Les lignes `data` d'un résultat de recherche (`recall_*`), ou vide.
fn lignes_de_recherche(v: &Value) -> Vec<Value> {
    v["result"]["result"]
        .as_array()
        .map(|a| a.iter().map(|x| x["data"].clone()).filter(|d| d.is_object()).collect())
        .unwrap_or_default()
}

/// Les fiches rendues par `cited_by` en json : le résultat est une **chaîne**
/// JSON (le rendu du Plan), relue ici.
fn fiches_citant(v: &Value) -> Vec<Value> {
    match &v["result"]["result"] {
        Value::String(s) => serde_json::from_str::<Value>(s).ok().and_then(|x| x.as_array().cloned()).unwrap_or_default(),
        Value::Array(a) => a.clone(),
        _ => Vec::new(),
    }
}

/// **L'identité d'une session** : la fiche qui cite `handle:session:<id>`
/// (page 04 du 25 octobre, §4). À défaut, l'indice facultatif
/// `RAG3WEAVER_AGENT_KEY`, qui ne vaut que si la fiche de cette clé existe :
/// une variable ne crée pas une identité.
#[derive(Debug, Clone, Default, PartialEq)]
struct Identite {
    cle: String,
    about: String,
    /// Le dernier apport `identity`, s'il y en a un.
    identity: Option<String>,
    /// La clé vient de la variable, pas d'un handle cité.
    par_indice: bool,
}

fn identite_de_la_session(h: &dyn Hote, session_id: &str, journal: &mut Vec<String>) -> Option<Identite> {
    if !session_id.is_empty() {
        match h.appeler("cited_by", json!({ "value": format!("handle:session:{session_id}"), "under": false, "format": "json", "limit": 3 })) {
            Ok(v) => {
                if let Some(f) = fiches_citant(&v).into_iter().find(|f| f["state"] != "retired") {
                    let cle = f["name"].as_str().unwrap_or("").to_string();
                    if !cle.is_empty() {
                        return Some(Identite { about: f["about"].as_str().unwrap_or("").to_string(), identity: dernier_apport(h, &cle, "identity"), cle, par_indice: false });
                    }
                }
            }
            Err(e) => journal.push(format!("cited_by handle:session : {e}")),
        }
    }
    let cle = std::env::var("RAG3WEAVER_AGENT_KEY").ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())?;
    match h.appeler("get_fiche", json!({ "record": { "name": cle } })) {
        Ok(v) if v["result"]["result"]["data"].is_object() => {
            let d = &v["result"]["result"]["data"];
            Some(Identite { about: d["about"].as_str().unwrap_or("").to_string(), identity: dernier_apport(h, &cle, "identity"), cle, par_indice: true })
        }
        Ok(_) => {
            journal.push(format!("RAG3WEAVER_AGENT_KEY={cle} : aucune fiche de cette clé, indice ignoré"));
            None
        }
        Err(e) => {
            journal.push(format!("get_fiche {cle} : {e}"));
            None
        }
    }
}

/// Le texte du dernier apport d'une section sur une fiche.
fn dernier_apport(h: &dyn Hote, fiche: &str, section: &str) -> Option<String> {
    let v = h
        .appeler(
            "recall_apports",
            json!({ "query": section, "options": { "limit": 5, "filter_condition": { "must": [
                { "field": { "key": "fiche", "value": fiche } },
                { "field": { "key": "section", "value": section } }
            ] } } }),
        )
        .ok()?;
    lignes_de_recherche(&v)
        .into_iter()
        .max_by_key(|d| d["created_at"].as_u64().unwrap_or(0))
        .and_then(|d| d["texte"].as_str().map(str::to_string))
}

/// La rubrique `clé:` d'un apport `identity` (six rubriques, page 04 §6).
fn rubrique<'a>(identity: &'a str, nom: &str) -> Option<&'a str> {
    identity.lines().map(str::trim).find_map(|l| l.strip_prefix(nom).and_then(|r| r.strip_prefix(':'))).map(str::trim).filter(|s| !s.is_empty())
}

/// Les noms `fiche:<nom>` cités dans un texte (la rubrique `doing`), dans l'ordre.
fn fiches_citees(texte: &str) -> Vec<String> {
    let mut v = Vec::new();
    for mot in texte.split(|c: char| c.is_whitespace() || c == ',' || c == ';' || c == '(' || c == ')') {
        if let Some(n) = mot.strip_prefix("fiche:") {
            let n = n.trim_matches(|c: char| c == '«' || c == '»' || c == '"' || c == '\'');
            if !n.is_empty() && !v.iter().any(|x| x == n) {
                v.push(n.to_string());
            }
        }
    }
    v
}

/// Le texte de la demande d'identité : la fiche d'abord, le L1 `handle` ensuite.
const SANS_FICHE: &str = "Aucune fiche d'identité ne cite cette session. Si tu as une clé (ton nom court, celui que les autres sessions emploient), écris ton L1 `handle` : `put_apport` {fiche: <clé>, source: \"sessions\", section: \"handle\", texte: \"session:<id>\", niveau: \"L1\", via: \"agent\"} puis `relier_apport` et `citer_source` genre `handle` valeur `session:<id>`. Si tu es nouvelle, crée d'abord ta fiche d'identité : `put_fiche` {name: <clé>, about: <une ligne>, state: \"current\"}. Rien n'est créé à ta place.";

// ─── SessionStart ────────────────────────────────────────────────────────────

fn bloc_de_session(e: &Evenement, _reglage: &Reglage, h: &dyn Hote, journal: &mut Vec<String>) -> String {
    let mut b = String::new();
    let source = e.source.as_deref().unwrap_or("startup");
    let identite = identite_de_la_session(h, &e.session_id, journal);
    if source == "compact" {
        b.push_str("Ta session vient d'être compactée. Au premier tour, avant toute autre chose, écris ton apport `identity` (who, runtime, can, doing, holding, mandates, peers) sur ta fiche d'identité par `put_apport` : c'est à toi de dire qui tu es, pas au harnais.\n\n");
    }
    let Some(id) = identite else {
        b.push_str(SANS_FICHE.replace("<id>", &e.session_id).as_str());
        b.push('\n');
        return b;
    };
    b.push_str(&format!("Tu es **{}**{} — {}\n", id.cle, if id.par_indice { " (clé donnée par RAG3WEAVER_AGENT_KEY ; ton L1 `handle` pour cette session n'est pas écrit)" } else { "" }, id.about));
    match &id.identity {
        Some(identity) => {
            for (nom, titre) in [("doing", "En cours"), ("holding", "Tu tiens"), ("mandates", "Mandats"), ("peers", "Pairs")] {
                if let Some(v) = rubrique(identity, nom) {
                    b.push_str(&format!("- {titre} : {v}\n"));
                }
            }
            // La tâche courante : la première fiche citée par `doing` ; son dernier plan.
            if let Some(tache) = rubrique(identity, "doing").map(fiches_citees).and_then(|v| v.into_iter().next()) {
                match dernier_apport(h, &tache, "plan") {
                    Some(plan) => b.push_str(&format!("- Plan en cours (fiche « {tache} ») : {}\n", premiere_ligne(&plan, 400))),
                    None => b.push_str(&format!("- Tâche courante : fiche « {tache} », sans plan écrit\n")),
                }
            }
        }
        None => b.push_str("- Aucun apport `identity` sur ta fiche : écris-en un (who, runtime, can, doing, holding, mandates, peers).\n"),
    }
    b
}

fn premiere_ligne(texte: &str, max: usize) -> String {
    let t = texte.trim();
    let une = t.lines().next().unwrap_or("").trim();
    let coupe: String = une.chars().take(max).collect();
    if coupe.len() < une.len() || t.lines().count() > 1 { format!("{coupe} …") } else { coupe }
}

// ─── UserPromptSubmit ────────────────────────────────────────────────────────

fn bloc_de_prompt(e: &Evenement, reglage: &Reglage, h: &dyn Hote, journal: &mut Vec<String>) -> String {
    let mut b = String::new();
    let transcription = e.transcript_path.as_deref().and_then(|p| crate::claude_code_transcript::Transcription::lire(p).map_err(|err| journal.push(format!("transcription : {err}"))).ok());
    let identite = identite_de_la_session(h, &e.session_id, journal);

    // Le premier tour après une compaction : le défi d'identité (Lucie, 25 octobre).
    if transcription.as_ref().is_some_and(|t| t.premier_tour_apres_compaction) {
        let deja = identite.as_ref().and_then(|i| i.identity.as_ref()).is_some_and(|txt| transcription.as_ref().and_then(|t| t.dernier_resume.as_ref()).is_some_and(|r| txt.len() > 20 && r.contains(&txt[..20.min(txt.len())])));
        if !deja {
            b.push_str("Ta session a été compactée. Avant toute autre chose, écris ton apport `identity` (who, runtime, can, doing, holding, mandates, peers) sur ta fiche d'identité par `put_apport`");
            match &identite {
                Some(id) => b.push_str(&format!(" (fiche « {} »)", id.cle)),
                None => b.push_str(" — et si aucune fiche ne cite cette session, ton L1 `handle` d'abord"),
            }
            b.push_str(".\n");
            if let Some(id) = identite.as_ref().and_then(|i| i.identity.as_ref()) {
                b.push_str(&format!("Ton dernier `identity` : {}\n", premiere_ligne(id, 300)));
            }
        }
    }

    // Les fiches que le prompt rappelle.
    if let Some(prompt) = e.prompt.as_deref().filter(|p| p.chars().count() >= 8) {
        match h.appeler("recall_fiches", json!({ "query": prompt, "options": { "limit": 3, "filter_condition": { "field": { "key": "state", "value": ["current", "to_review"] } } } })) {
            Ok(v) => {
                let lignes = lignes_de_recherche(&v);
                if !lignes.is_empty() {
                    b.push_str("Fiches en rapport : ");
                    b.push_str(&lignes.iter().filter_map(|d| d["name"].as_str().map(|n| format!("« {n} »{}", d["about"].as_str().map(|a| format!(" ({})", premiere_ligne(a, 80))).unwrap_or_default()))).collect::<Vec<_>>().join(" ; "));
                    b.push_str(" — `get_fiche` pour lire.\n");
                }
            }
            Err(err) => journal.push(format!("recall_fiches : {err}")),
        }
    }

    // Les fichiers touchés par la session, changés depuis par quelqu'un d'autre.
    if let (Some(t), Some(id)) = (&transcription, &identite) {
        let mut vus = 0;
        for f in t.fichiers_touches.iter().rev().take(5) {
            let Ok(v) = h.appeler("cited_by", json!({ "value": format!("path:{f}"), "under": true, "format": "json", "limit": 3, "since": depuis_48h() })) else { continue };
            for fiche in fiches_citant(&v) {
                for a in fiche["apports"].as_array().into_iter().flatten() {
                    if a["section"] == "diff" && a["author"].as_str().is_some_and(|x| x != id.cle) && vus < 2 {
                        b.push_str(&format!("`{f}` a changé depuis, par {} : {}\n", a["author"].as_str().unwrap_or("?"), premiere_ligne(a["texte"].as_str().unwrap_or(""), 160)));
                        vus += 1;
                    }
                }
            }
        }
        // Le dernier verdict du juge sur la tâche courante.
        if let Some(tache) = id.identity.as_deref().and_then(|i| rubrique(i, "doing")).map(fiches_citees).and_then(|v| v.into_iter().next()) {
            if let Some(bilan) = dernier_apport(h, &format!("bilan : {tache}"), "bilan") {
                b.push_str(&format!("Le juge, sur « {tache} » : {}\n", premiere_ligne(&bilan, 200)));
            }
        }
    }

    // Le remplissage du contexte, une suggestion une fois.
    if let Some(t) = &transcription {
        let (contexte, comment) = t.contexte_estime();
        let seuil = (reglage.context_tokens as f64 * reglage.suggest_ratio) as u64;
        if contexte >= seuil && !deja_suggere(reglage, &e.session_id) {
            b.push_str(&format!("Ton contexte est à ~{} k jetons sur {} k ({comment}) : range maintenant ce qui doit l'être — un apport, un plan clos — plutôt que d'attendre la compaction.\n", contexte / 1000, reglage.context_tokens / 1000));
            journal.push(format!("suggestion de ranger : {contexte} jetons ({comment})"));
        }
    }
    b
}

/// Millisecondes UTC d'il y a 48 heures.
fn depuis_48h() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0).saturating_sub(48 * 3600 * 1000)
}

/// La suggestion de ranger se fait **une fois** par session : une marque à
/// côté du journal.
fn deja_suggere(reglage: &Reglage, session_id: &str) -> bool {
    if session_id.is_empty() {
        return false;
    }
    let dossier = reglage.journal.parent().map(|d| d.join("hooks-suggestions")).unwrap_or_else(|| PathBuf::from("."));
    let marque = dossier.join(session_id);
    if marque.exists() {
        return true;
    }
    let _ = std::fs::create_dir_all(&dossier);
    let _ = std::fs::write(&marque, b"");
    false
}

/// Une ligne datée au journal ; un journal qu'on ne peut pas écrire ne fait
/// rien échouer.
pub fn journaliser(chemin: &Path, lignes: &[String]) {
    if lignes.is_empty() {
        return;
    }
    if let Some(d) = chemin.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    use std::io::Write;
    let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(chemin) else { return };
    let quand = jiff::Zoned::now().strftime("%Y-%m-%d %H:%M:%S").to_string();
    for l in lignes {
        let _ = writeln!(f, "{quand} {l}");
    }
}

/// **La commande entière** : lit l'entrée, le réglage, joint l'hôte, répond,
/// écrit le bloc et le journal. Rend toujours 0 ; `--config <chemin>` pour un
/// autre réglage, `--daemon <adresse>` pour un hôte donné (les témoins).
pub fn commande(args: &[String], entree: &str, sortie: &mut dyn std::io::Write) -> i32 {
    let mut config = Reglage::chemin_par_defaut();
    let mut adresse: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        match (args[i].as_str(), args.get(i + 1)) {
            ("--config", Some(v)) => config = PathBuf::from(v),
            ("--daemon", Some(v)) => adresse = Some(v.clone()),
            _ => {}
        }
        i += 2;
    }
    let reglage = match Reglage::charger(&config) {
        Ok(r) => r,
        Err(e) => {
            journaliser(&Reglage::defaut().journal, &[format!("réglage illisible, défauts pris : {e}")]);
            Reglage::defaut()
        }
    };
    let evenement = match Evenement::lire(entree) {
        Ok(e) => e,
        Err(e) => {
            journaliser(&reglage.journal, &[format!("entrée refusée : {e}")]);
            return 0;
        }
    };
    let delai = reglage.delai(&evenement.nom);
    let client = match &adresse {
        Some(a) => ClientHote::ouvrir(a, None, &reglage.manifest, &reglage.keys, delai),
        None => ClientHote::depuis_daemon_json(&reglage.manifest, &reglage.keys, delai),
    };
    let reponse = match &client {
        Ok(c) => repondre(&evenement, &reglage, Ok(c)),
        Err(e) => repondre(&evenement, &reglage, Err(e.clone())),
    };
    if let Ok(c) = &client {
        c.fermer();
    }
    if !reponse.bloc.is_empty() {
        let _ = sortie.write_all(reponse.bloc.as_bytes());
    }
    journaliser(&reglage.journal, &reponse.journal);
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    const SESSION_START: &str = r#"{"session_id":"25b7ff4b-dabf-42ef-8580-86661c846fb9","transcript_path":"/home/x/.claude/projects/p/25b7ff4b.jsonl","cwd":"/home/x/p","hook_event_name":"SessionStart","source":"compact"}"#;
    const PROMPT: &str = r#"{"session_id":"s1","transcript_path":"/t.jsonl","cwd":"/p","hook_event_name":"UserPromptSubmit","prompt":"range la fiche régime GPU","prompt_id":"p7"}"#;
    const PRE: &str = r#"{"session_id":"s1","transcript_path":"/t.jsonl","cwd":"/p","hook_event_name":"PreCompact","trigger":"auto","custom_instructions":null}"#;
    const POST: &str = r###"{"session_id":"s1","transcript_path":"/t.jsonl","cwd":"/p","hook_event_name":"PostCompact","trigger":"manual","compact_summary":"## Résumé\nla session rangeait."}"###;
    const STOP: &str = r#"{"session_id":"s1","transcript_path":"/t.jsonl","cwd":"/p","hook_event_name":"Stop","stop_hook_active":false,"last_assistant_message":"fait."}"#;

    #[test]
    fn les_cinq_evenements_se_lisent_avec_leurs_champs_propres() {
        let s = Evenement::lire(SESSION_START).unwrap();
        assert_eq!((s.nom.as_str(), s.source.as_deref()), ("SessionStart", Some("compact")));
        assert_eq!(s.transcript_path.as_deref(), Some(Path::new("/home/x/.claude/projects/p/25b7ff4b.jsonl")));
        let p = Evenement::lire(PROMPT).unwrap();
        assert_eq!((p.prompt.as_deref(), p.prompt_id.as_deref()), (Some("range la fiche régime GPU"), Some("p7")));
        let pre = Evenement::lire(PRE).unwrap();
        assert_eq!(pre.trigger.as_deref(), Some("auto"));
        let post = Evenement::lire(POST).unwrap();
        assert!(post.compact_summary.unwrap().starts_with("## Résumé"));
        let stop = Evenement::lire(STOP).unwrap();
        assert_eq!((stop.stop_hook_active, stop.last_assistant_message.as_deref()), (false, Some("fait.")));
        for e in [SESSION_START, PROMPT, PRE, POST, STOP] {
            assert!(Evenement::lire(e).unwrap().connu());
        }
    }

    #[test]
    fn une_entree_sans_nom_ou_pas_json_est_refusee_en_le_disant() {
        assert!(Evenement::lire("pas du json").unwrap_err().contains("JSON"));
        assert!(Evenement::lire(r#"{"session_id":"s"}"#).unwrap_err().contains("hook_event_name"));
        let inconnu = Evenement::lire(r#"{"hook_event_name":"Notification","session_id":"s"}"#).unwrap();
        assert!(!inconnu.connu());
    }

    #[test]
    fn le_reglage_prend_ses_defauts_et_refuse_un_champ_inconnu() {
        let d = Reglage::lire("{}").unwrap();
        assert_eq!(d, Reglage::defaut());
        let r = Reglage::lire(r#"{"context_tokens": 200000, "prompt_timeout_ms": 1000, "keys": ["admin"]}"#).unwrap();
        assert_eq!((r.context_tokens, r.prompt_timeout_ms, r.keys.as_slice()), (200_000, 1_000, &["admin".to_string()][..]));
        assert_eq!(r.delai("UserPromptSubmit"), Duration::from_millis(1_000));
        assert_eq!(r.delai("Stop"), Duration::from_millis(20_000));
        let e = Reglage::lire(r#"{"contexte": 1}"#).unwrap_err();
        assert!(e.contains("contexte") && e.contains("context_tokens"), "{e}");
        // Les défauts écrits se relisent à l'identique.
        assert_eq!(Reglage::lire(&Reglage::defaut().json().to_string()).unwrap(), Reglage::defaut());
        // Un fichier absent : les défauts, pas une erreur.
        assert_eq!(Reglage::charger(Path::new("/nulle/part/hooks.json")).unwrap(), Reglage::defaut());
    }

    struct FauxHote(Mutex<Vec<String>>);
    impl Hote for FauxHote {
        fn nom(&self) -> &str {
            "memoire-longue"
        }
        fn appeler(&self, outil: &str, _: Value) -> Result<Value, String> {
            self.0.lock().unwrap().push(outil.to_string());
            Ok(json!({ "ok": true }))
        }
    }

    #[test]
    fn un_evenement_sans_bloc_journalise_l_hote_joint_ou_le_silence() {
        let h = FauxHote(Mutex::new(Vec::new()));
        // Stop ne rend rien encore (lot 3) : l'hôte joint est dit, rien d'écrit.
        let r = repondre(&Evenement::lire(STOP).unwrap(), &Reglage::defaut(), Ok(&h));
        assert!(r.bloc.is_empty(), "rien d'injecté au Stop");
        assert_eq!(r.journal, vec!["Stop s1 : hôte joint, backend « memoire-longue »".to_string()]);
        assert!(h.0.lock().unwrap().is_empty(), "aucun outil appelé");
        let r = repondre(&Evenement::lire(PROMPT).unwrap(), &Reglage::defaut(), Err("aucun hôte connu (daemon.json absent) : la commande ne le lance pas".into()));
        assert!(r.bloc.is_empty());
        assert_eq!(r.journal[0], "UserPromptSubmit s1 : silence — aucun hôte connu (daemon.json absent) : la commande ne le lance pas");
        let r = repondre(&Evenement::lire(r#"{"hook_event_name":"Notification","session_id":"s"}"#).unwrap(), &Reglage::defaut(), Ok(&h));
        assert!(r.bloc.is_empty() && r.journal[0].contains("inconnu"));
    }

    /// Un hôte en carton sur un port libre : il rend les réponses données, dans
    /// l'ordre, et garde chaque requête (méthode, chemin, en-têtes, corps).
    fn hote_en_carton(reponses: Vec<(u16, String)>) -> (String, Arc<Mutex<Vec<(String, String, Vec<String>, String)>>>) {
        let ecoute = TcpListener::bind("127.0.0.1:0").unwrap();
        let adresse = ecoute.local_addr().unwrap().to_string();
        let recues = Arc::new(Mutex::new(Vec::new()));
        let gardees = recues.clone();
        std::thread::spawn(move || {
            for reponse in reponses {
                let Ok((mut s, _)) = ecoute.accept() else { return };
                let mut lecteur = BufReader::new(s.try_clone().unwrap());
                let mut ligne = String::new();
                lecteur.read_line(&mut ligne).unwrap();
                let mut mots = ligne.split_whitespace();
                let (methode, chemin) = (mots.next().unwrap_or("").to_string(), mots.next().unwrap_or("").to_string());
                let mut entetes = Vec::new();
                let mut longueur = 0usize;
                loop {
                    let mut l = String::new();
                    lecteur.read_line(&mut l).unwrap();
                    let l = l.trim_end().to_string();
                    if l.is_empty() {
                        break;
                    }
                    if let Some(n) = l.to_ascii_lowercase().strip_prefix("content-length:") {
                        longueur = n.trim().parse().unwrap_or(0);
                    }
                    entetes.push(l);
                }
                let mut corps = vec![0u8; longueur];
                if longueur > 0 {
                    lecteur.read_exact(&mut corps).unwrap();
                }
                gardees.lock().unwrap().push((methode, chemin, entetes, String::from_utf8_lossy(&corps).to_string()));
                let (statut, texte) = reponse;
                let _ = write!(s, "HTTP/1.1 {statut} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{texte}", texte.len());
            }
        });
        (adresse, recues)
    }

    #[test]
    fn le_client_ouvre_une_session_le_backend_appelle_et_ferme_sous_le_jeton() {
        let (adresse, recues) = hote_en_carton(vec![
            (200, json!({ "session": "S9" }).to_string()),
            (200, json!({ "ok": true, "describe": { "name": "memoire-longue" } }).to_string()),
            (200, json!({ "ok": true, "result": "x" }).to_string()),
            (200, json!({ "closed": true }).to_string()),
        ]);
        let c = ClientHote::ouvrir(&adresse, Some("jeton-j"), Path::new("/m/backend.json"), &["dev".into()], Duration::from_secs(5)).unwrap();
        assert_eq!(c.nom(), "memoire-longue");
        let r = c.appeler("get_fiche", json!({ "record": { "name": "c7" } })).unwrap();
        assert_eq!(r["result"], "x");
        c.fermer();
        std::thread::sleep(Duration::from_millis(50));
        let r = recues.lock().unwrap();
        let chemins: Vec<&str> = r.iter().map(|x| x.1.as_str()).collect();
        assert_eq!(chemins, ["/session", "/open", "/call", "/session"]);
        assert_eq!(r[3].0, "DELETE");
        for (i, (_, _, entetes, _)) in r.iter().enumerate() {
            assert!(entetes.iter().any(|h| h == "authorization: Bearer jeton-j" || h == "Authorization: Bearer jeton-j"), "jeton absent sur la requête {i} : {entetes:?}");
            if i > 0 {
                assert!(entetes.iter().any(|h| h.to_ascii_lowercase().starts_with("x-rag3weaver-session: s9")), "session absente sur la requête {i} : {entetes:?}");
            }
        }
        let appel: Value = serde_json::from_str(&r[2].3).unwrap();
        assert_eq!((appel["name"].as_str(), appel["manifest"].as_str()), (Some("get_fiche"), Some("/m/backend.json")));
        assert_eq!(r[0].3.contains("\"client\":\"claude-code-hook\""), true);
    }

    #[test]
    fn un_refus_de_l_hote_est_une_raison_dite_pas_une_panique() {
        let (adresse, _) = hote_en_carton(vec![(401, json!({ "ok": false, "error": "jeton refusé" }).to_string())]);
        let e = ClientHote::ouvrir(&adresse, Some("mauvais"), Path::new("/m"), &[], Duration::from_secs(5)).unwrap_err();
        assert!(e.contains("401") && e.contains("jeton refusé"), "{e}");
        let (adresse, _) = hote_en_carton(vec![(200, json!({ "session": "S" }).to_string()), (200, json!({ "ok": false, "error": "manifeste introuvable" }).to_string())]);
        let e = ClientHote::ouvrir(&adresse, None, Path::new("/m"), &[], Duration::from_secs(5)).unwrap_err();
        assert!(e.contains("manifeste introuvable"), "{e}");
    }

    #[test]
    fn un_hote_muet_rend_la_main_dans_le_delai() {
        // Un port qui accepte et ne répond jamais.
        let ecoute = TcpListener::bind("127.0.0.1:0").unwrap();
        let adresse = ecoute.local_addr().unwrap().to_string();
        std::thread::spawn(move || {
            let _garde = ecoute.accept();
            std::thread::sleep(Duration::from_secs(30));
        });
        let debut = std::time::Instant::now();
        let e = ClientHote::ouvrir(&adresse, None, Path::new("/m"), &[], Duration::from_millis(300)).unwrap_err();
        assert!(debut.elapsed() < Duration::from_secs(5), "le délai n'a pas coupé : {:?}", debut.elapsed());
        assert!(e.contains("ne répond pas"), "{e}");
    }

    #[test]
    fn la_commande_sort_en_zero_et_se_tait_sans_hote_et_journalise() {
        let dossier = std::env::temp_dir().join(format!("hooks-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dossier);
        std::fs::create_dir_all(&dossier).unwrap();
        let journal = dossier.join("hooks.log");
        let config = dossier.join("hooks.json");
        std::fs::write(&config, json!({ "journal": journal, "timeout_ms": 500 }).to_string()).unwrap();
        // Un port fermé : personne n'écoute.
        let ferme = TcpListener::bind("127.0.0.1:0").unwrap();
        let adresse = ferme.local_addr().unwrap().to_string();
        drop(ferme);
        let args = vec!["--config".to_string(), config.display().to_string(), "--daemon".to_string(), adresse];
        let mut sortie = Vec::new();
        assert_eq!(commande(&args, STOP, &mut sortie), 0);
        assert!(sortie.is_empty(), "rien sur la sortie sans hôte");
        let lu = std::fs::read_to_string(&journal).unwrap();
        assert!(lu.contains("Stop s1 : silence —") && lu.contains("ne répond pas"), "{lu}");
        // Une entrée illisible : zéro aussi, et dit au journal.
        assert_eq!(commande(&args, "{", &mut sortie), 0);
        assert!(std::fs::read_to_string(&journal).unwrap().contains("entrée refusée"));
        // Un hôte en carton qui répond : « hôte joint » au journal, sortie vide (ce lot).
        let (adresse, _) = hote_en_carton(vec![
            (200, json!({ "session": "S" }).to_string()),
            (200, json!({ "ok": true, "describe": { "name": "memoire-longue" } }).to_string()),
            (200, json!({ "closed": true }).to_string()),
        ]);
        let args = vec!["--config".to_string(), config.display().to_string(), "--daemon".to_string(), adresse];
        assert_eq!(commande(&args, STOP, &mut sortie), 0);
        assert!(sortie.is_empty(), "Stop ne rend rien encore");
        assert!(std::fs::read_to_string(&journal).unwrap().contains("Stop s1 : hôte joint, backend « memoire-longue »"));
        let _ = std::fs::remove_dir_all(&dossier);
    }

    // ─── Lot 2 : les blocs de lecture ─────────────────────────────────────

    /// Un hôte qui répond par outil, et garde les appels.
    struct HoteCanne {
        reponses: std::collections::HashMap<&'static str, Value>,
        appels: Mutex<Vec<(String, Value)>>,
    }
    impl HoteCanne {
        fn new(paires: Vec<(&'static str, Value)>) -> Self {
            Self { reponses: paires.into_iter().collect(), appels: Mutex::new(Vec::new()) }
        }
        fn noms(&self) -> Vec<String> {
            self.appels.lock().unwrap().iter().map(|(n, _)| n.clone()).collect()
        }
    }
    impl Hote for HoteCanne {
        fn nom(&self) -> &str {
            "memoire-longue"
        }
        fn appeler(&self, outil: &str, arguments: Value) -> Result<Value, String> {
            self.appels.lock().unwrap().push((outil.to_string(), arguments.clone()));
            // `recall_apports` répond selon la section demandée.
            if outil == "recall_apports" {
                let section = arguments["options"]["filter_condition"]["must"][1]["field"]["value"].as_str().unwrap_or("");
                let cle: &'static str = match section { "identity" => "recall_apports:identity", "plan" => "recall_apports:plan", "bilan" => "recall_apports:bilan", _ => "recall_apports" };
                return Ok(self.reponses.get(cle).cloned().unwrap_or_else(recherche_vide));
            }
            Ok(self.reponses.get(outil).cloned().unwrap_or_else(recherche_vide))
        }
    }
    fn recherche_vide() -> Value {
        json!({ "ok": true, "result": { "result": [] } })
    }
    fn recherche(lignes: Vec<Value>) -> Value {
        json!({ "ok": true, "result": { "result": lignes.into_iter().map(|d| json!({ "data": d })).collect::<Vec<_>>() } })
    }
    fn citant(fiches: Vec<Value>) -> Value {
        json!({ "ok": true, "result": { "result": Value::Array(fiches).to_string() } })
    }
    const IDENTITY: &str = "## identity\nwho: 96 — « mémoire », la session de la mémoire longue\nruntime: claude-code ; session:s1\ncan: les outils MCP memoire-longue\ndoing: la migration de la fiche de Lucie (fiche:migration-fiche) ; la page 04\nholding: worktrees wt-docs-96 ; branches lot-96-*\nmandates: chaque commit poussé dans la minute\npeers: c7 « orchestration » ; 73 « arbre principal »";

    fn hote_avec_identite() -> HoteCanne {
        HoteCanne::new(vec![
            ("cited_by", citant(vec![json!({ "name": "96", "about": "la session de la mémoire longue", "state": "current", "refs": [{ "genre": "handle", "valeur": "session:s1" }], "apports": [] })])),
            ("recall_apports:identity", recherche(vec![json!({ "fiche": "96", "section": "identity", "texte": IDENTITY, "created_at": 10 }), json!({ "fiche": "96", "section": "identity", "texte": "## identity\nwho: vieux", "created_at": 5 })])),
            ("recall_apports:plan", recherche(vec![json!({ "fiche": "migration-fiche", "section": "plan", "texte": "1. relire la fiche\n2. migrer les apports", "created_at": 7 })])),
        ])
    }

    #[test]
    fn session_start_rend_qui_je_suis_ce_que_je_tiens_et_le_plan_en_cours() {
        let h = hote_avec_identite();
        let r = repondre(&Evenement::lire(SESSION_START.replace("compact", "startup").replace("25b7ff4b-dabf-42ef-8580-86661c846fb9", "s1").as_str()).unwrap(), &Reglage::defaut(), Ok(&h));
        let b = &r.bloc;
        assert!(b.starts_with("[mémoire]\n"), "{b}");
        assert!(b.contains("Tu es **96** — la session de la mémoire longue"), "{b}");
        assert!(b.contains("- En cours : la migration de la fiche de Lucie"), "{b}");
        assert!(b.contains("- Tu tiens : worktrees wt-docs-96"), "{b}");
        assert!(b.contains("- Pairs : c7"), "{b}");
        assert!(b.contains("Plan en cours (fiche « migration-fiche ») : 1. relire la fiche …"), "{b}");
        assert!(!b.contains("compactée"), "pas de consigne de compaction au startup");
        let noms = h.noms();
        assert_eq!(noms[0], "cited_by");
        assert_eq!(h.appels.lock().unwrap()[0].1["value"], "handle:session:s1");
        assert!(noms.contains(&"recall_apports".to_string()));
        assert!(r.journal.iter().any(|l| l.contains("bloc de")), "{:?}", r.journal);
    }

    #[test]
    fn session_start_apres_compaction_demande_l_identity_a_l_agent() {
        let h = hote_avec_identite();
        let r = repondre(&Evenement::lire(SESSION_START.replace("25b7ff4b-dabf-42ef-8580-86661c846fb9", "s1").as_str()).unwrap(), &Reglage::defaut(), Ok(&h));
        assert!(r.bloc.contains("Ta session vient d'être compactée. Au premier tour"), "{}", r.bloc);
        assert!(r.bloc.contains("Tu es **96**"), "la fiche est rendue même après compaction");
    }

    #[test]
    fn sans_fiche_le_bloc_demande_le_handle_ou_la_fiche_et_ne_cree_rien() {
        let h = HoteCanne::new(vec![]);
        std::env::remove_var("RAG3WEAVER_AGENT_KEY");
        let r = repondre(&Evenement::lire(SESSION_START.replace("compact", "startup").as_str()).unwrap(), &Reglage::defaut(), Ok(&h));
        assert!(r.bloc.contains("Aucune fiche d'identité ne cite cette session"), "{}", r.bloc);
        assert!(r.bloc.contains("session:25b7ff4b-dabf-42ef-8580-86661c846fb9"), "l'id est donné à l'agent");
        assert!(r.bloc.contains("Rien n'est créé à ta place"));
        assert_eq!(h.noms(), vec!["cited_by"], "une lecture, aucune écriture");
    }

    #[test]
    fn une_cle_donnee_par_la_variable_ne_vaut_que_si_sa_fiche_existe() {
        let h = HoteCanne::new(vec![("get_fiche", json!({ "ok": true, "result": { "result": { "data": { "name": "archi", "about": "ex-orchestration", "state": "current" } } } }))]);
        std::env::set_var("RAG3WEAVER_AGENT_KEY", "archi");
        let r = repondre(&Evenement::lire(SESSION_START.replace("compact", "startup").as_str()).unwrap(), &Reglage::defaut(), Ok(&h));
        assert!(r.bloc.contains("Tu es **archi** (clé donnée par RAG3WEAVER_AGENT_KEY"), "{}", r.bloc);
        assert!(r.bloc.contains("Aucun apport `identity`"), "{}", r.bloc);
        let h2 = HoteCanne::new(vec![("get_fiche", json!({ "ok": true, "result": { "result": { "data": null } } }))]);
        let r = repondre(&Evenement::lire(SESSION_START.replace("compact", "startup").as_str()).unwrap(), &Reglage::defaut(), Ok(&h2));
        assert!(r.bloc.contains("Aucune fiche d'identité"), "{}", r.bloc);
        assert!(r.journal.iter().any(|l| l.contains("aucune fiche de cette clé, indice ignoré")), "{:?}", r.journal);
        std::env::remove_var("RAG3WEAVER_AGENT_KEY");
    }

    fn transcription_d_essai(nom: &str, lignes: &[&str]) -> (PathBuf, Reglage) {
        let dossier = std::env::temp_dir().join(format!("hooks-lot2-{}-{nom}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dossier);
        std::fs::create_dir_all(&dossier).unwrap();
        let chemin = dossier.join("t.jsonl");
        std::fs::write(&chemin, lignes.join("\n") + "\n").unwrap();
        let mut reglage = Reglage::defaut();
        reglage.journal = dossier.join("hooks.log");
        (chemin, reglage)
    }
    const T_USER: &str = r#"{"type":"user","promptId":"p1","message":{"role":"user","content":"salut"}}"#;
    const T_ASSISTANT_PETIT: &str = r#"{"type":"assistant","message":{"role":"assistant","usage":{"input_tokens":2,"cache_creation_input_tokens":100,"cache_read_input_tokens":30000,"output_tokens":5},"content":[{"type":"tool_use","name":"Edit","id":"t1","input":{"file_path":"/p/src/a.rs","old_string":"x","new_string":"y"}}]}}"#;
    const T_ASSISTANT_PLEIN: &str = r#"{"type":"assistant","message":{"role":"assistant","usage":{"input_tokens":2,"cache_creation_input_tokens":100,"cache_read_input_tokens":800000,"output_tokens":5},"content":[{"type":"text","text":"ok"}]}}"#;
    const T_BOUNDARY: &str = r#"{"type":"system","subtype":"compact_boundary","content":"Conversation compacted","compactMetadata":{"trigger":"auto"}}"#;
    const T_SUMMARY: &str = r#"{"type":"user","isCompactSummary":true,"message":{"role":"user","content":"This session is being continued… Summary: la session migrait la fiche."},"promptId":"p9"}"#;

    fn prompt(chemin: &Path, texte: &str) -> Evenement {
        Evenement::lire(&json!({ "session_id": "s1", "transcript_path": chemin, "cwd": "/p", "hook_event_name": "UserPromptSubmit", "prompt": texte, "prompt_id": "p2" }).to_string()).unwrap()
    }

    #[test]
    fn user_prompt_rappelle_les_fiches_les_diffs_des_fichiers_touches_et_le_verdict() {
        let (chemin, reglage) = transcription_d_essai("rappel", &[T_USER, T_ASSISTANT_PETIT]);
        let mut h = hote_avec_identite();
        h.reponses.insert("recall_fiches", recherche(vec![json!({ "name": "régime GPU", "about": "Comment on ménage les cartes du poste.", "state": "current" })]));
        h.reponses.insert("recall_apports:bilan", recherche(vec![json!({ "fiche": "bilan : migration-fiche", "section": "bilan", "texte": "l'outil put_apport a trompé l'agent sur `source`", "created_at": 9 })]));
        // `cited_by` répond pareil pour le handle et pour le fichier : la fiche 96 porte un apport diff d'un autre.
        h.reponses.insert("cited_by", citant(vec![json!({ "name": "96", "about": "la session de la mémoire longue", "state": "current", "refs": [], "apports": [
            { "section": "diff", "author": "73", "texte": "a.rs : le COPY passe par la lib", "created_at": 11 },
            { "section": "diff", "author": "96", "texte": "le mien, à ne pas rendre", "created_at": 12 }
        ] })]));
        let r = repondre(&prompt(&chemin, "range la fiche régime GPU"), &reglage, Ok(&h));
        let b = &r.bloc;
        assert!(b.contains("Fiches en rapport : « régime GPU » (Comment on ménage les cartes du poste.) — `get_fiche` pour lire."), "{b}");
        assert!(b.contains("`/p/src/a.rs` a changé depuis, par 73 : a.rs : le COPY passe par la lib"), "{b}");
        assert!(!b.contains("le mien"), "les diffs de la session elle-même ne sont pas rendus");
        assert!(b.contains("Le juge, sur « migration-fiche » : l'outil put_apport a trompé"), "{b}");
        assert!(!b.contains("compactée") && !b.contains("Ton contexte"), "{b}");
        let appels = h.appels.lock().unwrap();
        let path = appels.iter().find(|(n, a)| n == "cited_by" && a["value"] == "path:/p/src/a.rs").expect("cited_by path:");
        assert!(path.1["since"].as_u64().unwrap() > 0);
        let _ = std::fs::remove_dir_all(chemin.parent().unwrap());
    }

    #[test]
    fn user_prompt_court_sans_rien_a_dire_rend_un_bloc_vide() {
        let (chemin, reglage) = transcription_d_essai("vide", &[T_USER, T_ASSISTANT_PETIT]);
        let h = hote_avec_identite();
        let r = repondre(&prompt(&chemin, "oui"), &reglage, Ok(&h));
        assert!(r.bloc.is_empty(), "{}", r.bloc);
        assert!(!h.noms().contains(&"recall_fiches".to_string()), "un prompt de moins de 8 caractères ne cherche rien");
        let _ = std::fs::remove_dir_all(chemin.parent().unwrap());
    }

    #[test]
    fn le_premier_tour_apres_compaction_lance_le_defi_d_identite() {
        let (chemin, reglage) = transcription_d_essai("compact", &[T_USER, T_ASSISTANT_PETIT, T_BOUNDARY, T_SUMMARY]);
        let h = hote_avec_identite();
        let r = repondre(&prompt(&chemin, "on continue la migration"), &reglage, Ok(&h));
        assert!(r.bloc.contains("Ta session a été compactée. Avant toute autre chose, écris ton apport `identity`"), "{}", r.bloc);
        assert!(r.bloc.contains("(fiche « 96 »)") && r.bloc.contains("Ton dernier `identity` : ## identity …"), "{}", r.bloc);
        let (chemin2, reglage2) = transcription_d_essai("apres", &[T_USER, T_ASSISTANT_PETIT, T_BOUNDARY, T_SUMMARY, T_ASSISTANT_PETIT]);
        let r = repondre(&prompt(&chemin2, "on continue la migration"), &reglage2, Ok(&h));
        assert!(!r.bloc.contains("compactée"), "un tour est passé : plus de défi — {}", r.bloc);
        let _ = std::fs::remove_dir_all(chemin.parent().unwrap());
        let _ = std::fs::remove_dir_all(chemin2.parent().unwrap());
    }

    #[test]
    fn le_contexte_plein_suggere_de_ranger_une_seule_fois() {
        let (chemin, reglage) = transcription_d_essai("plein", &[T_USER, T_ASSISTANT_PLEIN]);
        let h = hote_avec_identite();
        let r = repondre(&prompt(&chemin, "encore une question"), &reglage, Ok(&h));
        assert!(r.bloc.contains("Ton contexte est à ~800 k jetons sur 1000 k (usage du dernier tour) : range maintenant"), "{}", r.bloc);
        assert!(r.journal.iter().any(|l| l.contains("suggestion de ranger")));
        let r2 = repondre(&prompt(&chemin, "encore une question"), &reglage, Ok(&h));
        assert!(!r2.bloc.contains("Ton contexte"), "une fois par session — {}", r2.bloc);
        let _ = std::fs::remove_dir_all(chemin.parent().unwrap());
    }

    #[test]
    fn le_bloc_est_borne_et_le_dit() {
        let long = "x".repeat(5000);
        let b = borner(&long, 100);
        assert_eq!(b.chars().count(), 100 - 12 + "\n[… borné]".chars().count());
        assert!(b.ends_with("[… borné]"));
        assert_eq!(borner("court", 100), "court");
        assert_eq!(borner("sans borne", 0), "sans borne");
    }

    #[test]
    fn les_rubriques_et_les_fiches_citees_se_lisent() {
        assert_eq!(rubrique(IDENTITY, "doing"), Some("la migration de la fiche de Lucie (fiche:migration-fiche) ; la page 04"));
        assert_eq!(rubrique(IDENTITY, "absent"), None);
        assert_eq!(fiches_citees("x fiche:a, fiche:b (fiche:a) fiche:«c»"), vec!["a", "b", "c"]);
        assert_eq!(premiere_ligne("une\ndeux", 10), "une …");
        assert_eq!(premiere_ligne("seule", 10), "seule");
    }
}
