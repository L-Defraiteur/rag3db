//! Les témoins du rechargement à chaud (lot 3 du proto « tout déclaratif ») :
//! un backend déclaré dans un dossier, monté sans base (une connexion qui ne
//! rend rien), dont on modifie les fichiers en service.
use super::*;
use crate::connection::{CallbackConnection, QueryResult};
use crate::embedder::HashEmbedder;

const MANIFEST: &str = r#"{
  "version": 1,
  "name": "toy",
  "database": "data/toy.rag3db",
  "embeddings": { "address": "127.0.0.1:1", "model": "toy", "dimensions": 8 },
  "vector_extension": "vector.rag3db_extension",
  "entities": {},
  "scripts": { "answer": "answer.rhai", "other": "other.rhai" },
  "tools": { "echo": { "graph": "echo.mmd" } }
}"#;

const GRAPH: &str = "%% tool: echo
%% description: answers with the declared version
%% param: value json! -- anything
%% result: answer.result
graph LR
    answer[\"RhaiNode(script_id=answer, value=$value)\"]
";

fn write(dir: &Path, name: &str, text: &str) {
    std::fs::write(dir.join(name), text).unwrap();
}

/// Un backend « toy » déclaré dans un dossier temporaire, monté sans base.
fn toy() -> (tempfile::TempDir, Backend) {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "backend.json", MANIFEST);
    write(dir.path(), "echo.mmd", GRAPH);
    write(dir.path(), "answer.rhai", "#{version: 1, got: input}");
    write(
        dir.path(),
        "other.rhai",
        "#{version: \"other\", got: input}",
    );
    let prepared = PreparedBackend::load(&dir.path().join("backend.json")).unwrap();
    let conn = CallbackConnection::new(|_, _| Ok(QueryResult::default()));
    let mut catalog = Catalog::new(
        Box::new(conn),
        Box::new(HashEmbedder::new(8)),
        CatalogConfig {
            allow_mock_embedder: true,
            embedding_dim: 8,
            ..Default::default()
        },
    );
    catalog.initialize().unwrap();
    let backend = Backend {
        decider: None,
        prepared: std::sync::RwLock::new(Arc::new(prepared)),
        version: std::sync::atomic::AtomicU64::new(0),
        catalog: Arc::new(Mutex::new(catalog)),
        #[cfg(feature = "code")]
        file_source: None,
        #[cfg(feature = "code")]
        garde: None,
        reactions: Mutex::new(None),
        cell_reactions: Mutex::new(std::collections::HashMap::new()),
        watches: Mutex::new(Vec::new()),
    };
    (dir, backend)
}

fn echo(backend: &Backend) -> Value {
    let response = backend.call("echo", json!({"value": {"n": 1}})).unwrap();
    response["result"].clone()
}

#[test]
fn a_changed_script_changes_the_output_without_restarting() {
    let (dir, backend) = toy();
    assert_eq!(echo(&backend)["version"], 1);
    write(dir.path(), "answer.rhai", "#{version: 2, got: input}");
    assert_eq!(
        echo(&backend)["version"],
        1,
        "nothing changes before the reload"
    );
    assert_eq!(backend.reload().unwrap().version, 1);
    assert_eq!(backend.declarations_version(), 1);
    assert_eq!(echo(&backend)["version"], 2);
}

#[test]
fn a_changed_graph_changes_the_output() {
    let (dir, backend) = toy();
    write(
        dir.path(),
        "echo.mmd",
        &GRAPH.replace("script_id=answer", "script_id=other"),
    );
    backend.reload().unwrap();
    assert_eq!(echo(&backend)["version"], "other");
}

#[test]
fn an_invalid_graph_is_refused_and_the_old_one_still_answers() {
    let (dir, backend) = toy();
    write(
        dir.path(),
        "echo.mmd",
        &GRAPH.replace("RhaiNode", "NoSuchNode"),
    );
    let refused = backend.reload().unwrap_err();
    assert!(refused.contains("NoSuchNode"), "{refused}");
    assert!(refused.contains("ancienne version"), "{refused}");
    assert_eq!(backend.declarations_version(), 0);
    assert_eq!(echo(&backend)["version"], 1);
}

#[test]
fn a_missing_script_is_refused_and_the_old_one_still_answers() {
    let (dir, backend) = toy();
    std::fs::remove_file(dir.path().join("answer.rhai")).unwrap();
    let refused = backend.reload().unwrap_err();
    assert!(
        refused.contains("answer.rhai") || refused.contains("No such file"),
        "{refused}"
    );
    assert_eq!(echo(&backend)["version"], 1);
}

#[test]
fn a_call_started_before_a_reload_ends_on_the_old_version() {
    let (dir, backend) = toy();
    // Ce qu'un appel prend à son départ.
    let started = backend.prepared();
    write(dir.path(), "answer.rhai", "#{version: 2, got: input}");
    backend.reload().unwrap();
    let old = backend
        .call_tool_on(&started, "echo", json!({"value": {}}))
        .unwrap();
    assert_eq!(old["result"]["version"], 1);
    assert_eq!(echo(&backend)["version"], 2);
}

