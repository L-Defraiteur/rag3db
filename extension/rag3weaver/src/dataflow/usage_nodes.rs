//! **`usages` : une chose, ce qui la définit, et ce qui s'en sert.**
//!
//! Le nœud est générique : un **pivot** (l'entité de rendez-vous, retrouvée
//! par son champ d'identité), la relation qui le **définit**, celle par
//! laquelle on s'en **sert**, et les relations **directes** vers chaque
//! définition. Pour le code : `Symbol` / `name`, `DEFINES`, `MENTIONS`, et
//! `CONSUMES` / `INHERITS_FROM` / `IMPLEMENTS` ; pour MTG, une capacité et
//! les cartes qui la portent. Rien ici ne nomme une entité ni une relation :
//! le gabarit les donne, le catalogue dit ce qu'elles relient.
//!
//! **Deux chemins, en union.** Pour le code, le rendez-vous (`MENTIONS`) ne
//! garde pas les références résolues dans leur propre fichier, et l'arête
//! directe (`CONSUMES`) n'existe que quand le nom n'a qu'une définition :
//! ni l'un ni l'autre seul ne rend tous les usages. Un usage trouvé par les
//! deux ne compte qu'une fois.
//!
//! **Un nom ambigu** (plusieurs définitions) : elles sont toutes montrées,
//! sans choisir ; les usages venus d'une arête directe sont rangés sous leur
//! définition, ceux venus du rendez-vous dans un groupe « non attribués ».
//!
//! **La forme des requêtes.** Le pivot se lit par une clé constante (l'index
//! de clé primaire) ; les voisins d'une liste d'uuid par `UNWIND $uuids AS u
//! MATCH (n:T {_uuid: u})`, une liste de valeurs simples, que le moteur joint
//! par hachage — jamais `item.champ`, qui le fait balayer la table par
//! produit cartésien (journal, §6, 3 octobre 2026).

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

use serde::Serialize;

use super::node::{Node, NodeContext};
use super::node_registry::{Choices, ConfigParam, ConfigParamType, NodeFactory, NodeSchema};
use super::port::{PortDef, PortType, PortValue};
use crate::catalog::{Catalog, CatalogError};
use crate::connection::{CypherValue, QueryParam};

/// Usages rendus par groupe, au plus.
pub const MAX_LIMIT: usize = 200;

/// Ce que le gabarit déclare : quelles entités, quelles relations, quels
/// champs et quelles propriétés d'arête lire.
#[derive(Debug, Clone)]
pub struct UsagesConfig {
    /// L'entité de rendez-vous et son champ d'identité.
    pub pivot: String,
    pub key: String,
    /// Relation entrante vers le pivot qui donne ses définitions ; sans elle,
    /// la fiche du pivot est la définition.
    pub defined_by: Option<String>,
    /// Relation entrante vers le pivot qui donne ses usages.
    pub used_by: Option<String>,
    /// Relations entrantes vers chaque définition qui sont des usages.
    pub direct: Vec<String>,
    /// Propriétés d'arête : le genre retenu, l'ensemble des genres, la ligne.
    pub group_by: String,
    pub usages_field: String,
    pub line: String,
    /// Champs des scopes montrés : le titre, le genre, les chemins (le
    /// premier non vide), la ligne de déclaration.
    pub title: String,
    pub kind_field: String,
    pub path_fields: Vec<String>,
    pub line_field: String,
    /// Les arêtes devinées : `usages` les dit, le voisinage ne les suit pas.
    pub edge_mark: super::graph_walk::EdgeMark,
    /// **Le nom qualifié** (`Classe::methode`, `module::fonction`,
    /// `Class.method`) : le champ de la définition qui porte son conteneur
    /// (pour le code, `parent_name`). Une définition convient quand son
    /// conteneur, ou le nom de son fichier, vaut le qualifiant. `None` : le
    /// nom se lit tel quel.
    pub qualifier_field: Option<String>,
}

/// Une définition, ou un usage.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Item {
    pub uuid: String,
    pub title: String,
    pub kind: String,
    pub path: String,
    /// Ligne de la déclaration (définition) ou du site (usage).
    pub line: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Usage {
    pub user: Item,
    /// Le genre retenu (`call`, `type`, …) ; `other` quand l'arête n'en dit rien.
    pub usage: String,
    pub usages: Vec<String>,
    /// La relation par laquelle on l'a trouvé.
    pub relation: String,
    /// L'uuid de la définition visée ; `None` pour un usage non attribué.
    pub definition: Option<String>,
    /// Trouvé par une arête devinée (`edge_guessed`) : peut-être.
    pub guessed: bool,
    /// Trouvé par une arête **ambiguë** (`resolution = ambigu`) : un des définisseurs que rien
    /// ne départageait, chacun relié. Dit, jamais tu.
    #[serde(default)]
    pub ambigu: bool,
    /// Trouvé par le rendez-vous, depuis un fichier C/C++ dont aucun définisseur n'est visible
    /// par les #include : la résolution s'est abstenue (`code::mentions_sans_definisseur_visible`).
    #[serde(default)]
    pub invisible: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct UsagesReport {
    pub name: String,
    pub definitions: Vec<Item>,
    /// Définitions écartées par le filtre de chemin.
    pub definitions_hidden: usize,
    pub usages: Vec<Usage>,
    pub ambiguous: bool,
    /// Le qualifiant lu dans un nom qualifié (`NodeTable` de
    /// `NodeTable::update`), et le nom cherché alors (`update`).
    pub qualifier: Option<(String, String)>,
    /// Définitions du nom seul écartées par le qualifiant.
    pub qualifier_hidden: usize,
    /// La portée demandée (vide : tout l'index), et ce qu'elle écarte.
    pub scope: QueryScope,
    pub scope_hidden_definitions: usize,
    pub scope_hidden_usages: usize,
    /// Usages C/C++ du rendez-vous sans définisseur visible par leurs #include (après la portée).
    #[serde(default)]
    pub invisible_usages: usize,
}

pub use crate::work_domain::QueryScope;

/// La description de `scope`, la même dans tous les outils.
pub const SCOPE_HELP: &str = "Borner la réponse à ces chemins relatifs à la racine (src/|test/) ; vide : tout l'index";
/// La description de `language`, la même dans tous les outils.
pub const LANGUAGE_HELP: &str = "Borner la réponse à ces langages (cpp|rust) ; vide : tous";

/// Le champ de langage de l'entité de code (`language`).
pub const LANGUAGE_FIELD: &str = "language";

/// **Les nœuds de `table` dans la portée**, par une lecture par clé de leurs
/// chemins et de leur langage (les colonnes des autres requêtes se lisent par
/// position : on n'y touche pas). Une portée vide admet tout.
pub fn in_scope(catalog: &dyn crate::read_view::ReadCatalog, table: &str, path_fields: &[String], uuids: &[String], scope: &QueryScope) -> Result<std::collections::HashSet<String>, String> {
    if scope.is_empty() || uuids.is_empty() {
        return Ok(uuids.iter().cloned().collect());
    }
    let mut colonnes: Vec<rag3weaver_ir::Column> = path_fields.iter().map(|f| rag3weaver_ir::Column::Node(f.clone())).collect();
    colonnes.push(rag3weaver_ir::Column::Node(LANGUAGE_FIELD.into()));
    let lire = rag3weaver_ir::Select::by_uuids(table, colonnes);
    let q = catalog.dialect_arc().select(&lire).map_err(|e| format!("portée : {e}"))?;
    let liste = CypherValue::List(uuids.iter().map(|u| CypherValue::String(u.clone())).collect());
    let rows = catalog.execute_read_with_params(&q, &[QueryParam::new("uuids", liste)]).map_err(|e| format!("portée : {e}"))?;
    let n = path_fields.len();
    Ok(rows
        .rows
        .iter()
        .filter(|r| {
            let chemins: Vec<String> = (0..n).map(|i| texte(r.get(1 + i))).collect();
            let refs: Vec<&str> = chemins.iter().map(String::as_str).collect();
            scope.admits(&refs, &texte(r.get(1 + n)))
        })
        .map(|r| texte(r.first()))
        .collect())
}

/// **Une déclaration** : la ligne d'un conteneur qui annonce un nom défini
/// ailleurs (`int bar(int x) const;` dans `class Foo` de foo.h, défini dans
/// foo.cpp) — ou jamais (une méthode virtuelle pure).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Declaration {
    /// La définition qu'elle annonce, quand elle est indexée.
    pub definition: Option<String>,
    pub container: String,
    pub path: String,
    pub line: Option<i64>,
    pub signature: String,
}

