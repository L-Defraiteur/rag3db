//! **L'atelier lit et écrit** (Lucie, 8 novembre 2026 au soir ; page
//! `09-novembre-2026-atelier-lire-ecrire/01-conception.md`). Les outils de fichier de
//! l'atelier passent par l'hôte : un chemin dans un worktree attaché va au backend de code
//! de ce worktree (sa source, son index, sa réindexation), un autre à la racine de
//! l'atelier, sans index ; hors de la racine, le modèle déclare un espace ou demande
//! l'accord.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::{json, Value};

use super::{DaemonHost, Hosted};
use crate::dataflow::fichier_nodes::Edition;

/// **Une demande d'accord** (ask_permission) : le chemin, le mode, la raison du modèle, et
/// la décision quand elle est prise. Une **commande en attente** du chat de l'atelier (lot 7)
/// en est une aussi : sa ligne et le backend dont la porte l'accordera ; son chemin est le
/// domaine de travail.
#[derive(Debug, Clone)]
pub(super) struct Demande {
    pub chemin: PathBuf,
    pub mode: String,
    pub raison: String,
    pub decision: Option<bool>,
    /// La ligne de commande et l'id du backend dont la porte (`Garde`) l'accorde.
    pub commande: Option<(String, String)>,
}

/// Les demandes et les chemins ouverts de l'atelier.
#[derive(Debug, Default)]
pub(super) struct Accords {
    demandes: std::collections::HashMap<String, Demande>,
    /// Les chemins accordés et leur mode (`read` ou `write` ; `write` vaut aussi lecture).
    ouverts: Vec<(PathBuf, String)>,
    suivant: u64,
}

/// Le worktree attaché qui contient un chemin : son backend, sa racine, son dépôt et sa
/// branche.
pub(super) struct Worktree {
    pub hosted: Arc<Hosted>,
    pub racine: PathBuf,
    pub depot: Option<String>,
    pub branche: Option<String>,
}

impl Worktree {
    fn nom(&self) -> String {
        match (&self.depot, &self.branche) {
            (Some(d), Some(b)) => format!("{d} · {b}"),
            (Some(d), None) => d.clone(),
            _ => self.racine.display().to_string(),
        }
    }
}

/// Un chemin résolu : absolu, dans la racine de l'atelier, et le worktree attaché qui
/// le contient s'il y en a un.
pub(super) struct Resolu {
    pub chemin: PathBuf,
    pub worktree: Option<Worktree>,
}

/// La forme canonique d'un chemin qui peut ne pas exister encore (un fichier à créer) :
/// son dossier canonique et son nom.
fn canonique(p: &Path) -> PathBuf {
    if let Ok(c) = p.canonicalize() {
        return c;
    }
    match (p.parent(), p.file_name()) {
        (Some(d), Some(n)) => canonique(d).join(n),
        _ => p.to_path_buf(),
    }
}

impl DaemonHost {
    /// **Résoudre un chemin de l'atelier** : relatif à sa racine, ou absolu (`~/`
    /// s'étend) ; refusé hors de la racine, avec les deux suites que le modèle a.
    pub(super) fn atelier_resoudre(&self, racine: Option<&Path>, path: &str, mode: &str) -> Result<Resolu, String> {
        let racine = racine.ok_or("l'atelier n'a pas de racine de travail (`workspace.root`)")?;
        let racine = canonique(racine);
        let p = crate::backend_code::etendre_tilde(path.trim());
        let chemin = canonique(&if p.is_absolute() { p } else { racine.join(p) });
        if !chemin.starts_with(&racine) && !self.accorde(&chemin, mode) {
            return Err(format!(
                "{} est hors de la racine de l'atelier ({}). Deux suites : déclarer l'espace (attach_worktree ou open_branch sur le worktree qui le contient), ou demander l'accord de l'utilisateur (ask_permission, avec la raison).",
                chemin.display(),
                racine.display()
            ));
        }
        Ok(Resolu { worktree: self.worktree_qui_contient(&chemin)?, chemin })
    }

