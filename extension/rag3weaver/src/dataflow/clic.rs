//! **Le graphe de clic d'un composant déclaré** (marche 3 de l'IDE, niveau 2, lot 1 ; étude
//! `docs/10-octobre-2026-18h20/codeparsers/08-le-composant-declare.md`).
//!
//! *« Un clic n'est pas une boucle, c'est un tour »* (vision générale) : dans un seul événement, le
//! chemin est sans cycle, de (l'événement, l'état d'avant) à l'état d'après. Un graphe de clic
//! s'écrit dans la syntaxe des graphes du backend (`components/<nom>.clic.mmd`), avec ses seuls
//! genres d'écran :
//!
//! - `Evenement(nom='onClick')` — l'entrée d'un tour : un port de sortie de la vue ; sort `charge`.
//! - `Etat(nom='filtre', schema='{…}', initial='…', portee='fenêtre')` — une case. Elle sort sa
//!   `valeur` (l'état d'avant) et reçoit `suivante` (l'état d'après). Portée `fenêtre` (par
//!   défaut, perdue au rechargement), `session` (rendue par l'hôte à la reconnexion) ou `hôte`
//!   (tenue par l'hôte, partagée par les fenêtres) — tranché par l'orchestration le 18 octobre,
//!   à relire par Lucie.
//! - `Pur(expr='…')` — une expression TypeScript sur ses ports d'entrée (des identifiants) ;
//!   sort `resultat`.
//! - `Effet(outil='search_code')` — un appel `/ide/call` ; ses ports d'entrée sont les arguments,
//!   il sort `reponse` et `erreur`, qui **ouvrent un tour nouveau** : un effet termine le tour.
//! - `Page(vers='…')` — changer de page.
//! - `Confirmer(message='…')` — la confirmation qu'un effet en écriture exige ; sort `confirme`.
//!
//! La validation dit chaque refus avec la ligne du `.mmd` : un genre du dataflow dans un graphe
//! de clic, un cycle dans un tour, une case mal typée, un effet que le composant ne déclare pas,
//! un outil que l'hôte ne rend pas appelable d'un écran, une écriture sans `Confirmer`, une case
//! de portée `hôte` dans un backend qui ne dit pas où la tenir.
use super::checkpoint::{EdgeDef, GraphDefinition};
use super::mermaid::parse_mermaid;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// Ce qu'un outil permet à un écran (la `ToolPolicy` du manifeste, `ecran`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Ecran {
    /// Appelable d'un écran ; ne change rien.
    Lecture,
    /// Appelable d'un écran, derrière un `Confirmer`.
    Ecriture,
}

/// La portée d'une case d'état.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Portee {
    /// Dans la page, perdue au rechargement (par défaut).
    #[serde(rename = "fenêtre")]
    Fenetre,
    /// Dans la page, rendue par l'hôte à la reconnexion.
    #[serde(rename = "session")]
    Session,
    /// Tenue par l'hôte, partagée par toutes les fenêtres.
    #[serde(rename = "hôte")]
    Hote,
}

impl Portee {
    fn lire(texte: &str) -> Option<Self> {
        match texte {
            "fenêtre" | "fenetre" => Some(Self::Fenetre),
            "session" => Some(Self::Session),
            "hôte" | "hote" => Some(Self::Hote),
            _ => None,
        }
    }
}

/// Le genre d'un nœud d'écran, avec sa configuration lue.
#[derive(Debug, Clone, PartialEq)]
pub enum Genre {
    Evenement { nom: String },
    Etat { nom: String, schema: Value, initial: Value, portee: Portee },
    Pur { expr: String },
    Effet { outil: String, ecran: Ecran },
    Page { vers: String },
    Confirmer { message: String },
}

/// Un nœud du graphe de clic. (La forme JSON du graphe, pour l'IDE, est celle de la route des
/// graphes : `node_type` = le genre, `config` telle qu'écrite, `line`.)
#[derive(Debug, Clone, PartialEq)]
pub struct NoeudClic {
    pub nom: String,
    pub genre: Genre,
    /// La ligne de sa déclaration dans le `.mmd`.
    pub ligne: usize,
}

