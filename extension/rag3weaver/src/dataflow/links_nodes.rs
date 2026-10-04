//! **Comment des résultats se relient** : entre les choses qu'une recherche
//! a rendues, les plus courts chemins du graphe, à quatre sauts au plus —
//! « A appelle X, qu'appelle aussi B ». Une section qui dit pourquoi des
//! résultats vont ensemble, ou qui se tait.
//!
//! **Une marche en largeur à plusieurs départs**, par niveaux, sur le moteur
//! du voisinage ([`super::graph_walk`]) : chaque résultat est un départ, tous
//! avancent d'un saut à chaque tour, dans les deux sens, et deux départs qui
//! atteignent le même nœud sont reliés par lui. Le point de rencontre d'un
//! chemin de longueur `d` est à `⌈d/2⌉` sauts d'un bout : deux tours
//! suffisent pour quatre sauts. Par paire, on garde la rencontre la plus
//! courte — le plus court chemin du graphe privé de ses carrefours ;
//! `SHORTEST` du moteur sert d'oracle au test.
//!
//! **Un carrefour n'explique rien** : « les deux appellent `len` » est vrai
//! et vide. Un nœud plus connecté que `max_degree` (les deux sens, toutes
//! relations suivies) n'est ni traversé ni point de rencontre — sauf s'il
//! est lui-même un résultat.
//!
//! **Le nœud est générique** : les relations suivies, les champs de titre et
//! de lieu, les libellés des relations sont déclarés par le gabarit. Les
//! départs viennent du port `results` ou de la config `uuids` (un crochet
//! après outil les reçoit en argument). Rien à dire : un texte vide — le
//! crochet se tait.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, Mutex};

use serde::Serialize;

use super::graph_walk::{degrees, neighbors, text, uuid_param, Direction};
use super::node::{Node, NodeContext};
use super::node_registry::{Choices, ConfigParam, ConfigParamType, NodeFactory, NodeSchema};
use super::port::{take_or_clone, PortDef, PortType, PortValue};
use super::usage_nodes::{rel_info, RelInfo};
use crate::catalog::Catalog;
use crate::search_strategy::UnifiedResult;

pub const MAX_HOPS: usize = 4;
pub const MAX_SOURCES: usize = 20;

/// Ce que le gabarit déclare.
#[derive(Debug, Clone)]
pub struct LinksConfig {
    /// L'entité des départs (les résultats).
    pub entity: String,
    /// Les relations suivies, dans les deux sens.
    pub relations: Vec<String>,
    /// `REL` → libellé (« appelle ») ; sans libellé, le nom de la relation.
    pub labels: BTreeMap<String, String>,
    pub title: String,
    pub path_fields: Vec<String>,
    pub line_field: String,
    /// Le champ du genre, montré entre parenthèses (vide : rien).
    pub kind_field: String,
    pub max_hops: usize,
    pub max_links: usize,
    pub max_degree: usize,
    pub sources: usize,
}

/// Une arête d'un chemin, dans son vrai sens.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub relation: String,
}

/// Un nœud d'un chemin.
#[derive(Debug, Clone, Serialize, Default)]
pub struct Stop {
    pub uuid: String,
    pub title: String,
    pub path: String,
    pub line: Option<i64>,
    /// Le genre (`scope_type`), quand le gabarit déclare son champ.
    pub kind: String,
}

/// Un lien entre deux résultats : le chemin, du premier au second.
#[derive(Debug, Clone, Serialize)]
pub struct Link {
    /// Rangs des deux résultats dans la liste d'entrée.
    pub a: usize,
    pub b: usize,
    pub stops: Vec<Stop>,
    /// `edges[i]` relie `stops[i]` et `stops[i + 1]`, dans un sens ou l'autre.
    pub edges: Vec<Edge>,
}