    /// Le worktree attaché (un arbre inscrit) dont la racine contient `chemin` : le plus
    /// profond s'il y en a plusieurs.
    fn worktree_qui_contient(&self, chemin: &Path) -> Result<Option<Worktree>, String> {
        let Ok(racines) = self.racines_de_code() else { return Ok(None) };
        let mut meilleur: Option<(PathBuf, PathBuf)> = None;
        for m in racines.registered_manifests() {
            let Some(v) = std::fs::read_to_string(&m).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()) else { continue };
            let Some(root) = v["workspace"]["root"].as_str().map(PathBuf::from) else { continue };
            let root = canonique(&if root.is_absolute() { root } else { m.parent().unwrap_or(Path::new("/")).join(root) });
            if chemin.starts_with(&root) && meilleur.as_ref().is_none_or(|(r, _)| root.components().count() > r.components().count()) {
                meilleur = Some((root, m));
            }
        }
        let Some((racine, m)) = meilleur else { return Ok(None) };
        let m = m.canonicalize().unwrap_or(m);
        let hosted = self.hosted(&m)?;
        Ok(Some(Worktree { hosted, depot: super::repo_of(&racine), branche: super::branch_of(&racine), racine }))
    }

    /// **L'atelier peut-il écrire ce worktree ?** Oui s'il est né de l'atelier (ouvert par
    /// `open_branch`, marqué à côté de son manifeste), ou si l'utilisatrice l'a accordé
    /// (ask_permission) ; sinon non (décision de c7, 9 novembre 2026 : `task_writes` garde
    /// son sens pour l'arbre qu'elle tient).
    fn atelier_peut_ecrire(&self, w: &Worktree, chemin: &Path) -> bool {
        if self.accorde(chemin, "write") {
            return true;
        }
        let Ok(racines) = self.racines_de_code() else { return false };
        crate::code_tree::name_of_tree(&w.racine).is_ok_and(|nom| racines.marque_de_l_atelier(&nom).is_file())
    }

    /// Un accord couvre-t-il ce chemin pour ce mode ? (`write` vaut aussi lecture.)
    fn accorde(&self, chemin: &Path, mode: &str) -> bool {
        let a = self.accords.lock().unwrap_or_else(|p| p.into_inner());
        a.ouverts.iter().any(|(p, m)| chemin.starts_with(p) && (m == "write" || m == mode))
    }

    /// **Demander l'accord** (`ask_permission`) : la demande gardée, sa carte rendue.
    pub(super) fn atelier_demander(&self, racine: Option<&Path>, path: &str, mode: &str, raison: &str) -> Result<crate::dataflow::worktree_nodes::WorktreeAnswer, String> {
        if !matches!(mode, "read" | "write") {
            return Err(format!("ask_permission : le mode est « read » ou « write » (reçu « {mode} »)"));
        }
        if raison.trim().is_empty() {
            return Err("ask_permission : donnez la raison (`reason`) : l'utilisatrice décide sur elle".into());
        }
        let p = crate::backend_code::etendre_tilde(path.trim());
        let chemin = canonique(&match (p.is_absolute(), racine) {
            (false, Some(r)) => r.join(p),
            _ => p,
        });
        let id = {
            let mut a = self.accords.lock().unwrap_or_else(|p| p.into_inner());
            a.suivant += 1;
            let id = format!("accord-{}-{}", crate::events::maintenant_ms(), a.suivant);
            a.demandes.insert(id.clone(), Demande { chemin: chemin.clone(), mode: mode.to_string(), raison: raison.trim().to_string(), decision: None, commande: None });
            id
        };
        let mot = if mode == "write" { "écrire" } else { "lire" };
        Ok(crate::dataflow::worktree_nodes::WorktreeAnswer {
            text: format!(
                "Demande {id} posée : {mot} {} ({}). L'utilisatrice accorde ou refuse sur la carte sous ce message ; attendez sa réponse, puis réessayez.",
                chemin.display(),
                raison.trim()
            ),
            ui: vec![json!({ "kind": "accord", "sujets": ["accords"], "id": id, "chemin": chemin, "mode": mode, "raison": raison.trim() })],
        })
    }

    /// **Une commande en attente devient une demande d'accord** (lot 7 ; c7, 9 novembre
    /// 2026) : le chat de l'atelier, hors d'une tâche, a vu la porte mettre `ligne` en
    /// attente ; la demande est gardée avec le backend dont la porte l'accordera, et la
    /// carte rendue (genre `commande`). Rend le texte à ajouter au résultat de l'outil, et
    /// la carte.
    pub(super) fn atelier_demander_commande(&self, backend: &str, ligne: &str, domaine: Option<&Path>) -> (String, Value) {
        let chemin = domaine.map(Path::to_path_buf).unwrap_or_default();
        let id = {
            let mut a = self.accords.lock().unwrap_or_else(|p| p.into_inner());
            a.suivant += 1;
            let id = format!("accord-{}-{}", crate::events::maintenant_ms(), a.suivant);
            a.demandes.insert(
                id.clone(),
                Demande { chemin: chemin.clone(), mode: "command".into(), raison: String::new(), decision: None, commande: Some((ligne.to_string(), backend.to_string())) },
            );
            id
        };
        (
            format!(
                "\n\n_La demande {id} est sur la carte d'accord, sous cet appel : l'utilisatrice accorde ou refuse. Accordée, cette même commande passe telle quelle ; attendez sa réponse, puis relancez-la._"
            ),
            json!({ "kind": "accord", "sujets": ["accords"], "id": id, "genre": "commande", "commande": ligne, "chemin": chemin }),
        )
    }

    /// **Décider une demande** : un oui ouvre le chemin pour ce mode, ou, pour une commande,
    /// l'accorde à la porte de son backend (comme le oui du superviseur d'une tâche) ; la
    /// décision va au journal des décisions (`<racine des runs>/decisions.jsonl`, une ligne)
    /// et au flux (`accords`).
    pub fn decider_accord(&self, id: &str, accorder: bool, qui: &str) -> Result<Value, String> {
        let d = {
            let a = self.accords.lock().unwrap_or_else(|p| p.into_inner());
            let d = a.demandes.get(id).ok_or_else(|| format!("aucune demande {id}"))?;
            if let Some(deja) = d.decision {
                return Err(format!("la demande {id} est déjà {}", if deja { "accordée" } else { "refusée" }));
            }
            d.clone()
        };
        // Une commande s'accorde à la porte avant que le oui ne s'écrive : un accord qui ne
        // se pose pas laisse la demande ouverte, et le dit.
        if let (true, Some((commande, backend))) = (accorder, &d.commande) {
            let domaine = Some(d.chemin.clone()).filter(|c| !c.as_os_str().is_empty());
            self.with_backend_id(backend, |b| match b.garde() {
                Some(g) => g.accorder_ligne(commande, &crate::commande::Contexte { accorde_par_l_utilisateur: true, domaine }),
                None => Err("ce backend n'a pas de porte des commandes (workspace.commands)".into()),
            })
            .and_then(|r| r)
            .map_err(|e| format!("l'accord de « {commande} » n'a pas pu se poser : {e}"))?;
        }
        {
            let mut a = self.accords.lock().unwrap_or_else(|p| p.into_inner());
            let Some(e) = a.demandes.get_mut(id) else { return Err(format!("aucune demande {id}")) };
            if e.decision.is_some() {
                return Err(format!("la demande {id} a été décidée entre-temps"));
            }
            e.decision = Some(accorder);
            if accorder && d.commande.is_none() {
                a.ouverts.push((d.chemin.clone(), d.mode.clone()));
            }
        }
        let mut ligne = json!({
            "id": id,
            "genre": if d.commande.is_some() { "commande" } else { "chemin" },
            "chemin": d.chemin,
            "mode": d.mode,
            "raison": d.raison,
            "decision": if accorder { "accordé" } else { "refusé" },
            "qui": qui,
            "quand_ms": crate::events::maintenant_ms(),
        });
        if let Some((commande, _)) = &d.commande {
            ligne["commande"] = json!(commande);
        }
        let journal = self.journal.racine().join("decisions.jsonl");
        if let Some(dir) = journal.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{} : {e}", dir.display()))?;
        }
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new().create(true).append(true).open(&journal).map_err(|e| format!("{} : {e}", journal.display()))?;
        writeln!(f, "{ligne}").map_err(|e| format!("{} : {e}", journal.display()))?;
        self.ide_flux().publier("accords", "*", json!({ "kind": "AccordDecide", "id": id, "decision": ligne["decision"], "chemin": d.chemin, "mode": d.mode, "commande": ligne["commande"] }));
        let mut rendu = ligne;
        if let Some((commande, backend)) = &d.commande {
            rendu["relance"] = json!(self.relancer_apres_la_carte(backend, commande, accorder));
        }
        Ok(rendu)
    }

    /// **La décision relance le tour d'elle-même** (c7, 9 novembre 2026 : que Lucie n'ait
    /// pas à retaper) : elle entre dans la conversation comme un message que le modèle lit,
    /// et un tour repart, dans un fil, sans que l'utilisatrice écrive. Rend vrai si le tour
    /// est parti ; un tour déjà en cours, ou un hôte qui ne se connaît pas, le laisse à
    /// l'utilisatrice, et le journal de l'hôte le dit.
    fn relancer_apres_la_carte(&self, backend: &str, commande: &str, accorde: bool) -> bool {
        let message = if accorde {
            format!("(carte d'accord) La commande `{commande}` est accordée : relance-la telle quelle.")
        } else {
            format!("(carte d'accord) La commande `{commande}` est refusée : ne la relance pas ; fais autrement, ou demande-moi.")
        };
        let Some(hote) = self.soi.get().and_then(std::sync::Weak::upgrade) else { return false };
        let backend = backend.to_string();
        std::thread::spawn(move || {
            if let Err(e) = hote.chat_turn(&backend, &message) {
                eprintln!("[démon] la relance après la carte d'accord n'est pas partie ({backend}) : {e}");
            }
        });
        true
    }

    /// L'état d'une demande (la carte relue après un rechargement).
    pub fn etat_accord(&self, id: &str) -> Option<Value> {
        let a = self.accords.lock().unwrap_or_else(|p| p.into_inner());
        a.demandes.get(id).map(|d| {
            json!({
                "id": id,
                "genre": if d.commande.is_some() { "commande" } else { "chemin" },
                "commande": d.commande.as_ref().map(|(c, _)| c),
                "chemin": d.chemin,
                "mode": d.mode,
                "raison": d.raison,
                "decision": d.decision.map(|x| if x { "accordé" } else { "refusé" }),
            })
        })
    }

    /// **Lire un fichier de l'atelier** (`read_file`) : 200 lignes au plus, la suite dite.
    pub(super) fn atelier_lire(&self, racine: Option<&Path>, path: &str, offset: usize, length: usize) -> Result<String, String> {
        let r = self.atelier_resoudre(racine, path, "read")?;
        if let Some(w) = &r.worktree {
            let rel = r.chemin.strip_prefix(&w.racine).unwrap_or(&r.chemin).to_string_lossy().to_string();
            let v = self.with_backend(&w.hosted, |b| b.call("read_file", json!({ "path": rel, "offset": offset, "limit": length }))).and_then(|x| x)?;
            let texte = v["presentation"].as_str().or_else(|| v["result"].as_str()).map(str::to_string).unwrap_or_else(|| v["result"].to_string());
            return Ok(format!("> lu dans le dépôt {} (son index le connaît)\n{texte}", w.nom()));
        }
        let contenu = std::fs::read_to_string(&r.chemin).map_err(|e| format!("{} : {e}", r.chemin.display()))?;
        let lignes: Vec<&str> = contenu.lines().collect();
        let total = lignes.len();
        let debut = offset.max(1);
        if total > 0 && debut > total {
            return Err(format!("{} a {total} lignes : offset {debut} est au-delà", r.chemin.display()));
        }
        let fin = (debut + length).saturating_sub(1).min(total);
        let mut out = format!("{} — lignes {debut}-{fin} sur {total}\n", r.chemin.display());
        for (i, l) in lignes.iter().enumerate().take(fin).skip(debut - 1) {
            out.push_str(&format!("{:05}| {l}\n", i + 1));
        }
        if fin < total {
            out.push_str(&format!("… {} ligne(s) de plus : read_file(path, offset={})\n", total - fin, fin + 1));
        }
        Ok(out)
    }

    /// Tous les worktrees attachés (les arbres inscrits).
    fn worktrees_attaches(&self) -> Vec<Worktree> {
        let Ok(racines) = self.racines_de_code() else { return Vec::new() };
        racines
            .registered_manifests()
            .into_iter()
            .filter_map(|m| {
                let v = std::fs::read_to_string(&m).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok())?;
                let root = PathBuf::from(v["workspace"]["root"].as_str()?);
                let racine = canonique(&if root.is_absolute() { root } else { m.parent()?.join(root) });
                let hosted = self.hosted(&m.canonicalize().unwrap_or(m)).ok()?;
                Some(Worktree { hosted, depot: super::repo_of(&racine), branche: super::branch_of(&racine), racine })
            })
            .collect()
    }

    /// **Lire un scope** (`read_scope`) : par `usages` (où il est défini) et `outline` (ses
    /// bornes) dans le worktree attaché ; son texte lu entre ses bornes (200 lignes au
    /// plus, la suite dite).
    pub(super) fn atelier_lire_scope(&self, racine: Option<&Path>, name: &str, path: &str) -> Result<String, String> {
        let (worktrees, rel_voulu): (Vec<Worktree>, Option<String>) = if path.trim().is_empty() {
            (self.worktrees_attaches(), None)
        } else {
            let r = self.atelier_resoudre(racine, path, "read")?;
            let Some(w) = r.worktree else {
                return Err(format!(
                    "{} est sous la racine de l'atelier, sans index : read_scope ne connaît que les worktrees attachés ; lisez-le par read_file",
                    r.chemin.display()
                ));
            };
            let rel = r.chemin.strip_prefix(&w.racine).unwrap_or(&r.chemin).to_string_lossy().to_string();
            (vec![w], Some(rel))
        };
        if worktrees.is_empty() {
            return Err("aucun worktree attaché : read_scope lit l'index d'un worktree attaché (attach_worktree, open_branch)".into());
        }
        let json_de = |v: &Value| -> Value {
            let r = &v["result"];
            match r.as_str() {
                Some(s) => serde_json::from_str(s).unwrap_or(Value::Null),
                None => r.get("result").cloned().unwrap_or_else(|| r.clone()),
            }
        };
        // Les définitions du nom, par worktree : (worktree, fichier relatif, ligne de début).
        let mut trouves: Vec<(&Worktree, String, u64)> = Vec::new();
        for w in &worktrees {
            let args = json!({ "name": name, "path": rel_voulu.clone().unwrap_or_default(), "limit": 1, "format": "json" });
            let Ok(Ok(v)) = self.with_backend(&w.hosted, |b| b.call("usages", args)) else { continue };
            for d in json_de(&v)["definitions"].as_array().into_iter().flatten() {
                if let (Some(p), Some(l)) = (d["path"].as_str(), d["line"].as_u64()) {
                    trouves.push((w, p.to_string(), l));
                }
            }
        }
        let (w, rel, ligne) = match trouves.len() {
            0 => return Err(format!("« {name} » : aucun scope de ce nom dans l'index des worktrees attachés")),
            1 => trouves.remove(0),
            n => {
                let liste: Vec<String> = trouves.iter().map(|(w, p, l)| format!("{}:{l} ({})", w.racine.join(p).display(), w.nom())).collect();
                return Err(format!("« {name} » est défini {n} fois : donnez `path` pour choisir — {}", liste.join(" ; ")));
            }
        };
        // Les bornes, par le plan du fichier.
        let v = self.with_backend(&w.hosted, |b| b.call("outline", json!({ "path": rel, "format": "json" }))).and_then(|x| x)?;
        let plan = json_de(&v);
        let scope = plan["rows"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|s| s["name"].as_str() == Some(name) && s["start_line"].as_u64() == Some(ligne))
            .cloned()
            .ok_or_else(|| format!("« {name} » : défini à {rel}:{ligne}, mais le plan du fichier ne le borne pas (l'index est-il à jour ?)"))?;
        let debut = scope["start_line"].as_u64().unwrap_or(ligne) as usize;
        let fin = scope["end_line"].as_u64().unwrap_or(ligne) as usize;
        let chemin = w.racine.join(&rel);
        let contenu = std::fs::read_to_string(&chemin).map_err(|e| format!("{} : {e}", chemin.display()))?;
        let lignes: Vec<&str> = contenu.lines().collect();
        let fin_lue = fin.min(debut + crate::dataflow::fichier_nodes::LIGNES_MAX - 1).min(lignes.len());
        let mut out = format!(
            "{} ({}) — {} lignes {debut}-{fin}, dépôt {}\n",
            name,
            scope["scope_type"].as_str().unwrap_or("scope"),
            chemin.display(),
            w.nom()
        );
        for (i, l) in lignes.iter().enumerate().take(fin_lue).skip(debut.saturating_sub(1)) {
            out.push_str(&format!("{:05}| {l}\n", i + 1));
        }
        if fin_lue < fin {
            out.push_str(&format!("… {} ligne(s) de plus : read_file(path, offset={})\n", fin - fin_lue, fin_lue + 1));
        }
        Ok(out)
    }

    /// **Écrire un fichier** (`replace_text`, `replace_lines`, `insert_lines`) : le texte
    /// neuf calculé, écrit (par le dépôt s'il est dans un worktree attaché, qui le
    /// réindexe), et le diff unifié rendu.
    pub(super) fn atelier_ecrire(&self, racine: Option<&Path>, path: &str, edition: Edition) -> Result<String, String> {
        let r = self.atelier_resoudre(racine, path, "write")?;
        let avant = match std::fs::read_to_string(&r.chemin) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound && matches!(edition, Edition::Insertion { avant: 1, .. }) => String::new(),
            Err(e) => return Err(format!("{} : {e}", r.chemin.display())),
        };
        let apres = texte_apres(&avant, &edition).map_err(|e| format!("{} : {e}", r.chemin.display()))?;
        let mut dit = Vec::new();
        if let Some(w) = &r.worktree {
            if !self.atelier_peut_ecrire(w, &r.chemin) {
                return Err(format!(
                    "{} est dans le dépôt {}, que l'atelier n'a pas ouvert lui-même (l'arbre que tient l'utilisatrice) : demandez son accord par ask_permission (path, mode « write », la raison), puis réessayez ; ou ouvrez une branche à vous (open_branch) et écrivez-y",
                    r.chemin.display(),
                    w.nom()
                ));
            }
        }
        match &r.worktree {
            Some(w) => {
                let rel = r.chemin.strip_prefix(&w.racine).unwrap_or(&r.chemin).to_string_lossy().to_string();
                let v = self.with_backend(&w.hosted, |b| b.call_humain("edit_file", json!({ "path": rel, "content": apres }))).and_then(|x| x)?;
                let rendu = v["presentation"].as_str().or_else(|| v["result"].as_str()).unwrap_or_default().to_string();
                // Ce que l'édition du dépôt dit de son index (le rendu de `EditResult`).
                let index = if let Some(reste) = rendu.split("Index updated: ").nth(1) {
                    let n = |mot: &str| reste.split(mot).next().and_then(|x| x.rsplit(|c: char| !c.is_ascii_digit()).find(|m| !m.is_empty())).unwrap_or("0").to_string();
                    format!("réindexé ({} scope(s) écrits, {} retirés)", n(" scopes upserted"), n(" removed"))
                } else if rendu.contains("index will pick it up") || rendu.contains("indexing in progress will read it") {
                    "un index tourne : il prendra ce fichier".to_string()
                } else if let Some(e) = rendu.split("the index did not follow (").nth(1) {
                    format!("sa réindexation n'a pas suivi ({})", e.split(") —").next().unwrap_or(e))
                } else {
                    "sans index".to_string()
                };
                dit.push(format!("écrit dans le dépôt {} ; {index}", w.nom()));
            }
            None => {
                ecrire_atomique(&r.chemin, &apres)?;
                dit.push("écrit sous la racine de l'atelier (sans index)".to_string());
            }
        }
        Ok(format!("{}\n{}", dit.join("\n"), diff_unifie(&r.chemin.display().to_string(), &avant, &apres)))
    }
}

