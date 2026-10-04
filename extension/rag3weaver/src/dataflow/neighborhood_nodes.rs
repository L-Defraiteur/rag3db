//! **Le voisinage d'une chose, par niveaux** : ce qui en dépend directement,
//! puis à deux sauts, à trois — avec un budget, un plafond de degré pour ne
//! pas traverser les carrefours, et un regroupement par un champ déclaré.
//!
//! Le nœud est générique : le gabarit déclare d'où partir (un pivot et son
//! champ d'identité, comme `usages`), quelles relations suivre et dans quel
//! sens, et par quel champ regrouper ce qu'on atteint. Pour le code,
//! `impact` suit les `CONSUMES`, `INHERITS_FROM` et `IMPLEMENTS` entrants et
//! regroupe par `test_role` : le code qui dépend de la chose d'un côté, les
//! tests qui la traversent de l'autre.
//!
//! **Le premier saut** reprend [`super::usage_nodes::usages_of`] : les arêtes
//! directes, et le rendez-vous pour les usages d'un nom ambigu, marqués « par
//! le nom ». Au-delà, seules les arêtes directes sont suivies : suivre le nom
//! seul à chaque niveau multiplierait les devinettes.
//!
//! **Un départ supplémentaire**, déclaré au gabarit : un chemin de relations
//! depuis chaque départ, gardé quand un champ y vaut celui du départ. Pour
//! Rust, « la méthode du trait qu'implémente l'impl de la méthode modifiée »
//! (`HAS_PARENT>`, `IMPLEMENTS>`, `PARENT_OF>`, même `name`) : ce qui
//! l'appelle *peut* atteindre la méthode modifiée. Ce qu'on atteint ainsi est
//! rendu à part, sous le libellé du gabarit, jamais mêlé au sûr. Le nœud ne
//! connaît ni trait ni méthode.
//!
//! **Les requêtes** : un saut est `UNWIND $uuids AS u MATCH (d:T {_uuid: u})
//! <-[r:REL]-(m:S)`, une liste de valeurs simples jointe par hachage (journal,
//! §6) ; le degré d'un nœud se compte de la même façon. Entre deux sauts, le
//! nœud dédoublonne, applique le plafond de degré et le budget — ce qu'un
//! chemin de longueur variable (`*1..3`) ne laisserait pas faire.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use serde::Serialize;

use super::node::{Node, NodeContext};
use super::node_registry::{Choices, ConfigParam, ConfigParamType, NodeFactory, NodeSchema};
use super::port::{PortDef, PortType, PortValue};
use super::usage_nodes::{rel_info, usages_of, RelInfo, UsagesConfig};
pub use super::graph_walk::{degree_query, Direction};
use crate::catalog::{Catalog, CatalogError};
use crate::connection::{CypherValue, QueryParam};

pub const MAX_DEPTH: usize = 3;
pub const MAX_BUDGET: usize = 2_000;

/// Un pas d'un chemin déclaré : une relation et son sens.
#[derive(Debug, Clone)]
pub struct Step {
    pub relation: String,
    pub direction: Direction,
}

/// Ce que le gabarit déclare.
#[derive(Debug, Clone)]
pub struct NeighborhoodConfig {
    /// D'où partir, par un nom : pivot, clé, relation de définition, et
    /// rendez-vous des usages (facultatif).
    pub start: UsagesConfig,
    /// Les relations suivies, et leur sens.
    pub relations: Vec<String>,
    pub direction: Direction,
    /// Le champ de regroupement de l'entité atteinte (vide : « autre »).
    pub group_by: String,
    /// Un titre de remplacement quand il n'est pas vide (`test_name`).
    pub label_field: String,
    /// Un champ montré entre parenthèses (`test_certainty`).
    pub note_field: String,
    /// Le départ supplémentaire : un chemin, le champ qui doit être égal, et
    /// le libellé de son groupe.
    pub also_path: Vec<Step>,
    pub also_same: String,
    pub also_label: String,
}