impl Link {
    pub fn hops(&self) -> usize {
        self.edges.len()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct LinksReport {
    pub links: Vec<Link>,
    /// Paires reliées au-delà de `max_links`, non rendues.
    pub cut: usize,
    /// Les titres que portent plusieurs résultats : au rendu, ces bouts
    /// disent leur fichier — sinon deux `from_bytes` se confondent.
    pub homonyms: Vec<String>,
    /// Les départs (les résultats de la recherche) : au rendu, un nom qui
    /// n'en est pas — un intermédiaire — se distingue.
    pub sources: Vec<String>,
}

/// L'arrivée d'un départ sur un nœud : sa distance, et d'où il vient.
#[derive(Clone)]
struct Label {
    dist: usize,
    parent: Option<(String, Edge)>,
}

/// Les liens entre `sources`, à `max_hops` sauts au plus.
pub fn links_between(catalog: &Catalog, cfg: &LinksConfig, sources: &[String]) -> Result<LinksReport, String> {
    let err = |e: String| format!("LinksNode: {e}");
    let rels: Vec<RelInfo> = cfg.relations.iter().map(|r| rel_info(catalog, r)).collect::<Result<_, _>>().map_err(err)?;
    let mut vus_src: HashSet<&str> = HashSet::new();
    let sources: Vec<String> = sources.iter().filter(|u| !u.is_empty() && vus_src.insert(u.as_str())).take(cfg.sources).cloned().collect();
    let rang: HashMap<&str, usize> = sources.iter().enumerate().map(|(i, u)| (u.as_str(), i)).collect();
    if sources.len() < 2 {
        return Ok(LinksReport { links: Vec::new(), cut: 0, homonyms: Vec::new(), sources: Vec::new() });
    }
    let both = [Direction::Outgoing, Direction::Incoming];
    // node → (départ → arrivée)
    let mut labels: HashMap<String, HashMap<usize, Label>> = HashMap::new();
    let mut frontier: Vec<Vec<String>> = sources.iter().map(|u| vec![u.clone()]).collect();
    for (i, u) in sources.iter().enumerate() {
        labels.entry(u.clone()).or_default().insert(i, Label { dist: 0, parent: None });
    }
    // Un départ trop connecté n'est pas traversé non plus : il reste un
    // bout possible, pas un passage.
    let deg0 = degrees(catalog, &rels, &both, &sources).map_err(err)?;
    let mut carrefours: HashSet<String> = sources.iter().filter(|u| deg0.get(*u).copied().unwrap_or(0) > cfg.max_degree).cloned().collect();
    let tours = cfg.max_hops.div_ceil(2);
    for tour in 1..=tours {
        let a_suivre: Vec<String> = {
            let mut s: Vec<String> = frontier.iter().flatten().filter(|u| !carrefours.contains(*u)).cloned().collect();
            s.sort();
            s.dedup();
            s
        };
        if a_suivre.is_empty() {
            break;
        }
        // Les voisins de toute la frontière, une requête par relation et par sens.
        let mut voisins: HashMap<String, Vec<(String, Edge)>> = HashMap::new();
        for rel in &rels {
            for &direction in &both {
                for (de, vers) in neighbors(catalog, rel, direction, &a_suivre).map_err(err)? {
                    let edge = match direction {
                        Direction::Outgoing => Edge { from: de.clone(), to: vers.clone(), relation: rel.name.clone() },
                        Direction::Incoming => Edge { from: vers.clone(), to: de.clone(), relation: rel.name.clone() },
                    };
                    voisins.entry(de).or_default().push((vers, edge));
                }
            }
        }
        let mut neufs: Vec<(String, usize, Label)> = Vec::new();
        for (i, f) in frontier.iter().enumerate() {
            for u in f {
                let Some(vs) = voisins.get(u) else { continue };
                for (v, edge) in vs {
                    if labels.get(v).is_some_and(|l| l.contains_key(&i)) || neufs.iter().any(|(n, j, _)| n == v && *j == i) {
                        continue;
                    }
                    neufs.push((v.clone(), i, Label { dist: tour, parent: Some((u.clone(), edge.clone())) }));
                }
            }
        }
        // Les carrefours atteints : ni passage ni rencontre, sauf un résultat.
        let atteints: Vec<String> = {
            let mut s: Vec<String> = neufs.iter().map(|(n, _, _)| n.clone()).filter(|n| !rang.contains_key(n.as_str())).collect();
            s.sort();
            s.dedup();
            s
        };
        let deg = degrees(catalog, &rels, &both, &atteints).map_err(err)?;
        carrefours.extend(atteints.into_iter().filter(|n| deg.get(n).copied().unwrap_or(0) > cfg.max_degree));
        frontier = vec![Vec::new(); sources.len()];
        for (n, i, l) in neufs {
            if carrefours.contains(&n) && !rang.contains_key(n.as_str()) {
                continue;
            }
            frontier[i].push(n.clone());
            labels.entry(n).or_default().insert(i, l);
        }
    }

    // Par paire, la rencontre la plus courte ; à égalité, l'ordre des nœuds.
    let mut meilleurs: BTreeMap<(usize, usize), (usize, String)> = BTreeMap::new();
    let mut noeuds: Vec<&String> = labels.keys().collect();
    noeuds.sort();
    for n in noeuds {
        let l = &labels[n];
        let mut origines: Vec<&usize> = l.keys().collect();
        origines.sort();
        for (x, &&a) in origines.iter().enumerate() {
            for &&b in &origines[x + 1..] {
                let d = l[&a].dist + l[&b].dist;
                if d == 0 || d > cfg.max_hops {
                    continue;
                }
                let e = meilleurs.entry((a, b)).or_insert((d, n.clone()));
                if d < e.0 {
                    *e = (d, n.clone());
                }
            }
        }
    }
    let mut paires: Vec<((usize, usize), (usize, String))> = meilleurs.into_iter().collect();
    // Les plus courts d'abord, puis les résultats les mieux classés.
    paires.sort_by_key(|((a, b), (d, _))| (*d, a + b, *a));

    let remonter = |depuis: &str, i: usize| -> Vec<(String, Option<Edge>)> {
        // Du nœud de rencontre au départ `i` : [(nœud, arête vers le suivant)].
        let mut out = vec![(depuis.to_string(), None)];
        let mut cur = depuis.to_string();
        while let Some((p, e)) = labels.get(&cur).and_then(|l| l.get(&i)).and_then(|l| l.parent.clone()) {
            out.last_mut().unwrap().1 = Some(e);
            out.push((p.clone(), None));
            cur = p;
        }
        out
    };
    let mut links = Vec::new();
    for ((a, b), (_, n)) in paires {
        // a … n, puis n … b.
        let mut vers_a = remonter(&n, a);
        vers_a.reverse();
        let vers_b = remonter(&n, b);
        let mut stops: Vec<String> = vers_a.iter().map(|(u, _)| u.clone()).collect();
        // Les arêtes de a vers n : chaque élément de vers_a (inversé) porte
        // l'arête qui le relie au précédent.
        let mut edges: Vec<Edge> = vers_a.iter().filter_map(|(_, e)| e.clone()).collect();
        for (u, _) in vers_b.iter().skip(1) {
            stops.push(u.clone());
        }
        edges.extend(vers_b.iter().filter_map(|(_, e)| e.clone()));
        links.push((a, b, stops, edges));
    }

    // Les fiches des nœuds rendus et de tous les résultats, table par table.
    let mut uuids: Vec<String> = links.iter().flat_map(|(_, _, s, _)| s.clone()).chain(sources.iter().cloned()).collect();
    uuids.sort();
    uuids.dedup();
    let fiches = fiches(catalog, cfg, &rels, &uuids).map_err(err)?;
    let links: Vec<Link> = links
        .into_iter()
        .map(|(a, b, stops, edges)| Link {
            a,
            b,
            stops: stops.iter().map(|u| fiches.get(u).cloned().unwrap_or(Stop { uuid: u.clone(), ..Default::default() })).collect(),
            edges,
        })
        // Deux scopes de même nom dans le même fichier sont une seule chose
        // (une `struct` et son `impl`, une déclaration et sa définition) :
        // ni lien entre eux, ni chemin qui passe de l'un à l'autre.
        .filter(|l| {
            let mut vus: HashSet<(&str, &str)> = HashSet::new();
            l.stops.iter().filter(|s| !s.title.is_empty() && !s.path.is_empty()).all(|s| vus.insert((s.title.as_str(), s.path.as_str())))
        })
        .collect();
    // Et une même paire de choses ne fait qu'une ligne : deux résultats
    // homonymes d'un même fichier (la struct et son impl) reliés au même
    // tiers, ou deux chemins entre les mêmes bouts — le plus court, venu
    // en premier, reste.
    let bout = |s: &Stop| if s.title.is_empty() { (s.uuid.clone(), String::new()) } else { (s.title.clone(), s.path.clone()) };
    let mut deja: HashSet<((String, String), (String, String))> = HashSet::new();
    let links: Vec<Link> = links
        .into_iter()
        .filter(|l| {
            let (a, b) = (bout(&l.stops[0]), bout(&l.stops[l.stops.len() - 1]));
            let cle = if a <= b { (a, b) } else { (b, a) };
            deja.insert(cle)
        })
        .collect();
    let cut = links.len().saturating_sub(cfg.max_links);
    let links: Vec<Link> = links.into_iter().take(cfg.max_links).collect();
    let mut par_titre: HashMap<&str, usize> = HashMap::new();
    for u in &sources {
        if let Some(f) = fiches.get(u) {
            *par_titre.entry(f.title.as_str()).or_default() += 1;
        }
    }
    let mut homonyms: Vec<String> = par_titre.into_iter().filter(|(t, n)| *n > 1 && !t.is_empty()).map(|(t, _)| t.to_string()).collect();
    homonyms.sort();
    Ok(LinksReport { links, cut, homonyms, sources: sources.clone() })
}

/// Titre et lieu de chaque nœud, dans toutes les tables que les relations
/// suivies touchent.
fn fiches(catalog: &Catalog, cfg: &LinksConfig, rels: &[RelInfo], uuids: &[String]) -> Result<HashMap<String, Stop>, String> {
    let mut tables: Vec<&str> = vec![cfg.entity.as_str()];
    for r in rels {
        tables.push(&r.from);
        tables.push(&r.to);
    }
    tables.sort();
    tables.dedup();
    let mut out = HashMap::new();
    if uuids.is_empty() {
        return Ok(out);
    }
    for t in tables {
        let mut champs = vec![format!("m.{}", cfg.title)];
        champs.extend(cfg.path_fields.iter().map(|p| format!("m.{p}")));
        champs.push(format!("m.{}", cfg.line_field));
        champs.push(if cfg.kind_field.is_empty() { "NULL".into() } else { format!("m.{}", cfg.kind_field) });
        let q = format!("UNWIND $uuids AS u MATCH (m:{t} {{_uuid: u}}) RETURN u, {}", champs.join(", "));
        // Une table qui n'a pas ces champs n'a pas de fiche : le nœud garde
        // son uuid, le lien reste vrai.
        let Ok(rows) = catalog.execute_raw_with_params(&q, &[uuid_param(uuids)]) else { continue };
        let n = cfg.path_fields.len();
        for r in &rows.rows {
            let uuid = text(r.first());
            let path = (0..n).map(|i| text(r.get(2 + i))).find(|p| !p.is_empty()).unwrap_or_default();
            out.insert(uuid.clone(), Stop { uuid, title: text(r.get(1)), path, line: r.get(2 + n).and_then(|v| v.as_i64()), kind: text(r.get(3 + n)) });
        }
    }
    Ok(out)
}

impl LinksReport {
    /// **Le rendu en arbre**, la forme du « Dependency Graph » des résultats
    /// et du « Graphe » de `grep` ([`crate::arbre`]) : les liens regroupés
    /// autour du nom le plus relié (le pivot), qui n'apparaît qu'une fois ;
    /// une branche par relation entre crochets, le sens dit par son nom
    /// (`labels` : `REL=sortant/entrant`, `CONSUMES/CONSUMED_BY`) ; les
    /// voisins dessous, `nom (genre)`. Le lieu (`@ fichier:ligne`) sur le
    /// pivot, et sur un voisin seulement s'il n'est pas un résultat de la
    /// recherche — un intermédiaire, qui le dit (`· hors résultats`). Rien
    /// quand il n'y a rien à dire.
    pub fn markdown(&self, cfg: &LinksConfig) -> String {
        if self.links.is_empty() {
            return String::new();
        }
        let mut fiche: HashMap<&str, &Stop> = HashMap::new();
        let mut aretes: Vec<&Edge> = Vec::new();
        let mut ordre: Vec<&str> = Vec::new();
        for l in &self.links {
            for s in &l.stops {
                fiche.entry(s.uuid.as_str()).or_insert(s);
                if !ordre.contains(&s.uuid.as_str()) {
                    ordre.push(s.uuid.as_str());
                }
            }
            for e in &l.edges {
                if !aretes.iter().any(|x| x.from == e.from && x.to == e.to && x.relation == e.relation) {
                    aretes.push(e);
                }
            }
        }
        let resultats: HashSet<&str> = self.sources.iter().map(String::as_str).collect();
        let lieu = |s: &Stop| match s.line {
            Some(n) if !s.path.is_empty() => format!("{}:{n}", s.path),
            _ => s.path.clone(),
        };
        // `nom (genre)`, le lieu si demandé, et la marque d'un intermédiaire.
        let decrire = |u: &str, avec_lieu: bool| -> String {
            let Some(s) = fiche.get(u) else { return u.to_string() };
            let mut t = if s.title.is_empty() { s.uuid.clone() } else { s.title.clone() };
            if !s.kind.is_empty() {
                t.push_str(&format!(" ({})", s.kind));
            }
            let ou = lieu(s);
            if avec_lieu && !ou.is_empty() {
                t.push_str(&format!(" @ {ou}"));
            }
            if !resultats.contains(u) {
                t.push_str(" · hors résultats");
            }
            t
        };
        let sens = |e: &Edge, sortant: bool| -> String {
            match cfg.labels.get(&e.relation).and_then(|l| l.split_once('/')) {
                Some((a, p)) => if sortant { a.trim().to_string() } else { p.trim().to_string() },
                None => if sortant { e.relation.clone() } else { format!("{} (entrant)", e.relation) },
            }
        };
        let mut restantes: Vec<&Edge> = aretes.clone();
        let mut out = String::from("```\n");
        while !restantes.is_empty() {
            let degre = |u: &str| restantes.iter().filter(|e| e.from == u || e.to == u).count();
            let pivot = *ordre.iter().max_by_key(|u| (degre(u), std::cmp::Reverse(ordre.iter().position(|x| x == *u)))).unwrap();
            let siennes: Vec<&Edge> = restantes.iter().copied().filter(|e| e.from == pivot || e.to == pivot).collect();
            restantes.retain(|e| !(e.from == pivot || e.to == pivot));
            let mut groupes: Vec<(String, Vec<String>)> = Vec::new();
            for e in siennes {
                let (autre, sortant) = if e.from == pivot { (e.to.as_str(), true) } else { (e.from.as_str(), false) };
                let rel = sens(e, sortant);
                let voisin = decrire(autre, !resultats.contains(autre));
                match groupes.iter_mut().find(|(r, _)| *r == rel) {
                    Some((_, v)) => v.push(voisin),
                    None => groupes.push((rel, vec![voisin])),
                }
            }
            out.push_str(&crate::arbre::arbre(&decrire(pivot, true), &groupes));
        }
        out.push_str("```\n");
        if self.cut > 0 {
            out.push_str(&format!("_… et {} autres paires reliées._\n", self.cut));
        }
        out
    }
}

// ─── Le nœud ─────────────────────────────────────────────────────────────────

/// Liens entre résultats : **input** `results` (facultatif), **output**
/// `result` (PortType::Map), markdown ou JSON. Service : `catalog`.
pub struct LinksNode {
    node_name: String,
    cfg: LinksConfig,
    uuids: Vec<String>,
    json: bool,
}

impl Node for LinksNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "LinksNode"
    }
    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        Some(Box::new(serde_json::json!({ "uuids": self.uuids, "relations": self.cfg.relations })))
    }
    fn inputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&LinksNodeFactory).0
    }
    fn outputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&LinksNodeFactory).1
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        use super::catalog_read::{read_catalog, CatalogRead};
        let mut sources = self.uuids.clone();
        if let Some(results) = ctx.take_input("results").and_then(take_or_clone::<Vec<UnifiedResult>>) {
            // Le balayage des fichiers n'a pas de nœud dans le graphe.
            sources.extend(results.into_iter().filter(|r| !r.uuid.starts_with("scan:")).map(|r| r.uuid));
        }
        let catalog = ctx.service::<Arc<Mutex<Catalog>>>("catalog").cloned().ok_or("LinksNode: service 'catalog' absent")?;
        // Une section qui accompagne un autre rendu : occupé ou jamais
        // indexé, elle se tait — l'outil qu'elle suit l'a déjà dit.
        let report = match read_catalog(&catalog, &self.cfg.entity) {
            CatalogRead::Refused(_) => LinksReport { links: Vec::new(), cut: 0, homonyms: Vec::new(), sources: Vec::new() },
            CatalogRead::Ready { catalog: cat, .. } => links_between(&cat, &self.cfg, &sources)?,
        };
        ctx.metric("links", report.links.len() as f64);
        let value = if self.json {
            serde_json::to_value(&report).map_err(|e| e.to_string())?
        } else {
            serde_json::Value::String(report.markdown(&self.cfg))
        };
        ctx.set_output("result", PortValue::new(value));
        Ok(())
    }
}