#[test]
fn an_opening_change_is_refused_as_a_migration() {
    let (dir, backend) = toy();
    write(
        dir.path(),
        "backend.json",
        &MANIFEST.replace("data/toy.rag3db", "data/other.rag3db"),
    );
    let refused = backend.reload().unwrap_err();
    assert!(
        refused.contains("database") && refused.contains("migration"),
        "{refused}"
    );
    assert_eq!(backend.declarations_version(), 0);
    assert_eq!(echo(&backend)["version"], 1);
}

#[test]
fn two_reloads_count_two_versions() {
    let (dir, backend) = toy();
    write(dir.path(), "answer.rhai", "#{version: 2}");
    assert_eq!(backend.reload().unwrap().version, 1);
    write(dir.path(), "answer.rhai", "#{version: 3}");
    assert_eq!(backend.reload().unwrap().version, 2);
    assert_eq!(echo(&backend)["version"], 3);
}

#[test]
fn a_refused_check_touches_nothing() {
    let (dir, backend) = toy();
    write(
        dir.path(),
        "echo.mmd",
        &GRAPH.replace("RhaiNode", "NoSuchNode"),
    );
    assert!(backend.check_reload().is_err());
    assert_eq!(backend.declarations_version(), 0);
    assert_eq!(echo(&backend)["version"], 1);
}

#[test]
fn between_check_and_apply_the_old_version_still_answers() {
    let (dir, backend) = toy();
    write(dir.path(), "answer.rhai", "#{version: 2}");
    let pending = backend.check_reload().unwrap();
    assert_eq!(echo(&backend)["version"], 1, "checked, not yet in service");
    assert_eq!(backend.apply_reload(pending).version, 1);
    assert_eq!(echo(&backend)["version"], 2);
}

const REACTION: &str = "%% tool: watch
%% description: transitions on catalog events
%% on: catalog
%% policy: debounce 500
%% param: target string = \"Memory\" -- la cible
%% result: react.report
graph LR
    events[\"EventSourceNode(topics='catalog', cursor='watch', limit=1000)\"]
    react[\"ReactTransitionNode(target=$target)\"]
    events -->|events| react
";

#[test]
fn a_removed_reaction_is_named_for_the_host() {
    let (dir, backend) = toy();
    write(dir.path(), "watch.mmd", REACTION);
    write(
        dir.path(),
        "backend.json",
        &MANIFEST.replace(
            "\"tools\": {",
            "\"reactions\": { \"watch\": { \"graph\": \"watch.mmd\" } },\n  \"tools\": {",
        ),
    );
    let added = backend.reload().unwrap();
    assert!(added.removed_reactions.is_empty());
    write(dir.path(), "backend.json", MANIFEST);
    let pending = backend.check_reload().unwrap();
    assert_eq!(pending.removed_reactions(), ["watch".to_string()]);
    assert_eq!(
        backend.apply_reload(pending).removed_reactions,
        ["watch".to_string()]
    );
}

// ─── Le dossier nodes/ (lot 4a) ─────────────────────────────────────────────

const DOUBLE_DECL: &str = r#"{
  "name": "Double",
  "description": "doubles n",
  "script": "double.ts",
  "inputs": { "value": { "schema": { "type": "object", "required": ["n"] } } },
  "outputs": { "doubled": { "schema": { "type": "object", "required": ["n"] } } }
}"#;

const DOUBLE_TS: &str = "function run({ inputs }: { inputs: { value: { n: number } } }) {
  return { doubled: { n: inputs.value.n * 2 } };
}";

