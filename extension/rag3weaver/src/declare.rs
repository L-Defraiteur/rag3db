//! `declare` : l'agent écrit les déclarations d'un backend — un **geste
//! d'hôte**, jamais un nœud de graphe (un graphe déclaré qui réécrirait ses
//! propres déclarations rendrait la frontière rechargeable de l'intérieur).
//!
//! Trois outils, donnés à l'agent par l'hôte à côté de ceux du manifeste
//! ([`DeclarationTools`]) : lister les déclarations, en lire une, en poser
//! une. Poser, c'est :
//!
//! 1. un chemin **relatif** au dossier du backend, d'une sorte connue
//!    (manifeste, graphe, schéma, nœud scripté et son script, vue) ; jamais
//!    hors du dossier, jamais la base, jamais `.git`, jamais du code compilé ;
//! 2. écrire, puis **revérifier comme au chargement** (`check_reload`) ; un
//!    refus remet l'ancien contenu et revient à l'agent avec sa raison (et sa
//!    ligne quand le chargement la connaît) ;
//! 3. accepté : mettre en service — les réactions de l'ancienne version
//!    arrêtées, celles de la nouvelle remontées
//!    (`apply_reload_with_reactions`) —, puis **un commit** dans le
//!    dépôt git qui englobe le dossier — par chemin, rien d'autre, sous
//!    l'identité git de la personne, avec la raison de l'agent pour message.
//!    C'est la première fiche de contexte : chaque changement dit pourquoi.
use crate::backend::Backend;
use crate::llm::{ToolCall, Turn};
use crate::tools::ToolDef;
use serde_json::{json, Value};
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

/// Une déclaration à la fois, dans tout le processus.
static DECLARING: Mutex<()> = Mutex::new(());

/// Les extensions d'une déclaration. `tsx` et `jsx` : la vue d'un composant (`components/`).
const KINDS: [&str; 13] = [
    "json", "mmd", "ts", "js", "tsx", "jsx", "rhai", "html", "htm", "jinja", "txt", "xml", "md",
];

/// Ce qu'une déclaration acceptée a mis en service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declared {
    /// Le chemin déclaré ; pour un ensemble, les chemins séparés par des virgules.
    pub path: String,
    /// Les chemins déclarés, un par fichier.
    pub paths: Vec<String>,
    /// La version des déclarations en service après elle.
    pub version: u64,
    /// Le commit qui la porte ; `None` si le contenu n'a pas changé.
    pub commit: Option<String>,
    /// Les réactions que cette version retire.
    pub removed_reactions: Vec<String>,
    /// Les réactions remontées sur la nouvelle version.
    pub mounted_reactions: Vec<String>,
    /// Les curseurs oubliés des réactions retirées (`<sujet>@<réaction>`) :
    /// leurs événements en attente sont perdus.
    pub dropped_cursors: Vec<String>,
    /// Les erreurs des réactions arrêtées — personne d'autre ne les relira.
    pub reaction_errors: Vec<String>,
}

impl Backend {
    /// Les déclarations du backend : les chemins relatifs, triés.
    pub fn list_declarations(&self) -> Vec<String> {
        let dir = self.prepared().directory.clone();
        let mut found = Vec::new();
        walk(&dir, &dir, &self.forbidden_dirs(), &mut found);
        found.sort();
        found
    }

    /// Le contenu d'une déclaration.
    pub fn read_declaration(&self, path: &str) -> Result<String, String> {
        let target = self.declaration_path(path)?;
        std::fs::read_to_string(&target).map_err(|e| format!("{path} : {e}"))
    }

    /// **Pose une déclaration** : vérifiée comme au chargement, mise en
    /// service, commitée avec sa raison ; refusée sans rien laisser derrière.
    pub fn declare(&self, path: &str, content: &str, reason: &str) -> Result<Declared, String> {
        self.declare_set(&[(path.to_string(), content.to_string())], reason)
    }