/// Où lire les déclarations : la relation d'une définition vers son
/// conteneur (`HAS_PARENT`), et le champ du conteneur qui les porte (une
/// liste JSON de `{name, line, signature, kind}`).
#[derive(Debug, Clone)]
pub struct DeclarationsConfig {
    pub relation: String,
    pub field: String,
}

/// Les déclarations d'un nom. Avec des définitions : celles de leurs
/// conteneurs. Sans : les conteneurs qui le déclarent sans qu'aucune
/// définition soit indexée. Rien tant que le schéma ne porte pas le champ.
pub fn declarations_of(catalog: &dyn crate::read_view::ReadCatalog, cfg: &UsagesConfig, d: &DeclarationsConfig, name: &str, definitions: &[Item]) -> Result<Vec<Declaration>, String> {
    let rel = rel_info(catalog, &d.relation)?;
    if !catalog.entity_config(&rel.to).is_some_and(|c| c.fields.contains_key(&d.field)) {
        return Ok(Vec::new());
    }
    let rows = if definitions.is_empty() {
        // Sans définition : les conteneurs qui déclarent le nom ; la première
        // colonne (la définition) est vide.
        let mut champs = vec![rag3weaver_ir::Column::Null, rag3weaver_ir::Column::Node(cfg.title.clone())];
        champs.extend(cfg.path_fields.iter().map(|f| rag3weaver_ir::Column::Node(f.clone())));
        champs.push(rag3weaver_ir::Column::Node(d.field.clone()));
        let q = catalog
            .dialect_arc()
            .select(&rag3weaver_ir::Select::filtered(&rel.to, rag3weaver_ir::Predicate::Contains { field: d.field.clone(), param: "nom".into() }, champs))
            .map_err(|e| format!("UsagesNode: {e}"))?;
        catalog
            .execute_read_with_params(&q, &[QueryParam::new("nom", CypherValue::String(format!("\"{name}\"")))])
            .map_err(|e| format!("UsagesNode: {e}"))?
    } else {
        // Le saut des définitions vers leurs conteneurs, dit par le dialecte ;
        // mêmes colonnes, dans le même ordre.
        let mut saut = rag3weaver_ir::Hop::new(&rel.from, &rel.name, &rel.to, rag3weaver_ir::Direction::Outgoing);
        saut.returns = std::iter::once(&cfg.title)
            .chain(cfg.path_fields.iter())
            .chain(std::iter::once(&d.field))
            .map(|f| rag3weaver_ir::Column::Node(f.clone()))
            .collect();
        let uuids = CypherValue::List(definitions.iter().map(|x| CypherValue::String(x.uuid.clone())).collect());
        super::graph_walk::run_hop(catalog, &saut, &[QueryParam::new("uuids", uuids)]).map_err(|e| format!("UsagesNode: {e}"))?
    };
    let n = cfg.path_fields.len();
    let mut out = Vec::new();
    for r in &rows.rows {
        let definition = Some(texte(r.first())).filter(|u| !u.is_empty());
        let container = texte(r.get(1));
        let path = (0..n).map(|i| texte(r.get(2 + i))).find(|p| !p.is_empty()).unwrap_or_default();
        let Ok(serde_json::Value::Array(entrees)) = serde_json::from_str::<serde_json::Value>(&texte(r.get(2 + n))) else { continue };
        for e in entrees.iter().filter(|e| e.get("name").and_then(|v| v.as_str()) == Some(name)) {
            out.push(Declaration {
                definition: definition.clone(),
                container: container.clone(),
                path: path.clone(),
                line: e.get("line").and_then(|v| v.as_i64()),
                signature: e.get("signature").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            });
        }
    }
    out.sort_by(|a, b| (&a.path, a.line).cmp(&(&b.path, b.line)));
    out.dedup();
    Ok(out)
}

// ─── Requêtes ────────────────────────────────────────────────────────────────

/// **L'envoi virtuel** : où chercher la méthode de base qu'une définition
/// redéfinit (le chemin d'`also_path`, `HAS_PARENT>|IMPLEMENTS/INHERITS_FROM>+|PARENT_OF>`),
/// et le champ qui doit valoir le nom au bout.
#[derive(Debug, Clone)]
pub struct VirtualConfig {
    pub path: Vec<super::neighborhood_nodes::Step>,
    pub same: String,
}

/// Une méthode de base que les définitions redéfinissent, et ses appelants :
/// chacun peut atteindre la redéfinition par envoi virtuel.
#[derive(Debug, Clone, Serialize)]
pub struct VirtualBase {
    pub base: Item,
    pub callers: Vec<Usage>,
}

