//! **Le graphe de clic transpilé en TSX** (marche 3, niveau 2, lot 2 ; étude
//! `docs/10-octobre-2026-18h20/codeparsers/08-le-composant-declare.md` §2).
//!
//! Un graphe validé (`dataflow::clic`) devient un composant React, puis passe par
//! `script::tsx::compile_component` comme tout composant :
//!
//! - **les cases** sont l'état d'un `useReducer` ;
//! - **un tour est une action** : un `Evenement` (`evenement:<nom>`), la réponse ou l'erreur d'un
//!   `Effet` (`effet:<nœud>:reponse|erreur`), l'accord d'un `Confirmer` (`confirmer:<nœud>`). Le
//!   réducteur évalue les nœuds purs que le tour atteint, dans l'ordre du graphe, écrit les cases et
//!   **rend les effets comme des valeurs** ; il est pur, donc rejouable ;
//! - **les effets** sont lancés par un `useEffect` après le rendu (`/ide/call`, une confirmation, un
//!   changement de page) ; chaque demande porte son numéro de tour, et **une réponse dont le tour
//!   n'est plus le dernier demandé pour ce nœud est ignorée** ;
//! - **les données dérivées** (un `Pur` qui ne lit que des cases) se calculent au rendu et vont en
//!   props à la vue, avec les cases et un gestionnaire par événement : *le graphe câble, la vue
//!   affiche*.
//!
//! Le TSX généré n'est jamais écrit dans un arbre : c'est un produit en mémoire. Sa source map est
//! **composée** : module → TSX (le moteur `tsx`) puis TSX → nœud du `.clic.mmd` (le transpileur),
//! si bien qu'une erreur renvoie à la ligne du graphe. Une erreur de compilation (une expression
//! mal écrite) est rendue comme un refus à la ligne de son nœud.
use super::checkpoint::EdgeDef;
use super::clic::{valider_graphe_de_clic, ContexteClic, Genre, GrapheClic, NoeudClic, RefusClic};
use crate::script::tsx::{compile_component, CompiledComponent, ComponentMode, ModuleFormat};
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// Ce que le transpileur ne lit pas dans le graphe.
#[derive(Debug, Clone, Default)]
pub struct OptionsClic {
    /// Le manifeste du backend que les effets appellent (`/ide/call`, `manifest`).
    pub manifeste: String,
    /// La vue écrite à la main qui affiche le composant : (spécifier du module, nom exporté).
    /// Sans elle, le composant ne rend rien (le graphe reste éprouvable par son réducteur).
    pub vue: Option<(String, String)>,
}

/// Le TSX d'un graphe, et pour chaque ligne écrite le nœud du `.mmd` d'où elle vient.
#[derive(Debug, Clone)]
pub struct ClicTsx {
    pub tsx: String,
    /// Indexé par ligne du TSX (0 pour la première) : (ligne du `.mmd`, nom du nœud).
    pub origines: Vec<Option<(usize, String)>>,
}

/// Un graphe de clic compilé : le module, sa source map composée (vers le `.mmd`), et le TSX
/// intermédiaire (pour le déboguer, jamais écrit dans un arbre indexé).
#[derive(Debug, Clone)]
pub struct ClicCompile {
    pub module: CompiledComponent,
    pub tsx: String,
}

struct Ecrit {
    lignes: Vec<String>,
    origines: Vec<Option<(usize, String)>>,
}

impl Ecrit {
    fn ligne(&mut self, texte: impl Into<String>) {
        self.lignes.push(texte.into());
        self.origines.push(None);
    }
    fn du_noeud(&mut self, texte: impl Into<String>, n: &NoeudClic) {
        self.lignes.push(texte.into());
        self.origines.push(Some((n.ligne, n.nom.clone())));
    }
}

fn js(v: &str) -> String {
    serde_json::to_string(v).unwrap_or_else(|_| "\"\"".into())
}