    /// **Pose plusieurs déclarations ensemble** : toutes écrites, vérifiées d'un seul
    /// rechargement, mises en service et commitées en un commit avec leur raison — ou aucune.
    /// Un composant neuf en a plusieurs (sa déclaration, sa vue, son graphe de clic) : posés un à
    /// un, l'ensemble à moitié écrit serait refusé.
    /// **Valider un ensemble de déclarations sans rien écrire** (une
    /// proposition, page 08) : les mêmes chemins que [`Self::declare_set`],
    /// puis le chargement entier sur un recouvrement — le dossier du backend
    /// sous les fichiers proposés, en mémoire ([`crate::lecteur::Recouvrement`])
    /// — et la règle de `check_reload` : un changement de base, de modèles ou
    /// d'entités est une migration, pas une déclaration. Rend le refus tel que
    /// le chargement le dit (chemin et ligne quand il les sait). L'arbre n'est
    /// pas touché.
    pub fn valider_sur(&self, files: &[(String, String)]) -> Result<(), String> {
        if files.is_empty() {
            return Err("une proposition nomme au moins un fichier".into());
        }
        let mut vus = std::collections::BTreeSet::new();
        if let Some((p, _)) = files.iter().find(|(p, _)| !vus.insert(p.as_str())) {
            return Err(format!("« {p} » est proposé deux fois dans le même ensemble"));
        }
        let mut absolus = Vec::new();
        for (path, content) in files {
            let target = self.declaration_path(path)?;
            absolus.push((target.to_string_lossy().to_string(), content.clone()));
        }
        let current = self.prepared();
        let recouvrement = crate::lecteur::Recouvrement::sur(&current.directory, &absolus);
        let next = crate::backend::PreparedBackend::load_avec(current.manifest_path(), &recouvrement)?;
        let changed = crate::backend::opening_changes(&current, &next);
        if !changed.is_empty() {
            return Err(format!(
                "{} change — fixé à l'ouverture de la base : c'est une migration, pas une déclaration",
                changed.join(", ")
            ));
        }
        Ok(())
    }

    pub fn declare_set(&self, files: &[(String, String)], reason: &str) -> Result<Declared, String> {
        let _one = DECLARING.lock().unwrap_or_else(|e| e.into_inner());
        if reason.trim().is_empty() {
            return Err("une déclaration dit pourquoi : la raison est vide".into());
        }
        if files.is_empty() {
            return Err("une déclaration nomme au moins un fichier".into());
        }
        let mut vus = std::collections::BTreeSet::new();
        if let Some((p, _)) = files.iter().find(|(p, _)| !vus.insert(p.as_str())) {
            return Err(format!("« {p} » est déclaré deux fois dans le même ensemble"));
        }
        git_present()?;
        let paths: Vec<String> = files.iter().map(|(p, _)| p.clone()).collect();
        let nom = paths.join(", ");
        // (chemin, cible, contenu d'avant) de ce qui change.
        let mut changes: Vec<(String, PathBuf, Option<String>, &str)> = Vec::new();
        for (path, content) in files {
            let target = self.declaration_path(path)?;
            let before = std::fs::read_to_string(&target).ok();
            if before.as_deref() != Some(content.as_str()) {
                changes.push((path.clone(), target, before, content.as_str()));
            }
        }
        if changes.is_empty() {
            return Ok(Declared {
                path: nom,
                paths,
                version: self.declarations_version(),
                commit: None,
                removed_reactions: vec![],
                mounted_reactions: vec![],
                dropped_cursors: vec![],
                reaction_errors: vec![],
            });
        }
        // Le retour à l'état d'avant, de tout ce qui a été écrit.
        let undo = |n: usize| -> Result<(), String> {
            for (_, target, before, _) in &changes[..n] {
                match before {
                    Some(old) => write_atomically(target, old)?,
                    None => std::fs::remove_file(target).map_err(|e| e.to_string())?,
                }
            }
            Ok(())
        };
        for (i, (path, target, _, content)) in changes.iter().enumerate() {
            if let Err(e) = write_atomically(target, content) {
                undo(i)?;
                return Err(format!("{path} : ne s'écrit pas ({e}), rien n'est écrit"));
            }
        }
        let tout = changes.len();
        let pending = match self.check_reload() {
            Ok(pending) => pending,
            Err(refused) => {
                undo(tout)?;
                return Err(format!("{nom} : déclaration refusée, rien n'est écrit — {refused}"));
            }
        };
        let dir = self.prepared().directory.clone();
        let back = |why: String| -> Result<Declared, String> {
            // Pas de déclaration à moitié : on revient en arrière, réactions
            // comprises.
            undo(tout)?;
            self.reload_reactions()
                .map_err(|e| format!("{nom} : {why}, et le retour arrière aussi : {e}"))?;
            Err(format!("{nom} : {why}, rien n'est écrit"))
        };
        let reloaded = match self.apply_reload_with_reactions(pending) {
            Ok(reloaded) => reloaded,
            Err(error) => return back(format!("les réactions ne se remontent pas ({error})")),
        };
        let changes_paths: Vec<&str> = changes.iter().map(|(p, ..)| p.as_str()).collect();
        match commit(&dir, &changes_paths, reason) {
            Ok(hash) => Ok(Declared {
                path: nom,
                paths,
                version: reloaded.reloaded.version,
                commit: Some(hash),
                removed_reactions: reloaded.reloaded.removed_reactions,
                mounted_reactions: reloaded.mounted,
                dropped_cursors: reloaded.dropped_cursors,
                reaction_errors: reloaded.errors_before,
            }),
            // Pas de déclaration sans sa raison : on revient en arrière.
            Err(git) => back(format!("git a refusé le commit ({git})")),
        }
    }