/// Les appelants des méthodes de base que `definitions` redéfinissent
/// (classe de base ou trait, à tout niveau).
pub fn virtual_usages_of(catalog: &dyn crate::read_view::ReadCatalog, cfg: &UsagesConfig, v: &VirtualConfig, definitions: &[Item]) -> Result<Vec<VirtualBase>, String> {
    let err = |e: CatalogError| format!("UsagesNode: {e}");
    let departs: Vec<(String, String)> = definitions.iter().map(|d| (d.uuid.clone(), d.title.clone())).collect();
    let propres: std::collections::HashSet<&str> = definitions.iter().map(|d| d.uuid.as_str()).collect();
    let mut bases: Vec<String> = super::neighborhood_nodes::follow_also(catalog, &v.path, &v.same, &departs)?
        .into_iter()
        .map(|(u, _)| u)
        .filter(|u| !propres.contains(u.as_str()))
        .collect();
    bases.sort();
    bases.dedup();
    if bases.is_empty() {
        return Ok(Vec::new());
    }
    let liste = CypherValue::List(bases.iter().map(|u| CypherValue::String(u.clone())).collect());
    // Les bases elles-mêmes, pour les nommer.
    // La table des définitions : celle d'où part `defined_by` (le Scope).
    let table = match &cfg.defined_by {
        Some(d) => rel_info(catalog, d)?.from,
        None => cfg.pivot.clone(),
    };
    // Le dialecte rend l'uuid demandé en tête, puis les colonnes.
    let q = catalog.dialect_arc().select(&rag3weaver_ir::Select::by_uuids(&table, colonnes_item(cfg))).map_err(|e| format!("UsagesNode: {e}"))?;
    let rows = catalog.execute_read_with_params(&q, &[QueryParam::new("uuids", liste.clone())]).map_err(err)?;
    let mut out: Vec<VirtualBase> = rows.rows.iter().map(|r| VirtualBase { base: item(r, 1, cfg), callers: Vec::new() }).collect();
    // Leurs appelants directs.
    for name_rel in &cfg.direct {
        let rel = rel_info(catalog, name_rel)?;
        let rows = super::graph_walk::run_hop(catalog, &direct_usages_hop(cfg, &rel), &[QueryParam::new("uuids", liste.clone())])
            .map_err(|e| format!("UsagesNode: {e}"))?;
        for r in &rows.rows {
            let def = texte(r.first());
            let mut user = item(r, 1, cfg);
            let debut = 1 + 4 + cfg.path_fields.len();
            let (usage, ensemble, site) = usage_props(r, debut);
            if usage != "call" || propres.contains(user.uuid.as_str()) {
                continue;
            }
            user.line = site.or(user.line);
            let marque = texte(r.get(debut + 3));
            let guessed = cfg.edge_mark.is_guessed(&marque);
            let ambigu = cfg.edge_mark.is_doubtful(&marque);
            if let Some(b) = out.iter_mut().find(|b| b.base.uuid == def) {
                if !b.callers.iter().any(|c| c.user.uuid == user.uuid && c.user.line == user.line) {
                    b.callers.push(Usage { user, usage, usages: ensemble, relation: rel.name.clone(), definition: Some(def), guessed, ambigu, invisible: false });
                }
            }
        }
    }
    out.retain(|b| !b.callers.is_empty());
    for b in &mut out {
        b.callers.sort_by(|a, b| (&a.user.path, a.user.line, &a.user.title).cmp(&(&b.user.path, b.user.line, &b.user.title)));
    }
    out.sort_by(|a, b| (&a.base.path, a.base.line).cmp(&(&b.base.path, b.base.line)));
    Ok(out)
}

/// La section « par envoi virtuel » du rendu d'`usages`.
pub fn virtual_markdown(bases: &[VirtualBase], limit: usize) -> String {
    if bases.is_empty() {
        return String::new();
    }
    let n: usize = bases.iter().map(|b| b.callers.len()).sum();
    let mut out = format!("\n## Par envoi virtuel ({n}) — appels de la méthode de base, qui peuvent atteindre celle-ci\n");
    for b in bases {
        out.push_str(&format!("\n### {} {} — {}\n", b.base.kind, b.base.title, lieu(&b.base)));
        for u in b.callers.iter().take(limit) {
            let devine = if u.guessed { " (par le nom)" } else if u.ambigu { " (ambigu : un des homonymes)" } else { "" };
            out.push_str(&format!("- {} {} — {}{devine}\n", u.user.kind, u.user.title, lieu(&u.user)));
        }
        if b.callers.len() > limit {
            out.push_str(&format!("… {} de plus\n", b.callers.len() - limit));
        }
    }
    out
}

/// Le texte d'avant le dialecte, gardé pour la parité des tests — les noms
/// cités comme le dialecte rag3db les cite.
#[cfg(test)]
fn champs(alias: &str, cfg: &UsagesConfig) -> String {
    let q = crate::dialect::cypher_quote;
    let mut c = vec![format!("{alias}.{}", q("_uuid")), format!("{alias}.{}", q(&cfg.title)), format!("{alias}.{}", q(&cfg.kind_field))];
    c.extend(cfg.path_fields.iter().map(|p| format!("{alias}.{}", q(p))));
    c.push(format!("{alias}.{}", q(&cfg.line_field)));
    c.join(", ")
}

/// Le texte d'avant le dialecte, gardé pour la parité des tests — les noms
/// cités comme le dialecte rag3db les cite.
#[cfg(test)]
fn proprietes(rel: &RelInfo, cfg: &UsagesConfig) -> String {
    let mut c: Vec<String> = [&cfg.group_by, &cfg.usages_field, &cfg.line]
        .iter()
        .map(|p| if rel.props.iter().any(|x| x == *p) { format!("r.{}", crate::dialect::cypher_quote(p)) } else { "NULL".to_string() })
        .collect();
    c.push(cfg.edge_mark.column(rel));
    c.join(", ")
}

/// Les champs d'un scope rendus par un saut, dans l'ordre que lit [`item`].
fn colonnes_item(cfg: &UsagesConfig) -> Vec<rag3weaver_ir::Column> {
    use rag3weaver_ir::Column;
    let mut c = vec![Column::Node("_uuid".into()), Column::Node(cfg.title.clone()), Column::Node(cfg.kind_field.clone())];
    c.extend(cfg.path_fields.iter().map(|p| Column::Node(p.clone())));
    c.push(Column::Node(cfg.line_field.clone()));
    c
}

/// Les propriétés de l'arête, dans l'ordre que lit [`usage_props`], puis la
/// marque d'arête devinée ; vides quand la relation ne les porte pas.
fn colonnes_props(cfg: &UsagesConfig, rel: &RelInfo) -> Vec<rag3weaver_ir::Column> {
    use rag3weaver_ir::Column;
    let mut c: Vec<Column> = [&cfg.group_by, &cfg.usages_field, &cfg.line]
        .into_iter()
        .map(|p| if rel.props.iter().any(|x| x == p) { Column::Edge(p.clone()) } else { Column::Null })
        .collect();
    c.push(cfg.edge_mark.column_ir(rel));
    c
}

/// Les définitions des pivots (`$uuids`) : un saut entrant par la relation
/// de définition. La requête vient du dialecte.
pub fn definitions_hop(cfg: &UsagesConfig, defined_by: &RelInfo) -> rag3weaver_ir::Hop {
    let mut hop = rag3weaver_ir::Hop::new(&cfg.pivot, &defined_by.name, &defined_by.from, rag3weaver_ir::Direction::Incoming);
    hop.returns = colonnes_item(cfg);
    hop
}

/// Les usages par le rendez-vous : un saut entrant depuis les pivots, les
/// champs de l'usager puis les propriétés de l'arête.
pub fn pivot_usages_hop(cfg: &UsagesConfig, used_by: &RelInfo) -> rag3weaver_ir::Hop {
    let mut hop = rag3weaver_ir::Hop::new(&cfg.pivot, &used_by.name, &used_by.from, rag3weaver_ir::Direction::Incoming);
    hop.returns = colonnes_item(cfg);
    hop.returns.extend(colonnes_props(cfg, used_by));
    hop
}

/// Les usages directs des définitions : un saut entrant depuis leurs uuids,
/// les champs de l'usager puis les propriétés de l'arête (vides quand la
/// relation ne les porte pas). La requête vient du dialecte.
pub fn direct_usages_hop(cfg: &UsagesConfig, rel: &RelInfo) -> rag3weaver_ir::Hop {
    let mut hop = rag3weaver_ir::Hop::new(&rel.to, &rel.name, &rel.from, rag3weaver_ir::Direction::Incoming);
    hop.returns = colonnes_item(cfg);
    hop.returns.extend(colonnes_props(cfg, rel));
    hop
}

