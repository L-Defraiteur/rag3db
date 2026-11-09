//! **Le pas d'un équipier, étanche au moteur** (page
//! `docs/07-novembre-2026-everything-declarative/01-l-equipier-agent-de-code.md`, § 1) :
//! « si défaillance du moteur d'ingestion, elle ne plante jamais et se comporte comme un
//! Claude Code normal » (Lucie, 7 novembre 2026).
//!
//! Le chat tient le backend ouvert pour tout son tour ; le pas d'un équipier le prend **à
//! chaque appel d'outil** ([`Etanche`]) : une fermeture passe entre deux appels, et un
//! backend qui ne s'ouvre pas, ou qui se ferme pendant le pas, ne tue ni le pas ni le
//! lancement. Les outils de fichiers (`read_file`, `list_files`, `grep_files`,
//! `edit_file`) et `search_code` se replient alors sur le worktree de la tâche
//! ([`FileTools`]) et le disent en tête de leur retour ; `outline`, `usages`, `impact`
//! et les autres rendent « indisponible » avec la cause. Un appel qui attend (un index
//! qui tient l'écriture) n'est jamais abandonné : au-delà de `teammate.tool_wait_notice_s`,
//! la boîte le dit, et l'appel continue.
use super::chat::{OutilsDuChat, Tour};
use super::teammate::{Step, TeammateFiles};
use super::{DaemonHost, Hosted};
use crate::agent::{ToolBox, ToolInvocation};
use crate::chat::ChatSession;
use crate::llm::{ToolCall, Turn};
use crate::tools::ToolDef;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Les outils qui ont un repli sur les fichiers ; les autres disent « indisponible ».
pub(super) const FILE_TOOLS: [&str; 5] = ["read_file", "list_files", "grep_files", "edit_file", "search_code"];

impl DaemonHost {
    /// **Le pas d'un équipier** : le backend est pris pour lire ce que le tour en
    /// annonce (ses outils, son bus, son mode) ; s'il ne s'ouvre pas, le pas part en
    /// mode fichiers, sur le manifeste préparé seul (son modèle, `models.llm`).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn run_teammate_turn(
        &self,
        hosted: &Arc<Hosted>,
        backend_id: &str,
        run_id: &str,
        session: &mut ChatSession,
        message: &str,
        annule: Arc<AtomicBool>,
        arrete_par: Arc<Mutex<Option<String>>>,
        step: &Step,
    ) -> Result<Value, String> {
        let ouvert = self.with_backend(hosted, |b| {
            let declares = crate::declare::DeclarationTools { backend: b, inner: &crate::backend_tools::BackendTools { backend: b } };
            let outils = OutilsDuChat::de(b, &declares, true);
            (b.prepared(), b.mode_declarations(), b.bus_de_publication(), outils.tool_defs(), outils.ecrivent.clone())
        });
        let racine = self.taches.lock().unwrap_or_else(|p| p.into_inner()).get(&hosted.manifest).and_then(|t| t.workspace.as_ref().map(|w| w.racine.clone()));
        let files = racine.map(|root| FileTools { root });
        let (prepared, mode, bus, defs, ecrivent, hors_ligne) = match ouvert {
            Ok((p, m, bus, defs, ecrivent)) => (p, m, Some(bus), defs, ecrivent, None),
            Err(cause) => {
                let p = crate::backend::PreparedBackend::load(&hosted.manifest)
                    .map_err(|e| format!("le backend ne s'ouvre pas ({cause}), et son manifeste ne se lit pas : {e}"))?;
                let defs = if files.is_some() { FileTools::defs() } else { Vec::new() };
                (Arc::new(p), Default::default(), None, defs, HashSet::from(["edit_file".to_string()]), Some(cause))
            }
        };
        let notice = prepared.manifest.teammate.clone().unwrap_or_default().tool_wait_notice_s;
        let etanche = Etanche { host: self, hosted: hosted.clone(), defs, files, hors_ligne, notice: Duration::from_secs(notice), boite: step.files.clone(), repli: Mutex::new(Vec::new()) };
        let tour = Tour { prepared, mode, dans_une_tache: true, bus, boite: &etanche, ecrivent: &ecrivent, repli: Some(&etanche.repli) };
        self.run_chat_turn(tour, backend_id, &hosted.manifest, run_id, session, message, annule, arrete_par, Some(step))
    }
}

