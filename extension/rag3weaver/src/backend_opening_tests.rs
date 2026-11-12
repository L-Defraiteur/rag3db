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

#[cfg(feature = "code")]
fn code_workspace(m: &mut Value) {
    m["workspace"] = json!({"source":"working_tree","root":".","index":"code",
        "index_signals":{"File":["bm25"],"Scope":["bm25"],"Library":["bm25"],"Symbol":["bm25"]}});
}

#[cfg(feature = "code")]
#[test]
fn opening_named_schema_accepts_a_manifest_relation_to_its_entity() {
    let (dir,mut m) = manifest();
    code_workspace(&mut m);
    m["relations"] = json!({"ANNOTATES":{"from":"One","to":"Scope"}});
    let backend = prepare(dir.path(),&m).unwrap_or_else(|e| panic!("relation vers le schéma nommé : {e}"))
        .open(Box::new(crate::Rag3dbConnection::in_memory().unwrap()),None).unwrap();
    assert!(backend.catalog.lock().unwrap().relation_names().iter().any(|n| n=="ANNOTATES"));
    assert_eq!(backend.catalog.lock().unwrap().entity_config("Scope").unwrap().signals,
        crate::search::SearchSignals::BM25,"l'overlay déclaré n'est pas un conflit avec le fournisseur");
}

#[cfg(feature = "code")]
#[test]
fn opening_named_schema_accepts_a_derived_root_from_its_entity() {
    let (dir,mut m) = manifest();
    code_workspace(&mut m);
    set_derived(&mut m,"One","Scope",json!([]));
    m["entities"]["One"]["config"]["derived"]["render"]["body"] = json!("{{ root.content }}");
    prepare(dir.path(),&m).unwrap_or_else(|e| panic!("racine du schéma nommé : {e}"))
        .open(Box::new(crate::Rag3dbConnection::in_memory().unwrap()),None).unwrap();
}

#[cfg(feature = "code")]
#[test]
fn opening_named_schema_accepts_its_gather_relation_and_fields() {
    let (dir,mut m) = manifest();
    code_workspace(&mut m);
    set_derived(&mut m,"One","Scope",json!([{"name":"file","relation":"DEFINED_IN",
        "direction":"out","fields":["path"]}]));
    m["entities"]["One"]["config"]["derived"]["render"]["body"] = json!("{{ root.content }}");
    prepare(dir.path(),&m).unwrap_or_else(|e| panic!("gather du schéma nommé : {e}"))
        .open(Box::new(crate::Rag3dbConnection::in_memory().unwrap()),None).unwrap();
}

#[cfg(feature = "code")]
#[test]
fn opening_named_schema_shares_equivalent_facts_and_names_conflicting_entities() {
    let provided = crate::code::schema_configs(crate::code::default_scope_chunking());
    let mut explicit = HashMap::from([("File".into(),provided.0["File"].clone())]);
    let relations = HashMap::new();
    let facts = backend_schema::SchemaFacts::compose(&explicit,&relations,&provided).unwrap();
    assert_eq!(facts.entities.len(),provided.0.len());
    facts.registration_order().unwrap();
    drop(facts);
    explicit.get_mut("File").unwrap().signals = crate::search::SearchSignals::NONE;
    let error = backend_schema::SchemaFacts::compose(&explicit,&relations,&provided)
        .err().expect("deux définitions différentes ne s'écrasent pas");
    assert!(error.contains("File") && error.contains("incompatibles"),"{error}");
}

#[cfg(feature = "code")]
#[test]
fn opening_named_schema_coalesces_relations_with_the_same_effective_properties() {
    let (dir,mut m) = manifest();
    code_workspace(&mut m);
    // Le fournisseur emploie Some({}), le manifeste omet les propriétés.
    m["relations"] = json!({"DEFINED_IN":{"from":"Scope","to":"File"}});
    let backend = prepare(dir.path(),&m).unwrap()
        .open(Box::new(crate::Rag3dbConnection::in_memory().unwrap()),None).unwrap();
    assert_eq!(backend.catalog.lock().unwrap().relation_names().iter().filter(|n|n.as_str()=="DEFINED_IN").count(),1);
}

#[cfg(feature = "code")]
#[test]
fn opening_named_schema_keeps_compatible_relation_extensions() {
    let (dir,mut m) = manifest();
    code_workspace(&mut m);
    m["relations"] = json!({"MENTIONS":{"from":"Scope","to":"Symbol",
        "properties":{"note":{"type":"string"}}}});
    let backend = prepare(dir.path(),&m).unwrap()
        .open(Box::new(crate::Rag3dbConnection::in_memory().unwrap()),None).unwrap();
    let cat = backend.catalog.lock().unwrap();
    let properties = cat.get_relation_def("MENTIONS").unwrap().properties.as_ref().unwrap();
    assert!(properties.contains_key("note") && properties.contains_key("kind"));
}