    /// Le chemin absolu d'une déclaration, ou pourquoi ce n'en est pas une.
    pub(crate) fn declaration_path(&self, path: &str) -> Result<PathBuf, String> {
        let relative = Path::new(path);
        let refuse = |why: &str| {
            Err(format!(
                "« {path} » n'est pas une déclaration de ce backend : {why}"
            ))
        };
        if path.is_empty()
            || relative.is_absolute()
            || relative
                .components()
                .any(|c| !matches!(c, Component::Normal(_)))
        {
            return refuse("un chemin relatif au dossier du backend, sans « .. »");
        }
        if relative
            .components()
            .any(|c| c.as_os_str().to_string_lossy().starts_with('.'))
        {
            return refuse("un fichier ou dossier caché (.git compris) ne se déclare pas");
        }
        let kind = relative.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !KINDS.contains(&kind) {
            return refuse(&format!(
                "les sortes connues sont {} — jamais du code compilé",
                KINDS.join(", ")
            ));
        }
        let dir = self.prepared().directory.clone();
        let target = dir.join(relative);
        if self.forbidden_dirs().iter().any(|f| target.starts_with(f)) {
            return refuse("le dossier de la base ne se déclare pas");
        }
        Ok(target)
    }

    /// Le dossier de la base, s'il est dans celui du backend.
    fn forbidden_dirs(&self) -> Vec<PathBuf> {
        let prepared = self.prepared();
        let database = prepared.path(&prepared.manifest.database);
        database
            .parent()
            .filter(|p| p.starts_with(&prepared.directory) && *p != prepared.directory)
            .map(|p| vec![p.to_path_buf()])
            .unwrap_or_else(|| vec![database])
    }
}

fn walk(root: &Path, dir: &Path, forbidden: &[PathBuf], found: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let hidden = path
            .file_name()
            .is_some_and(|n| n.to_string_lossy().starts_with('.'));
        if hidden || forbidden.iter().any(|f| path.starts_with(f)) {
            continue;
        }
        if path.is_dir() {
            walk(root, &path, forbidden, found);
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| KINDS.contains(&e))
        {
            if let Ok(relative) = path.strip_prefix(root) {
                found.push(
                    relative
                        .components()
                        .map(|c| c.as_os_str().to_string_lossy())
                        .collect::<Vec<_>>()
                        .join("/"),
                );
            }
        }
    }
}

fn write_atomically(target: &Path, content: &str) -> Result<(), String> {
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let temporary = target.with_extension("declare-tmp");
    std::fs::write(&temporary, content).map_err(|e| e.to_string())?;
    std::fs::rename(&temporary, target).map_err(|e| e.to_string())
}

fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .map_err(|e| format!("git : {e}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

fn git_present() -> Result<(), String> {
    Command::new("git")
        .arg("--version")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|_| ())
        .ok_or_else(|| "declare demande git sur ce poste (chaque déclaration est un commit)".into())
}

/// Commite **ces chemins seuls**, en un commit, dans le dépôt qui englobe le dossier — créé
/// s'il n'y en a aucun au-dessus —, sous l'identité git du dépôt.
fn commit(dir: &Path, paths: &[&str], reason: &str) -> Result<String, String> {
    if git(dir, &["rev-parse", "--show-toplevel"]).is_err() {
        git(dir, &["init", "-q"])?;
    }
    let mut add = vec!["add", "--"];
    add.extend_from_slice(paths);
    git(dir, &add)?;
    let pied = format!("declare: {}", paths.join(", "));
    let mut c = vec!["commit", "-q", "-m", reason.trim(), "-m", pied.as_str(), "--"];
    c.extend_from_slice(paths);
    git(dir, &c)?;
    git(dir, &["rev-parse", "HEAD"])
}

// ─── Les trois outils, donnés par l'hôte ────────────────────────────────────

/// Les outils d'un agent sur un backend, plus les trois de la déclaration.
pub struct DeclarationTools<'a> {
    pub backend: &'a Backend,
    pub inner: &'a (dyn crate::agent::ToolBox + Sync),
}