const DOUBLING_GRAPH: &str = "%% tool: echo
%% description: doubles through a declared node
%% param: value json! -- anything
%% result: dbl.doubled
graph LR
    src[\"RhaiNode(script_id=answer, value=$value)\"]
    dbl[\"Double\"]
    src -->|result:value| dbl
";

/// Le jouet, avec un nœud déclaré dans `nodes/` et un outil qui s'en sert.
fn toy_with_nodes() -> (tempfile::TempDir, Backend) {
    let (dir, backend) = toy();
    std::fs::create_dir(dir.path().join("nodes")).unwrap();
    write(&dir.path().join("nodes"), "double.node.json", DOUBLE_DECL);
    write(&dir.path().join("nodes"), "double.ts", DOUBLE_TS);
    write(dir.path(), "echo.mmd", DOUBLING_GRAPH);
    write(dir.path(), "answer.rhai", "input");
    backend.reload().unwrap();
    (dir, backend)
}

fn doubled(backend: &Backend, n: i64) -> Value {
    let response = backend.call("echo", json!({"value": {"n": n}})).unwrap();
    response["result"].clone()
}

#[test]
fn a_node_declared_in_the_nodes_folder_serves_a_tool() {
    let (dir, backend) = toy_with_nodes();
    assert_eq!(doubled(&backend, 21), json!({"n": 42}));
    // Et un chargement à froid le trouve aussi.
    PreparedBackend::load(&dir.path().join("backend.json")).unwrap();
}

#[test]
fn an_edited_declared_node_is_reloaded() {
    let (dir, backend) = toy_with_nodes();
    write(
        &dir.path().join("nodes"),
        "double.ts",
        &DOUBLE_TS.replace("* 2", "* 3"),
    );
    assert_eq!(
        doubled(&backend, 21),
        json!({"n": 42}),
        "not before the reload"
    );
    backend.reload().unwrap();
    assert_eq!(doubled(&backend, 21), json!({"n": 63}));
}

#[test]
fn a_declared_node_named_like_a_provided_one_is_refused() {
    let (dir, backend) = toy_with_nodes();
    write(
        &dir.path().join("nodes"),
        "double.node.json",
        &DOUBLE_DECL.replace("\"Double\"", "\"RhaiNode\""),
    );
    let refused = backend.reload().unwrap_err();
    assert!(
        refused.contains("RhaiNode") && refused.contains("nœud fourni"),
        "{refused}"
    );
    assert_eq!(doubled(&backend, 1), json!({"n": 2}));
}

#[test]
fn a_bad_declaration_refuses_the_reload_and_names_its_file() {
    let (dir, backend) = toy_with_nodes();
    write(
        &dir.path().join("nodes"),
        "bad.node.json",
        "{ \"name\": \"Bad\" }",
    );
    let refused = backend.reload().unwrap_err();
    assert!(refused.contains("bad.node.json"), "{refused}");
    assert_eq!(doubled(&backend, 1), json!({"n": 2}));
}

// ─── Les routes et les vues (lot 4b) ────────────────────────────────────────

const GREET_GRAPH: &str = "%% tool: greet
%% description: greets someone
%% param: name string! -- who
%% result: hello.greeting
graph LR
    hello[\"Greet(name=$name)\"]
";

const GREET_DECL: &str = r#"{
  "name": "Greet",
  "script": "greet.ts",
  "outputs": { "greeting": { "schema": { "type": "object", "required": ["hello"] } } },
  "config": { "name": { "schema": { "type": "string" }, "required": true } }
}"#;

const GREET_TS: &str = "function run({ config }: { config: { name: string } }) {
  return { greeting: { hello: config.name } };
}";

const ROUTES: &str = r#""routes": {
    "GET /hello/{name}": { "tool": "greet", "view": "hello.html" },
    "GET /echo": { "tool": "echo" }
  },
  "tools": { "greet": { "graph": "greet.mmd" }, "#;

/// Le jouet, avec un outil `greet`, deux routes et une vue.
fn toy_with_routes() -> (tempfile::TempDir, Backend) {
    let (dir, backend) = toy();
    std::fs::create_dir(dir.path().join("nodes")).unwrap();
    write(&dir.path().join("nodes"), "greet.node.json", GREET_DECL);
    write(&dir.path().join("nodes"), "greet.ts", GREET_TS);
    write(dir.path(), "greet.mmd", GREET_GRAPH);
    std::fs::create_dir(dir.path().join("views")).unwrap();
    write(
        &dir.path().join("views"),
        "hello.html",
        "<h1>Bonjour {{ result.hello }}</h1>",
    );
    write(dir.path(), "backend.json", &routes_manifest(ROUTES));
    backend.reload().unwrap();
    (dir, backend)
}

fn routes_manifest(routes: &str) -> String {
    MANIFEST.replace("\"tools\": { ", routes)
}

fn get(backend: &Backend, path: &str, query: &[(&str, &str)]) -> crate::routes::RouteResponse {
    backend.route(&crate::routes::RouteRequest {
        method: "GET".into(),
        path: path.into(),
        query: query
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        body: None,
        form: vec![],
    })
}

#[test]
fn a_declared_route_renders_its_view_from_the_same_call() {
    let (_dir, backend) = toy_with_routes();
    let page = get(&backend, "/hello/Lucie", &[]);
    assert_eq!(page.status, 200, "{}", page.body);
    assert!(page.content_type.starts_with("text/html"));
    assert_eq!(page.body, "<h1>Bonjour Lucie</h1>");
    assert_eq!(page.result, Some(json!({"hello": "Lucie"})));
}

#[test]
fn a_view_escapes_what_it_receives() {
    let (_dir, backend) = toy_with_routes();
    let page = get(&backend, "/hello/<b>", &[]);
    assert!(page.body.contains("&lt;b&gt;"), "{}", page.body);
}

#[test]
fn a_route_without_a_view_answers_json() {
    let (_dir, backend) = toy_with_routes();
    let answer = get(&backend, "/echo", &[("value", "{\"n\": 1}")]);
    assert_eq!(answer.status, 200, "{}", answer.body);
    assert_eq!(answer.content_type, "application/json");
    let body: Value = serde_json::from_str(&answer.body).unwrap();
    assert_eq!(body, json!({"version": 1, "got": {"n": 1}}));
}

#[test]
fn an_unknown_address_or_method_is_said() {
    let (_dir, backend) = toy_with_routes();
    assert_eq!(get(&backend, "/nowhere", &[]).status, 404);
    let wrong = backend.route(&crate::routes::RouteRequest {
        method: "POST".into(),
        path: "/hello/Lucie".into(),
        ..Default::default()
    });
    assert_eq!(wrong.status, 405);
    assert!(wrong.body.contains("GET"), "{}", wrong.body);
}

#[test]
fn a_route_to_an_unknown_tool_is_refused_at_load() {
    let (dir, backend) = toy_with_routes();
    write(
        dir.path(),
        "backend.json",
        &routes_manifest(&ROUTES.replace("\"tool\": \"greet\"", "\"tool\": \"nope\"")),
    );
    let refused = backend.reload().unwrap_err();
    assert!(
        refused.contains("GET /hello/{name}") && refused.contains("nope"),
        "{refused}"
    );
    assert_eq!(get(&backend, "/hello/Lucie", &[]).status, 200);
}

#[test]
fn a_missing_or_broken_view_is_refused_at_load() {
    let (dir, backend) = toy_with_routes();
    write(
        dir.path(),
        "backend.json",
        &routes_manifest(&ROUTES.replace("hello.html", "missing.html")),
    );
    let refused = backend.reload().unwrap_err();
    assert!(refused.contains("missing.html"), "{refused}");
    write(dir.path(), "backend.json", &routes_manifest(ROUTES));
    write(&dir.path().join("views"), "hello.html", "<h1>{% if %}</h1>");
    let refused = backend.reload().unwrap_err();
    assert!(refused.contains("views/hello.html"), "{refused}");
    assert_eq!(
        get(&backend, "/hello/Lucie", &[]).body,
        "<h1>Bonjour Lucie</h1>"
    );
}

#[test]
fn an_edited_view_changes_the_page_after_a_reload() {
    let (dir, backend) = toy_with_routes();
    write(
        &dir.path().join("views"),
        "hello.html",
        "<p>Salut {{ result.hello }}</p>",
    );
    assert_eq!(
        get(&backend, "/hello/Lucie", &[]).body,
        "<h1>Bonjour Lucie</h1>"
    );
    backend.reload().unwrap();
    assert_eq!(
        get(&backend, "/hello/Lucie", &[]).body,
        "<p>Salut Lucie</p>"
    );
}

// ─── Un formulaire est lu selon le type des paramètres (lot 4d) ─────────────

const TIMES_DECL: &str = r#"{
  "name": "Times",
  "script": "times.ts",
  "outputs": { "value": { "schema": { "type": "object", "required": ["n"] } } },
  "config": { "n": { "schema": { "type": "integer" }, "required": true } }
}"#;