/// Une relation déclarée au catalogue : ses extrémités et ses propriétés.
#[derive(Debug, Clone)]
pub struct RelInfo {
    pub name: String,
    pub from: String,
    pub to: String,
    pub props: Vec<String>,
}

pub fn rel_info(catalog: &dyn crate::read_view::ReadCatalog, name: &str) -> Result<RelInfo, String> {
    let def = catalog.get_relation_def(name).ok_or_else(|| format!("UsagesNode: relation inconnue « {name} »"))?;
    Ok(RelInfo {
        name: name.to_string(),
        from: def.from.clone(),
        to: def.to.clone(),
        props: def.properties.as_ref().map(|p| p.keys().cloned().collect()).unwrap_or_default(),
    })
}

fn texte(v: Option<&CypherValue>) -> String {
    v.and_then(|v| v.as_str()).unwrap_or("").to_string()
}

fn item(row: &[CypherValue], debut: usize, cfg: &UsagesConfig) -> Item {
    let n = cfg.path_fields.len();
    let path = (0..n).map(|i| texte(row.get(debut + 3 + i))).find(|p| !p.is_empty()).unwrap_or_default();
    Item {
        uuid: texte(row.get(debut)),
        title: texte(row.get(debut + 1)),
        kind: texte(row.get(debut + 2)),
        path,
        line: row.get(debut + 3 + n).and_then(|v| v.as_i64()),
    }
}

/// Après les champs d'un scope : genre, ensemble des genres, ligne du site.
fn usage_props(row: &[CypherValue], debut: usize) -> (String, Vec<String>, Option<i64>) {
    let usage = texte(row.get(debut));
    let usages: Vec<String> = texte(row.get(debut + 1)).split(',').filter(|s| !s.is_empty()).map(String::from).collect();
    let line = row.get(debut + 2).and_then(|v| v.as_i64());
    (if usage.is_empty() { "other".to_string() } else { usage }, usages, line)
}

/// Le rapport : les définitions du nom, puis les usages par les deux chemins.
/// **Lire un nom qualifié** : `A::B::m` ou `A.m` → (`m`, le dernier qualifiant
/// `B` ou `A`). Sans `qualifier_field` déclaré, ou sans séparateur, le nom
/// tel quel.
pub fn lire_le_nom<'a>(name: &'a str, cfg: &UsagesConfig) -> (&'a str, Option<&'a str>) {
    if cfg.qualifier_field.is_none() {
        return (name, None);
    }
    let coupe = name.rfind("::").map(|i| (i, 2)).or_else(|| name.rfind('.').map(|i| (i, 1)));
    let Some((i, n)) = coupe else { return (name, None) };
    let (avant, nom) = (&name[..i], &name[i + n..]);
    let qualifiant = avant.rsplit("::").next().unwrap_or(avant).rsplit('.').next().unwrap_or(avant);
    if nom.is_empty() || qualifiant.is_empty() {
        return (name, None);
    }
    (nom, Some(qualifiant))
}

pub fn usages_of(catalog: &dyn crate::read_view::ReadCatalog, cfg: &UsagesConfig, name: &str, path_prefix: &str) -> Result<UsagesReport, String> {
    usages_of_in(catalog, cfg, name, path_prefix, &QueryScope::default())
}