#[cfg(feature = "code")]
#[test]
fn opening_named_schema_refuses_conflicting_property_types() {
    let (dir,mut m) = manifest();
    code_workspace(&mut m);
    m["relations"] = json!({"MENTIONS":{"from":"Scope","to":"Symbol",
        "properties":{"kind":{"type":"int64"}}}});
    let error = prepare(dir.path(),&m).err().expect("type incompatible refusé au chargement");
    assert!(error.contains("MENTIONS") && error.contains("kind") && error.contains("incompatibles"),"{error}");
}

#[cfg(feature = "code")]
#[test]
fn opening_named_schema_refuses_conflicting_relations_before_any_database_call() {
    use std::sync::atomic::{AtomicUsize,Ordering};
    let (dir,mut m) = manifest();
    code_workspace(&mut m);
    let mut p = prepare(dir.path(),&m).unwrap();
    p.manifest.relations.insert("DEFINED_IN".into(),serde_json::from_value(json!({"from":"One","to":"Scope"})).unwrap());
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = calls.clone();
    let conn = crate::connection::CallbackConnection::new(move |_,_| {
        observed.fetch_add(1,Ordering::SeqCst);
        Ok(crate::connection::QueryResult::default())
    });
    let error = p.open(Box::new(conn),None).err().expect("conflit de définitions refusé");
    assert!(error.contains("DEFINED_IN") && error.contains("incompatibles"),"{error}");
    assert_eq!(calls.load(Ordering::SeqCst),0);
}

#[cfg(feature = "code")]
#[test]
fn named_catalog_existing_relation_extension_is_additive() {
    let (dir,mut m) = manifest();
    code_workspace(&mut m);
    let backend = prepare(dir.path(),&m).unwrap()
        .open(Box::new(crate::Rag3dbConnection::in_memory().unwrap()),None).unwrap();
    let mut cat = backend.catalog.lock().unwrap();
    let note: crate::config::FieldDef = serde_json::from_value(json!({"type":"string"})).unwrap();
    cat.register_relation_with("MENTIONS","Scope","Symbol",HashMap::from([("note".into(),note)])).unwrap();
    let props = cat.get_relation_def("MENTIONS").unwrap().properties.as_ref().unwrap();
    assert!(props.contains_key("kind") && props.contains_key("note"));
}

#[cfg(feature = "code")]
#[test]
fn named_catalog_existing_relation_endpoints_cannot_be_replaced() {
    let (dir,mut m) = manifest();
    code_workspace(&mut m);
    let backend = prepare(dir.path(),&m).unwrap()
        .open(Box::new(crate::Rag3dbConnection::in_memory().unwrap()),None).unwrap();
    let mut cat = backend.catalog.lock().unwrap();
    let error = cat.register_relation_with("MENTIONS","Symbol","Scope",HashMap::new()).unwrap_err();
    assert!(error.to_string().contains("MENTIONS"));
    let def = cat.get_relation_def("MENTIONS").unwrap();
    assert_eq!((&def.from,&def.to),(&"Scope".to_string(),&"Symbol".to_string()));
}

fn public_content_index(m: &mut Value, view: &str, root: &str) {
    set_derived(m, view, root, json!([]));
    m["entities"][view]["config"]["derived"]["identity"] = json!({"mode":"content","format":"opening-content-v1"});
    m["entities"][root]["config"]["searchIndex"] = json!(view);
    m["entities"][root]["config"]["returnFields"] = json!(["key"]);
}