/// Les lignes d'un texte, chacune avec sa fin de ligne.
fn lignes(t: &str) -> Vec<&str> {
    t.split_inclusive('\n').collect()
}

/// Un texte écrit en lignes : sa dernière ligne finie par un retour.
fn en_lignes(t: &str) -> String {
    if t.is_empty() || t.ends_with('\n') { t.to_string() } else { format!("{t}\n") }
}

/// **Le texte après une édition**, ou le refus qui dit pourquoi.
pub(super) fn texte_apres(avant: &str, edition: &Edition) -> Result<String, String> {
    match edition {
        Edition::Texte { ancien, nouveau } => match avant.matches(ancien.as_str()).count() {
            0 => Err("l'ancien texte n'y est pas : relisez le fichier (read_file) et recopiez-le exactement".into()),
            1 => Ok(avant.replacen(ancien.as_str(), nouveau, 1)),
            n => Err(format!("l'ancien texte y est {n} occurrences : donnez plus de contexte pour qu'il soit unique")),
        },
        Edition::Lignes { de, a, texte } => {
            let l = lignes(avant);
            if *de < 1 || *a < *de || *a > l.len() {
                return Err(format!("le fichier a {} lignes : les lignes {de}..{a} n'y sont pas", l.len()));
            }
            Ok(format!("{}{}{}", l[..de - 1].concat(), en_lignes(texte), l[*a..].concat()))
        }
        Edition::Insertion { avant: at, texte } => {
            let l = lignes(avant);
            if *at < 1 || *at > l.len() + 1 {
                return Err(format!("le fichier a {} lignes : on insère avant une ligne de 1 à {}", l.len(), l.len() + 1));
            }
            // Un fichier sans retour final : la ligne d'avant reçoit le sien avant l'ajout.
            let tete = l[..at - 1].concat();
            let tete = if *at - 1 == l.len() { en_lignes(&tete) } else { tete };
            Ok(format!("{tete}{}{}", en_lignes(texte), l[at - 1..].concat()))
        }
    }
}

