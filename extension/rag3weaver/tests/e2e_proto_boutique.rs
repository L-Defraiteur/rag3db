//! Le backend jouet du proto « tout déclaratif » (`templates/proto/boutique`)
//! ouvert sur une vraie base : un formulaire écrit, une liste et une fiche se
//! lisent par leurs routes, et une vue modifiée en service change la page —
//! le début de la recette (« ajoute une colonne prix à la liste »), sans
//! compilation.
#![cfg(feature = "rag3db-native")]

use rag3weaver::backend::{Backend, PreparedBackend};
use rag3weaver::routes::{RouteRequest, RouteResponse};
use rag3weaver::Rag3dbConnection;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

fn copy(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let path = entry.unwrap().path();
        let target = to.join(path.file_name().unwrap());
        if path.is_dir() {
            copy(&path, &target);
        } else {
            std::fs::copy(&path, &target).unwrap();
        }
    }
}

/// Le jouet copié dans `dir`, l'extension vectorielle prise à la racine du
/// moteur (`RAG3DB_ROOT`).
fn shop(dir: &Path) -> PathBuf {
    let template = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/proto/boutique");
    copy(&template, dir);
    let path = dir.join("backend.json");
    let mut manifest: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let root = std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .display()
            .to_string()
    });
    manifest["vector_extension"] = json!(format!(
        "{root}/extension/vector/build/libvector.rag3db_extension"
    ));
    std::fs::write(&path, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
    path
}

fn open(manifest: &Path) -> Backend {
    let prepared = PreparedBackend::load(manifest).unwrap();
    assert!(!prepared.needs_embeddings());
    let database = prepared.path(&prepared.manifest.database);
    std::fs::create_dir_all(database.parent().unwrap()).unwrap();
    let conn =
        Rag3dbConnection::with_manifest_buffer_pool(&database, prepared.manifest.buffer_pool)
            .unwrap();
    prepared.open(Box::new(conn), None).unwrap()
}

fn request(method: &str, path: &str) -> RouteRequest {
    RouteRequest {
        method: method.into(),
        path: path.into(),
        ..Default::default()
    }
}

fn get(backend: &Backend, path: &str) -> RouteResponse {
    backend.route(&request("GET", path))
}

fn form(backend: &Backend, path: &str, fields: &[(&str, &str)]) -> RouteResponse {
    let mut r = request("POST", path);
    r.form = fields
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    backend.route(&r)
}

fn post_json(backend: &Backend, path: &str, body: Value) -> RouteResponse {
    let mut r = request("POST", path);
    r.body = Some(body);
    backend.route(&r)
}

#[test]
#[ignore = "e2e : ouvre une base rag3db (run_e2e.sh)"]
fn the_toy_shop_serves_its_routes_and_changes_under_a_reload() {
    let dir = tempfile::tempdir().unwrap();
    let manifest = shop(dir.path());
    let mut backend = open(&manifest);

    // Le formulaire écrit un produit (le prix arrive en texte, lu comme nombre).
    let saved = form(
        &backend,
        "/products",
        &[
            ("key", "chaise"),
            ("name", "Chaise"),
            ("price", "12.5"),
            ("category", "salon"),
        ],
    );
    assert_eq!(saved.status, 200, "{}", saved.body);
    assert!(saved.body.contains("Chaise"), "{}", saved.body);

    // La liste et la fiche le relisent.
    let list = get(&backend, "/products");
    assert_eq!(list.status, 200, "{}", list.body);
    assert!(list.body.contains("/products/chaise"), "{}", list.body);
    let card = get(&backend, "/products/chaise");
    assert!(
        card.body.contains("<h1>Chaise</h1>") && card.body.contains("12.5"),
        "{}",
        card.body
    );
    assert!(get(&backend, "/products/inconnu")
        .body
        .contains("Introuvable"));

    // Les routes sans vue parlent JSON.
    let category = post_json(
        &backend,
        "/categories",
        json!({"record": {"key": "salon", "name": "Salon"}}),
    );
    assert_eq!(category.status, 200, "{}", category.body);
    let order = post_json(
        &backend,
        "/orders",
        json!({"record": {"key": "c1", "product": "chaise", "quantity": 2}}),
    );
    assert_eq!(order.status, 200, "{}", order.body);
    let orders: Value = serde_json::from_str(&get(&backend, "/orders").body).unwrap();
    assert!(orders.to_string().contains("\"c1\""), "{orders}");

    // Un formulaire invalide n'écrit rien et dit pourquoi.
    let refused = form(
        &backend,
        "/products",
        &[("key", "table"), ("name", "Table"), ("price", "abc")],
    );
    assert_eq!(refused.status, 422, "{}", refused.body);
    assert!(get(&backend, "/products/table")
        .body
        .contains("Introuvable"));

    // La recette : une colonne de plus dans la vue, en service, sans compiler.
    let view = dir.path().join("views/products.html");
    let source = std::fs::read_to_string(&view).unwrap();
    std::fs::write(
        &view,
        source
            .replace("<th>Nom</th></tr>", "<th>Nom</th><th>Prix</th></tr>")
            .replace(
                "<td>{{ product.data.name }}</td></tr>",
                "<td>{{ product.data.name }}</td><td>{{ product.data.price }}</td></tr>",
            ),
    )
    .unwrap();
    assert!(!get(&backend, "/products").body.contains("<th>Prix</th>"));
    backend.reload().unwrap();
    let list = get(&backend, "/products");
    assert!(
        list.body.contains("<th>Prix</th>") && list.body.contains("<td>12.5</td>"),
        "{}",
        list.body
    );

    backend.shutdown().unwrap();
}