/// Un graphe de clic validé.
#[derive(Debug, Clone)]
pub struct GrapheClic {
    pub noeuds: Vec<NoeudClic>,
    pub aretes: Vec<EdgeDef>,
}

/// Un refus, à sa ligne quand elle se lit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefusClic {
    pub ligne: Option<usize>,
    pub message: String,
}

impl std::fmt::Display for RefusClic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.ligne {
            Some(l) => write!(f, "ligne {l} : {}", self.message),
            None => write!(f, "{}", self.message),
        }
    }
}

/// Ce que le backend et le composant disent, contre quoi le graphe se vérifie.
#[derive(Debug, Clone, Default)]
pub struct ContexteClic {
    /// Les effets que le composant déclare (`effets` de son `.component.json`).
    pub effets: Vec<String>,
    /// Les outils exposés du backend, et ce que chacun permet à un écran (`None` : pas appelable
    /// d'un écran).
    pub outils: BTreeMap<String, Option<Ecran>>,
    /// Où l'hôte tient les cases de portée `hôte` (l'entité déclarée par le manifeste) ; `None`
    /// si le manifeste ne le dit pas.
    pub etat_hote: Option<String>,
}

const GENRES: &[&str] = &["Evenement", "Etat", "Pur", "Effet", "Page", "Confirmer"];

/// Les ports de chaque genre : (entrées, sorties). `None` en entrée : des ports libres (les
/// variables d'un `Pur`, les arguments d'un `Effet`).
fn ports(genre: &Genre) -> (Option<&'static [&'static str]>, &'static [&'static str]) {
    match genre {
        Genre::Evenement { .. } => (Some(&[]), &["charge"]),
        Genre::Etat { .. } => (Some(&["suivante"]), &["valeur"]),
        Genre::Pur { .. } => (None, &["resultat"]),
        Genre::Effet { .. } => (None, &["reponse", "erreur"]),
        Genre::Page { .. } => (Some(&["parametres"]), &[]),
        Genre::Confirmer { .. } => (Some(&["demande"]), &["confirme"]),
    }
}

fn est_un_identifiant(nom: &str) -> bool {
    let mut c = nom.chars();
    matches!(c.next(), Some(p) if p.is_ascii_alphabetic() || p == '_' || p == '$')
        && c.all(|x| x.is_ascii_alphanumeric() || x == '_' || x == '$')
}

/// La ligne où chaque nœud est déclaré (`nom["…"]`), lue dans le texte.
/// La ligne où chaque nœud d'un `.mmd` est déclaré (sa première apparition
/// `nom["…"]`) : pour les refus, et pour la route des graphes.
pub(crate) fn lignes_des_noeuds(texte: &str) -> HashMap<String, usize> {
    let mut lignes = HashMap::new();
    for (i, brute) in texte.lines().enumerate() {
        let ligne = brute.split("%%").next().unwrap_or("");
        for morceau in ligne.split("-->") {
            let morceau = morceau.trim();
            let morceau = morceau.strip_prefix('|').and_then(|m| m.split_once('|')).map(|(_, r)| r.trim()).unwrap_or(morceau);
            if let Some(pos) = morceau.find("[\"") {
                let nom = morceau[..pos].trim().to_string();
                if !nom.is_empty() {
                    lignes.entry(nom).or_insert(i + 1);
                }
            }
        }
    }
    lignes
}

fn texte_de(config: &Value, cle: &str) -> Option<String> {
    config.get(cle).map(|v| match v {
        Value::String(s) => s.clone(),
        autre => autre.to_string(),
    })
}

/// Une valeur JSON écrite en texte dans la config (`schema='{…}'`), ou déjà typée (`initial=0`).
fn json_de(config: &Value, cle: &str) -> Option<Result<Value, String>> {
    config.get(cle).map(|v| match v {
        Value::String(s) => serde_json::from_str(s).map_err(|e| format!("{cle} n'est pas du JSON : {e}")),
        autre => Ok(autre.clone()),
    })
}