pub struct LinksNodeFactory;

fn liste(v: Option<&serde_json::Value>) -> Vec<String> {
    match v {
        Some(serde_json::Value::Array(a)) => a.iter().filter_map(|x| x.as_str()).map(str::to_string).collect(),
        Some(serde_json::Value::String(s)) => s.split(['|', ',']).map(str::trim).filter(|s| !s.is_empty()).map(String::from).collect(),
        _ => Vec::new(),
    }
}

impl NodeFactory for LinksNodeFactory {
    fn create(&self, name: &str, config: &serde_json::Value) -> Result<Box<dyn Node>, String> {
        let s = |k: &str| config.get(k).and_then(|v| v.as_str()).map(str::to_string);
        let entier = |k: &str, defaut: u64| config.get(k).and_then(|v| v.as_u64()).unwrap_or(defaut);
        let relations = liste(config.get("relations"));
        if relations.is_empty() {
            return Err("LinksNode: « relations » requis".into());
        }
        let mut labels = BTreeMap::new();
        for paire in liste(config.get("labels")) {
            let (rel, lib) = paire.split_once('=').ok_or_else(|| format!("LinksNode: libellé « {paire} » : écrire REL=libellé"))?;
            labels.insert(rel.trim().to_string(), lib.trim().to_string());
        }
        let cfg = LinksConfig {
            entity: s("entity").filter(|v| !v.is_empty()).ok_or("LinksNode: « entity » requis")?,
            relations,
            labels,
            title: s("title").unwrap_or_else(|| "name".into()),
            path_fields: {
                let p = liste(config.get("path_fields"));
                if p.is_empty() { vec!["file_path".into()] } else { p }
            },
            line_field: s("line_field").unwrap_or_else(|| "start_line".into()),
            kind_field: s("kind_field").unwrap_or_default(),
            max_hops: entier("max_hops", MAX_HOPS as u64).clamp(1, MAX_HOPS as u64) as usize,
            max_links: entier("max_links", 5).clamp(1, 50) as usize,
            max_degree: entier("max_degree", 50).max(1) as usize,
            sources: entier("sources", 10).clamp(2, MAX_SOURCES as u64) as usize,
        };
        // Des identifiants seulement : ils entrent dans le texte des requêtes.
        let ident = |x: &str| !x.is_empty() && x.chars().all(|c| c.is_alphanumeric() || c == '_');
        let tous = [&cfg.entity, &cfg.title, &cfg.line_field].into_iter().chain(&cfg.relations).chain(&cfg.path_fields).chain(std::iter::once(&cfg.kind_field).filter(|k| !k.is_empty()));
        if let Some(x) = tous.into_iter().find(|x| !ident(x)) {
            return Err(format!("LinksNode: « {x} » n'est pas un identifiant"));
        }
        let json = match s("format").as_deref() {
            None | Some("") | Some("markdown") => false,
            Some("json") => true,
            Some(f) => return Err(format!("LinksNode: format inconnu « {f} »")),
        };
        Ok(Box::new(LinksNode { node_name: name.to_string(), cfg, uuids: liste(config.get("uuids")), json }))
    }
    fn node_type(&self) -> &'static str {
        "LinksNode"
    }
    fn schema(&self) -> NodeSchema {
        let p = |name: &'static str, param_type, required, default: Option<serde_json::Value>, description: &'static str| ConfigParam {
            name,
            param_type,
            required,
            default,
            description,
            choices: None,
            json_schema: None,
        };
        use ConfigParamType::{Int, Json, String as S};
        let mut format = p("format", S, false, Some(serde_json::json!("markdown")), "markdown | json");
        format.choices = Some(Choices::fixed(["markdown", "json"]));
        NodeSchema {
            node_type: "LinksNode",
            description: "How search results connect: the shortest graph paths between them (up to four hops, both directions, along declared relations), hubs neither traversed nor used as meeting points. Renders one line per link, or nothing — an empty text when the results do not connect.",
            inputs: vec![PortDef { name: "results", port_type: PortType::Results, required: false }],
            outputs: vec![PortDef { name: "result", port_type: PortType::Map, required: false }],
            config_params: vec![
                p("entity", S, true, None, "Entité des résultats (ex. Scope)"),
                p("relations", S, true, None, "Relations suivies dans les deux sens, séparées par |"),
                p("labels", S, false, Some(serde_json::json!("")), "Libellés : REL=libellé, séparés par |"),
                p("uuids", Json, false, Some(serde_json::json!([])), "Départs, en plus du port results (liste d'uuid)"),
                p("title", S, false, Some(serde_json::json!("name")), "Champ titre"),
                p("path_fields", S, false, Some(serde_json::json!("file_path")), "Champs de chemin, le premier non vide, séparés par |"),
                p("line_field", S, false, Some(serde_json::json!("start_line")), "Champ de ligne"),
                p("max_hops", Int, false, Some(serde_json::json!(4)), "Longueur maximale d'un lien, 1 à 4"),
                p("max_links", Int, false, Some(serde_json::json!(5)), "Liens rendus au plus"),
                p("max_degree", Int, false, Some(serde_json::json!(50)), "Un nœud plus connecté n'est ni traversé ni point de rencontre"),
                p("sources", Int, false, Some(serde_json::json!(10)), "Résultats pris comme départs, dans l'ordre (2 à 20)"),
                format,
            ],
        }
    }
}