/// Un nœud atteint.
#[derive(Debug, Clone, Serialize)]
pub struct Reached {
    pub uuid: String,
    pub title: String,
    pub label: String,
    pub kind: String,
    pub path: String,
    pub line: Option<i64>,
    pub group: String,
    pub note: String,
    /// 1 : touché directement.
    pub level: usize,
    /// La relation par laquelle on l'a atteint.
    pub relation: String,
    /// Atteint par le nom seul (nom ambigu) : peut-être.
    pub by_name: bool,
    /// Atteint par le départ supplémentaire.
    pub by_also: bool,
    /// Un carrefour : son degré, et il n'a pas été traversé vers le code.
    pub hub: Option<usize>,
    /// Atteint à travers un carrefour, dont c'est le titre : seuls les nœuds
    /// du groupe déclaré (pour le code, les tests — personne ne les appelle)
    /// sont relevés au-delà d'un carrefour.
    pub through_hub: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NeighborhoodReport {
    pub name: String,
    pub starts: Vec<super::usage_nodes::Item>,
    pub ambiguous: bool,
    pub reached: Vec<Reached>,
    /// Nœuds trouvés mais non rendus, faute de budget.
    pub cut: usize,
    pub depth: usize,
}

// ─── Requêtes ────────────────────────────────────────────────────────────────

fn champs(alias: &str, cfg: &NeighborhoodConfig) -> String {
    let s = &cfg.start;
    let mut c = vec![format!("{alias}._uuid"), format!("{alias}.{}", s.title), format!("{alias}.{}", s.kind_field)];
    c.extend(s.path_fields.iter().map(|p| format!("{alias}.{p}")));
    c.push(format!("{alias}.{}", s.line_field));
    for f in [&cfg.group_by, &cfg.label_field, &cfg.note_field] {
        c.push(if f.is_empty() { "NULL".into() } else { format!("{alias}.{f}") });
    }
    c.join(", ")
}

/// Un saut : depuis une liste d'uuid, par une relation, dans un sens.
pub fn hop_query(cfg: &NeighborhoodConfig, rel: &RelInfo, direction: Direction) -> String {
    let ligne = if rel.props.iter().any(|p| p == &cfg.start.line) { format!("r.{}", cfg.start.line) } else { "NULL".into() };
    format!("UNWIND $uuids AS u MATCH {}{} RETURN u, {c}, {ligne}", direction.pattern(rel), cfg.start.edge_mark.clause(rel), c = champs("m", cfg))
}

fn texte(v: Option<&CypherValue>) -> String {
    v.and_then(|v| v.as_str()).unwrap_or("").to_string()
}

/// Les champs d'un nœud à partir de la colonne `debut` d'une ligne.
fn reached_from(row: &[CypherValue], debut: usize, cfg: &NeighborhoodConfig) -> Reached {
    let n = cfg.start.path_fields.len();
    let path = (0..n).map(|i| texte(row.get(debut + 3 + i))).find(|p| !p.is_empty()).unwrap_or_default();
    let title = texte(row.get(debut + 1));
    let label = texte(row.get(debut + 5 + n));
    Reached {
        uuid: texte(row.get(debut)),
        label: if label.is_empty() { title.clone() } else { label },
        title,
        kind: texte(row.get(debut + 2)),
        path,
        line: row.get(debut + 3 + n).and_then(|v| v.as_i64()),
        group: texte(row.get(debut + 4 + n)),
        note: texte(row.get(debut + 6 + n)),
        level: 0,
        relation: String::new(),
        by_name: false,
        by_also: false,
        hub: None,
        through_hub: None,
    }
}

fn largeur(cfg: &NeighborhoodConfig) -> usize {
    7 + cfg.start.path_fields.len()
}

struct Moteur<'a> {
    catalog: &'a Catalog,
    cfg: &'a NeighborhoodConfig,
    rels: Vec<RelInfo>,
}

impl Moteur<'_> {
    fn err(e: CatalogError) -> String {
        format!("NeighborhoodNode: {e}")
    }

    /// Un saut depuis `depuis`, par toutes les relations suivies.
    fn saut(&self, depuis: &[String]) -> Result<Vec<(String, Reached)>, String> {
        if depuis.is_empty() {
            return Ok(Vec::new());
        }
        let liste = CypherValue::List(depuis.iter().map(|u| CypherValue::String(u.clone())).collect());
        let mut out = Vec::new();
        for rel in &self.rels {
            let q = hop_query(self.cfg, rel, self.cfg.direction);
            let rows = self.catalog.execute_raw_with_params(&q, &[QueryParam::new("uuids", liste.clone())]).map_err(Self::err)?;
            for r in &rows.rows {
                let de = texte(r.first());
                let mut m = reached_from(r, 1, self.cfg);
                if let Some(site) = r.get(1 + largeur(self.cfg)).and_then(|v| v.as_i64()) {
                    m.line = Some(site);
                }
                m.relation = rel.name.clone();
                out.push((de, m));
            }
        }
        Ok(out)
    }

    /// Le degré de chaque nœud dans le sens suivi.
    fn degres(&self, uuids: &[String]) -> Result<HashMap<String, usize>, String> {
        super::graph_walk::degrees(self.catalog, &self.rels, &[self.cfg.direction], uuids).map_err(|e| format!("NeighborhoodNode: {e}"))
    }

    /// Le départ supplémentaire : le chemin déclaré depuis chaque départ,
    /// gardé quand son champ `also_same` vaut celui du départ.
    fn aussi(&self, departs: &[super::usage_nodes::Item]) -> Result<Vec<String>, String> {
        let cfg = self.cfg;
        if cfg.also_path.is_empty() || departs.is_empty() {
            return Ok(Vec::new());
        }
        let mut courant: Vec<(String, String)> = departs.iter().map(|d| (d.uuid.clone(), d.title.clone())).collect();
        for (i, pas) in cfg.also_path.iter().enumerate() {
            let rel = rel_info(self.catalog, &pas.relation)?;
            let dernier = i + 1 == cfg.also_path.len();
            let (motif, champ) = (pas.direction.pattern(&rel), "m");
            let q = if dernier {
                format!("UNWIND $uuids AS u MATCH {motif} RETURN u, {champ}._uuid, {champ}.{}", cfg.also_same)
            } else {
                format!("UNWIND $uuids AS u MATCH {motif} RETURN u, {champ}._uuid")
            };
            let liste = CypherValue::List(courant.iter().map(|(u, _)| CypherValue::String(u.clone())).collect());
            let rows = self.catalog.execute_raw_with_params(&q, &[QueryParam::new("uuids", liste)]).map_err(Self::err)?;
            let origine: HashMap<&str, &str> = courant.iter().map(|(u, n)| (u.as_str(), n.as_str())).collect();
            let mut suivant = Vec::new();
            for r in &rows.rows {
                let de = texte(r.first());
                let Some(nom_depart) = origine.get(de.as_str()) else { continue };
                let vers = texte(r.get(1));
                if dernier && texte(r.get(2)) != *nom_depart {
                    continue;
                }
                suivant.push((vers, nom_depart.to_string()));
            }
            suivant.sort();
            suivant.dedup();
            courant = suivant;
        }
        let a_exclure: HashSet<&str> = departs.iter().map(|d| d.uuid.as_str()).collect();
        Ok(courant.into_iter().map(|(u, _)| u).filter(|u| !a_exclure.contains(u.as_str())).collect())
    }
}

