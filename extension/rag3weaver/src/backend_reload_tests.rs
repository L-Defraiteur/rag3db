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
    events[\"EventSourceNode(topics='catalog', cursor='r', limit=1000)\"]
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
    let refused = crate::serve::serve(Arc::new(backend), "0.0.0.0:0", 1).unwrap_err();
    assert!(refused.contains("boucle locale"), "{refused}");
}