// ─── La cohésion ─────────────────────────────────────────────────────────────

/// **La cohésion de chaque candidat** : la somme, sur les autres candidats
/// qu'il rejoint en `max_hops` sauts au plus (carrefours exclus), de
/// `1 / sauts`. Un résultat isolé vaut 0 ; un résultat au milieu d'une
/// grappe de résultats reliés vaut plus. C'est un signal de rang, pas de
/// pertinence : la fusion le prend en `boost`, avec un poids déclaré (0 par
/// défaut — éteint).
pub fn cohesion_of(catalog: &Catalog, cfg: &LinksConfig, sources: &[String]) -> Result<HashMap<String, f64>, String> {
    let tous = LinksConfig { max_links: usize::MAX, ..cfg.clone() };
    let report = links_between(catalog, &tous, sources)?;
    let mut vus: HashSet<&str> = HashSet::new();
    let sources: Vec<&String> = sources.iter().filter(|u| !u.is_empty() && vus.insert(u.as_str())).take(cfg.sources).collect();
    let mut out: HashMap<String, f64> = HashMap::new();
    for l in &report.links {
        let poids = 1.0 / l.hops().max(1) as f64;
        for i in [l.a, l.b] {
            if let Some(u) = sources.get(i) {
                *out.entry((*u).clone()).or_default() += poids;
            }
        }
    }
    Ok(out)
}

