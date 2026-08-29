//! Rendre des résultats **pour un modèle**, pas pour un programme.
//!
//! Le JSON brut d'une recherche coûte cher : `uuid`, `score` en flottant
//! long, `_content_hash`, le `content` entier de chaque résultat, et — pour
//! un voisin de graphe — **toutes** les colonnes de la table, nulles
//! comprises. Mesuré le 26 août : 370 000 jetons pour trois questions, dont
//! l'essentiel en champs que le modèle ne lit jamais
//! ([doc 11](../../docs/25-aout-2026-18h58/11-gemini-fiches-bornees-mesure.md)).
//!
//! `read` et `grep` rendent du markdown compact depuis le début ; ce nœud
//! fait la même chose pour les résultats. Il est **passe-plat** : le port
//! `results` ressort tel quel, pour qu'un graphe continue à composer (c'est
//! ce qui permet à `search_expand` de contenir `search`), et `text` porte la
//! version lisible.

use crate::connection::CypherValue;
use crate::search_strategy::UnifiedResult;

use super::node::{Node, NodeContext};
use super::node_registry::{Choices, ConfigParam, ConfigParamType, NodeFactory, NodeSchema};
use super::port::{take_or_clone, PortDef, PortType, PortValue, QueryPayload};

/// Longueur d'un extrait, en caractères.
const DEFAULT_MAX_CHARS: usize = 300;
/// Longueur d'une valeur de champ, en caractères.
const FIELD_CHARS: usize = 120;

/// Une valeur de colonne, si elle vaut la peine d'être montrée.
///
/// `Null` disparaît — c'est la moitié du poids d'un voisin de graphe. Les
/// listes et les cartes aussi : personne ne lit un vecteur d'embedding.
fn scalar(v: &CypherValue) -> Option<String> {
    match v {
        CypherValue::String(s) if s.is_empty() => None,
        CypherValue::String(s) => Some(ellipsize(s, FIELD_CHARS)),
        CypherValue::Int(i) => Some(i.to_string()),
        CypherValue::Float(f) => Some(format!("{f:.4}")),
        CypherValue::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

fn ellipsize(s: &str, max: usize) -> String {
    let clean = s.replace(['\n', '\r'], " ");
    let clean = clean.trim();
    if clean.chars().count() <= max {
        return clean.to_string();
    }
    let cut: String = clean.chars().take(max).collect();
    format!("{cut}…")
}

/// Les champs internes du moteur — jamais pour le modèle.
fn is_internal(key: &str) -> bool {
    key.starts_with('_')
}

/// Les champs que le rendu consomme lui-même : ils deviennent le lien, le
/// titre hiérarchique ou l'en-tête de groupe, et ne sont donc pas répétés
/// dans la liste des champs.
const CONSUMED: [&str; 15] = [
    "file_path", "path", "start_line", "end_line", "parent_name", "language",
    // Une empreinte de 64 caractères hexadécimaux sur chaque voisin : elle ne
    // dit rien à un modèle, et elle coûte plus que la ligne qui la porte.
    "content_hash",
    // Le corps d'un scope : il est déjà dans l'extrait, en dessous. L'avoir
    // aussi en `content=…` le fait payer deux fois, tronqué différemment.
    "content",
    // Les coordonnées : elles servent à identifier, à filtrer et à écrire le
    // chemin — jamais à être récitées à côté de lui.
    "source", "repo", "repo_path", "revision",
    // Promus : ils ont leur place dans la fiche (`📝`, `🔹`, le type entre
    // parenthèses), pas dans la liste des colonnes restantes.
    "docstring", "signature", "scope_type",
];

/// **Par rapport à quoi on écrit un chemin.**
///
/// Le stockage est absolu (doc 04 v3) ; l'affichage, lui, est un point de
/// vue, et il change à chaque tour de boucle sans que rien ne soit
/// réindexé. C'est la quatrième des quatre notions qui s'appelaient toutes
/// « racine », et la seule qui ait le droit de bouger si souvent.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum PathLens {
    /// Le chemin **dans son dépôt** quand on le connaît — `repo_path` est
    /// déjà stocké, donc c'est gratuit — et l'absolu sinon.
    #[default]
    Origin,
    /// Depuis un préfixe donné : « montre-moi les chemins d'ici ».
    From(String),
    /// Tel qu'il est stocké.
    Absolute,
}

impl PathLens {
    /// Le chemin d'un résultat, vu par cette lentille.
    fn path_of(&self, data: Option<&Data>) -> Option<String> {
        let stored = text_field(data, "file_path").or_else(|| text_field(data, "path"))?;
        Some(match self {
            Self::Absolute => stored,
            Self::Origin => text_field(data, "repo_path").unwrap_or(stored),
            Self::From(prefix) => {
                let p = prefix.trim_end_matches('/');
                match stored.strip_prefix(p) {
                    // Hors du préfixe : on rend l'absolu plutôt qu'un chemin
                    // faux. Un `../../..` serait juste et illisible.
                    None => stored,
                    Some(rest) => {
                        let rest = rest.trim_start_matches('/');
                        if rest.is_empty() { stored } else { rest.to_string() }
                    }
                }
            }
        })
    }
}

type Data = std::collections::BTreeMap<String, CypherValue>;

fn text_field(data: Option<&Data>, key: &str) -> Option<String> {
    match data?.get(key) {
        Some(CypherValue::String(s)) if !s.is_empty() => Some(s.clone()),
        _ => None,
    }
}

fn int_field(data: Option<&Data>, key: &str) -> Option<i64> {
    match data?.get(key) {
        Some(CypherValue::Int(i)) => Some(*i),
        _ => None,
    }
}

/// `port.rs:101-140` — de quoi lancer `read(path, offset)` sans réfléchir.
/// C'est la forme que tout le monde sait lire, et la seule que le modèle
/// peut réutiliser telle quelle.
fn location(data: Option<&Data>, lens: &PathLens) -> Option<String> {
    let path = lens.path_of(data)?;
    match (int_field(data, "start_line"), int_field(data, "end_line")) {
        (Some(a), Some(b)) if b > a => Some(format!("{path}:{a}-{b}")),
        (Some(a), _) => Some(format!("{path}:{a}")),
        _ => Some(path),
    }
}

/// Le séparateur de portée de la langue : `Classe.methode` en Python et en
/// JavaScript, `Classe::methode` ailleurs. Détail, mais c'est ce qu'un
/// humain écrirait, donc ce qu'un modèle reconnaît.
fn scope_sep(data: Option<&Data>) -> &'static str {
    match text_field(data, "language").as_deref() {
        Some("python" | "javascript" | "typescript" | "ruby" | "java" | "csharp" | "go") => ".",
        _ => "::",
    }
}

