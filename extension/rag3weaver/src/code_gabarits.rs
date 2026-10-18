//! **Les gabarits indexés comme du code** (marche 3, niveau 2, lot 4 ; étude
//! `docs/10-octobre-2026-18h20/codeparsers/08-le-composant-declare.md` §4).
//!
//! Un graphe `.mmd`, un `backend.json`, la déclaration d'un composant (`*.component.json`) ou d'un
//! nœud scripté (`*.node.json`) n'ont pas de grammaire dans codeparsers : ils entraient dans
//! l'index comme un seul scope `texte_brut`. Ici, ils sont découpés en **scopes nommés**, et leurs
//! renvois deviennent des **mentions par le nom**, que la résolution de l'index relie comme celles
//! du code (`pending` → `MENTIONS` → `CONSUMES`) :
//!
//! - un `.mmd` : un scope `graphe` (le nom du fichier sans `.mmd`), puis un scope `noeud` par nœud,
//!   enfant du graphe, qui mentionne **son type de nœud** (un nœud scripté y trouve sa déclaration)
//!   et, pour un `Effet` d'un graphe de clic, **l'outil qu'il appelle** ;
//! - un `backend.json` : un scope `outil` par outil exposé, qui mentionne **son graphe** ;
//! - un `*.component.json` : un scope `composant`, qui mentionne **son composant TSX** (le nom
//!   déclaré, que la vue exporte), **ses effets** et **son graphe de clic** ;
//! - un `*.node.json` : un scope `noeud_script` au nom déclaré — celui qu'emploient les graphes.
//!
//! **Les noms des gabarits vivent dans un espace à eux** : `outil/<x>`, `graphe/<x>`,
//! `noeud/<graphe>.<x>`, `composant/<x>`, `noeud_script/<x>`, `backend/<x>`. Nus, un outil
//! `edit_file` ou un nœud `decide` devenaient des définisseurs de plus pour la fonction
//! `edit_file` ou la méthode `Decider.decide` du code, et la résolution par le nom, ambiguë,
//! laissait tomber 59 arêtes du code de rag3db (18 octobre 2026). Aucun symbole du code n'a la
//! forme `genre/nom`. La mention du type d'un nœud garde aussi le nom nu : c'est le lien vers le
//! type qui l'implémente (`RhaiNode`), une mention, jamais un définisseur.
//!
//! `usages outil/search_code` dit alors quels écrans et quels outils l'appellent. Le TSX qu'un
//! graphe de clic génère n'est jamais écrit dans un arbre (`dataflow::clic_tsx`) : il ne crée
//! aucun usage.
use crate::dataflow::parse_mermaid;
use serde_json::Value;

/// Un scope tiré d'un gabarit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeGabarit {
    pub name: String,
    /// `graphe`, `noeud`, `outil`, `backend`, `composant`, `noeud_script`.
    pub scope_type: String,
    pub start_line: usize,
    pub end_line: usize,
    /// L'indice du scope parent dans la liste, s'il y en a un.
    pub parent: Option<usize>,
    /// Les noms que ce scope mentionne, avec la ligne de la mention.
    pub mentions: Vec<(String, usize)>,
}

/// Le genre de gabarit d'un chemin, s'il en est un.
fn genre(rel: &str) -> Option<&'static str> {
    let nom = rel.rsplit('/').next().unwrap_or(rel);
    if nom.ends_with(".mmd") {
        Some("mmd")
    } else if nom == "backend.json" {
        Some("backend")
    } else if nom.ends_with(".component.json") {
        Some("composant")
    } else if nom.ends_with(".node.json") {
        Some("noeud_script")
    } else {
        None
    }
}

/// Le nom de fichier sans son suffixe de gabarit (`search_workspace.mmd` → `search_workspace`,
/// `fiche.clic.mmd` → `fiche.clic`).
fn tige(rel: &str, suffixe: &str) -> String {
    let nom = rel.rsplit('/').next().unwrap_or(rel);
    nom.strip_suffix(suffixe).unwrap_or(nom).to_string()
}