pub const LIST: &str = "list_declarations";
pub const READ: &str = "read_declaration";
pub const DECLARE: &str = "declare";
pub const DECLARE_SET: &str = "declare_set";
pub const PROPOSE: &str = "propose";
pub const PROPOSALS: &str = "proposals";
pub const APPLY: &str = "apply";
pub const REJECT: &str = "reject";

/// Les outils de déclaration que voit l'agent, selon le mode du backend
/// (page 08) : en `approbation` (le défaut), il lit et **propose** — une
/// personne applique dans l'IDE ; en `auto`, il déclare directement, et
/// applique ou rejette les propositions.
pub fn tool_defs_pour(mode: crate::propositions::ModeDeclarations) -> Vec<ToolDef> {
    let fichiers = json!({
        "type": "array",
        "minItems": 1,
        "items": {
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Relative to the backend folder."},
                "content": {"type": "string", "description": "The whole new content of the file."}
            },
            "required": ["path", "content"],
            "additionalProperties": false
        }
    });
    let mut defs: Vec<ToolDef> = tool_defs()
        .into_iter()
        .filter(|d| mode == crate::propositions::ModeDeclarations::Auto || ![DECLARE, DECLARE_SET].contains(&d.name.as_str()))
        .collect();
    defs.push(ToolDef {
        name: PROPOSE.into(),
        description: "Proposes several declaration files together (each whole). They are checked as at \
                      load time WITHOUT being written; the proposal is kept with its verdict (accepted, \
                      or the refusal with its file and line) and shown in the IDE, where a person applies \
                      or rejects it. Propose again to correct a refused or rejected one."
            .into(),
        parameters: json!({
            "type": "object",
            "properties": {
                "files": fichiers,
                "reason": {"type": "string", "description": "Why — the person reading the proposal sees it, and it becomes the commit message."}
            },
            "required": ["files", "reason"],
            "additionalProperties": false
        }),
    });
    defs.push(ToolDef {
        name: PROPOSALS.into(),
        description: "Lists the proposals of this backend: id, reason, files, verdict, and state \
                      (proposed, applied with its commit, rejected with why)."
            .into(),
        parameters: json!({"type": "object", "properties": {}, "additionalProperties": false}),
    });
    if mode == crate::propositions::ModeDeclarations::Auto {
        defs.push(ToolDef {
            name: APPLY.into(),
            description: "Applies a proposal: checked again on the current tree, written, put in service and committed.".into(),
            parameters: json!({"type": "object", "properties": {"id": {"type": "integer"}}, "required": ["id"], "additionalProperties": false}),
        });
        defs.push(ToolDef {
            name: REJECT.into(),
            description: "Rejects a proposal, saying why.".into(),
            parameters: json!({"type": "object", "properties": {"id": {"type": "integer"}, "why": {"type": "string"}}, "required": ["id", "why"], "additionalProperties": false}),
        });
    }
    defs
}

/// Les trois définitions d'outil.
pub fn tool_defs() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: LIST.into(),
            description: "Lists the declaration files of this backend (manifest, graphs, schemas, \
                          scripted nodes, views), as paths relative to its folder."
                .into(),
            parameters: json!({"type": "object", "properties": {}, "additionalProperties": false}),
        },
        ToolDef {
            name: READ.into(),
            description: "Reads one declaration file of this backend.".into(),
            parameters: json!({
                "type": "object",
                "properties": {"path": {"type": "string", "description": "Relative to the backend folder."}},
                "required": ["path"],
                "additionalProperties": false
            }),
        },
        ToolDef {
            name: DECLARE.into(),
            description: "Writes one declaration file (its whole content). It is checked as at \
                          load time, then put in service and committed with your reason; if it is \
                          refused, nothing is written and the refusal says why (and where)."
                .into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Relative to the backend folder."},
                    "content": {"type": "string", "description": "The whole new content of the file."},
                    "reason": {"type": "string", "description": "Why — it becomes the commit message."}
                },
                "required": ["path", "content", "reason"],
                "additionalProperties": false
            }),
        },
        ToolDef {
            name: DECLARE_SET.into(),
            description: "Writes several declaration files together (each whole), checked by one \
                          reload, put in service and committed in one commit with your reason — or \
                          none of them. Use it when files depend on each other: a component's \
                          .component.json, its .tsx view and its .clic.mmd graph."
                .into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "files": {
                        "type": "array",
                        "minItems": 1,
                        "items": {
                            "type": "object",
                            "properties": {
                                "path": {"type": "string", "description": "Relative to the backend folder."},
                                "content": {"type": "string", "description": "The whole new content of the file."}
                            },
                            "required": ["path", "content"],
                            "additionalProperties": false
                        }
                    },
                    "reason": {"type": "string", "description": "Why — it becomes the commit message."}
                },
                "required": ["files", "reason"],
                "additionalProperties": false
            }),
        },
    ]
}

