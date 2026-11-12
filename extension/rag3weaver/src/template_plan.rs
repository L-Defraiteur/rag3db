//! Plans de gabarits sur instantanés fournis. Ni acquisition, ni application.
//!
//! Une réussite valide les déclarations, pas les données ni une future ouverture.
//! Les objets produits ne portent aucun droit d'écriture ou de migration.
use crate::lecteur::{Contenus, Lecteur};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Package {
    /// Chemins relatifs à la racine logique, inventoriés explicitement par l'appelant.
    pub files: BTreeMap<String, Vec<u8>>,
    /// Contenus des références builtin de CET instantané, sans repli sur le binaire.
    pub builtin: BTreeMap<String, Vec<u8>>,
    pub fingerprint: String,
    /// Information reçue ; le plan ne résout ni ne certifie une révision Git.
    pub declared_provenance: Option<String>,
}

impl Package {
    pub fn new(files: BTreeMap<String, Vec<u8>>, builtin: BTreeMap<String, Vec<u8>>) -> Self {
        let fingerprint = fingerprint(&files, &builtin);
        Self {
            files,
            builtin,
            fingerprint,
            declared_provenance: None,
        }
    }
}

/// Inventaire canonique avec cadres de longueur et espaces de noms distincts.
pub fn fingerprint(
    files: &BTreeMap<String, Vec<u8>>,
    builtin: &BTreeMap<String, Vec<u8>>,
) -> String {
    fn frame(h: &mut blake3::Hasher, bytes: &[u8]) {
        h.update(&(bytes.len() as u64).to_be_bytes());
        h.update(bytes);
    }
    let mut h = blake3::Hasher::new();
    frame(&mut h, b"rag3weaver-template-contents-v1");
    for (name, contents) in [("files", files), ("builtin", builtin)] {
        frame(&mut h, name.as_bytes());
        h.update(&(contents.len() as u64).to_be_bytes());
        for (path, bytes) in contents {
            frame(&mut h, path.as_bytes());
            frame(&mut h, bytes);
        }
    }
    h.finalize().to_hex().to_string()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RefusalKind {
    MissingBase,
    AmbiguousBase,
    Fingerprint,
    Source,
    Conflict,
    Compatibility,
    Validation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Refusal {
    pub kind: RefusalKind,
    pub path: String,
    pub reason: String,
    pub conflict: Option<crate::fusion::Conflict>,
}

fn refusal(kind: RefusalKind, path: impl Into<String>, reason: impl Into<String>) -> Refusal {
    Refusal {
        kind,
        path: path.into(),
        reason: reason.into(),
        conflict: None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ToolDiff {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    /// Attachements modifiés ; les changements de contenus figurent dans Plan::files.
    pub changed: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceIdentity {
    pub fingerprint: String,
    pub declared_provenance: Option<String>,
}
impl From<&Package> for SourceIdentity {
    fn from(value: &Package) -> Self {
        Self {
            fingerprint: value.fingerprint.clone(),
            declared_provenance: value.declared_provenance.clone(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeKind {
    Added,
    Removed,
    Changed,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileChange {
    pub path: String,
    pub kind: ChangeKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plan {
    pub base: SourceIdentity,
    pub local: SourceIdentity,
    pub target: SourceIdentity,
    pub candidate: Package,
    pub tools: ToolDiff,
    pub files: Vec<FileChange>,
}

fn file_diff(local: &Package, candidate: &Package) -> Vec<FileChange> {
    let mut out = BTreeMap::new();
    for (prefix, l, c) in [
        ("", &local.files, &candidate.files),
        ("builtin:", &local.builtin, &candidate.builtin),
    ] {
        let names: BTreeSet<&String> = l.keys().chain(c.keys()).collect();
        for name in names {
            let kind = match (l.get(name), c.get(name)) {
                (None, Some(_)) => ChangeKind::Added,
                (Some(_), None) => ChangeKind::Removed,
                (Some(a), Some(b)) if a != b => ChangeKind::Changed,
                _ => continue,
            };
            out.insert(format!("{prefix}{name}"), kind);
        }
    }
    out.into_iter()
        .map(|(path, kind)| FileChange { path, kind })
        .collect()
}

fn root() -> PathBuf {
    PathBuf::from(std::path::MAIN_SEPARATOR_STR).join("__rag3weaver_template_contents__")
}

fn reader(package: &Package) -> Result<Contenus, Refusal> {
    if fingerprint(&package.files, &package.builtin) != package.fingerprint {
        return Err(refusal(
            RefusalKind::Fingerprint,
            "package",
            "empreinte fournie incompatible avec les contenus",
        ));
    }
    Contenus::nouveau(&root(), &package.files, &package.builtin)
        .map_err(|e| refusal(RefusalKind::Source, "package", e))
}

fn manifest(package: &Package, name: &str) -> Result<Value, Refusal> {
    let bytes = package.files.get(name).ok_or_else(|| {
        refusal(
            RefusalKind::Source,
            name,
            "manifeste requis absent des contenus",
        )
    })?;
    serde_json::from_slice(bytes).map_err(|e| refusal(RefusalKind::Validation, name, e.to_string()))
}

fn merge_files(
    base: &BTreeMap<String, Vec<u8>>,
    local: &BTreeMap<String, Vec<u8>>,
    target: &BTreeMap<String, Vec<u8>>,
    except: Option<&str>,
    prefix: &str,
    errors: &mut Vec<Refusal>,
) -> BTreeMap<String, Vec<u8>> {
    let paths: BTreeSet<&String> = base
        .keys()
        .chain(local.keys())
        .chain(target.keys())
        .collect();
    let mut out = BTreeMap::new();
    for path in paths {
        if except == Some(path.as_str()) {
            continue;
        }
        match crate::fusion::atomic(base.get(path), local.get(path), target.get(path)) {
            Ok(Some(bytes)) => {
                out.insert(path.clone(), bytes);
            }
            Ok(None) => {}
            Err(_) => errors.push(refusal(
                RefusalKind::Conflict,
                format!("{prefix}{path}"),
                "contenu modifié différemment des deux côtés ; aucune fusion textuelle implicite",
            )),
        }
    }
    out
}

fn schema_content(reader: &Contenus, manifest_name: &str, path: &str) -> Result<Value, Refusal> {
    let base = root().join(manifest_name).parent().unwrap().to_path_buf();
    let text = reader
        .lire_reference(&base, path)
        .map_err(|e| refusal(RefusalKind::Source, path, e))?;
    serde_json::from_str(&text).map_err(|e| refusal(RefusalKind::Validation, path, e.to_string()))
}

/// Les références externes du compilateur JSON Schema n'ont pas encore de raccord
/// au lecteur fermé : refus de cette capacité, pas un résolveur parallèle.
fn refuse_external_schema_refs(value: &Value, path: &str, errors: &mut Vec<Refusal>) {
    match value {
        Value::Object(map) => {
            for reference in ["$ref", "$dynamicRef", "$recursiveRef"]
                .iter()
                .filter_map(|key| map.get(*key).and_then(Value::as_str))
            {
                if !reference.starts_with('#') {
                    errors.push(refusal(RefusalKind::Source, path, format!("référence de schéma externe non prise en charge par la validation fermée : {reference}")));
                }
            }
            for v in map.values() {
                refuse_external_schema_refs(v, path, errors);
            }
        }
        Value::Array(values) => {
            for v in values {
                refuse_external_schema_refs(v, path, errors);
            }
        }
        _ => {}
    }
}

/// Premier périmètre fail-closed : aucune compatibilité de données n'est déduite
/// d'un ajout facultatif, d'une nouvelle relation ou d'un changement de format.
fn compatibility(
    local: &Value,
    candidate: &Value,
    local_reader: &Contenus,
    candidate_reader: &Contenus,
    manifest_name: &str,
) -> Vec<Refusal> {
    let mut errors = Vec::new();
    for key in [
        "version",
        "storage",
        "database",
        "vector_extension",
        "seed",
        "files_root",
        "files_format",
        "fts_positions",
        "entities",
        "relations",
        "embeddings",
        "models",
        "workspace",
    ] {
        if local.get(key) != candidate.get(key) {
            errors.push(refusal(
                RefusalKind::Compatibility,
                format!("{manifest_name}#/{key}"),
                "changement exigeant une qualification hors de ce plan déclaratif",
            ));
        }
    }
    if let Some(entities) = candidate.get("entities").and_then(Value::as_object) {
        for (name, entity) in entities {
            let Some(schema_path) = entity.get("schema").and_then(Value::as_str) else {
                continue;
            };
            let schema = match schema_content(candidate_reader, manifest_name, schema_path) {
                Ok(schema) => schema,
                Err(e) => {
                    errors.push(e);
                    continue;
                }
            };
            refuse_external_schema_refs(&schema, schema_path, &mut errors);
            let Some(old_path) = local
                .get("entities")
                .and_then(|v| v.get(name))
                .and_then(|v| v.get("schema"))
                .and_then(Value::as_str)
            else {
                continue;
            };
            match schema_content(local_reader, manifest_name, old_path) {
                Ok(old) if old != schema => errors.push(refusal(RefusalKind::Compatibility, schema_path, "schéma modifié : aucune migration ou compatibilité des données n'est certifiée")),
                Err(e) => errors.push(e),
                _ => {}
            }
        }
    }
    errors
}

fn tool_diff(local: &Value, candidate: &Value) -> ToolDiff {
    let l: BTreeMap<&str, &Value> = local
        .get("tools")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .map(|(k, v)| (k.as_str(), v))
        .collect();
    let c: BTreeMap<&str, &Value> = candidate
        .get("tools")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .map(|(k, v)| (k.as_str(), v))
        .collect();
    ToolDiff {
        added: c
            .keys()
            .filter(|k| !l.contains_key(**k))
            .map(|k| (*k).to_string())
            .collect(),
        removed: l
            .keys()
            .filter(|k| !c.contains_key(**k))
            .map(|k| (*k).to_string())
            .collect(),
        changed: c
            .iter()
            .filter(|(k, v)| l.get(**k).is_some_and(|old| old != *v))
            .map(|(k, _)| (*k).to_string())
            .collect(),
    }
}

/// Les bases candidates sont explicites : aucune recherche Git, choix par date,
/// adoption de copie ressemblante ou substitution de builtin n'est effectuée.
/// Les textes de fichiers sont atomiques ; une divergence nommée refuse le plan.
pub fn plan(
    bases: &[Package],
    local: &Package,
    target: &Package,
    manifest_name: &str,
) -> Result<Plan, Vec<Refusal>> {
    let base = match bases {
        [] => {
            return Err(vec![refusal(
                RefusalKind::MissingBase,
                "base",
                "aucune base fournie",
            )])
        }
        [base] => base,
        _ => {
            return Err(vec![refusal(
                RefusalKind::AmbiguousBase,
                "base",
                "plusieurs bases fournies ; aucune sélection implicite",
            )])
        }
    };
    let base_reader = reader(base).map_err(|e| vec![e])?;
    let local_reader = reader(local).map_err(|e| vec![e])?;
    let _target_reader = reader(target).map_err(|e| vec![e])?;
    // Le nom du manifeste doit être une identité relative non ambiguë, comme les autres sources.
    let path = root().join(manifest_name);
    if base_reader.identite(&path).is_err() || !base.files.contains_key(manifest_name) {
        return Err(vec![refusal(
            RefusalKind::Source,
            manifest_name,
            "chemin de manifeste absent ou ambigu",
        )]);
    }
    let b = manifest(base, manifest_name).map_err(|e| vec![e])?;
    let l = manifest(local, manifest_name).map_err(|e| vec![e])?;
    let t = manifest(target, manifest_name).map_err(|e| vec![e])?;
    let merged = crate::fusion::json(Some(&b), Some(&l), Some(&t))
        .map_err(|conflicts| {
            conflicts
                .into_iter()
                .map(|conflict| {
                    let mut r = refusal(
                        RefusalKind::Conflict,
                        format!("{manifest_name}#{}", conflict.pointer()),
                        "changements JSON divergents",
                    );
                    r.conflict = Some(conflict);
                    r
                })
                .collect::<Vec<_>>()
        })?
        .ok_or_else(|| {
            vec![refusal(
                RefusalKind::Validation,
                manifest_name,
                "le manifeste ne peut pas être supprimé",
            )]
        })?;
    let mut errors = Vec::new();
    let mut files = merge_files(
        &base.files,
        &local.files,
        &target.files,
        Some(manifest_name),
        "",
        &mut errors,
    );
    let builtin = merge_files(
        &base.builtin,
        &local.builtin,
        &target.builtin,
        None,
        "builtin:",
        &mut errors,
    );
    if !errors.is_empty() {
        return Err(errors);
    }
    // Une proposition sémantiquement inchangée conserve les octets locaux.
    let bytes = if merged == l {
        local.files[manifest_name].clone()
    } else if merged == t {
        target.files[manifest_name].clone()
    } else {
        serde_json::to_vec_pretty(&merged).map_err(|e| {
            vec![refusal(
                RefusalKind::Validation,
                manifest_name,
                e.to_string(),
            )]
        })?
    };
    files.insert(manifest_name.into(), bytes);
    let candidate = Package::new(files, builtin);
    let candidate_reader = reader(&candidate).map_err(|e| vec![e])?;
    let errors = compatibility(&l, &merged, &local_reader, &candidate_reader, manifest_name);
    if !errors.is_empty() {
        return Err(errors);
    }
    crate::backend::PreparedBackend::load_avec(&path, &candidate_reader)
        .map_err(|e| vec![refusal(RefusalKind::Validation, manifest_name, e)])?;
    Ok(Plan {
        base: SourceIdentity::from(base),
        local: SourceIdentity::from(local),
        target: SourceIdentity::from(target),
        tools: tool_diff(&l, &merged),
        files: file_diff(local, &candidate),
        candidate,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    const GRAPH: &str = "%% tool: echo\n%% description: fourni\n%% result: calc.result\n\ngraph LR\n    calc[\"RhaiNode(script='input')\"]\n";
    fn fixture() -> Package {
        Package::new(BTreeMap::from([
            ("backend.json".into(), serde_json::to_vec(&json!({"version":1,"name":"m8","database":"memory.rag3db","entities":{},"embeddings":{"model":"fixture","dimensions":4,"address":"127.0.0.1:1"},"tools":{"echo":{"graph":"echo.mmd"}}})).unwrap()),
            ("echo.mmd".into(), GRAPH.as_bytes().to_vec()),
        ]), BTreeMap::new())
    }
    fn edit(p: &Package, f: impl FnOnce(&mut Value)) -> Package {
        let mut out = p.clone();
        let mut m = manifest(p, "backend.json").unwrap();
        f(&mut m);
        out.files
            .insert("backend.json".into(), serde_json::to_vec(&m).unwrap());
        out.fingerprint = fingerprint(&out.files, &out.builtin);
        out
    }
    fn rehash(p: &mut Package) {
        p.fingerprint = fingerprint(&p.files, &p.builtin);
    }

    #[test]
    fn the_common_validator_accepts_a_complete_virtual_candidate() {
        let b = fixture();
        let l = edit(&b, |v| v["name"] = json!("local"));
        let t = edit(&b, |v| {
            v["tools"]["second"] = json!({"graph":"echo.mmd"});
        });
        let result = plan(&[b], &l, &t, "backend.json").unwrap();
        assert_eq!(
            manifest(&result.candidate, "backend.json").unwrap()["name"],
            "local"
        );
        assert_eq!(result.tools.added, ["second"]);
        assert!(
            !root().exists(),
            "l'identité logique ne doit pas être matérialisée"
        );
    }
    #[test]
    fn missing_ambiguous_and_bad_fingerprint_refuse() {
        let b = fixture();
        assert_eq!(
            plan(&[], &b, &b, "backend.json").unwrap_err()[0].kind,
            RefusalKind::MissingBase
        );
        assert_eq!(
            plan(&[b.clone(), b.clone()], &b, &b, "backend.json").unwrap_err()[0].kind,
            RefusalKind::AmbiguousBase
        );
        let mut bad = b.clone();
        bad.fingerprint = "faux".into();
        assert_eq!(
            plan(&[bad], &b, &b, "backend.json").unwrap_err()[0].kind,
            RefusalKind::Fingerprint
        );
    }
    #[test]
    fn file_divergence_and_removal_against_edit_are_named() {
        let b = fixture();
        let mut l = b.clone();
        let mut t = b.clone();
        l.files.remove("echo.mmd");
        rehash(&mut l);
        t.files.insert("echo.mmd".into(), b"changed".to_vec());
        rehash(&mut t);
        let errors = plan(&[b], &l, &t, "backend.json").unwrap_err();
        assert_eq!(errors[0].kind, RefusalKind::Conflict);
        assert_eq!(errors[0].path, "echo.mmd");
    }
    #[test]
    fn supplied_builtin_never_becomes_the_binary_builtin() {
        let b = fixture();
        let mut supplied = edit(&b, |v| {
            v["tools"]["echo"]["graph"] = json!("builtin:tools/list.mmd")
        });
        supplied
            .builtin
            .insert("tools/list.mmd".into(), GRAPH.as_bytes().to_vec());
        rehash(&mut supplied);
        plan(&[supplied.clone()], &supplied, &supplied, "backend.json").unwrap();
        let mut missing = supplied.clone();
        missing.builtin.clear();
        rehash(&mut missing);
        let errors = plan(&[missing.clone()], &missing, &missing, "backend.json").unwrap_err();
        assert!(
            errors[0].reason.contains("source requise absente")
                && errors[0].reason.contains("tools/list.mmd")
        );
    }
    #[test]
    fn missing_dependency_is_not_read_from_disk() {
        let dir = tempfile::tempdir().unwrap();
        let graph = dir.path().join("graph.mmd");
        std::fs::write(&graph, GRAPH).unwrap();
        let b = edit(&fixture(), |v| v["tools"]["echo"]["graph"] = json!(graph));
        let errors = plan(&[b.clone()], &b, &b, "backend.json").unwrap_err();
        assert!(
            errors[0].reason.contains("source requise absente"),
            "{errors:?}"
        );
        assert_eq!(std::fs::read_to_string(&graph).unwrap(), GRAPH);
    }
    #[test]
    fn schema_changes_refuse_without_opening_a_database() {
        let b = fixture();
        let t = edit(&b, |v| {
            v["entities"]["Note"] = json!({"schema":"note.json"})
        });
        let e = plan(&[b.clone()], &b, &t, "backend.json").unwrap_err();
        assert!(e
            .iter()
            .any(|r| r.kind == RefusalKind::Compatibility && r.path.ends_with("/entities")));
    }
    #[test]
    fn no_op_preserves_bytes_and_reports_no_tools() {
        let b = fixture();
        let result = plan(&[b.clone()], &b, &b, "backend.json").unwrap();
        assert_eq!(result.candidate.files, b.files);
        assert_eq!(result.tools, ToolDiff::default());
        assert_eq!(result, plan(&[b.clone()], &b, &b, "backend.json").unwrap());
    }
    #[test]
    fn nested_builtin_uses_the_supplied_snapshot() {
        let mut b = fixture();
        b.files.insert("echo.mmd".into(), b"%% tool: echo\n%% description: parent\n%% result: each.report\n\ngraph LR\n    each[\"ForEachNode(graph='@builtin:tools/list.mmd')\"]\n".to_vec());
        b.builtin.insert("tools/list.mmd".into(), b"%% tool: child\n%% description: fourni\n%% param: element json! -- valeur\n%% result: r.result\n\ngraph LR\n    r[\"RhaiNode(script='input', value=$element)\"]\n".to_vec());
        rehash(&mut b);
        plan(&[b.clone()], &b, &b, "backend.json").unwrap();
        b.builtin.clear();
        rehash(&mut b);
        let e = plan(&[b.clone()], &b, &b, "backend.json").unwrap_err();
        assert!(e[0].reason.contains("builtin:tools/list.mmd") && e[0].reason.contains("absente"));
    }
    fn with_schema() -> Package {
        let mut b = edit(
            &fixture(),
            |v| v["entities"]["Note"] = json!({"schema":"note.json","config":{"fields":{"name":{"type":"string","isTitle":true,"isContent":true}},"signals":["bm25"],"hashsafe":["name"]}}),
        );
        b.files.insert("note.json".into(), serde_json::to_vec(&json!({"type":"object","properties":{"name":{"type":"string"}},"required":["name"]})).unwrap());
        rehash(&mut b);
        b
    }
    #[test]
    fn changing_a_schema_file_is_not_an_implicitly_safe_optional_addition() {
        let b = with_schema();
        plan(&[b.clone()], &b, &b, "backend.json").unwrap();
        let mut t = b.clone();
        t.files.insert("note.json".into(), serde_json::to_vec(&json!({"type":"object","properties":{"name":{"type":"string"},"optional":{"type":"string"}},"required":["name"]})).unwrap());
        rehash(&mut t);
        let e = plan(&[b.clone()], &b, &t, "backend.json").unwrap_err();
        assert!(e
            .iter()
            .any(|r| r.kind == RefusalKind::Compatibility && r.path == "note.json"));
    }
    #[test]
    fn unqualified_storage_resource_and_seed_transitions_refuse() {
        let plain = fixture();
        let storage = json!({"application_format":"memory-v1","recipe":"recipe-v1"});
        let pinned = edit(&plain, |v| v["storage"] = storage.clone());
        let cases = vec![
            (plain.clone(), pinned.clone(), "storage"),
            (pinned.clone(), plain.clone(), "storage"),
            (
                pinned.clone(),
                edit(&pinned, |v| {
                    v["storage"]["application_format"] = json!("memory-v2")
                }),
                "storage",
            ),
            (
                pinned.clone(),
                edit(&pinned, |v| v["storage"]["recipe"] = json!("recipe-v2")),
                "storage",
            ),
            (
                plain.clone(),
                edit(&plain, |v| {
                    v["vector_extension"] = json!("private/vector.so")
                }),
                "vector_extension",
            ),
            (
                plain.clone(),
                edit(&plain, |v| v["seed"] = json!({"Note":[{"name":"new"}]})),
                "seed",
            ),
        ];
        for (local, target, key) in cases {
            let errors = plan(&[local.clone()], &local, &target, "backend.json").expect_err(key);
            assert!(
                errors.iter().any(|r| r.kind == RefusalKind::Compatibility
                    && r.path == format!("backend.json#/{key}")),
                "{key}: {errors:?}"
            );
        }
    }
    #[test]
    fn file_format_changes_are_never_silently_qualified() {
        let b = fixture();
        let t = edit(&b, |v| v["files_format"] = json!({"version":2}));
        let e = plan(&[b.clone()], &b, &t, "backend.json").unwrap_err();
        assert!(e
            .iter()
            .any(|r| r.kind == RefusalKind::Compatibility && r.path.ends_with("/files_format")));
    }
    #[cfg(all(feature = "code", target_os = "linux"))]
    #[test]
    fn a_physical_sandbox_check_is_not_executed_by_the_plan() {
        let b = edit(
            &fixture(),
            |v| v["workspace"] = json!({"source":"working_tree","root":"workspace","commands":"auto","sandbox":{"mode":"landlock"}}),
        );
        let e = plan(&[b.clone()], &b, &b, "backend.json").unwrap_err();
        assert!(
            e.iter()
                .any(|r| r.reason.contains("preuve d'environnement")
                    && r.reason.contains("landlock")),
            "{e:?}"
        );
    }
    #[test]
    fn external_schema_resolution_is_refused_before_the_common_compiler() {
        let mut b = with_schema();
        b.files.insert(
            "note.json".into(),
            br#"{"$ref":"file:///source-non-fournie.json"}"#.to_vec(),
        );
        rehash(&mut b);
        let e = plan(&[b.clone()], &b, &b, "backend.json").unwrap_err();
        assert!(e
            .iter()
            .any(|r| r.kind == RefusalKind::Source
                && r.reason.contains("externe non prise en charge")));
    }
    #[test]
    fn content_changes_are_not_mistaken_for_unchanged_tool_attachments() {
        let b = fixture();
        let mut t = b.clone();
        t.files.insert(
            "echo.mmd".into(),
            GRAPH.replace("fourni", "nouveau").into_bytes(),
        );
        rehash(&mut t);
        let p = plan(&[b.clone()], &b, &t, "backend.json").unwrap();
        assert!(
            p.tools.changed.is_empty(),
            "ce champ ne prétend décrire que les attachements"
        );
        assert!(p.files.contains(&FileChange {
            path: "echo.mmd".into(),
            kind: ChangeKind::Changed
        }));
    }
    #[test]
    fn a_valid_candidate_ignores_an_inaccessible_builtin_cache() {
        const CHILD: &str = "RAG3WEAVER_TEST_M8_BLOCKED_CACHE";
        const NAME: &str =
            "template_plan::tests::a_valid_candidate_ignores_an_inaccessible_builtin_cache";
        if std::env::var_os(CHILD).is_some() {
            let cache =
                PathBuf::from(std::env::var_os("RAG3WEAVER_BUILTIN_TEMPLATES_DIR").unwrap());
            let mut b = edit(&fixture(), |v| {
                v["tools"]["echo"]["graph"] = json!("builtin:tools/list.mmd")
            });
            b.builtin
                .insert("tools/list.mmd".into(), GRAPH.as_bytes().to_vec());
            rehash(&mut b);
            plan(&[b.clone()], &b, &b, "backend.json").unwrap();
            assert_eq!(std::fs::read(&cache).unwrap(), b"cache-interdit");
            return;
        }
        let d = tempfile::tempdir().unwrap();
        let cache = d.path().join("cache");
        std::fs::write(&cache, b"cache-interdit").unwrap();
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", NAME, "--nocapture"])
            .env(CHILD, "1")
            .env("RAG3WEAVER_BUILTIN_TEMPLATES_DIR", &cache)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
    }

    /// Un processus neuf évite qu'un cache builtin déjà chaud cache une extraction.
    #[test]
    fn a_rejected_candidate_does_not_extract_a_builtin_cache() {
        const CHILD: &str = "RAG3WEAVER_TEST_M8_COLD_CACHE";
        const NAME: &str =
            "template_plan::tests::a_rejected_candidate_does_not_extract_a_builtin_cache";
        if std::env::var_os(CHILD).is_some() {
            let cache =
                PathBuf::from(std::env::var_os("RAG3WEAVER_BUILTIN_TEMPLATES_DIR").unwrap());
            assert!(!cache.exists());
            let mut b = edit(&fixture(), |v| {
                v["tools"]["echo"]["graph"] = json!("builtin:tools/list.mmd")
            });
            // Contenu invalide fourni : le refus ne doit pas chercher un autre builtin.
            b.builtin
                .insert("tools/list.mmd".into(), b"invalide".to_vec());
            rehash(&mut b);
            assert!(plan(&[b.clone()], &b, &b, "backend.json").is_err());
            assert!(
                !cache.exists(),
                "le refus a extrait les gabarits dans {}",
                cache.display()
            );
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", NAME, "--nocapture"])
            .env(CHILD, "1")
            .env(
                "RAG3WEAVER_BUILTIN_TEMPLATES_DIR",
                dir.path().join("cache-vide"),
            )
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(!dir.path().join("cache-vide").exists());
    }
}