const TIMES_GRAPH: &str = "%% tool: times
%% description: doubles an integer
%% param: n int! -- the integer
%% result: times.value
graph LR
    times[\"Times(n=$n)\"]
";

#[test]
fn a_form_field_is_read_with_the_type_of_its_parameter() {
    let (dir, backend) = toy_with_routes();
    write(&dir.path().join("nodes"), "times.node.json", TIMES_DECL);
    write(
        &dir.path().join("nodes"),
        "times.ts",
        "function run({ config }: { config: { n: number } }) { return { value: { n: config.n * 2 } }; }",
    );
    write(dir.path(), "times.mmd", TIMES_GRAPH);
    write(
        dir.path(),
        "backend.json",
        &routes_manifest(&ROUTES.replace(
            "\"GET /echo\": { \"tool\": \"echo\" }",
            "\"GET /echo\": { \"tool\": \"echo\" },\n    \"POST /times\": { \"tool\": \"times\" }",
        ).replace("\"greet\": { \"graph\": \"greet.mmd\" }, ", "\"greet\": { \"graph\": \"greet.mmd\" }, \"times\": { \"graph\": \"times.mmd\" }, ")),
    );
    backend.reload().unwrap();
    let answer = backend.route(&crate::routes::RouteRequest {
        method: "POST".into(),
        path: "/times".into(),
        form: vec![("n".into(), "21".into())],
        ..Default::default()
    });
    assert_eq!(answer.status, 200, "{}", answer.body);
    assert_eq!(answer.result, Some(json!({"n": 42})));
}

// ─── Le jouet livré (lot 4d) ────────────────────────────────────────────────

/// `templates/proto/boutique` se charge : manifeste, schémas, graphes, nœuds
/// scriptés, routes et vues — sans base.
#[test]
fn the_shipped_toy_shop_loads() {
    let manifest =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/proto/boutique/backend.json");
    let prepared = PreparedBackend::load(&manifest).unwrap();
    assert!(!prepared.needs_embeddings());
    let tools = prepared.describe()["tools"].as_array().unwrap().len();
    assert_eq!(tools, 7);
}

// ─── declare : l'agent écrit les déclarations (lot 5) ───────────────────────

fn git_in(dir: &Path, args: &[&str]) -> String {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?} : {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// Le jouet des routes dans un dépôt git à lui, identité locale, tout commité.
fn toy_in_a_repository() -> (tempfile::TempDir, Backend) {
    let (dir, backend) = toy_with_routes();
    git_in(dir.path(), &["init", "-q"]);
    git_in(dir.path(), &["config", "user.name", "Personne Test"]);
    git_in(
        dir.path(),
        &["config", "user.email", "personne@example.org"],
    );
    git_in(dir.path(), &["add", "-A"]);
    git_in(dir.path(), &["commit", "-q", "-m", "le jouet"]);
    (dir, backend)
}

#[test]
fn declare_refuses_what_is_not_a_declaration_of_this_backend() {
    let (_dir, backend) = toy_in_a_repository();
    for path in [
        "../outside.mmd",
        "/etc/passwd",
        ".git/config",
        "src/main.rs",
        "data/toy.json",
        "",
    ] {
        let refused = backend.declare(path, "x", "essai").unwrap_err();
        assert!(
            refused.contains("n'est pas une déclaration"),
            "{path} : {refused}"
        );
    }
    let refused = backend
        .declare("views/hello.html", "<p>x</p>", "  ")
        .unwrap_err();
    assert!(refused.contains("raison"), "{refused}");
}

#[test]
fn an_invalid_declaration_is_refused_with_its_line_and_nothing_is_written() {
    let (dir, backend) = toy_in_a_repository();
    let head = git_in(dir.path(), &["rev-parse", "HEAD"]);
    let refused = backend
        .declare("views/hello.html", "<h1>\n{% if %}\n</h1>", "casser la vue")
        .unwrap_err();
    assert!(
        refused.contains("views/hello.html") && refused.contains("ligne 2"),
        "{refused}"
    );
    assert_eq!(
        std::fs::read_to_string(dir.path().join("views/hello.html")).unwrap(),
        "<h1>Bonjour {{ result.hello }}</h1>"
    );
    assert_eq!(
        get(&backend, "/hello/Lucie", &[]).body,
        "<h1>Bonjour Lucie</h1>"
    );
    assert_eq!(
        git_in(dir.path(), &["rev-parse", "HEAD"]),
        head,
        "no commit"
    );
    assert_eq!(
        backend.declarations_version(),
        1,
        "the toy's own reload only"
    );
}

#[test]
fn a_valid_declaration_is_in_service_and_committed_with_its_reason() {
    let (dir, backend) = toy_in_a_repository();
    let declared = backend
        .declare(
            "views/hello.html",
            "<h1>Bonjour <em>{{ result.hello }}</em></h1>",
            "Mettre le prénom en italique",
        )
        .unwrap();
    assert_eq!(
        get(&backend, "/hello/Lucie", &[]).body,
        "<h1>Bonjour <em>Lucie</em></h1>"
    );
    let commit = declared.commit.expect("a commit");
    assert_eq!(git_in(dir.path(), &["rev-parse", "HEAD"]), commit);
    let message = git_in(dir.path(), &["log", "-1", "--format=%B"]);
    assert!(
        message.starts_with("Mettre le prénom en italique"),
        "{message}"
    );
    assert!(message.contains("declare: views/hello.html"), "{message}");
    assert_eq!(
        git_in(dir.path(), &["log", "-1", "--format=%an <%ae>"]),
        "Personne Test <personne@example.org>"
    );
    assert_eq!(
        git_in(dir.path(), &["show", "--name-only", "--format=", "HEAD"]),
        "views/hello.html"
    );
    // Le même contenu une seconde fois : rien à commiter.
    let again = backend
        .declare(
            "views/hello.html",
            "<h1>Bonjour <em>{{ result.hello }}</em></h1>",
            "encore",
        )
        .unwrap();
    assert_eq!(again.commit, None);
}

#[test]
fn a_declaration_commits_its_path_only() {
    let (dir, backend) = toy_in_a_repository();
    write(dir.path(), "notes.txt", "à moi");
    git_in(dir.path(), &["add", "notes.txt"]);
    git_in(dir.path(), &["commit", "-q", "-m", "des notes"]);
    write(dir.path(), "notes.txt", "modifiées, pas pour declare");
    write(dir.path(), "brouillon.md", "pas suivi");
    backend
        .declare(
            "views/hello.html",
            "<p>{{ result.hello }}</p>",
            "Une vue plus sobre",
        )
        .unwrap();
    assert_eq!(
        git_in(dir.path(), &["show", "--name-only", "--format=", "HEAD"]),
        "views/hello.html"
    );
    let status = git_in(dir.path(), &["status", "--porcelain"]);
    assert!(
        status.contains("M notes.txt") && status.contains("?? brouillon.md"),
        "{status}"
    );
}

#[test]
fn a_new_declaration_in_a_new_folder_is_written_and_listed() {
    let (dir, backend) = toy_in_a_repository();
    assert!(backend
        .list_declarations()
        .contains(&"views/hello.html".to_string()));
    backend
        .declare(
            "views/parts/footer.html",
            "<footer>boutique</footer>",
            "Un pied de page",
        )
        .unwrap();
    assert!(dir.path().join("views/parts/footer.html").exists());
    assert!(backend
        .list_declarations()
        .contains(&"views/parts/footer.html".to_string()));
    assert_eq!(
        backend.read_declaration("views/parts/footer.html").unwrap(),
        "<footer>boutique</footer>"
    );
}

/// Un modèle enregistré : il pose une vue invalide, lit le refus, corrige.
#[test]
fn an_agent_corrects_a_refused_declaration_on_the_next_turn() {
    use crate::agent::{Agent, AgentLimits, ToolBox};
    use crate::llm::{CallbackLlm, Llm, MockLlm, StringSink, ToolCall, Turn};
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct Nothing;
    impl ToolBox for Nothing {
        fn call(&self, call: &ToolCall) -> Turn {
            Turn::tool_result(&call.id, &call.name, "{\"ok\":false}")
        }
    }
    let (dir, backend) = toy_in_a_repository();
    let broken =
        json!({"path": "views/hello.html", "content": "<h1>{% if %}</h1>", "reason": "Titre"})
            .to_string();
    let fixed = json!({"path": "views/hello.html", "content": "<h2>Salut {{ result.hello }}</h2>", "reason": "Un titre plus discret"}).to_string();
    let calls = AtomicUsize::new(0);
    let llm = CallbackLlm::new("declare-test", 8192, move |turns, opts, sink| {
        let model = match calls.fetch_add(1, Ordering::SeqCst) {
            0 => MockLlm::new("").with_tool_calls(vec![("declare", broken.as_str())]),
            1 => MockLlm::new("").with_tool_calls(vec![("declare", fixed.as_str())]),
            _ => MockLlm::new("C'est fait."),
        };
        model.generate(turns, opts, sink)
    });
    let tools = crate::declare::DeclarationTools {
        backend: &backend,
        inner: &Nothing,
    };
    assert!(tools.tool_defs().iter().any(|d| d.name == "declare"));
    let mut turns = vec![Turn::user("mets un titre plus discret")];
    Agent::new(&llm, &tools)
        .with_limits(AgentLimits {
            max_iterations: 5,
            ..Default::default()
        })
        .run(&mut turns, &mut StringSink::default())
        .unwrap();
    let refusal = turns
        .iter()
        .find(|t| t.content.contains("déclaration refusée"))
        .expect("the refusal came back to the model");
    assert!(
        refusal.content.contains("views/hello.html"),
        "{}",
        refusal.content
    );
    assert_eq!(
        get(&backend, "/hello/Lucie", &[]).body,
        "<h2>Salut Lucie</h2>"
    );
    assert!(git_in(dir.path(), &["log", "-1", "--format=%B"]).starts_with("Un titre plus discret"));
}

// ─── La page vivante : les outils en processus, le graphe d'un outil (lot 6) ─

#[test]
fn a_backend_gives_its_tools_to_an_agent_in_process() {
    use crate::agent::ToolBox;
    use crate::llm::ToolCall;
    let (_dir, backend) = toy_with_routes();
    let tools = crate::backend_tools::BackendTools { backend: &backend };
    let names: Vec<String> = tools.tool_defs().into_iter().map(|d| d.name).collect();
    assert!(
        names.contains(&"greet".to_string()) && names.contains(&"echo".to_string()),
        "{names:?}"
    );
    let turn = tools.call(&ToolCall::new(
        "c1",
        "greet",
        json!({"name": "Lucie"}).to_string(),
    ));
    assert!(turn.content.contains("Lucie"), "{}", turn.content);
    let turn = tools.call(&ToolCall::new("c2", "nope", "{}"));
    assert!(turn.content.contains("error"), "{}", turn.content);
}

#[test]
fn the_graph_of_a_tool_is_readable_for_drawing() {
    let (_dir, backend) = toy_with_routes();
    let prepared = backend.prepared();
    let graph = prepared.tool_graph("greet").expect("the greet tool");
    let drawn = crate::dataflow::to_mermaid(graph.template());
    assert!(drawn.contains("Greet"), "{drawn}");
    assert!(prepared.tool_graph("nope").is_none());
}

// ─── serve : les routes sur HTTP (lot 4c) ───────────────────────────────────

#[cfg(feature = "daemon")]
fn http(port: u16, line: &str) -> String {
    use std::io::{Read, Write};
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    write!(
        stream,
        "{line}\r\nHost: localhost\r\nConnection: close\r\nContent-Length: 0\r\n\r\n"
    )
    .unwrap();
    let mut answer = String::new();
    stream.read_to_string(&mut answer).unwrap();
    answer
}

#[cfg(feature = "daemon")]
#[test]
fn serve_answers_a_route_over_http_and_hands_the_backend_back_on_stop() {
    let (_dir, backend) = toy_with_routes();
    let backend = Arc::new(backend);
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let port = server.server_addr().to_ip().unwrap().port();
    let serving = {
        let backend = backend.clone();
        std::thread::spawn(move || crate::serve::serve_on(server, backend, 3))
    };
    let page = http(port, "GET /hello/Lucie%20D HTTP/1.1");
    assert!(page.starts_with("HTTP/1.1 200"), "{page}");
    assert!(
        page.to_lowercase().contains("content-type: text/html"),
        "{page}"
    );
    assert!(page.ends_with("<h1>Bonjour Lucie D</h1>"), "{page}");
    assert!(http(port, "GET /nowhere HTTP/1.1").starts_with("HTTP/1.1 404"));
    assert!(http(port, "POST /arret HTTP/1.1").starts_with("HTTP/1.1 200"));
    serving.join().unwrap();
    // Plus aucun fil ne le tient : l'appelant peut fermer la base proprement.
    assert!(Arc::try_unwrap(backend).is_ok());
}

#[cfg(feature = "daemon")]
#[test]
fn serve_refuses_an_address_outside_the_local_loop() {
    let (_dir, backend) = toy_with_routes();
    let refused = crate::serve::serve(Arc::new(backend), "0.0.0.0:0", 1, None).unwrap_err();
    assert!(refused.contains("boucle locale"), "{refused}");
}

// ─── La page vivante (lot 6) ────────────────────────────────────────────────

/// Ce que le flux pousse, gardé pour le relire.
#[cfg(feature = "daemon")]
#[derive(Clone, Default)]
struct Captured(Arc<Mutex<Vec<u8>>>);

#[cfg(feature = "daemon")]
impl std::io::Write for Captured {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(feature = "daemon")]
impl Captured {
    fn text(&self) -> String {
        String::from_utf8_lossy(&self.0.lock().unwrap()).to_string()
    }
}

#[cfg(feature = "daemon")]
fn atelier_without_model(backend: Arc<Backend>) -> Arc<crate::atelier::Atelier> {
    crate::atelier::Atelier::new(backend, Err("pas de models.llm".into())).unwrap()
}

#[cfg(feature = "daemon")]
#[test]
fn the_atelier_page_is_served_and_says_why_the_chat_is_off() {
    let (dir, backend) = toy_with_routes();
    let backend = Arc::new(backend);
    let atelier = atelier_without_model(backend.clone());
    assert!(dir.path().join(".atelier/.gitignore").exists());
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let port = server.server_addr().to_ip().unwrap().port();
    let serving = {
        let (backend, atelier) = (backend.clone(), atelier.clone());
        std::thread::spawn(move || crate::serve::serve_with(server, backend, 2, Some(atelier)))
    };
    let page = http(port, "GET /_atelier HTTP/1.1");
    assert!(
        page.starts_with("HTTP/1.1 200") && page.contains("EventSource"),
        "{page}"
    );
    let state = http(port, "GET /_atelier/state HTTP/1.1");
    let body: Value = serde_json::from_str(state.split("\r\n\r\n").nth(1).unwrap()).unwrap();
    assert_eq!(body["chat"]["on"], false);
    assert!(
        body["chat"]["why"].as_str().unwrap().contains("models.llm"),
        "{body}"
    );
    assert!(body["routes"].to_string().contains("GET /echo"), "{body}");
    let graph = http(
        port,
        "GET /_atelier/graph?tool=greet&highlight=Greet HTTP/1.1",
    );
    assert!(
        graph.contains("Greet") && graph.contains("style hello"),
        "{graph}"
    );
    // Les routes déclarées répondent toujours à côté.
    assert!(http(port, "GET /hello/Lucie HTTP/1.1").ends_with("<h1>Bonjour Lucie</h1>"));
    // Le chat éteint dit pourquoi.
    assert!(atelier.chat("bonjour").unwrap_err().contains("éteint"));
    assert!(http(port, "POST /arret HTTP/1.1").starts_with("HTTP/1.1 200"));
    serving.join().unwrap();
}

#[cfg(feature = "daemon")]
#[test]
fn the_event_stream_reaches_a_subscriber_over_http() {
    use std::io::{BufRead, BufReader};
    let (_dir, backend) = toy_with_routes();
    let backend = Arc::new(backend);
    let atelier = atelier_without_model(backend.clone());
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let port = server.server_addr().to_ip().unwrap().port();
    let serving = {
        let (backend, atelier) = (backend.clone(), atelier.clone());
        std::thread::spawn(move || crate::serve::serve_with(server, backend, 2, Some(atelier)))
    };
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(10)))
        .unwrap();
    use std::io::Write;
    write!(
        stream,
        "GET /_atelier/events HTTP/1.1\r\nHost: localhost\r\n\r\n"
    )
    .unwrap();
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    while !line.starts_with(": atelier") {
        line.clear();
        reader.read_line(&mut line).unwrap();
    }
    assert!(atelier.hub().subscribers() >= 1);
    atelier.hub().publish(&json!({"event": "ping", "n": 1}));
    let mut seen = String::new();
    while !seen.contains("\"ping\"") {
        line.clear();
        reader.read_line(&mut line).unwrap();
        seen.push_str(&line);
    }
    assert!(seen.contains("event: ping"), "{seen}");
    drop(reader);
    assert!(http(port, "POST /arret HTTP/1.1").starts_with("HTTP/1.1 200"));
    serving.join().unwrap();
}

