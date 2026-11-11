//! Un schéma se monte selon ses dépendances, indépendamment de l'ordre JSON/map.
use super::*;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum SchemaStep<'a> {
    Entity(&'a str),
    Relation(&'a str),
}

pub(super) fn registration_order<'a>(entities: &'a HashMap<String, EntityConfig>,
    relations: &'a HashMap<String, BackendRelation>) -> Result<Vec<SchemaStep<'a>>, String> {
    use SchemaStep::*;
    let mut pending = BTreeMap::<SchemaStep<'a>, BTreeSet<SchemaStep<'a>>>::new();
    for (name, config) in entities {
        let mut dependencies = BTreeSet::new();
        if let Some(d) = &config.derived {
            d.validate_context(name, |n| entities.get(n), |n| relations.get(n).map(|r| &r.def))
                .map_err(|e| format!("{name}: {e}"))?;
            dependencies.insert(Entity(&d.from));
            for gather in &d.gather {
                if !relations.contains_key(&gather.relation) {
                    return Err(format!("{name}: derived : relation « {} » (règle « {} ») non déclarée",gather.relation,gather.name));
                }
                dependencies.insert(Relation(&gather.relation));
            }
        }
        pending.insert(Entity(name), dependencies);
    }
    for (name, relation) in relations {
        for endpoint in [&relation.from, &relation.to] {
            if !entities.contains_key(endpoint) {
                return Err(format!("{name}: extrémité « {endpoint} » non déclarée"));
            }
        }
        pending.insert(Relation(name),[Entity(&relation.from),Entity(&relation.to)].into_iter().collect());
    }
    let mut done = BTreeSet::new();
    let mut order = Vec::with_capacity(pending.len());
    while !pending.is_empty() {
        let next = pending.iter().find(|(_, dependencies)| dependencies.is_subset(&done))
            .map(|(step,_)| step.clone());
        let Some(next) = next else {
            let blocked = pending.keys().map(|step| match step {
                Entity(n) => format!("entité « {n} »"), Relation(n) => format!("relation « {n} »"),
            }).collect::<Vec<_>>().join(", ");
            return Err(format!("cycle de dépendances du schéma : {blocked}"));
        };
        pending.remove(&next);
        done.insert(next.clone());
        order.push(next);
    }
    Ok(order)
}