/// Le nom JavaScript d'une valeur : `v_<nœud>` (les noms de nœud du `.mmd` sont des mots).
fn var(n: &str) -> String {
    let propre: String = n.chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' }).collect();
    format!("v_{propre}")
}

/// Transpile un graphe validé en TSX.
pub fn transpiler(graphe: &GrapheClic, nom: &str, options: &OptionsClic) -> ClicTsx {
    let par_nom: HashMap<&str, &NoeudClic> = graphe.noeuds.iter().map(|n| (n.nom.as_str(), n)).collect();
    let mut entrees: HashMap<&str, Vec<&EdgeDef>> = HashMap::new();
    let mut sorties: HashMap<&str, Vec<&EdgeDef>> = HashMap::new();
    for e in &graphe.aretes {
        entrees.entry(e.to_node.as_str()).or_default().push(e);
        sorties.entry(e.from_node.as_str()).or_default().push(e);
    }
    let purs: Vec<&NoeudClic> = graphe.noeuds.iter().filter(|n| matches!(n.genre, Genre::Pur { .. })).collect();
    let ordre = ordre_des_purs(&purs, &entrees);
    // Un Pur dérivé ne lit que des cases ou d'autres dérivés.
    let mut derives: BTreeSet<&str> = BTreeSet::new();
    for p in &ordre {
        let ok = entrees.get(p.as_str()).is_some_and(|es| {
            !es.is_empty()
                && es.iter().all(|e| match par_nom.get(e.from_node.as_str()).map(|n| &n.genre) {
                    Some(Genre::Etat { .. }) => true,
                    Some(Genre::Pur { .. }) => derives.contains(e.from_node.as_str()),
                    _ => false,
                })
        });
        if ok {
            derives.insert(p.as_str());
        }
    }

    let mut w = Ecrit { lignes: Vec::new(), origines: Vec::new() };
    w.ligne("// Généré depuis un graphe de clic (dataflow::clic_tsx) — jamais écrit dans un arbre indexé.");
    w.ligne("import { useReducer, useEffect, useMemo, useRef } from \"react\";");
    if let Some((module, export)) = &options.vue {
        w.ligne(format!("import {{ {export} as Vue }} from {};", js(module)));
    }
    w.ligne(format!("const MANIFESTE = {};", js(&options.manifeste)));

    // L'état initial : les cases.
    // `allumes` : les nœuds que le dernier tour a traversés, dans l'ordre — ce que l'aperçu
    // allume (`__rag3weaverClic`). Connus à la génération : chaque cas les écrit en dur, le
    // réducteur reste pur.
    w.ligne("const initial: any = { tour: 0, attente: {}, effets: [], allumes: [], cases: {");
    for n in &graphe.noeuds {
        if let Genre::Etat { nom, initial, .. } = &n.genre {
            w.du_noeud(format!("  {}: {},", js(nom), initial), n);
        }
    }
    w.ligne("} };");

    // La valeur que lit une entrée : une case (l'état d'avant), un dérivé, la charge du tour, un
    // Pur du tour. `None` : pas disponible dans ce tour.
    let valeur_de = |e: &EdgeDef, source: &str, actifs: &BTreeSet<&str>| -> Option<String> {
        let de = par_nom.get(e.from_node.as_str())?;
        match &de.genre {
            Genre::Etat { nom, .. } => Some(format!("avant[{}]", js(nom))),
            Genre::Pur { .. } if derives.contains(e.from_node.as_str()) => Some(format!("d[{}]", js(&e.from_node))),
            Genre::Pur { .. } if actifs.contains(e.from_node.as_str()) => Some(var(&e.from_node)),
            Genre::Evenement { .. } | Genre::Effet { .. } | Genre::Confirmer { .. } if e.from_node == source => {
                Some("action.charge".to_string())
            }
            _ => None,
        }
    };
    let expr_pur = |p: &NoeudClic, args: &[(String, String)]| -> String {
        let Genre::Pur { expr } = &p.genre else { unreachable!() };
        let params: Vec<&str> = args.iter().map(|(port, _)| port.as_str()).collect();
        let vals: Vec<&str> = args.iter().map(|(_, v)| v.as_str()).collect();
        format!("((({}) => ({expr}))({}))", params.join(", "), vals.join(", "))
    };

    // Les dérivés, calculés depuis les cases.
    w.ligne("function deriver(avant: any): any {");
    w.ligne("  const d: any = {};");
    for p in &ordre {
        if !derives.contains(p.as_str()) {
            continue;
        }
        let n = par_nom[p.as_str()];
        let args: Vec<(String, String)> = entrees[p.as_str()]
            .iter()
            .filter_map(|e| valeur_de(e, "", &BTreeSet::new()).map(|v| (e.to_port.clone(), v)))
            .collect();
        w.du_noeud(format!("  d[{}] = {};", js(p), expr_pur(n, &args)), n);
    }
    w.ligne("  return d;");
    w.ligne("}");

    // Le réducteur : un cas par source de tour.
    w.ligne("function reduire(e: any, action: any): any {");
    w.ligne("  const avant = e.cases;");
    w.ligne("  const d = deriver(avant);");
    w.ligne("  const tour = e.tour + 1;");
    w.ligne("  const cases: any = { ...avant };");
    w.ligne("  const attente: any = { ...e.attente };");
    w.ligne("  const effets: any[] = [];");
    w.ligne("  switch (action.type) {");
    let mut sources: Vec<(String, &NoeudClic, Option<&str>)> = Vec::new();
    for n in &graphe.noeuds {
        match &n.genre {
            Genre::Evenement { nom } => sources.push((format!("evenement:{nom}"), n, None)),
            Genre::Effet { .. } => {
                sources.push((format!("effet:{}:reponse", n.nom), n, Some("reponse")));
                sources.push((format!("effet:{}:erreur", n.nom), n, Some("erreur")));
            }
            Genre::Confirmer { .. } => sources.push((format!("confirmer:{}", n.nom), n, Some("confirme"))),
            _ => {}
        }
    }
    for (genre_action, source, port) in &sources {
        let mut allumes: Vec<&str> = vec![source.nom.as_str()];
        let mut cases_ecrites: BTreeSet<&str> = BTreeSet::new();
        w.du_noeud(format!("    case {}: {{", js(genre_action)), source);
        if matches!(source.genre, Genre::Effet { .. } | Genre::Confirmer { .. }) {
            w.du_noeud(
                format!("      if (action.tour !== e.attente[{}]) return e; // réponse obsolète", js(&source.nom)),
                source,
            );
        }
        // Les nœuds que le tour atteint, depuis le port de sa source.
        // Depuis la source, par son port (la réponse ou l'erreur d'un effet), puis à travers les
        // seuls nœuds purs : une case, un effet, une page, une confirmation arrêtent le tour.
        let mut atteints: BTreeSet<&str> = BTreeSet::new();
        let mut file: Vec<&str> = vec![source.nom.as_str()];
        while let Some(x) = file.pop() {
            for e in sorties.get(x).into_iter().flatten() {
                if x == source.nom && port.is_some_and(|p| p != e.from_port) {
                    continue;
                }
                let cible = e.to_node.as_str();
                let Some(cn) = par_nom.get(cible) else { continue };
                if atteints.insert(cible) && matches!(cn.genre, Genre::Pur { .. }) {
                    file.push(cible);
                }
            }
        }
        let mut actifs: BTreeSet<&str> = BTreeSet::new();
        for p in &ordre {
            if !atteints.contains(p.as_str()) || derives.contains(p.as_str()) {
                continue;
            }
            let es = &entrees[p.as_str()];
            let args: Option<Vec<(String, String)>> =
                es.iter().map(|e| valeur_de(e, &source.nom, &actifs).map(|v| (e.to_port.clone(), v))).collect();
            if let Some(args) = args {
                let n = par_nom[p.as_str()];
                w.du_noeud(format!("      const {} = {};", var(p), expr_pur(n, &args)), n);
                actifs.insert(p.as_str());
                allumes.push(p.as_str());
            }
        }
        // Les écritures, les effets demandés, les confirmations et les pages.
        for cible in &atteints {
            let n = par_nom[cible];
            let es: Vec<&&EdgeDef> = entrees[cible].iter().collect();
            let args: Vec<(String, String)> =
                es.iter().filter_map(|e| valeur_de(e, &source.nom, &actifs).map(|v| (e.to_port.clone(), v))).collect();
            let complet = args.len() == es.len();
            match &n.genre {
                Genre::Etat { nom, .. } => {
                    if let Some((_, v)) = args.iter().find(|(p, _)| p == "suivante") {
                        w.du_noeud(format!("      cases[{}] = {v};", js(nom)), n);
                        allumes.push(*cible);
                        cases_ecrites.insert(*cible);
                    }
                }
                Genre::Effet { outil, .. } if complet => {
                    allumes.push(*cible);
                    let a: Vec<String> = args.iter().map(|(p, v)| format!("{}: {v}", js(p))).collect();
                    w.du_noeud(format!("      attente[{}] = tour;", js(&n.nom)), n);
                    w.du_noeud(
                        format!(
                            "      effets.push({{ genre: \"effet\", noeud: {}, outil: {}, args: {{ {} }}, tour }});",
                            js(&n.nom),
                            js(outil),
                            a.join(", ")
                        ),
                        n,
                    );
                }
                Genre::Confirmer { message } if complet => {
                    allumes.push(*cible);
                    let charge = args.iter().find(|(p, _)| p == "demande").map(|(_, v)| v.clone()).unwrap_or("null".into());
                    w.du_noeud(format!("      attente[{}] = tour;", js(&n.nom)), n);
                    w.du_noeud(
                        format!(
                            "      effets.push({{ genre: \"confirmer\", noeud: {}, message: {}, charge: {charge}, tour }});",
                            js(&n.nom),
                            js(message)
                        ),
                        n,
                    );
                }
                Genre::Page { vers } if complet => {
                    allumes.push(*cible);
                    let p = args.iter().find(|(p, _)| p == "parametres").map(|(_, v)| v.clone()).unwrap_or("null".into());
                    w.du_noeud(format!("      effets.push({{ genre: \"page\", vers: {}, parametres: {p}, tour }});", js(vers)), n);
                }
                _ => {}
            }
        }
        // Les dérivés qu'une case écrite fait recalculer, dans leur ordre.
        let mut touches: BTreeSet<&str> = cases_ecrites.clone();
        for p in &ordre {
            if derives.contains(p.as_str()) && entrees[p.as_str()].iter().any(|e| touches.contains(e.from_node.as_str())) {
                touches.insert(p.as_str());
                allumes.push(p.as_str());
            }
        }
        let liste: Vec<String> = allumes.iter().map(|x| js(x)).collect();
        w.ligne(format!("      return {{ tour, attente, effets, cases, allumes: [{}] }};", liste.join(", ")));
        w.ligne("    }");
    }
    w.ligne("    default:");
    w.ligne("      return e;");
    w.ligne("  }");
    w.ligne("}");

    // Lancer les effets d'un tour.
    w.ligne("async function lancer(f: any, dispatch: (a: any) => void) {");
    w.ligne("  if (f.genre === \"effet\") {");
    w.ligne("    try {");
    w.ligne("      const r = await fetch(\"/ide/call\", { method: \"POST\", credentials: \"same-origin\", headers: { \"content-type\": \"application/json\" }, body: JSON.stringify({ manifest: MANIFESTE, name: f.outil, arguments: f.args }) });");
    w.ligne("      const corps = await r.json();");
    w.ligne("      if (!r.ok || corps.error) throw corps.error ?? corps;");
    w.ligne("      dispatch({ type: \"effet:\" + f.noeud + \":reponse\", tour: f.tour, charge: corps.result ?? corps });");
    w.ligne("    } catch (erreur) {");
    w.ligne("      dispatch({ type: \"effet:\" + f.noeud + \":erreur\", tour: f.tour, charge: erreur });");
    w.ligne("    }");
    w.ligne("  } else if (f.genre === \"confirmer\") {");
    w.ligne("    const confirmer = (globalThis as any).__rag3weaverConfirmer ?? ((m: string) => (globalThis as any).confirm(m));");
    w.ligne("    if (await confirmer(f.message)) dispatch({ type: \"confirmer:\" + f.noeud, tour: f.tour, charge: f.charge });");
    w.ligne("  } else if (f.genre === \"page\") {");
    w.ligne("    (globalThis as any).__rag3weaverNaviguer?.(f.vers, f.parametres);");
    w.ligne("  }");
    w.ligne("}");

    // Le composant.
    w.ligne(format!("export function {nom}(props: any) {{"));
    w.ligne("  const [etat, dispatch] = useReducer(reduire, initial);");
    // À chaque tour : ses effets, puis ce qu'il a traversé, dit à l'hôte de la page (l'aperçu
    // l'allume dans le dessin du graphe).
    // **Un tour ne se traite qu'une fois** : React Refresh rejoue les effets d'un composant
    // rechargé à chaud, ce qui relançait les effets du dernier tour (un second appel d'outil) et
    // le redisait à l'aperçu. Le dernier tour traité vit dans une ref, gardée au rechargement
    // comme l'état.
    w.ligne("  const traite = useRef(0);");
    w.ligne(format!(
        "  useEffect(() => {{ if (etat.tour === traite.current) return; traite.current = etat.tour; for (const f of etat.effets) lancer(f, dispatch); if (etat.tour > 0) (globalThis as any).__rag3weaverClic?.({}, etat.allumes, etat.tour); }}, [etat.tour]);",
        js(nom)
    ));
    w.ligne("  const derives = useMemo(() => deriver(etat.cases), [etat.cases]);");
    w.ligne("  const gestionnaires: any = {");
    for n in &graphe.noeuds {
        if let Genre::Evenement { nom: ev } = &n.genre {
            w.du_noeud(format!("    {}: (charge: any) => dispatch({{ type: {}, charge }}),", js(ev), js(&format!("evenement:{ev}"))), n);
        }
    }
    w.ligne("  };");
    if options.vue.is_some() {
        w.ligne("  return <Vue {...props} {...etat.cases} {...derives} {...gestionnaires} />;");
    } else {
        w.ligne("  return null;");
    }
    w.ligne("}");
    w.ligne(format!("{nom}.reduire = reduire;"));
    w.ligne(format!("{nom}.deriver = deriver;"));
    w.ligne(format!("{nom}.initial = initial;"));

    let mut tsx = w.lignes.join("\n");
    tsx.push('\n');
    ClicTsx { tsx, origines: w.origines }
}