/// La première ligne (base un) où paraît `aiguille`, sinon 1.
fn ligne_de(content: &str, aiguille: &str) -> usize {
    content.lines().position(|l| l.contains(aiguille)).map(|i| i + 1).unwrap_or(1)
}

/// Les scopes d'un gabarit ; `None` si le chemin n'en est pas un, ou s'il ne se lit pas (il reste
/// alors un `texte_brut`, comme avant).
pub fn analyser_gabarit(rel: &str, content: &str) -> Option<Vec<ScopeGabarit>> {
    let total = content.lines().count().max(1);
    match genre(rel)? {
        "mmd" => {
            let def = parse_mermaid(content).ok()?;
            let graphe = tige(rel, ".mmd");
            let mut scopes = vec![ScopeGabarit {
                name: format!("graphe/{graphe}"),
                scope_type: "graphe".into(),
                start_line: 1,
                end_line: total,
                parent: None,
                mentions: Vec::new(),
            }];
            for n in &def.nodes {
                let ligne = ligne_de(content, &format!("{}[\"", n.name));
                let mut mentions = vec![(n.node_type.clone(), ligne), (format!("noeud_script/{}", n.node_type), ligne)];
                if n.node_type == "Effet" {
                    if let Some(Value::String(outil)) = n.config.get("outil") {
                        mentions.push((format!("outil/{outil}"), ligne));
                    }
                }
                scopes.push(ScopeGabarit {
                    name: format!("noeud/{graphe}.{}", n.name),
                    scope_type: "noeud".into(),
                    start_line: ligne,
                    end_line: ligne,
                    parent: Some(0),
                    mentions,
                });
            }
            Some(scopes)
        }
        "backend" => {
            let v: Value = serde_json::from_str(content).ok()?;
            let nom = v.get("name").and_then(Value::as_str).map(str::to_string).unwrap_or_else(|| tige(rel, ".json"));
            let mut scopes = vec![ScopeGabarit {
                name: format!("backend/{nom}"),
                scope_type: "backend".into(),
                start_line: 1,
                end_line: total,
                parent: None,
                mentions: Vec::new(),
            }];
            if let Some(Value::Object(outils)) = v.get("tools") {
                for (outil, decl) in outils {
                    let ligne = ligne_de(content, &format!("\"{outil}\""));
                    let mut mentions = Vec::new();
                    if let Some(Value::String(graphe)) = decl.get("graph") {
                        mentions.push((format!("graphe/{}", tige(graphe, ".mmd")), ligne));
                    }
                    scopes.push(ScopeGabarit {
                        name: format!("outil/{outil}"),
                        scope_type: "outil".into(),
                        start_line: ligne,
                        end_line: ligne,
                        parent: Some(0),
                        mentions,
                    });
                }
            }
            Some(scopes)
        }
        "composant" => {
            let v: Value = serde_json::from_str(content).ok()?;
            let nom = v.get("name").and_then(Value::as_str)?.to_string();
            let mut mentions = vec![(nom.clone(), ligne_de(content, "\"source\""))];
            if let Some(Value::Array(effets)) = v.get("effets") {
                let ligne = ligne_de(content, "\"effets\"");
                mentions.extend(effets.iter().filter_map(Value::as_str).map(|e| (format!("outil/{e}"), ligne)));
            }
            if let Some(Value::String(clic)) = v.get("clic") {
                mentions.push((format!("graphe/{}", tige(clic, ".mmd")), ligne_de(content, "\"clic\"")));
            }
            // Le nom déclaré est celui du composant TSX (sa vue), qu'il mentionne nu ; le scope,
            // lui, vit dans l'espace des gabarits.
            Some(vec![ScopeGabarit {
                name: format!("composant/{}", tige(rel, ".component.json")),
                scope_type: "composant".into(),
                start_line: 1,
                end_line: total,
                parent: None,
                mentions,
            }])
        }
        "noeud_script" => {
            let v: Value = serde_json::from_str(content).ok()?;
            let nom = v.get("name").and_then(Value::as_str)?.to_string();
            Some(vec![ScopeGabarit {
                name: format!("noeud_script/{nom}"),
                scope_type: "noeud_script".into(),
                start_line: 1,
                end_line: total,
                parent: None,
                mentions: Vec::new(),
            }])
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_graphe_de_clic_donne_un_scope_par_noeud_et_mentionne_l_outil() {
        let g = "graph LR\n    clic[\"Evenement(nom='onClick')\"]\n    chercher[\"Effet(outil='search_code')\"]\n    clic -->|charge:query| chercher\n";
        let s = analyser_gabarit("templates/x/components/fiche.clic.mmd", g).unwrap();
        assert_eq!(s[0].name, "graphe/fiche.clic");
        assert_eq!(s[0].scope_type, "graphe");
        let chercher = s.iter().find(|x| x.name == "noeud/fiche.clic.chercher").unwrap();
        assert_eq!((chercher.scope_type.as_str(), chercher.start_line, chercher.parent), ("noeud", 3, Some(0)));
        assert!(chercher.mentions.contains(&("outil/search_code".to_string(), 3)), "{:?}", chercher.mentions);
        assert!(chercher.mentions.contains(&("Effet".to_string(), 3)));
    }

    #[test]
    fn un_backend_donne_un_scope_par_outil_qui_mentionne_son_graphe() {
        let b = r#"{
  "name": "code",
  "tools": {
    "search_code": { "graph": "../../tools/search_workspace.mmd" },
    "read_file": { "graph": "graphs/read_file.mmd" }
  }
}"#;
        let s = analyser_gabarit("templates/backends/code/backend.json", b).unwrap();
        let outil = s.iter().find(|x| x.name == "outil/search_code").unwrap();
        assert_eq!(outil.scope_type, "outil");
        assert_eq!(outil.mentions, vec![("graphe/search_workspace".to_string(), 4)]);
    }

    #[test]
    fn un_composant_mentionne_sa_vue_ses_effets_et_son_graphe() {
        let c = r#"{
  "name": "FicheProduit",
  "source": "fiche_produit.tsx",
  "effets": ["search_code", "save_product"],
  "clic": "fiche_produit.clic.mmd"
}"#;
        let s = analyser_gabarit("components/fiche_produit.component.json", c).unwrap();
        assert_eq!(s.len(), 1);
        assert_eq!((s[0].name.as_str(), s[0].scope_type.as_str()), ("composant/fiche_produit", "composant"));
        let noms: Vec<&str> = s[0].mentions.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(noms, vec!["FicheProduit", "outil/search_code", "outil/save_product", "graphe/fiche_produit.clic"]);
    }

    #[test]
    fn un_noeud_scripte_se_nomme_comme_son_type() {
        let n = r#"{ "name": "product_form", "script": "product_form.ts", "inputs": {}, "outputs": {} }"#;
        let s = analyser_gabarit("templates/proto/boutique/nodes/product_form.node.json", n).unwrap();
        assert_eq!((s[0].name.as_str(), s[0].scope_type.as_str()), ("noeud_script/product_form", "noeud_script"));
    }

    #[test]
    fn un_autre_fichier_n_est_pas_un_gabarit_et_un_gabarit_illisible_reste_du_texte() {
        assert!(analyser_gabarit("README.md", "# x").is_none());
        assert!(analyser_gabarit("package.json", "{}").is_none());
        assert!(analyser_gabarit("x/backend.json", "{ pas du json").is_none());
        assert!(analyser_gabarit("y.mmd", "ceci n'est pas un graphe ->").is_none());
    }

    /// Bout à bout dans l'analyse d'un projet : le nœud d'effet mentionne l'outil, l'outil son
    /// graphe, le composant sa vue et ses effets ; chaque scope est dans son fichier, le nœud sous
    /// son graphe.
    #[test]
    fn l_analyse_d_un_projet_pose_les_scopes_et_les_mentions() {
        let fichiers = vec![
            (
                "backend.json".to_string(),
                r#"{ "name": "boutique", "tools": { "search_code": { "graph": "graphs/search_workspace.mmd" } } }"#.to_string(),
            ),
            ("graphs/search_workspace.mmd".to_string(), "graph LR\n    q[\"Spot(x=1)\"]\n".to_string()),
            (
                "components/fiche.component.json".to_string(),
                r#"{ "name": "Fiche", "source": "fiche.tsx", "effets": ["search_code"], "clic": "fiche.clic.mmd" }"#.to_string(),
            ),
            (
                "components/fiche.clic.mmd".to_string(),
                "graph LR\n    clic[\"Evenement(nom='onClick')\"]\n    chercher[\"Effet(outil='search_code')\"]\n    clic -->|charge:query| chercher\n".to_string(),
            ),
            ("components/fiche.tsx".to_string(), "export function Fiche() { return <b/>; }\n".to_string()),
        ];
        let a = crate::code::analyze_in_project("/virtuel", fichiers, "", None);
        let cle = |nom: &str, genre: &str| {
            a.scopes.iter().find(|s| s.name == nom && s.scope_type == genre).map(|s| s.key.clone()).unwrap_or_else(|| {
                panic!("pas de scope {genre} « {nom} » : {:?}", a.scopes.iter().map(|s| (&s.name, &s.scope_type)).collect::<Vec<_>>())
            })
        };
        let mentionne = |de: &str, nom: &str| a.pending.iter().any(|(k, n, r)| k == de && n == nom && r == "CONSUMES");
        let chercher = cle("noeud/fiche.clic.chercher", "noeud");
        assert!(mentionne(&chercher, "outil/search_code"), "{:?}", a.pending);
        assert!(mentionne(&cle("outil/search_code", "outil"), "graphe/search_workspace"), "{:?}", a.pending);
        let composant = cle("composant/fiche", "composant");
        for nom in ["Fiche", "outil/search_code", "graphe/fiche.clic"] {
            assert!(mentionne(&composant, nom), "{nom} : {:?}", a.pending);
        }
        let graphe = cle("graphe/fiche.clic", "graphe");
        assert!(a.relations.iter().any(|r| r.rel == "HAS_PARENT" && r.from_key == chercher && r.to_key == graphe));
        assert!(a.relations.iter().any(|r| r.rel == "DEFINED_IN" && r.from_key == chercher && r.to_key == "components/fiche.clic.mmd"));
        // La vue est du code (TSX), lue par codeparsers : son composant y est défini.
        assert!(a.scopes.iter().any(|s| s.name == "Fiche" && s.file_path == "components/fiche.tsx"));
    }

    /// **Un nom de gabarit ne devient pas un homonyme du code** : un outil `edit_file`, un nœud
    /// `decide` ne définissent pas `edit_file` ni `decide` ; seuls `outil/edit_file` et
    /// `noeud/<graphe>.decide`. Sans cet espace, la fonction `edit_file` d'à côté perdait ses
    /// arêtes par ambiguïté (59 sur rag3db, le 18 octobre).
    #[test]
    fn un_nom_de_gabarit_ne_fait_pas_d_homonyme_dans_le_code() {
        let fichiers = vec![
            ("backend.json".to_string(), r#"{ "name": "b", "tools": { "edit_file": { "graph": "g.mmd" } } }"#.to_string()),
            ("g.mmd".to_string(), "graph LR\n    decide[\"Decide(x=1)\"]\n".to_string()),
            ("outils.rs".to_string(), "pub fn edit_file() {}\npub fn decide() {}\n".to_string()),
            ("appel.rs".to_string(), "fn f() { edit_file(); decide(); }\n".to_string()),
        ];
        let a = crate::code::analyze_in_project("/virtuel", fichiers, "", None);
        let noms: Vec<&str> = a.scopes.iter().map(|s| s.name.as_str()).collect();
        for nu in ["edit_file", "decide"] {
            let definisseurs = a.scopes.iter().filter(|s| s.name == nu).count();
            assert_eq!(definisseurs, 1, "« {nu} » n'a que sa définition du code : {noms:?}");
        }
        assert!(noms.contains(&"outil/edit_file") && noms.contains(&"noeud/g.decide"), "{noms:?}");
    }
}
