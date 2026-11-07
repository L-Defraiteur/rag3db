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

/// **Répond à un événement**, l'hôte joint ou non. C'est ici que les lots
/// suivants écrivent les blocs ; ce lot ne rend rien et journalise ce qu'il a
/// vu — l'événement, la session, l'hôte joint ou la raison du silence.
pub fn repondre(e: &Evenement, _reglage: &Reglage, hote: Result<&dyn Hote, String>) -> Reponse {
    let mut r = Reponse::default();
    let session = if e.session_id.is_empty() { "?" } else { e.session_id.as_str() };
    if !e.connu() {
        r.journal.push(format!("{} {session} : événement inconnu, ignoré", e.nom));
        return r;
    }
    let detail = match e.nom.as_str() {
        "SessionStart" => format!("source {}", e.source.as_deref().unwrap_or("?")),
        "PreCompact" | "PostCompact" => format!("trigger {}", e.trigger.as_deref().unwrap_or("?")),
        "UserPromptSubmit" => format!("prompt de {} caractères", e.prompt.as_deref().map(|p| p.chars().count()).unwrap_or(0)),
        _ => String::new(),
    };
    match hote {
        Ok(h) => r.journal.push(format!("{} {session} : hôte joint, backend « {} »{}{}", e.nom, h.nom(), if detail.is_empty() { "" } else { ", " }, detail)),
        Err(raison) => r.journal.push(format!("{} {session} : silence — {raison}", e.nom)),
    }
    r
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
    fn ce_lot_ne_rend_rien_et_journalise_l_hote_joint_ou_le_silence() {
        let h = FauxHote(Mutex::new(Vec::new()));
        let r = repondre(&Evenement::lire(SESSION_START).unwrap(), &Reglage::defaut(), Ok(&h));
        assert!(r.bloc.is_empty(), "rien d'injecté dans ce lot");
        assert_eq!(r.journal, vec!["SessionStart 25b7ff4b-dabf-42ef-8580-86661c846fb9 : hôte joint, backend « memoire-longue », source compact".to_string()]);
        assert!(h.0.lock().unwrap().is_empty(), "aucun outil appelé dans ce lot");
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
        assert_eq!(commande(&args, SESSION_START, &mut sortie), 0);
        assert!(sortie.is_empty());
        assert!(std::fs::read_to_string(&journal).unwrap().contains("SessionStart 25b7ff4b-dabf-42ef-8580-86661c846fb9 : hôte joint, backend « memoire-longue », source compact"));
        let _ = std::fs::remove_dir_all(&dossier);
    }
}
