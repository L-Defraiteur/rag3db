//! **Rendre les lignes d'une entité dérivée.**
//!
//! Une entité dérivée (`EntityConfig.derived`, doc du 7 septembre 2026) n'est
//! pas ingérée : chacune de ses lignes est rendue depuis une ligne racine et
//! les voisines rassemblées par relation, par un gabarit Jinja par champ.
//! `DeriveNode` fait exactement ça, et rien d'autre : ce qu'il rend part dans
//! la chaîne de n'importe quelle entité — insertion, découpe, embarquement,
//! plein texte. C'est ce qui remplace le pipeline des bases de connaissances
//! (`KBGather → KBUpdate → KBChunk → KBEmbed`) par un seul nœud.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

use crate::config::{CatalogConfig, DerivedConfig, EntityConfig, GatherDirection};
use crate::connection::{CypherValue, DbConnection, QueryParam};
use crate::hash::content_hash;
use crate::records::{Derivation, EntityRecord, RefOrUuid, RelationRecord};
use crate::refs::{EntityRef, RelationRef};
use crate::uuid::hashsafe_uuid;

use super::node::{Node, NodeContext};
use super::port::{BatchPayload, PortDef, PortType, PortValue};

/// **Une dérivation à rendre** : une entité dérivée et l'uuid de sa racine.
/// Le nœud lit la racine et ses voisines, rend chaque champ, et ne produit une
/// ligne que si le hash des entrées a changé depuis le dernier rendu
/// (`_render_hash`) — le court-circuit de l'inchangé, avant même l'insertion.
///
/// **Input** : `derivations` — `BatchPayload<Derivation>` (PortType::Derivations)
/// **Output** : `entities` — les lignes rendues (PortType::Entities) ;
/// `relations` — `{Entité}_DERIVED_FROM` vers la racine ; `done`.
/// **Services** : `conn`, `dialect`, `config`, `entity_configs`.
pub struct DeriveNode {
    name: String,
}