/// Écrire un fichier d'un coup (un fichier voisin, puis renommé).
fn ecrire_atomique(chemin: &Path, texte: &str) -> Result<(), String> {
    if let Some(d) = chemin.parent() {
        std::fs::create_dir_all(d).map_err(|e| format!("{} : {e}", d.display()))?;
    }
    let tmp = chemin.with_extension(format!("{}.atelier-tmp", chemin.extension().and_then(|e| e.to_str()).unwrap_or("")));
    std::fs::write(&tmp, texte).map_err(|e| format!("{} : {e}", tmp.display()))?;
    std::fs::rename(&tmp, chemin).map_err(|e| format!("{} : {e}", chemin.display()))
}

/// **Le diff unifié d'une édition** : une région changée (les trois écritures n'en
/// changent qu'une), entre le préfixe et le suffixe communs, avec trois lignes de
/// contexte.
pub(super) fn diff_unifie(nom: &str, avant: &str, apres: &str) -> String {
    const CONTEXTE: usize = 3;
    let a: Vec<&str> = avant.lines().collect();
    let b: Vec<&str> = apres.lines().collect();
    let prefixe = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let max_suffixe = a.len().min(b.len()) - prefixe;
    let suffixe = a.iter().rev().zip(b.iter().rev()).take(max_suffixe).take_while(|(x, y)| x == y).count();
    if prefixe == a.len() && prefixe == b.len() {
        return format!("--- a/{}\n+++ b/{}\n(aucun changement)\n", nom.trim_start_matches('/'), nom.trim_start_matches('/'));
    }
    let debut = prefixe.saturating_sub(CONTEXTE);
    let fin_a = (a.len() - suffixe + CONTEXTE).min(a.len());
    let fin_b = (b.len() - suffixe + CONTEXTE).min(b.len());
    let nom = nom.trim_start_matches('/');
    let mut out = format!("--- a/{nom}\n+++ b/{nom}\n@@ -{},{} +{},{} @@\n", debut + 1, fin_a - debut, debut + 1, fin_b - debut);
    for l in &a[debut..prefixe] {
        out.push_str(&format!(" {l}\n"));
    }
    for l in &a[prefixe..a.len() - suffixe] {
        out.push_str(&format!("-{l}\n"));
    }
    for l in &b[prefixe..b.len() - suffixe] {
        out.push_str(&format!("+{l}\n"));
    }
    for l in &a[a.len() - suffixe..fin_a] {
        out.push_str(&format!(" {l}\n"));
    }
    out
}

#[cfg(all(test, feature = "rag3db-native"))]
pub(super) mod tests {
    use super::super::chat::ChatModel;
    use super::super::chat_qui_code::hote;
    use crate::llm::{Llm, MockLlm};
    use serde_json::{json, Value};
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    pub(crate) fn git(dir: &Path, args: &[&str]) {
        assert!(std::process::Command::new("git").args(args).current_dir(dir).status().unwrap().success(), "git {args:?}");
    }

    /// L'atelier d'un témoin : le gabarit livré copié, sa racine posée sur `racine`, un hôte
    /// qui se connaît (les outils passent par lui), ses racines de code sous `d`.
    pub(crate) struct Atelier {
        pub host: Arc<super::super::DaemonHost>,
        pub h: Arc<super::super::Hosted>,
        pub racine: PathBuf,
    }

    impl Atelier {
        pub fn appeler(&self, outil: &str, args: Value) -> Result<Value, String> {
            self.host.with_backend(&self.h, |b| b.call(outil, args)).and_then(|r| r)
        }
        /// Le texte que lit le modèle : `result` s'il est une chaîne, sinon le JSON.
        pub fn texte(v: &Value) -> String {
            v["result"].as_str().map(str::to_string).unwrap_or_else(|| v["result"].to_string())
        }
    }

    pub(crate) fn atelier(d: &Path) -> Atelier {
        let model: ChatModel = Arc::new(|_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> { Ok(Arc::new(MockLlm::new(""))) });
        atelier_avec(d, model)
    }