#[cfg(feature = "daemon")]
#[test]
fn the_watcher_reloads_a_view_edited_by_hand() {
    let (dir, backend) = toy_with_routes();
    let backend = Arc::new(backend);
    let atelier = atelier_without_model(backend.clone());
    let captured = Captured::default();
    atelier.hub().subscribe(Box::new(captured.clone()));
    atelier.look();
    assert!(!captured.text().contains("reloaded"), "nothing changed yet");
    write(
        &dir.path().join("views"),
        "hello.html",
        "<h3>Coucou {{ result.hello }}</h3>",
    );
    atelier.look();
    assert!(
        captured.text().contains("event: reloaded"),
        "{}",
        captured.text()
    );
    assert!(
        captured.text().contains("views/hello.html"),
        "{}",
        captured.text()
    );
    assert_eq!(
        get(&backend, "/hello/Lucie", &[]).body,
        "<h3>Coucou Lucie</h3>"
    );
    // Une vue cassée à la main : refusée, dite, l'ancienne répond.
    write(&dir.path().join("views"), "hello.html", "<h3>{% if %}</h3>");
    atelier.look();
    assert!(
        captured.text().contains("event: refused"),
        "{}",
        captured.text()
    );
    assert_eq!(
        get(&backend, "/hello/Lucie", &[]).body,
        "<h3>Coucou Lucie</h3>"
    );
}