impl DeriveNode {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

/// Une ligne relue en base, par champ.
type Ligne = BTreeMap<String, CypherValue>;

/// **Le contexte d'un rendu**, tel que le gabarit le voit : `root` et une
/// liste par règle `gather`, dans un ordre stable. C'est aussi ce dont on
/// prend le hash pour savoir si la ligne dérivée est à jour.
pub fn render_context(root: &Ligne, gathered: &BTreeMap<String, Vec<Ligne>>) -> serde_json::Value {
    let mut ctx = serde_json::Map::new();
    ctx.insert(DerivedConfig::ROOT.to_string(), ligne_en_json(root));
    for (nom, lignes) in gathered {
        ctx.insert(nom.clone(), serde_json::Value::Array(lignes.iter().map(ligne_en_json).collect()));
    }
    serde_json::Value::Object(ctx)
}

fn ligne_en_json(ligne: &Ligne) -> serde_json::Value {
    serde_json::Value::Object(ligne.iter().map(|(k, v)| (k.clone(), valeur_en_json(v))).collect())
}

fn valeur_en_json(v: &CypherValue) -> serde_json::Value {
    match v {
        CypherValue::Null => serde_json::Value::Null,
        CypherValue::Bool(b) => serde_json::Value::Bool(*b),
        CypherValue::Int(i) => serde_json::Value::from(*i),
        CypherValue::Float(f) => serde_json::Value::from(*f),
        CypherValue::String(s) => serde_json::Value::String(s.clone()),
        CypherValue::List(l) => serde_json::Value::Array(l.iter().map(valeur_en_json).collect()),
        CypherValue::Map(m) => serde_json::Value::Object(m.iter().map(|(k, v)| (k.clone(), valeur_en_json(v))).collect()),
        CypherValue::Blob(b) => serde_json::Value::String(format!("<blob {} octets>", b.len())),
    }
}

/// **Le hash des entrées d'un rendu** : la forme canonique du contexte. Les
/// clés d'un objet `serde_json` sont triées, les listes sont dans l'ordre
/// stable du rassemblement — deux entrées égales donnent le même hash.
pub fn render_inputs_hash(context: &serde_json::Value) -> String {
    content_hash(&serde_json::to_string(context).unwrap_or_default())
}

/// **Rendre les champs d'une ligne dérivée.** Un gabarit par champ, résolu
/// comme ceux du rendu de résultats (nom de gabarit ou source en ligne).
pub fn render_fields(derived: &DerivedConfig, context: &serde_json::Value) -> Result<BTreeMap<String, String>, String> {
    let mut rendus = BTreeMap::new();
    for (champ, spec) in &derived.render {
        let source = super::render_nodes::resolve_template(spec)?;
        let texte = super::render_nodes::rendre(context, &source).map_err(|e| format!("champ « {champ} » : {e}"))?;
        rendus.insert(champ.clone(), texte);
    }
    Ok(rendus)
}

/// L'uuid d'une ligne dérivée : une par racine, déterministe.
pub fn derived_uuid(entity: &str, root_uuid: &str) -> String {
    hashsafe_uuid(entity, &[root_uuid])
}

impl Node for DeriveNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn node_type(&self) -> &'static str {
        "DeriveNode"
    }
    fn inputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::node_factories::DeriveNodeFactory).0
    }
    fn outputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::node_factories::DeriveNodeFactory).1
    }

    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let derivations: Vec<Derivation> = ctx
            .take_input("derivations")
            .and_then(|pv| pv.take::<BatchPayload>())
            .and_then(|bp| bp.take::<Derivation>())
            .unwrap_or_default();
        let conn = ctx.service::<Arc<dyn DbConnection>>("conn").cloned().ok_or("DeriveNode: 'conn' service not registered")?;
        let dialect = ctx
            .service::<Arc<dyn crate::dialect::SchemaDialect>>("dialect")
            .cloned()
            .ok_or("DeriveNode: 'dialect' service not registered")?;
        let config = ctx.service::<CatalogConfig>("config").cloned().ok_or("DeriveNode: 'config' service not registered")?;
        let entity_configs = ctx
            .service::<HashMap<String, EntityConfig>>("entity_configs")
            .cloned()
            .ok_or("DeriveNode: 'entity_configs' service not registered")?;

        // Par entité dérivée, les racines à rendre, sans doublon.
        let mut par_entite: BTreeMap<String, Vec<String>> = BTreeMap::new();
        {
            let mut vues: HashSet<(String, String)> = HashSet::new();
            for d in derivations {
                if vues.insert((d.entity.clone(), d.root_uuid.clone())) {
                    par_entite.entry(d.entity).or_default().push(d.root_uuid);
                }
            }
        }

        let mut lignes: Vec<EntityRecord> = Vec::new();
        let mut liens: Vec<RelationRecord> = Vec::new();
        let mut inchangees = 0usize;
        let mut racines_absentes = 0usize;

        for (entity, racines) in &par_entite {
            let Some(cfg) = entity_configs.get(entity) else {
                ctx.warn(&format!("DeriveNode: « {entity} » n'est pas une entité enregistrée, {} dérivation(s) ignorée(s)", racines.len()));
                continue;
            };
            let Some(derived) = cfg.derived.as_ref() else {
                ctx.warn(&format!("DeriveNode: « {entity} » n'est pas une entité dérivée, {} dérivation(s) ignorée(s)", racines.len()));
                continue;
            };
            let Some(cfg_racine) = entity_configs.get(&derived.from) else {
                ctx.warn(&format!("DeriveNode: la racine « {} » de « {entity} » n'est pas enregistrée", derived.from));
                continue;
            };

            // 1. Les racines, tous leurs champs déclarés.
            let mut champs_racine: Vec<&str> = cfg_racine.fields.keys().map(|s| s.as_str()).collect();
            champs_racine.sort_unstable();
            let racines_lues = lire_par_uuid(conn.as_ref(), dialect.as_ref(), &derived.from, &champs_racine, racines)?;

            // 2. Les voisines, par règle.
            let mut voisines: HashMap<String, BTreeMap<String, Vec<Ligne>>> = HashMap::new();
            for regle in &derived.gather {
                let Some(rel) = config.relations.get(&regle.relation) else { continue };
                let (voisine, en_avant) = match regle.direction {
                    GatherDirection::Out => (rel.to.as_str(), true),
                    GatherDirection::In => (rel.from.as_str(), false),
                };
                let mut champs: Vec<&str> = if regle.fields.is_empty() {
                    entity_configs.get(voisine).map(|c| c.fields.keys().map(|s| s.as_str()).collect()).unwrap_or_default()
                } else {
                    regle.fields.iter().map(|s| s.as_str()).collect()
                };
                champs.sort_unstable();
                champs.retain(|c| *c != "_uuid");
                champs.insert(0, "_uuid");
                let requete = dialect.kb_gather_content(&derived.from, &regle.relation, voisine, en_avant, &champs);
                let items = CypherValue::List(
                    racines
                        .iter()
                        .map(|u| CypherValue::Map(BTreeMap::from([("uuid".to_string(), CypherValue::String(u.clone()))])))
                        .collect(),
                );
                let lu = conn
                    .execute_with_params(&requete, &[QueryParam { name: "items".into(), value: items }])
                    .map_err(|e| format!("DeriveNode: voisines « {} » de « {entity} » : {e}", regle.name))?;
                for row in &lu.rows {
                    let Some(racine) = row.first().and_then(|v| v.as_str()) else { continue };
                    let mut ligne = Ligne::new();
                    for (k, champ) in champs.iter().enumerate() {
                        if let Some(v) = row.get(k + 1) {
                            ligne.insert(champ.to_string(), v.clone());
                        }
                    }
                    voisines.entry(racine.to_string()).or_default().entry(regle.name.clone()).or_default().push(ligne);
                }
            }
            // Un ordre stable : par uuid de voisine.
            for regles in voisines.values_mut() {
                for lignes_regle in regles.values_mut() {
                    lignes_regle.sort_by(|a, b| a.get("_uuid").and_then(|v| v.as_str()).cmp(&b.get("_uuid").and_then(|v| v.as_str())));
                }
            }

            // 3. Ce qui est déjà rendu, et avec quelles entrées.
            let uuids_derives: Vec<String> = racines.iter().map(|r| derived_uuid(entity, r)).collect();
            let deja = lire_par_uuid(conn.as_ref(), dialect.as_ref(), entity, &[DerivedConfig::RENDER_HASH], &uuids_derives)?;

            // 4. Rendre ce qui a changé.
            let champs_contenu: Vec<String> = cfg.content_fields().iter().map(|s| s.to_string()).collect();
            for racine in racines {
                let Some(ligne_racine) = racines_lues.get(racine) else {
                    racines_absentes += 1;
                    continue;
                };
                let vide = BTreeMap::new();
                let regles: BTreeMap<String, Vec<Ligne>> = {
                    let mut m: BTreeMap<String, Vec<Ligne>> = derived.gather.iter().map(|r| (r.name.clone(), Vec::new())).collect();
                    for (nom, l) in voisines.get(racine).unwrap_or(&vide) {
                        m.insert(nom.clone(), l.clone());
                    }
                    m
                };
                let contexte = render_context(ligne_racine, &regles);
                let hash_entrees = render_inputs_hash(&contexte);
                let uuid = derived_uuid(entity, racine);
                if deja.get(&uuid).and_then(|l| l.get(DerivedConfig::RENDER_HASH)).and_then(|v| v.as_str()) == Some(hash_entrees.as_str()) {
                    inchangees += 1;
                    continue;
                }
                let rendus = render_fields(derived, &contexte).map_err(|e| format!("DeriveNode: « {entity} » depuis {racine} : {e}"))?;

                let mut data: Ligne = BTreeMap::new();
                for (champ, texte) in &rendus {
                    data.insert(champ.clone(), CypherValue::String(texte.clone()));
                }
                let contenu: String = champs_contenu
                    .iter()
                    .filter_map(|c| rendus.get(c).map(|s| s.as_str()))
                    .collect::<Vec<_>>()
                    .join("\n\n");
                data.insert("_uuid".into(), CypherValue::String(uuid.clone()));
                data.insert("_content_hash".into(), CypherValue::String(content_hash(&contenu)));
                data.insert(DerivedConfig::SOURCE_ENTITY.into(), CypherValue::String(derived.from.clone()));
                data.insert(DerivedConfig::SOURCE_UUID.into(), CypherValue::String(racine.clone()));
                data.insert(DerivedConfig::RENDER_HASH.into(), CypherValue::String(hash_entrees));

                let (entity_ref, resolver) = EntityRef::new(entity);
                lignes.push(EntityRecord { entity_name: entity.clone(), data, entity_ref, resolver: Some(resolver), vectors: None });

                let rel_name = crate::schema::derived_rel_name(entity);
                let (relation_ref, rel_resolver) = RelationRef::new(&rel_name);
                liens.push(RelationRecord::new(
                    rel_name,
                    RefOrUuid::Uuid(uuid),
                    RefOrUuid::Uuid(racine.clone()),
                    BTreeMap::new(),
                    rel_resolver,
                    relation_ref,
                ));
            }
        }

        ctx.metric("rendered", lignes.len() as f64);
        ctx.metric("unchanged", inchangees as f64);
        ctx.metric("missing_roots", racines_absentes as f64);
        ctx.set_output("entities", PortValue::new(BatchPayload::new(PortType::Entities, lignes)));
        ctx.set_output("relations", PortValue::new(BatchPayload::new(PortType::Relations, liens)));
        ctx.trigger("done");
        Ok(())
    }
}