/// Le voisinage d'un nom, jusqu'à `depth` sauts, au plus `budget` nœuds
/// rendus, sans traverser un nœud de degré supérieur à `max_degree`.
pub fn neighborhood_of(
    catalog: &Catalog,
    cfg: &NeighborhoodConfig,
    name: &str,
    path_prefix: &str,
    depth: usize,
    budget: usize,
    max_degree: usize,
) -> Result<NeighborhoodReport, String> {
    let depth = depth.clamp(1, MAX_DEPTH);
    let rels: Vec<RelInfo> = cfg.relations.iter().map(|r| rel_info(catalog, r)).collect::<Result<_, _>>()?;
    let moteur = Moteur { catalog, cfg, rels };

    // Le départ et le premier saut : comme `usages`, en union avec le
    // rendez-vous (dans le sens entrant seulement : c'est un usage).
    let usages = usages_of(catalog, &cfg.start, name, path_prefix)?;
    let vus: HashSet<String> = usages.definitions.iter().map(|d| d.uuid.clone()).collect();

    let mut premiers: Vec<Reached> = Vec::new();
    let departs: Vec<String> = usages.definitions.iter().map(|d| d.uuid.clone()).collect();
    premiers.extend(moteur.saut(&departs)?.into_iter().map(|(_, m)| m));
    // Le départ supplémentaire, avant le nom seul : un chemin déclaré dit
    // plus qu'un nom partagé.
    let aussi = moteur.aussi(&usages.definitions)?;
    if !aussi.is_empty() {
        for (_, m) in moteur.saut(&aussi)? {
            premiers.push(Reached { by_also: true, ..m });
        }
    }
    // Les usages par le nom seul (nom ambigu), qu'aucune arête directe ne
    // porte — un usage, donc dans le sens entrant seulement.
    if cfg.direction == Direction::Incoming {
        let deja: HashSet<String> = premiers.iter().map(|m| m.uuid.clone()).collect();
        let rv: Vec<String> = usages
            .usages
            .iter()
            .filter(|u| u.definition.is_none() && !deja.contains(&u.user.uuid))
            .map(|u| u.user.uuid.clone())
            .collect();
        if !rv.is_empty() {
            for m in fiches(&moteur, &rv)? {
                premiers.push(Reached { by_name: true, relation: cfg.start.used_by.clone().unwrap_or_default(), ..m });
            }
        }
    }

    let (atteints, cut) = parcourir(&moteur, vus, premiers, depth, budget, max_degree)?;
    Ok(NeighborhoodReport { name: name.to_string(), starts: usages.definitions, ambiguous: usages.ambiguous, reached: atteints, cut, depth })
}