/// [`usages_of`] borné à une portée : les définitions et les usagers hors de
/// `scope` sont écartés et comptés ; l'ambiguïté se juge dans la portée.
pub fn usages_of_in(catalog: &dyn crate::read_view::ReadCatalog, cfg: &UsagesConfig, name: &str, path_prefix: &str, scope: &QueryScope) -> Result<UsagesReport, String> {
    let err = |e: CatalogError| format!("UsagesNode: {e}");
    let nom_demande = name;
    let (name, qualifiant) = lire_le_nom(name, cfg);
    let entity = catalog.entity_config(&cfg.pivot).ok_or_else(|| format!("UsagesNode: entité inconnue « {} »", cfg.pivot))?;
    // Le pivot : par son uuid déterministe quand la clé est son identité.
    let pivots: Vec<String> = if entity.hashsafe.as_deref() == Some(std::slice::from_ref(&cfg.key)) {
        let data = BTreeMap::from([(cfg.key.clone(), CypherValue::String(name.to_string()))]);
        vec![catalog.entity_uuid(&cfg.pivot, &data).map_err(err)?]
    } else {
        let q = catalog
            .dialect_arc()
            .select(&rag3weaver_ir::Select::filtered(
                &cfg.pivot,
                rag3weaver_ir::Predicate::Equals { field: cfg.key.clone(), param: "name".into() },
                vec![rag3weaver_ir::Column::Node("_uuid".into())],
            ))
            .map_err(|e| format!("UsagesNode: {e}"))?;
        let rows = catalog.execute_read_with_params(&q, &[QueryParam::new("name", CypherValue::String(name.to_string()))]).map_err(err)?;
        rows.rows.iter().map(|r| texte(r.first())).filter(|u| !u.is_empty()).collect()
    };

    // Les définitions.
    let mut definitions: Vec<Item> = Vec::new();
    if let Some(def) = &cfg.defined_by {
        let rel = rel_info(catalog, def)?;
        let saut = definitions_hop(cfg, &rel);
        // Un pivot à la fois, dans leur ordre ; la première colonne est l'uuid
        // du pivot, la définition vient après.
        for u in &pivots {
            let rows = super::graph_walk::run_hop(catalog, &saut, &[super::graph_walk::uuid_param(std::slice::from_ref(u))])
                .map_err(|e| format!("UsagesNode: {e}"))?;
            definitions.extend(rows.rows.iter().map(|r| item(r, 1, cfg)));
        }
    }
    definitions.sort_by(|a, b| (&a.path, a.line, &a.uuid).cmp(&(&b.path, b.line, &b.uuid)));
    definitions.dedup_by(|a, b| a.uuid == b.uuid);
    // La portée d'abord : une définition hors de la question ne la rend pas
    // ambiguë.
    let table_du_code = match &cfg.defined_by {
        Some(def) if !scope.is_empty() => Some(rel_info(catalog, def)?.from),
        _ => None,
    };
    let mut scope_hidden_definitions = 0;
    if let Some(table) = &table_du_code {
        let uuids: Vec<String> = definitions.iter().map(|d| d.uuid.clone()).collect();
        let dedans = in_scope(catalog, table, &cfg.path_fields, &uuids, scope)?;
        let avant = definitions.len();
        definitions.retain(|d| dedans.contains(&d.uuid));
        scope_hidden_definitions = avant - definitions.len();
    }
    let ambiguous = definitions.len() > 1;
    let total = definitions.len();
    if !path_prefix.is_empty() {
        definitions.retain(|d| sous_le_chemin(&d.path, path_prefix));
    }
    let definitions_hidden = total - definitions.len();

    // **Le qualifiant** : le conteneur de chaque définition, lu à part (les
    // colonnes des autres requêtes se lisent par position), puis le filtre.
    let mut qualifier_hidden = 0;
    if let (Some(q), Some(champ), Some(def)) = (qualifiant, cfg.qualifier_field.as_ref(), cfg.defined_by.as_ref()) {
        if !definitions.is_empty() {
            let table = rel_info(catalog, def)?.from;
            let lire = rag3weaver_ir::Select::by_uuids(&table, vec![rag3weaver_ir::Column::Node(champ.clone())]);
            let qs = catalog.dialect_arc().select(&lire).map_err(|e| format!("UsagesNode: {e}"))?;
            let liste = CypherValue::List(definitions.iter().map(|d| CypherValue::String(d.uuid.clone())).collect());
            let rows = catalog.execute_read_with_params(&qs, &[QueryParam::new("uuids", liste)]).map_err(err)?;
            let conteneur: HashMap<String, String> = rows.rows.iter().map(|r| (texte(r.first()), texte(r.get(1)))).collect();
            let fichier = |chemin: &str| {
                let base = chemin.rsplit('/').next().unwrap_or(chemin);
                base.split('.').next().unwrap_or(base).to_string()
            };
            let avant = definitions.len();
            definitions.retain(|d| {
                conteneur.get(&d.uuid).is_some_and(|c| c == q) || fichier(&d.path) == q || d.path.contains(&format!("/{q}/"))
            });
            qualifier_hidden = avant - definitions.len();
        }
    }

    // Les usages directs, sous leur définition.
    let mut usages: Vec<Usage> = Vec::new();
    let mut vus: HashMap<(String, Option<String>), usize> = HashMap::new();
    let def_uuids: Vec<CypherValue> = definitions.iter().map(|d| CypherValue::String(d.uuid.clone())).collect();
    if !def_uuids.is_empty() {
        for name_rel in &cfg.direct {
            let rel = rel_info(catalog, name_rel)?;
            let rows = super::graph_walk::run_hop(catalog, &direct_usages_hop(cfg, &rel), &[QueryParam::new("uuids", CypherValue::List(def_uuids.clone()))])
                .map_err(|e| format!("UsagesNode: {e}"))?;
            for r in &rows.rows {
                let def = texte(r.first());
                let mut user = item(r, 1, cfg);
                let debut = 1 + 4 + cfg.path_fields.len();
                let (usage, ensemble, site) = usage_props(r, debut);
                // Une arête d'une définition vers elle-même : seul l'appel
                // récursif en pose une, et c'est un appelant.
                if user.uuid == def && usage != "call" {
                    continue;
                }
                user.line = site.or(user.line);
                let marque = texte(r.get(debut + 3));
                let guessed = cfg.edge_mark.is_guessed(&marque);
                let ambigu = cfg.edge_mark.is_doubtful(&marque);
                ajouter(&mut usages, &mut vus, Usage { user, usage, usages: ensemble, relation: rel.name.clone(), definition: Some(def), guessed, ambigu, invisible: false });
            }
        }
    }

    // Les usages par le rendez-vous : attribués à la définition s'il n'y en a
    // qu'une, sinon « non attribués ».
    if let Some(used) = &cfg.used_by {
        let rel = rel_info(catalog, used)?;
        let saut = pivot_usages_hop(cfg, &rel);
        let unique = if ambiguous { None } else { definitions.first().map(|d| d.uuid.clone()) };
        let def_set: std::collections::HashSet<&str> = definitions.iter().map(|d| d.uuid.as_str()).collect();
        for u in &pivots {
            let rows = super::graph_walk::run_hop(catalog, &saut, &[super::graph_walk::uuid_param(std::slice::from_ref(u))])
                .map_err(|e| format!("UsagesNode: {e}"))?;
            for r in &rows.rows {
                let mut user = item(r, 1, cfg);
                if def_set.contains(user.uuid.as_str()) {
                    continue;
                }
                let (usage, ensemble, site) = usage_props(r, 1 + 4 + cfg.path_fields.len());
                user.line = site.or(user.line);
                // Déjà trouvé par une arête directe : il compte une fois.
                if usages.iter().any(|x| x.user.uuid == user.uuid && (x.definition == unique || unique.is_none())) {
                    continue;
                }
                // Le rendez-vous n'a rien relié : attribué au seul définisseur
                // pour le rangement, mais par le nom seul — dit tel.
                ajouter(&mut usages, &mut vus, Usage { user, usage, usages: ensemble, relation: rel.name.clone(), definition: unique.clone(), guessed: true, ambigu: false, invisible: false });
            }
        }
    }
    // **Ce que la résolution a tu se dit** : un usager du rendez-vous venu d'un fichier C/C++,
    // dont aucune définition n'est visible par ses #include, s'est abstenu ; il reste listé, marqué.
    #[cfg(feature = "code")]
    if let Some(used) = &cfg.used_by {
        let defs: Vec<&str> = definitions.iter().map(|d| d.uuid.as_str()).collect();
        let idx: Vec<usize> = (0..usages.len()).filter(|&i| usages[i].guessed && &usages[i].relation == used).collect();
        if !idx.is_empty() {
            let couples: Vec<(&str, Vec<&str>)> = idx.iter().map(|&i| (usages[i].user.uuid.as_str(), defs.clone())).collect();
            let invisibles = crate::code::mentions_sans_definisseur_visible(catalog, &couples).map_err(err)?;
            for (&i, inv) in idx.iter().zip(invisibles) {
                usages[i].invisible = inv;
            }
        }
    }
    // Un même site vu par la méthode et par la classe (ou le module) qui la
    // contient — un conteneur garde les références de ses membres : on ne
    // garde que le scope le plus étroit.
    let conteneur = |k: &str| matches!(k, "class" | "interface" | "namespace" | "module");
    let sites_etroits: std::collections::HashSet<(String, Option<i64>, Option<String>)> = usages
        .iter()
        .filter(|u| !conteneur(&u.user.kind))
        .map(|u| (u.user.path.clone(), u.user.line, u.definition.clone()))
        .collect();
    usages.retain(|u| !conteneur(&u.user.kind) || !sites_etroits.contains(&(u.user.path.clone(), u.user.line, u.definition.clone())));
    let mut scope_hidden_usages = 0;
    if let Some(table) = &table_du_code {
        let uuids: Vec<String> = usages.iter().map(|u| u.user.uuid.clone()).collect();
        let dedans = in_scope(catalog, table, &cfg.path_fields, &uuids, scope)?;
        let avant = usages.len();
        usages.retain(|u| dedans.contains(&u.user.uuid));
        scope_hidden_usages = avant - usages.len();
    }
    usages.sort_by(|a, b| (&a.user.path, a.user.line, &a.user.title).cmp(&(&b.user.path, b.user.line, &b.user.title)));
    let invisible_usages = usages.iter().filter(|u| u.invisible).count();
    Ok(UsagesReport {
        name: nom_demande.to_string(),
        definitions,
        definitions_hidden,
        usages,
        ambiguous,
        qualifier: qualifiant.map(|q| (q.to_string(), name.to_string())),
        qualifier_hidden,
        scope: scope.clone(),
        scope_hidden_definitions,
        scope_hidden_usages,
        invisible_usages,
    })
}

/// Le chemin d'un scope commence par `prefixe`. Les deux sont relatifs à la
/// racine et en « / » depuis le 11 octobre 2026 (`workspace_path`) : le
/// préfixe tapé est normalisé comme le chemin rangé. L'appartenance « après
/// un `/` » reste, pour une base d'avant la règle (chemins absolus), le temps
/// qu'elle se réindexe.
fn sous_le_chemin(chemin: &str, prefixe: &str) -> bool {
    let p = crate::workspace_path::slash_prefix(prefixe);
    chemin.starts_with(&p) || chemin.contains(&format!("/{p}"))
}