/// Cohésion des candidats : **input** `results` (fan-in : les signaux de la
/// recherche), **output** `results` — un signal étiqueté (`label`, par
/// défaut `cohesion`), à brancher sur une fusion avec `boost='<label>'`.
pub struct CohesionNode {
    node_name: String,
    cfg: LinksConfig,
    label: String,
}

impl Node for CohesionNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "CohesionNode"
    }
    fn inputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&CohesionNodeFactory).0
    }
    fn outputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&CohesionNodeFactory).1
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        use super::catalog_read::{read_catalog, CatalogRead};
        let candidats: Vec<UnifiedResult> = ctx.take_input("results").and_then(take_or_clone::<Vec<UnifiedResult>>).unwrap_or_default();
        // L'ordre des candidats : le meilleur rang de chacun, tous signaux confondus.
        let mut tri: Vec<&UnifiedResult> = candidats.iter().filter(|r| !r.uuid.starts_with("scan:")).collect();
        tri.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        let sources: Vec<String> = tri.iter().map(|r| r.uuid.clone()).collect();
        let catalog = ctx.service::<Arc<Mutex<Catalog>>>("catalog").cloned().ok_or("CohesionNode: service 'catalog' absent")?;
        let scores = match read_catalog(&catalog, &self.cfg.entity) {
            CatalogRead::Refused(_) => HashMap::new(),
            CatalogRead::Ready { catalog: cat, .. } => cohesion_of(&cat, &self.cfg, &sources)?,
        };
        let mut sortie: Vec<UnifiedResult> = Vec::new();
        let mut deja: HashSet<String> = HashSet::new();
        for r in tri {
            let Some(&s) = scores.get(&r.uuid) else { continue };
            if s <= 0.0 || !deja.insert(r.uuid.clone()) {
                continue;
            }
            sortie.push(UnifiedResult {
                uuid: r.uuid.clone(),
                score: s,
                entity: r.entity.clone(),
                data: None,
                chunk: None,
                chunks: None,
                relation: None,
                matched_children: None,
                other_children: None,
                graph: None,
                signal: Some(self.label.clone()),
            });
        }
        ctx.metric("coherent", sortie.len() as f64);
        ctx.set_output("results", PortValue::new(sortie));
        Ok(())
    }
}

