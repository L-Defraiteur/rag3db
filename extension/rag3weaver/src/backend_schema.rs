//! Un schéma se monte selon ses dépendances, indépendamment de l'ordre JSON/map.
use super::*;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum SchemaStep {
    Entity(String),
    Relation(String),
}

pub(super) type ProvidedSchema = (
    BTreeMap<String, EntityConfig>,
    BTreeMap<String, RelationDef>,
);

pub(super) fn provided_schema(
    workspace: Option<&crate::backend_code::WorkspaceConfig>,
) -> ProvidedSchema {
    #[cfg(feature = "code")]
    if workspace.and_then(|w| w.index.as_deref()) == Some("code") {
        return crate::code::schema_configs(crate::code::default_scope_chunking());
    }
    let _ = workspace;
    Default::default()
}

/// Faits réels copiés sans perte : aucun schéma synthétique pour un fournisseur.
/// Une définition identique se partage ; deux sens différents ne s'écrasent pas.
pub(super) struct SchemaFacts {
    pub entities: HashMap<String, EntityConfig>,
    pub relations: HashMap<String, RelationDef>,
}

impl SchemaFacts {
    pub fn compose(
        entities: &HashMap<String, EntityConfig>,
        relations: &HashMap<String, BackendRelation>,
        provided: &ProvidedSchema,
    ) -> Result<Self, String> {
        let mut facts = Self {
            entities: provided
                .0
                .iter()
                .map(|(n, c)| (n.clone(), c.clone()))
                .collect(),
            relations: provided
                .1
                .iter()
                .map(|(n, r)| (n.clone(), r.clone()))
                .collect(),
        };
        // Le fournisseur se monte d'un bloc : ses dépendances sont internes.
        for (name, c) in &facts.entities {
            c.validate().map_err(|e| format!("{name}: {e}"))?;
        }
        facts.registration_order()?;
        for (name, config) in entities {
            if let Some(other) = facts.entities.get(name.as_str()) {
                if serde_json::to_value(config).map_err(|e| e.to_string())?
                    != serde_json::to_value(other).map_err(|e| e.to_string())?
                {
                    return Err(format!("schéma : définitions incompatibles de l'entité « {name} » (manifeste et fournisseur nommé)"));
                }
            } else {
                facts.entities.insert(name.clone(), config.clone());
            }
        }
        for (name, relation) in relations {
            if let Some(other) = facts.relations.get_mut(name) {
                if other.from != relation.from || other.to != relation.to {
                    return Err(format!(
                        "schéma : définitions incompatibles de la relation « {name} » (extrémités)"
                    ));
                }
                let properties = other.properties.get_or_insert_with(HashMap::new);
                for (field, definition) in relation.properties.as_ref().into_iter().flatten() {
                    if let Some(previous) = properties.get(field) {
                        if serde_json::to_value(previous).map_err(|e| e.to_string())?
                            != serde_json::to_value(definition).map_err(|e| e.to_string())?
                        {
                            return Err(format!("schéma : définitions incompatibles de la relation « {name} », propriété « {field} »"));
                        }
                    } else {
                        properties.insert(field.clone(), definition.clone());
                    }
                }
            } else {
                facts.relations.insert(name.clone(), relation.def.clone());
            }
        }
        Ok(facts)
    }

    pub fn registration_order(&self) -> Result<Vec<SchemaStep>, String> {
        use SchemaStep::*;
        let mut pending = BTreeMap::<SchemaStep, BTreeSet<SchemaStep>>::new();
        for (name, config) in &self.entities {
            // Une association de lecture doit être valable avant toute ouverture,
            // sans devenir l'arête inverse de la dépendance derived.from.
            config.validate_search_index_context(name, |n| self.entities.get(n))?;
            let mut dependencies = BTreeSet::new();
            if let Some(d) = &config.derived {
                d.validate_context(name, |n| self.entities.get(n), |n| self.relations.get(n))
                    .map_err(|e| format!("{name}: {e}"))?;
                dependencies.insert(Entity(d.from.clone()));
                for gather in &d.gather {
                    if !self.relations.contains_key(gather.relation.as_str()) {
                        return Err(format!(
                            "{name}: derived : relation « {} » (règle « {} ») non déclarée",
                            gather.relation, gather.name
                        ));
                    }
                    dependencies.insert(Relation(gather.relation.clone()));
                }
            }
            pending.insert(Entity(name.clone()), dependencies);
        }
        for (name, relation) in &self.relations {
            for endpoint in [&relation.from, &relation.to] {
                if !self.entities.contains_key(endpoint.as_str()) {
                    return Err(format!("{name}: extrémité « {endpoint} » non déclarée"));
                }
            }
            pending.insert(
                Relation(name.clone()),
                [Entity(relation.from.clone()), Entity(relation.to.clone())]
                    .into_iter()
                    .collect(),
            );
        }
        let mut done = BTreeSet::new();
        let mut order = Vec::with_capacity(pending.len());
        while !pending.is_empty() {
            let next = pending
                .iter()
                .find(|(_, dependencies)| dependencies.is_subset(&done))
                .map(|(step, _)| step.clone());
            let Some(next) = next else {
                let blocked = pending
                    .keys()
                    .map(|step| match step {
                        Entity(n) => format!("entité « {n} »"),
                        Relation(n) => format!("relation « {n} »"),
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                return Err(format!("cycle de dépendances du schéma : {blocked}"));
            };
            pending.remove(&next);
            done.insert(next.clone());
            order.push(next);
        }
        Ok(order)
    }
}