/// **L'impact d'un fichier** : le même voisinage, parti de toutes les
/// lignes dont un champ de chemin vaut `file` (l'entité définie par
/// `defined_by`, sinon le pivot). Ce qui est dans le fichier lui-même n'est
/// jamais compté — un fichier qui s'appelle lui-même n'est pas un impact :
/// seuls les dépendants extérieurs, dédoublonnés, puis leurs propres
/// dépendants. Pas de rendez-vous par le nom : on part de lignes, pas d'un
/// nom.
pub fn neighborhood_of_file(catalog: &Catalog, cfg: &NeighborhoodConfig, file: &str, depth: usize, budget: usize, max_degree: usize) -> Result<NeighborhoodReport, String> {
    let depth = depth.clamp(1, MAX_DEPTH);
    let rels: Vec<RelInfo> = cfg.relations.iter().map(|r| rel_info(catalog, r)).collect::<Result<_, _>>()?;
    let table = match &cfg.start.defined_by {
        Some(d) => rel_info(catalog, d)?.from,
        None => cfg.start.pivot.clone(),
    };
    let moteur = Moteur { catalog, cfg, rels };
    let condition = cfg.start.path_fields.iter().map(|f| format!("s.{f} = $file")).collect::<Vec<_>>().join(" OR ");
    let q = format!("MATCH (s:{table}) WHERE {condition} RETURN s._uuid, {}", champs("s", cfg));
    let rows = catalog
        .execute_raw_with_params(&q, &[QueryParam::new("file", CypherValue::String(file.to_string()))])
        .map_err(Moteur::err)?;
    let departs: Vec<super::usage_nodes::Item> = rows
        .rows
        .iter()
        .map(|r| {
            let m = reached_from(r, 1, cfg);
            super::usage_nodes::Item { uuid: m.uuid, title: m.title, kind: m.kind, path: m.path, line: m.line }
        })
        .collect();
    let vus: HashSet<String> = departs.iter().map(|d| d.uuid.clone()).collect();
    let uuids: Vec<String> = departs.iter().map(|d| d.uuid.clone()).collect();
    let premiers: Vec<Reached> = moteur.saut(&uuids)?.into_iter().map(|(_, m)| m).collect();
    let (atteints, cut) = parcourir(&moteur, vus, premiers, depth, budget, max_degree)?;
    Ok(NeighborhoodReport { name: file.to_string(), starts: departs, ambiguous: false, reached: atteints, cut, depth })
}

/// Le parcours par niveaux, commun aux deux départs : dédoublonner, couper
/// au budget, montrer un carrefour sans le traverser vers le code.
fn parcourir(
    moteur: &Moteur<'_>,
    mut vus: HashSet<String>,
    premiers: Vec<Reached>,
    depth: usize,
    budget: usize,
    max_degree: usize,
) -> Result<(Vec<Reached>, usize), String> {
    let cfg = moteur.cfg;
    let mut atteints: Vec<Reached> = Vec::new();
    let mut niveau: Vec<String> = Vec::new();
    let mut cut = 0usize;
    let mut a_explorer: Vec<Reached> = premiers;
    for lvl in 1..=depth {
        let mut neufs: Vec<Reached> = Vec::new();
        for mut m in plus_etroits(a_explorer.drain(..).collect()) {
            if !vus.insert(m.uuid.clone()) {
                continue;
            }
            if atteints.len() + neufs.len() >= budget {
                cut += 1;
                continue;
            }
            m.level = lvl;
            neufs.push(m);
        }
        // Le plafond de degré : un carrefour est montré, pas traversé vers le
        // code ; au-delà, seuls ses voisins du groupe déclaré sont relevés.
        let a_suivre: Vec<String> = neufs.iter().filter(|m| !m.by_name && !m.by_also && m.through_hub.is_none()).map(|m| m.uuid.clone()).collect();
        let degres = moteur.degres(&a_suivre)?;
        niveau.clear();
        let mut carrefours: Vec<String> = Vec::new();
        for m in &mut neufs {
            if !a_suivre.contains(&m.uuid) {
                continue;
            }
            let d = degres.get(&m.uuid).copied().unwrap_or(0);
            if d > max_degree {
                m.hub = Some(d);
                carrefours.push(m.uuid.clone());
            } else {
                niveau.push(m.uuid.clone());
            }
        }
        let titre_de: HashMap<String, String> = neufs.iter().map(|m| (m.uuid.clone(), m.title.clone())).collect();
        atteints.extend(neufs);
        if lvl == depth {
            break;
        }
        a_explorer = moteur.saut(&niveau)?.into_iter().map(|(_, m)| m).collect();
        if !cfg.group_by.is_empty() {
            for (de, m) in moteur.saut(&carrefours)? {
                if !m.group.is_empty() {
                    let hub = titre_de.get(&de).cloned();
                    a_explorer.push(Reached { through_hub: hub, ..m });
                }
            }
        }
        if a_explorer.is_empty() {
            break;
        }
    }
    // Ce qui reste à explorer quand on s'arrête n'est pas compté : seul le
    // budget coupe ce qui a été trouvé.
    Ok((atteints, cut))
}

/// Un même site vu par un membre et par le conteneur qui le contient — un
/// conteneur garde les références de ses membres — : le membre seul, le
/// plus étroit.
fn plus_etroits(v: Vec<Reached>) -> Vec<Reached> {
    let conteneur = |k: &str| matches!(k, "class" | "interface" | "namespace" | "module");
    let sites: HashSet<(String, Option<i64>)> = v.iter().filter(|m| !conteneur(&m.kind)).map(|m| (m.path.clone(), m.line)).collect();
    v.into_iter().filter(|m| !conteneur(&m.kind) || !sites.contains(&(m.path.clone(), m.line))).collect()
}