    pub(crate) fn atelier_avec(d: &Path, model: ChatModel) -> Atelier {
        let racine = d.join("rw");
        std::fs::create_dir_all(&racine).unwrap();
        let gabarits = d.join("atelier/templates");
        std::fs::create_dir_all(gabarits.parent().unwrap()).unwrap();
        crate::code_tree::copy(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates"), &gabarits, false).unwrap();
        let manifeste = gabarits.join("backends/atelier/backend.json");
        let mut m: Value = serde_json::from_slice(&std::fs::read(&manifeste).unwrap()).unwrap();
        m["workspace"]["root"] = json!(racine.to_string_lossy());
        std::fs::write(&manifeste, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
        let racines = crate::code_tree::Roots { config: d.join("config"), data: d.join("data") };
        let host = Arc::new(hote(d, model).with_racines_de_code(racines).with_runs_root(d.join("runs")));
        host.se_connaitre();
        let h = host.hosted(&manifeste).unwrap();
        host.with_backend(&h, |_| ()).expect("l'atelier s'ouvre");
        Atelier { host, h, racine }
    }

    /// Attendre que plus aucun index ne tourne sur le worktree `depot` (attaché) : un
    /// témoin qui finit pendant un index laisse le moteur couper sous lui (SIGSEGV à la
    /// sortie du processus).
    pub(crate) fn attendre_l_index(a: &Atelier, depot: &Path) {
        let racines = a.host.racines_de_code().unwrap();
        let nom = crate::code_tree::name_of_tree(&depot.canonicalize().unwrap()).unwrap();
        let m = racines.manifest(&nom).canonicalize().unwrap();
        let h = a.host.hosted(&m).unwrap();
        let debut = std::time::Instant::now();
        while a.host.with_backend(&h, |b| crate::dataflow::index_nodes::indexings_running(&b.catalog)).unwrap() > 0 {
            assert!(debut.elapsed() < std::time::Duration::from_secs(120), "l'index du worktree n'a pas fini en 120 s");
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }

    /// Un petit dépôt git sous `dans`, sur sa branche `essai`.
    pub(crate) fn petit_depot(dans: &Path) -> PathBuf {
        let depot = dans.join("rag3db-essai");
        std::fs::create_dir_all(depot.join("src")).unwrap();
        std::fs::write(depot.join("src/lib.rs"), "pub fn un() -> u32 {\n    1\n}\n\npub fn deux() -> u32 {\n    2\n}\n").unwrap();
        for args in [&["init", "-q", "-b", "essai"][..], &["config", "user.name", "Personne Test"], &["config", "user.email", "personne@example.org"], &["add", "-A"], &["commit", "-q", "-m", "le dépôt"]] {
            git(&depot, args);
        }
        depot
    }

    /// **`read_file` dans l'atelier** : jamais plus de 200 lignes d'un coup (Lucie), la
    /// suite dite ; un fichier sous la racine se lit sans index ; un fichier d'un worktree
    /// attaché se lit par le backend de ce worktree.
    #[test]
    fn the_workshop_reads_a_file_two_hundred_lines_at_a_time() {
        let d = tempfile::tempdir().unwrap();
        let a = atelier(d.path());
        let long: String = (1..=450).map(|i| format!("ligne {i}\n")).collect();
        std::fs::write(a.racine.join("notes.txt"), long).unwrap();

        let r = a.appeler("read_file", json!({ "path": "notes.txt", "length": 500 })).expect("read_file dans l'atelier");
        let t = Atelier::texte(&r);
        assert!(t.contains("ligne 200") && !t.contains("ligne 201\n"), "200 lignes au plus : {t}");
        assert!(t.contains("201"), "la suite est dite (offset 201) : {t}");
        let r = a.appeler("read_file", json!({ "path": "notes.txt", "offset": 401 })).unwrap();
        let t = Atelier::texte(&r);
        assert!(t.contains("ligne 401") && t.contains("ligne 450"), "{t}");

        // Un worktree attaché sous la racine : lu par son backend de code.
        let depot = petit_depot(&a.racine);
        let dit = a.appeler("attach_worktree", json!({ "path": depot })).unwrap();
        assert!(Atelier::texte(&dit).contains("Worktree attaché"), "{dit}");
        let r = a.appeler("read_file", json!({ "path": depot.join("src/lib.rs") })).unwrap();
        let t = Atelier::texte(&r);
        assert!(t.contains("pub fn un()") && t.contains("pub fn deux()"), "{t}");
        assert!(t.contains("rag3db-essai"), "la lecture dit le dépôt qui l'a servie : {t}");
        attendre_l_index(&a, &depot);
    }

    /// **`replace_text`** : un ancien texte unique, sinon un refus qui dit le nombre
    /// d'occurrences ; le diff unifié rendu ; dans un worktree attaché, l'index suit (le
    /// nom neuf se trouve par `usages`).
    #[test]
    fn the_workshop_replaces_a_unique_text_and_says_the_diff() {
        let d = tempfile::tempdir().unwrap();
        let a = atelier(d.path());
        std::fs::write(a.racine.join("conf.txt"), "a = 1\nb = 1\nc = 2\n").unwrap();
        let refus = a.appeler("replace_text", json!({ "path": "conf.txt", "old": "= 1", "new": "= 9" })).unwrap_err();
        assert!(refus.contains("2 occurrences"), "le refus dit le nombre d'occurrences : {refus}");
        let r = a.appeler("replace_text", json!({ "path": "conf.txt", "old": "c = 2", "new": "c = 3" })).expect("replace_text");
        let t = Atelier::texte(&r);
        assert!(t.contains("--- ") && t.contains("+++ ") && t.contains("@@") && t.contains("-c = 2") && t.contains("+c = 3"), "le diff unifié : {t}");
        assert_eq!(std::fs::read_to_string(a.racine.join("conf.txt")).unwrap(), "a = 1\nb = 1\nc = 3\n");

        let depot = petit_depot(&a.racine);
        a.appeler("attach_worktree", json!({ "path": depot })).unwrap();
        attendre_l_index(&a, &depot);
        // Un worktree que l'atelier n'a pas ouvert lui-même (attaché d'un dossier existant :
        // l'arbre que tient l'utilisatrice) : l'écrire demande son accord (décision de c7,
        // 9 novembre 2026), et rien n'est écrit.
        let refus = a.appeler("replace_text", json!({ "path": depot.join("src/lib.rs"), "old": "pub fn deux()", "new": "pub fn trois()" })).unwrap_err();
        assert!(refus.contains("ask_permission") && refus.contains("rag3db-essai"), "un arbre tenu par l'utilisatrice demande son accord : {refus}");
        assert!(std::fs::read_to_string(depot.join("src/lib.rs")).unwrap().contains("pub fn deux()"), "rien n'est écrit sans accord");
        attendre_l_index(&a, &depot);
    }

    /// **`replace_lines` et `insert_lines`** (par le graphe ; l'empreinte de lecture est
    /// tenue par le chat, son témoin est à part) : les lignes remplacées ou insérées, le
    /// diff unifié, les bornes refusées en le disant.
    #[test]
    fn the_workshop_replaces_and_inserts_lines_with_a_diff() {
        let d = tempfile::tempdir().unwrap();
        let a = atelier(d.path());
        std::fs::write(a.racine.join("liste.txt"), "un\ndeux\ntrois\nquatre\ncinq\n").unwrap();
        let r = a.appeler("replace_lines", json!({ "path": "liste.txt", "from": 2, "to": 3, "text": "DEUX\nTROIS" })).expect("replace_lines");
        let t = Atelier::texte(&r);
        assert!(t.contains("-deux") && t.contains("-trois") && t.contains("+DEUX") && t.contains("+TROIS"), "{t}");
        a.appeler("insert_lines", json!({ "path": "liste.txt", "at": 1, "text": "zéro" })).expect("insert_lines au début");
        let r = a.appeler("insert_lines", json!({ "path": "liste.txt", "at": 7, "text": "six" })).expect("insert_lines à la fin");
        assert!(Atelier::texte(&r).contains("+six"), "{r}");
        assert_eq!(std::fs::read_to_string(a.racine.join("liste.txt")).unwrap(), "zéro\nun\nDEUX\nTROIS\nquatre\ncinq\nsix\n");
        let refus = a.appeler("insert_lines", json!({ "path": "liste.txt", "at": 10, "text": "x" })).unwrap_err();
        assert!(refus.contains("7 lignes"), "{refus}");
        let refus = a.appeler("replace_lines", json!({ "path": "liste.txt", "from": 6, "to": 9, "text": "x" })).unwrap_err();
        assert!(refus.contains("7 lignes"), "{refus}");
    }

    /// **`read_scope`** : un scope de l'index d'un worktree attaché, par son nom (et son
    /// fichier pour lever une ambiguïté) : son texte et ses bornes ; un nom inconnu, ou
    /// un fichier sous la racine sans index, refusés en disant pourquoi.
    #[test]
    fn the_workshop_reads_a_scope_from_the_index_of_an_attached_worktree() {
        let d = tempfile::tempdir().unwrap();
        let a = atelier(d.path());
        let depot = petit_depot(&a.racine);
        a.appeler("attach_worktree", json!({ "path": depot })).unwrap();
        attendre_l_index(&a, &depot);
        let r = a.appeler("read_scope", json!({ "name": "deux" })).expect("read_scope par le nom");
        let t = Atelier::texte(&r);
        assert!(t.contains("pub fn deux()") && t.contains("2") && !t.contains("pub fn un()"), "le texte du scope seul : {t}");
        assert!(t.contains("lignes 5-7") && t.contains("src/lib.rs") && t.contains("rag3db-essai"), "ses bornes, son fichier, son dépôt : {t}");
        let r = a.appeler("read_scope", json!({ "name": "un", "path": depot.join("src/lib.rs") })).expect("read_scope par le nom et le fichier");
        assert!(Atelier::texte(&r).contains("lignes 1-3"), "{r}");
        let refus = a.appeler("read_scope", json!({ "name": "inconnu_xyz" })).unwrap_err();
        assert!(refus.contains("inconnu_xyz") && refus.contains("aucun"), "{refus}");
        std::fs::write(a.racine.join("notes.txt"), "rien\n").unwrap();
        let refus = a.appeler("read_scope", json!({ "name": "rien", "path": "notes.txt" })).unwrap_err();
        assert!(refus.contains("sans index") && refus.contains("read_file"), "sous la racine, read_scope dit pourquoi : {refus}");
        attendre_l_index(&a, &depot);
    }

    /// **L'empreinte de lecture** (Lucie : replace_lines et insert_lines exigent d'avoir lu
    /// ces lignes dans ce tour ou le précédent) : par de vrais tours de chat dans l'atelier,
    /// un faux modèle qui suit un script par message.
    #[test]
    fn the_workshop_chat_writes_lines_only_after_reading_them_recently() {
        use crate::llm::{CallbackLlm, Turn};
        use std::sync::Mutex;
        // Le script : pour chaque message, les appels dans l'ordre, puis « fait ».
        let script: Vec<(&str, Vec<(&str, Value)>)> = vec![
            ("t1", vec![
                ("replace_lines", json!({ "path": "liste.txt", "from": 2, "to": 2, "text": "DEUX" })),
                ("read_file", json!({ "path": "liste.txt" })),
                ("replace_lines", json!({ "path": "liste.txt", "from": 2, "to": 2, "text": "DEUX" })),
            ]),
            ("t2", vec![("insert_lines", json!({ "path": "liste.txt", "at": 2, "text": "un et demi" }))]),
            ("t3", vec![("read_file", json!({ "path": "liste.txt", "offset": 1, "length": 3 }))]),
            ("t4", vec![("insert_lines", json!({ "path": "liste.txt", "at": 2, "text": "un et demi" }))]),
            ("t5", vec![]),
            ("t6", vec![("replace_lines", json!({ "path": "liste.txt", "from": 1, "to": 1, "text": "UN" }))]),
        ];
        let resultats: Arc<Mutex<Vec<(String, String, String)>>> = Arc::new(Mutex::new(Vec::new()));
        let vus = resultats.clone();
        let script = Arc::new(script.into_iter().map(|(m, c)| (m.to_string(), c.into_iter().map(|(n, a)| (n.to_string(), a)).collect::<Vec<_>>())).collect::<Vec<_>>());
        let model: ChatModel = Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let (script, vus) = (script.clone(), vus.clone());
            Ok(Arc::new(CallbackLlm::new("faux-atelier", 8192, move |turns: &[Turn], opts, sink| {
                let depuis = turns.iter().rposition(|t| t.role == "user").unwrap_or(0);
                let message = turns[depuis].content.clone();
                let faits: Vec<&Turn> = turns[depuis..].iter().filter(|t| t.role == "tool").collect();
                if let Some(dernier) = faits.last() {
                    vus.lock().unwrap().push((message.clone(), dernier.tool_name.clone().unwrap_or_default(), dernier.content.clone()));
                }
                let appels = script.iter().find(|(m, _)| *m == message).map(|(_, c)| c.clone()).unwrap_or_default();
                match appels.get(faits.len()) {
                    Some((nom, args)) => MockLlm::new("").with_tool_calls(vec![(nom.clone(), args.to_string())]).generate(turns, opts, sink),
                    None => MockLlm::new("fait").generate(turns, opts, sink),
                }
            })))
        });
        let d = tempfile::tempdir().unwrap();
        let a = atelier_avec(d.path(), model);
        std::fs::write(a.racine.join("liste.txt"), "un\ndeux\ntrois\nquatre\ncinq\n").unwrap();
        for m in ["t1", "t2", "t3", "t4", "t5", "t6"] {
            a.host.chat_turn(&a.h.id, m).unwrap_or_else(|e| panic!("le tour {m} : {e}"));
        }
        let r = resultats.lock().unwrap().clone();
        let de = |m: &str, outil: &str| -> Vec<String> { r.iter().filter(|(mm, o, _)| mm == m && o == outil).map(|(_, _, c)| c.clone()).collect() };
        let t1 = de("t1", "replace_lines");
        assert_eq!(t1.len(), 2, "{r:#?}");
        assert!(t1[0].contains("relis d'abord") && t1[0].contains("2-2"), "sans lecture, refusé : {}", t1[0]);
        assert!(t1[1].contains("+DEUX"), "après lecture, écrit : {}", t1[1]);
        assert!(de("t2", "insert_lines")[0].contains("relis d'abord"), "l'écriture du tour 1 a déplacé les lignes : {r:#?}");
        assert!(de("t4", "insert_lines")[0].contains("+un et demi"), "lu au tour précédent : écrit : {r:#?}");
        assert!(de("t6", "replace_lines")[0].contains("relis d'abord"), "lu deux tours plus tôt, puis écrit : refusé : {r:#?}");
        assert_eq!(std::fs::read_to_string(a.racine.join("liste.txt")).unwrap(), "un\nun et demi\nDEUX\ntrois\nquatre\ncinq\n");
    }

    /// **Un worktree né de l'atelier s'écrit sans question** (décision de c7, 9 novembre
    /// 2026) : open_branch crée la branche dans un worktree frère et le marque
    /// (`opened_by = "atelier"`) ; l'atelier l'écrit au nom de l'utilisatrice, son index
    /// suit. Le dépôt d'où elle part, que tient l'utilisatrice, demande toujours son accord.
    #[test]
    fn a_worktree_opened_by_the_workshop_is_written_without_asking() {
        let d = tempfile::tempdir().unwrap();
        let a = atelier(d.path());
        let depot = petit_depot(&a.racine);
        let dit = a.appeler("open_branch", json!({ "path": depot, "branch": "atelier/essai" })).expect("open_branch");
        let dit = Atelier::texte(&dit);
        assert!(dit.contains("atelier/essai"), "{dit}");
        let frere = a.racine.join("rag3db-essai-atelier-essai");
        assert!(frere.join("src/lib.rs").is_file(), "le worktree frère : {dit}");
        attendre_l_index(&a, &frere);
        let racines = a.host.racines_de_code().unwrap();
        let nom = crate::code_tree::name_of_tree(&frere.canonicalize().unwrap()).unwrap();
        let m = racines.manifest(&nom);
        assert!(racines.marque_de_l_atelier(&nom).is_file(), "la marque à côté du manifeste inscrit : {dit}");

        let r = a.appeler("replace_text", json!({ "path": frere.join("src/lib.rs"), "old": "pub fn deux()", "new": "pub fn trois()" })).expect("écrire le worktree né de l'atelier");
        let t = Atelier::texte(&r);
        assert!(t.contains("-pub fn deux()") && t.contains("+pub fn trois()") && t.contains("réindexé"), "{t}");
        let h = a.host.hosted(&m.canonicalize().unwrap()).unwrap();
        let u = a.host.with_backend(&h, |b| b.call("usages", json!({ "name": "trois", "format": "json" }))).unwrap().unwrap();
        assert!(u["result"]["result"]["definitions"].as_array().is_some_and(|d| !d.is_empty()), "trois est défini dans l'index : {u}");
        // Le dépôt d'origine n'est pas né de l'atelier : il n'est pas attaché ici, il est
        // sous la racine et s'écrit sans index ; attaché, il demanderait l'accord (témoin
        // the_workshop_replaces_a_unique_text_and_says_the_diff).
        attendre_l_index(&a, &frere);
    }

    /// **Une commande en attente se décide sur la carte d'accord** (lot 7 de la page
    /// `09-novembre-2026-atelier-lire-ecrire/01-conception.md`, c7, 9 novembre 2026) : dans
    /// le chat de l'atelier, une commande que la porte met en attente (`mkdir` écrit) rend la
    /// carte d'accord sous l'appel. La décision relance le tour d'elle-même : un oui l'accorde
    /// à la porte de l'atelier et le modèle, qui le lit, relance la même commande, qui passe ;
    /// un non le lui dit, et rien ne passe. Chaque décision va au journal des décisions.
    #[test]
    fn a_command_on_hold_in_the_workshop_chat_is_decided_on_the_permission_card() {
        use crate::llm::{CallbackLlm, Turn};
        let model: ChatModel = Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            Ok(Arc::new(CallbackLlm::new("faux-atelier", 8192, move |turns: &[Turn], opts, sink| {
                let depuis = turns.iter().rposition(|t| t.role == "user").unwrap_or(0);
                let message = turns[depuis].content.clone();
                let fait = turns[depuis..].iter().any(|t| t.role == "tool");
                // Un ordre (`mkdir …`), ou l'accord dit par la carte : la commande entre
                // accents graves, relancée telle quelle.
                let commande = match message.strip_prefix("lance ") {
                    Some(c) => Some(c.to_string()),
                    None if message.contains("est accordée : relance-la") => message.split('`').nth(1).map(str::to_string),
                    None => None,
                };
                match commande.filter(|_| !fait) {
                    Some(c) => MockLlm::new("").with_tool_calls(vec![("run_command".to_string(), json!({ "command": c }).to_string())]).generate(turns, opts, sink),
                    None => MockLlm::new("fait").generate(turns, opts, sink),
                }
            })))
        });
        let d = tempfile::tempdir().unwrap();
        let a = atelier_avec(d.path(), model);
        let etat = |a: &Atelier| a.host.chat_state(&a.h.id).unwrap();
        // Le dernier résultat de run_command de la conversation.
        let dernier = |a: &Atelier| -> Value { etat(a)["turns"].as_array().unwrap().iter().filter(|t| t["role"] == "tool").last().cloned().unwrap_or(Value::Null) };
        // Le tour relancé par la carte : `n` messages de l'utilisatrice, et plus aucun tour en vol.
        let attendre_le_tour = |a: &Atelier, n: usize| {
            let debut = std::time::Instant::now();
            loop {
                let e = etat(a);
                let messages = e["turns"].as_array().unwrap().iter().filter(|t| t["role"] == "user").count();
                if messages >= n && e["run"].is_null() {
                    return e;
                }
                assert!(debut.elapsed() < std::time::Duration::from_secs(30), "la carte n'a pas relancé de tour : {e:#}");
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        };

        a.host.chat_turn(&a.h.id, "lance mkdir faits").expect("le premier tour");
        let t = dernier(&a);
        assert!(t["content"].as_str().unwrap_or_default().contains("Commande en attente"), "mkdir attend l'accord : {t:#}");
        assert!(!a.racine.join("faits").exists(), "rien ne s'est exécuté");
        let carte = t["ui"][0].clone();
        assert_eq!(carte["kind"], "accord", "la carte d'accord sous l'appel : {t:#}");
        assert_eq!(carte["genre"], "commande", "{carte:#}");
        assert_eq!(carte["commande"], "mkdir faits", "{carte:#}");
        let id = carte["id"].as_str().expect("l'id de la demande").to_string();
        assert_eq!(a.host.etat_accord(&id).unwrap()["commande"], "mkdir faits");

        let rendu = a.host.decider_accord(&id, true, "le témoin").expect("accorder");
        assert_eq!((rendu["genre"].as_str(), rendu["relance"].as_bool()), (Some("commande"), Some(true)), "{rendu}");
        let e = attendre_le_tour(&a, 2);
        let dit = e["turns"].as_array().unwrap().iter().filter(|t| t["role"] == "user").last().unwrap()["content"].clone();
        assert!(dit.as_str().is_some_and(|d| d.contains("`mkdir faits` est accordée")), "le modèle lit l'accord : {dit}");
        assert!(a.racine.join("faits").is_dir(), "accordée et relancée, la commande passe : {:#}", dernier(&a));

        a.host.chat_turn(&a.h.id, "lance mkdir autres").expect("le troisième tour");
        let id = dernier(&a)["ui"][0]["id"].as_str().expect("une seconde carte").to_string();
        a.host.decider_accord(&id, false, "le témoin").expect("refuser");
        let e = attendre_le_tour(&a, 4);
        let dit = e["turns"].as_array().unwrap().iter().filter(|t| t["role"] == "user").last().unwrap()["content"].clone();
        assert!(dit.as_str().is_some_and(|d| d.contains("`mkdir autres` est refusée")), "le modèle lit le refus : {dit}");
        assert!(!a.racine.join("autres").exists(), "refusée, rien ne passe");

        let journal = std::fs::read_to_string(d.path().join("runs/decisions.jsonl")).expect("le journal des décisions");
        let lignes: Vec<Value> = journal.lines().map(|l| serde_json::from_str(l).unwrap()).collect();
        assert_eq!(lignes.len(), 2, "{journal}");
        assert_eq!((lignes[0]["commande"].as_str(), lignes[0]["decision"].as_str()), (Some("mkdir faits"), Some("accordé")), "{journal}");
        assert_eq!((lignes[1]["commande"].as_str(), lignes[1]["decision"].as_str()), (Some("mkdir autres"), Some("refusé")), "{journal}");
    }

