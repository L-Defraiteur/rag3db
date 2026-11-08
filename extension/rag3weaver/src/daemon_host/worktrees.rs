//! **L'atelier : l'hôte attache les worktrees** (page
//! 07-novembre-2026-everything-declarative/03, lot 0 ; Lucie, 8 novembre 2026 :
//! « l'atelier oui absolument »). `attach_worktree` inscrit un worktree déjà sur le
//! disque depuis le gabarit de code livré (comme `code-tree add`), l'ouvre, lance son
//! premier index en fond et le publie sur le flux (`repos`, `RepoAttached`) ;
//! `list_worktrees` dit les worktrees inscrits. Aucune commande git n'est lancée, rien
//! n'est cloné. Les phrases parlent de dépôt, de worktree, de branche et d'index.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Weak};

use serde_json::{json, Value};

use super::DaemonHost;
use crate::dataflow::worktree_nodes::WorktreeHost;

/// L'hôte des worktrees que l'hôte pose sur chaque backend qu'il ouvre.
pub struct HoteDesWorktrees {
    pub hote: Weak<DaemonHost>,
}

impl HoteDesWorktrees {
    fn hote(&self) -> Result<Arc<DaemonHost>, String> {
        self.hote.upgrade().ok_or_else(|| "l'hôte de rag3weaver s'arrête : réessayez quand il sera revenu".to_string())
    }
}

/// Le gabarit de code livré, copié sous un dossier `templates/` à soi : `code-tree
/// add` prend son gabarit sous un tel dossier. Rendu avec le dossier à effacer.
fn gabarit_livre(sous: &Path) -> Result<(PathBuf, PathBuf), String> {
    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let dossier = sous.join(format!(".atelier-gabarit-{}-{}", std::process::id(), N.fetch_add(1, std::sync::atomic::Ordering::Relaxed)));
    let templates = dossier.join("templates");
    std::fs::create_dir_all(&dossier).map_err(|e| format!("{} : {e}", dossier.display()))?;
    crate::code_tree::copy(&crate::builtin_templates::root()?, &templates, false)?;
    Ok((templates.join("backends/code/backend.json"), dossier))
}

/// L'état de l'index d'un manifeste hébergé, dit en mots, d'après `/etat`.
fn etat_de_l_index(etat: &Value, manifeste: &Path) -> String {
    let b = etat["backends"].as_array().and_then(|bs| bs.iter().find(|b| b["manifest"].as_str().map(Path::new) == Some(manifeste)));
    match b {
        None => "pas ouvert".into(),
        Some(b) if b["open"] != true => "pas ouvert".into(),
        Some(b) => match &b["index"] {
            Value::Object(entites) if !entites.is_empty() => {
                // Par entité, l'état du plein texte et des vecteurs (`ready`, `never`, …).
                let n = entites.len();
                let prets = |cle: &str| entites.values().filter(|e| e[cle] == "ready").count();
                let en_cours = entites.values().any(|e| e["text_percent"].is_number() || e["vectors_percent"].as_f64().is_some_and(|p| p > 0.0 && p < 100.0));
                format!(
                    "plein texte {}/{n} entités, vecteurs {}/{n}{}",
                    prets("text"),
                    prets("vectors"),
                    if en_cours { ", en cours" } else { "" }
                )
            }
            Value::Null | Value::Object(_) => "ouvert, pas encore indexé".into(),
            Value::String(s) => s.clone(),
            autre => autre.to_string(),
        },
    }
}

