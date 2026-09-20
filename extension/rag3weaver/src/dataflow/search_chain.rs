//! Typed search verbs. Compilation is pure and independent of MCP/backend hosts.
//! The resulting definition runs on the existing dataflow runtime and services.
use super::generic_search_nodes::DuplicatePolicy;
use super::{
    build_definition, DataflowOutput, DataflowRuntime, EdgeDef, GraphDefinition, NodeDef,
    NodeRegistry, NodeTypePolicy, ServiceRegistry,
};
use crate::{
    config::{EntityConfig, RelationDef},
    filter::FilterCondition,
    search::{FusionStrategy, SearchSignals},
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchProgram {
    #[serde(default)]
    pub branches: BTreeMap<String, Chain>,
    pub pipeline: Chain,
}
/// Each chain starts with search/select/from/fuse, followed by transformations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Chain(pub Vec<Verb>);
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verb {
    Search(Search),
    Select(Selection),
    From(String),
    Follow(Follow),
    Where(FilterCondition),
    Within(String),
    Fuse(Fusion),
    Page(Page),
    Render(Render),
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Search {
    pub entity: String,
    pub query: String,
    /// Candidate budget per search source, deliberately required (not a page).
    pub candidates: usize,
    #[serde(default = "hybrid")]
    pub signals: SearchSignals,
    #[serde(default)]
    pub filter: Option<FilterCondition>,
}
fn hybrid() -> SearchSignals {
    SearchSignals::BM25 | SearchSignals::VECTOR
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    pub entity: String,
    pub filter: FilterCondition,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Follow {
    pub relation: String,
    pub direction: Direction,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Incoming,
    Outgoing,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fusion {
    /// Keys are reusable branch names; values are finite, positive weights.
    pub inputs: BTreeMap<String, f64>,
    #[serde(default)]
    pub strategy: FusionStrategy,
    #[serde(default)]
    pub duplicates: DuplicatePolicy,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Page {
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Render {
    pub template: String,
}
impl Chain {
    pub fn search(search: Search) -> Self {
        Self(vec![Verb::Search(search)])
    }
    pub fn select(entity: impl Into<String>, filter: FilterCondition) -> Self {
        Self(vec![Verb::Select(Selection {
            entity: entity.into(),
            filter,
        })])
    }
    pub fn from(branch: impl Into<String>) -> Self {
        Self(vec![Verb::From(branch.into())])
    }
    pub fn fuse(fusion: Fusion) -> Self {
        Self(vec![Verb::Fuse(fusion)])
    }
    pub fn follow(mut self, relation: impl Into<String>, direction: Direction) -> Self {
        self.0.push(Verb::Follow(Follow {
            relation: relation.into(),
            direction,
        }));
        self
    }
    pub fn filter(mut self, filter: FilterCondition) -> Self {
        self.0.push(Verb::Where(filter));
        self
    }
    pub fn within(mut self, branch: impl Into<String>) -> Self {
        self.0.push(Verb::Within(branch.into()));
        self
    }
    pub fn page(mut self, limit: usize, offset: usize) -> Self {
        self.0.push(Verb::Page(Page { limit, offset }));
        self
    }
    pub fn render(mut self, template: impl Into<String>) -> Self {
        self.0.push(Verb::Render(Render {
            template: template.into(),
        }));
        self
    }
}
/// Declared entity schemas and relation endpoints, with no connection required.
pub struct SearchSchema<'a> {
    pub entities: &'a HashMap<String, EntityConfig>,
    pub relations: &'a HashMap<String, RelationDef>,
}
#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    pub at: String,
    pub message: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct SearchPlan {
    pub graph: GraphDefinition,
    pub result_node: String,
    pub entity: String,
    /// True even if fewer rows than the candidate budget were returned: ANN
    /// recall and downstream relation/filter coverage are not proven exhaustive.
    pub candidate_bounded: bool,
    pub diagnostics: Vec<Diagnostic>,
    /// Search metadata remains exposed, including runtime partial/pending flags.
    pub metadata_nodes: Vec<String>,
}
/// A portable GraphTool template plus typed arguments. User text never becomes
/// Mermaid syntax, and nested JSON retains its types during instantiation.
#[derive(Debug, Clone, Serialize)]
pub struct SearchTemplate {
    pub mermaid: String,
    pub arguments: Value,
}
impl SearchPlan {
    pub fn template(&self) -> SearchTemplate {
        let mut header = format!("%% tool: search_program\n%% description: Compiled search chain\n%% result: {}.results\n", self.result_node);
        let mut body = String::from("graph LR\n");
        let mut arguments = serde_json::Map::new();
        for node in &self.graph.nodes {
            let mut params = vec![];
            if let Some(config) = node.config.as_object() {
                for (key, value) in config {
                    let name = format!("p{}", arguments.len());
                    let ty = match value {
                        Value::String(_) => "string",
                        Value::Bool(_) => "bool",
                        Value::Number(n) if n.is_i64() || n.is_u64() => "int",
                        Value::Number(_) => "float",
                        _ => "json",
                    };
                    header.push_str(&format!("%% param: {name} {ty}! -- {}.{key}\n", node.name));
                    params.push(format!("{key}=${name}"));
                    arguments.insert(name, value.clone());
                }
            }
            body.push_str(&format!(
                " {}[\"{}({})\"]\n",
                node.name,
                node.node_type,
                params.join(", ")
            ));
        }
        for edge in &self.graph.edges {
            body.push_str(&format!(
                " {} -->|{}:{}| {}\n",
                edge.from_node, edge.from_port, edge.to_port, edge.to_node
            ));
        }
        SearchTemplate {
            mermaid: header + body.as_str(),
            arguments: Value::Object(arguments),
        }
    }
    /// No alternative executor: build and run exactly the exported DAG.
    pub fn execute(
        &self,
        nodes: &NodeRegistry,
        services: ServiceRegistry,
    ) -> Result<DataflowOutput, String> {
        let mut graph =
            build_definition(&self.graph, nodes, &policy()).map_err(|e| e.to_string())?;
        DataflowRuntime::with_services(100, services).execute(&mut graph)
    }
}
const NODES: &[&str] = &[
    "SearchSourceNode",
    "BM25SearchNode",
    "VectorSearchNode",
    "SparseSearchNode",
    "FuseResultsNode",
    "ResolveParentNode",
    "SelectRecordsNode",
    "FetchRelatedNode",
    "RelatedResultsNode",
    "IntersectResultsNode",
    "LabelResultsNode",
    "PaginateNode",
    "RenderResultsNode",
];
fn policy() -> NodeTypePolicy {
    NodeTypePolicy::only(NODES.iter().copied())
}
#[derive(Clone)]
struct Set {
    node: String,
    entity: String,
    bounded: bool,
}
struct Compiler<'a, 's> {
    program: &'a SearchProgram,
    schema: &'a SearchSchema<'s>,
    graph: GraphDefinition,
    cache: HashMap<String, Set>,
    visiting: HashSet<String>,
    diagnostics: Vec<Diagnostic>,
    metadata_nodes: Vec<String>,
}
impl SearchProgram {
    pub fn compile(
        &self,
        schema: &SearchSchema<'_>,
        nodes: &NodeRegistry,
    ) -> Result<SearchPlan, String> {
        if self.branches.len() > 32 {
            return Err("at most 32 branches".into());
        }
        let mut c = Compiler {
            program: self,
            schema,
            graph: GraphDefinition {
                nodes: vec![],
                edges: vec![],
            },
            cache: HashMap::new(),
            visiting: HashSet::new(),
            diagnostics: vec![],
            metadata_nodes: vec![],
        };
        let result = c.chain(&self.pipeline, "pipeline", true)?;
        let unused: Vec<_> = self
            .branches
            .keys()
            .filter(|k| !c.cache.contains_key(*k))
            .collect();
        if !unused.is_empty() {
            return Err(format!("unused branches: {unused:?}"));
        }
        if c.graph.nodes.len() > 256 || c.graph.edges.len() > 1024 {
            return Err("compiled search exceeds 256 nodes / 1024 edges".into());
        }
        build_definition(&c.graph, nodes, &policy()).map_err(|e| e.to_string())?;
        Ok(SearchPlan {
            graph: c.graph,
            result_node: result.node,
            entity: result.entity,
            candidate_bounded: result.bounded,
            diagnostics: c.diagnostics,
            metadata_nodes: c.metadata_nodes,
        })
    }
}
impl Compiler<'_, '_> {
    fn node(&mut self, kind: &str, config: Value) -> String {
        let name = format!("q{}", self.graph.nodes.len());
        self.graph.nodes.push(NodeDef {
            name: name.clone(),
            node_type: kind.into(),
            config,
        });
        name
    }
    fn edge(&mut self, from: &str, port: &str, to: &str, input: &str) {
        self.graph.edges.push(EdgeDef {
            from_node: from.into(),
            from_port: port.into(),
            to_node: to.into(),
            to_port: input.into(),
        });
    }
    fn note(&mut self, at: &str, message: &str) {
        self.diagnostics.push(Diagnostic {
            at: at.into(),
            message: message.into(),
        });
    }
    fn entity(&self, name: &str) -> Result<(), String> {
        if !self.schema.entities.contains_key(name) {
            return Err(format!(
                "unknown entity {name}; choose from {:?}",
                self.schema.entities.keys().collect::<Vec<_>>()
            ));
        }
        crate::schema::validate_identifier(name, "entity").map_err(|e| e.to_string())
    }
    fn filter(&self, entity: &str, condition: &FilterCondition) -> Result<FilterCondition, String> {
        self.entity(entity)?;
        // Legacy Field predicates can imply joins. Chains require explicit
        // follow instead, so every field is a locally validated typed path.
        fn canonical(c: &FilterCondition, depth: usize) -> Result<FilterCondition, String> {
            if depth > 32 {
                return Err("filter nesting exceeds 32".into());
            }
            Ok(match c {
                FilterCondition::Field { key, value } => FilterCondition::Path {
                    path: vec![key.clone()],
                    value: value.clone(),
                },
                FilterCondition::Nested { path, condition } => FilterCondition::Nested {
                    path: path.clone(),
                    condition: Box::new(canonical(condition, depth + 1)?),
                },
                FilterCondition::Must(cs) => FilterCondition::Must(
                    cs.iter()
                        .map(|c| canonical(c, depth + 1))
                        .collect::<Result<_, _>>()?,
                ),
                FilterCondition::Should(cs) => FilterCondition::Should(
                    cs.iter()
                        .map(|c| canonical(c, depth + 1))
                        .collect::<Result<_, _>>()?,
                ),
                FilterCondition::MustNot(cs) => FilterCondition::MustNot(
                    cs.iter()
                        .map(|c| canonical(c, depth + 1))
                        .collect::<Result<_, _>>()?,
                ),
                _ => c.clone(),
            })
        }
        let filter = canonical(condition, 0)?;
        crate::json_schema::validate_structured_filter(
            &filter,
            &self.schema.entities[entity].fields,
        )?;
        Ok(filter)
    }
    fn select(&mut self, selection: &Selection) -> Result<Set, String> {
        let filter = self.filter(&selection.entity, &selection.filter)?;
        let node = self.node(
            "SelectRecordsNode",
            json!({"entity":selection.entity,"filter":filter}),
        );
        Ok(Set {
            node,
            entity: selection.entity.clone(),
            bounded: false,
        })
    }
    fn branch(&mut self, name: &str) -> Result<Set, String> {
        if let Some(s) = self.cache.get(name) {
            return Ok(s.clone());
        }
        if !self.visiting.insert(name.into()) {
            return Err(format!("cyclic branch reference: {name}"));
        }
        let chain = self
            .program
            .branches
            .get(name)
            .ok_or_else(|| format!("unknown branch {name}"))?
            .clone();
        let mut set = self.chain(&chain, &format!("branches.{name}"), false)?;
        let label = self.node("LabelResultsNode", json!({"signal":name}));
        self.edge(&set.node, "results", &label, "results");
        set.node = label;
        self.visiting.remove(name);
        self.cache.insert(name.into(), set.clone());
        Ok(set)
    }
    fn search(&mut self, search: &Search, at: &str) -> Result<Set, String> {
        self.entity(&search.entity)?;
        if search.candidates == 0 || search.candidates > 10000 {
            return Err("search.candidates must be in 1..=10000".into());
        }
        if search.signals == SearchSignals::NONE {
            return Err("search.signals must not be empty".into());
        }
        let filter = search
            .filter
            .as_ref()
            .map(|f| self.filter(&search.entity, f))
            .transpose()?;
        let source=self.node("SearchSourceNode",json!({"target_name":search.entity,"query":search.query,"options":{"limit":search.candidates,"signals":search.signals,"filter_condition":filter}}));
        self.metadata_nodes.push(source.clone());
        let fuse = self.node("FuseResultsNode", json!({}));
        self.edge(&source, "query", &fuse, "query");
        for (enabled, kind, signal) in [
            (search.signals.bm25(), "BM25SearchNode", "bm25"),
            (search.signals.vector(), "VectorSearchNode", "vector"),
            (search.signals.sparse(), "SparseSearchNode", "sparse"),
        ] {
            if enabled {
                let n = self.node(kind, json!({}));
                self.edge(&source, "query", &n, "query");
                self.edge(&n, "results", &fuse, signal);
                self.metadata_nodes.push(n);
            }
        }
        let cap = self.node("PaginateNode", json!({}));
        self.edge(&source, "query", &cap, "query");
        self.edge(&fuse, "results", &cap, "results");
        let resolve = self.node("ResolveParentNode", json!({}));
        self.edge(&source, "query", &resolve, "query");
        self.edge(&cap, "results", &resolve, "results");
        self.note(at,"Candidate-bounded search; downstream filtering/traversal cannot establish exhaustive coverage. Increase candidates or use exact select for exhaustive predicates.");
        Ok(Set {
            node: resolve,
            entity: search.entity.clone(),
            bounded: true,
        })
    }
    fn chain(&mut self, chain: &Chain, path: &str, terminal: bool) -> Result<Set, String> {
        if chain.0.is_empty() || chain.0.len() > 64 {
            return Err(format!("{path}: expected 1..=64 verbs"));
        }
        let mut verbs = chain.0.clone();
        let mut pushed_filters = 0;
        // Only adjacent source filters are safely pushed before candidate selection.
        // Never move across follow/fuse/page or a dynamic set intersection.
        if matches!(verbs[0], Verb::Search(_) | Verb::Select(_)) {
            let mut count = 0;
            let mut filters = vec![];
            for v in verbs.iter().skip(1) {
                if let Verb::Where(f) = v {
                    filters.push(f.clone());
                    count += 1;
                } else {
                    break;
                }
            }
            if count > 0 {
                match &mut verbs[0] {
                    Verb::Search(s) => {
                        if let Some(f) = s.filter.take() {
                            filters.insert(0, f);
                        }
                        s.filter = Some(FilterCondition::Must(filters));
                    }
                    Verb::Select(s) => {
                        filters.insert(0, s.filter.clone());
                        s.filter = FilterCondition::Must(filters);
                    }
                    _ => unreachable!(),
                }
                verbs.drain(1..=count);
                pushed_filters = count;
                self.note(
                    path,
                    "Adjacent where filters applied at the source, before candidate selection.",
                );
            }
        }
        let first = (|| match &verbs[0] {
            Verb::Search(s) => self.search(s, path),
            Verb::Select(s) => self.select(s),
            Verb::From(name) => self.branch(name),
            Verb::Fuse(f) => {
                if f.inputs.is_empty() {
                    return Err("fuse requires at least one branch".into());
                }
                let mut inputs = vec![];
                for (name, weight) in &f.inputs {
                    if !weight.is_finite() || *weight <= 0.0 {
                        return Err(format!(
                            "fuse weight for {name} must be finite and positive"
                        ));
                    }
                    inputs.push(self.branch(name)?);
                }
                let entity = inputs[0].entity.clone();
                if inputs.iter().any(|s| s.entity != entity) {
                    return Err("fuse requires the same entity type in every branch; follow relations first".into());
                }
                let node = self.node(
                    "FuseResultsNode",
                    json!({"weights":f.inputs,"strategy":f.strategy,"duplicates":f.duplicates}),
                );
                for input in &inputs {
                    self.edge(&input.node, "results", &node, "signals");
                }
                Ok(Set {
                    node,
                    entity,
                    bounded: inputs.iter().any(|s| s.bounded),
                })
            }
            _ => Err("chain must start with search, select, from or fuse".into()),
        })();
        let mut set = first.map_err(|e| format!("{path}[0]: {e}"))?;
        let mut paged = false;
        let mut rendered = false;
        for (index, verb) in verbs.iter().enumerate().skip(1) {
            let at = format!("{path}[{}]", index + pushed_filters);
            let step = (|| -> Result<(), String> {
                if rendered {
                    return Err("render must be last".into());
                }
                if paged && !matches!(verb, Verb::Render(_)) {
                    return Err(
                        "only render may follow page; move restrictions before pagination".into(),
                    );
                }
                match verb {
                    Verb::Follow(f)=>{
                        crate::schema::validate_identifier(&f.relation,"relation").map_err(|e|e.to_string())?;
                        let rel=self.schema.relations.get(&f.relation).ok_or_else(||format!("unknown relation {}",f.relation))?;
                        let (from,to)=match f.direction{Direction::Incoming=>(&rel.to,&rel.from),Direction::Outgoing=>(&rel.from,&rel.to)};
                        if &set.entity!=from{return Err(format!("follow {} expects {from}, got {}",f.relation,set.entity));}
                        self.entity(to)?;
                        let entity=to.clone();
                        let fetch=self.node("FetchRelatedNode",json!({"relation":f.relation,"direction":match f.direction{Direction::Incoming=>"Incoming",Direction::Outgoing=>"Outgoing"},"limit":0,"source_entity":set.entity}));
                        let result=self.node("RelatedResultsNode",json!({"signal":format!("follow_{}",self.graph.nodes.len())}));
                        self.edge(&set.node,"results",&fetch,"results");self.edge(&set.node,"results",&result,"parents");self.edge(&fetch,"children",&result,"children");set.node=result;set.entity=entity;
                    }
                    Verb::Where(filter)=>{
                        let allowed=self.select(&Selection{entity:set.entity.clone(),filter:filter.clone()})?;
                        let node=self.node("IntersectResultsNode",json!({}));self.edge(&set.node,"results",&node,"results");self.edge(&allowed.node,"results",&node,"allowed");set.node=node;
                        if set.bounded{self.note(&at,"Post-candidate where: matching objects outside upstream candidate sets are not recovered.");}
                    }
                    Verb::Within(name)=>{
                        let allowed=self.branch(name)?;
                        if allowed.entity!=set.entity{return Err(format!("within requires the same entity identity: {} vs {}; use an explicit relation (no name/UUID cross-entity join)",set.entity,allowed.entity));}
                        let node=self.node("IntersectResultsNode",json!({}));self.edge(&set.node,"results",&node,"results");self.edge(&allowed.node,"results",&node,"allowed");set.node=node;set.bounded|=allowed.bounded;
                        if set.bounded{self.note(&at,"Intersection includes a candidate-bounded input; membership outside that input is unknown.");}
                    }
                    Verb::Page(page)=>{
                        if !terminal{return Err("page is only allowed in the final pipeline".into());}
                        if page.limit==0 || page.limit>10000 || page.offset.checked_add(page.limit).is_none(){return Err("invalid page limit/offset".into());}
                        let source=self.node("SearchSourceNode",json!({"target_name":set.entity,"query":"","options":{"limit":page.limit,"offset":page.offset,"signals":SearchSignals::NONE}}));
                        let node=self.node("PaginateNode",json!({}));self.edge(&source,"query",&node,"query");self.edge(&set.node,"results",&node,"results");set.node=node;paged=true;
                    }
                    Verb::Render(r)=>{
                        if !terminal{return Err("render is only allowed in the final pipeline".into());}
                        if r.template.is_empty(){return Err("render.template must not be empty".into());}
                        let node=self.node("RenderResultsNode",json!({"template":r.template}));self.edge(&set.node,"results",&node,"results");set.node=node;rendered=true;
                    }
                    _=>return Err("search/select/from/fuse are source verbs; use named branches to compose them".into()),
                }
                Ok(())
            })();
            step.map_err(|e| format!("{at}: {e}"))?;
        }
        if terminal && !rendered {
            // Expose a terminal port even when the output reuses a shared branch.
            let node = self.node("LabelResultsNode", json!({}));
            self.edge(&set.node, "results", &node, "results");
            set.node = node;
        }
        Ok(set)
    }
}

#[cfg(test)]
mod tests {
    use super::super::{builtin_graph_tools, GraphTool, UnifiedResult};
    use super::*;
    use crate::{
        catalog::Catalog,
        config::CatalogConfig,
        connection::{CallbackConnection, CypherValue, QueryResult},
        embedder::HashEmbedder,
        json_schema::JsonSchemaMapping,
    };
    use std::sync::{Arc, Mutex};

    fn schema() -> (HashMap<String, EntityConfig>, HashMap<String, RelationDef>) {
        let mut mapping=JsonSchemaMapping::from_schema(&json!({"type":"object","properties":{"text":{"type":"string"},"owned":{"type":"boolean"},"key":{"type":"string"}}})).unwrap();
        mapping.fields.get_mut("text").unwrap().is_content = true;
        let entity = EntityConfig {
            fields: mapping.fields,
            hashsafe: Some(vec!["key".into()]),
            signals: SearchSignals::NONE,
            ..Default::default()
        };
        (
            [("Card".into(), entity.clone()), ("Ability".into(), entity)].into(),
            [(
                "HasAbility".into(),
                RelationDef {
                    from: "Card".into(),
                    to: "Ability".into(),
                    properties: None,
                },
            )]
            .into(),
        )
    }
    fn program() -> SearchProgram {
        serde_json::from_value(json!({"branches":{
            "abilities":[{"select":{"entity":"Ability","filter":{"must":[]}}}],
            "indirect":[{"from":"abilities"},{"follow":{"relation":"HasAbility","direction":"incoming"}}],
            "direct":[{"select":{"entity":"Card","filter":{"field":{"key":"text","value":[{"op":"contains","value":"graveyard"}]}}}}],
            "owned":[{"select":{"entity":"Card","filter":{"field":{"key":"owned","value":true}}}}]
        },"pipeline":[{"fuse":{"inputs":{"indirect":2,"direct":1}}},{"within":"owned"},{"page":{"limit":20}},{"render":{"template":"tree"}}]})).unwrap()
    }
    fn compile(p: &SearchProgram) -> Result<SearchPlan, String> {
        let (entities, relations) = schema();
        let (nodes, _) = builtin_graph_tools().unwrap();
        p.compile(
            &SearchSchema {
                entities: &entities,
                relations: &relations,
            },
            &nodes,
        )
    }
    fn services(empty: bool) -> ServiceRegistry {
        let card = |id: &str| {
            CypherValue::Map(
                [
                    ("_uuid".into(), CypherValue::String(id.into())),
                    (
                        "text".into(),
                        CypherValue::String("same card name, different identity".into()),
                    ),
                ]
                .into(),
            )
        };
        let conn = CallbackConnection::new(move |q, _| {
            if empty {
                return Ok(QueryResult::default());
            }
            let rows = if q.starts_with("MATCH (n:Ability)") {
                ["a", "b"].iter().map(|s| vec![card(s)]).collect()
            } else if q.starts_with("MATCH (n:Card)") {
                let ids = if q.contains("owned") {
                    vec!["c1", "c2"]
                } else {
                    vec!["c2", "c3"]
                };
                ids.iter().map(|s| vec![card(s)]).collect()
            } else if q.starts_with("UNWIND $uuids") {
                [("a", "c1"), ("a", "c3"), ("b", "c1"), ("b", "c2")]
                    .iter()
                    .map(|(a, c)| {
                        vec![
                            CypherValue::String((*a).into()),
                            CypherValue::String((*c).into()),
                            CypherValue::String("Card".into()),
                            card(c),
                        ]
                    })
                    .collect()
            } else {
                vec![]
            };
            Ok(QueryResult {
                columns: vec![],
                rows,
            })
        });
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
        let (entities, _) = schema();
        for (n, c) in entities {
            catalog.register_entity(&n, c).unwrap();
        }
        let catalog = Arc::new(Mutex::new(catalog));
        let mut services = ServiceRegistry::new();
        catalog
            .lock()
            .unwrap()
            .register_search_services(&mut services);
        services.register("catalog", catalog);
        services
    }
    #[test]
    fn runs_real_nodes_without_any_mcp_and_matches_handwritten_dag() {
        let p = program();
        let plan = compile(&p).unwrap();
        assert_eq!(plan.entity, "Card");
        assert!(!plan.candidate_bounded);
        let (nodes, _) = builtin_graph_tools().unwrap();
        let out = plan.execute(&nodes, services(false)).unwrap();
        let rows = out
            .get(&plan.result_node, "results")
            .unwrap()
            .downcast::<Vec<UnifiedResult>>()
            .unwrap();
        assert_eq!(
            rows.iter().map(|r| r.uuid.as_str()).collect::<Vec<_>>(),
            vec!["c2", "c1"]
        );
        assert!(rows[0].score > rows[1].score);
        assert!(rows[1]
            .matched_children
            .as_ref()
            .unwrap()
            .iter()
            .any(|r| r.uuid == "a"));
        assert!(rows[1]
            .matched_children
            .as_ref()
            .unwrap()
            .iter()
            .any(|r| r.uuid == "b"));
        assert!(out.get(&plan.result_node, "text").is_some());
        let manual=GraphTool::from_mermaid(r#"%% tool: reference
%% description: Handwritten composable search
%% param: all json! -- All records
%% param: text json! -- Direct text filter
%% param: owned json! -- Ownership filter
%% result: scoped.results
graph LR
 a["SelectRecordsNode(entity='Ability', filter=$all)"]
 al["LabelResultsNode(signal='abilities')"]
 links["FetchRelatedNode(relation='HasAbility', direction='Incoming', limit=0, source_entity='Ability')"]
 indirect["RelatedResultsNode(signal='indirect')"]
 direct["SelectRecordsNode(entity='Card', filter=$text)"]
 owned["SelectRecordsNode(entity='Card', filter=$owned)"]
 fused["FuseResultsNode(weights='indirect:2,direct:1')"]
 scoped["IntersectResultsNode"]
 a -->|results| al
 al -->|results| links
 al -->|results:parents| indirect
 links -->|children| indirect
 direct -->|results:signals| fused
 indirect -->|results:signals| fused
 fused -->|results| scoped
 owned -->|results:allowed| scoped
 "#).unwrap().instantiate(&json!({"all":{"must":[]},"text":{"path":{"path":["text"],"value":[{"op":"contains","value":"graveyard"}]}},"owned":{"path":{"path":["owned"],"value":true}}})).unwrap();
        let mut graph = build_definition(&manual, &nodes, &policy()).unwrap();
        let reference = DataflowRuntime::with_services(100, services(false))
            .execute(&mut graph)
            .unwrap();
        let expected = reference
            .get("scoped", "results")
            .unwrap()
            .downcast::<Vec<UnifiedResult>>()
            .unwrap();
        assert_eq!(
            serde_json::to_value(rows).unwrap(),
            serde_json::to_value(expected).unwrap()
        );
        // No duplicated scan of a branch referenced through another branch.
        assert_eq!(
            plan.graph
                .nodes
                .iter()
                .filter(|n| n.node_type == "SelectRecordsNode" && n.config["entity"] == "Ability")
                .count(),
            1
        );
    }
    #[test]
    fn empty_pipeline_results_and_keep_duplicates() {
        let (nodes, _) = builtin_graph_tools().unwrap();
        let mut p = program();
        let plan = compile(&p).unwrap();
        let out = plan.execute(&nodes, services(true)).unwrap();
        assert!(out
            .get(&plan.result_node, "results")
            .unwrap()
            .downcast::<Vec<UnifiedResult>>()
            .unwrap()
            .is_empty());
        if let Verb::Fuse(f) = &mut p.pipeline.0[0] {
            f.duplicates = DuplicatePolicy::Keep;
        }
        let plan = compile(&p).unwrap();
        let out = plan.execute(&nodes, services(false)).unwrap();
        let rows = out
            .get(&plan.result_node, "results")
            .unwrap()
            .downcast::<Vec<UnifiedResult>>()
            .unwrap();
        assert_eq!(rows.iter().filter(|r| r.uuid == "c2").count(), 2);
        assert_eq!(rows.iter().filter(|r| r.uuid == "c1").count(), 1);
    }
    #[test]
    fn source_filters_precede_top_k_and_later_restrictions_are_disclosed() {
        let mut p = SearchProgram {
            branches: BTreeMap::new(),
            pipeline: Chain::search(Search {
                entity: "Card".into(),
                query: "graveyard".into(),
                candidates: 50,
                signals: hybrid(),
                filter: None,
            })
            .filter(serde_json::from_value(json!({"field":{"key":"owned","value":true}})).unwrap())
            .page(10, 0),
        };
        let plan = compile(&p).unwrap();
        assert!(plan.candidate_bounded);
        assert!(!plan
            .graph
            .nodes
            .iter()
            .any(|n| n.node_type == "IntersectResultsNode"));
        assert!(plan
            .graph
            .nodes
            .iter()
            .any(|n| n.node_type == "SearchSourceNode"
                && !n.config["options"]["filter_condition"].is_null()));
        let template = plan.template();
        let restored = GraphTool::from_mermaid(&template.mermaid)
            .unwrap()
            .instantiate(&template.arguments)
            .unwrap();
        assert_eq!(
            serde_json::to_value(restored).unwrap(),
            serde_json::to_value(&plan.graph).unwrap()
        );
        p.branches.insert(
            "all".into(),
            Chain::select("Card", FilterCondition::Must(vec![])),
        );
        p.pipeline.0.insert(2, Verb::Within("all".into()));
        assert!(compile(&p)
            .unwrap()
            .diagnostics
            .iter()
            .any(|d| d.message.contains("Intersection")));
    }
    #[test]
    fn rejects_invalid_types_cycles_fields_and_pagination_before_filters() {
        let mut p = program();
        p.pipeline.0.insert(1, Verb::Within("abilities".into()));
        assert!(compile(&p).unwrap_err().contains("same entity identity"));
        let mut p = program();
        p.branches
            .insert("abilities".into(), Chain::from("indirect"));
        assert!(compile(&p).unwrap_err().contains("cyclic"));
        let mut p = program();
        p.pipeline
            .0
            .insert(3, Verb::Where(FilterCondition::Must(vec![])));
        assert!(compile(&p)
            .unwrap_err()
            .contains("only render may follow page"));
        let p = SearchProgram {
            branches: BTreeMap::new(),
            pipeline: Chain::select(
                "Card",
                serde_json::from_value(json!({"field":{"key":"missing","value":true}})).unwrap(),
            ),
        };
        assert!(compile(&p).is_err());
        let mut p = program();
        if let Verb::Follow(f) = &mut p.branches.get_mut("indirect").unwrap().0[1] {
            f.direction = Direction::Outgoing;
        }
        assert!(compile(&p).unwrap_err().contains("expects Card"));
        let mut p = program();
        if let Verb::Fuse(f) = &mut p.pipeline.0[0] {
            f.inputs.insert("abilities".into(), 1.0);
        }
        assert!(compile(&p).unwrap_err().contains("same entity type"));
    }
    #[test]
    fn invalid_budgets_weights_and_unknown_keys_fail_closed() {
        let mut p = program();
        if let Verb::Fuse(f) = &mut p.pipeline.0[0] {
            f.inputs.insert("direct".into(), f64::NAN);
        }
        assert!(compile(&p).unwrap_err().contains("finite"));
        let p = SearchProgram {
            branches: BTreeMap::new(),
            pipeline: Chain::search(Search {
                entity: "Card".into(),
                query: "x".into(),
                candidates: 0,
                signals: hybrid(),
                filter: None,
            }),
        };
        assert!(compile(&p).is_err());
        assert!(serde_json::from_value::<SearchProgram>(json!({"pipeline":[{"search":{"entity":"Card","query":"x","candidates":5,"misspelled":true}}]})).is_err());
        assert!(
            serde_json::from_value::<SearchProgram>(json!({"pipeline":[{"delete":"Card"}]}))
                .is_err()
        );
    }
    #[test]
    fn template_roundtrip_preserves_literal_text_and_nested_types() {
        let mut p = program();
        p.branches.insert(
            "direct".into(),
            Chain::search(Search {
                entity: "Card".into(),
                query: "$p0 'quoted' \"double\" %% -->\nnext".into(),
                candidates: 40,
                signals: hybrid(),
                filter: None,
            }),
        );
        let plan = compile(&p).unwrap();
        let template = plan.template();
        assert!(!template.mermaid.contains("quoted"));
        let restored = GraphTool::from_mermaid(&template.mermaid)
            .unwrap()
            .instantiate(&template.arguments)
            .unwrap();
        assert_eq!(
            serde_json::to_value(restored).unwrap(),
            serde_json::to_value(plan.graph).unwrap()
        );
    }
    #[test]
    fn shared_branch_executes_once_and_terminal_output_preserves_signal() {
        let p = SearchProgram {
            branches: [(
                "owned".into(),
                Chain::select("Card", FilterCondition::Must(vec![])),
            )]
            .into(),
            pipeline: Chain::from("owned").within("owned"),
        };
        let plan = compile(&p).unwrap();
        assert_eq!(
            plan.graph
                .nodes
                .iter()
                .filter(|n| n.node_type == "SelectRecordsNode")
                .count(),
            1
        );
        let (nodes, _) = builtin_graph_tools().unwrap();
        let out = plan.execute(&nodes, services(false)).unwrap();
        let rows = out
            .get(&plan.result_node, "results")
            .unwrap()
            .downcast::<Vec<UnifiedResult>>()
            .unwrap();
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|r| r.signal.as_deref() == Some("owned")));
        let mut paged = p.clone();
        paged
            .branches
            .get_mut("owned")
            .unwrap()
            .0
            .push(Verb::Page(Page {
                limit: 1,
                offset: 0,
            }));
        assert!(compile(&paged).unwrap_err().contains("final pipeline"));
    }
}