/// **La boîte d'outils étanche** d'un pas : le backend pris à chaque appel, le repli
/// sur les fichiers quand il manque.
pub(super) struct Etanche<'a> {
    host: &'a DaemonHost,
    hosted: Arc<Hosted>,
    defs: Vec<ToolDef>,
    files: Option<FileTools>,
    /// La cause, quand le backend ne s'est pas ouvert au départ du pas.
    hors_ligne: Option<String>,
    notice: Duration,
    boite: TeammateFiles,
    /// Les replis de ce pas, pour le journal du run.
    pub repli: Mutex<Vec<String>>,
}

impl Etanche<'_> {
    /// L'appel par le backend, sous son verrou de lecture pour cet appel seul ; un
    /// appel qui dure au-delà du délai annoncé le dit dans la boîte, et continue.
    fn par_le_backend(&self, call: &ToolCall, f: impl FnOnce(&OutilsDuChat) -> Turn) -> Turn {
        if let Some(cause) = &self.hors_ligne {
            return self.repli_sur_fichiers(call, cause);
        }
        let fini = AtomicBool::new(false);
        let r = std::thread::scope(|sc| {
            sc.spawn(|| {
                let depart = Instant::now();
                while !fini.load(Ordering::SeqCst) {
                    if depart.elapsed() >= self.notice {
                        let _ = self.boite.append(
                            "host",
                            "report",
                            None,
                            &format!("« {} » attend depuis {} s (un index tient sans doute l'écriture) ; l'appel continue, rien n'est abandonné", call.name, self.notice.as_secs()),
                        );
                        return;
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
            });
            let r = self.host.with_backend(&self.hosted, |b| {
                let outils = crate::backend_tools::BackendTools { backend: b };
                let declares = crate::declare::DeclarationTools { backend: b, inner: &outils };
                let du_chat = OutilsDuChat::de(b, &declares, true);
                f(&du_chat)
            });
            fini.store(true, Ordering::SeqCst);
            r
        });
        match r {
            Ok(turn) => turn,
            Err(cause) => self.repli_sur_fichiers(call, &cause),
        }
    }

    /// **Le repli** : les outils de fichiers sur le worktree de la tâche, en le
    /// disant ; les autres, « indisponible » avec la cause. Jamais une panne du pas.
    fn repli_sur_fichiers(&self, call: &ToolCall, cause: &str) -> Turn {
        self.repli.lock().unwrap_or_else(|p| p.into_inner()).push(format!("{} : {cause}", call.name));
        let texte = match (&self.files, FILE_TOOLS.contains(&call.name.as_str())) {
            (Some(files), true) => {
                let args: Value = serde_json::from_str(&call.arguments).unwrap_or(Value::Null);
                match files.call(&call.name, &args) {
                    Ok(t) => format!("index indisponible : repli sur les fichiers ({cause})\n{t}"),
                    Err(e) => json!({ "error": e, "fallback": format!("index indisponible : repli sur les fichiers ({cause})") }).to_string(),
                }
            }
            _ => json!({ "ok": false, "unavailable": format!("« {} » demande l'index, indisponible : {cause}", call.name) }).to_string(),
        };
        Turn::tool_result(&call.id, &call.name, texte)
    }
}

impl ToolBox for Etanche<'_> {
    fn call(&self, call: &ToolCall) -> Turn {
        self.par_le_backend(call, |o| o.call(call))
    }
    fn call_in(&self, call: &ToolCall, run: &str) -> Turn {
        self.par_le_backend(call, |o| o.call_in(call, run))
    }
    fn call_with(&self, call: &ToolCall, run: &str, inv: &ToolInvocation) -> Turn {
        self.par_le_backend(call, |o| o.call_with(call, run, inv))
    }
    fn tool_defs(&self) -> Vec<ToolDef> {
        self.defs.clone()
    }
}

