//! **Les propositions** (page 08 de everything declarative, P2) : un ensemble
//! de déclarations proposé, validé sans être écrit ([`Backend::valider_sur`]),
//! gardé par le backend avec son verdict, publié sur le bus (sujet
//! `declarations`), puis appliqué ([`Backend::declare_set`], qui revalide sur
//! l'arbre du moment) ou rejeté avec son pourquoi.
//!
//! La proposition est un état **de l'hôte** : la page de l'IDE ne fait que la
//! montrer et dire oui ou non ; l'agent la lit, voit le verdict et les rejets,
//! et repropose. Le mode du backend (`declarations` du manifeste) dit qui
//! applique : `approbation` (le défaut) — une personne, dans l'IDE ; `auto` —
//! l'agent aussi.

use crate::backend::Backend;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

/// Qui applique une proposition (`declarations` du manifeste).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModeDeclarations {
    /// L'agent propose, une personne applique dans l'IDE (le défaut).
    #[default]
    Approbation,
    /// L'agent déclare directement, et peut appliquer ses propositions.
    Auto,
}

/// Un fichier d'une proposition : son contenu d'avant (absent : un fichier
/// neuf) et celui d'après.
#[derive(Debug, Clone, Serialize)]
pub struct FichierPropose {
    pub path: String,
    pub avant: Option<String>,
    pub apres: String,
}

/// Où en est une proposition.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "etat", rename_all = "lowercase")]
pub enum EtatProposition {
    Proposee,
    Appliquee { commit: Option<String>, version: u64 },
    Rejetee { pourquoi: String },
}

/// Une proposition gardée.
#[derive(Debug, Clone, Serialize)]
pub struct Proposition {
    pub id: u64,
    pub raison: String,
    /// Qui l'a faite : `ide`, l'agent, une session.
    pub auteur: String,
    pub at_ms: u64,
    pub fichiers: Vec<FichierPropose>,
    /// Le refus de la validation, s'il y en a un (chemin et ligne quand le
    /// chargement les dit). `None` : acceptée.
    pub refus: Option<String>,
    #[serde(flatten)]
    pub etat: EtatProposition,
}

/// Les propositions d'un backend, en mémoire, dans l'ordre.
#[derive(Debug, Default)]
pub struct Propositions {
    suivante: AtomicU64,
    liste: Mutex<Vec<Proposition>>,
}

fn maintenant_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

impl Backend {
    /// **Proposer un ensemble de déclarations** : il est validé sur un
    /// recouvrement, sans rien écrire ; la proposition est gardée **avec son
    /// verdict** (refusée, elle reste lisible : l'agent repropose) et publiée
    /// (`PropositionChanged`, `proposed`). Une raison vide ou un ensemble vide
    /// est une erreur, pas une proposition.
    pub fn proposer(&self, files: &[(String, String)], raison: &str, auteur: &str) -> Result<Proposition, String> {
        if raison.trim().is_empty() {
            return Err("une proposition dit pourquoi : la raison est vide".into());
        }
        if files.is_empty() {
            return Err("une proposition nomme au moins un fichier".into());
        }
        let refus = self.valider_sur(files).err();
        let fichiers = files
            .iter()
            .map(|(path, apres)| FichierPropose {
                path: path.clone(),
                // Le contenu d'avant, lu dans l'arbre (un chemin refusé n'en a pas).
                avant: self.declaration_path(path).ok().and_then(|p| std::fs::read_to_string(p).ok()),
                apres: apres.clone(),
            })
            .collect();
        let p = Proposition {
            id: self.propositions.suivante.fetch_add(1, Ordering::SeqCst) + 1,
            raison: raison.to_string(),
            auteur: auteur.to_string(),
            at_ms: maintenant_ms(),
            fichiers,
            refus,
            etat: EtatProposition::Proposee,
        };
        self.propositions.liste.lock().unwrap_or_else(|e| e.into_inner()).push(p.clone());
        self.publier_proposition(&p, "proposed");
        Ok(p)
    }