#[test]
fn opening_public_content_manifest_writes_roots_and_searches_occurrences() {
    use crate::connection::CypherValue;
    use crate::search::{Consistency,ResultMode,SearchOptions,SearchSignals};
    for (view,root) in [("One","Two"),("Three","One"),("Two","Three")] {
        let (dir,mut m) = manifest();
        public_content_index(&mut m,view,root);
        let backend = prepare(dir.path(),&m).unwrap()
            .open(Box::new(crate::Rag3dbConnection::in_memory().unwrap()),None).unwrap();
        {
            let mut cat = backend.catalog.lock().unwrap();
            cat.regime_d_ecriture(crate::disponibilite::RegimeEcriture::ParLot);
            for key in ["a","b"] {
                cat.create(root,BTreeMap::from([
                    ("key".into(),CypherValue::String(key.into())),
                    ("body".into(),CypherValue::String("marquemanifeste".into())),
                ])).unwrap();
            }
            let result = cat.drain();
            assert_eq!(result.failed,0,"{:?}",result.warnings);
            let rows = cat.conn().execute(&format!("MATCH (v:{view}) RETURN count(v)")).unwrap();
            assert_eq!(rows.rows[0][0].as_i64(),Some(1));
            assert!(cat.fermeture(root,false).contains(view),"l'index est une dépendance de lecture, pas un faux cycle de création");
        }
        for mode in [ResultMode::Aggregated,ResultMode::Detailed,ResultMode::SourceResolved] {
            let result = Catalog::rechercher(&backend.catalog,root,"marquemanifeste",SearchOptions {
                signals:Some(SearchSignals::BM25),consistency:Consistency::Strict,
                result_mode:mode,strict_signals:true,..Default::default()
            }).unwrap();
            assert_eq!(result.results.len(),2,"{root} -> {view}, {mode:?}: {:?}",result.meta);
            let mut keys: Vec<_> = result.results.iter().map(|r| {
                assert_eq!(r.entity.as_deref(),Some(root));
                r.data.as_ref().unwrap()["key"].as_str().unwrap().to_string()
            }).collect();
            keys.sort();
            assert_eq!(keys,vec!["a","b"]);
        }
    }
}

#[test]
fn opening_public_content_manifest_refuses_invalid_associations_before_database() {
    let (dir,base) = manifest();
    for case in ["missing","root_identity","wrong_origin","self","chain"] {
        let mut m = base.clone();
        public_content_index(&mut m,"One","Two");
        match case {
            "missing" => m["entities"]["Two"]["config"]["searchIndex"] = json!("Missing"),
            "root_identity" => m["entities"]["One"]["config"]["derived"]["identity"] = json!({"mode":"root"}),
            "wrong_origin" => m["entities"]["One"]["config"]["derived"]["from"] = json!("Three"),
            "self" => m["entities"]["Two"]["config"]["searchIndex"] = json!("Two"),
            "chain" => m["entities"]["One"]["config"]["searchIndex"] = json!("Three"),
            _ => unreachable!(),
        }
        let error = prepare(dir.path(),&m).err().unwrap_or_else(|| panic!("{case}: association invalide acceptée avant ouverture"));
        assert!(error.contains("Two") || (case == "chain" && error.contains("One")),"{case}: {error}");
    }

    // La configuration préparée peut aussi être modifiée avant open : sa
    // validation ne doit jamais se fier au seul succès antérieur du chargement.
    use std::sync::atomic::{AtomicUsize,Ordering};
    struct NoDatabase(Arc<AtomicUsize>);
    impl crate::connection::DbConnection for NoDatabase {
        fn execute(&self,_: &str) -> Result<crate::connection::QueryResult,crate::connection::DbError> {
            self.0.fetch_add(1,Ordering::SeqCst);
            Err(crate::connection::DbError::QueryError("database must remain untouched".into()))
        }
        fn execute_with_params(&self,q: &str,_: &[crate::connection::QueryParam]) -> Result<crate::connection::QueryResult,crate::connection::DbError> { self.execute(q) }
    }
    let mut valid = base;
    public_content_index(&mut valid,"One","Two");
    let mut prepared = prepare(dir.path(),&valid).unwrap();
    prepared.entities.get_mut("Two").unwrap().search_index = Some("Missing".into());
    let calls = Arc::new(AtomicUsize::new(0));
    let error = prepared.open(Box::new(NoDatabase(calls.clone())),None).err().expect("refus avant le premier appel base");
    assert!(error.contains("Missing"),"{error}");
    assert_eq!(calls.load(Ordering::SeqCst),0);

    // La façade produit doit refuser la même mutation avant d'ouvrir sa
    // connexion native ou de construire un embarqueur.
    let mut prepared = prepare(dir.path(),&valid).unwrap();
    prepared.entities.get_mut("Two").unwrap().search_index = Some("Missing".into());
    let database = prepared.path(&prepared.manifest.database);
    assert!(!database.exists());
    let constructions = Arc::new(AtomicUsize::new(0));
    let observed = constructions.clone();
    let error = prepared.open_on_disk_with_embedder(move |_| {
        observed.fetch_add(1,Ordering::SeqCst);
        (None,None)
    }).err().expect("refus pur avant connexion native et fabrique modèle");
    assert!(error.contains("Missing"),"{error}");
    assert_eq!(constructions.load(Ordering::SeqCst),0);
    assert!(!database.exists(),"la façade ne crée pas une base pour un plan invalide");
}