/// L'ordre des nœuds purs : chacun après ceux qu'il lit (le validateur a refusé les cycles).
fn ordre_des_purs(purs: &[&NoeudClic], entrees: &HashMap<&str, Vec<&EdgeDef>>) -> Vec<String> {
    let noms: BTreeSet<&str> = purs.iter().map(|p| p.nom.as_str()).collect();
    let mut faits: Vec<String> = Vec::new();
    let mut reste: Vec<&str> = noms.iter().copied().collect();
    while !reste.is_empty() {
        let avant = reste.len();
        reste.retain(|p| {
            let pret = entrees.get(p).into_iter().flatten().all(|e| {
                !noms.contains(e.from_node.as_str()) || faits.iter().any(|f| f == &e.from_node)
            });
            if pret {
                faits.push(p.to_string());
            }
            !pret
        });
        if reste.len() == avant {
            faits.extend(reste.iter().map(|s| s.to_string()));
            break;
        }
    }
    faits
}

/// Valide, transpile et compile un graphe de clic. `source` nomme le `.clic.mmd` (il paraît dans
/// la source map) ; les refus, de validation comme de compilation, disent la ligne du graphe.
pub fn compiler_graphe_de_clic(
    texte: &str,
    source: &str,
    nom: &str,
    template_id: &str,
    ctx: &ContexteClic,
    options: &OptionsClic,
    mode: ComponentMode,
    format: ModuleFormat,
) -> Result<ClicCompile, Vec<RefusClic>> {
    let graphe = valider_graphe_de_clic(texte, ctx)?;
    let ClicTsx { tsx, origines } = transpiler(&graphe, nom, options);
    let origine_de = |ligne_tsx: u32| -> Option<&(usize, String)> {
        (ligne_tsx as usize).checked_sub(1).and_then(|i| origines.get(i)).and_then(|o| o.as_ref())
    };
    let mut module = compile_component(&tsx, template_id, mode, format).map_err(|e| {
        let (ligne, noeud) = match e.line.and_then(origine_de) {
            Some((l, n)) => (Some(*l), Some(n.clone())),
            None => (None, None),
        };
        vec![RefusClic {
            ligne,
            message: match noeud {
                Some(n) => format!("{n} : le TSX généré ne se compile pas : {}", e.message),
                None => format!("le TSX généré ne se compile pas : {}", e.message),
            },
        }]
    })?;
    module.source_map = composer(&module.source_map, &origines, source).unwrap_or_default();
    Ok(ClicCompile { module, tsx })
}