    /// Les propositions de ce backend, dans l'ordre.
    pub fn propositions(&self) -> Vec<Proposition> {
        self.propositions.liste.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Une proposition par son id.
    pub fn proposition(&self, id: u64) -> Option<Proposition> {
        self.propositions.liste.lock().unwrap_or_else(|e| e.into_inner()).iter().find(|p| p.id == id).cloned()
    }

    /// **Appliquer une proposition** : par [`Backend::declare_set`], qui la
    /// revalide sur l'arbre du moment (il a pu bouger), l'écrit, recharge et
    /// commite sous son verrou. Refusé pour une proposition déjà appliquée,
    /// rejetée, ou dont la validation a refusé ; un refus de `declare_set` la
    /// laisse proposée, et le dit.
    pub fn appliquer(&self, id: u64) -> Result<Proposition, String> {
        let p = self.proposition(id).ok_or_else(|| format!("pas de proposition {id}"))?;
        match &p.etat {
            EtatProposition::Proposee => {}
            autre => return Err(format!("la proposition {id} n'est plus proposée : {autre:?}")),
        }
        if let Some(refus) = &p.refus {
            return Err(format!("la proposition {id} a été refusée à la validation : {refus}"));
        }
        let files: Vec<(String, String)> = p.fichiers.iter().map(|f| (f.path.clone(), f.apres.clone())).collect();
        let declared = self.declare_set(&files, &p.raison)?;
        let etat = EtatProposition::Appliquee { commit: declared.commit, version: declared.version };
        self.changer_l_etat(id, etat, "applied")
    }

    /// **Rejeter une proposition**, avec son pourquoi (gardé, lu par l'agent).
    pub fn rejeter(&self, id: u64, pourquoi: &str) -> Result<Proposition, String> {
        if pourquoi.trim().is_empty() {
            return Err("un rejet dit pourquoi".into());
        }
        let p = self.proposition(id).ok_or_else(|| format!("pas de proposition {id}"))?;
        if p.etat != EtatProposition::Proposee {
            return Err(format!("la proposition {id} n'est plus proposée : {:?}", p.etat));
        }
        self.changer_l_etat(id, EtatProposition::Rejetee { pourquoi: pourquoi.to_string() }, "rejected")
    }

    fn changer_l_etat(&self, id: u64, etat: EtatProposition, change: &str) -> Result<Proposition, String> {
        let p = {
            let mut liste = self.propositions.liste.lock().unwrap_or_else(|e| e.into_inner());
            let p = liste.iter_mut().find(|p| p.id == id).ok_or_else(|| format!("pas de proposition {id}"))?;
            p.etat = etat;
            p.clone()
        };
        self.publier_proposition(&p, change);
        Ok(p)
    }

    fn publier_proposition(&self, p: &Proposition, change: &str) {
        self.bus_de_publication().emit(crate::events::Event::PropositionChanged {
            id: p.id,
            change: change.to_string(),
            paths: p.fichiers.iter().map(|f| f.path.clone()).collect(),
            accepted: p.refus.is_none(),
            at_ms: maintenant_ms(),
        });
    }

    /// Le mode des déclarations de ce backend.
    pub fn mode_declarations(&self) -> ModeDeclarations {
        self.prepared().manifest.declarations
    }
}

/// Une proposition en JSON, sans les contenus (la liste).
pub fn resume(p: &Proposition) -> Value {
    let mut v = serde_json::to_value(p).unwrap_or(Value::Null);
    if let Some(fichiers) = v.get_mut("fichiers").and_then(Value::as_array_mut) {
        for f in fichiers {
            if let Some(o) = f.as_object_mut() {
                let neuf = o.get("avant").is_none_or(Value::is_null);
                o.remove("avant");
                o.remove("apres");
                o.insert("neuf".into(), json!(neuf));
            }
        }
    }
    v
}

/// **Le diff de deux graphes**, nœud à nœud et arête à arête (la forme de
/// `/ide/graphe`) : les nœuds ajoutés, retirés, changés (type ou config), et
/// les arêtes ajoutées ou retirées. Un graphe absent d'un côté : tout est
/// ajouté, ou tout retiré.
pub fn diff_de_graphes(avant: Option<&Value>, apres: Option<&Value>) -> Value {
    let noeuds = |g: Option<&Value>| -> std::collections::BTreeMap<String, Value> {
        g.and_then(|g| g["nodes"].as_array())
            .into_iter()
            .flatten()
            .filter_map(|n| Some((n["name"].as_str()?.to_string(), json!({"node_type": n["node_type"], "config": n["config"]}))))
            .collect()
    };
    let aretes = |g: Option<&Value>| -> std::collections::BTreeSet<String> {
        g.and_then(|g| g["edges"].as_array())
            .into_iter()
            .flatten()
            .map(|e| format!("{}.{}→{}.{}", e["from_node"].as_str().unwrap_or(""), e["from_port"].as_str().unwrap_or(""), e["to_node"].as_str().unwrap_or(""), e["to_port"].as_str().unwrap_or("")))
            .collect()
    };
    let (na, nb) = (noeuds(avant), noeuds(apres));
    let (ea, eb) = (aretes(avant), aretes(apres));
    json!({
        "ajoutes": nb.keys().filter(|k| !na.contains_key(*k)).collect::<Vec<_>>(),
        "retires": na.keys().filter(|k| !nb.contains_key(*k)).collect::<Vec<_>>(),
        "changes": nb.iter().filter(|(k, v)| na.get(*k).is_some_and(|a| a != *v)).map(|(k, _)| k).collect::<Vec<_>>(),
        "aretes_ajoutees": eb.difference(&ea).collect::<Vec<_>>(),
        "aretes_retirees": ea.difference(&eb).collect::<Vec<_>>(),
    })
}

impl Backend {
    /// **Le détail d'une proposition**, pour la vue de l'IDE : la
    /// proposition entière (contenus d'avant et d'après), les graphes qu'elle
    /// touche — un outil ou une réaction dont le `.mmd` est proposé, un
    /// composant dont la déclaration, la vue ou le graphe de clic l'est —,
    /// chacun avant (le disque), après (le recouvrement) et leur diff ; et
    /// les composants touchés, que l'aperçu peut jouer sur la proposition.
    pub fn detail_de_proposition(&self, id: u64) -> Result<Value, String> {
        use crate::backend::GraphTarget;
        let p = self.proposition(id).ok_or_else(|| format!("pas de proposition {id}"))?;
        let avant = self.prepared();
        let fichiers: Vec<(String, String)> = p.fichiers.iter().map(|f| (f.path.clone(), f.apres.clone())).collect();
        let recouvrement = crate::lecteur::Recouvrement::sur(&avant.directory, &fichiers);
        let proposes: std::collections::BTreeSet<std::path::PathBuf> =
            fichiers.iter().map(|(chemin, _)| crate::lecteur::normaliser(&avant.directory.join(chemin))).collect();
        let propose = |chemin: &std::path::Path| proposes.contains(&crate::lecteur::normaliser(&avant.directory.join(chemin)));
        // L'après n'existe que si la validation l'a accepté.
        let apres = match &p.refus {
            None => crate::backend::PreparedBackend::load_avec(avant.manifest_path(), &recouvrement).ok(),
            Some(_) => None,
        };
        let mut cibles: Vec<GraphTarget> = Vec::new();
        for m in [Some(&avant.manifest), apres.as_ref().map(|a| &a.manifest)].into_iter().flatten() {
            for (nom, t) in &m.tools {
                if propose(&t.graph) && !cibles.contains(&GraphTarget::Tool(nom.clone())) {
                    cibles.push(GraphTarget::Tool(nom.clone()));
                }
            }
            for (nom, r) in &m.reactions {
                if propose(&r.graph) && !cibles.contains(&GraphTarget::Reaction(nom.clone())) {
                    cibles.push(GraphTarget::Reaction(nom.clone()));
                }
            }
        }
        // Les composants : par leur déclaration, leur vue ou leur graphe de clic.
        let mut composants: Vec<String> = Vec::new();
        let touche = |c: &crate::components::ComponentDecl| {
            propose(&std::path::Path::new("components").join(&c.source))
                || c.clic.as_ref().is_some_and(|k| propose(&std::path::Path::new("components").join(k)))
        };
        let declares: Vec<String> = fichiers
            .iter()
            .filter(|(chemin, _)| chemin.ends_with(crate::components::SUFFIX))
            .filter_map(|(_, contenu)| serde_json::from_str::<Value>(contenu).ok()?["name"].as_str().map(str::to_string))
            .collect();
        let les_composants = match &apres {
            Some(a) => a.components_on_disk_avec(&recouvrement).ok(),
            None => avant.components_on_disk().ok(),
        };
        for c in les_composants.iter().flat_map(|cs| cs.components.values()) {
            if (touche(c) || declares.contains(&c.name)) && !composants.contains(&c.name) {
                composants.push(c.name.clone());
            }
        }
        for c in &composants {
            cibles.push(GraphTarget::Component(c.clone()));
        }
        let graphes: Vec<Value> = cibles
            .iter()
            .map(|cible| {
                let g_avant = avant.graph_json(cible).ok();
                let g_apres = apres.as_ref().and_then(|a| a.graph_json_avec(cible, &recouvrement).ok());
                json!({
                    "cible": cible.to_json(),
                    "avant": g_avant,
                    "apres": g_apres,
                    "diff": diff_de_graphes(g_avant.as_ref(), g_apres.as_ref()),
                })
            })
            .filter(|g| !(g["avant"]["nodes"].as_array().is_none_or(Vec::is_empty) && g["apres"]["nodes"].as_array().is_none_or(Vec::is_empty)))
            .collect();
        Ok(json!({ "proposition": p, "graphes": graphes, "composants": composants }))
    }

    /// **Le recouvrement d'une proposition** : ce sur quoi l'aperçu joue un
    /// composant proposé sans l'appliquer.
    pub fn recouvrement_de_proposition(&self, id: u64) -> Result<crate::lecteur::Recouvrement, String> {
        let p = self.proposition(id).ok_or_else(|| format!("pas de proposition {id}"))?;
        let fichiers: Vec<(String, String)> = p.fichiers.iter().map(|f| (f.path.clone(), f.apres.clone())).collect();
        Ok(crate::lecteur::Recouvrement::sur(&self.prepared().directory, &fichiers))
    }
}
