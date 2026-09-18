//! **Une base de connaissances est une entité dérivée** (doc du 7 septembre
//! 2026, « replier les bases de connaissances en entités dérivées »).
//!
//! `knowledge_bases` et `title_for` / `content_for` restent lisibles : on les
//! traduit ici en `EntityConfig { derived: Some(…) }`. La racine est l'entité
//! qui porte `title_for`, chaque entité contributrice devient une règle
//! `gather` par la relation qui la lie à la racine, et le gabarit de `content`
//! rend le même texte qu'avant : les champs de contenu joints par un saut de
//! ligne, dans l'ordre `(entité, uuid, champ)`, les champs vides sautés. Les
//! réglages de recherche (`signals`, fusion, découpe) suivent.

use std::collections::{BTreeMap, HashMap};

use crate::config::{
    CatalogConfig, DerivedConfig, EntityConfig, EntityDef, FieldType, GatherDirection, GatherRule, KBConfig, SimpleFieldDef,
};
use crate::schema::{resolve_entity_kbs, resolve_kb_title_entities};

/// Le nom du champ titre d'une KB traduite, et celui de son contenu.
pub const TITLE_FIELD: &str = "title";
pub const CONTENT_FIELD: &str = "content";

/// **La config d'enregistrement d'une entité déclarée dans le schéma.** Les
/// entités de `CatalogConfig.entities` n'ont qu'une `EntityDef` ; une dérivée
/// qui les lit a besoin de leur `EntityConfig` (champs, signaux). Sans
/// `is_content`, elles n'ont pas de pipeline simple : des données, que les
/// dérivées rassemblent.
pub fn entity_config_from_def(def: &EntityDef) -> EntityConfig {
    let fields: HashMap<String, SimpleFieldDef> = def
        .fields
        .iter()
        .map(|(name, f)| {
            (
                name.clone(),
                SimpleFieldDef {
                    field_type: f.field_type.clone(),
                    title_for: f.title_for.clone(),
                    content_for: f.content_for.clone(),
                    ..Default::default()
                },
            )
        })
        .collect();
    EntityConfig { fields, hashsafe: def.hashsafe.clone(), ..Default::default() }
}

/// La relation qui lie la racine à une entité contributrice, et son sens.
fn relation_vers(config: &CatalogConfig, racine: &str, voisine: &str) -> Option<(String, GatherDirection)> {
    let mut noms: Vec<&String> = config.relations.keys().collect();
    noms.sort();
    for nom in noms {
        let rel = &config.relations[nom];
        if rel.from == racine && rel.to == voisine {
            return Some((nom.clone(), GatherDirection::Out));
        }
        if rel.from == voisine && rel.to == racine {
            return Some((nom.clone(), GatherDirection::In));
        }
    }
    None
}