    /// **`ask_permission`** (décision de c7, 9 novembre 2026) : hors de la racine, ou sur un
    /// arbre que tient l'utilisatrice, le modèle demande l'accord ; la demande rend une carte
    /// d'accord (`ui`), l'utilisatrice accorde ou refuse ; un oui ouvre ce chemin pour ce
    /// mode, un non le garde fermé ; chaque décision va au journal des décisions (une ligne,
    /// à côté des runs), avec le chemin, le mode, la raison du modèle, qui, quand.
    #[test]
    fn a_permission_asked_by_the_workshop_opens_the_path_only_when_granted() {
        let d = tempfile::tempdir().unwrap();
        let a = atelier(d.path());
        let dehors = d.path().join("dehors");
        std::fs::create_dir_all(&dehors).unwrap();
        std::fs::write(dehors.join("a.txt"), "hors de la racine\n").unwrap();
        std::fs::write(dehors.join("b.txt"), "aussi\n").unwrap();
        let refus = a.appeler("read_file", json!({ "path": dehors.join("a.txt") })).unwrap_err();
        assert!(refus.contains("hors de la racine") && refus.contains("ask_permission"), "{refus}");

        let demande = a.appeler("ask_permission", json!({ "path": dehors.join("a.txt"), "mode": "read", "reason": "lire la note posée hors de l'atelier" })).expect("ask_permission");
        let carte = demande["ui"][0].clone();
        assert_eq!(carte["kind"], "accord", "la carte d'accord sous l'appel : {demande}");
        let id = carte["id"].as_str().expect("l'id de la demande").to_string();
        assert!(Atelier::texte(&demande).contains(&id), "le modèle a l'id : {demande}");
        // Rien n'est ouvert avant la décision.
        assert!(a.appeler("read_file", json!({ "path": dehors.join("a.txt") })).is_err());
        a.host.decider_accord(&id, true, "le témoin").expect("accorder");
        let r = a.appeler("read_file", json!({ "path": dehors.join("a.txt") })).expect("lu après l'accord");
        assert!(Atelier::texte(&r).contains("hors de la racine"), "{r}");
        // Un accord de lecture n'ouvre pas l'écriture.
        assert!(a.appeler("replace_text", json!({ "path": dehors.join("a.txt"), "old": "hors", "new": "dans" })).is_err());

        let demande = a.appeler("ask_permission", json!({ "path": dehors.join("b.txt"), "mode": "write", "reason": "corriger b" })).unwrap();
        let id2 = demande["ui"][0]["id"].as_str().unwrap().to_string();
        a.host.decider_accord(&id2, false, "le témoin").expect("refuser");
        assert!(a.appeler("replace_text", json!({ "path": dehors.join("b.txt"), "old": "aussi", "new": "non" })).is_err(), "un refus garde fermé");
        assert_eq!(std::fs::read_to_string(dehors.join("b.txt")).unwrap(), "aussi\n");

        let journal = std::fs::read_to_string(d.path().join("runs/decisions.jsonl")).expect("le journal des décisions");
        let lignes: Vec<Value> = journal.lines().map(|l| serde_json::from_str(l).unwrap()).collect();
        assert_eq!(lignes.len(), 2, "{journal}");
        assert_eq!((lignes[0]["decision"].clone(), lignes[0]["mode"].clone()), (json!("accordé"), json!("read")), "{journal}");
        assert_eq!(lignes[0]["raison"], "lire la note posée hors de l'atelier");
        assert!(lignes[0]["chemin"].as_str().is_some_and(|c| c.ends_with("dehors/a.txt")) && lignes[0]["qui"] == "le témoin" && lignes[0]["quand_ms"].is_u64(), "{journal}");
        assert_eq!(lignes[1]["decision"], "refusé");
    }