impl DeclarationTools<'_> {
    fn ours(&self, call: &ToolCall) -> Option<Turn> {
        let answer =
            |content: Value| Some(Turn::tool_result(&call.id, &call.name, content.to_string()));
        if ![LIST, READ, DECLARE, DECLARE_SET, PROPOSE, PROPOSALS, APPLY, REJECT].contains(&call.name.as_str()) {
            return None;
        }
        let mode = self.backend.mode_declarations();
        if mode == crate::propositions::ModeDeclarations::Approbation && [DECLARE, DECLARE_SET, APPLY, REJECT].contains(&call.name.as_str()) {
            return answer(json!({"ok": false, "error": format!(
                "ce backend est en mode approbation : « {} » n'est pas à l'agent — propose (une personne applique dans l'IDE)",
                call.name
            )}));
        }
        let args: Value = match serde_json::from_str(&call.arguments) {
            Ok(args) => args,
            Err(e) => {
                return answer(json!({"ok": false, "error": format!("arguments illisibles : {e}")}))
            }
        };
        let text = |key: &str| args[key].as_str().unwrap_or_default().to_string();
        let fichiers = || -> Vec<(String, String)> {
            args["files"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|f| (f["path"].as_str().unwrap_or_default().to_string(), f["content"].as_str().unwrap_or_default().to_string()))
                .collect()
        };
        let proposition = |r: Result<crate::propositions::Proposition, String>| match r {
            Ok(p) => json!({"ok": true, "proposal": crate::propositions::resume(&p)}),
            Err(error) => json!({"ok": false, "error": error}),
        };
        match call.name.as_str() {
            PROPOSE => return answer(proposition(self.backend.proposer(&fichiers(), &text("reason"), "agent"))),
            PROPOSALS => {
                return answer(json!({"ok": true, "proposals": self.backend.propositions().iter().map(crate::propositions::resume).collect::<Vec<_>>()}))
            }
            APPLY => return answer(proposition(self.backend.appliquer(args["id"].as_u64().unwrap_or(0)))),
            REJECT => return answer(proposition(self.backend.rejeter(args["id"].as_u64().unwrap_or(0), &text("why")))),
            _ => {}
        }
        match call.name.as_str() {
            LIST => answer(json!({"ok": true, "paths": self.backend.list_declarations()})),
            READ => answer(match self.backend.read_declaration(&text("path")) {
                Ok(content) => json!({"ok": true, "content": content}),
                Err(error) => json!({"ok": false, "error": error}),
            }),
            _ => answer(
                match if call.name == DECLARE_SET {
                    let files: Vec<(String, String)> = args["files"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .map(|f| {
                            (f["path"].as_str().unwrap_or_default().to_string(), f["content"].as_str().unwrap_or_default().to_string())
                        })
                        .collect();
                    self.backend.declare_set(&files, &text("reason"))
                } else {
                    self.backend.declare(&text("path"), &text("content"), &text("reason"))
                } {
                    Ok(declared) => json!({
                        "ok": true,
                        "path": declared.path,
                        "paths": declared.paths,
                        "version": declared.version,
                        "commit": declared.commit,
                        "removed_reactions": declared.removed_reactions,
                        "mounted_reactions": declared.mounted_reactions,
                        "dropped_cursors": declared.dropped_cursors,
                        "reaction_errors": declared.reaction_errors,
                    }),
                    Err(error) => json!({"ok": false, "error": error}),
                },
            ),
        }
    }
}

impl crate::agent::ToolBox for DeclarationTools<'_> {
    fn tool_defs(&self) -> Vec<ToolDef> {
        let mut defs = self.inner.tool_defs();
        defs.extend(tool_defs_pour(self.backend.mode_declarations()));
        defs
    }

    fn is_async(&self, name: &str) -> bool {
        ![LIST, READ, DECLARE, DECLARE_SET].contains(&name) && self.inner.is_async(name)
    }

    fn call(&self, call: &ToolCall) -> Turn {
        self.ours(call).unwrap_or_else(|| self.inner.call(call))
    }

    fn call_in(&self, call: &ToolCall, run: &str) -> Turn {
        self.ours(call)
            .unwrap_or_else(|| self.inner.call_in(call, run))
    }

    fn call_with(&self, call: &ToolCall, run: &str, inv: &crate::agent::ToolInvocation) -> Turn {
        self.ours(call)
            .unwrap_or_else(|| self.inner.call_with(call, run, inv))
    }
}
