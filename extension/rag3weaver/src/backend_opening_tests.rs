//! Contrat déclaratif d'ouverture : identité dérivée et dépendances du schéma.
use super::*;

fn manifest() -> (tempfile::TempDir, Value) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("row.json"), serde_json::to_vec(&json!({
        "type":"object", "properties":{"key":{"type":"string"},"body":{"type":"string"}}
    })).unwrap()).unwrap();
    let entity = json!({"schema":"row.json", "config":{
        "fields":{"key":{"type":"string"},"body":{"type":"string","isContent":true}},
        "hashsafe":["key"], "signals":["bm25"]
    }});
    let m = json!({"version":1,"name":"opening","database":"test.rag3db",
        "embeddings":{"address":"127.0.0.1:1","model":"toy","dimensions":8},
        "vector_extension":vector_extension_of_this_process().unwrap(),
        "entities":{"One":entity,"Two":entity,"Three":entity},"tools":{}});
    (dir,m)
}

fn prepare(dir: &Path, m: &Value) -> Result<PreparedBackend,String> {
    let path = dir.join("backend.json");
    std::fs::write(&path, serde_json::to_vec_pretty(m).unwrap()).unwrap();
    PreparedBackend::load(&path)
}

fn derived(from: &str, gather: Value) -> Value {
    json!({"from":from,"render":{"body":"{{ root.body }}"},"gather":gather})
}

#[test]
fn opening_manifest_accepts_identity_inherited_from_the_root() {
    let (dir,mut m) = manifest();
    m["entities"]["One"]["config"].as_object_mut().unwrap().remove("hashsafe");
    m["entities"]["One"]["config"]["derived"] = derived("Two",json!([]));
    prepare(dir.path(),&m).unwrap_or_else(|e| panic!("une dérivée hérite de l'identité de Two : {e}"));
}

// Isoler l'ouverture de son verrou de chargement : préparer les trois racines
// puis appliquer la configuration dérivée à la version préparée. Le premier
// témoin ci-dessus couvre le vrai chargement du manifeste dérivé.
fn prepared_for_open(gather: bool) -> (tempfile::TempDir,PreparedBackend) {
    let (dir,m) = manifest();
    let mut p = prepare(dir.path(),&m).unwrap();
    let names: Vec<_> = p.entities.keys().cloned().collect();
    // Contrôle de l'ordre réellement parcouru, sans supposer de graine HashMap.
    let (view,root,neighbor) = if gather { (&names[2],&names[0],&names[1]) }
        else { (&names[0],&names[1],&names[2]) };
    let rules = if gather { json!([{"name":"neighbors","relation":"LINK",
        "direction":"out","fields":["body"]}]) } else { json!([]) };
    if gather {
        p.manifest.relations.insert("LINK".into(),serde_json::from_value(json!({"from":root,"to":neighbor})).unwrap());
    }
    let config = p.entities.get_mut(view).unwrap();
    config.hashsafe = None;
    config.derived = Some(serde_json::from_value(derived(root,rules)).unwrap());
    p.manifest.entities.get_mut(view).unwrap().config = config.clone();
    (dir,p)
}

#[test]
fn opening_manifest_registers_the_root_before_its_derived_entity() {
    let (_dir,p) = prepared_for_open(false);
    p.open(Box::new(crate::Rag3dbConnection::in_memory().unwrap()),None)
        .unwrap_or_else(|e| panic!("racine requise avant dérivée, indépendamment de l'ordre de la map : {e}"));
}

#[test]
fn opening_manifest_registers_gather_relations_before_the_derived_entity() {
    let (_dir,p) = prepared_for_open(true);
    p.open(Box::new(crate::Rag3dbConnection::in_memory().unwrap()),None)
        .unwrap_or_else(|e| panic!("racines et relation gather requises avant dérivée : {e}"));
}

fn set_derived(m: &mut Value, view: &str, root: &str, gather: Value) {
    m["entities"][view]["config"].as_object_mut().unwrap().remove("hashsafe");
    m["entities"][view]["config"]["derived"] = derived(root,gather);
}