/// Un tour de chat avec un modèle enregistré : il déclare une vue ; le flux
/// porte l'outil, la déclaration et la réponse ; la montre ne recharge pas
/// une seconde fois ; la session est gardée.
#[cfg(feature = "daemon")]
#[test]
fn a_chat_turn_declares_and_pushes_its_events() {
    use crate::llm::{CallbackLlm, Llm, MockLlm};
    use std::sync::atomic::{AtomicUsize, Ordering};
    let (dir, backend) = toy_in_a_repository();
    let backend = Arc::new(backend);
    let view = json!({"path": "views/hello.html", "content": "<h1>Hello {{ result.hello }}</h1>", "reason": "En anglais"}).to_string();
    let calls = AtomicUsize::new(0);
    let llm = CallbackLlm::new("atelier-test", 8192, move |turns, opts, sink| {
        let model = match calls.fetch_add(1, Ordering::SeqCst) {
            0 => MockLlm::new("").with_tool_calls(vec![("declare", view.as_str())]),
            _ => MockLlm::new("Fait."),
        };
        model.generate(turns, opts, sink)
    });
    let atelier = crate::atelier::Atelier::new(backend.clone(), Ok(Arc::new(llm))).unwrap();
    let captured = Captured::default();
    atelier.hub().subscribe(Box::new(captured.clone()));
    assert_eq!(atelier.chat("passe la page en anglais").unwrap(), "Fait.");
    let events = captured.text();
    for kind in ["event: tool_start", "event: declared", "event: answer"] {
        assert!(events.contains(kind), "{kind} : {events}");
    }
    assert_eq!(
        get(&backend, "/hello/Lucie", &[]).body,
        "<h1>Hello Lucie</h1>"
    );
    let version = backend.declarations_version();
    atelier.look();
    assert_eq!(
        backend.declarations_version(),
        version,
        "declare already reloaded"
    );
    assert!(git_in(dir.path(), &["log", "-1", "--format=%B"]).starts_with("En anglais"));
    assert!(dir.path().join(".atelier/session.json").exists());
    // La session n'est pas commitée.
    assert!(!git_in(dir.path(), &["status", "--porcelain"]).contains(".atelier"));
}