/// Les fiches de nœuds connus par leur uuid.
fn fiches(moteur: &Moteur<'_>, uuids: &[String]) -> Result<Vec<Reached>, String> {
    let Some(rel) = moteur.rels.first() else { return Ok(Vec::new()) };
    let table = match moteur.cfg.direction {
        Direction::Incoming => &rel.from,
        Direction::Outgoing => &rel.to,
    };
    let q = format!("UNWIND $uuids AS u MATCH (m:{table} {{_uuid: u}}) RETURN u, {}", champs("m", moteur.cfg));
    let liste = CypherValue::List(uuids.iter().map(|u| CypherValue::String(u.clone())).collect());
    let rows = moteur.catalog.execute_raw_with_params(&q, &[QueryParam::new("uuids", liste)]).map_err(Moteur::err)?;
    Ok(rows.rows.iter().map(|r| reached_from(r, 1, moteur.cfg)).collect())
}

// ─── Rendu ───────────────────────────────────────────────────────────────────

fn lieu(path: &str, line: Option<i64>) -> String {
    match line {
        Some(l) => format!("{path}:{l}"),
        None => path.to_string(),
    }
}

impl NeighborhoodReport {
    /// Les nœuds dont le groupe n'est pas vide (pour le code : les tests),
    /// une fois chacun, dans l'ordre des niveaux.
    pub fn grouped(&self) -> Vec<&Reached> {
        let mut vus = HashSet::new();
        self.reached.iter().filter(|r| !r.group.is_empty() && vus.insert(r.uuid.clone())).collect()
    }

    /// **Le résumé** : les comptes d'abord, puis les premiers du groupe
    /// déclaré (pour le code, les tests), dans la limite. Rien quand rien
    /// n'est atteint — une section qui accompagne un autre rendu se tait.
    /// `only` : la seule valeur du groupe à compter et nommer (pour le code,
    /// `case` — les tests qu'on relance, pas leurs aides) ; vide, toutes.
    pub fn summary(&self, group_title: &str, rest_title: &str, limit: usize, only: &str) -> String {
        if self.reached.is_empty() {
            return String::new();
        }
        let groupes: Vec<&Reached> = self.grouped().into_iter().filter(|m| only.is_empty() || m.group == only).collect();
        let reste: Vec<&Reached> = self.reached.iter().filter(|m| m.group.is_empty()).collect();
        let directs = reste.iter().filter(|m| m.level == 1).count();
        let mut out = format!(
            "{rest_title} : {directs} directement, {} en tout sur {} niveau{}",
            reste.len(),
            self.depth,
            if self.depth > 1 { "x" } else { "" }
        );
        if self.cut > 0 {
            out.push_str(&format!(" (et {} au-delà du budget)", self.cut));
        }
        out.push_str(".\n");
        if groupes.is_empty() {
            return out;
        }
        let noms: Vec<String> = groupes
            .iter()
            .take(limit)
            .map(|m| if m.path.is_empty() { format!("`{}`", m.label) } else { format!("`{}` ({})", m.label, lieu(&m.path, m.line)) })
            .collect();
        out.push_str(&format!("{group_title} : {} — {}", groupes.len(), noms.join(", ")));
        if groupes.len() > limit {
            out.push_str(&format!(", et {} autres", groupes.len() - limit));
        }
        out.push_str(".\n");
        out
    }