impl WorktreeHost for HoteDesWorktrees {
    fn attach(&self, path: &Path) -> Result<String, String> {
        let worktree = path
            .canonicalize()
            .map_err(|e| format!("le worktree {} n'existe pas ({e}) : donnez le chemin d'un dossier sur ce poste", path.display()))?;
        if !worktree.is_dir() {
            return Err(format!("{} n'est pas un dossier : un worktree est un dossier", worktree.display()));
        }
        let hote = self.hote()?;
        let racines = hote.racines_de_code()?;
        let nom = crate::code_tree::name_of_tree(&worktree)?;
        let manifeste = racines.manifest(&nom);
        let deja = manifeste.exists();
        if !deja {
            std::fs::create_dir_all(&racines.data).map_err(|e| format!("{} : {e}", racines.data.display()))?;
            let (source, a_effacer) = gabarit_livre(&racines.data)?;
            let inscrit = crate::code_tree::add_files(&racines, &worktree, &source.display().to_string(), Some(&nom));
            let _ = std::fs::remove_dir_all(&a_effacer);
            inscrit?;
        }
        // Comparé à `/etat` par son chemin canonique, comme l'hôte le tient.
        let manifeste = manifeste.canonicalize().unwrap_or(manifeste);
        let h = hote.hosted(&manifeste)?;
        hote.with_backend(&h, |_| ())?;
        let depot = super::repo_of(&worktree);
        let branche = super::branch_of(&worktree);
        // Le premier index part en fond ; un worktree déjà attaché garde le sien.
        let index = if deja {
            None
        } else {
            Some(match hote.with_backend(&h, |b| b.call("index", json!({ "confirm": true }))) {
                Ok(Ok(_)) => "son index est lancé en fond".to_string(),
                Ok(Err(e)) | Err(e) => format!("son index n'a pas pu partir : {e}"),
            })
        };
        hote.ide_flux().publier(
            "repos",
            &manifeste.display().to_string(),
            json!({ "kind": "RepoAttached", "worktree": worktree, "depot": depot, "branche": branche, "deja": deja }),
        );
        let ou = match (&depot, &branche) {
            (Some(d), Some(b)) => format!("dépôt {d}, branche {b}"),
            (Some(d), None) => format!("dépôt {d}, tête détachée"),
            (None, _) => "hors d'un dépôt git".to_string(),
        };
        Ok(match index {
            Some(i) => format!("Worktree attaché : {} ({ou}) ; {i}. list_worktrees dit où il en est.", worktree.display()),
            None => format!(
                "Le worktree {} est déjà attaché ({ou}) : il est ouvert ; son index : {}.",
                worktree.display(),
                etat_de_l_index(&hote.state(), &manifeste)
            ),
        })
    }

    fn list(&self) -> Result<String, String> {
        let hote = self.hote()?;
        let manifestes = hote.racines_de_code()?.registered_manifests();
        if manifestes.is_empty() {
            return Ok("Aucun worktree attaché : attach_worktree en attache un.".into());
        }
        let etat = hote.state();
        let mut lignes = vec![format!("{} worktree(s) attaché(s) :", manifestes.len())];
        for m in manifestes {
            let m = m.canonicalize().unwrap_or(m);
            let racine = super::espace_du_manifeste(&m);
            let (depot, branche) = racine.as_deref().map(|r| (super::repo_of(r), super::branch_of(r))).unwrap_or((None, None));
            lignes.push(format!(
                "- {} — dépôt {}, branche {} ; index : {}",
                racine.as_deref().map(|r| r.display().to_string()).unwrap_or_else(|| "?".into()),
                depot.as_deref().unwrap_or("hors git"),
                branche.as_deref().unwrap_or("détachée"),
                etat_de_l_index(&etat, &m)
            ));
        }
        Ok(lignes.join("\n"))
    }
}

#[cfg(all(test, feature = "rag3db-native"))]
mod tests {
    use super::super::chat::ChatModel;
    use super::super::chat_qui_code::hote;
    use crate::llm::{CallbackLlm, Llm, MockLlm, Turn};
    use serde_json::{json, Value};
    use std::path::Path;
    use std::sync::{Arc, Mutex};

    fn git(dir: &Path, args: &[&str]) {
        assert!(std::process::Command::new("git").args(args).current_dir(dir).status().unwrap().success(), "git {args:?}");
    }