    /// **La carte d'accord glisse sous l'appel** (décision de c7, 9 novembre 2026) : dans le
    /// chat de l'atelier, le modèle demande à lire hors de la racine ; la carte paraît sous
    /// l'appel (le chemin, le mode, la raison), « accorder » la décide sans recharger la page,
    /// la décision va au journal ; au tour suivant, la lecture passe.
    #[test]
    #[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
    fn the_permission_card_slides_under_the_call_and_grants_the_read() {
        use crate::daemon_host::navigateur::{navigateur_ou_saut, Navigateur};
        use crate::llm::{CallbackLlm, Turn};
        const TEMOIN: &str = "the_permission_card_slides_under_the_call_and_grants_the_read";
        let Some(binaire) = navigateur_ou_saut(TEMOIN) else {
            return;
        };
        let d = tempfile::tempdir().unwrap();
        let racine = d.path().join("rw");
        std::fs::create_dir_all(&racine).unwrap();
        let dehors = d.path().join("dehors");
        std::fs::create_dir_all(&dehors).unwrap();
        std::fs::write(dehors.join("note.txt"), "la note du dehors\n").unwrap();
        let gabarits = d.path().join("atelier/templates");
        std::fs::create_dir_all(gabarits.parent().unwrap()).unwrap();
        crate::code_tree::copy(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates"), &gabarits, false).unwrap();
        let manifeste = gabarits.join("backends/atelier/backend.json");
        let mut m: Value = serde_json::from_slice(&std::fs::read(&manifeste).unwrap()).unwrap();
        m["workspace"]["root"] = json!(racine.to_string_lossy());
        std::fs::write(&manifeste, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
        let note = dehors.join("note.txt").display().to_string();
        let vus: Arc<std::sync::Mutex<Vec<String>>> = Arc::new(std::sync::Mutex::new(Vec::new()));
        let releve = vus.clone();
        let model: ChatModel = Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let (note, vus) = (note.clone(), releve.clone());
            Ok(Arc::new(CallbackLlm::new("faux-atelier", 8192, move |turns: &[Turn], opts, sink| {
                let depuis = turns.iter().rposition(|t| t.role == "user").unwrap_or(0);
                let message = turns[depuis].content.clone();
                let faits: Vec<&Turn> = turns[depuis..].iter().filter(|t| t.role == "tool").collect();
                if let Some(t) = faits.last() {
                    vus.lock().unwrap().push(t.content.clone());
                }
                match (message.as_str(), faits.len()) {
                    ("lis la note", 0) => MockLlm::new("").with_tool_calls(vec![("ask_permission".to_string(), json!({ "path": note, "mode": "read", "reason": "lire la note posée hors de l'atelier" }).to_string())]).generate(turns, opts, sink),
                    ("c'est accordé", 0) => MockLlm::new("").with_tool_calls(vec![("read_file".to_string(), json!({ "path": note }).to_string())]).generate(turns, opts, sink),
                    _ => MockLlm::new("fait").generate(turns, opts, sink),
                }
            })))
        });
        let (runs, chats, taches) = (d.path().join("runs"), d.path().join("chats"), d.path().join("taches"));
        let racines = crate::code_tree::Roots { config: d.path().join("config"), data: d.path().join("data") };
        let (adresse, fil) = crate::daemon_host::tests::hote_garde_avec("j", move |h| {
            h.with_racines_de_code(racines).with_runs_root(runs).with_chats_root(chats).with_racine_des_taches(taches).with_chat_model(model)
        });
        let racine_url = format!("http://{adresse}");
        let poster = |route: &str, corps: Value| -> Value {
            let mut r = crate::daemon::agent_de_temoin().post(format!("{racine_url}{route}")).header("Cookie", "rag3weaver_ide=j").header("content-type", "application/json").send(corps.to_string()).unwrap();
            serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap()
        };
        assert_eq!(poster("/ide/describe", json!({ "manifest": manifeste }))["ok"], true);
        let mut r = crate::daemon::agent_de_temoin().get(format!("{racine_url}/ide/etat")).header("Cookie", "rag3weaver_ide=j").call().unwrap();
        let etat: Value = serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap();
        let canon = manifeste.canonicalize().unwrap();
        let id = etat["backends"].as_array().and_then(|bs| bs.iter().find(|b| b["manifest"].as_str().map(Path::new) == Some(canon.as_path()))).and_then(|b| b["id"].as_str()).unwrap().to_string();

        let mut n = Navigateur::lancer(&binaire);
        n.aller(&format!("{racine_url}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
        let prete = "document.querySelector('#send') && !document.querySelector('#send').disabled && document.getElementById('lien')?.textContent === 'flux : ouvert'";
        n.aller(&format!("{racine_url}/ide/code/{id}/chat"), prete);
        n.evaluer("document.querySelector('#message').value = 'lis la note'; document.querySelector('#send').click()");
        n.attendre("document.querySelector('#status')?.textContent === 'fini'", "le premier tour");
        let carte = "details.outil[data-outil=\"ask_permission\"] + .ui-du-tour .carte-accord";
        n.attendre(&format!("!!document.querySelector('{carte} button.accorder')"), "la carte d'accord sous l'appel");
        let texte = n.evaluer(&format!("document.querySelector('{carte}').textContent"));
        assert!(texte.as_str().is_some_and(|t| t.contains("lire") && t.contains("note.txt") && t.contains("lire la note posée hors de l'atelier")), "{texte}");
        n.evaluer("window.__pas_rechargee = 1");
        n.evaluer(&format!("document.querySelector('{carte} button.accorder').click()"));
        n.attendre(&format!("document.querySelector('{carte}')?.dataset.decision === 'accordé'"), "la carte dit la décision");
        assert_eq!(n.evaluer("window.__pas_rechargee ?? null"), json!(1), "la page s'est rechargée");
        let journal = std::fs::read_to_string(d.path().join("runs/decisions.jsonl")).expect("le journal des décisions");
        assert!(journal.contains("accordé") && journal.contains("l'utilisatrice, dans l'IDE") && journal.contains("note.txt"), "{journal}");

        n.evaluer("document.querySelector('#message').value = \"c'est accordé\"; document.querySelector('#send').click()");
        n.attendre("document.querySelectorAll('.message.assistant').length === 2 && document.querySelector('#status')?.textContent === 'fini'", "le second tour");
        assert!(vus.lock().unwrap().iter().any(|c| c.contains("la note du dehors")), "la lecture passe après l'accord : {:?}", vus.lock().unwrap());

        assert!(n.erreurs.is_empty(), "erreurs à la console : {:#?}", n.erreurs);
        drop(n);
        let (statut, _) = crate::daemon_host::tests::demander("POST", &format!("{racine_url}/arret"), Some("j"));
        assert_eq!(statut, 200);
        fil.join().unwrap();
    }

    /// **La carte d'une commande, dans un vrai navigateur** (lot 7, c7, 9 novembre 2026) :
    /// `mkdir` attend l'accord, la carte glisse sous l'appel et dit la commande ; « accorder »
    /// la décide sans recharger la page, et le tour repart de lui-même : la page montre le
    /// message de la carte et la réponse, sans que rien ne soit tapé.
    #[test]
    #[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
    fn the_command_card_grants_the_command_and_the_turn_starts_again_by_itself() {
        use crate::daemon_host::navigateur::{navigateur_ou_saut, Navigateur};
        use crate::llm::{CallbackLlm, Turn};
        const TEMOIN: &str = "the_command_card_grants_the_command_and_the_turn_starts_again_by_itself";
        let Some(binaire) = navigateur_ou_saut(TEMOIN) else {
            return;
        };
        let d = tempfile::tempdir().unwrap();
        let racine = d.path().join("rw");
        std::fs::create_dir_all(&racine).unwrap();
        let gabarits = d.path().join("atelier/templates");
        std::fs::create_dir_all(gabarits.parent().unwrap()).unwrap();
        crate::code_tree::copy(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates"), &gabarits, false).unwrap();
        let manifeste = gabarits.join("backends/atelier/backend.json");
        let mut m: Value = serde_json::from_slice(&std::fs::read(&manifeste).unwrap()).unwrap();
        m["workspace"]["root"] = json!(racine.to_string_lossy());
        std::fs::write(&manifeste, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
        let model: ChatModel = Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            Ok(Arc::new(CallbackLlm::new("faux-atelier", 8192, move |turns: &[Turn], opts, sink| {
                let depuis = turns.iter().rposition(|t| t.role == "user").unwrap_or(0);
                let message = turns[depuis].content.clone();
                let fait = turns[depuis..].iter().any(|t| t.role == "tool");
                let commande = match message.strip_prefix("lance ") {
                    Some(c) => Some(c.to_string()),
                    None if message.contains("est accordée : relance-la") => message.split('`').nth(1).map(str::to_string),
                    None => None,
                };
                match commande.filter(|_| !fait) {
                    Some(c) => MockLlm::new("").with_tool_calls(vec![("run_command".to_string(), json!({ "command": c }).to_string())]).generate(turns, opts, sink),
                    None => MockLlm::new("fait").generate(turns, opts, sink),
                }
            })))
        });
        let (runs, chats, taches) = (d.path().join("runs"), d.path().join("chats"), d.path().join("taches"));
        let racines = crate::code_tree::Roots { config: d.path().join("config"), data: d.path().join("data") };
        let (adresse, fil) = crate::daemon_host::tests::hote_garde_avec("j", move |h| {
            h.with_racines_de_code(racines).with_runs_root(runs).with_chats_root(chats).with_racine_des_taches(taches).with_chat_model(model)
        });
        let racine_url = format!("http://{adresse}");
        let mut r = crate::daemon::agent_de_temoin()
            .post(format!("{racine_url}/ide/describe"))
            .header("Cookie", "rag3weaver_ide=j")
            .header("content-type", "application/json")
            .send(json!({ "manifest": manifeste }).to_string())
            .unwrap();
        let decrit: Value = serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap();
        assert_eq!(decrit["ok"], true, "{decrit}");
        let mut r = crate::daemon::agent_de_temoin().get(format!("{racine_url}/ide/etat")).header("Cookie", "rag3weaver_ide=j").call().unwrap();
        let etat: Value = serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap();
        let canon = manifeste.canonicalize().unwrap();
        let id = etat["backends"].as_array().and_then(|bs| bs.iter().find(|b| b["manifest"].as_str().map(Path::new) == Some(canon.as_path()))).and_then(|b| b["id"].as_str()).unwrap().to_string();

        let mut n = Navigateur::lancer(&binaire);
        n.aller(&format!("{racine_url}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
        let prete = "document.querySelector('#send') && !document.querySelector('#send').disabled && document.getElementById('lien')?.textContent === 'flux : ouvert'";
        n.aller(&format!("{racine_url}/ide/code/{id}/chat"), prete);
        n.evaluer("document.querySelector('#message').value = 'lance mkdir faits'; document.querySelector('#send').click()");
        n.attendre("document.querySelector('#status')?.textContent === 'fini'", "le premier tour");
        let carte = "details.outil[data-outil=\"run_command\"] + .ui-du-tour .carte-accord";
        n.attendre(&format!("!!document.querySelector('{carte} button.accorder')"), "la carte d'accord sous l'appel");
        let texte = n.evaluer(&format!("document.querySelector('{carte}').textContent"));
        assert!(texte.as_str().is_some_and(|t| t.contains("lancer") && t.contains("mkdir faits")), "{texte}");
        assert!(!racine.join("faits").exists(), "rien ne s'est exécuté avant l'accord");
        n.evaluer("window.__pas_rechargee = 1");
        n.evaluer(&format!("document.querySelector('{carte} button.accorder').click()"));
        n.attendre(&format!("document.querySelector('{carte}')?.dataset.decision === 'accordé'"), "la carte dit la décision");
        n.attendre(
            "[...document.querySelectorAll('.message.user')].some(m => m.textContent.includes('est accordée')) && document.querySelectorAll('.message.assistant').length === 2 && document.querySelector('#status')?.textContent === 'fini'",
            "le tour relancé par la carte, dans la page",
        );
        assert_eq!(n.evaluer("window.__pas_rechargee ?? null"), json!(1), "la page s'est rechargée");
        assert!(racine.join("faits").is_dir(), "accordée et relancée, la commande passe");
        let journal = std::fs::read_to_string(d.path().join("runs/decisions.jsonl")).expect("le journal des décisions");
        assert!(journal.contains("mkdir faits") && journal.contains("l'utilisatrice, dans l'IDE"), "{journal}");

        assert!(n.erreurs.is_empty(), "erreurs à la console : {:#?}", n.erreurs);
        drop(n);
        let (statut, _) = crate::daemon_host::tests::demander("POST", &format!("{racine_url}/arret"), Some("j"));
        assert_eq!(statut, 200);
        fil.join().unwrap();
    }
}