/// **Les outils de fichiers sans index**, sur le worktree de la tâche : les mêmes
/// noms et paramètres que les graphes du gabarit `code` (`read`, `list`, `grep`,
/// `edit`, `search_workspace`), un texte pour le modèle, les chemins confinés à la
/// racine.
pub(super) struct FileTools {
    pub root: PathBuf,
}

/// Les dossiers qu'un parcours ne visite pas.
const IGNORES: [&str; 4] = [".git", "target", "node_modules", ".rag3weaver"];

impl FileTools {
    pub(super) fn defs() -> Vec<ToolDef> {
        let def = |name: &str, description: &str, parameters: Value| ToolDef { name: name.into(), description: description.into(), parameters };
        vec![
            def(
                "read_file",
                "Lire un fichier du worktree par tranche de lignes numérotées (index indisponible : lecture directe des fichiers).",
                json!({ "type": "object", "properties": {
                    "path": { "type": "string", "description": "Chemin relatif au projet." },
                    "offset": { "type": "integer", "description": "Première ligne (1-based)." },
                    "limit": { "type": "integer", "description": "Lignes au plus (plafond 2000)." },
                    "format": { "type": "string", "enum": ["markdown", "brut"] } }, "required": ["path"] }),
            ),
            def(
                "list_files",
                "Lister les fichiers du worktree sous un préfixe (index indisponible : parcours direct).",
                json!({ "type": "object", "properties": {
                    "path_prefix": { "type": "string" }, "limit": { "type": "integer" } } }),
            ),
            def(
                "grep_files",
                "Chercher une expression régulière dans les fichiers du worktree (index indisponible : parcours direct).",
                json!({ "type": "object", "properties": {
                    "pattern": { "type": "string", "description": "Expression régulière (syntaxe Rust regex)." },
                    "path_prefix": { "type": "string" }, "extension": { "type": "string" },
                    "max_results": { "type": "integer" }, "context_lines": { "type": "integer" } }, "required": ["pattern"] }),
            ),
            def(
                "edit_file",
                "Modifier un fichier du worktree : remplacer un passage unique (old → new), ou écrire le fichier entier (content). L'index se mettra à jour au retour du backend.",
                json!({ "type": "object", "properties": {
                    "path": { "type": "string" }, "old": { "type": "string" }, "new": { "type": "string" },
                    "content": { "type": "string" }, "expected_hash": { "type": "string" } }, "required": ["path"] }),
            ),
            def(
                "search_code",
                "Chercher des mots dans les fichiers du worktree (index indisponible : un grep des mots de la requête, classé par mots trouvés).",
                json!({ "type": "object", "properties": {
                    "query": { "type": "string" }, "scope": { "type": "string" } }, "required": ["query"] }),
            ),
        ]
    }

    /// Le chemin d'un argument, sous la racine : ni absolu, ni `..`.
    fn chemin(&self, rel: &str) -> Result<PathBuf, String> {
        let p = Path::new(rel.trim_start_matches("./"));
        if p.is_absolute() || p.components().any(|c| !matches!(c, Component::Normal(_) | Component::CurDir)) {
            return Err(format!("« {rel} » sort du worktree de la tâche : un chemin relatif, sans ..")); 
        }
        Ok(self.root.join(p))
    }