    pub fn markdown(&self, group_title: &str, rest_title: &str, also_label: &str, limit: usize) -> String {
        let mut out = format!("# impact: {}\n\n", self.name);
        out.push_str(&format!("## Départ ({})\n", self.starts.len()));
        if self.starts.is_empty() {
            out.push_str("(aucune définition indexée sous ce nom)\n");
        }
        for s in &self.starts {
            out.push_str(&format!("- {} {} — {}\n", s.kind, s.title, lieu(&s.path, s.line)));
        }
        if self.ambiguous {
            out.push_str("\nNom ambigu : toutes les définitions sont des départs ; les usages trouvés par le nom seul sont marqués « par le nom ».\n");
        }
        let groupes = self.grouped();
        let resume_niveaux: Vec<String> = (1..=self.depth)
            .map(|l| format!("{} à {} saut{}", self.reached.iter().filter(|r| r.level == l && !r.by_also).count(), l, if l > 1 { "s" } else { "" }))
            .collect();
        out.push_str(&format!(
            "\n**{} touchés** ({}) — dont **{} {}**",
            self.reached.iter().filter(|r| !r.by_also).count(),
            resume_niveaux.join(", "),
            groupes.iter().filter(|r| !r.by_also).count(),
            group_title
        ));
        if self.cut > 0 {
            out.push_str(&format!(" ; budget atteint, {} de plus non rendus", self.cut));
        }
        out.push_str(".\n");

        let ligne = |r: &Reached, out: &mut String| {
            let mut extra = String::new();
            if r.by_name {
                extra.push_str(" (par le nom)");
            }
            if let Some(d) = r.hub {
                extra.push_str(&format!(" — carrefour, {d} usages, non suivi"));
            }
            out.push_str(&format!("- {} {} — {}{extra}\n", r.kind, r.title, lieu(&r.path, r.line)));
        };
        for l in 1..=self.depth {
            let tous: Vec<&Reached> = self.reached.iter().filter(|r| r.level == l && !r.by_also && r.group.is_empty()).collect();
            if tous.is_empty() {
                continue;
            }
            out.push_str(&format!("\n## {rest_title}, à {l} saut{} ({})\n", if l > 1 { "s" } else { "" }, tous.len()));
            for r in tous.iter().take(limit) {
                ligne(r, &mut out);
            }
            if tous.len() > limit {
                out.push_str(&format!("… {} de plus\n", tous.len() - limit));
            }
        }
        let surs: Vec<&&Reached> = groupes.iter().filter(|r| !r.by_also).collect();
        if !surs.is_empty() {
            out.push_str(&format!("\n## {} ({})\n", group_title, surs.len()));
            for r in surs.iter().take(limit) {
                let note = if r.note.is_empty() { String::new() } else { format!(", {}", r.note) };
                let par = match (&r.through_hub, r.by_name) {
                (Some(h), _) => format!(", par le carrefour {h}"),
                (None, true) => ", par le nom".to_string(),
                _ => String::new(),
            };
                out.push_str(&format!("- `{}` — {} ({} saut{}{note}{par})\n", r.label, lieu(&r.path, r.line), r.level, if r.level > 1 { "s" } else { "" }));
            }
            if surs.len() > limit {
                out.push_str(&format!("… {} de plus\n", surs.len() - limit));
            }
        }
        let aussi: Vec<&Reached> = self.reached.iter().filter(|r| r.by_also).collect();
        if !aussi.is_empty() {
            out.push_str(&format!("\n## {also_label} ({})\n", aussi.len()));
            for r in aussi.iter().take(limit) {
                let marque = if r.group.is_empty() { String::new() } else { format!(" [{}]", r.label) };
                out.push_str(&format!("- {} {} — {}{marque}\n", r.kind, r.title, lieu(&r.path, r.line)));
            }
            if aussi.len() > limit {
                out.push_str(&format!("… {} de plus\n", aussi.len() - limit));
            }
        }
        out
    }
}

// ─── Le nœud ─────────────────────────────────────────────────────────────────

/// Voisinage par niveaux : **output** `result` (PortType::Map), markdown ou
/// JSON. Service : `catalog`.
pub struct NeighborhoodNode {
    node_name: String,
    cfg: NeighborhoodConfig,
    name: String,
    path: String,
    /// Mode par fichier : départ de toutes les lignes de ce fichier.
    file: String,
    /// `summary` : les comptes et les premiers du groupe.
    summary: bool,
    /// La valeur du groupe que le résumé compte et nomme (vide : toutes).
    summary_group: String,
    depth: usize,
    budget: usize,
    max_degree: usize,
    limit: usize,
    group_title: String,
    rest_title: String,
    json: bool,
}

impl Node for NeighborhoodNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "NeighborhoodNode"
    }
    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        Some(Box::new(serde_json::json!({ "name": self.name, "path": self.path, "depth": self.depth })))
    }
    fn outputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&NeighborhoodNodeFactory).1
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        use super::catalog_read::{read_catalog, refusal, with_status, CatalogRead};
        let catalog = ctx.service::<Arc<Mutex<Catalog>>>("catalog").cloned().ok_or("NeighborhoodNode: service 'catalog' absent")?;
        let (report, status) = match read_catalog(&catalog, &self.cfg.start.pivot) {
            CatalogRead::Refused(message) => {
                ctx.set_output("result", PortValue::new(refusal(&message, self.json)));
                return Ok(());
            }
            CatalogRead::Ready { catalog: cat, status } => {
                let r = if self.name.is_empty() {
                    neighborhood_of_file(&cat, &self.cfg, &self.file, self.depth, self.budget, self.max_degree)?
                } else {
                    neighborhood_of(&cat, &self.cfg, &self.name, &self.path, self.depth, self.budget, self.max_degree)?
                };
                (r, status)
            }
        };
        ctx.metric("reached", report.reached.len() as f64);
        ctx.metric("grouped", report.grouped().len() as f64);
        let value = if self.json {
            serde_json::to_value(&report).map_err(|e| e.to_string())?
        } else if self.summary {
            serde_json::Value::String(report.summary(&self.group_title, &self.rest_title, self.limit, &self.summary_group))
        } else {
            serde_json::Value::String(report.markdown(&self.group_title, &self.rest_title, &self.cfg.also_label, self.limit))
        };
        ctx.set_output("result", PortValue::new(with_status(status.as_deref(), value, self.json)));
        Ok(())
    }
}

pub struct NeighborhoodNodeFactory;

fn liste(v: Option<&serde_json::Value>) -> Vec<String> {
    v.and_then(|v| v.as_str()).unwrap_or("").split(['|', ',']).map(str::trim).filter(|s| !s.is_empty()).map(String::from).collect()
}

