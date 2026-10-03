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
}

#[derive(Debug, Clone, Serialize)]
pub struct UsagesReport {
    pub name: String,
    pub definitions: Vec<Item>,
    /// Définitions écartées par le filtre de chemin.
    pub definitions_hidden: usize,
    pub usages: Vec<Usage>,
    pub ambiguous: bool,
}

// ─── Requêtes ────────────────────────────────────────────────────────────────

fn champs(alias: &str, cfg: &UsagesConfig) -> String {
    let mut c = vec![format!("{alias}._uuid"), format!("{alias}.{}", cfg.title), format!("{alias}.{}", cfg.kind_field)];
    c.extend(cfg.path_fields.iter().map(|p| format!("{alias}.{p}")));
    c.push(format!("{alias}.{}", cfg.line_field));
    c.join(", ")
}

fn proprietes(rel: &RelInfo, cfg: &UsagesConfig) -> String {
    [&cfg.group_by, &cfg.usages_field, &cfg.line]
        .iter()
        .map(|p| if rel.props.iter().any(|x| x == *p) { format!("r.{p}") } else { "NULL".to_string() })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Les définitions d'un pivot : `MATCH (x:Pivot {_uuid: $u})<-[:DEF]-(d:T)`.
pub fn definitions_query(cfg: &UsagesConfig, defined_by: &RelInfo) -> String {
    format!(
        "MATCH (x:{p} {{_uuid: $u}})<-[:{rel}]-(d:{t}) RETURN {c}",
        p = cfg.pivot,
        rel = defined_by.name,
        t = defined_by.from,
        c = champs("d", cfg)
    )
}

/// Les usages par le rendez-vous : `MATCH (x:Pivot {_uuid: $u})<-[r:USED]-(m:T)`.
pub fn pivot_usages_query(cfg: &UsagesConfig, used_by: &RelInfo) -> String {
    format!(
        "MATCH (x:{p} {{_uuid: $u}})<-[r:{rel}]-(m:{t}) RETURN {c}, {props}",
        p = cfg.pivot,
        rel = used_by.name,
        t = used_by.from,
        c = champs("m", cfg),
        props = proprietes(used_by, cfg)
    )
}

/// Les usages directs des définitions : une liste de valeurs simples, que le
/// moteur joint par hachage.
pub fn direct_usages_query(cfg: &UsagesConfig, rel: &RelInfo) -> String {
    format!(
        "UNWIND $uuids AS u MATCH (d:{to} {{_uuid: u}})<-[r:{name}]-(m:{from}) RETURN u, {c}, {props}",
        to = rel.to,
        name = rel.name,
        from = rel.from,
        c = champs("m", cfg),
        props = proprietes(rel, cfg)
    )
}

/// Une relation déclarée au catalogue : ses extrémités et ses propriétés.
#[derive(Debug, Clone)]
pub struct RelInfo {
    pub name: String,
    pub from: String,
    pub to: String,
    pub props: Vec<String>,
}

pub fn rel_info(catalog: &Catalog, name: &str) -> Result<RelInfo, String> {
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
pub fn usages_of(catalog: &Catalog, cfg: &UsagesConfig, name: &str, path_prefix: &str) -> Result<UsagesReport, String> {
    let err = |e: CatalogError| format!("UsagesNode: {e}");
    let entity = catalog.entity_config(&cfg.pivot).ok_or_else(|| format!("UsagesNode: entité inconnue « {} »", cfg.pivot))?;
    // Le pivot : par son uuid déterministe quand la clé est son identité.
    let pivots: Vec<String> = if entity.hashsafe.as_deref() == Some(std::slice::from_ref(&cfg.key)) {
        let data = BTreeMap::from([(cfg.key.clone(), CypherValue::String(name.to_string()))]);
        vec![catalog.entity_uuid(&cfg.pivot, &data).map_err(err)?]
    } else {
        let q = format!("MATCH (x:{p}) WHERE x.{k} = $name RETURN x._uuid", p = cfg.pivot, k = cfg.key);
        let rows = catalog.execute_raw_with_params(&q, &[QueryParam::new("name", CypherValue::String(name.to_string()))]).map_err(err)?;
        rows.rows.iter().map(|r| texte(r.first())).filter(|u| !u.is_empty()).collect()
    };

    // Les définitions.
    let mut definitions: Vec<Item> = Vec::new();
    if let Some(def) = &cfg.defined_by {
        let rel = rel_info(catalog, def)?;
        let q = definitions_query(cfg, &rel);
        for u in &pivots {
            let rows = catalog.execute_raw_with_params(&q, &[QueryParam::new("u", CypherValue::String(u.clone()))]).map_err(err)?;
            definitions.extend(rows.rows.iter().map(|r| item(r, 0, cfg)));
        }
    }
    definitions.sort_by(|a, b| (&a.path, a.line, &a.uuid).cmp(&(&b.path, b.line, &b.uuid)));
    definitions.dedup_by(|a, b| a.uuid == b.uuid);
    let ambiguous = definitions.len() > 1;
    let total = definitions.len();
    if !path_prefix.is_empty() {
        definitions.retain(|d| sous_le_chemin(&d.path, path_prefix));
    }
    let definitions_hidden = total - definitions.len();

    // Les usages directs, sous leur définition.
    let mut usages: Vec<Usage> = Vec::new();
    let mut vus: HashMap<(String, Option<String>), usize> = HashMap::new();
    let def_uuids: Vec<CypherValue> = definitions.iter().map(|d| CypherValue::String(d.uuid.clone())).collect();
    if !def_uuids.is_empty() {
        for name_rel in &cfg.direct {
            let rel = rel_info(catalog, name_rel)?;
            let q = direct_usages_query(cfg, &rel);
            let rows = catalog
                .execute_raw_with_params(&q, &[QueryParam::new("uuids", CypherValue::List(def_uuids.clone()))])
                .map_err(err)?;
            for r in &rows.rows {
                let def = texte(r.first());
                let mut user = item(r, 1, cfg);
                let (usage, ensemble, site) = usage_props(r, 1 + 4 + cfg.path_fields.len());
                if user.uuid == def {
                    continue;
                }
                user.line = site.or(user.line);
                ajouter(&mut usages, &mut vus, Usage { user, usage, usages: ensemble, relation: rel.name.clone(), definition: Some(def) });
            }
        }
    }

    // Les usages par le rendez-vous : attribués à la définition s'il n'y en a
    // qu'une, sinon « non attribués ».
    if let Some(used) = &cfg.used_by {
        let rel = rel_info(catalog, used)?;
        let q = pivot_usages_query(cfg, &rel);
        let unique = if ambiguous { None } else { definitions.first().map(|d| d.uuid.clone()) };
        let def_set: std::collections::HashSet<&str> = definitions.iter().map(|d| d.uuid.as_str()).collect();
        for u in &pivots {
            let rows = catalog.execute_raw_with_params(&q, &[QueryParam::new("u", CypherValue::String(u.clone()))]).map_err(err)?;
            for r in &rows.rows {
                let mut user = item(r, 0, cfg);
                if def_set.contains(user.uuid.as_str()) {
                    continue;
                }
                let (usage, ensemble, site) = usage_props(r, 4 + cfg.path_fields.len());
                user.line = site.or(user.line);
                // Déjà trouvé par une arête directe : il compte une fois.
                if usages.iter().any(|x| x.user.uuid == user.uuid && (x.definition == unique || unique.is_none())) {
                    continue;
                }
                ajouter(&mut usages, &mut vus, Usage { user, usage, usages: ensemble, relation: rel.name.clone(), definition: unique.clone() });
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
    usages.sort_by(|a, b| (&a.user.path, a.user.line, &a.user.title).cmp(&(&b.user.path, b.user.line, &b.user.title)));
    Ok(UsagesReport { name: name.to_string(), definitions, definitions_hidden, usages, ambiguous })
}

/// Le chemin d'un scope commence par `prefixe`, au début ou après un `/` :
/// l'agent donne un chemin relatif (`src/dataflow`), l'index peut garder un
/// chemin absolu (`/projet/src/dataflow/port.rs`).
fn sous_le_chemin(chemin: &str, prefixe: &str) -> bool {
    let p = prefixe.trim_start_matches("./");
    chemin.starts_with(p) || chemin.contains(&format!("/{p}"))
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
        let mut out = format!("# usages: {}\n\n", self.name);
        let defs = &self.definitions;
        out.push_str(&format!("## Définitions ({})", defs.len()));
        if self.definitions_hidden > 0 {
            out.push_str(&format!(" — {} autre(s) hors du chemin demandé", self.definitions_hidden));
        }
        out.push('\n');
        if defs.is_empty() {
            out.push_str("(aucune définition indexée sous ce nom)\n");
        }
        for d in defs {
            out.push_str(&format!("- {} {} — {}\n", d.kind, d.title, lieu(d)));
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
                out.push_str(&format!("- {} {} — {}{cible}\n", u.user.kind, u.user.title, lieu(&u.user)));
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
    usage: String,
    limit: usize,
    /// `true` : le rapport en JSON ; sinon du markdown.
    json: bool,
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
        use super::catalog_read::{read_catalog, refusal, with_status, CatalogRead};
        let catalog = ctx.service::<Arc<Mutex<Catalog>>>("catalog").cloned().ok_or("UsagesNode: service 'catalog' absent")?;
        // Jamais un vide sans dire d'où il vient : occupé, jamais indexé,
        // ou partiel, le rendu le dit.
        let (report, status) = match read_catalog(&catalog, &self.cfg.pivot) {
            CatalogRead::Refused(message) => {
                ctx.set_output("result", PortValue::new(refusal(&message, self.json)));
                return Ok(());
            }
            CatalogRead::Ready { catalog: cat, status } => (usages_of(&cat, &self.cfg, &self.name, &self.path)?, status),
        };
        ctx.metric("definitions", report.definitions.len() as f64);
        ctx.metric("usages", report.usages.len() as f64);
        let value = if self.json {
            serde_json::to_value(&report).map_err(|e| e.to_string())?
        } else {
            serde_json::Value::String(report.markdown(&self.usage, self.limit))
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
        };
        // Des identifiants seulement : ils entrent dans le texte des requêtes.
        let ident = |x: &str| !x.is_empty() && x.chars().all(|c| c.is_alphanumeric() || c == '_');
        let tous: Vec<&String> = [&cfg.pivot, &cfg.key, &cfg.group_by, &cfg.usages_field, &cfg.line, &cfg.title, &cfg.kind_field, &cfg.line_field]
            .into_iter()
            .chain(cfg.defined_by.iter())
            .chain(cfg.used_by.iter())
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
        Ok(Box::new(UsagesNode {
            node_name: name.to_string(),
            cfg,
            name: requis("name")?,
            path: s("path").unwrap_or_default(),
            usage,
            limit,
            json,
        }))
    }
    fn node_type(&self) -> &'static str {
        "UsagesNode"
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
        let mut usage = p("usage", S, false, Some(serde_json::json!("all")), "Ne garder qu'un genre d'usage");
        usage.choices = Some(Choices::fixed(["all", "call", "type", "import", "inheritance", "other"]));
        let mut format = p("format", S, false, Some(serde_json::json!("markdown")), "markdown (pour le modèle) | json (structuré)");
        format.choices = Some(Choices::fixed(["markdown", "json"]));
        NodeSchema {
            node_type: "UsagesNode",
            description: "A thing, what defines it, and what uses it: through a declared pivot entity (definitions and usages relations) and direct relations to each definition, grouped by the usage kind read on the edge, with file and line. Ambiguous names show every definition and leave pivot-only usages unattributed.",
            inputs: vec![],
            outputs: vec![PortDef { name: "result", port_type: PortType::Map, required: false }],
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
                p("name", S, true, None, "Le nom cherché"),
                p("path", S, false, Some(serde_json::json!("")), "Ne montrer que les définitions sous ce chemin"),
                usage,
                p("limit", Int, false, Some(serde_json::json!(20)), "Usages rendus par genre (plafond 200) ; tous sont comptés"),
                format,
            ],
        }
    }
}