    /// Les fichiers sous un préfixe, triés, chemins relatifs.
    fn fichiers(&self, prefixe: &str) -> Vec<String> {
        let mut v = Vec::new();
        let mut pile = vec![self.root.clone()];
        while let Some(d) = pile.pop() {
            let Ok(lecture) = std::fs::read_dir(&d) else { continue };
            for e in lecture.flatten() {
                let p = e.path();
                let nom = e.file_name().to_string_lossy().to_string();
                if p.is_dir() {
                    if !IGNORES.contains(&nom.as_str()) {
                        pile.push(p);
                    }
                } else if let Ok(rel) = p.strip_prefix(&self.root) {
                    let rel = rel.to_string_lossy().replace('\\', "/");
                    if rel.starts_with(prefixe) {
                        v.push(rel);
                    }
                }
            }
        }
        v.sort();
        v
    }

    pub(super) fn call(&self, name: &str, a: &Value) -> Result<String, String> {
        let s = |k: &str| a[k].as_str().unwrap_or_default().to_string();
        let n = |k: &str, defaut: u64, plafond: u64| a[k].as_u64().unwrap_or(defaut).min(plafond) as usize;
        match name {
            "read_file" => {
                let path = s("path");
                let texte = std::fs::read_to_string(self.chemin(&path)?).map_err(|e| format!("{path} : {e}"))?;
                let empreinte = crate::hash::content_hash(&texte);
                if s("format") == "brut" {
                    return Ok(json!({ "path": path, "content": texte, "content_hash": empreinte }).to_string());
                }
                let lignes: Vec<&str> = texte.lines().collect();
                let debut = n("offset", 1, u64::MAX).max(1);
                let fin = (debut - 1 + n("limit", 200, 2000)).min(lignes.len());
                let mut sortie = format!("# {path} — lignes {debut}–{fin} sur {} (content_hash {empreinte})\n", lignes.len());
                for (i, l) in lignes.iter().enumerate().take(fin).skip(debut - 1) {
                    sortie.push_str(&format!("{:>5}│ {l}\n", i + 1));
                }
                if fin < lignes.len() {
                    sortie.push_str(&format!("… la suite : offset {}\n", fin + 1));
                }
                Ok(sortie)
            }
            "list_files" => {
                let tous = self.fichiers(&s("path_prefix"));
                let limite = n("limit", 200, 2000);
                let mut sortie = format!("{} fichier(s)\n", tous.len());
                for f in tous.iter().take(limite) {
                    let lignes = std::fs::read_to_string(self.root.join(f)).map(|t| t.lines().count()).unwrap_or(0);
                    sortie.push_str(&format!("- {f} ({lignes} lignes)\n"));
                }
                Ok(sortie)
            }
            "grep_files" => self.grep(&s("pattern"), &s("path_prefix"), &s("extension"), n("max_results", 50, 500), n("context_lines", 0, 5)),
            "edit_file" => self.edit(&s("path"), &s("old"), &s("new"), a.get("content").and_then(Value::as_str), &s("expected_hash")),
            "search_code" => self.search(&s("query"), &s("scope")),
            autre => Err(format!("« {autre} » n'a pas de repli sur les fichiers")),
        }
    }

    #[cfg(feature = "code")]
    fn grep(&self, motif: &str, prefixe: &str, extension: &str, max: usize, contexte: usize) -> Result<String, String> {
        let re = regex::Regex::new(motif).map_err(|e| format!("expression illisible : {e}"))?;
        let mut trouves = Vec::new();
        let mut total = 0usize;
        for f in self.fichiers(prefixe) {
            if !extension.is_empty() && !f.ends_with(&format!(".{extension}")) {
                continue;
            }
            let Ok(texte) = std::fs::read_to_string(self.root.join(&f)) else { continue };
            let lignes: Vec<&str> = texte.lines().collect();
            for (i, l) in lignes.iter().enumerate() {
                if re.is_match(l) {
                    total += 1;
                    if trouves.len() < max {
                        let (de, a) = (i.saturating_sub(contexte), (i + contexte + 1).min(lignes.len()));
                        let bloc: Vec<String> = (de..a).map(|j| format!("{f}:{}: {}", j + 1, lignes[j])).collect();
                        trouves.push(bloc.join("\n"));
                    }
                }
            }
        }
        Ok(format!("{total} ligne(s) trouvée(s), {} rendue(s)\n{}", trouves.len(), trouves.join("\n")))
    }