fn ajouter(usages: &mut Vec<Usage>, vus: &mut HashMap<(String, Option<String>), usize>, u: Usage) {
    let cle = (u.user.uuid.clone(), u.definition.clone());
    if let Some(&i) = vus.get(&cle) {
        // Le même scope par deux relations : on garde le genre le plus fort.
        let rang = |g: &str| ["inheritance", "call", "type", "import", "other"].iter().position(|x| *x == g).unwrap_or(5);
        if rang(&u.usage) < rang(&usages[i].usage) {
            usages[i].usage = u.usage;
        }
        for g in u.usages {
            if !usages[i].usages.contains(&g) {
                usages[i].usages.push(g);
            }
        }
        return;
    }
    vus.insert(cle, usages.len());
    usages.push(u);
}

// ─── Rendu ───────────────────────────────────────────────────────────────────

fn lieu(i: &Item) -> String {
    match i.line {
        Some(l) => format!("{}:{l}", i.path),
        None => i.path.clone(),
    }
}

impl UsagesReport {
    /// Les usages retenus par le filtre de genre (`all` : tous).
    pub fn filtered(&self, usage: &str) -> Vec<&Usage> {
        self.usages
            .iter()
            .filter(|u| usage == "all" || u.usage == usage || u.usages.iter().any(|g| g == usage))
            .collect()
    }

    pub fn markdown(&self, usage: &str, limit: usize) -> String {
        self.markdown_with(usage, limit, &[])
    }

    /// Le rendu, avec les déclarations : « défini foo.cpp:3 — déclaré
    /// foo.h:5 (dans Foo) » ; sans définition indexée, la déclaration seule.
    pub fn markdown_with(&self, usage: &str, limit: usize, declarations: &[Declaration]) -> String {
        let mut out = format!("# usages: {}\n\n", self.name);
        // Le nom qualifié se dit : comment il a été lu, et ce qu'il écarte.
        if let Some((q, nom)) = &self.qualifier {
            out.push_str(&format!("Nom qualifié : « {nom} » dans « {q} » (conteneur ou fichier)"));
            if self.qualifier_hidden > 0 {
                out.push_str(&format!(" — {} définition(s) de « {nom} » ailleurs, écartée(s)", self.qualifier_hidden));
            }
            out.push_str(".\n\n");
        }
        if !self.scope.is_empty() {
            out.push_str(&format!(
                "Portée : {} — {} définition(s) et {} usage(s) hors de la portée, écartés.\n\n",
                self.scope.describe(),
                self.scope_hidden_definitions,
                self.scope_hidden_usages
            ));
        }
        if self.invisible_usages > 0 {
            out.push_str(&format!(
                "{} mention(s) C/C++ sans définisseur visible par leurs #include : un en-tête non indexé, ou une définition dans un autre fichier — marquée(s) « (invisible par les #include) », non reliée(s).\n\n",
                self.invisible_usages
            ));
        }
        let defs = &self.definitions;
        out.push_str(&format!("## Définitions ({})", defs.len()));
        if self.definitions_hidden > 0 {
            out.push_str(&format!(" — {} autre(s) hors du chemin demandé", self.definitions_hidden));
        }
        out.push('\n');
        let lieu_decl = |x: &Declaration| match x.line {
            Some(l) => format!("{}:{l}", x.path),
            None => x.path.clone(),
        };
        if defs.is_empty() {
            let seules: Vec<&Declaration> = declarations.iter().filter(|x| x.definition.is_none()).collect();
            if seules.is_empty() && self.qualifier.is_some() && self.qualifier_hidden > 0 {
                out.push_str("(aucune définition dans ce conteneur : donnez le nom seul, avec un chemin, pour les voir toutes)\n");
            } else if seules.is_empty() {
                out.push_str("(aucune définition indexée sous ce nom)\n");
            } else {
                out.push_str("(aucune définition indexée — déclaré seulement)\n");
                for x in seules {
                    out.push_str(&format!("- déclaré dans {} — {} : `{}`\n", x.container, lieu_decl(x), x.signature));
                }
            }
        }
        for d in defs {
            let decl = declarations.iter().find(|x| x.definition.as_deref() == Some(d.uuid.as_str()));
            match decl {
                Some(x) => out.push_str(&format!("- {} {} — {} — déclaré {} (dans {})\n", d.kind, d.title, lieu(d), lieu_decl(x), x.container)),
                None => out.push_str(&format!("- {} {} — {}\n", d.kind, d.title, lieu(d))),
            }
        }
        if self.ambiguous {
            out.push_str("\nNom ambigu : plusieurs définitions. Les usages trouvés par le nom seul ne sont pas attribués.\n");
        }
        let retenus = self.filtered(usage);
        let mut par_genre: BTreeMap<&str, Vec<&Usage>> = BTreeMap::new();
        let mut non_attribues: Vec<&Usage> = Vec::new();
        for u in &retenus {
            if u.definition.is_none() {
                non_attribues.push(u);
            } else {
                par_genre.entry(u.usage.as_str()).or_default().push(u);
            }
        }
        let resume: Vec<String> = ["call", "type", "import", "inheritance", "other"]
            .iter()
            .filter_map(|g| par_genre.get(g).map(|v| format!("{g} {}", v.len())))
            .collect();
        out.push_str(&format!("\n## Usages ({})", retenus.len()));
        if !resume.is_empty() {
            out.push_str(&format!(" — {}", resume.join(" · ")));
        }
        if usage != "all" {
            out.push_str(&format!(" (filtre : {usage})"));
        }
        out.push('\n');
        let groupe = |titre: &str, v: &[&Usage], out: &mut String| {
            out.push_str(&format!("\n### {titre} ({})\n", v.len()));
            for u in v.iter().take(limit) {
                let cible = if defs.len() > 1 {
                    u.definition.as_ref().and_then(|d| defs.iter().find(|x| &x.uuid == d)).map(|d| format!(" → {}", lieu(d))).unwrap_or_default()
                } else {
                    String::new()
                };
                let devine = if u.invisible {
                    " (invisible par les #include)"
                } else if u.guessed {
                    " (par le nom)"
                } else if u.ambigu { " (ambigu : un des homonymes)" } else { "" };
                out.push_str(&format!("- {} {} — {}{cible}{devine}\n", u.user.kind, u.user.title, lieu(&u.user)));
            }
            if v.len() > limit {
                out.push_str(&format!("… {} de plus\n", v.len() - limit));
            }
        };
        for g in ["call", "type", "import", "inheritance", "other"] {
            if let Some(v) = par_genre.get(g) {
                groupe(g, v, &mut out);
            }
        }
        if !non_attribues.is_empty() {
            groupe("non attribués (nom ambigu)", &non_attribues, &mut out);
        }
        out
    }
}

// ─── Le nœud ─────────────────────────────────────────────────────────────────