    /// Un écrivain qui garde ce que le flux lui envoie.
    struct Capte(Arc<Mutex<Vec<u8>>>);
    impl std::io::Write for Capte {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// **L'atelier ouvre un worktree existant par un mot** (page 03 du 7 novembre 2026,
    /// lot 0 ; Lucie, 8 novembre : « l'atelier oui absolument »). Dans le chat de l'atelier,
    /// le modèle appelle `attach_worktree` avec le chemin d'un petit dépôt git : le
    /// manifeste du worktree est posé, il paraît dans `/etat`, `RepoAttached` passe sur le
    /// flux, son index part en fond, `list_worktrees` le dit ; un second appel dit « déjà
    /// attaché ». Aucune commande git n'est lancée par l'atelier.
    #[test]
    fn the_workshop_attaches_an_existing_worktree_from_the_chat() {
        let d = tempfile::tempdir().unwrap();
        let depot = d.path().join("rag3db-essai");
        std::fs::create_dir_all(depot.join("src")).unwrap();
        std::fs::write(depot.join("src/lib.rs"), "pub fn un() -> u32 { 1 }\n").unwrap();
        for args in [&["init", "-q", "-b", "essai"][..], &["config", "user.name", "Personne Test"], &["config", "user.email", "personne@example.org"], &["add", "-A"], &["commit", "-q", "-m", "le dépôt"]] {
            git(&depot, args);
        }
        // L'atelier : le gabarit livré, copié comme un manifeste de l'utilisatrice.
        let gabarits = d.path().join("atelier/templates");
        std::fs::create_dir_all(gabarits.parent().unwrap()).unwrap();
        crate::code_tree::copy(&std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates"), &gabarits, false).unwrap();
        let atelier = gabarits.join("backends/atelier/backend.json");
        let racines = crate::code_tree::Roots { config: d.path().join("config"), data: d.path().join("data") };

        let vus = Arc::new(Mutex::new(Vec::<String>::new()));
        let chemin = depot.display().to_string();
        let model: ChatModel = {
            let vus = vus.clone();
            Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
                let (vus, chemin) = (vus.clone(), chemin.clone());
                Ok(Arc::new(CallbackLlm::new("faux-atelier", 8192, move |turns: &[Turn], opts, sink| {
                    let depuis = turns.iter().rposition(|t| t.role == "user").unwrap_or(0);
                    let resultats: Vec<String> = turns[depuis..].iter().filter(|t| t.role == "tool").map(|t| t.content.clone()).collect();
                    match resultats.last() {
                        None => MockLlm::new("").with_tool_calls(vec![("attach_worktree".to_string(), json!({ "path": chemin }).to_string())]).generate(turns, opts, sink),
                        Some(r) => {
                            vus.lock().unwrap().push(r.clone());
                            MockLlm::new("C'est fait.").generate(turns, opts, sink)
                        }
                    }
                })))
            })
        };
        let host = Arc::new(hote(d.path(), model).with_racines_de_code(racines.clone()).with_runs_root(d.path().join("runs")));
        host.se_connaitre();
        let flux = Arc::new(Mutex::new(Vec::new()));
        host.ide_flux().abonner(Box::new(Capte(flux.clone())), Some(["repos".to_string()].into_iter().collect()), None);
        let h = host.hosted(&atelier).unwrap();
        host.with_backend(&h, |_| ()).expect("l'atelier s'ouvre");

        host.chat_turn(&h.id, &format!("Ouvre le worktree {}", depot.display())).expect("le tour");
        let vu = vus.lock().unwrap().first().cloned().unwrap_or_default();
        assert!(vu.contains("Worktree attaché") && vu.contains("dépôt rag3db-essai, branche essai") && vu.contains("son index est lancé en fond"), "{vu}");
        assert!(!vu.contains("backend"), "le vocabulaire de l'utilisatrice : {vu}");
        // 1. Le manifeste posé.
        let manifeste = racines.manifest("code-rag3db-essai");
        assert!(manifeste.is_file(), "{}", manifeste.display());
        // 2. Le dépôt dans /etat.
        let etat = host.state();
        let canon = manifeste.canonicalize().unwrap();
        assert!(etat["backends"].as_array().unwrap().iter().any(|b| b["manifest"].as_str().map(Path::new) == Some(canon.as_path()) && b["open"] == true), "{etat:#}");
        // 3. RepoAttached sur le flux.
        let capte = String::from_utf8_lossy(&flux.lock().unwrap()).into_owned();
        assert!(capte.contains("RepoAttached") && capte.contains("rag3db-essai"), "{capte}");
        // 4. L'index lancé, que list_worktrees dit ; le second appel dit « déjà attaché ».
        let liste = host.with_backend(&h, |b| b.call("list_worktrees", json!({}))).unwrap().unwrap();
        let liste = liste["result"].as_str().unwrap_or_default().to_string();
        assert!(liste.contains("1 worktree(s) attaché(s)") && liste.contains(&depot.canonicalize().unwrap().display().to_string()) && liste.contains("branche essai"), "{liste}");
        let encore = host.with_backend(&h, |b| b.call("attach_worktree", json!({ "path": depot }))).unwrap().unwrap();
        let encore = encore["result"].as_str().unwrap_or_default().to_string();
        assert!(encore.contains("déjà attaché") && encore.contains("il est ouvert"), "{encore}");
        let _: Value = etat;
    }
}