// ─── declare et la montre remontent les réactions (après la porte de la mémoire) ─

/// Le jouet des routes, dans un dépôt, avec une réaction « watch » montée.
fn toy_with_a_reaction() -> (tempfile::TempDir, Backend) {
    let (dir, backend) = toy_in_a_repository();
    write(dir.path(), "watch.mmd", REACTION);
    let manifest = std::fs::read_to_string(dir.path().join("backend.json")).unwrap();
    write(
        dir.path(),
        "backend.json",
        &manifest.replace(
            "\"tools\": {",
            "\"reactions\": { \"watch\": { \"graph\": \"watch.mmd\" } },\n  \"tools\": {",
        ),
    );
    backend.reload().unwrap();
    assert_eq!(backend.mount_reactions().unwrap(), ["watch".to_string()]);
    git_in(dir.path(), &["add", "-A"]);
    git_in(dir.path(), &["commit", "-q", "-m", "une réaction"]);
    (dir, backend)
}

#[test]
fn a_declaration_stops_and_remounts_the_reactions() {
    let (_dir, backend) = toy_with_a_reaction();
    let declared = backend
        .declare(
            "views/hello.html",
            "<p>{{ result.hello }}</p>",
            "Une vue plus sobre",
        )
        .unwrap();
    assert_eq!(declared.mounted_reactions, ["watch".to_string()]);
    assert!(declared.dropped_cursors.is_empty());
    backend.stop_reactions();
}