/// `usages` : **output** `result` (PortType::Map) — du markdown, ou le rapport
/// en JSON. Service : `catalog`.
pub struct UsagesNode {
    node_name: String,
    cfg: UsagesConfig,
    name: String,
    path: String,
    /// La portée de la question (`scope`, `language`).
    scope: QueryScope,
    usage: String,
    limit: usize,
    /// `true` : le rapport en JSON ; sinon du markdown.
    json: bool,
    /// Où lire les déclarations (`declared_in`), quand le gabarit le dit.
    declarations: Option<DeclarationsConfig>,
    /// L'envoi virtuel (`virtual_path`), quand le gabarit le dit.
    virtuel: Option<VirtualConfig>,
}

impl Node for UsagesNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "UsagesNode"
    }
    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        Some(Box::new(serde_json::json!({ "name": self.name, "path": self.path, "usage": self.usage, "limit": self.limit })))
    }
    fn outputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&UsagesNodeFactory).1
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        use super::catalog_read::{read_for_tool, refusal, with_status};
        let catalog = ctx.service::<Arc<Mutex<Catalog>>>("catalog").cloned().ok_or("UsagesNode: service 'catalog' absent")?;
        // Jamais un vide sans dire d'où il vient : occupé, jamais indexé,
        // ou partiel, le rendu le dit.
        // Par la vue de lecture quand elle est publiée : sans attendre
        // l'écrivain (lot L2).
        let lecture = read_for_tool(&catalog, ctx.service::<Arc<crate::read_view::ReadViewCell>>(crate::read_view::READ_VIEW_SERVICE), &self.cfg.pivot, ctx.service::<crate::scope::Scope>("scope"));
        let (report, status) = match lecture.into_reader() {
            Err(message) => {
                ctx.set_output("result", PortValue::new(refusal(&message, self.json)));
                return Ok(());
            }
            Ok((source, status)) => {
                let cat = source.get();
                let report = usages_of_in(cat, &self.cfg, &self.name, &self.path, &self.scope)?;
                let declarations = match &self.declarations {
                    Some(d) => declarations_of(cat, &self.cfg, d, &self.name, &report.definitions)?,
                    None => Vec::new(),
                };
                let virtuels = match &self.virtuel {
                    Some(v) => virtual_usages_of(cat, &self.cfg, v, &report.definitions)?,
                    None => Vec::new(),
                };
                ((report, declarations, virtuels), status)
            }
        };
        let (report, declarations, virtuels) = report;
        ctx.metric("definitions", report.definitions.len() as f64);
        ctx.metric("usages", report.usages.len() as f64);
        let value = if self.json {
            let mut v = serde_json::to_value(&report).map_err(|e| e.to_string())?;
            if !declarations.is_empty() {
                v["declarations"] = serde_json::to_value(&declarations).map_err(|e| e.to_string())?;
            }
            if !virtuels.is_empty() {
                v["virtual"] = serde_json::to_value(&virtuels).map_err(|e| e.to_string())?;
            }
            v
        } else {
            serde_json::Value::String(report.markdown_with(&self.usage, self.limit, &declarations) + virtual_markdown(&virtuels, self.limit).as_str())
        };
        ctx.set_output("result", PortValue::new(with_status(status.as_deref(), value, self.json)));
        Ok(())
    }
}

pub struct UsagesNodeFactory;

fn liste(v: Option<&serde_json::Value>) -> Vec<String> {
    v.and_then(|v| v.as_str()).unwrap_or("").split(['|', ',']).map(str::trim).filter(|s| !s.is_empty()).map(String::from).collect()
}

