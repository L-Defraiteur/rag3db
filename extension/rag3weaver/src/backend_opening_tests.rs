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
