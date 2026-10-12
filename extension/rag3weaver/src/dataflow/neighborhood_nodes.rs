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
use super::usage_nodes::{in_scope, rel_info, usages_of_in, QueryScope, RelInfo, UsagesConfig};
pub use super::graph_walk::{degree_count, Direction};
use crate::catalog::{Catalog, CatalogError};
use crate::connection::{CypherValue, QueryParam};
use rag3weaver_ir::{Column, Hop};

pub const MAX_DEPTH: usize = 3;
pub const MAX_BUDGET: usize = 2_000;

/// Un pas d'un chemin déclaré : une relation et son sens. Dans `also_path`,
/// un pas peut nommer plusieurs relations (`IMPLEMENTS/INHERITS_FROM>`) et
/// se répéter (`…>+` : de 1 à [`MAX_REPEAT`] niveaux, une hiérarchie), ou
/// être facultatif (`…>?` : zéro ou un saut, le nœud lui-même compris).
#[derive(Debug, Clone)]
pub struct Step {
    /// La première relation (la seule hors de `also_path`).
    pub relation: String,
    /// Toutes les relations du pas, la première comprise.
    pub alternatives: Vec<String>,
    pub direction: Direction,
    pub repeat: bool,
    /// Zéro ou un saut : le nœud de départ du pas reste dans la suite.
    pub optional: bool,
}

impl Step {
    /// Un pas d'une seule relation, non répété.
    pub fn new(relation: &str, direction: Direction) -> Self {
        Step { relation: relation.to_string(), alternatives: vec![relation.to_string()], direction, repeat: false, optional: false }
    }
}

/// Les niveaux au plus d'un pas répété (`+`) : la profondeur d'une hiérarchie.
pub const MAX_REPEAT: usize = 8;

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
    /// Un relevé sur le chemin : pour chaque nœud atteint (départs compris),
    /// la relation déclarée suivie d'un pas — pour le code, les verrous que
    /// chaque scope prend (`LOCKS>`).
    pub collect: Option<Collect>,
    /// **Les tests par requêtes** : un chemin déclaré, suivi depuis chaque
    /// nœud atteint (départs compris) ; au bout, les nœuds du groupe déclaré
    /// (pour le code : un test qui demande ce que le nœud ou sa classe
    /// définit ou enregistre, `HAS_PARENT>?|DEFINES/REGISTERS>|<QUERIES`)
    /// sont relevés comme traversants, notés `queried_note`. Vide : rien.
    pub queried_by: Vec<Step>,
    pub queried_note: String,
}

/// **Un relevé sur le chemin**, déclaré au gabarit : la relation et son sens,
/// le champ montré de la cible (`name`), celui de l'arête (`usage`), et le
/// titre de la section. Rien ici ne parle de verrous.
#[derive(Debug, Clone)]
pub struct Collect {
    pub step: Step,
    pub field: String,
    pub note: String,
    pub title: String,
}

/// Un relevé : qui, à quel niveau (0 : un départ), quelle cible, et la note
/// de l'arête.
#[derive(Debug, Clone, Serialize)]
pub struct Collected {
    pub by: String,
    pub level: usize,
    pub target: String,
    pub note: String,
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
    /// Le relevé sur le chemin (vide sans `collect`).
    pub collected: Vec<Collected>,
    /// Le gabarit déclare un chemin de tests par requêtes, mais l'index n'en
    /// a jamais écrit la relation : « 0 tests » n'y veut rien dire.
    #[serde(default)]
    pub queries_untracked: bool,
    /// La portée demandée (vide : tout l'index), et combien de nœuds
    /// atteints elle écarte du rendu — ils restent des relais du parcours.
    #[serde(default)]
    pub scope: QueryScope,
    #[serde(default)]
    pub scope_hidden: usize,
}

// ─── Requêtes ────────────────────────────────────────────────────────────────

/// Le texte d'avant le dialecte, gardé pour la parité des tests — les noms
/// cités comme le dialecte rag3db les cite.
#[cfg(test)]
fn champs(alias: &str, cfg: &NeighborhoodConfig) -> String {
    let q = crate::dialect::cypher_quote;
    let s = &cfg.start;
    let mut c = vec![format!("{alias}.{}", q("_uuid")), format!("{alias}.{}", q(&s.title)), format!("{alias}.{}", q(&s.kind_field))];
    c.extend(s.path_fields.iter().map(|p| format!("{alias}.{}", q(p))));
    c.push(format!("{alias}.{}", q(&s.line_field)));
    for f in [&cfg.group_by, &cfg.label_field, &cfg.note_field] {
        c.push(if f.is_empty() { "NULL".into() } else { format!("{alias}.{}", q(f)) });
    }
    c.join(", ")
}

/// Les colonnes du nœud atteint, dans l'ordre de [`champs`] ; une colonne
/// vide garde la place d'un champ non déclaré.
fn colonnes(cfg: &NeighborhoodConfig) -> Vec<Column> {
    let s = &cfg.start;
    let mut c = vec![Column::Node("_uuid".into()), Column::Node(s.title.clone()), Column::Node(s.kind_field.clone())];
    c.extend(s.path_fields.iter().map(|p| Column::Node(p.clone())));
    c.push(Column::Node(s.line_field.clone()));
    for f in [&cfg.group_by, &cfg.label_field, &cfg.note_field] {
        c.push(if f.is_empty() { Column::Null } else { Column::Node(f.clone()) });
    }
    c
}