impl NodeFactory for UsagesNodeFactory {
    // L6 : ne fait que lire.
    fn snapshot_safe(&self, _config: &serde_json::Value) -> bool {
        true
    }
    fn create(&self, name: &str, config: &serde_json::Value) -> Result<Box<dyn Node>, String> {
        let s = |k: &str| config.get(k).and_then(|v| v.as_str()).map(str::to_string);
        let requis = |k: &str| s(k).filter(|v| !v.is_empty()).ok_or_else(|| format!("UsagesNode: « {k} » requis"));
        let cfg = UsagesConfig {
            pivot: requis("pivot")?,
            key: requis("key")?,
            defined_by: s("defined_by").filter(|v| !v.is_empty()),
            used_by: s("used_by").filter(|v| !v.is_empty()),
            direct: liste(config.get("direct")),
            group_by: s("group_by").unwrap_or_else(|| "usage".into()),
            usages_field: s("usages_field").unwrap_or_else(|| "usages".into()),
            line: s("line").unwrap_or_else(|| "line".into()),
            title: s("title").unwrap_or_else(|| "name".into()),
            kind_field: s("kind_field").unwrap_or_else(|| "scope_type".into()),
            path_fields: {
                let p = liste(config.get("path_fields"));
                if p.is_empty() { vec!["file_path".into()] } else { p }
            },
            line_field: s("line_field").unwrap_or_else(|| "start_line".into()),
            edge_mark: super::graph_walk::EdgeMark::from_config(config, "UsagesNode")?,
            qualifier_field: s("qualifier_field").filter(|v| !v.is_empty()),
        };
        // Des identifiants seulement : ils entrent dans le texte des requêtes.
        let ident = |x: &str| !x.is_empty() && x.chars().all(|c| c.is_alphanumeric() || c == '_');
        let tous: Vec<&String> = [&cfg.pivot, &cfg.key, &cfg.group_by, &cfg.usages_field, &cfg.line, &cfg.title, &cfg.kind_field, &cfg.line_field]
            .into_iter()
            .chain(cfg.defined_by.iter())
            .chain(cfg.used_by.iter())
            .chain(cfg.qualifier_field.iter())
            .chain(cfg.direct.iter())
            .chain(cfg.path_fields.iter())
            .collect();
        if let Some(x) = tous.iter().find(|x| !ident(x)) {
            return Err(format!("UsagesNode: « {x} » n'est pas un identifiant"));
        }
        let usage = s("usage").filter(|v| !v.is_empty()).unwrap_or_else(|| "all".into());
        if !["all", "call", "type", "import", "inheritance", "other"].contains(&usage.as_str()) {
            return Err(format!("UsagesNode: genre inconnu « {usage} »"));
        }
        let limit = config.get("limit").and_then(|v| v.as_u64()).unwrap_or(20).clamp(1, MAX_LIMIT as u64) as usize;
        let json = match s("format").as_deref() {
            None | Some("") | Some("markdown") => false,
            Some("json") => true,
            Some(f) => return Err(format!("UsagesNode: format inconnu « {f} »")),
        };
        let declarations = s("declared_in").filter(|v| !v.is_empty()).map(|relation| DeclarationsConfig {
            relation,
            field: s("declarations_field").filter(|v| !v.is_empty()).unwrap_or_else(|| "declarations".into()),
        });
        if let Some(d) = &declarations {
            let ident = |x: &str| x.chars().all(|c| c.is_alphanumeric() || c == '_');
            if let Some(x) = [&d.relation, &d.field].into_iter().find(|x| !ident(x)) {
                return Err(format!("UsagesNode: « {x} » n'est pas un identifiant"));
            }
        }
        let virtuel = match s("virtual_path").filter(|v| !v.is_empty()) {
            Some(p) => {
                let path = liste(Some(&serde_json::Value::String(p))).iter().map(|x| super::neighborhood_nodes::pas(x)).collect::<Result<Vec<_>, _>>()?;
                let same = s("virtual_same").filter(|v| !v.is_empty()).unwrap_or_else(|| cfg.title.clone());
                if let Some(x) = path.iter().flat_map(|p| p.alternatives.iter()).chain(std::iter::once(&same)).find(|x| !ident(x)) {
                    return Err(format!("UsagesNode: « {x} » n'est pas un identifiant"));
                }
                super::neighborhood_nodes::check_also_path(&path).map_err(|e| format!("UsagesNode: virtual_path : {e}"))?;
                Some(VirtualConfig { path, same })
            }
            None => None,
        };
        Ok(Box::new(UsagesNode {
            node_name: name.to_string(),
            cfg,
            virtuel,
            name: requis("name")?,
            path: s("path").unwrap_or_default(),
            scope: QueryScope::parse(&s("scope").unwrap_or_default(), &s("language").unwrap_or_default()),
            usage,
            limit,
            json,
            declarations,
        }))
    }
    fn node_type(&self) -> &'static str {
        "UsagesNode"
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
        use ConfigParamType::{Int, String as S};
        let mut usage = p("usage", S, false, Some(serde_json::json!("all")), "Ne garder qu'un genre d'usage");
        usage.choices = Some(Choices::fixed(["all", "call", "type", "import", "inheritance", "other"]));
        let mut format = p("format", S, false, Some(serde_json::json!("markdown")), "markdown (pour le modèle) | json (structuré)");
        format.choices = Some(Choices::fixed(["markdown", "json"]));
        NodeSchema {
            node_type: "UsagesNode".into(),
            description: "A thing, what defines it, and what uses it: through a declared pivot entity (definitions and usages relations) and direct relations to each definition, grouped by the usage kind read on the edge, with file and line. Ambiguous names show every definition and leave pivot-only usages unattributed.".into(),
            inputs: vec![],
            outputs: vec![PortDef::value("result", PortType::Map, false)],
            config_params: vec![
                p("pivot", S, true, None, "Entité de rendez-vous (ex. Symbol)"),
                p("key", S, true, None, "Champ d'identité du pivot (ex. name)"),
                p("defined_by", S, false, None, "Relation entrante vers le pivot qui donne ses définitions"),
                p("used_by", S, false, None, "Relation entrante vers le pivot qui donne ses usages"),
                p("direct", S, false, None, "Relations entrantes vers chaque définition qui sont des usages, séparées par |"),
                p("group_by", S, false, Some(serde_json::json!("usage")), "Propriété d'arête : le genre retenu"),
                p("usages_field", S, false, Some(serde_json::json!("usages")), "Propriété d'arête : l'ensemble des genres"),
                p("line", S, false, Some(serde_json::json!("line")), "Propriété d'arête : la ligne du site"),
                p("title", S, false, Some(serde_json::json!("name")), "Champ montré comme titre"),
                p("kind_field", S, false, Some(serde_json::json!("scope_type")), "Champ montré comme genre"),
                p("path_fields", S, false, Some(serde_json::json!("file_path")), "Champs de chemin, le premier non vide, séparés par |"),
                p("line_field", S, false, Some(serde_json::json!("start_line")), "Champ de ligne de la déclaration"),
                p("declared_in", S, false, Some(serde_json::json!("")), "Relation d'une définition vers son conteneur, qui porte les déclarations (ex. HAS_PARENT)"),
                p("virtual_path", S, false, Some(serde_json::json!("")), "Chemin d'une définition vers la méthode de base qu'elle redéfinit (pas REL> ou <REL, A/B pour plusieurs relations, + pour répéter, séparés par |) : ses appelants sont dits « par envoi virtuel »"),
                p("virtual_same", S, false, Some(serde_json::json!("")), "Champ qui doit valoir le nom de la définition au bout du chemin (défaut : title)"),
                p("declarations_field", S, false, Some(serde_json::json!("declarations")), "Champ du conteneur : liste JSON de {name, line, signature, kind}"),
                p("name", S, true, None, "Le nom cherché"),
                p("path", S, false, Some(serde_json::json!("")), "Ne montrer que les définitions sous ce chemin"),
                p("scope", S, false, Some(serde_json::json!("")), SCOPE_HELP),
                p("language", S, false, Some(serde_json::json!("")), LANGUAGE_HELP),
                usage,
                p("limit", Int, false, Some(serde_json::json!(20)), "Usages rendus par genre (plafond 200) ; tous sont comptés"),
                p("edge_field", S, false, Some(serde_json::json!("")), "Champ d'arête qui dit comment elle a été posée (ex. resolution)"),
                p("edge_guessed", S, false, Some(serde_json::json!("")), "Valeurs de ce champ pour une arête devinée (ex. nom), séparées par |"),
                p("edge_doubtful", S, false, Some(serde_json::json!("")), "Valeurs de ce champ pour une arête douteuse, suivie mais dite (ex. ambigu), séparées par |"),
                format,
            ],
        }
    }
}

#[cfg(test)]
mod tests_saut {
    use super::*;
    use super::super::graph_walk::EdgeMark;
    use crate::dialect::{Rag3dbDialect, SchemaDialect};

    /// La requête que `direct_usages_query` écrivait avant le dialecte.
    fn requete_d_avant(cfg: &UsagesConfig, rel: &RelInfo) -> String {
        format!(
            "UNWIND $uuids AS u MATCH (d:{to} {{_uuid: u}})<-[r:{name}]-(m:{from}) RETURN u, {c}, {props}",
            to = crate::dialect::cypher_quote(&rel.to),
            name = crate::dialect::cypher_quote(&rel.name),
            from = crate::dialect::cypher_quote(&rel.from),
            c = champs("m", cfg),
            props = proprietes(rel, cfg)
        )
    }

    /// Parité : au caractère près, avec et sans propriétés d'arête, avec et
    /// sans marque d'arête devinée.
    #[test]
    fn les_usages_directs_sont_la_requete_d_avant() {
        let mark = EdgeMark::from_config(&serde_json::json!({"edge_field": "resolution", "edge_guessed": "nom"}), "N").unwrap();
        for edge_mark in [EdgeMark::default(), mark] {
            let cfg = UsagesConfig {
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
                edge_mark,
                qualifier_field: None,
            };
            for props in [vec![], vec!["usage".to_string(), "line".into(), "resolution".into()]] {
                let rel = RelInfo { name: "CONSUMES".into(), from: "Scope".into(), to: "Symbol".into(), props };
                assert_eq!(Rag3dbDialect.hop(&direct_usages_hop(&cfg, &rel)).unwrap(), requete_d_avant(&cfg, &rel));
                // Les définitions et les usages par le rendez-vous partent
                // maintenant d'une liste d'uuids (un pivot à la fois) : la
                // première colonne est l'uuid du pivot.
                assert_eq!(
                    Rag3dbDialect.hop(&definitions_hop(&cfg, &rel)).unwrap(),
                    format!("UNWIND $uuids AS u MATCH (d:`Symbol` {{_uuid: u}})<-[r:`CONSUMES`]-(m:`Scope`) RETURN u, {}", champs("m", &cfg))
                );
                assert_eq!(
                    Rag3dbDialect.hop(&pivot_usages_hop(&cfg, &rel)).unwrap(),
                    format!("UNWIND $uuids AS u MATCH (d:`Symbol` {{_uuid: u}})<-[r:`CONSUMES`]-(m:`Scope`) RETURN u, {}, {}", champs("m", &cfg), proprietes(&rel, &cfg))
                );
            }
        }
    }
}