/// `HAS_PARENT>` (sortant) ou `<IMPLEMENTS` (entrant).
fn pas(s: &str) -> Result<Step, String> {
    if let Some(r) = s.strip_suffix('>') {
        Ok(Step { relation: r.to_string(), direction: Direction::Outgoing })
    } else if let Some(r) = s.strip_prefix('<') {
        Ok(Step { relation: r.to_string(), direction: Direction::Incoming })
    } else {
        Err(format!("NeighborhoodNode: pas « {s} » : écrire REL> (sortant) ou <REL (entrant)"))
    }
}

impl NodeFactory for NeighborhoodNodeFactory {
    fn create(&self, name: &str, config: &serde_json::Value) -> Result<Box<dyn Node>, String> {
        let s = |k: &str| config.get(k).and_then(|v| v.as_str()).map(str::to_string);
        let requis = |k: &str| s(k).filter(|v| !v.is_empty()).ok_or_else(|| format!("NeighborhoodNode: « {k} » requis"));
        let start = UsagesConfig {
            pivot: requis("pivot")?,
            key: requis("key")?,
            defined_by: s("defined_by").filter(|v| !v.is_empty()),
            used_by: s("used_by").filter(|v| !v.is_empty()),
            direct: Vec::new(),
            group_by: s("usage_field").unwrap_or_else(|| "usage".into()),
            usages_field: s("usages_field").unwrap_or_else(|| "usages".into()),
            line: s("line").unwrap_or_else(|| "line".into()),
            title: s("title").unwrap_or_else(|| "name".into()),
            kind_field: s("kind_field").unwrap_or_else(|| "scope_type".into()),
            path_fields: {
                let p = liste(config.get("path_fields"));
                if p.is_empty() { vec!["file_path".into()] } else { p }
            },
            line_field: s("line_field").unwrap_or_else(|| "start_line".into()),
            edge_mark: super::graph_walk::EdgeMark::from_config(config, "NeighborhoodNode")?,
        };
        let relations = liste(config.get("relations"));
        if relations.is_empty() {
            return Err("NeighborhoodNode: « relations » requis".into());
        }
        let direction = match s("direction").as_deref() {
            None | Some("") | Some("incoming") => Direction::Incoming,
            Some("outgoing") => Direction::Outgoing,
            Some(d) => return Err(format!("NeighborhoodNode: sens inconnu « {d} »")),
        };
        let also_path = liste(config.get("also_path")).iter().map(|p| pas(p)).collect::<Result<Vec<_>, _>>()?;
        let mut start_cfg = start;
        start_cfg.direct = relations.clone();
        let cfg = NeighborhoodConfig {
            start: start_cfg,
            relations,
            direction,
            group_by: s("group_by").unwrap_or_default(),
            label_field: s("label_field").unwrap_or_default(),
            note_field: s("note_field").unwrap_or_default(),
            also_same: s("also_same").unwrap_or_else(|| "name".into()),
            also_label: s("also_label").unwrap_or_else(|| "Aussi, peut-être".into()),
            also_path,
        };
        // Des identifiants seulement : ils entrent dans le texte des requêtes.
        let ident = |x: &str| x.is_empty() || x.chars().all(|c| c.is_alphanumeric() || c == '_');
        let st = &cfg.start;
        let tous: Vec<&str> = [&st.pivot, &st.key, &st.group_by, &st.usages_field, &st.line, &st.title, &st.kind_field, &st.line_field, &cfg.group_by, &cfg.label_field, &cfg.note_field, &cfg.also_same]
            .into_iter()
            .map(String::as_str)
            .chain(st.defined_by.iter().map(String::as_str))
            .chain(st.used_by.iter().map(String::as_str))
            .chain(cfg.relations.iter().map(String::as_str))
            .chain(st.path_fields.iter().map(String::as_str))
            .chain(cfg.also_path.iter().map(|p| p.relation.as_str()))
            .collect();
        if let Some(x) = tous.iter().find(|x| !ident(x)) {
            return Err(format!("NeighborhoodNode: « {x} » n'est pas un identifiant"));
        }
        let entier = |k: &str, defaut: u64| config.get(k).and_then(|v| v.as_u64()).unwrap_or(defaut);
        let (json, summary) = match s("format").as_deref() {
            None | Some("") | Some("markdown") => (false, false),
            Some("json") => (true, false),
            Some("summary") => (false, true),
            Some(f) => return Err(format!("NeighborhoodNode: format inconnu « {f} »")),
        };
        // Un nom, ou un fichier : l'un des deux.
        let nom = s("name").unwrap_or_default();
        let fichier = s("file").unwrap_or_default();
        if nom.is_empty() && fichier.is_empty() {
            return Err("NeighborhoodNode: « name » ou « file » requis".into());
        }
        Ok(Box::new(NeighborhoodNode {
            node_name: name.to_string(),
            cfg,
            name: nom,
            file: fichier,
            summary,
            summary_group: s("summary_group").unwrap_or_default(),
            path: s("path").unwrap_or_default(),
            depth: entier("depth", 2).clamp(1, MAX_DEPTH as u64) as usize,
            budget: entier("budget", 200).clamp(1, MAX_BUDGET as u64) as usize,
            max_degree: entier("max_degree", 50).max(1) as usize,
            limit: entier("limit", 30).clamp(1, 500) as usize,
            group_title: s("group_title").unwrap_or_else(|| "Groupés".into()),
            rest_title: s("rest_title").unwrap_or_else(|| "Touchés".into()),
            json,
        }))
    }
    fn node_type(&self) -> &'static str {
        "NeighborhoodNode"
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
        let mut direction = p("direction", S, false, Some(serde_json::json!("incoming")), "incoming (ce qui en dépend) | outgoing (ce dont il dépend)");
        direction.choices = Some(Choices::fixed(["incoming", "outgoing"]));
        let mut format = p("format", S, false, Some(serde_json::json!("markdown")), "markdown | json | summary (les comptes, puis les premiers du groupe)");
        format.choices = Some(Choices::fixed(["markdown", "json", "summary"]));
        NodeSchema {
            node_type: "NeighborhoodNode",
            description: "The neighbourhood of a thing by levels: what depends on it directly, then at two and three hops, along declared relations, with a node budget, a degree cap that shows hubs without traversing them, a grouping field, and an optional extra start reached by a declared path (rendered apart).",
            inputs: vec![],
            outputs: vec![PortDef { name: "result", port_type: PortType::Map, required: false }],
            config_params: vec![
                p("pivot", S, true, None, "Entité de rendez-vous du départ (ex. Symbol)"),
                p("key", S, true, None, "Champ d'identité du pivot"),
                p("defined_by", S, false, None, "Relation entrante vers le pivot qui donne les départs"),
                p("used_by", S, false, None, "Relation entrante vers le pivot : usages par le nom, au premier saut"),
                p("relations", S, true, None, "Relations suivies, séparées par |"),
                direction,
                p("group_by", S, false, Some(serde_json::json!("")), "Champ de regroupement de ce qu'on atteint"),
                p("label_field", S, false, Some(serde_json::json!("")), "Titre de remplacement quand non vide"),
                p("note_field", S, false, Some(serde_json::json!("")), "Champ montré entre parenthèses"),
                p("also_path", S, false, Some(serde_json::json!("")), "Départ supplémentaire : pas REL> ou <REL, séparés par |"),
                p("also_same", S, false, Some(serde_json::json!("name")), "Champ qui doit valoir celui du départ, au bout du chemin"),
                p("also_label", S, false, Some(serde_json::json!("Aussi, peut-être")), "Libellé du groupe du départ supplémentaire"),
                p("usage_field", S, false, Some(serde_json::json!("usage")), "Propriété d'arête : genre"),
                p("usages_field", S, false, Some(serde_json::json!("usages")), "Propriété d'arête : ensemble des genres"),
                p("line", S, false, Some(serde_json::json!("line")), "Propriété d'arête : ligne du site"),
                p("title", S, false, Some(serde_json::json!("name")), "Champ titre"),
                p("kind_field", S, false, Some(serde_json::json!("scope_type")), "Champ genre"),
                p("path_fields", S, false, Some(serde_json::json!("file_path")), "Champs de chemin, le premier non vide, séparés par |"),
                p("line_field", S, false, Some(serde_json::json!("start_line")), "Champ de ligne"),
                p("group_title", S, false, Some(serde_json::json!("Groupés")), "Titre de la section des nœuds groupés"),
                p("rest_title", S, false, Some(serde_json::json!("Touchés")), "Titre des sections par niveau"),
                p("name", S, false, Some(serde_json::json!("")), "Le nom de départ (ou `file`)"),
                p("summary_group", S, false, Some(serde_json::json!("")), "Résumé : la seule valeur du groupe à compter et nommer (ex. case)"),
                p("file", S, false, Some(serde_json::json!("")), "Mode par fichier : départ de toutes les lignes dont un champ de chemin vaut ce chemin ; ce qui est dans le fichier ne compte pas"),
                p("path", S, false, Some(serde_json::json!("")), "Ne partir que des définitions sous ce chemin"),
                p("depth", Int, false, Some(serde_json::json!(2)), "Profondeur, 1 à 3"),
                p("budget", Int, false, Some(serde_json::json!(200)), "Nœuds rendus au plus (plafond 2000)"),
                p("max_degree", Int, false, Some(serde_json::json!(50)), "Un nœud plus connecté est montré, pas traversé"),
                p("edge_field", S, false, Some(serde_json::json!("")), "Champ d'arête qui dit comment elle a été posée (ex. resolution)"),
                p("edge_guessed", S, false, Some(serde_json::json!("")), "Valeurs de ce champ pour une arête devinée (ex. nom), séparées par |"),
                p("limit", Int, false, Some(serde_json::json!(30)), "Lignes par section"),
                format,
            ],
        }
    }
}
