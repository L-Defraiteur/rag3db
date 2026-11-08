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

/// Une commande git dans `dossier`, sa sortie ; l'erreur dit la commande et ce que
/// git a écrit.
fn git(dossier: &Path, args: &[&str]) -> Result<String, String> {
    let sortie = std::process::Command::new("git")
        .arg("-C")
        .arg(dossier)
        .args(args)
        .output()
        .map_err(|e| format!("git {} : {e}", args.join(" ")))?;
    if !sortie.status.success() {
        return Err(format!("git {} : {}", args.join(" "), String::from_utf8_lossy(&sortie.stderr).trim()));
    }
    Ok(String::from_utf8_lossy(&sortie.stdout).trim().to_string())
}

/// Le nom d'un dossier frère pour une branche : `/` devient `-`, le reste qui n'est
/// pas sûr dans un nom de dossier devient `_`.
fn dossier_de_branche(branche: &str) -> String {
    branche.chars().map(|c| match c {
        '/' => '-',
        c if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') => c,
        _ => '_',
    }).collect()
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

    fn open_branch(&self, path: &Path, branch: &str, from: Option<&str>) -> Result<String, String> {
        let lieu = path
            .canonicalize()
            .map_err(|e| format!("{} n'existe pas ({e}) : donnez un worktree ou un dépôt de ce poste", path.display()))?;
        let depot = PathBuf::from(git(&lieu, &["rev-parse", "--show-toplevel"]).map_err(|_| format!("{} n'est dans aucun dépôt git : open_branch ouvre une branche d'un dépôt", lieu.display()))?);
        // Le dépôt principal (pas un worktree lié) : ses worktrees frères vivent à côté de lui.
        let commun = git(&depot, &["rev-parse", "--path-format=absolute", "--git-common-dir"])?;
        let principal = Path::new(&commun).parent().map(Path::to_path_buf).unwrap_or_else(|| depot.clone());
        git(&principal, &["check-ref-format", "--branch", branch]).map_err(|_| format!("« {branch} » n'est pas un nom de branche valide"))?;
        let nom = principal.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "depot".into());
        let parent = principal.parent().ok_or_else(|| format!("{} n'a pas de dossier parent", principal.display()))?;
        let frere = parent.join(format!("{nom}-{}", dossier_de_branche(branch)));
        // La branche déjà ouverte dans un worktree : on l'attache, sans git.
        let ouverts = git(&principal, &["worktree", "list", "--porcelain"])?;
        let mut deja: Option<PathBuf> = None;
        let mut courant: Option<PathBuf> = None;
        for l in ouverts.lines() {
            if let Some(p) = l.strip_prefix("worktree ") {
                courant = Some(PathBuf::from(p));
            } else if l.strip_prefix("branch ").is_some_and(|b| b == format!("refs/heads/{branch}")) {
                deja = courant.clone();
            }
        }
        let (worktree, commande) = match deja {
            Some(w) => (w, None),
            None => {
                if frere.exists() {
                    return Err(format!("{} existe déjà et ne tient pas la branche {branch} : choisissez une autre branche, ou attachez ce dossier", frere.display()));
                }
                let existe = git(&principal, &["rev-parse", "--verify", "--quiet", &format!("refs/heads/{branch}")]).is_ok();
                let frere_txt = frere.display().to_string();
                let depart = from.unwrap_or("HEAD");
                let args: Vec<&str> = if existe { vec!["worktree", "add", &frere_txt, branch] } else { vec!["worktree", "add", "-b", branch, &frere_txt, depart] };
                git(&principal, &args)?;
                (frere.clone(), Some(format!("git -C {} {}", principal.display(), args.join(" "))))
            }
        };
        let attache = self.attach(&worktree)?;
        Ok(match commande {
            Some(c) => format!("Branche {branch} ouverte dans le worktree {} (commande : `{c}`). {attache}", worktree.display()),
            None => format!("La branche {branch} est déjà ouverte dans le worktree {} : aucune commande git. {attache}", worktree.display()),
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

    /// **Un worktree attaché paraît sous « Dépôts » sans recharger la page** (c7, 8
    /// novembre 2026, la suite de la barre). Dans le chat de l'atelier, l'utilisatrice
    /// demande d'ouvrir un worktree ; le modèle appelle `attach_worktree` ; la barre, en
    /// vue par dépôt, montre le dépôt et sa branche dès `RepoAttached`, la page jamais
    /// rechargée.
    #[test]
    #[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
    fn un_worktree_attache_parait_sous_depots_sans_recharger_la_page() {
        use crate::daemon_host::navigateur::{navigateur_ou_saut, Navigateur};
        const TEMOIN: &str = "un_worktree_attache_parait_sous_depots_sans_recharger_la_page";
        let Some(binaire) = navigateur_ou_saut(TEMOIN) else {
            return;
        };
        let d = tempfile::tempdir().unwrap();
        let depot = d.path().join("rag3db-essai");
        std::fs::create_dir_all(depot.join("src")).unwrap();
        std::fs::write(depot.join("src/lib.rs"), "pub fn un() -> u32 { 1 }\n").unwrap();
        for args in [&["init", "-q", "-b", "essai"][..], &["config", "user.name", "Personne Test"], &["config", "user.email", "personne@example.org"], &["add", "-A"], &["commit", "-q", "-m", "le dépôt"]] {
            git(&depot, args);
        }
        let gabarits = d.path().join("atelier/templates");
        std::fs::create_dir_all(gabarits.parent().unwrap()).unwrap();
        crate::code_tree::copy(&std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates"), &gabarits, false).unwrap();
        let atelier = gabarits.join("backends/atelier/backend.json");
        let racines = crate::code_tree::Roots { config: d.path().join("config"), data: d.path().join("data") };
        let chemin = depot.display().to_string();
        let model: ChatModel = Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let chemin = chemin.clone();
            Ok(Arc::new(CallbackLlm::new("faux-atelier", 8192, move |turns: &[Turn], opts, sink| {
                let depuis = turns.iter().rposition(|t| t.role == "user").unwrap_or(0);
                if turns[depuis..].iter().any(|t| t.role == "tool") {
                    MockLlm::new("C'est fait.").generate(turns, opts, sink)
                } else {
                    MockLlm::new("").with_tool_calls(vec![("attach_worktree".to_string(), json!({ "path": chemin }).to_string())]).generate(turns, opts, sink)
                }
            })))
        });
        let (runs, chats) = (d.path().join("runs"), d.path().join("chats"));
        let (adresse, fil) = crate::daemon_host::tests::hote_garde_avec("j", move |h| h.with_racines_de_code(racines).with_runs_root(runs).with_chats_root(chats).with_chat_model(model));
        let racine = format!("http://{adresse}");
        let ide = |methode: &str, route: &str, corps: Value| -> Value {
            let agent = crate::daemon::agent();
            let mut r = if methode == "GET" {
                agent.get(format!("{racine}{route}")).header("Cookie", "rag3weaver_ide=j").call().unwrap()
            } else {
                agent.post(format!("{racine}{route}")).header("Cookie", "rag3weaver_ide=j").header("content-type", "application/json").send(corps.to_string()).unwrap()
            };
            serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap()
        };
        assert_eq!(ide("POST", "/ide/describe", json!({ "manifest": atelier }))["ok"], true);
        let etat = ide("GET", "/ide/etat", Value::Null);
        let canon = atelier.canonicalize().unwrap();
        let id = etat["backends"]
            .as_array()
            .and_then(|a| a.iter().find(|b| b["manifest"].as_str().map(Path::new) == Some(canon.as_path())))
            .and_then(|b| b["id"].as_str())
            .unwrap_or_else(|| panic!("l'atelier dans l'état : {etat:#}"))
            .to_string();

        let mut n = Navigateur::lancer(&binaire);
        n.commande("Emulation.setDeviceMetricsOverride", json!({ "width": 1920, "height": 1080, "deviceScaleFactor": 1, "mobile": false }));
        n.aller(&format!("{racine}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
        n.aller(
            &format!("{racine}/ide/code/{id}/chat"),
            "document.querySelector('#send') && !document.querySelector('#send').disabled && document.getElementById('lien')?.textContent === 'flux : ouvert'",
        );
        n.evaluer("(() => { const s = document.querySelector('#vue-barre'); const set = Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, 'value').set; set.call(s, 'repo'); s.dispatchEvent(new Event('change', { bubbles: true })); })()");
        n.attendre("!!document.querySelector('#arbre-depots')", "la vue par dépôt");
        let sel = "#arbre-depots .depot[data-depot=\"rag3db-essai\"]";
        assert_eq!(n.evaluer(&format!("!!document.querySelector('{sel}')")), json!(false), "le dépôt paraît avant d'être attaché");
        // Une marque dans la page : un rechargement l'effacerait.
        n.evaluer("window.__pas_rechargee = 1");
        n.evaluer(&format!("document.querySelector('#message').value = 'Ouvre le worktree {}'; document.querySelector('#send').click()", depot.display()));
        n.attendre("document.querySelector('#status')?.textContent === 'fini'", "le tour de l'atelier");
        n.attendre(&format!("!!document.querySelector('{sel} .espace')"), "le dépôt attaché sous « Dépôts »");
        let espace = n.evaluer(&format!("document.querySelector('{sel} .espace').textContent"));
        assert!(espace.as_str().unwrap_or_default().contains("rag3db-essai · essai"), "le dépôt et sa branche : {espace}");
        assert_eq!(n.evaluer("window.__pas_rechargee ?? null"), json!(1), "la page s'est rechargée");

        assert!(n.erreurs.is_empty(), "erreurs à la console : {:#?}", n.erreurs);
        drop(n);
        let (statut, _) = crate::daemon_host::tests::demander("POST", &format!("{racine}/arret"), Some("j"));
        assert_eq!(statut, 200);
        fil.join().unwrap();
    }

    /// **L'atelier ouvre une branche dans un worktree frère** (page 03, le pas d'après
    /// attach_worktree ; c7 : « aucun git caché : la commande git est dite dans la
    /// réponse »). Une branche neuve naît dans `<dépôt>-<branche>` à côté du dépôt, le
    /// worktree est attaché, la réponse dit la commande lancée ; rouvrir la même branche
    /// ne lance aucune commande ; un nom invalide est refusé.
    #[test]
    fn the_workshop_opens_a_branch_in_a_sibling_worktree() {
        let d = tempfile::tempdir().unwrap();
        let depot = d.path().join("rag3db-essai");
        std::fs::create_dir_all(depot.join("src")).unwrap();
        std::fs::write(depot.join("src/lib.rs"), "pub fn un() -> u32 { 1 }\n").unwrap();
        for args in [&["init", "-q", "-b", "essai"][..], &["config", "user.name", "Personne Test"], &["config", "user.email", "personne@example.org"], &["add", "-A"], &["commit", "-q", "-m", "le dépôt"]] {
            git(&depot, args);
        }
        let gabarits = d.path().join("atelier/templates");
        std::fs::create_dir_all(gabarits.parent().unwrap()).unwrap();
        crate::code_tree::copy(&std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates"), &gabarits, false).unwrap();
        let atelier = gabarits.join("backends/atelier/backend.json");
        let racines = crate::code_tree::Roots { config: d.path().join("config"), data: d.path().join("data") };
        let model: ChatModel = Arc::new(|_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> { Ok(Arc::new(MockLlm::new("rien"))) });
        let host = Arc::new(hote(d.path(), model).with_racines_de_code(racines.clone()).with_runs_root(d.path().join("runs")));
        host.se_connaitre();
        let h = host.hosted(&atelier).unwrap();
        host.with_backend(&h, |_| ()).expect("l'atelier s'ouvre");
        let appel = |args: Value| -> Result<String, String> {
            host.with_backend(&h, |b| b.call("open_branch", args)).unwrap().map(|r| r["result"].as_str().unwrap_or_default().to_string())
        };

        let dit = appel(json!({ "path": depot, "branch": "lot/neuve" })).expect("la branche s'ouvre");
        let frere = d.path().join("rag3db-essai-lot-neuve");
        assert!(frere.is_dir(), "le worktree frère : {dit}");
        let tete = std::process::Command::new("git").args(["rev-parse", "--abbrev-ref", "HEAD"]).current_dir(&frere).output().unwrap();
        assert_eq!(String::from_utf8_lossy(&tete.stdout).trim(), "lot/neuve");
        assert!(dit.contains("Branche lot/neuve ouverte dans le worktree") && dit.contains("worktree add -b lot/neuve") && dit.contains("(commande : `git -C "), "la commande est dite : {dit}");
        assert!(dit.contains("Worktree attaché") && racines.manifest("code-rag3db-essai-lot-neuve").is_file(), "le worktree est attaché : {dit}");
        assert!(!dit.contains("backend"), "le vocabulaire de l'utilisatrice : {dit}");

        let encore = appel(json!({ "path": depot, "branch": "lot/neuve" })).expect("rouvrir");
        assert!(encore.contains("déjà ouverte") && encore.contains("aucune commande git"), "{encore}");
        let refus = appel(json!({ "path": depot, "branch": "a..b" })).expect_err("un nom invalide");
        assert!(refus.contains("pas un nom de branche valide"), "{refus}");
    }
}