/// Le nom d'un résultat : ce qu'un humain citerait.
fn title_of(
    data: Option<&std::collections::BTreeMap<String, CypherValue>>,
    uuid: &str,
    lens: &PathLens,
) -> String {
    let Some(data) = data else { return uuid.chars().take(8).collect() };
    for key in ["_title", "name", "title", "path", "file_path", "summary", "content"] {
        // Le titre d'un fichier **est** son chemin : il passe donc par la
        // lentille comme le reste.
        if matches!(key, "path" | "file_path") {
            if let Some(p) = lens.path_of(Some(data)) {
                return ellipsize(&p, 80);
            }
            continue;
        }
        if let Some(CypherValue::String(s)) = data.get(key) {
            if !s.is_empty() {
                return ellipsize(s, 80);
            }
        }
    }
    uuid.chars().take(8).collect()
}

/// Les champs à montrer, dans l'ordre du schéma, sans le titre ni les
/// internes ni les vides.
fn fields_of(
    data: Option<&std::collections::BTreeMap<String, CypherValue>>,
    title: &str,
) -> Vec<String> {
    let Some(data) = data else { return Vec::new() };
    data.iter()
        .filter(|(k, _)| !is_internal(k) && !CONSUMED.contains(&k.as_str()))
        .filter_map(|(k, v)| scalar(v).map(|s| (k, s)))
        .filter(|(_, s)| s != title)
        .map(|(k, s)| format!("{k}={s}"))
        .collect()
}

/// La clé de regroupement : le parent, dans son fichier. Vide = pas de
/// groupe.
fn group_key(r: &UnifiedResult, lens: &PathLens) -> Option<(String, String)> {
    let parent = text_field(r.data.as_ref(), "parent_name")?;
    let file = lens.path_of(r.data.as_ref()).unwrap_or_default();
    Some((file, parent))
}

// ─── Le modèle de vue ───────────────────────────────────────────────────────

