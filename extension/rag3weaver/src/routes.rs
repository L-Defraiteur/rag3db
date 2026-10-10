//! Les routes d'un backend déclaré : **une adresse mène à un contrôleur**,
//! et le contrôleur est un outil déclaré — le même graphe que l'agent appelle
//! par son nom, qu'un événement déclenche par une réaction, et qu'une route
//! atteint par une adresse. La fin du graphe va à **une vue** : un gabarit du
//! dossier `views/` du backend, qui reçoit le résultat typé.
//!
//! Le contrat de données n'est pas réécrit : ce sont les paramètres et les
//! validateurs de l'outil qui le tiennent. Une route ne fait que traduire une
//! requête (chemin, paramètres, corps) en arguments, puis un résultat en page.
//!
//! Tout se vérifie au chargement — donc à chaque rechargement : une route vers
//! un outil inconnu, une vue absente ou qui ne se compile pas, deux routes
//! ambiguës sont refusées en les nommant.
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::path::Path;

/// Une route telle qu'écrite au manifeste, sous sa clé `"GET /products/{key}"`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RouteDecl {
    /// L'outil déclaré qui sert de contrôleur.
    pub tool: String,
    /// Le gabarit du dossier `views/` qui rend le résultat ; absent, la route
    /// rend le résultat en JSON.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
enum Segment {
    Literal(String),
    Param(String),
}

#[derive(Debug, Clone)]
struct Route {
    key: String,
    method: String,
    segments: Vec<Segment>,
    tool: String,
    view: Option<String>,
}

/// Les routes vérifiées d'un backend, et les sources de ses vues.
#[derive(Debug, Clone, Default)]
pub struct Routes {
    routes: Vec<Route>,
    views: BTreeMap<String, String>,
}

const METHODS: [&str; 5] = ["GET", "POST", "PUT", "PATCH", "DELETE"];

/// Une requête, déjà décodée par le transport.
#[derive(Debug, Clone, Default)]
pub struct RouteRequest {
    pub method: String,
    pub path: String,
    pub query: Vec<(String, String)>,
    /// Un corps JSON (objet), s'il y en a un.
    pub body: Option<Value>,
}

/// Ce qu'une route rend : la page (ou le JSON) **et** le résultat du graphe,
/// sortis du même appel.
#[derive(Debug, Clone)]
pub struct RouteResponse {
    pub status: u16,
    pub content_type: &'static str,
    pub body: String,
    pub result: Option<Value>,
}

impl RouteResponse {
    pub fn text(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            content_type: "text/plain; charset=utf-8",
            body: body.into(),
            result: None,
        }
    }
}

/// Les extensions lues dans `views/`.
const VIEW_EXTENSIONS: [&str; 5] = ["html", "htm", "jinja", "txt", "xml"];

impl Routes {
    /// Vérifie les routes déclarées contre les outils du backend et lit le
    /// dossier `views/` (découvert, comme `nodes/`).
    pub fn load(
        decls: &BTreeMap<String, RouteDecl>,
        directory: &Path,
        tools: &dyn Fn(&str) -> bool,
    ) -> Result<Self, String> {
        let views = read_views(&directory.join("views"))?;
        let mut routes: Vec<Route> = Vec::new();
        for (key, decl) in decls {
            let at = |why: String| format!("route « {key} » : {why}");
            let (method, path) = key
                .split_once(' ')
                .ok_or_else(|| at("la clé s'écrit « MÉTHODE /chemin »".into()))?;
            if !METHODS.contains(&method) {
                return Err(at(format!(
                    "méthode « {method} » inconnue (connues : {})",
                    METHODS.join(", ")
                )));
            }
            let segments = parse_path(path).map_err(at)?;
            if !tools(&decl.tool) {
                return Err(at(format!(
                    "l'outil « {} » n'est pas déclaré dans \"tools\"",
                    decl.tool
                )));
            }
            if let Some(view) = &decl.view {
                if !views.contains_key(view) {
                    return Err(at(format!(
                        "la vue « {view} » n'est pas dans views/ (vues : {})",
                        views.keys().cloned().collect::<Vec<_>>().join(", ")
                    )));
                }
            }
            if let Some(other) = routes
                .iter()
                .find(|r| r.method == method && same_shape(&r.segments, &segments))
            {
                return Err(at(format!(
                    "ambiguë avec « {} » : mêmes segments, seuls les noms des paramètres changent",
                    other.key
                )));
            }
            routes.push(Route {
                key: key.clone(),
                method: method.to_string(),
                segments,
                tool: decl.tool.clone(),
                view: decl.view.clone(),
            });
        }
        // Les littéraux d'abord : « /products/new » passe devant « /products/{key} ».
        routes.sort_by_key(|r| {
            r.segments
                .iter()
                .map(|s| matches!(s, Segment::Param(_)))
                .collect::<Vec<_>>()
        });
        let checked = Self { routes, views };
        // Chaque vue se compile, avec toutes les autres (extends, include).
        let env = checked.environment();
        for name in checked.views.keys() {
            env.get_template(name)
                .map_err(|e| format!("views/{name} : {}", describe(&e)))?;
        }
        Ok(checked)
    }

    pub fn is_empty(&self) -> bool {
        self.routes.is_empty()
    }