/// Compose la carte module → TSX avec TSX → `.mmd` : chaque position du module qui vient d'une
/// ligne écrite pour un nœud renvoie à la ligne de ce nœud, et porte son nom.
fn composer(carte: &str, origines: &[Option<(usize, String)>], source: &str) -> Option<String> {
    let lue = swc_sourcemap::SourceMap::from_slice(carte.as_bytes()).ok()?;
    let mut b = swc_sourcemap::SourceMapBuilder::new(None);
    let mut vus: BTreeMap<(u32, u32), ()> = BTreeMap::new();
    for t in lue.tokens() {
        let src = t.get_src_line();
        if src == u32::MAX {
            continue;
        }
        let Some(Some((ligne, noeud))) = origines.get(src as usize) else { continue };
        let cle = (t.get_dst_line(), t.get_dst_col());
        if vus.insert(cle, ()).is_some() {
            continue;
        }
        b.add(
            t.get_dst_line(),
            t.get_dst_col(),
            (*ligne as u32).saturating_sub(1),
            0,
            Some(source.to_string().into()),
            Some(noeud.clone().into()),
            false,
        );
    }
    let mut sortie = Vec::new();
    b.into_sourcemap().to_writer(&mut sortie).ok()?;
    String::from_utf8(sortie).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataflow::clic::Ecran;

    fn ctx() -> ContexteClic {
        ContexteClic {
            effets: vec!["search_code".into()],
            outils: BTreeMap::from([("search_code".to_string(), Some(Ecran::Lecture))]),
            etat_hote: None,
        }
    }

    const COMPTEUR: &str = r#"graph LR
    clic["Evenement(nom='onClick')"]
    n["Etat(nom='n', schema='{"type":"number"}', initial=0)"]
    plus["Pur(expr='n + 1')"]
    double["Pur(expr='n * 2')"]
    clic -->|charge:declenche| plus
    n -->|valeur:n| plus
    plus -->|resultat:suivante| n
    n -->|valeur:n| double
"#;

    const FILTRE: &str = r#"graph LR
    saisie["Evenement(nom='onSaisie')"]
    filtre["Etat(nom='filtre', schema='{"type":"string"}', initial='""')"]
    liste["Etat(nom='liste', schema='{"type":"array"}', initial='[]')"]
    chercher["Effet(outil='search_code')"]
    saisie -->|charge:suivante| filtre
    saisie -->|charge:query| chercher
    chercher -->|reponse:suivante| liste
"#;

    fn compiler(texte: &str) -> ClicCompile {
        compiler_graphe_de_clic(
            texte,
            "components/x.clic.mmd",
            "Compteur",
            "boutique/x",
            &ctx(),
            &OptionsClic { manifeste: "/m/backend.json".into(), vue: None },
            ComponentMode::Production,
            ModuleFormat::CommonJs,
        )
        .unwrap_or_else(|r| panic!("{}", r.iter().map(|x| x.to_string()).collect::<Vec<_>>().join("\n")))
    }

    /// Exécute le module CommonJS dans QuickJS (React factice) puis `scenario`, qui voit
    /// `C` (le composant exporté) et rend une chaîne.
    fn executer(code: &str, scenario: &str) -> String {
        let rt = rquickjs::Runtime::new().unwrap();
        let ctx = rquickjs::Context::full(&rt).unwrap();
        ctx.with(|ctx| {
            let script = format!(
                r#"
                const libs = {{ "react": {{ useReducer: () => null, useEffect: () => null, useMemo: (f) => f() }},
                               "react/jsx-runtime": {{ jsx: () => null, jsxs: () => null, Fragment: "F" }} }};
                const module = {{ exports: {{}} }};
                (function (require, module, exports) {{
                {code}
                }})((s) => libs[s], module, module.exports);
                const C = module.exports.Compteur;
                {scenario}
                "#
            );
            ctx.eval::<String, _>(script).unwrap()
        })
    }

    #[test]
    fn un_compteur_compte_et_derive() {
        let c = compiler(COMPTEUR);
        let r = executer(
            &c.module.code,
            r#"let e = C.initial;
               e = C.reduire(e, { type: "evenement:onClick", charge: null });
               e = C.reduire(e, { type: "evenement:onClick", charge: null });
               JSON.stringify({ n: e.cases.n, tour: e.tour, double: C.deriver(e.cases).double, effets: e.effets.length })"#,
        );
        assert_eq!(r, r#"{"n":2,"tour":2,"double":4,"effets":0}"#, "TSX :\n{}", c.tsx);
    }

    /// **Un tour dit ce qu'il a traversé** (`allumes`), pour que l'aperçu l'allume : la source,
    /// les purs du tour dans leur ordre, les cases écrites, puis les dérivés qu'elles font
    /// recalculer. Une réponse d'effet allume l'effet et ce qu'elle écrit.
    #[test]
    fn un_tour_dit_ce_qu_il_a_traverse() {
        let c = compiler(COMPTEUR);
        let r = executer(
            &c.module.code,
            r#"let e = C.reduire(C.initial, { type: "evenement:onClick", charge: null });
               JSON.stringify({ initial: C.initial.allumes, allumes: e.allumes })"#,
        );
        assert_eq!(r, r#"{"initial":[],"allumes":["clic","plus","n","double"]}"#, "TSX :\n{}", c.tsx);
        let c = compiler(FILTRE);
        let r = executer(
            &c.module.code,
            r#"let e = C.reduire(C.initial, { type: "evenement:onSaisie", charge: "a" });
               const saisie = e.allumes;
               e = C.reduire(e, { type: "effet:chercher:reponse", tour: e.effets[0].tour, charge: ["x"] });
               JSON.stringify({ saisie, reponse: e.allumes })"#,
        );
        let v: serde_json::Value = serde_json::from_str(&r).unwrap();
        assert_eq!(v["saisie"][0], "saisie", "{r}\nTSX :\n{}", c.tsx);
        assert!(v["saisie"].as_array().unwrap().iter().any(|x| x == "chercher"), "{r}");
        assert_eq!(v["reponse"][0], "chercher", "{r}");
        assert!(v["reponse"].as_array().unwrap().len() >= 2, "la réponse écrit une case : {r}");
    }

    #[test]
    fn une_reponse_obsolete_est_ignoree() {
        let c = compiler(FILTRE);
        let r = executer(
            &c.module.code,
            r#"let e = C.initial;
               e = C.reduire(e, { type: "evenement:onSaisie", charge: "a" });
               const premier = e.effets[0];
               e = C.reduire(e, { type: "evenement:onSaisie", charge: "ab" });
               const second = e.effets[0];
               const avant = e;
               const ignoree = C.reduire(e, { type: "effet:chercher:reponse", tour: premier.tour, charge: ["vieux"] });
               e = C.reduire(e, { type: "effet:chercher:reponse", tour: second.tour, charge: ["neuf"] });
               JSON.stringify({ ignoree: ignoree === avant, args: second.args, filtre: avant.cases.filtre, liste: e.cases.liste, outil: second.outil })"#,
        );
        assert_eq!(
            r,
            r#"{"ignoree":true,"args":{"query":"ab"},"filtre":"ab","liste":["neuf"],"outil":"search_code"}"#,
            "TSX :\n{}",
            c.tsx
        );
    }

    #[test]
    fn une_expression_mal_ecrite_est_refusee_a_la_ligne_de_son_noeud() {
        let mauvais = COMPTEUR.replace("expr='n * 2'", "expr='n * (2'");
        let r = compiler_graphe_de_clic(
            &mauvais,
            "components/x.clic.mmd",
            "Compteur",
            "boutique/x",
            &ctx(),
            &OptionsClic::default(),
            ComponentMode::Production,
            ModuleFormat::Esm,
        )
        .unwrap_err();
        assert_eq!(r[0].ligne, Some(5), "{r:?}");
        assert!(r[0].message.contains("double"), "{r:?}");
    }

    #[test]
    fn la_source_map_renvoie_au_noeud_du_graphe() {
        let c = compiler(COMPTEUR);
        let carte = swc_sourcemap::SourceMap::from_slice(c.module.source_map.as_bytes()).unwrap();
        assert_eq!(carte.get_source(0).map(|s| s.to_string()).as_deref(), Some("components/x.clic.mmd"));
        let noms: BTreeSet<String> = carte.tokens().filter_map(|t| t.get_name().map(|n| n.to_string())).collect();
        assert!(noms.contains("plus") && noms.contains("double") && noms.contains("n"), "{noms:?}");
        // Le jeton du nœud `plus` vise sa ligne du graphe (4, soit 3 en base zéro).
        assert!(carte.tokens().any(|t| t.get_name().map(|n| n.to_string()).as_deref() == Some("plus") && t.get_src_line() == 3));
    }

    #[test]
    fn le_tsx_genere_appelle_l_hote_et_rend_la_vue() {
        let c = compiler_graphe_de_clic(
            FILTRE,
            "components/x.clic.mmd",
            "Filtre",
            "boutique/x",
            &ctx(),
            &OptionsClic { manifeste: "/m/backend.json".into(), vue: Some(("./filtre.vue".into(), "FiltreVue".into())) },
            ComponentMode::Development,
            ModuleFormat::Esm,
        )
        .unwrap();
        assert!(c.tsx.contains("fetch(\"/ide/call\""), "{}", c.tsx);
        assert!(c.module.imports.contains(&"./filtre.vue".to_string()), "{:?}", c.module.imports);
        assert!(c.module.code.contains("$RefreshReg$("), "le composant généré est rechargé à chaud comme un autre");
        assert!(c.module.exports_only_components);
    }
}