    #[cfg(not(feature = "code"))]
    fn grep(&self, _: &str, _: &str, _: &str, _: usize, _: usize) -> Result<String, String> {
        Err("cet hôte est bâti sans la feature `code` : pas de grep de repli".into())
    }

    fn edit(&self, path: &str, old: &str, new: &str, content: Option<&str>, attendue: &str) -> Result<String, String> {
        let p = self.chemin(path)?;
        let avant = std::fs::read_to_string(&p).ok();
        if !attendue.is_empty() {
            let vue = avant.as_deref().map(crate::hash::content_hash).unwrap_or_default();
            if vue != attendue {
                return Err(format!("fichier changé depuis la lecture : {path} (empreinte {vue}, attendue {attendue}) ; relisez-le avant d'éditer"));
            }
        }
        let apres = match content {
            Some(c) if !c.is_empty() || old.is_empty() => {
                if !old.is_empty() {
                    return Err("content et old/new s'excluent".into());
                }
                c.to_string()
            }
            _ => {
                let texte = avant.ok_or_else(|| format!("{path} n'existe pas : écrivez-le entier par content"))?;
                match texte.matches(old).count() {
                    1 => texte.replacen(old, new, 1),
                    0 => return Err(format!("le passage à remplacer n'est pas dans {path} ; relisez-le")),
                    k => return Err(format!("le passage à remplacer est {k} fois dans {path} : élargissez-le pour qu'il soit unique")),
                }
            }
        };
        if let Some(d) = p.parent() {
            std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
        }
        std::fs::write(&p, &apres).map_err(|e| format!("{path} : {e}"))?;
        Ok(format!("édité : {path} (content_hash {}) ; l'index se mettra à jour quand le backend reviendra", crate::hash::content_hash(&apres)))
    }

    /// Un grep des mots de la requête (trois lettres au moins, sans la casse), les
    /// fichiers classés par le nombre de mots distincts trouvés : le signal
    /// `grep_repli`, dit en tête.
    fn search(&self, requete: &str, scope: &str) -> Result<String, String> {
        let mots: Vec<String> = requete.split(|c: char| !c.is_alphanumeric() && c != '_').filter(|m| m.chars().count() >= 3).map(str::to_lowercase).collect();
        if mots.is_empty() {
            return Err("la requête n'a aucun mot de trois lettres ou plus".into());
        }
        let prefixes: Vec<&str> = scope.split('|').map(str::trim).filter(|s| !s.is_empty()).collect();
        let mut classes: Vec<(usize, String, Vec<String>)> = Vec::new();
        for f in self.fichiers("") {
            if !prefixes.is_empty() && !prefixes.iter().any(|p| f.starts_with(p)) {
                continue;
            }
            let Ok(texte) = std::fs::read_to_string(self.root.join(&f)) else { continue };
            let bas = texte.to_lowercase();
            let vus = mots.iter().filter(|m| bas.contains(m.as_str())).count();
            if vus == 0 {
                continue;
            }
            let lignes: Vec<String> = texte
                .lines()
                .enumerate()
                .filter(|(_, l)| {
                    let l = l.to_lowercase();
                    mots.iter().any(|m| l.contains(m.as_str()))
                })
                .take(3)
                .map(|(i, l)| format!("  {}: {}", i + 1, l.trim()))
                .collect();
            classes.push((vus, f, lignes));
        }
        classes.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        let mut sortie = format!("signal : grep_repli — {} fichier(s), les mots : {}\n", classes.len(), mots.join(", "));
        for (vus, f, lignes) in classes.iter().take(20) {
            sortie.push_str(&format!("- {f} ({vus}/{} mots)\n{}\n", mots.len(), lignes.join("\n")));
        }
        Ok(sortie)
    }
}