/// Un saut : depuis une liste d'uuid, par une relation, dans un sens ; les
/// colonnes du nœud atteint, puis la ligne portée par l'arête si la relation
/// la porte. La requête vient du dialecte.
pub fn hop_of(cfg: &NeighborhoodConfig, rel: &RelInfo, direction: Direction) -> Hop {
    let mut hop = super::graph_walk::hop_of(rel, direction, &cfg.start.edge_mark);
    hop.returns = colonnes(cfg);
    hop.returns.push(if rel.props.iter().any(|p| p == &cfg.start.line) { Column::Edge(cfg.start.line.clone()) } else { Column::Null });
    hop
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

    fn err_ir(e: rag3weaver_ir::TranslateError) -> String {
        format!("NeighborhoodNode: {e}")
    }

    /// Un saut depuis `depuis`, par toutes les relations suivies.
    fn saut(&self, depuis: &[String]) -> Result<Vec<(String, Reached)>, String> {
        if depuis.is_empty() {
            return Ok(Vec::new());
        }
        let liste = CypherValue::List(depuis.iter().map(|u| CypherValue::String(u.clone())).collect());
        // Un saut par relation, sur la même liste : un éventail, une
        // instruction quand le dialecte sait le fondre (`crate::plan`).
        let mut plan = rag3weaver_ir::Plan::new();
        for (k, rel) in self.rels.iter().enumerate() {
            let id = format!("r{k}");
            plan = plan
                .node(&id, rag3weaver_ir::Form::Hop(hop_of(self.cfg, rel, self.cfg.direction)), vec![QueryParam::new("uuids", liste.clone())])
                .output(&id);
        }
        let options = crate::plan::PlanOptions { no_transaction: true, ..Default::default() };
        let mut rapport = self.catalog.run_plan(&plan, options).map_err(|e| format!("NeighborhoodNode: {e}"))?;
        let mut out = Vec::new();
        for (k, rel) in self.rels.iter().enumerate() {
            let rows = rapport.outputs.remove(&format!("r{k}")).unwrap_or_default();
            for r in &rows.rows {
                let de = texte(r.first());
                let mut m = reached_from(r, 1, self.cfg);
                // La ligne du site est dans le nœud d'où part l'arête : celle
                // du nœud atteint en entrant (l'appelant) ; en sortant, il
                // garde sa propre ligne (l'appelé, là où il est défini).
                if self.cfg.direction == Direction::Incoming {
                    if let Some(site) = r.get(1 + largeur(self.cfg)).and_then(|v| v.as_i64()) {
                        m.line = Some(site);
                    }
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
        let depuis: Vec<(String, String)> = departs.iter().map(|d| (d.uuid.clone(), d.title.clone())).collect();
        let courant = follow_also(self.catalog, &cfg.also_path, &cfg.also_same, &depuis)?;
        let a_exclure: HashSet<&str> = departs.iter().map(|d| d.uuid.as_str()).collect();
        Ok(courant.into_iter().map(|(u, _)| u).filter(|u| !a_exclure.contains(u.as_str())).collect())
    }
}

/// **Un pas répété ne finit pas un chemin** : le dernier pas filtre `same`
/// à chaque niveau, et une hiérarchie s'arrêterait au premier niveau qui ne
/// correspond pas — refusé en le nommant. Les gabarits répètent le pas du
/// milieu (`HAS_PARENT>|IMPLEMENTS/INHERITS_FROM>+|PARENT_OF>`).
pub fn check_also_path(chemin: &[Step]) -> Result<(), String> {
    match chemin.last() {
        Some(p) if p.repeat => Err(format!(
            "chemin : le dernier pas ({}) ne se répète pas — il filtre le nom à chaque niveau ; répéter un pas du milieu",
            p.alternatives.join("/")
        )),
        _ => Ok(()),
    }
}

/// **Suivre un chemin déclaré** (`also_path`) depuis des départs (uuid,
/// titre) : chaque pas, par chacune de ses relations, répété s'il le dit ; au
/// bout, ne garder que les nœuds dont le champ `same` vaut le titre du départ.
/// Rend les paires (atteint, titre du départ). Partagé par `impact` (« par
/// envoi virtuel ») et `usages`.
pub fn follow_also(catalog: &Catalog, chemin: &[Step], same: &str, departs: &[(String, String)]) -> Result<Vec<(String, String)>, String> {
    if chemin.is_empty() || departs.is_empty() {
        return Ok(Vec::new());
    }
    check_also_path(chemin)?;
    let mut courant: Vec<(String, String)> = departs.to_vec();
    for (i, pas) in chemin.iter().enumerate() {
        let dernier = i + 1 == chemin.len();
        // Un saut par chacune des relations du pas, depuis `depuis`.
        let sauter = |depuis: &[(String, String)]| -> Result<Vec<(String, String)>, String> {
            let mut suivant = Vec::new();
            if depuis.is_empty() {
                return Ok(suivant);
            }
            let origine: HashMap<&str, &str> = depuis.iter().map(|(u, n)| (u.as_str(), n.as_str())).collect();
            let liste = CypherValue::List(depuis.iter().map(|(u, _)| CypherValue::String(u.clone())).collect());
            for relation in &pas.alternatives {
                let rel = rel_info(catalog, relation)?;
                let mut hop = super::graph_walk::hop_of(&rel, pas.direction, &super::graph_walk::EdgeMark::default());
                // `same` vide : rien à comparer au bout, le chemin seul.
                let filtre = dernier && !same.is_empty();
                if filtre {
                    hop.returns.push(Column::Node(same.to_string()));
                }
                let q = catalog.dialect_arc().hop(&hop).map_err(|e| format!("NeighborhoodNode: {e}"))?;
                let rows = catalog.execute_raw_with_params(&q, &[QueryParam::new("uuids", liste.clone())]).map_err(|e| format!("NeighborhoodNode: {e}"))?;
                for r in &rows.rows {
                    let de = texte(r.first());
                    let Some(nom_depart) = origine.get(de.as_str()) else { continue };
                    let vers = texte(r.get(1));
                    if filtre && texte(r.get(2)) != *nom_depart {
                        continue;
                    }
                    suivant.push((vers, nom_depart.to_string()));
                }
            }
            suivant.sort();
            suivant.dedup();
            Ok(suivant)
        };
        if pas.repeat {
            // Tous les niveaux atteints, de 1 à MAX_REPEAT : une méthode
            // redéfinie chez un grand-parent compte aussi.
            let mut tous: Vec<(String, String)> = Vec::new();
            let mut vus: HashSet<String> = courant.iter().map(|(u, _)| u.clone()).collect();
            let mut front = courant.clone();
            for _ in 0..MAX_REPEAT {
                let neufs: Vec<(String, String)> = sauter(&front)?.into_iter().filter(|(u, _)| vus.insert(u.clone())).collect();
                if neufs.is_empty() {
                    break;
                }
                tous.extend(neufs.iter().cloned());
                front = neufs;
            }
            courant = tous;
        } else if pas.optional {
            // Zéro ou un saut : la suite part du nœud et de ce qu'il atteint.
            let mut suivant = courant.clone();
            suivant.extend(sauter(&courant)?);
            suivant.sort();
            suivant.dedup();
            courant = suivant;
        } else {
            courant = sauter(&courant)?;
        }
    }
    Ok(courant)
}

/// **Le relevé sur le chemin** : la relation déclarée (`collect`), suivie
/// d'un pas depuis les départs (niveau 0) et chaque nœud atteint.
fn relever(moteur: &Moteur<'_>, departs: &[super::usage_nodes::Item], atteints: &[Reached]) -> Result<Vec<Collected>, String> {
    let Some(c) = &moteur.cfg.collect else { return Ok(Vec::new()) };
    let mut qui: HashMap<String, (String, usize)> = HashMap::new();
    for d in departs {
        qui.insert(d.uuid.clone(), (d.title.clone(), 0));
    }
    for r in atteints {
        qui.entry(r.uuid.clone()).or_insert_with(|| (r.title.clone(), r.level));
    }
    if qui.is_empty() {
        return Ok(Vec::new());
    }
    let rel = rel_info(moteur.catalog, &c.step.relation)?;
    let mut hop = super::graph_walk::hop_of(&rel, c.step.direction, &super::graph_walk::EdgeMark::default());
    hop.returns.push(Column::Node(c.field.clone()));
    hop.returns.push(Column::Edge(c.note.clone()));
    let q = moteur.catalog.dialect_arc().hop(&hop).map_err(|e| format!("NeighborhoodNode: {e}"))?;
    let liste = CypherValue::List(qui.keys().map(|u| CypherValue::String(u.clone())).collect());
    let rows = moteur.catalog.execute_raw_with_params(&q, &[QueryParam::new("uuids", liste)]).map_err(Moteur::err)?;
    let mut out: Vec<Collected> = rows
        .rows
        .iter()
        .filter_map(|r| {
            let (by, level) = qui.get(&texte(r.first()))?.clone();
            Some(Collected { by, level, target: texte(r.get(2)), note: texte(r.get(3)) })
        })
        .collect();
    out.sort_by(|a, b| (&a.target, a.level, &a.by, &a.note).cmp(&(&b.target, b.level, &b.by, &b.note)));
    out.dedup_by(|a, b| a.target == b.target && a.by == b.by && a.note == b.note);
    Ok(out)
}

/// **Borner le rendu à la portée** : les nœuds atteints hors de `scope` sont
/// retirés du rapport et comptés. Le parcours a déjà eu lieu : ils ont servi
/// de relais.
fn borner(moteur: &Moteur<'_>, atteints: Vec<Reached>, scope: &QueryScope) -> Result<(Vec<Reached>, usize), String> {
    let Some(def) = moteur.cfg.start.defined_by.as_ref().filter(|_| !scope.is_empty()) else { return Ok((atteints, 0)) };
    let table = rel_info(moteur.catalog, def)?.from;
    let uuids: Vec<String> = atteints.iter().map(|a| a.uuid.clone()).collect();
    let dedans = in_scope(moteur.catalog, &table, &moteur.cfg.start.path_fields, &uuids, scope)?;
    let avant = atteints.len();
    let gardes: Vec<Reached> = atteints.into_iter().filter(|a| dedans.contains(&a.uuid)).collect();
    let n = avant - gardes.len();
    Ok((gardes, n))
}

/// La clé de méta qu'une ingestion pose quand elle écrit des arêtes d'une
/// relation : ce qui distingue « aucun test par requête » de « jamais relié ».
pub fn written_key(relation: &str) -> String {
    format!("written:{relation}")
}

/// **Les tests par requêtes** (`queried_by`) : depuis les départs (niveau 0)
/// et chaque nœud atteint, le chemin déclaré ; au bout, les nœuds du groupe
/// déclaré qui ne sont pas déjà relevés, au niveau de leur origine plus un,
/// notés `queried_note`. Rend aussi si la relation du dernier pas n'a jamais
/// été écrite dans cet index.
fn interroges(moteur: &Moteur<'_>, departs: &[super::usage_nodes::Item], mut atteints: Vec<Reached>) -> Result<(Vec<Reached>, bool), String> {
    let cfg = moteur.cfg;
    let Some(dernier) = cfg.queried_by.last() else { return Ok((atteints, false)) };
    let jamais = dernier
        .alternatives
        .iter()
        .all(|rel| moteur.catalog.read_meta_key(&written_key(rel)).ok().flatten().is_none());
    if jamais {
        return Ok((atteints, true));
    }
    let mut niveau: HashMap<String, usize> = HashMap::new();
    for d in departs {
        niveau.insert(d.uuid.clone(), 0);
    }
    for a in &atteints {
        niveau.entry(a.uuid.clone()).or_insert(a.level);
    }
    let origines: Vec<(String, String)> = niveau.keys().map(|u| (u.clone(), u.clone())).collect();
    let bouts = follow_also(moteur.catalog, &cfg.queried_by, "", &origines)?;
    let deja: HashSet<String> = atteints.iter().map(|a| a.uuid.clone()).chain(departs.iter().map(|d| d.uuid.clone())).collect();
    let mut meilleur: HashMap<String, usize> = HashMap::new();
    for (bout, origine) in bouts {
        if deja.contains(&bout) {
            continue;
        }
        let l = niveau.get(&origine).copied().unwrap_or(0) + 1;
        let e = meilleur.entry(bout).or_insert(l);
        *e = (*e).min(l);
    }
    if meilleur.is_empty() {
        return Ok((atteints, false));
    }
    let uuids: Vec<String> = meilleur.keys().cloned().collect();
    for m in fiches(moteur, &uuids)? {
        if m.group.is_empty() {
            continue;
        }
        let level = meilleur.get(&m.uuid).copied().unwrap_or(1);
        atteints.push(Reached { level, relation: dernier.relation.clone(), note: cfg.queried_note.clone(), ..m });
    }
    Ok((atteints, false))
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
    neighborhood_of_in(catalog, cfg, name, path_prefix, depth, budget, max_degree, &QueryScope::default())
}

/// [`neighborhood_of`] borné à une portée : le parcours est le même (un nœud
/// hors portée reste un relais), seuls le départ et le rendu sont bornés, et
/// le compte des écartés est rendu.
#[allow(clippy::too_many_arguments)]
pub fn neighborhood_of_in(
    catalog: &Catalog,
    cfg: &NeighborhoodConfig,
    name: &str,
    path_prefix: &str,
    depth: usize,
    budget: usize,
    max_degree: usize,
    scope: &QueryScope,
) -> Result<NeighborhoodReport, String> {
    let depth = depth.clamp(1, MAX_DEPTH);
    let rels: Vec<RelInfo> = cfg.relations.iter().map(|r| rel_info(catalog, r)).collect::<Result<_, _>>()?;
    let moteur = Moteur { catalog, cfg, rels };

    // Le départ et le premier saut : comme `usages`, en union avec le
    // rendez-vous (dans le sens entrant seulement : c'est un usage).
    // En sortant, seules les définitions servent : les usages (arêtes
    // directes entrantes, rendez-vous) ne sont lus que dans le sens entrant.
    // Les lignes rendues avant et après sont les mêmes (empreintes des
    // lignes triées, `tests/e2e_mesure_impact.rs`, 11 octobre 2026).
    let depart = if cfg.direction == Direction::Incoming {
        std::borrow::Cow::Borrowed(&cfg.start)
    } else {
        std::borrow::Cow::Owned(UsagesConfig { direct: Vec::new(), used_by: None, ..cfg.start.clone() })
    };
    let usages = usages_of_in(catalog, &depart, name, path_prefix, scope)?;
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
    let (atteints, queries_untracked) = interroges(&moteur, &usages.definitions, atteints)?;
    let collected = relever(&moteur, &usages.definitions, &atteints)?;
    let (atteints, scope_hidden) = borner(&moteur, atteints, scope)?;
    Ok(NeighborhoodReport {
        name: name.to_string(),
        starts: usages.definitions,
        ambiguous: usages.ambiguous,
        reached: atteints,
        cut,
        depth,
        collected,
        queries_untracked,
        scope: scope.clone(),
        scope_hidden,
    })
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
    // Les scopes du fichier, par l'un de ses champs de chemin ; l'uuid
    // d'abord, puis les champs que lit `reached_from`.
    let condition = rag3weaver_ir::Predicate::AnyOf(
        cfg.start.path_fields.iter().map(|f| rag3weaver_ir::Predicate::Equals { field: f.clone(), param: "file".into() }).collect(),
    );
    let mut colonnes_depart = vec![Column::Node("_uuid".into())];
    colonnes_depart.extend(colonnes(cfg));
    let q = catalog.dialect_arc().select(&rag3weaver_ir::Select::filtered(&table, condition, colonnes_depart)).map_err(Moteur::err_ir)?;
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
    let (atteints, queries_untracked) = interroges(&moteur, &departs, atteints)?;
    let collected = relever(&moteur, &departs, &atteints)?;
    Ok(NeighborhoodReport {
        name: file.to_string(),
        starts: departs,
        ambiguous: false,
        reached: atteints,
        cut,
        depth,
        collected,
        queries_untracked,
        scope: QueryScope::default(),
        scope_hidden: 0,
    })
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
        // L'ordre des lignes rendues par la base suit l'ordre d'insertion,
        // qui suit des tables de hachage : sans tri, le rendu et ce que le
        // budget coupe changeaient d'un processus à l'autre. Le rang garde
        // la priorité d'avant (le premier vu l'emporte) : une arête, puis à
        // travers un carrefour, puis par le trait, puis par le nom seul.
        let mut candidats: Vec<Reached> = a_explorer.drain(..).collect();
        candidats.sort_by(|a, b| ordre_stable(a).cmp(&ordre_stable(b)));
        for mut m in plus_etroits(candidats) {
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

/// La clé du tri d'un niveau : le rang de la voie, puis le lieu, le titre et
/// l'uuid ; la relation et le carrefour départagent un même nœud atteint
/// deux fois.
fn ordre_stable(m: &Reached) -> (u8, &str, Option<i64>, &str, &str, &str, Option<&str>) {
    let rang = if m.by_name {
        3
    } else if m.by_also {
        2
    } else if m.through_hub.is_some() {
        1
    } else {
        0
    };
    (rang, &m.path, m.line, &m.title, &m.uuid, &m.relation, m.through_hub.as_deref())
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
    let q = moteur.catalog.dialect_arc().select(&rag3weaver_ir::Select::by_uuids(table, colonnes(moteur.cfg))).map_err(|e| format!("NeighborhoodNode: {e}"))?;
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
    /// `include_by_name` : les usages par le nom seul nommés et comptés comme
    /// les autres ; sinon, un compte à part.
    pub fn summary(&self, group_title: &str, rest_title: &str, limit: usize, only: &str, include_by_name: bool) -> String {
        if self.reached.is_empty() {
            return String::new();
        }
        // Comme le rendu complet : un usage par le nom seul n'est ni nommé ni
        // compté parmi les touchés, il a son compte à part.
        let cache = |m: &Reached| m.by_name && !include_by_name;
        let par_nom = self.reached.iter().filter(|m| cache(m) && !m.by_also).count();
        let groupes: Vec<&Reached> = self.grouped().into_iter().filter(|m| !cache(m) && (only.is_empty() || m.group == only)).collect();
        let reste: Vec<&Reached> = self.reached.iter().filter(|m| m.group.is_empty() && !cache(m)).collect();
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
        if par_nom > 0 {
            out.push_str(&format!(" ; {par_nom} par le nom seul, non montrés"));
        }
        if !self.scope.is_empty() {
            out.push_str(&format!(" ; portée {} : {} hors de la portée, écartés (traversés)", self.scope.describe(), self.scope_hidden));
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
        self.markdown_avec("impact", group_title, rest_title, also_label, None, limit, false)
    }

    /// [`Self::markdown`], sous ce titre (`heading`), avec la section du
    /// relevé sur le chemin, sous le sien ; `include_by_name` : les usages
    /// par le nom seul listés, marqués. Un `group_title` vide : pas de
    /// groupe, rien n'en est dit.
    pub fn markdown_avec(&self, heading: &str, group_title: &str, rest_title: &str, also_label: &str, collect_title: Option<&str>, limit: usize, include_by_name: bool) -> String {
        let mut out = format!("# {heading}: {}\n\n", self.name);
        out.push_str(&format!("## Départ ({})\n", self.starts.len()));
        if self.starts.is_empty() {
            out.push_str("(aucune définition indexée sous ce nom)\n");
        }
        for s in &self.starts {
            out.push_str(&format!("- {} {} — {}\n", s.kind, s.title, lieu(&s.path, s.line)));
        }
        if self.ambiguous {
            out.push_str(if include_by_name {
                "\nNom ambigu : toutes les définitions sont des départs ; les usages trouvés par le nom seul sont marqués « par le nom ».\n"
            } else {
                "\nNom ambigu : toutes les définitions sont des départs ; les usages trouvés par le nom seul sont comptés à part, non montrés.\n"
            });
        }
        // Un usage par le nom seul n'est attribué à aucune définition : il
        // noierait les sûrs (trente lignes pour deux appelants). Ni listé ni
        // compté parmi les touchés, il a sa ligne de compte.
        let cache = |r: &Reached| r.by_name && !include_by_name;
        let par_nom = self.reached.iter().filter(|r| cache(r) && !r.by_also).count();
        let groupes: Vec<&Reached> = self.grouped().into_iter().filter(|r| !cache(r)).collect();
        let resume_niveaux: Vec<String> = (1..=self.depth)
            .map(|l| format!("{} à {} saut{}", self.reached.iter().filter(|r| r.level == l && !r.by_also && !cache(r)).count(), l, if l > 1 { "s" } else { "" }))
            .collect();
        out.push_str(&format!(
            "\n**{} touchés** ({})",
            self.reached.iter().filter(|r| !r.by_also && !cache(r)).count(),
            resume_niveaux.join(", ")
        ));
        if !group_title.is_empty() {
            out.push_str(&format!(" — dont **{} {group_title}**", groupes.iter().filter(|r| !r.by_also).count()));
            if self.queries_untracked {
                // Un 0 sec mentirait : les tests qui passent par des requêtes
                // ne sont pas reliés dans cet index.
                out.push_str(" (tests par requêtes : non suivis — `workspace.queries_via` non déclaré pour cet index)");
            }
        }
        if self.cut > 0 {
            out.push_str(&format!(" ; budget atteint, {} de plus non rendus", self.cut));
        }
        if par_nom > 0 {
            out.push_str(&format!(" ; {par_nom} par le nom seul, non montrés"));
        }
        if !self.scope.is_empty() {
            out.push_str(&format!(" ; portée {} : {} hors de la portée, écartés (traversés)", self.scope.describe(), self.scope_hidden));
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
            let tous: Vec<&Reached> = self.reached.iter().filter(|r| r.level == l && !r.by_also && !cache(r) && r.group.is_empty()).collect();
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
        if let Some(titre) = collect_title {
            if !self.collected.is_empty() {
                let mut cibles: Vec<&str> = self.collected.iter().map(|c| c.target.as_str()).collect();
                cibles.dedup();
                out.push_str(&format!("\n## {titre} ({})\n", cibles.len()));
                for cible in cibles.iter().take(limit) {
                    let qui: Vec<String> = self
                        .collected
                        .iter()
                        .filter(|c| c.target == *cible)
                        .map(|c| {
                            let ou = if c.level == 0 { "départ".to_string() } else { format!("{} saut{}", c.level, if c.level > 1 { "s" } else { "" }) };
                            let note = if c.note.is_empty() { String::new() } else { format!(", {}", c.note) };
                            format!("{} ({ou}{note})", c.by)
                        })
                        .collect();
                    out.push_str(&format!("- `{cible}` — {}\n", qui.join(" ; ")));
                }
                if cibles.len() > limit {
                    out.push_str(&format!("… {} de plus\n", cibles.len() - limit));
                }
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
    /// La portée de la question (`scope`, `language`).
    scope: QueryScope,
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
    /// Les usages par le nom seul listés au lieu d'être comptés à part.
    include_by_name: bool,
    /// Le titre du rendu (`# impact: nom`).
    heading: String,
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
        use super::catalog_read::{read_catalog_in, refusal, with_status, CatalogRead};
        let catalog = ctx.service::<Arc<Mutex<Catalog>>>("catalog").cloned().ok_or("NeighborhoodNode: service 'catalog' absent")?;
        let (report, status) = match read_catalog_in(&catalog, &self.cfg.start.pivot, ctx.service::<crate::scope::Scope>("scope")) {
            CatalogRead::Refused(message) => {
                ctx.set_output("result", PortValue::new(refusal(&message, self.json)));
                return Ok(());
            }
            CatalogRead::Ready { catalog: cat, status } => {
                let r = if self.name.is_empty() {
                    neighborhood_of_file(&cat, &self.cfg, &self.file, self.depth, self.budget, self.max_degree)?
                } else {
                    neighborhood_of_in(&cat, &self.cfg, &self.name, &self.path, self.depth, self.budget, self.max_degree, &self.scope)?
                };
                (r, status)
            }
        };
        ctx.metric("reached", report.reached.len() as f64);
        ctx.metric("grouped", report.grouped().len() as f64);
        let value = if self.json {
            serde_json::to_value(&report).map_err(|e| e.to_string())?
        } else if self.summary {
            serde_json::Value::String(report.summary(&self.group_title, &self.rest_title, self.limit, &self.summary_group, self.include_by_name))
        } else {
            serde_json::Value::String(report.markdown_avec(
                &self.heading,
                // Sans regroupement déclaré, rien à en dire.
                if self.cfg.group_by.is_empty() { "" } else { &self.group_title },
                &self.rest_title,
                &self.cfg.also_label,
                self.cfg.collect.as_ref().map(|c| c.title.as_str()),
                self.limit,
                self.include_by_name,
            ))
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
pub fn pas(s: &str) -> Result<Step, String> {
    let (s, optional) = match s.strip_suffix('?') {
        Some(r) => (r, true),
        None => (s, false),
    };
    let (s, repeat) = match s.strip_suffix('+') {
        Some(r) => (r, true),
        None => (s, false),
    };
    let (r, direction) = if let Some(r) = s.strip_suffix('>') {
        (r, Direction::Outgoing)
    } else if let Some(r) = s.strip_prefix('<') {
        (r, Direction::Incoming)
    } else {
        return Err(format!("NeighborhoodNode: pas « {s} » : écrire REL> (sortant) ou <REL (entrant), A/B> pour plusieurs relations, + pour répéter, ? pour un pas facultatif"));
    };
    let alternatives: Vec<String> = r.split('/').map(str::trim).filter(|x| !x.is_empty()).map(String::from).collect();
    let relation = alternatives.first().cloned().ok_or_else(|| format!("NeighborhoodNode: pas « {s} » sans relation"))?;
    Ok(Step { relation, alternatives, direction, repeat, optional })
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
            qualifier_field: s("qualifier_field").filter(|v| !v.is_empty()),
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
        let queried_by = liste(config.get("queried_by")).iter().map(|p| pas(p)).collect::<Result<Vec<_>, _>>()?;
        check_also_path(&also_path).map_err(|e| format!("NeighborhoodNode: also_path : {e}"))?;
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
            queried_by,
            queried_note: s("queried_note").unwrap_or_else(|| "par requête".into()),
            collect: match s("collect").filter(|c| !c.is_empty()) {
                Some(c) => Some(Collect {
                    step: pas(&c)?,
                    field: s("collect_field").unwrap_or_else(|| "name".into()),
                    note: s("collect_note").unwrap_or_default(),
                    title: s("collect_title").unwrap_or_else(|| "Relevé sur le chemin".into()),
                }),
                None => None,
            },
        };
        // Des identifiants seulement : ils entrent dans le texte des requêtes.
        let ident = |x: &str| x.is_empty() || x.chars().all(|c| c.is_alphanumeric() || c == '_');
        let st = &cfg.start;
        let tous: Vec<&str> = [&st.pivot, &st.key, &st.group_by, &st.usages_field, &st.line, &st.title, &st.kind_field, &st.line_field, &cfg.group_by, &cfg.label_field, &cfg.note_field, &cfg.also_same]
            .into_iter()
            .map(String::as_str)
            .chain(st.defined_by.iter().map(String::as_str))
            .chain(st.used_by.iter().map(String::as_str))
            .chain(st.qualifier_field.iter().map(String::as_str))
            .chain(cfg.relations.iter().map(String::as_str))
            .chain(st.path_fields.iter().map(String::as_str))
            .chain(cfg.also_path.iter().flat_map(|p| p.alternatives.iter().map(String::as_str)))
            .chain(cfg.queried_by.iter().flat_map(|p| p.alternatives.iter().map(String::as_str)))
            .chain(cfg.collect.iter().flat_map(|c| [c.step.relation.as_str(), c.field.as_str(), c.note.as_str()]))
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
            scope: QueryScope::parse(&s("scope").unwrap_or_default(), &s("language").unwrap_or_default()),
            depth: entier("depth", 2).clamp(1, MAX_DEPTH as u64) as usize,
            budget: entier("budget", 200).clamp(1, MAX_BUDGET as u64) as usize,
            max_degree: entier("max_degree", 50).max(1) as usize,
            limit: entier("limit", 30).clamp(1, 500) as usize,
            group_title: s("group_title").unwrap_or_else(|| "Groupés".into()),
            rest_title: s("rest_title").unwrap_or_else(|| "Touchés".into()),
            include_by_name: config.get("include_by_name").and_then(|v| v.as_bool()).unwrap_or(false),
            heading: s("heading").unwrap_or_else(|| "impact".into()),
            json,
        }))
    }
    fn node_type(&self) -> &'static str {
        "NeighborhoodNode"
    }
    fn schema(&self) -> NodeSchema {
        let p = |name: &'static str, param_type, required, default: Option<serde_json::Value>, description: &'static str| ConfigParam {
            name: name.into(),
            param_type,
            required,
            default,
            description: description.into(),
            choices: None,
            json_schema: None,
        };
        use ConfigParamType::{Bool, Int, String as S};
        let mut direction = p("direction", S, false, Some(serde_json::json!("incoming")), "incoming (ce qui en dépend) | outgoing (ce dont il dépend)");
        direction.choices = Some(Choices::fixed(["incoming", "outgoing"]));
        let mut format = p("format", S, false, Some(serde_json::json!("markdown")), "markdown | json | summary (les comptes, puis les premiers du groupe)");
        format.choices = Some(Choices::fixed(["markdown", "json", "summary"]));
        NodeSchema {
            node_type: "NeighborhoodNode".into(),
            description: "The neighbourhood of a thing by levels: what depends on it directly, then at two and three hops, along declared relations, with a node budget, a degree cap that shows hubs without traversing them, a grouping field, and an optional extra start reached by a declared path (rendered apart).".into(),
            inputs: vec![],
            outputs: vec![PortDef::value("result", PortType::Map, false)],
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
                p("queried_by", S, false, Some(serde_json::json!("")), "Tests par requêtes : chemin depuis chaque nœud atteint, pas REL> ou <REL séparés par |, ? pour un pas facultatif"),
                p("queried_note", S, false, Some(serde_json::json!("par requête")), "La note d'un test relevé par ce chemin"),
                p("also_same", S, false, Some(serde_json::json!("name")), "Champ qui doit valoir celui du départ, au bout du chemin"),
                p("also_label", S, false, Some(serde_json::json!("Aussi, peut-être")), "Libellé du groupe du départ supplémentaire"),
                p("collect", S, false, Some(serde_json::json!("")), "Relevé sur le chemin : une relation et son sens, suivie d'un pas depuis chaque nœud atteint (`LOCKS>`)"),
                p("collect_field", S, false, Some(serde_json::json!("name")), "Champ montré de la cible du relevé"),
                p("collect_note", S, false, Some(serde_json::json!("")), "Champ de l'arête du relevé, montré entre parenthèses (`usage`)"),
                p("collect_title", S, false, Some(serde_json::json!("Relevé sur le chemin")), "Titre de la section du relevé"),
                p("usage_field", S, false, Some(serde_json::json!("usage")), "Propriété d'arête : genre"),
                p("usages_field", S, false, Some(serde_json::json!("usages")), "Propriété d'arête : ensemble des genres"),
                p("line", S, false, Some(serde_json::json!("line")), "Propriété d'arête : ligne du site"),
                p("title", S, false, Some(serde_json::json!("name")), "Champ titre"),
                p("kind_field", S, false, Some(serde_json::json!("scope_type")), "Champ genre"),
                p("path_fields", S, false, Some(serde_json::json!("file_path")), "Champs de chemin, le premier non vide, séparés par |"),
                p("line_field", S, false, Some(serde_json::json!("start_line")), "Champ de ligne"),
                p("group_title", S, false, Some(serde_json::json!("Groupés")), "Titre de la section des nœuds groupés"),
                p("rest_title", S, false, Some(serde_json::json!("Touchés")), "Titre des sections par niveau"),
                p("heading", S, false, Some(serde_json::json!("impact")), "Titre du rendu markdown (`# impact: nom`)"),
                p("include_by_name", Bool, false, Some(serde_json::json!(false)), "Lister les usages trouvés par le nom seul (non attribués), au lieu de les compter à part"),
                p("name", S, false, Some(serde_json::json!("")), "Le nom de départ (ou `file`)"),
                p("summary_group", S, false, Some(serde_json::json!("")), "Résumé : la seule valeur du groupe à compter et nommer (ex. case)"),
                p("file", S, false, Some(serde_json::json!("")), "Mode par fichier : départ de toutes les lignes dont un champ de chemin vaut ce chemin ; ce qui est dans le fichier ne compte pas"),
                p("path", S, false, Some(serde_json::json!("")), "Ne partir que des définitions sous ce chemin"),
                p("scope", S, false, Some(serde_json::json!("")), super::usage_nodes::SCOPE_HELP),
                p("language", S, false, Some(serde_json::json!("")), super::usage_nodes::LANGUAGE_HELP),
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

#[cfg(test)]
mod tests_saut {

    #[test]
    fn un_pas_se_lit_avec_ses_alternatives_et_sa_repetition() {
        let p = pas("IMPLEMENTS/INHERITS_FROM>+").unwrap();
        assert_eq!(p.alternatives, vec!["IMPLEMENTS".to_string(), "INHERITS_FROM".to_string()]);
        assert!(p.repeat && p.relation == "IMPLEMENTS");
        let chemin: Vec<Step> = ["HAS_PARENT>", "IMPLEMENTS/INHERITS_FROM>+", "PARENT_OF>"].iter().map(|x| pas(x).unwrap()).collect();
        assert!(check_also_path(&chemin).is_ok());
        let fautif: Vec<Step> = ["HAS_PARENT>", "PARENT_OF>+"].iter().map(|x| pas(x).unwrap()).collect();
        let e = check_also_path(&fautif).unwrap_err();
        assert!(e.contains("PARENT_OF") && e.contains("ne se répète pas"), "{e}");
    }

    use super::*;
    use crate::dialect::{Rag3dbDialect, SchemaDialect};
    use super::super::graph_walk::EdgeMark;

    fn cfg(group_by: &str, mark: EdgeMark) -> NeighborhoodConfig {
        NeighborhoodConfig {
            start: UsagesConfig {
                pivot: "Symbol".into(),
                key: "name".into(),
                defined_by: None,
                used_by: None,
                direct: vec![],
                group_by: "usage".into(),
                usages_field: "usages".into(),
                line: "line".into(),
                title: "name".into(),
                kind_field: "kind".into(),
                path_fields: vec!["file_path".into(), "source".into()],
                line_field: "start_line".into(),
                edge_mark: mark,
                qualifier_field: None,
            },
            relations: vec!["CONSUMES".into()],
            direction: Direction::Incoming,
            group_by: group_by.into(),
            label_field: String::new(),
            note_field: "test_certainty".into(),
            also_path: vec![],
            queried_by: vec![],
            queried_note: String::new(),
            also_same: "name".into(),
            also_label: String::new(),
            collect: None,
        }
    }

    /// La requête que `hop_query` écrivait avant de passer par le dialecte.
    fn requete_d_avant(cfg: &NeighborhoodConfig, rel: &RelInfo, direction: Direction) -> String {
        let ligne = if rel.props.iter().any(|p| p == &cfg.start.line) { format!("r.{}", crate::dialect::cypher_quote(&cfg.start.line)) } else { "NULL".into() };
        format!("UNWIND $uuids AS u MATCH {}{} RETURN u, {c}, {ligne}", direction.pattern(rel), cfg.start.edge_mark.clause(rel), c = champs("m", cfg))
    }

    /// Parité : au caractère près, pour les deux sens, avec et sans ligne
    /// sur l'arête, avec et sans arêtes devinées, champs déclarés ou non.
    #[test]
    fn le_saut_du_voisinage_est_la_requete_d_avant() {
        let mark = EdgeMark::from_config(&serde_json::json!({"edge_field": "resolution", "edge_guessed": "nom"}), "N").unwrap();
        let rels = [
            RelInfo { name: "CONSUMES".into(), from: "Scope".into(), to: "Scope".into(), props: vec!["usage".into(), "line".into(), "resolution".into()] },
            RelInfo { name: "TESTS".into(), from: "Test".into(), to: "Scope".into(), props: vec![] },
        ];
        for c in [cfg("", EdgeMark::default()), cfg("kind", mark)] {
            for r in &rels {
                for d in [Direction::Incoming, Direction::Outgoing] {
                    assert_eq!(Rag3dbDialect.hop(&hop_of(&c, r, d)).unwrap(), requete_d_avant(&c, r, d));
                }
            }
        }
    }

    /// Le pas « aussi » : `RETURN u, m._uuid`, et le champ comparé au dernier pas.
    #[test]
    fn le_pas_aussi_est_la_requete_d_avant() {
        let r = RelInfo { name: "DEFINES".into(), from: "File".into(), to: "Scope".into(), props: vec![] };
        for d in [Direction::Incoming, Direction::Outgoing] {
            let mut h = super::super::graph_walk::hop_of(&r, d, &EdgeMark::default());
            assert_eq!(Rag3dbDialect.hop(&h).unwrap(), format!("UNWIND $uuids AS u MATCH {} RETURN u, m.`_uuid`", d.pattern(&r)));
            h.returns.push(Column::Node("name".into()));
            assert_eq!(Rag3dbDialect.hop(&h).unwrap(), format!("UNWIND $uuids AS u MATCH {} RETURN u, m.`_uuid`, m.`name`", d.pattern(&r)));
        }
    }
}
