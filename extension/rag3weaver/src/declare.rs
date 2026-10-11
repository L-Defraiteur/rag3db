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
//! 3. accepté : mettre en service (`apply_reload`), puis **un commit** dans le
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

/// Les extensions d'une déclaration.
const KINDS: [&str; 11] = [
    "json", "mmd", "ts", "js", "rhai", "html", "htm", "jinja", "txt", "xml", "md",
];

/// Ce qu'une déclaration acceptée a mis en service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declared {
    pub path: String,
    /// La version des déclarations en service après elle.
    pub version: u64,
    /// Le commit qui la porte ; `None` si le contenu n'a pas changé.
    pub commit: Option<String>,
    /// Les réactions que cette version retire (l'hôte oublie leur curseur).
    pub removed_reactions: Vec<String>,
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
        let _one = DECLARING.lock().unwrap_or_else(|e| e.into_inner());
        if reason.trim().is_empty() {
            return Err("une déclaration dit pourquoi : la raison est vide".into());
        }
        git_present()?;
        let target = self.declaration_path(path)?;
        let before = std::fs::read_to_string(&target).ok();
        if before.as_deref() == Some(content) {
            return Ok(Declared {
                path: path.to_string(),
                version: self.declarations_version(),
                commit: None,
                removed_reactions: vec![],
            });
        }
        write_atomically(&target, content)?;
        let undo = |target: &Path| match &before {
            Some(old) => write_atomically(target, old),
            None => std::fs::remove_file(target).map_err(|e| e.to_string()),
        };
        let pending = match self.check_reload() {
            Ok(pending) => pending,
            Err(refused) => {
                undo(&target)?;
                return Err(format!(
                    "{path} : déclaration refusée, rien n'est écrit — {refused}"
                ));
            }
        };
        let dir = self.prepared().directory.clone();
        let reloaded = self.apply_reload(pending);
        match commit(&dir, path, reason) {
            Ok(hash) => Ok(Declared {
                path: path.to_string(),
                version: reloaded.version,
                commit: Some(hash),
                removed_reactions: reloaded.removed_reactions,
            }),
            Err(git) => {
                // Pas de déclaration sans sa raison : on revient en arrière.
                undo(&target)?;
                self.reload().map_err(|e| {
                    format!("{path} : le commit a échoué ({git}) et le retour arrière aussi : {e}")
                })?;
                Err(format!(
                    "{path} : git a refusé le commit, rien n'est écrit — {git}"
                ))
            }
        }
    }

    /// Le chemin absolu d'une déclaration, ou pourquoi ce n'en est pas une.
    fn declaration_path(&self, path: &str) -> Result<PathBuf, String> {
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

/// Commite **ce chemin seul** dans le dépôt qui englobe le dossier — créé
/// s'il n'y en a aucun au-dessus —, sous l'identité git du dépôt.
fn commit(dir: &Path, path: &str, reason: &str) -> Result<String, String> {
    if git(dir, &["rev-parse", "--show-toplevel"]).is_err() {
        git(dir, &["init", "-q"])?;
    }
    git(dir, &["add", "--", path])?;
    git(
        dir,
        &[
            "commit",
            "-q",
            "-m",
            reason.trim(),
            "-m",
            &format!("declare: {path}"),
            "--",
            path,
        ],
    )?;
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
    ]
}

impl DeclarationTools<'_> {
    fn ours(&self, call: &ToolCall) -> Option<Turn> {
        let answer =
            |content: Value| Some(Turn::tool_result(&call.id, &call.name, content.to_string()));
        if ![LIST, READ, DECLARE].contains(&call.name.as_str()) {
            return None;
        }
        let args: Value = match serde_json::from_str(&call.arguments) {
            Ok(args) => args,
            Err(e) => {
                return answer(json!({"ok": false, "error": format!("arguments illisibles : {e}")}))
            }
        };
        let text = |key: &str| args[key].as_str().unwrap_or_default().to_string();
        match call.name.as_str() {
            LIST => answer(json!({"ok": true, "paths": self.backend.list_declarations()})),
            READ => answer(match self.backend.read_declaration(&text("path")) {
                Ok(content) => json!({"ok": true, "content": content}),
                Err(error) => json!({"ok": false, "error": error}),
            }),
            _ => answer(
                match self
                    .backend
                    .declare(&text("path"), &text("content"), &text("reason"))
                {
                    Ok(declared) => json!({
                        "ok": true,
                        "path": declared.path,
                        "version": declared.version,
                        "commit": declared.commit,
                        "removed_reactions": declared.removed_reactions,
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
        defs.extend(tool_defs());
        defs
    }

    fn is_async(&self, name: &str) -> bool {
        ![LIST, READ, DECLARE].contains(&name) && self.inner.is_async(name)
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