/// Lit et vérifie un graphe de clic. Tous les refus sont rendus, dans l'ordre des lignes.
pub fn valider_graphe_de_clic(texte: &str, ctx: &ContexteClic) -> Result<GrapheClic, Vec<RefusClic>> {
    let def: GraphDefinition = parse_mermaid(texte).map_err(|e| {
        vec![RefusClic { ligne: None, message: e.to_string() }]
    })?;
    let lignes = lignes_des_noeuds(texte);
    let mut refus = Vec::new();
    let mut noeuds = Vec::new();

    for n in &def.nodes {
        let ligne = lignes.get(&n.name).copied();
        let mut refuse = |message: String| refus.push(RefusClic { ligne, message });
        let c = &n.config;
        let genre = match n.node_type.as_str() {
            "Evenement" => match texte_de(c, "nom") {
                Some(nom) if est_un_identifiant(&nom) => Genre::Evenement { nom },
                _ => {
                    refuse(format!("{} : un Evenement nomme le port de la vue qui l'émet (nom='onClick')", n.name));
                    continue;
                }
            },
            "Etat" => {
                let Some(nom) = texte_de(c, "nom").filter(|n| est_un_identifiant(n)) else {
                    refuse(format!("{} : une case se nomme (nom='filtre')", n.name));
                    continue;
                };
                let schema = match json_de(c, "schema") {
                    Some(Ok(s)) => s,
                    Some(Err(e)) => {
                        refuse(format!("{} : {e}", n.name));
                        continue;
                    }
                    None => {
                        refuse(format!("{} : une case porte son type (schema='{{\"type\": …}}')", n.name));
                        continue;
                    }
                };
                let initial = match json_de(c, "initial") {
                    Some(Ok(v)) => v,
                    Some(Err(e)) => {
                        refuse(format!("{} : {e}", n.name));
                        continue;
                    }
                    None => Value::Null,
                };
                match jsonschema::validator_for(&schema) {
                    Ok(v) => {
                        if let Err(e) = v.validate(&initial) {
                            refuse(format!("{} : la valeur initiale {initial} ne suit pas son schéma : {e}", n.name));
                        }
                    }
                    Err(e) => {
                        refuse(format!("{} : schéma invalide : {e}", n.name));
                        continue;
                    }
                }
                let portee = match texte_de(c, "portee") {
                    None => Portee::Fenetre,
                    Some(p) => match Portee::lire(&p) {
                        Some(p) => p,
                        None => {
                            refuse(format!("{} : portée « {p} » inconnue (fenêtre, session ou hôte)", n.name));
                            continue;
                        }
                    },
                };
                if portee == Portee::Hote && ctx.etat_hote.is_none() {
                    refuse(format!(
                        "{} : la case « {nom} » est de portée hôte, mais le manifeste ne dit pas où l'hôte la tient ; \
                         déclarez \"ui_state\": {{\"entity\": \"<Entité>\"}} dans le backend.json, ou donnez-lui la portée fenêtre ou session",
                        n.name
                    ));
                }
                Genre::Etat { nom, schema, initial, portee }
            }
            "Pur" => match texte_de(c, "expr").filter(|e| !e.trim().is_empty()) {
                Some(expr) => Genre::Pur { expr },
                None => {
                    refuse(format!("{} : un nœud pur porte son expression TypeScript (expr='…')", n.name));
                    continue;
                }
            },
            "Effet" => {
                let Some(outil) = texte_de(c, "outil") else {
                    refuse(format!("{} : un effet nomme l'outil qu'il appelle (outil='…')", n.name));
                    continue;
                };
                if !ctx.effets.contains(&outil) {
                    refuse(format!(
                        "{} : l'effet appelle « {outil} », que le composant ne déclare pas ; ajoutez-le à ses \"effets\"",
                        n.name
                    ));
                }
                let ecran = match ctx.outils.get(&outil) {
                    None => {
                        refuse(format!("{} : « {outil} » n'est pas un outil exposé de ce backend", n.name));
                        continue;
                    }
                    Some(None) => {
                        refuse(format!(
                            "{} : l'outil « {outil} » n'est pas appelable d'un écran ; son \"policy\" doit dire \"ecran\": \"lecture\" ou \"ecriture\"",
                            n.name
                        ));
                        continue;
                    }
                    Some(Some(e)) => *e,
                };
                Genre::Effet { outil, ecran }
            }
            "Page" => match texte_de(c, "vers") {
                Some(vers) => Genre::Page { vers },
                None => {
                    refuse(format!("{} : un changement de page dit où il va (vers='…')", n.name));
                    continue;
                }
            },
            "Confirmer" => Genre::Confirmer { message: texte_de(c, "message").unwrap_or_default() },
            autre => {
                refuse(format!(
                    "{} : « {autre} » n'est pas un genre d'écran ; un graphe de clic n'a que {}",
                    n.name,
                    GENRES.join(", ")
                ));
                continue;
            }
        };
        noeuds.push(NoeudClic { nom: n.name.clone(), genre, ligne: ligne.unwrap_or(0) });
    }

    let par_nom: HashMap<&str, &NoeudClic> = noeuds.iter().map(|n| (n.nom.as_str(), n)).collect();
    // Les ports de chaque arête, et ce qui arrive à chaque nœud.
    let mut entrees: HashMap<&str, Vec<&EdgeDef>> = HashMap::new();
    for e in &def.edges {
        let (Some(de), Some(vers)) = (par_nom.get(e.from_node.as_str()), par_nom.get(e.to_node.as_str())) else {
            continue; // un nœud refusé plus haut : déjà dit
        };
        let ligne_arete = Some(vers.ligne).filter(|l| *l > 0);
        let (_, sorties) = ports(&de.genre);
        if !sorties.contains(&e.from_port.as_str()) {
            refus.push(RefusClic {
                ligne: ligne_arete,
                message: format!("{} n'a pas de port de sortie « {} » (il a : {})", de.nom, e.from_port, sorties.join(", ")),
            });
        }
        let (permises, _) = ports(&vers.genre);
        match permises {
            Some(p) if !p.contains(&e.to_port.as_str()) => refus.push(RefusClic {
                ligne: ligne_arete,
                message: format!(
                    "{} n'a pas de port d'entrée « {} »{}",
                    vers.nom,
                    e.to_port,
                    if p.is_empty() { " (il n'en a aucun)".to_string() } else { format!(" (il a : {})", p.join(", ")) }
                ),
            }),
            None if !est_un_identifiant(&e.to_port) => refus.push(RefusClic {
                ligne: ligne_arete,
                message: format!("le port « {} » de {} doit être un identifiant : c'est une variable de son expression", e.to_port, vers.nom),
            }),
            _ => {}
        }
        entrees.entry(e.to_node.as_str()).or_default().push(e);
    }

    // Une écriture passe par un Confirmer.
    for n in &noeuds {
        if let Genre::Effet { outil, ecran: Ecran::Ecriture } = &n.genre {
            let confirme = entrees.get(n.nom.as_str()).is_some_and(|es| {
                es.iter().any(|e| matches!(par_nom.get(e.from_node.as_str()).map(|x| &x.genre), Some(Genre::Confirmer { .. })))
            });
            if !confirme {
                refus.push(RefusClic {
                    ligne: Some(n.ligne),
                    message: format!("{} : l'outil « {outil} » écrit ; l'effet doit venir d'un Confirmer (… -->|confirme:…| {})", n.nom, n.nom),
                });
            }
        }
    }

    // Un tour est sans cycle. La valeur d'une case est l'état d'avant, sa suivante l'état
    // d'après : une case ne referme donc pas de boucle. La réponse d'un effet ouvre un autre tour.
    let mut suivants: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for e in &def.edges {
        let (Some(de), Some(_)) = (par_nom.get(e.from_node.as_str()), par_nom.get(e.to_node.as_str())) else { continue };
        if matches!(de.genre, Genre::Etat { .. } | Genre::Effet { .. }) {
            continue;
        }
        suivants.entry(e.from_node.as_str()).or_default().insert(e.to_node.as_str());
    }
    if let Some(cycle) = un_cycle(&suivants) {
        let ligne = par_nom.get(cycle[0]).map(|n| n.ligne);
        refus.push(RefusClic {
            ligne,
            message: format!(
                "un tour de clic est sans cycle : {} revient sur lui-même ; passez par une case (Etat) ou un effet",
                cycle.join(" → ")
            ),
        });
    }

    if refus.is_empty() {
        Ok(GrapheClic { noeuds, aretes: def.edges })
    } else {
        refus.sort_by_key(|r| r.ligne.unwrap_or(usize::MAX));
        Err(refus)
    }
}