#[test]
fn a_declaration_that_removes_a_reaction_forgets_its_cursor() {
    let (dir, backend) = toy_with_a_reaction();
    let manifest = std::fs::read_to_string(dir.path().join("backend.json")).unwrap();
    let without = manifest.replace(
        "\"reactions\": { \"watch\": { \"graph\": \"watch.mmd\" } },\n  ",
        "",
    );
    let declared = backend
        .declare("backend.json", &without, "Retirer la réaction")
        .unwrap();
    assert_eq!(declared.removed_reactions, ["watch".to_string()]);
    assert_eq!(declared.dropped_cursors, ["catalog@watch".to_string()]);
    assert!(declared.mounted_reactions.is_empty());
    // Plus rien en service : un montage neuf ne trouve rien à monter.
    assert!(backend.mount_reactions().unwrap().is_empty());
}

#[cfg(feature = "daemon")]
#[test]
fn the_watcher_remounts_the_reactions_and_says_the_forgotten_cursors() {
    let (dir, backend) = toy_with_a_reaction();
    let backend = Arc::new(backend);
    let atelier = atelier_without_model(backend.clone());
    let captured = Captured::default();
    atelier.hub().subscribe(Box::new(captured.clone()));
    let manifest = std::fs::read_to_string(dir.path().join("backend.json")).unwrap();
    write(
        dir.path(),
        "backend.json",
        &manifest.replace(
            "\"reactions\": { \"watch\": { \"graph\": \"watch.mmd\" } },\n  ",
            "",
        ),
    );
    atelier.look();
    let events = captured.text();
    assert!(events.contains("event: reloaded"), "{events}");
    assert!(events.contains("catalog@watch"), "{events}");
}