pub struct CohesionNodeFactory;

impl NodeFactory for CohesionNodeFactory {
    fn create(&self, name: &str, config: &serde_json::Value) -> Result<Box<dyn Node>, String> {
        let s = |k: &str| config.get(k).and_then(|v| v.as_str()).map(str::to_string);
        let entier = |k: &str, defaut: u64| config.get(k).and_then(|v| v.as_u64()).unwrap_or(defaut);
        let relations = liste(config.get("relations"));
        if relations.is_empty() {
            return Err("CohesionNode: « relations » requis".into());
        }
        let cfg = LinksConfig {
            entity: s("entity").filter(|v| !v.is_empty()).ok_or("CohesionNode: « entity » requis")?,
            relations,
            labels: BTreeMap::new(),
            title: "_uuid".into(),
            path_fields: Vec::new(),
            line_field: "_uuid".into(),
            kind_field: String::new(),
            max_hops: entier("max_hops", 2).clamp(1, MAX_HOPS as u64) as usize,
            max_links: usize::MAX,
            max_degree: entier("max_degree", 50).max(1) as usize,
            sources: entier("sources", 40).clamp(2, 200) as usize,
        };
        let ident = |x: &str| !x.is_empty() && x.chars().all(|c| c.is_alphanumeric() || c == '_');
        if let Some(x) = std::iter::once(&cfg.entity).chain(&cfg.relations).find(|x| !ident(x)) {
            return Err(format!("CohesionNode: « {x} » n'est pas un identifiant"));
        }
        Ok(Box::new(CohesionNode { node_name: name.to_string(), cfg, label: s("label").filter(|v| !v.is_empty()).unwrap_or_else(|| "cohesion".into()) }))
    }
    fn node_type(&self) -> &'static str {
        "CohesionNode"
    }
    fn schema(&self) -> NodeSchema {
        let p = |name: &'static str, param_type, required, default: Option<serde_json::Value>, description: &'static str| ConfigParam {
            name,
            param_type,
            required,
            default,
            description,
            choices: None,
            json_schema: None,
        };
        use ConfigParamType::{Int, String as S};
        NodeSchema {
            node_type: "CohesionNode",
            description: "How cohesive each search candidate is: the sum of 1/hops to the other candidates it reaches within max_hops along declared relations (hubs excluded). Emits a labelled result signal to plug into FuseResultsNode as a boost (weight declared there, 0 = off).",
            inputs: vec![PortDef { name: "results", port_type: PortType::Results, required: false }],
            outputs: vec![PortDef { name: "results", port_type: PortType::Results, required: false }],
            config_params: vec![
                p("entity", S, true, None, "Entité des candidats (ex. Scope)"),
                p("relations", S, true, None, "Relations suivies dans les deux sens, séparées par |"),
                p("max_hops", Int, false, Some(serde_json::json!(2)), "Longueur maximale d'un lien, 1 à 4"),
                p("max_degree", Int, false, Some(serde_json::json!(50)), "Un nœud plus connecté n'est ni traversé ni point de rencontre"),
                p("sources", Int, false, Some(serde_json::json!(40)), "Candidats pris, dans l'ordre des scores"),
                p("label", S, false, Some(serde_json::json!("cohesion")), "L'étiquette du signal (celle que la fusion boost)"),
            ],
        }
    }
}