/// **Ce que le gabarit voit.**
///
/// La séparation est nette et c'est tout l'intérêt : Rust décide *quoi*
/// montrer — la lentille de chemin, le regroupement, les champs internes qui
/// disparaissent, les extraits bornés — et le gabarit décide *comment*. La
/// forme d'une fiche de recherche change beaucoup plus vite que le moteur ;
/// elle n'a rien à faire dans un `format!`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ResultsView {
    /// Le texte cherché, quand le nœud le reçoit sur son port `query`.
    pub query: Option<String>,
    /// L'entité ou la base où l'on a cherché.
    pub target: Option<String>,
    pub count: usize,
    pub results: Vec<ResultView>,
    /// Le décompte par type, dans l'ordre décroissant.
    pub types: Vec<TypeCount>,
    /// Vrai si au moins un résultat a un voisin — de quoi décider d'écrire la
    /// section « graphe » sans la parcourir deux fois.
    pub has_graph: bool,
    /// Ce que le domaine de travail ne montre pas, en une phrase.
    pub domain: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ResultView {
    pub rank: usize,
    /// `Classe::methode` quand le parent est connu.
    pub title: String,
    /// Le nom seul.
    pub name: String,
    pub entity: String,
    /// `function`, `class`, `struct`… — le `scope_type` du code, quand il existe.
    pub kind: Option<String>,
    /// Déjà arrondi : un gabarit n'a pas à savoir formater un flottant.
    pub score: String,
    pub signal: Option<String>,
    pub relation: Option<String>,
    /// `port.rs:120-140` — de quoi lancer un `read` sans réfléchir.
    pub location: Option<String>,
    /// Le chemin seul, sans les lignes.
    pub path: Option<String>,
    pub signature: Option<String>,
    pub doc: Option<String>,
    /// Les colonnes restantes, `clé=valeur`, sans les internes ni les nulles.
    pub fields: Vec<String>,
    pub snippets: Vec<String>,
    pub more_snippets: usize,
    /// Présent **seulement sur le premier** d'un groupe : c'est lui qui porte
    /// l'en-tête.
    pub group: Option<GroupView>,
    /// Les voisins, regroupés par relation.
    pub relations: Vec<RelationGroup>,
    pub matched: Vec<MatchedView>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct GroupView {
    pub title: String,
    pub file: String,
    pub count: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RelationGroup {
    pub relation: String,
    pub items: Vec<NeighbourView>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct NeighbourView {
    pub entity: String,
    pub title: String,
    pub location: Option<String>,
    pub fields: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct MatchedView {
    pub entity: String,
    pub title: String,
    pub score: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TypeCount {
    pub name: String,
    pub count: usize,
}

/// **Assez de décimales pour que l'étoile serve à quelque chose.**
///
/// Deux décimales conviennent à BM25 (`14.19`, `0.92`) et écrasent la fusion :
/// un score RRF vaut `poids / (60 + rang)`, donc quatre résultats sortaient
/// tous à `★ 0.01`. Un modèle qui lit quatre fois le même nombre n'apprend
/// rien — autant ne rien écrire.
///
/// On **ne normalise pas** sur le meilleur : mettre le premier à `1.00`
/// laisserait croire à une correspondance parfaite là où les quatre peuvent
/// être mauvais. On augmente la précision jusqu'à ce que les valeurs se
/// distinguent, et on s'arrête là.
fn score_scale(scores: &[f64]) -> usize {
    for decimals in 2..=5 {
        let mut rendered: Vec<String> = scores.iter().map(|s| format!("{s:.decimals$}")).collect();
        let before = rendered.len();
        rendered.sort();
        rendered.dedup();
        if rendered.len() == before {
            return decimals;
        }
    }
    5
}

/// Construit la vue : l'ordre de sortie, les groupes, les champs retenus.
///
/// Le regroupement **réordonne** — les groupes sortent dans l'ordre de leur
/// meilleur score, la numérotation reste globale, et un résultat seul dans son
/// groupe n'a pas d'en-tête.
pub fn build_view(
    results: &[UnifiedResult],
    max_chars: usize,
    group: bool,
    lens: &PathLens,
) -> ResultsView {
    let mut order: Vec<usize> = (0..results.len()).collect();
    let mut header_at: std::collections::HashMap<usize, GroupView> = std::collections::HashMap::new();
    if group {
        // Un groupe par (fichier, parent) ; sans parent, chacun le sien.
        let mut groups: Vec<(Option<(String, String)>, Vec<usize>)> = Vec::new();
        for (i, r) in results.iter().enumerate() {
            let key = group_key(r, lens);
            match key.as_ref().and_then(|k| groups.iter_mut().find(|(g, _)| g.as_ref() == Some(k))) {
                Some((_, members)) => members.push(i),
                None => groups.push((key, vec![i])),
            }
        }
        order.clear();
        for (key, members) in groups {
            if let (Some((file, parent)), true) = (key, members.len() > 1) {
                header_at.insert(
                    members[0],
                    GroupView { title: parent, file, count: members.len() },
                );
            }
            order.extend(members);
        }
    }

    // Deux résultats *peuvent* légitimement avoir le même score ; c'est
    // seulement quand ils s'écrasent tous qu'il faut plus de précision.
    let decimals = score_scale(&results.iter().map(|r| r.score).collect::<Vec<_>>());

    let mut views = Vec::with_capacity(order.len());
    let mut types: Vec<TypeCount> = Vec::new();
    let mut has_graph = false;
    for (rank, &i) in order.iter().enumerate() {
        let r = &results[i];
        let name = title_of(r.data.as_ref(), &r.uuid, lens);
        let title = match text_field(r.data.as_ref(), "parent_name") {
            Some(parent) if parent != name => format!("{parent}{}{name}", scope_sep(r.data.as_ref())),
            _ => name.clone(),
        };
        let entity = r.entity.as_deref().unwrap_or("?").to_string();
        let kind = text_field(r.data.as_ref(), "scope_type");

        // Le décompte par type : le `scope_type` s'il existe, l'entité sinon.
        let bucket = kind.clone().unwrap_or_else(|| entity.clone());
        match types.iter_mut().find(|t| t.name == bucket) {
            Some(t) => t.count += 1,
            None => types.push(TypeCount { name: bucket, count: 1 }),
        }

        let signature = text_field(r.data.as_ref(), "signature").map(|s| ellipsize(&s, FIELD_CHARS));

        // Un extrait qui répète le titre ou la signature ne dit rien de plus,
        // et il le dit sur deux lignes. C'est le cas de tous les scopes d'une
        // ligne — une signature de fonction *est* son contenu.
        let redundant = |text: &str| {
            text.is_empty() || text == name || Some(text) == signature.as_deref()
        };

        let mut snippets = Vec::new();
        let mut more_snippets = 0;
        if let Some(chunk) = &r.chunk {
            let text = ellipsize(&chunk.text, max_chars);
            if !redundant(&text) {
                snippets.push(text);
            }
        }
        if let Some(chunks) = &r.chunks {
            for c in chunks.iter().take(3) {
                let text = ellipsize(&c.text, max_chars);
                if !redundant(&text) {
                    snippets.push(text);
                }
            }
            more_snippets = chunks.len().saturating_sub(3);
        }

        // Les voisins, regroupés par relation — l'ordre de première apparition.
        let mut relations: Vec<RelationGroup> = Vec::new();
        for child in r.other_children.iter().flatten() {
            let child_title = title_of(Some(&child.data), &child.uuid, lens);
            // Un `File` a pour titre son chemin : le répéter en « @ … » ne
            // dit rien de plus.
            let loc = location(Some(&child.data), lens).filter(|l| *l != child_title);
            let view = NeighbourView {
                entity: child.entity.clone(),
                fields: fields_of(Some(&child.data), &child_title),
                location: loc,
                title: child_title,
            };
            match relations.iter_mut().find(|g| g.relation == child.relation) {
                Some(g) => g.items.push(view),
                None => relations.push(RelationGroup { relation: child.relation.clone(), items: vec![view] }),
            }
        }
        has_graph |= !relations.is_empty();

        let matched = r
            .matched_children
            .iter()
            .flatten()
            .map(|child| MatchedView {
                entity: child.entity.as_deref().unwrap_or("?").to_string(),
                title: title_of(child.data.as_ref(), &child.uuid, lens),
                score: format!("{:.decimals$}", child.score),
            })
            .collect();

        views.push(ResultView {
            rank: rank + 1,
            name: name.clone(),
            fields: fields_of(r.data.as_ref(), &name),
            title,
            entity,
            kind,
            score: format!("{:.decimals$}", r.score),
            signal: r.signal.clone(),
            relation: r.relation.clone(),
            location: location(r.data.as_ref(), lens),
            path: lens.path_of(r.data.as_ref()),
            signature,
            doc: text_field(r.data.as_ref(), "docstring").map(|s| ellipsize(&s, max_chars)),
            snippets,
            more_snippets,
            group: header_at.remove(&i),
            relations,
            matched,
        });
    }
    types.sort_by(|a, b| b.count.cmp(&a.count).then(a.name.cmp(&b.name)));

    ResultsView {
        query: None,
        target: None,
        count: results.len(),
        results: views,
        types,
        has_graph,
        domain: None,
    }
}

// ─── Les gabarits ───────────────────────────────────────────────────────────

/// La fiche par défaut, reprise de la maquette d'origine (`LR_CodeRag`,
/// `BRAIN_SEARCH_OUTPUT_PROPOSAL.md`).
pub const DEFAULT_TEMPLATE: &str = include_str!("../../templates/render/results.md.jinja");
/// Une ligne par résultat — trois fois moins cher en jetons.
pub const COMPACT_TEMPLATE: &str = include_str!("../../templates/render/compact.md.jinja");

/// Les gabarits fournis, par nom.
pub fn builtin_template(name: &str) -> Option<&'static str> {
    match name {
        "" | "default" | "results" => Some(DEFAULT_TEMPLATE),
        "compact" => Some(COMPACT_TEMPLATE),
        _ => None,
    }
}

/// Où chercher les gabarits écrits à la main. Défaut : `templates/render/`
/// depuis le répertoire courant.
pub const TEMPLATES_DIR_ENV: &str = "RAG3WEAVER_RENDER_TEMPLATES";

fn templates_dir() -> std::path::PathBuf {
    std::env::var_os(TEMPLATES_DIR_ENV)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("templates/render"))
}

/// **Ce que `template=` veut dire**, dans l'ordre où on essaie :
/// un nom fourni, puis — si ça ressemble à du Jinja — la source elle-même,
/// puis un gabarit posé dans `templates/render/<nom>.md.jinja`.
///
/// **Un nom, pas un chemin.** `template=` est une configuration de nœud, et un
/// graphe peut être écrit par un modèle : accepter un chemin quelconque ferait
/// de ce champ une lecture de fichier arbitraire, rendue au modèle, qui
/// contournerait le domaine de travail par lequel passent `read` et `grep`.
/// Le séparateur et le `..` sont donc refusés, et la lecture est confinée au
/// répertoire des gabarits.
pub fn resolve_template(spec: &str) -> Result<std::borrow::Cow<'static, str>, String> {
    if let Some(builtin) = builtin_template(spec) {
        return Ok(std::borrow::Cow::Borrowed(builtin));
    }
    if spec.contains("{{") || spec.contains("{%") {
        return Ok(std::borrow::Cow::Owned(spec.to_string()));
    }
    if spec.contains('/') || spec.contains('\\') || spec.contains("..") {
        return Err(format!(
            "gabarit '{spec}' : un nom, pas un chemin — les gabarits écrits à la main vivent dans {} (voir ${TEMPLATES_DIR_ENV})",
            templates_dir().display()
        ));
    }
    let chemin = templates_dir().join(format!("{spec}.md.jinja"));
    std::fs::read_to_string(&chemin)
        .map(std::borrow::Cow::Owned)
        .map_err(|e| format!(
            "gabarit '{spec}' : ni un nom fourni (default, compact), ni du Jinja, ni {} ({e})",
            chemin.display()
        ))
}

/// Rend une vue à travers un gabarit.
pub fn render_view(view: &ResultsView, template: &str) -> Result<String, String> {
    let mut env = minijinja::Environment::new();
    env.add_template("results", template).map_err(|e| format!("gabarit invalide : {e}"))?;
    let tpl = env.get_template("results").map_err(|e| format!("gabarit : {e}"))?;
    let out = tpl.render(view).map_err(|e| format!("gabarit : {e}"))?;
    Ok(out.trim_end().to_string())
}

/// Le rendu markdown d'une liste de résultats — la surface que le modèle lit.
pub fn render_results_markdown(results: &[UnifiedResult], max_chars: usize) -> String {
    render_results_with(results, max_chars, true)
}

/// La même, en choisissant de regrouper ou non les résultats qui partagent
/// une classe (ou une fonction englobante).
pub fn render_results_with(results: &[UnifiedResult], max_chars: usize, group: bool) -> String {
    render_results_through(results, max_chars, group, &PathLens::default())
}

/// La même, en choisissant **par rapport à quoi les chemins sont écrits**.
pub fn render_results_through(
    results: &[UnifiedResult],
    max_chars: usize,
    group: bool,
    lens: &PathLens,
) -> String {
    let view = build_view(results, max_chars, group, lens);
    render_view(&view, DEFAULT_TEMPLATE).unwrap_or_else(|e| format!("**Render failed.** {e}"))
}

// ─── RenderResultsNode ──────────────────────────────────────────────────────

/// Rend `results` en markdown sur `text`, et **laisse passer** les résultats
/// sur `results` — un graphe qui contient celui-ci continue à composer.
///
/// La forme est un **gabarit** (`templates/render/`), pas du `format!` :
/// `template=` prend un nom fourni, un chemin, ou la source elle-même.
pub struct RenderResultsNode {
    node_name: String,
    json: bool,
    max_chars: usize,
    group: bool,
    lens: PathLens,
    template: String,
}

impl RenderResultsNode {
    pub fn new(name: &str) -> Self {
        Self {
            node_name: name.to_string(),
            json: false,
            max_chars: DEFAULT_MAX_CHARS,
            group: true,
            lens: PathLens::default(),
            template: "default".to_string(),
        }
    }
    /// Le gabarit : un nom fourni (`default`, `compact`), un chemin de
    /// fichier, ou la source Jinja elle-même.
    pub fn with_template(mut self, spec: impl Into<String>) -> Self {
        self.template = spec.into();
        self
    }
    /// `true` : le JSON brut, pour un appelant qui est un programme.
    pub fn with_json(mut self, json: bool) -> Self {
        self.json = json;
        self
    }
    pub fn with_max_chars(mut self, max_chars: usize) -> Self {
        self.max_chars = max_chars;
        self
    }
    /// Regrouper les résultats d'une même classe (défaut : oui).
    pub fn with_group(mut self, group: bool) -> Self {
        self.group = group;
        self
    }
    /// Par rapport à quoi écrire les chemins (défaut : leur dépôt).
    pub fn with_lens(mut self, lens: PathLens) -> Self {
        self.lens = lens;
        self
    }

    /// La lentille dérivée du poste de travail : la racine de la source, plus
    /// le répertoire courant s'il existe.
    ///
    /// Rien de tout ça n'est obligatoire — sans `FileSource` dans le registre,
    /// on retombe sur `Origin`, qui est le bon défaut pour un catalogue qu'on
    /// interroge de l'extérieur.
    fn lens_du_travail(&self, ctx: &mut NodeContext) -> PathLens {
        let Some(source) = ctx
            .service::<std::sync::Arc<dyn crate::code_tools::FileSource>>(crate::code_tools::FILE_SOURCE_SERVICE)
        else {
            return PathLens::Origin;
        };
        // `worktree:<racine>` — une source virtuelle (instantané) n'a pas de
        // racine sur le disque, donc rien à retrancher.
        let Some(racine) = source.cursor().strip_prefix("worktree:").map(str::to_string) else {
            return PathLens::Origin;
        };
        // Le répertoire courant est déjà absolu : c'est la lentille, telle
        // quelle. Sans lui, la racine de la source.
        let prefixe = ctx
            .service::<std::sync::Arc<crate::code_tools::Cwd>>(crate::code_tools::CWD_SERVICE)
            .map(|c| c.get().to_string_lossy().to_string())
            .unwrap_or(racine);
        PathLens::From(prefixe)
    }
}

impl Node for RenderResultsNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "RenderResultsNode"
    }
    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        Some(Box::new(serde_json::json!({
            "format": if self.json { "json" } else { "markdown" },
            "max_chars": self.max_chars,
            "group": self.group,
            "template": self.template,
            "relative_to": match &self.lens {
                PathLens::From(p) => p.clone(),
                PathLens::Absolute => "/".to_string(),
                PathLens::Origin => String::new(),
            },
        })))
    }
    fn inputs(&self) -> Vec<PortDef> {
        vec![
            PortDef { name: "results", port_type: PortType::Results, required: false },
            // Facultatif : la requête, pour que la fiche puisse dire ce qu'on
            // cherchait et où. Un graphe qui ne la branche pas rend la même
            // liste, sans l'en-tête.
            PortDef { name: "query", port_type: PortType::Query, required: false },
        ]
    }
    fn outputs(&self) -> Vec<PortDef> {
        vec![
            PortDef { name: "text", port_type: PortType::Text, required: false },
            PortDef { name: "results", port_type: PortType::Results, required: false },
            // **La requête ressort, comme les résultats.** Un graphe qui en
            // contient un autre ne voit de lui que ses ports **libres** : le
            // `source.query` du sous-graphe est consommé à l'intérieur, donc
            // invisible. Sans ce passe-plat, l'étage extérieur de `search` ne
            // pouvait pas dire ce qu'on avait cherché, et sa fiche perdait son
            // en-tête dès qu'on demandait une relation (28 août 2026).
            PortDef { name: "query", port_type: PortType::Query, required: false },
        ]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let results: Vec<UnifiedResult> = ctx
            .take_input("results")
            .and_then(take_or_clone::<Vec<UnifiedResult>>)
            .unwrap_or_default();
        let query: Option<QueryPayload> = ctx.take_input("query").and_then(take_or_clone::<QueryPayload>);
        let text = if self.json {
            serde_json::to_string(&results).map_err(|e| format!("RenderResultsNode: {e}"))?
        } else {
            // **Par rapport à où l'agent se tient.**
            //
            // Sans lentille explicite, on écrit les chemins relativement à la
            // source — et au répertoire courant s'il y en a un. C'est ce qui
            // rend un `📍` copiable tel quel dans `read` : ces outils parlent
            // en chemins relatifs à la source depuis toujours, alors que le
            // catalogue stocke l'absolu (doc 04 v3). L'agent n'a plus à
            // traduire, et chaque ligne cesse de porter soixante caractères de
            // préfixe payés en jetons.
            let lens = match &self.lens {
                PathLens::Origin => self.lens_du_travail(ctx),
                explicite => explicite.clone(),
            };
            let mut view = build_view(&results, self.max_chars, self.group, &lens);
            if let Some(qp) = &query {
                view.query = Some(qp.query.clone());
                view.target = Some(qp.target_name.clone());
            }
            // **Le domaine dit ce qu'il ne montre pas.** Sans cette ligne, un
            // agent ne peut pas distinguer « ça n'existe pas » de « ce n'est
            // pas dans mon champ », et l'absence devient un mensonge par
            // omission — la famille de défauts qu'on passe nos journées à
            // débusquer.
            if let Some(domain) = ctx.service::<std::sync::Arc<crate::work_domain::WorkDomain>>(crate::work_domain::WORK_DOMAIN_SERVICE) {
                if !domain.is_everything() {
                    view.domain = Some(domain.describe());
                }
            }
            // Un gabarit cassé est une erreur de configuration : elle se voit,
            // elle ne se rattrape pas en silence.
            let tpl = resolve_template(&self.template).map_err(|e| format!("RenderResultsNode: {e}"))?;
            render_view(&view, &tpl).map_err(|e| format!("RenderResultsNode: {e}"))?
        };
        ctx.set_output("text", PortValue::new(text));
        ctx.set_output("results", PortValue::new(results));
        if let Some(qp) = query {
            ctx.set_output("query", PortValue::new(qp));
        }
        Ok(())
    }
}