/// Relire des lignes par uuid, par tranches, en une carte `uuid → champs`.
fn lire_par_uuid(
    conn: &dyn DbConnection,
    dialect: &dyn crate::dialect::SchemaDialect,
    table: &str,
    champs: &[&str],
    uuids: &[String],
) -> Result<HashMap<String, Ligne>, String> {
    let mut colonnes: Vec<&str> = vec!["_uuid"];
    colonnes.extend(champs.iter().copied().filter(|c| *c != "_uuid"));
    let requete = dialect.select_by_uuids(table, &colonnes);
    let mut lues = HashMap::with_capacity(uuids.len());
    for tranche in uuids.chunks(5_000) {
        let param = CypherValue::List(tranche.iter().map(|u| CypherValue::String(u.clone())).collect());
        let lu = conn
            .execute_with_params(&requete, &[QueryParam { name: "uuids".into(), value: param }])
            .map_err(|e| format!("DeriveNode: lecture de « {table} » : {e}"))?;
        for row in &lu.rows {
            let Some(uuid) = row.first().and_then(|v| v.as_str()) else { continue };
            let mut ligne = Ligne::new();
            for (k, col) in colonnes.iter().enumerate() {
                if let Some(v) = row.get(k) {
                    ligne.insert(col.to_string(), v.clone());
                }
            }
            lues.insert(uuid.to_string(), ligne);
        }
    }
    Ok(lues)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn derived() -> DerivedConfig {
        DerivedConfig {
            from: "Ticket".into(),
            gather: vec![],
            render: [
                ("title".to_string(), "{{ root.subject }}".to_string()),
                ("content".to_string(), "{{ root.body }}\n{% for c in comments %}{{ c.author }} : {{ c.body }}\n{% endfor %}".to_string()),
            ]
            .into_iter()
            .collect(),
        }
    }

    #[test]
    fn le_contexte_rend_les_champs_et_a_un_hash_stable() {
        let root: Ligne = [("subject".to_string(), CypherValue::String("Panne".into())), ("body".to_string(), CypherValue::String("Ça casse.".into()))].into_iter().collect();
        let commentaires = vec![
            [("_uuid".to_string(), CypherValue::String("c1".into())), ("author".to_string(), CypherValue::String("Ana".into())), ("body".to_string(), CypherValue::String("Chez moi aussi.".into()))].into_iter().collect::<Ligne>(),
        ];
        let regles: BTreeMap<String, Vec<Ligne>> = [("comments".to_string(), commentaires)].into_iter().collect();
        let ctx = render_context(&root, &regles);
        let rendus = render_fields(&derived(), &ctx).unwrap();
        assert_eq!(rendus["title"], "Panne");
        assert_eq!(rendus["content"], "Ça casse.\nAna : Chez moi aussi.\n");
        let h1 = render_inputs_hash(&ctx);
        let h2 = render_inputs_hash(&render_context(&root, &regles));
        assert_eq!(h1, h2, "les mêmes entrées donnent le même hash");
        let mut autres = regles.clone();
        autres.get_mut("comments").unwrap().clear();
        assert_ne!(h1, render_inputs_hash(&render_context(&root, &autres)), "une voisine de moins change le hash");
    }

    #[test]
    fn l_uuid_d_une_derivee_ne_depend_que_de_sa_racine() {
        assert_eq!(derived_uuid("TicketView", "r1"), derived_uuid("TicketView", "r1"));
        assert_ne!(derived_uuid("TicketView", "r1"), derived_uuid("TicketView", "r2"));
        assert_ne!(derived_uuid("TicketView", "r1"), derived_uuid("AutreVue", "r1"));
    }
}