// ─── La cohésion, en option de recherche ─────────────────────────────────────

/// **La cohésion après la fusion**, pilotée par `SearchOptions.cohesion` :
/// sans option (ou poids nul), les résultats passent tels quels, à coût nul.
/// Sinon, la cohésion des `candidates` premiers (même calcul que
/// [`CohesionNode`]), normalisée par son maximum, multiplie le score —
/// `score × (1 + weight × cohésion)` — et la liste se réordonne. Un nœud
/// après la fusion plutôt qu'un signal de boost : le poids est celui de
/// l'appelant, quels que soient les poids de fusion qu'il déclare.
/// **Inputs** `results`, `query` ; **output** `results`. Service : `catalog`.
pub struct CohesionBoostNode {
    node_name: String,
}

impl Node for CohesionBoostNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "CohesionBoostNode"
    }
    fn inputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&CohesionBoostNodeFactory).0
    }
    fn outputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&CohesionBoostNodeFactory).1
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        use super::catalog_read::{read_catalog, CatalogRead};
        let mut results: Vec<UnifiedResult> = ctx.take_input("results").and_then(take_or_clone::<Vec<UnifiedResult>>).unwrap_or_default();
        let qp = ctx.take_input("query").and_then(take_or_clone::<super::port::QueryPayload>);
        let opt = qp.as_ref().and_then(|q| q.options.cohesion.clone()).filter(|c| c.weight != 0.0 && !c.relations.is_empty());
        if let (Some(c), Some(catalog)) = (opt, ctx.service::<Arc<Mutex<Catalog>>>("catalog").cloned()) {
            let entity = results.iter().find_map(|r| r.entity.clone()).or_else(|| qp.as_ref().map(|q| q.target_name.clone())).unwrap_or_default();
            let cfg = LinksConfig {
                entity: entity.clone(),
                relations: c.relations.clone(),
                labels: BTreeMap::new(),
                title: "_uuid".into(),
                path_fields: Vec::new(),
                line_field: "_uuid".into(),
                kind_field: String::new(),
                max_hops: c.max_hops.clamp(1, MAX_HOPS),
                max_links: usize::MAX,
                max_degree: c.max_degree.max(1),
                sources: c.candidates.clamp(2, 200),
            };
            let ident = |x: &str| !x.is_empty() && x.chars().all(|ch| ch.is_alphanumeric() || ch == '_');
            if !std::iter::once(&cfg.entity).chain(&cfg.relations).all(|x| ident(x)) {
                return Err("CohesionBoostNode: entité ou relation qui n'est pas un identifiant".into());
            }
            let uuids: Vec<String> = results.iter().filter(|r| !r.uuid.starts_with("scan:")).map(|r| r.uuid.clone()).collect();
            let scores = match read_catalog(&catalog, &entity) {
                CatalogRead::Refused(_) => HashMap::new(),
                CatalogRead::Ready { catalog: cat, .. } => cohesion_of(&cat, &cfg, &uuids)?,
            };
            let max = scores.values().cloned().fold(0.0_f64, f64::max);
            if max > 0.0 {
                for r in results.iter_mut() {
                    let v = scores.get(&r.uuid).copied().unwrap_or(0.0) / max;
                    r.score *= 1.0 + c.weight * v;
                }
                results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
            }
            ctx.metric("coherent", scores.len() as f64);
        }
        ctx.set_output("results", PortValue::new(results));
        Ok(())
    }
}

pub struct CohesionBoostNodeFactory;

impl NodeFactory for CohesionBoostNodeFactory {
    fn create(&self, name: &str, _config: &serde_json::Value) -> Result<Box<dyn Node>, String> {
        Ok(Box::new(CohesionBoostNode { node_name: name.to_string() }))
    }
    fn node_type(&self) -> &'static str {
        "CohesionBoostNode"
    }
    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "CohesionBoostNode",
            description: "After fusion: when SearchOptions.cohesion is set (weight, relations), multiplies each fused score by 1 + weight × normalised cohesion (how many other candidates it reaches within max_hops along the declared relations, hubs excluded) and re-sorts. Without the option, results pass through at no cost.",
            inputs: vec![
                PortDef { name: "results", port_type: PortType::Results, required: false },
                PortDef { name: "query", port_type: PortType::Query, required: false },
            ],
            outputs: vec![PortDef { name: "results", port_type: PortType::Results, required: false }],
            config_params: vec![],
        }
    }
}