    /// La route d'une requête, ses paramètres de chemin ; ou la réponse qui
    /// dit pourquoi aucune ne convient (404, 405).
    pub(crate) fn find(
        &self,
        method: &str,
        path: &str,
    ) -> Result<(&str, Option<&str>, Map<String, Value>), RouteResponse> {
        let parts: Vec<&str> = path.trim_end_matches('/').split('/').skip(1).collect();
        let mut allowed = Vec::new();
        for route in &self.routes {
            let Some(params) = matches(&route.segments, &parts) else {
                continue;
            };
            if route.method != method {
                allowed.push(route.method.clone());
                continue;
            }
            return Ok((&route.tool, route.view.as_deref(), params));
        }
        Err(if allowed.is_empty() {
            RouteResponse::text(404, format!("aucune route ne mène à {path}"))
        } else {
            RouteResponse::text(
                405,
                format!("{method} {path} : méthodes admises {}", allowed.join(", ")),
            )
        })
    }

    /// Rend une vue avec son contexte. Les vues `.html`/`.htm`/`.xml`
    /// échappent ce qu'elles reçoivent.
    pub(crate) fn render(&self, view: &str, context: &Value) -> Result<String, String> {
        let env = self.environment();
        env.get_template(view)
            .and_then(|t| t.render(context))
            .map_err(|e| format!("views/{view} : {}", describe(&e)))
    }

    fn environment(&self) -> minijinja::Environment<'_> {
        let mut env = minijinja::Environment::new();
        for (name, source) in &self.views {
            // Vérifié par `load` : une erreur ici serait un défaut à nous.
            let _ = env.add_template(name, source);
        }
        env
    }
}

/// Les arguments de l'outil : le corps (un objet JSON), puis la requête, puis
/// le chemin. Une même clé donnée deux fois est refusée en la nommant. Une
/// valeur de chemin ou de requête est du texte ; elle est lue comme JSON
/// quand le paramètre de l'outil n'est pas une chaîne.
pub(crate) fn arguments(
    request: &RouteRequest,
    path: Map<String, Value>,
    input_schema: Option<&Value>,
) -> Result<Map<String, Value>, RouteResponse> {
    let mut args = match &request.body {
        None => Map::new(),
        Some(Value::Object(body)) => body.clone(),
        Some(_) => return Err(RouteResponse::text(400, "le corps doit être un objet JSON")),
    };
    let typed = |name: &str, text: &str| -> Value {
        let is_string = input_schema
            .and_then(|s| s["properties"][name]["type"].as_str())
            .is_none_or(|t| t == "string");
        if is_string {
            Value::String(text.to_string())
        } else {
            serde_json::from_str(text).unwrap_or_else(|_| Value::String(text.to_string()))
        }
    };
    let query = request.query.iter().map(|(k, v)| (k.clone(), typed(k, v)));
    let path = path.into_iter().map(|(k, v)| {
        let text = v.as_str().unwrap_or_default().to_string();
        (k.clone(), typed(&k, &text))
    });
    for (key, value) in query.chain(path) {
        if args.insert(key.clone(), value).is_some() {
            return Err(RouteResponse::text(
                400,
                format!("« {key} » est donné deux fois (corps, requête ou chemin)"),
            ));
        }
    }
    Ok(args)
}

fn parse_path(path: &str) -> Result<Vec<Segment>, String> {
    if !path.starts_with('/') {
        return Err("le chemin commence par /".into());
    }
    let mut seen = std::collections::BTreeSet::new();
    path.trim_end_matches('/')
        .split('/')
        .skip(1)
        .map(|part| {
            if let Some(name) = part.strip_prefix('{').and_then(|p| p.strip_suffix('}')) {
                let valid = name
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                    && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
                if !valid || !seen.insert(name.to_string()) {
                    return Err(format!(
                        "paramètre « {name} » : un identifiant (ASCII), une seule fois"
                    ));
                }
                Ok(Segment::Param(name.to_string()))
            } else if part.is_empty() || part.contains(['{', '}']) {
                Err(format!("segment « {part} » invalide"))
            } else {
                Ok(Segment::Literal(part.to_string()))
            }
        })
        .collect()
}

fn same_shape(a: &[Segment], b: &[Segment]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(x, y)| match (x, y) {
            (Segment::Literal(x), Segment::Literal(y)) => x == y,
            (Segment::Param(_), Segment::Param(_)) => true,
            _ => false,
        })
}

fn matches(segments: &[Segment], parts: &[&str]) -> Option<Map<String, Value>> {
    let parts: Vec<&str> = parts.iter().copied().filter(|p| !p.is_empty()).collect();
    if segments.len() != parts.len() {
        return None;
    }
    let mut params = Map::new();
    for (segment, part) in segments.iter().zip(parts) {
        match segment {
            Segment::Literal(text) if text == part => {}
            Segment::Literal(_) => return None,
            Segment::Param(name) => {
                params.insert(name.clone(), Value::String(part.to_string()));
            }
        }
    }
    Some(params)
}

fn read_views(dir: &Path) -> Result<BTreeMap<String, String>, String> {
    let mut views = BTreeMap::new();
    if !dir.exists() {
        return Ok(views);
    }
    let mut stack = vec![dir.to_path_buf()];
    while let Some(folder) = stack.pop() {
        let entries = std::fs::read_dir(&folder).map_err(|e| format!("views/ : {e}"))?;
        for entry in entries {
            let path = entry.map_err(|e| format!("views/ : {e}"))?.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let known = path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| VIEW_EXTENSIONS.contains(&e));
            if !known {
                continue;
            }
            let name = path
                .strip_prefix(dir)
                .unwrap_or(&path)
                .components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            let source =
                std::fs::read_to_string(&path).map_err(|e| format!("views/{name} : {e}"))?;
            views.insert(name, source);
        }
    }
    Ok(views)
}

/// Une erreur de gabarit, avec sa ligne quand minijinja la connaît.
fn describe(error: &minijinja::Error) -> String {
    match error.line() {
        Some(line) => format!("{} (ligne {line})", error),
        None => error.to_string(),
    }
}