#[test]
fn opening_manifest_writes_the_root_and_searches_its_derived_entity_with_permuted_names() {
    use crate::connection::CypherValue;
    use crate::search::{Consistency,ResultMode,SearchOptions,SearchSignals};
    // Les rôles changent de noms ; aucun ordre alphabétique ou de map attendu.
    for (view,root,neighbor) in [("One","Two","Three"),("Three","One","Two"),("Two","Three","One")] {
        let (dir,mut m) = manifest();
        set_derived(&mut m,view,root,json!([{"name":"neighbors","relation":"LINK","fields":["body"]}]));
        m["relations"] = json!({"LINK":{"from":root,"to":neighbor}});
        m["entities"][view]["config"]["derived"]["render"]["body"] =
            json!("{{ root.body }} {% for n in neighbors %}{{ n.body }} {% endfor %}");
        let backend = prepare(dir.path(),&m).unwrap()
            .open(Box::new(crate::Rag3dbConnection::in_memory().unwrap()),None).unwrap();
        let source_uuid = {
            let mut cat = backend.catalog.lock().unwrap();
            cat.regime_d_ecriture(crate::disponibilite::RegimeEcriture::ParLot);
            let record = |key: &str,body: &str| BTreeMap::from([
                ("key".into(),CypherValue::String(key.into())),("body".into(),CypherValue::String(body.into()))]);
            let source = cat.create(root,record("r","chaudiere")).unwrap();
            let other = cat.create(neighbor,record("n","thermostat")).unwrap();
            cat.link("LINK",source.clone(),other,BTreeMap::new()).unwrap();
            let drain = cat.drain();
            assert_eq!(drain.failed,0,"{:?}",drain.warnings);
            source.uuid().unwrap().to_string()
        };
        let result = Catalog::rechercher(&backend.catalog,view,"thermostat",SearchOptions {
            signals:Some(SearchSignals::BM25),consistency:Consistency::Immediate,
            result_mode:ResultMode::SourceResolved,..Default::default()
        }).unwrap();
        assert_eq!(result.results.len(),1);
        assert_eq!(result.results[0].entity.as_deref(),Some(root));
        assert_eq!(result.results[0].uuid,source_uuid);
    }
}

#[test]
fn opening_manifest_refuses_missing_roots_relations_chains_and_cycles_by_name() {
    let (dir,base) = manifest();
    for (root,gather,relations,expected) in [
        ("Missing",json!([]),json!({}),"Missing"),
        ("Two",json!([{"name":"n","relation":"ABSENT"}]),json!({}),"ABSENT"),
        ("Two",json!([{"name":"n","relation":"LINK"}]),json!({"LINK":{"from":"Two","to":"One"}}),"cycle"),
        ("Two",json!([{"name":"n","relation":"LINK","fields":["unknown"]}]),json!({"LINK":{"from":"Two","to":"Three"}}),"unknown"),
        ("Two",json!([{"name":"n","relation":"LINK","direction":"in"}]),json!({"LINK":{"from":"Two","to":"Three"}}),"direction"),
    ] {
        let mut m = base.clone();
        set_derived(&mut m,"One",root,gather);
        m["relations"] = relations;
        let error = prepare(dir.path(),&m).err().expect("refus avant ouverture");
        assert!(error.contains(expected),"{expected}: {error}");
        assert!(error.contains("One"),"la dérivée en cause doit être nommée : {error}");
    }
    let mut m = base;
    set_derived(&mut m,"One","Two",json!([]));
    set_derived(&mut m,"Two","Three",json!([]));
    let error = prepare(dir.path(),&m).err().expect("chaîne refusée");
    assert!(error.contains("elle-même dérivée"),"{error}");
}

#[test]
fn opening_manifest_validates_the_whole_plan_before_any_database_call() {
    use std::sync::atomic::{AtomicUsize,Ordering};
    let (_dir,mut p) = prepared_for_open(true);
    let derived_name = p.entities.iter().find(|(_,c)| c.derived.is_some()).unwrap().0.clone();
    // La relation attend sa propre dérivée : cycle introduit après préparation.
    p.manifest.relations.get_mut("LINK").unwrap().def.to = derived_name;
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = calls.clone();
    let conn = crate::connection::CallbackConnection::new(move |_,_| {
        observed.fetch_add(1,Ordering::SeqCst);
        Ok(crate::connection::QueryResult::default())
    });
    let error = p.open(Box::new(conn),None).err().expect("plan cyclique refusé");
    assert!(error.contains("cycle"),"{error}");
    assert_eq!(calls.load(Ordering::SeqCst),0,"ni initialisation ni schéma partiel");
}