/// Un cycle du graphe, s'il y en a un (parcours en profondeur, ordre déterministe).
fn un_cycle<'a>(suivants: &BTreeMap<&'a str, BTreeSet<&'a str>>) -> Option<Vec<&'a str>> {
    #[derive(Clone, Copy, PartialEq)]
    enum Etat {
        EnCours,
        Fini,
    }
    fn visiter<'a>(
        n: &'a str,
        suivants: &BTreeMap<&'a str, BTreeSet<&'a str>>,
        vus: &mut HashMap<&'a str, Etat>,
        pile: &mut Vec<&'a str>,
    ) -> Option<Vec<&'a str>> {
        vus.insert(n, Etat::EnCours);
        pile.push(n);
        for &s in suivants.get(n).into_iter().flatten() {
            match vus.get(s) {
                Some(Etat::EnCours) => {
                    let debut = pile.iter().position(|x| *x == s).unwrap_or(0);
                    let mut c = pile[debut..].to_vec();
                    c.push(s);
                    return Some(c);
                }
                Some(Etat::Fini) => {}
                None => {
                    if let Some(c) = visiter(s, suivants, vus, pile) {
                        return Some(c);
                    }
                }
            }
        }
        pile.pop();
        vus.insert(n, Etat::Fini);
        None
    }
    let mut vus = HashMap::new();
    for &n in suivants.keys() {
        if !vus.contains_key(n) {
            if let Some(c) = visiter(n, suivants, &mut vus, &mut Vec::new()) {
                return Some(c);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> ContexteClic {
        ContexteClic {
            effets: vec!["search_code".into(), "save_product".into()],
            outils: BTreeMap::from([
                ("search_code".to_string(), Some(Ecran::Lecture)),
                ("save_product".to_string(), Some(Ecran::Ecriture)),
                ("run_command".to_string(), None),
            ]),
            etat_hote: None,
        }
    }

    const FILTRE: &str = r#"graph LR
    saisie["Evenement(nom='onSaisie')"]
    filtre["Etat(nom='filtre', schema='{"type":"string"}', initial='""')"]
    liste["Etat(nom='liste', schema='{"type":"array"}', initial='[]')"]
    visibles["Pur(expr='liste.filter(p => p.nom.includes(filtre))')"]
    chercher["Effet(outil='search_code')"]
    saisie -->|charge:suivante| filtre
    saisie -->|charge:query| chercher
    filtre -->|valeur:filtre| visibles
    liste -->|valeur:liste| visibles
    chercher -->|reponse:suivante| liste
"#;

    fn messages(r: &[RefusClic]) -> String {
        r.iter().map(|r| r.to_string()).collect::<Vec<_>>().join("\n")
    }

    #[test]
    fn un_graphe_de_clic_bien_forme_passe() {
        let g = valider_graphe_de_clic(FILTRE, &ctx()).unwrap_or_else(|r| panic!("{}", messages(&r)));
        assert_eq!(g.noeuds.len(), 5);
        let filtre = g.noeuds.iter().find(|n| n.nom == "filtre").unwrap();
        assert_eq!(filtre.ligne, 3);
        assert!(matches!(&filtre.genre, Genre::Etat { portee: Portee::Fenetre, .. }));
    }

    #[test]
    fn un_cycle_dans_un_tour_est_refuse() {
        let g = r#"graph LR
    clic["Evenement(nom='onClick')"]
    a["Pur(expr='x + 1')"]
    b["Pur(expr='y * 2')"]
    clic -->|charge:x| a
    a -->|resultat:y| b
    b -->|resultat:x| a
"#;
        let r = valider_graphe_de_clic(g, &ctx()).unwrap_err();
        assert!(messages(&r).contains("sans cycle"), "{}", messages(&r));
        assert!(r.iter().any(|x| x.ligne == Some(3)), "{}", messages(&r));
    }

    #[test]
    fn une_case_ne_referme_pas_de_boucle() {
        // compteur.valeur → +1 → compteur.suivante : l'état d'avant donne l'état d'après.
        let g = r#"graph LR
    clic["Evenement(nom='onClick')"]
    compteur["Etat(nom='n', schema='{"type":"number"}', initial=0)"]
    plus["Pur(expr='n + 1')"]
    clic -->|charge:declenche| plus
    compteur -->|valeur:n| plus
    plus -->|resultat:suivante| compteur
"#;
        valider_graphe_de_clic(g, &ctx()).unwrap_or_else(|r| panic!("{}", messages(&r)));
    }

    #[test]
    fn un_effet_non_declare_est_refuse_a_sa_ligne() {
        let mut c = ctx();
        c.effets.retain(|e| e != "search_code");
        let r = valider_graphe_de_clic(FILTRE, &c).unwrap_err();
        assert!(r.iter().any(|x| x.ligne == Some(6) && x.message.contains("ne déclare pas")), "{}", messages(&r));
    }

    #[test]
    fn un_outil_sans_ecran_est_refuse() {
        let g = r#"graph LR
    clic["Evenement(nom='onClick')"]
    lancer["Effet(outil='run_command')"]
    clic -->|charge:command| lancer
"#;
        let mut c = ctx();
        c.effets.push("run_command".into());
        let r = valider_graphe_de_clic(g, &c).unwrap_err();
        assert!(messages(&r).contains("pas appelable d'un écran"), "{}", messages(&r));
        assert!(messages(&r).contains("\"ecran\""), "le message dit quoi déclarer : {}", messages(&r));
    }

    #[test]
    fn une_ecriture_sans_confirmer_est_refusee_et_avec_elle_passe() {
        let sans = r#"graph LR
    clic["Evenement(nom='onClick')"]
    enregistrer["Effet(outil='save_product')"]
    clic -->|charge:record| enregistrer
"#;
        let r = valider_graphe_de_clic(sans, &ctx()).unwrap_err();
        assert!(messages(&r).contains("Confirmer"), "{}", messages(&r));
        let avec = r#"graph LR
    clic["Evenement(nom='onClick')"]
    ok["Confirmer(message='Enregistrer ?')"]
    enregistrer["Effet(outil='save_product')"]
    clic -->|charge:demande| ok
    ok -->|confirme:record| enregistrer
"#;
        valider_graphe_de_clic(avec, &ctx()).unwrap_or_else(|r| panic!("{}", messages(&r)));
    }

    #[test]
    fn une_case_de_portee_hote_sans_lieu_declare_est_refusee_en_disant_quoi_declarer() {
        let g = r#"graph LR
    commun["Etat(nom='filtre', schema='{"type":"string"}', initial='""', portee='hôte')"]
"#;
        let r = valider_graphe_de_clic(g, &ctx()).unwrap_err();
        let m = messages(&r);
        assert!(m.contains("portée hôte") && m.contains("\"ui_state\""), "{m}");
        let mut c = ctx();
        c.etat_hote = Some("EtatEcran".into());
        valider_graphe_de_clic(g, &c).unwrap_or_else(|r| panic!("{}", messages(&r)));
    }

    #[test]
    fn un_genre_du_dataflow_n_entre_pas_dans_un_graphe_de_clic() {
        let g = "graph LR\n    w[\"EntityRecordNode(entity='Product')\"]\n";
        let r = valider_graphe_de_clic(g, &ctx()).unwrap_err();
        assert!(messages(&r).contains("n'est pas un genre d'écran"), "{}", messages(&r));
    }

    #[test]
    fn une_case_dont_l_initial_ne_suit_pas_le_schema_est_refusee() {
        let g = r#"graph LR
    n["Etat(nom='n', schema='{"type":"number"}', initial='"trois"')"]
"#;
        let r = valider_graphe_de_clic(g, &ctx()).unwrap_err();
        assert!(messages(&r).contains("ne suit pas son schéma"), "{}", messages(&r));
    }

    #[test]
    fn un_port_inconnu_est_refuse() {
        let g = r#"graph LR
    clic["Evenement(nom='onClick')"]
    n["Etat(nom='n', schema='{"type":"number"}', initial=0)"]
    clic -->|charge:valeur| n
"#;
        let r = valider_graphe_de_clic(g, &ctx()).unwrap_err();
        assert!(messages(&r).contains("pas de port d'entrée « valeur »"), "{}", messages(&r));
    }
}