/// **Traduire une KB en entité dérivée.** `None` quand aucune entité ne porte
/// encore `title_for` pour elle : la KB attend sa racine, comme avant.
pub fn derived_config_for_kb(kb_name: &str, kb_config: &KBConfig, config: &CatalogConfig) -> Option<EntityConfig> {
    let racines = resolve_kb_title_entities(config);
    let info = racines.get(kb_name)?;
    let racine = info.title_entity.clone();

    // Les contributions, par entité, champs triés.
    let mut contributions: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (nom, def) in &config.entities {
        if let Some(mapping) = resolve_entity_kbs(def).get(kb_name) {
            if !mapping.content_fields.is_empty() {
                contributions.insert(nom.clone(), mapping.content_fields.clone());
            }
        }
    }

    // Le gabarit du contenu : les entités dans l'ordre de leur nom — c'est
    // l'ordre `(entité, uuid, champ)` d'avant, puisque le nœud de dérivation
    // trie les voisines par uuid et que les champs sont triés ici.
    let mut gather = Vec::new();
    let mut contenu = String::new();
    for (entite, champs) in &contributions {
        if *entite == racine {
            for champ in champs {
                contenu.push_str(&format!("{{% if root.{champ} %}}{{{{ root.{champ} }}}}\n{{% endif %}}"));
            }
            continue;
        }
        let Some((relation, direction)) = relation_vers(config, &racine, entite) else {
            // Sans relation vers la racine, la contribution était déjà
            // inatteignable avant : on ne l'invente pas.
            continue;
        };
        gather.push(GatherRule { name: entite.clone(), relation, direction, fields: champs.clone() });
        contenu.push_str(&format!("{{% for r in {entite} %}}"));
        for champ in champs {
            contenu.push_str(&format!("{{% if r.{champ} %}}{{{{ r.{champ} }}}}\n{{% endif %}}"));
        }
        contenu.push_str("{% endfor %}");
    }

    // Le titre, borné à `title_max_chars` comme avant (0 : entier).
    let mut render = BTreeMap::new();
    let borne = kb_config.chunking.title_max_chars;
    let titre = if borne > 0 {
        format!("{{{{ root.{}[:{borne}] }}}}", info.title_field)
    } else {
        format!("{{{{ root.{} }}}}", info.title_field)
    };
    render.insert(TITLE_FIELD.to_string(), titre);
    render.insert(CONTENT_FIELD.to_string(), contenu);

    let mut fields = HashMap::new();
    fields.insert(TITLE_FIELD.to_string(), SimpleFieldDef { field_type: FieldType::String, is_title: true, ..Default::default() });
    fields.insert(CONTENT_FIELD.to_string(), SimpleFieldDef { field_type: FieldType::Text, is_content: true, ..Default::default() });

    Some(EntityConfig {
        fields,
        signals: kb_config.signals,
        chunking: kb_config.chunking.clone(),
        fusion: Some(kb_config.fusion_config()),
        derived: Some(DerivedConfig { from: racine, gather, render }),
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{FieldDef, RelationDef};

    fn champ(title_for: Option<&str>, content_for: Option<&[&str]>) -> FieldDef {
        let mut f: FieldDef = serde_json::from_str(r#"{"type":"text"}"#).unwrap();
        f.title_for = title_for.map(|s| s.to_string());
        f.content_for = content_for.map(|c| c.iter().map(|s| s.to_string()).collect());
        f
    }

    fn config() -> CatalogConfig {
        let mut config = CatalogConfig::default();
        let mut doc = HashMap::new();
        doc.insert("title".to_string(), champ(Some("main"), None));
        doc.insert("body".to_string(), champ(None, Some(&["main"])));
        doc.insert("summary".to_string(), champ(None, Some(&["main"])));
        config.entities.insert("Document".into(), EntityDef { fields: doc, hashsafe: None, derived_from: None });
        let mut author = HashMap::new();
        author.insert("bio".to_string(), champ(None, Some(&["main"])));
        config.entities.insert("Author".into(), EntityDef { fields: author, hashsafe: None, derived_from: None });
        config.relations.insert("WRITTEN_BY".into(), RelationDef { from: "Document".into(), to: "Author".into(), properties: None });
        config.knowledge_bases.insert("main".into(), KBConfig::default());
        config
    }

    #[test]
    fn une_kb_devient_une_derivee_avec_ses_regles_et_ses_gabarits() {
        let config = config();
        let cfg = derived_config_for_kb("main", &config.knowledge_bases["main"], &config).expect("traduite");
        let derived = cfg.derived.as_ref().unwrap();
        assert_eq!(derived.from, "Document");
        assert_eq!(derived.gather.len(), 1);
        assert_eq!(derived.gather[0].name, "Author");
        assert_eq!(derived.gather[0].relation, "WRITTEN_BY");
        assert_eq!(derived.gather[0].direction, GatherDirection::Out);
        assert_eq!(derived.gather[0].fields, vec!["bio".to_string()]);
        assert_eq!(derived.render["title"], "{{ root.title[:256] }}", "borné à title_max_chars par défaut");
        // Author avant Document : l'ordre des entités par nom, comme avant.
        assert_eq!(
            derived.render["content"],
            "{% for r in Author %}{% if r.bio %}{{ r.bio }}\n{% endif %}{% endfor %}{% if root.body %}{{ root.body }}\n{% endif %}{% if root.summary %}{{ root.summary }}\n{% endif %}"
        );
        assert!(cfg.fusion.is_some());
        assert!(cfg.fields["title"].is_title && cfg.fields["content"].is_content);
        cfg.validate().expect("la traduction est une config valide");
    }

    #[test]
    fn sans_racine_la_kb_attend() {
        let mut config = config();
        config.entities.get_mut("Document").unwrap().fields.get_mut("title").unwrap().title_for = None;
        assert!(derived_config_for_kb("main", &KBConfig::default(), &config).is_none());
    }

    #[test]
    fn le_gabarit_traduit_rend_le_texte_d_avant() {
        let config = config();
        let cfg = derived_config_for_kb("main", &config.knowledge_bases["main"], &config).unwrap();
        let contexte = serde_json::json!({
            "root": { "title": "T", "body": "corps", "summary": "" },
            "Author": [ { "_uuid": "a1", "bio": "Ana écrit." }, { "_uuid": "a2", "bio": "" } ]
        });
        let rendus = crate::dataflow::derive_nodes::render_fields(cfg.derived.as_ref().unwrap(), &contexte).unwrap();
        assert_eq!(rendus["title"], "T");
        assert_eq!(rendus["content"], "Ana écrit.\ncorps\n");
    }

    #[test]
    fn une_def_du_schema_donne_une_config_sans_pipeline_simple() {
        let config = config();
        let cfg = entity_config_from_def(&config.entities["Author"]);
        assert!(!cfg.has_simple_pipeline());
        assert!(cfg.has_kb_participation());
        assert_eq!(cfg.fields["bio"].content_for.as_deref(), Some(&["main".to_string()][..]));
    }
}