pub struct RenderResultsNodeFactory;

impl NodeFactory for RenderResultsNodeFactory {
    fn create(&self, name: &str, config: &serde_json::Value) -> Result<Box<dyn Node>, String> {
        let mut node = RenderResultsNode::new(name);
        match config.get("format").and_then(|v| v.as_str()) {
            None | Some("markdown") => {}
            Some("json") => node = node.with_json(true),
            Some(other) => return Err(format!("RenderResultsNode: unknown format '{other}' (markdown | json)")),
        }
        if let Some(n) = config.get("max_chars").and_then(|v| v.as_u64()) {
            node = node.with_max_chars(n as usize);
        }
        if let Some(g) = config.get("group").and_then(|v| v.as_bool()) {
            node = node.with_group(g);
        }
        if let Some(t) = config.get("template").and_then(|v| v.as_str()) {
            // Résolu ici, à la construction : un gabarit introuvable se dit au
            // montage du graphe, pas au milieu d'un tour d'agent.
            resolve_template(t).map_err(|e| format!("RenderResultsNode: {e}"))?;
            node = node.with_template(t);
        }
        // `relative_to` : vide = le chemin dans son dépôt ; `/` = l'absolu
        // tel qu'il est stocké ; un chemin = depuis là.
        if let Some(p) = config.get("relative_to").and_then(|v| v.as_str()) {
            node = node.with_lens(match p {
                "" => PathLens::Origin,
                "/" => PathLens::Absolute,
                other => PathLens::From(other.to_string()),
            });
        }
        Ok(Box::new(node))
    }
    fn node_type(&self) -> &'static str {
        "RenderResultsNode"
    }
    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "RenderResultsNode",
            description: "Renders results as markdown on 'text' through a template (nulls, internal fields and embeddings dropped) and passes them through on 'results'",
            inputs: vec![
                PortDef { name: "results", port_type: PortType::Results, required: false },
                PortDef { name: "query", port_type: PortType::Query, required: false },
            ],
            outputs: vec![
                PortDef { name: "text", port_type: PortType::Text, required: false },
                PortDef { name: "results", port_type: PortType::Results, required: false },
                PortDef { name: "query", port_type: PortType::Query, required: false },
            ],
            config_params: vec![
                ConfigParam {
                    name: "format",
                    param_type: ConfigParamType::String,
                    required: false,
                    default: Some(serde_json::json!("markdown")),
                    description: "markdown (compact, for a model) | json (raw, for a program)",
                    choices: Some(Choices::fixed(["markdown", "json"])),
                    json_schema: None,
                },
                ConfigParam {
                    name: "max_chars",
                    param_type: ConfigParamType::Int,
                    required: false,
                    default: Some(serde_json::json!(DEFAULT_MAX_CHARS)),
                    description: "Snippet length, in characters",
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "group",
                    param_type: ConfigParamType::Bool,
                    required: false,
                    default: Some(serde_json::json!(true)),
                    description: "Group results that share a parent scope under one header (reorders by best score)",
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "template",
                    param_type: ConfigParamType::String,
                    required: false,
                    default: Some(serde_json::json!("default")),
                    description: "Gabarit de rendu : un nom fourni (default | compact), un chemin de fichier, ou la source Jinja elle-même",
                    choices: Some(Choices::fixed(["default", "compact"])),
                    json_schema: None,
                },
                ConfigParam {
                    name: "relative_to",
                    param_type: ConfigParamType::String,
                    required: false,
                    default: Some(serde_json::json!("")),
                    description: "Par rapport à quoi écrire les chemins : vide = dans leur dépôt, '/' = absolu tel que stocké, un chemin = depuis là",
                    choices: None,
                    json_schema: None,
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Les trois lentilles sur le même résultat. Le stockage est absolu ; ce
    /// que le modèle lit ne l'est pas (doc 04 §5).
    #[test]
    fn the_same_result_is_written_three_ways_without_reindexing_anything() {
        let data = std::collections::BTreeMap::from([
            ("file_path".to_string(), CypherValue::String("/home/x/dépôt/src/dataflow/port.rs".into())),
            ("repo_path".to_string(), CypherValue::String("src/dataflow/port.rs".into())),
            ("repo".to_string(), CypherValue::String("github.com/o/dépôt".into())),
            ("name".to_string(), CypherValue::String("merge_port_values".into())),
            ("start_line".to_string(), CypherValue::Int(101)),
            ("end_line".to_string(), CypherValue::Int(140)),
        ]);
        let r = UnifiedResult {
            uuid: "u1".into(),
            entity: Some("Scope".into()),
            score: 1.0,
            data: Some(data),
            chunk: None,
            chunks: None,
            relation: None,
            matched_children: None,
            other_children: None,
            graph: None,
            signal: None,
        };

        let through = |lens: PathLens| render_results_through(std::slice::from_ref(&r), 200, false, &lens);

        let origin = through(PathLens::Origin);
        assert!(origin.contains("📍 `src/dataflow/port.rs:101-140`"), "{origin}");
        assert!(!origin.contains("/home/x/"), "la lentille par défaut ne montre pas le disque : {origin}");

        let absolute = through(PathLens::Absolute);
        assert!(absolute.contains("/home/x/dépôt/src/dataflow/port.rs:101-140"), "{absolute}");

        let from = through(PathLens::From("/home/x/dépôt/src".into()));
        assert!(from.contains("merge_port_values") && from.contains("dataflow/port.rs:101-140"), "{from}");
        assert!(!from.contains("/home/x/"), "{from}");

        // Hors du préfixe : l'absolu, jamais un `../..` juste et illisible.
        let ailleurs = through(PathLens::From("/autre/projet".into()));
        assert!(ailleurs.contains("/home/x/dépôt/src/dataflow/port.rs"), "{ailleurs}");

        // Et les coordonnées ne sont jamais récitées à côté du chemin :
        // elles servent à l'écrire, pas à occuper la place qu'on a passé la
        // journée à libérer.
        for lens in [PathLens::Origin, PathLens::Absolute] {
            let out = through(lens);
            assert!(!out.contains("repo="), "{out}");
            assert!(!out.contains("repo_path="), "{out}");
        }
    }

    use crate::search::ChunkInfo;
    use crate::search_strategy::ChildSummary;

    fn data(pairs: &[(&str, CypherValue)]) -> Data {
        pairs.iter().map(|(k, v)| (k.to_string(), v.clone())).collect()
    }

    fn scope(name: &str, parent: &str, start: i64, end: i64, score: f64) -> UnifiedResult {
        UnifiedResult {
            uuid: format!("uuid-of-{name}"),
            score,
            entity: Some("Scope".into()),
            data: Some(data(&[
                ("name", CypherValue::String(name.into())),
                ("parent_name", CypherValue::String(parent.into())),
                ("file_path", CypherValue::String("port.rs".into())),
                ("language", CypherValue::String("rust".into())),
                ("start_line", CypherValue::Int(start)),
                ("end_line", CypherValue::Int(end)),
                ("_content_hash", CypherValue::String("dead".into())),
                ("docstring", CypherValue::Null),
            ])),
            chunk: None,
            chunks: None,
            relation: None,
            matched_children: None,
            other_children: None,
            graph: None,
            signal: None,
        }
    }

    #[test]
    fn a_result_carries_its_file_link_and_its_hierarchy() {
        let md = render_results_markdown(&[scope("take", "PortValue", 120, 140, 0.813)], 300);
        assert!(md.contains("### 1. PortValue::take ★ 0.81"), "{md}");
        assert!(md.contains("📍 `port.rs:120-140`"), "le lieu est sur sa ligne, copiable : {md}");
        // Les champs consommés ne sont pas répétés, les internes et les nuls
        // ont disparu.
        for absent in ["file_path=", "start_line=", "parent_name=", "_content_hash", "docstring", "uuid"] {
            assert!(!md.contains(absent), "{absent} devrait avoir disparu :\n{md}");
        }
    }

    #[test]
    fn the_separator_follows_the_language() {
        let mut r = scope("run", "Session", 1, 2, 0.5);
        r.data.as_mut().unwrap().insert("language".into(), CypherValue::String("python".into()));
        assert!(render_results_markdown(&[r], 300).contains("Session.run"));
    }

    #[test]
    fn results_of_one_class_are_grouped_once_and_numbered_globally() {
        let results = vec![
            scope("take", "PortValue", 120, 140, 0.81),
            scope("merge_port_values", "", 20, 50, 0.75),
            scope("downcast", "PortValue", 110, 118, 0.60),
        ];
        let md = render_results_with(&results, 300, true);
        assert_eq!(md.matches("**PortValue** · `port.rs` — 2 matches").count(), 1, "{md}");
        // Regroupés, donc réordonnés : les deux de la classe d'abord (leur
        // meilleur score), la numérotation reste globale et continue.
        let order: Vec<&str> = md.lines().filter(|l| l.starts_with("### ")).collect();
        assert_eq!(order.len(), 3, "{md}");
        assert!(order[0].starts_with("### 1. PortValue::take"), "{md}");
        assert!(order[1].starts_with("### 2. PortValue::downcast"), "{md}");
        assert!(order[2].starts_with("### 3. merge_port_values"), "{md}");

        // Sans regroupement : l'ordre des scores, et aucun en-tête.
        let flat = render_results_with(&results, 300, false);
        assert!(!flat.contains("matches"), "{flat}");
        let order: Vec<&str> = flat.lines().filter(|l| l.starts_with("### ")).collect();
        assert!(order[1].starts_with("### 2. merge_port_values"), "{flat}");
    }

    #[test]
    fn a_neighbour_carries_its_link_too() {
        let mut r = scope("take", "PortValue", 120, 140, 0.8);
        r.other_children = Some(vec![ChildSummary {
            uuid: "u".into(),
            entity: "File".into(),
            relation: "DEFINED_IN".into(),
            data: data(&[
                ("path", CypherValue::String("src/dataflow/port.rs".into())),
                ("language", CypherValue::String("rust".into())),
                ("lines_of_code", CypherValue::Int(313)),
                ("cursor", CypherValue::Null),
            ]),
        }]);
        let md = render_results_markdown(&[r], 300);
        // Le voisin va dans le graphe de dépendances, groupé par relation.
        assert!(md.contains("## Dependency Graph"), "{md}");
        assert!(md.contains("└── [DEFINED_IN]"), "{md}");
        assert!(md.contains("└── src/dataflow/port.rs (File)"), "{md}");
        assert!(md.contains("lines_of_code=313"), "{md}");
        assert!(!md.contains("cursor"), "un champ nul ne se rend pas : {md}");
        // Le chemin n'est pas récité deux fois quand il **est** le titre.
        assert_eq!(md.matches("src/dataflow/port.rs").count(), 1, "{md}");
    }

    #[test]
    fn a_snippet_is_bounded_and_single_line() {
        let mut r = scope("take", "PortValue", 1, 2, 0.5);
        r.chunk = Some(ChunkInfo {
            uuid: "c".into(),
            text: format!("ligne une\nligne deux{}", "x".repeat(500)),
            index: 0,
            score: 0.5,
            start_line: 1,
            end_line: 2,
            start_char: 0,
            end_char: 0,
        });
        let md = render_results_markdown(&[r], 40);
        let quoted = md.lines().find(|l| l.trim_start().starts_with("> ")).expect("un extrait");
        let _ = &md;
        assert!(quoted.chars().count() <= 50, "{quoted}");
        assert!(quoted.ends_with('…'), "{quoted}");
    }

    #[test]
    fn nothing_found_says_so() {
        assert_eq!(render_results_markdown(&[], 300), "**No results.**");
    }

    /// **Le gabarit est la forme, et il est remplaçable.**
    ///
    /// C'est la seule chose que ce test vérifie, et c'est celle qui compte :
    /// la même vue rend trois surfaces différentes sans qu'une ligne de Rust
    /// ne change. Jusqu'au 27 août 2026 la fiche était un `format!` — pour la
    /// bouger il fallait recompiler le moteur.
    #[test]
    fn the_same_results_render_three_ways_through_three_templates() {
        let results = vec![scope("take", "PortValue", 120, 140, 0.81)];
        let view = build_view(&results, 300, true, &PathLens::default());

        let defaut = render_view(&view, DEFAULT_TEMPLATE).unwrap();
        assert!(defaut.contains("### 1. PortValue::take ★ 0.81"), "{defaut}");

        // Le compact : une ligne, trois fois moins cher.
        let compact = render_view(&view, COMPACT_TEMPLATE).unwrap();
        assert!(compact.contains("1. `PortValue::take` — Scope · 0.81 · port.rs:120-140"), "{compact}");
        assert!(compact.lines().count() < defaut.lines().count(), "{compact}");

        // Et n'importe quel gabarit écrit à la main.
        let mien = render_view(&view, "{% for r in results %}{{ r.name }}@{{ r.location }}{% endfor %}").unwrap();
        assert_eq!(mien, "take@port.rs:120-140");
    }

    /// Les trois façons de nommer un gabarit, et l'erreur quand il n'en est
    /// aucune — dite au montage du graphe, pas au milieu d'un tour d'agent.
    #[test]
    fn a_template_is_a_name_a_source_or_a_file() {
        assert_eq!(resolve_template("default").unwrap(), DEFAULT_TEMPLATE);
        assert_eq!(resolve_template("compact").unwrap(), COMPACT_TEMPLATE);
        assert_eq!(resolve_template("{{ count }}").unwrap(), "{{ count }}");

        let e = resolve_template("gabarit-qui-n-existe-pas").unwrap_err();
        assert!(e.contains("default, compact"), "{e}");

        // **Un nom, pas un chemin.** Un graphe peut être écrit par un modèle :
        // sans cette règle, `template=` serait une lecture de fichier
        // arbitraire, rendue au modèle, hors du domaine de travail.
        for tentative in ["/etc/passwd", "../../secret", "a/b"] {
            let e = resolve_template(tentative).unwrap_err();
            assert!(e.contains("un nom, pas un chemin"), "{tentative} : {e}");
        }

        // Un gabarit posé dans le répertoire prévu se charge par son nom.
        let dir = std::env::temp_dir().join("rag3weaver-gabarits-test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("mien.md.jinja"), "{{ count }} trouvés").unwrap();
        std::env::set_var(TEMPLATES_DIR_ENV, &dir);
        assert_eq!(resolve_template("mien").unwrap(), "{{ count }} trouvés");
        std::env::remove_var(TEMPLATES_DIR_ENV);

        let mut node = RenderResultsNodeFactory
            .create("render", &serde_json::json!({"template": "gabarit-qui-n-existe-pas"}));
        assert!(node.is_err(), "un gabarit introuvable refuse de se monter");
        node = RenderResultsNodeFactory.create("render", &serde_json::json!({"template": "compact"}));
        assert!(node.is_ok());
    }

    /// **La fiche du code, comme la maquette d'origine la voulait** : le nom
    /// et le type, le lieu copiable, la signature, et la phrase qui dit ce que
    /// ça fait. C'est cette dernière qui manquait — `docstring` ne remontait
    /// pas dans les `return_fields` de `Scope`, donc la ligne `📝` n'existait
    /// nulle part.
    #[test]
    fn a_code_result_reads_like_the_original_mockup() {
        let mut r = scope("merge_port_values", "", 192, 228, 0.0098);
        let d = r.data.as_mut().unwrap();
        d.insert("scope_type".into(), CypherValue::String("function".into()));
        d.insert("signature".into(), CypherValue::String("fn merge_port_values(a: PortValue, b: PortValue)".into()));
        d.insert("docstring".into(), CypherValue::String("Fusionne deux valeurs arrivant sur le même port.".into()));

        let md = render_results_markdown(&[r], 300);
        assert!(md.contains("### 1. merge_port_values (function) ★"), "{md}");
        assert!(md.contains("📍 `port.rs:192-228`"), "{md}");
        assert!(md.contains("🔹 `fn merge_port_values(a: PortValue, b: PortValue)`"), "{md}");
        assert!(md.contains("📝 Fusionne deux valeurs arrivant sur le même port."), "{md}");
        // Promus dans la fiche, donc absents de la liste des colonnes brutes.
        assert!(!md.contains("docstring=") && !md.contains("signature="), "{md}");
    }

    /// Un extrait qui répète la signature ne dit rien de plus, et il le dit
    /// sur deux lignes. C'est le cas de tout scope d'une ligne : sa signature
    /// **est** son contenu.
    #[test]
    fn a_snippet_that_repeats_the_signature_is_dropped() {
        let signature = "fn merge_port_values(a: PortValue, b: PortValue)";
        let mut r = scope("merge_port_values", "", 192, 228, 0.5);
        r.data.as_mut().unwrap().insert("signature".into(), CypherValue::String(signature.into()));
        r.chunk = Some(ChunkInfo {
            uuid: "c".into(),
            text: signature.into(),
            index: 0,
            score: 0.5,
            start_line: 192,
            end_line: 228,
            start_char: 0,
            end_char: 0,
        });
        let md = render_results_markdown(&[r], 300);
        assert_eq!(md.matches(signature).count(), 1, "une seule fois, pas deux :\n{md}");
        assert!(!md.contains("> fn merge_port_values"), "{md}");
    }

    /// **L'étoile doit distinguer.** Un score RRF vaut `poids / (60 + rang)` :
    /// à deux décimales, quatre résultats sortaient tous à `★ 0.01`, et un
    /// modèle qui lit quatre fois le même nombre n'apprend rien.
    #[test]
    fn the_star_gains_precision_until_it_separates() {
        let scores = [0.00983, 0.00641, 0.00632, 0.00625];
        let results: Vec<_> = scores
            .iter()
            .enumerate()
            .map(|(i, s)| scope(&format!("f{i}"), "", 1 + i as i64, 2 + i as i64, *s))
            .collect();
        let md = render_results_with(&results, 300, false);
        assert!(md.contains("★ 0.0098"), "{md}");
        assert!(md.contains("★ 0.0064") && md.contains("★ 0.0063"), "{md}");
        assert!(!md.contains("★ 0.01"), "quatre fois le même nombre n'apprend rien :\n{md}");

        // Et on ne dépense pas de décimales quand deux suffisent : la
        // précision est un remède, pas un défaut.
        let bm25: Vec<_> = [14.19, 12.95, 8.32]
            .iter()
            .enumerate()
            .map(|(i, s)| scope(&format!("g{i}"), "", 1 + i as i64, 2 + i as i64, *s))
            .collect();
        let md = render_results_with(&bm25, 300, false);
        assert!(md.contains("★ 14.19") && md.contains("★ 8.32"), "{md}");
    }

    /// La requête, quand elle est branchée, devient l'en-tête de la fiche.
    #[test]
    fn the_header_says_what_was_asked_and_where() {
        let mut view = build_view(&[scope("take", "PortValue", 1, 2, 0.5)], 300, true, &PathLens::default());
        view.query = Some("comment un port rend sa valeur".into());
        view.target = Some("Scope".into());
        let md = render_view(&view, DEFAULT_TEMPLATE).unwrap();
        assert!(md.starts_with("# Search: \"comment un port rend sa valeur\" — `Scope`"), "{md}");
    }

    /// Le décompte par type sort du `scope_type` quand il existe, de l'entité
    /// sinon — et il est trié.
    #[test]
    fn the_summary_counts_by_kind() {
        let mut a = scope("take", "PortValue", 1, 2, 0.9);
        a.data.as_mut().unwrap().insert("scope_type".into(), CypherValue::String("function".into()));
        let mut b = scope("downcast", "PortValue", 3, 4, 0.8);
        b.data.as_mut().unwrap().insert("scope_type".into(), CypherValue::String("function".into()));
        let mut c = scope("PortValue", "", 5, 6, 0.7);
        c.data.as_mut().unwrap().insert("scope_type".into(), CypherValue::String("struct".into()));

        let md = render_results_markdown(&[a, b, c], 300);
        assert!(md.contains("| function | 2 |"), "{md}");
        assert!(md.contains("| struct | 1 |"), "{md}");
        // Le type est dans la fiche, plus dans la liste des colonnes brutes.
        assert!(md.contains("(function) ★"), "{md}");
        assert!(!md.contains("scope_type="), "{md}");
    }
}
