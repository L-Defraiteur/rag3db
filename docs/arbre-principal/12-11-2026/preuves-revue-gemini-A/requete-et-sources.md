Revue statique de code Rust. Cherche des omissions CONCRÈTES : complétude et canonicalisation des configurations entités/relations, recettes/gabarits/modèles ; liaison des faits au plan ; provenance déclarée vs stockage réellement observé ; plans immuables et contrat composite. Distingue défaut démontré, hypothèse dépendant d’un contrat absent et amélioration facultative. Cite fichier et lignes, scénario reproductible et gravité. N’invente pas de garantie de stockage : A est volontairement pur, alimenté par déclarations, sans apply ni producteur persistant. Un provider Absent est une assertion explicite. Ne réclame pas que A réalise la future migration durable. Maximum 6 constats argumentés, ou aucun défaut trouvé. Réponds en français.
Preuves existantes, pas à rejouer : ciblé 64/0/0 ; F5 5/5 ; minimal sans défaut ; lib2222/0/65 ; navigateur40/0/0 ; CLI5/0/0 ; attestation4ELF/native29/220gabarits. Passe composée avec incidents de vérificateur/environnement conservés, quiescence finale et événements mémoire nuls. A intégré6d4d1c882, revue parallèle sans barrière d’intégration.

SOURCE d2cb7f0636d099823f63de2cef674b630a46bc6f:extension/rag3weaver/src/dataflow/migrations/facts.rs
1: //! Binding of complete SUPPLIED declarations to fixed migration inputs.
2: //!
3: //! This module does not observe storage, establish an origin/generation, connect
4: //! a model or grant execution. Missing declarations are not empty declarations.
5: use std::collections::{BTreeMap, BTreeSet};
6: use serde::Serialize;
7: use crate::{backend::BackendRelation, config::{EntityConfig, RelationDef},
8:     embedding_storage::{EmbeddingModelEntry, StorageKind, validate_slug}, hash::content_hash,
9:     model_source::{Capability, ModelSource}, search::SearchSignals};
10: use super::plan::{MigrationPlanInputs, ResolvedMigrationPlan, StorageDescriptor, StorageGeneration};
11: 
12: const CONFIG_DOMAIN: &str = "declared-entity-configs:v1:";
13: const FACTS_DOMAIN: &str = "declared-migration-facts:v1:";
14: 
15: #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
16: pub enum FactsError {
17:     Incomplete(String), Invalid(String), UnresolvedRecipe(String),
18:     UnknownContract(String), Mismatch(String),
19: }
20: impl std::fmt::Display for FactsError {
21:     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{self:?}") }
22: }
23: impl std::error::Error for FactsError {}
24: 
25: /// Explicit provider knowledge: absent, complete definitions, or unavailable.
26: /// `Absent` is an assertion about the supplied declaration, not discovery.
27: #[derive(Debug, Clone, Serialize)]
28: pub enum ProviderFacts {
29:     Absent,
30:     Complete { name: String, entities: BTreeMap<String, EntityConfig>, relations: BTreeMap<String, RelationDef> },
31:     Unavailable { name: String },
32: }
33: 
34: #[derive(Debug, Clone, Serialize)]
35: pub struct RecipeFactsInput {
36:     pub id: String,
37:     /// Source bytes for non-inline derived templates, indexed by their spec.
38:     /// None is unknown; Some(empty) explicitly declares no such templates.
39:     pub resolved_templates: Option<BTreeMap<String, String>>,
40:     /// Complete declarations, already normalized from legacy aliases by caller.
41:     /// No environment resolution or builders are called here.
42:     pub model_sources: Option<BTreeMap<Capability, ModelSource>>,
43:     pub dense_models: Option<BTreeMap<String, EmbeddingModelEntry>>,
44: }
45: 
46: #[derive(Debug, Clone, Serialize)]
47: pub struct DeclaredSchemaInput {
48:     /// Effective EntityConfig values, including schema fields already merged.
49:     pub entities: Option<BTreeMap<String, EntityConfig>>,
50:     pub relations: Option<BTreeMap<String, BackendRelation>>,
51:     pub provider: ProviderFacts,
52:     /// Supplied final workspace signal overrides, never read from environment.
53:     pub index_signals: BTreeMap<String, SearchSignals>,
54:     pub recipe: RecipeFactsInput,
55: }
56: 
57: /// Immutable normalized facts, constructed through the existing schema validator.
58: #[derive(Debug, Clone, Serialize)]
59: pub struct DeclaredSchemaFacts {
60:     input: DeclaredSchemaInput,
61:     entities: BTreeMap<String, EntityConfig>,
62:     relations: BTreeMap<String, RelationDef>,
63:     config_fingerprint: String,
64: }
65: impl DeclaredSchemaFacts {
66:     pub fn from_complete_input(input: DeclaredSchemaInput) -> Result<Self, FactsError> {
67:         if input.entities.is_none() { return Err(FactsError::Incomplete("effective entity configs".into())); }
68:         if input.relations.is_none() { return Err(FactsError::Incomplete("relation definitions/policies".into())); }
69:         match &input.provider {
70:             ProviderFacts::Unavailable { name } => return Err(FactsError::Incomplete(format!("provider {name}"))),
71:             ProviderFacts::Complete { name, entities, relations } => {
72:                 if name.trim().is_empty() { return Err(FactsError::Invalid("empty provider name".into())); }
73:                 validate_finite(entities, relations)?;
74:             },
75:             ProviderFacts::Absent => {},
76:         }
77:         let entities = input.entities.as_ref().expect("checked above");
78:         let relations = input.relations.as_ref().expect("checked above");
79:         let defs = relations.iter().map(|(n,r)| (n.clone(),r.def.clone())).collect();
80:         validate_finite(entities, &defs)?;
81:         let templates = input.recipe.resolved_templates.as_ref()
82:             .ok_or_else(|| FactsError::Incomplete("resolved recipe templates".into()))?;
83:         let model_sources = input.recipe.model_sources.as_ref()
84:             .ok_or_else(|| FactsError::Incomplete("effective model declarations".into()))?;
85:         let models = input.recipe.dense_models.as_ref()
86:             .ok_or_else(|| FactsError::Incomplete("dense model registry".into()))?;
87:         if input.recipe.id.trim().is_empty() { return Err(FactsError::Invalid("empty recipe identifier".into())); }
88:         let (entities, relations) = Self::compose_effective(&input).map_err(FactsError::Invalid)?;
89:         validate_finite(&entities, &relations)?;
90:         let mut required_templates = BTreeSet::new();
91:         for config in entities.values() {
92:             if let Some(derived) = &config.derived {
93:                 for spec in derived.render.values() {
94:                     // This is the inline rule of the existing template resolver;
95:                     // named bytes must be provided instead of loading a file.
96:                     if !spec.contains("{{") && !spec.contains("{%") {
97:                         required_templates.insert(spec.clone());
98:                         if !templates.contains_key(spec) { return Err(FactsError::UnresolvedRecipe(spec.clone())); }
99:                     }
100:                 }
101:             }
102:         }
103:         if templates.keys().any(|k| !required_templates.contains(k)) {
104:             return Err(FactsError::Invalid("unreferenced resolved template in recipe".into()));
105:         }
106:         let mut slugs = BTreeSet::new();
107:         let mut legacy = 0;
108:         for (name, model) in models {
109:             if name != &model.name || name.trim().is_empty() || model.dim == 0 || !slugs.insert(model.slug()) {
110:                 return Err(FactsError::Invalid("invalid dense registry or colliding model slugs".into()));
111:             }
112:             validate_slug(&model.slug()).map_err(FactsError::Invalid)?;
113:             if model.storage == StorageKind::Legacy { legacy += 1; }
114:         }
115:         if legacy > 1 { return Err(FactsError::Invalid("multiple legacy dense columns".into())); }
116:         if entities.values().any(|c| c.signals.vector()) && models.is_empty() {
117:             return Err(FactsError::Incomplete("dense registry for vector signals".into()));
118:         }
119:         for source in model_sources.values() {
120:             if source.model.trim().is_empty() || source.dimensions == Some(0) || source.context_tokens == Some(0) {
121:                 return Err(FactsError::Invalid("invalid effective model declaration".into()));
122:             }
123:         }
124:         if let Some(embed) = model_sources.get(&Capability::Embed) {
125:             let entry = models.get(&embed.model).ok_or_else(|| FactsError::Incomplete("declared embed model missing from registry".into()))?;
126:             if embed.dimensions.is_some_and(|d| d != entry.dim) {
127:                 return Err(FactsError::Invalid("embed model dimension contradicts registry".into()));
128:             }
129:         }
130:         let config_fingerprint = digest(CONFIG_DOMAIN, &entities)?;
131:         Ok(Self { input, entities, relations, config_fingerprint })
132:     }
133:     pub fn entities(&self) -> &BTreeMap<String, EntityConfig> { &self.entities }
134:     pub fn relations(&self) -> &BTreeMap<String, RelationDef> { &self.relations }
135:     pub fn config_fingerprint(&self) -> &str { &self.config_fingerprint }
136: }
137: 
138: /// All facts are declared inputs. No source is represented as observed/persisted.
139: #[derive(Debug, Clone, Serialize)]
140: pub struct DeclaredMigrationFacts {
141:     source: DeclaredSchemaFacts,
142:     target: DeclaredSchemaFacts,
143:     schema_facts_fingerprint: String,
144: }
145: impl DeclaredMigrationFacts {
146:     pub fn new(source: DeclaredSchemaFacts, target: DeclaredSchemaFacts) -> Result<Self, FactsError> {
147:         let schema_facts_fingerprint = digest(FACTS_DOMAIN, &(&source, &target))?;
148:         Ok(Self { source, target, schema_facts_fingerprint })
149:     }
150:     pub fn source(&self) -> &DeclaredSchemaFacts { &self.source }
151:     pub fn target(&self) -> &DeclaredSchemaFacts { &self.target }
152:     pub fn schema_facts_fingerprint(&self) -> &str { &self.schema_facts_fingerprint }
153: 
154:     /// Generate both config fingerprints and the full facts fingerprint. No
155:     /// caller-supplied fingerprint is accepted on this constructor path.
156:     pub fn bind_inputs(&self, source: StorageFormatFacts, target: StorageFormatFacts,
157:         variables: BTreeMap<String, String>) -> Result<MigrationPlanInputs, FactsError>
158:     {
159:         let descriptor = |format: StorageFormatFacts, facts: &DeclaredSchemaFacts| {
160:             let d = StorageDescriptor { engine_format: format.engine_format,
161:                 internal_schema: format.internal_schema, application_format: format.application_format,
162:                 generation: format.generation, recipe: facts.input.recipe.id.clone(),
163:                 config_fingerprint: facts.config_fingerprint.clone() };
164:             d.validate().map_err(FactsError::Invalid)?;
165:             Ok::<_, FactsError>(d)
166:         };
167:         Ok(MigrationPlanInputs { source: descriptor(source, &self.source)?, target: descriptor(target, &self.target)?,
168:             variables, schema_facts_fingerprint: self.schema_facts_fingerprint.clone() })
169:     }
170: 
171:     /// Only declaration-to-plan correspondence. Unversioned is valid here;
172:     /// execution generation checks and storage provenance remain separate.
173:     pub fn compare_with_fixed_plan(&self, plan: &ResolvedMigrationPlan) -> Result<DeclaredFactsMatch, FactsError> {
174:         let i = plan.inputs();
175:         for (label, value, domain, expected) in [
176:             ("source configs", i.source.config_fingerprint.as_str(), CONFIG_DOMAIN, self.source.config_fingerprint.as_str()),
177:             ("target configs", i.target.config_fingerprint.as_str(), CONFIG_DOMAIN, self.target.config_fingerprint.as_str()),
178:             ("schema facts", i.schema_facts_fingerprint.as_str(), FACTS_DOMAIN, self.schema_facts_fingerprint.as_str()),
179:         ] {
180:             check_contract(value, domain, label)?;
181:             if value != expected { return Err(FactsError::Mismatch(label.into())); }
182:         }
183:         if i.source.recipe != self.source.input.recipe.id || i.target.recipe != self.target.input.recipe.id {
184:             return Err(FactsError::Mismatch("recipe identifier".into()));
185:         }
186:         Ok(DeclaredFactsMatch { referenced_plan_identity: plan.identity().into(),
187:             schema_facts_fingerprint: self.schema_facts_fingerprint.clone(), provenance: FactsProvenance::SuppliedDeclarations })
188:     }
189: }
190: 
191: #[derive(Debug, Clone, Serialize)]
192: pub struct StorageFormatFacts {
193:     pub engine_format: String,
194:     pub internal_schema: String,
195:     pub application_format: String,
196:     pub generation: StorageGeneration,
197: }
198: #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
199: pub enum FactsProvenance { SuppliedDeclarations }
200: /// Sealed report: no Deserialize, storage observation, journal or writer token.
201: #[derive(Debug, Clone, Serialize)]
202: pub struct DeclaredFactsMatch {
203:     referenced_plan_identity: String,
204:     schema_facts_fingerprint: String,
205:     provenance: FactsProvenance,
206: }
207: impl DeclaredFactsMatch {
208:     pub fn referenced_plan_identity(&self) -> &str { &self.referenced_plan_identity }
209:     pub fn schema_facts_fingerprint(&self) -> &str { &self.schema_facts_fingerprint }
210:     pub fn provenance(&self) -> FactsProvenance { self.provenance }
211: }
212: fn check_contract(value: &str, domain: &str, label: &str) -> Result<(), FactsError> {
213:     match value.strip_prefix(domain) {
214:         Some(hash) if hash.len()==64 && hash.bytes().all(|b| b.is_ascii_hexdigit()) => Ok(()),
215:         _ => Err(FactsError::UnknownContract(label.into())),
216:     }
217: }
218: fn digest<T: Serialize>(domain: &str, value: &T) -> Result<String, FactsError> {
219:     Ok(format!("{domain}{}", content_hash(&canonical_json(value).map_err(FactsError::Invalid)?)))
220: }
221: 
222: /// Encoding mechanism shared with the preview report, not its semantic domain.
223: /// Callers must validate non-finite/unsupported typed values before serialization.
224: pub(super) fn canonical_json<T: Serialize>(value: &T) -> Result<String, String> {
225:     fn canonical(v: serde_json::Value) -> serde_json::Value {
226:         match v {
227:             serde_json::Value::Object(m) => serde_json::Value::Object(m.into_iter().collect::<BTreeMap<_, _>>()
228:                 .into_iter().map(|(k,v)| (k,canonical(v))).collect()),
229:             serde_json::Value::Array(a) => serde_json::Value::Array(a.into_iter().map(canonical).collect()),
230:             v => v,
231:         }
232:     }
233:     serde_json::to_value(value).map(canonical).and_then(|v| serde_json::to_string(&v)).map_err(|e| e.to_string())
234: }
235: pub(super) fn entity_config_is_finite(c: &EntityConfig) -> bool {
236:     c.fields.values().all(|f| f.boost.is_none_or(|v| v.is_finite()))
237:         && c.snapshot.as_ref().is_none_or(|s| s.max_missing_ratio.is_finite())
238:         && c.fusion.as_ref().is_none_or(|f| f.rrf_k.is_finite()
239:             && f.bm25.weight.is_finite() && f.vector.weight.is_finite() && f.sparse.weight.is_finite())
240:         && c.field_weights.iter().all(|w| w.default.is_finite() && w.weights.values().all(|v| v.is_finite()))
241: }
242: fn validate_finite(entities: &BTreeMap<String, EntityConfig>, relations: &BTreeMap<String, RelationDef>) -> Result<(), FactsError> {
243:     if entities.values().any(|c| !entity_config_is_finite(c))
244:         || relations.values().any(|r| r.properties.as_ref().is_some_and(|p| p.values().any(|f| f.boost.is_some_and(|v| !v.is_finite())))) {
245:         return Err(FactsError::Invalid("non-finite schema value cannot be fingerprinted faithfully".into()));
246:     }
247:     Ok(())
248: }
249: 
250: #[cfg(test)]
251: mod tests;

SOURCE d2cb7f0636d099823f63de2cef674b630a46bc6f:extension/rag3weaver/src/dataflow/migrations/facts/tests.rs
1: use super::*;
2: use crate::{config::{DerivedConfig, DerivedIdentity, FieldDef, FieldType, SimpleFieldDef},
3:     dataflow::{migrations::{MigrationRunner, plan::{PlanJournalState, ResolvedMigrationStep, StoragePosition}}, node_registry::NodeRegistry}};
4: use std::{collections::HashMap, sync::Arc};
5: 
6: fn config() -> EntityConfig {
7:     EntityConfig { fields: HashMap::from([
8:         ("title".into(), SimpleFieldDef { field_type: FieldType::Text, is_title: true, ..Default::default() }),
9:         ("body".into(), SimpleFieldDef { field_type: FieldType::Text, is_content: true, ..Default::default() }),
10:     ]), signals: SearchSignals::BM25, ..Default::default() }
11: }
12: fn recipe() -> RecipeFactsInput {
13:     let mut source = ModelSource::local("synthetic-4");source.dimensions=Some(4);
14:     RecipeFactsInput { id: "synthetic-recipe".into(), resolved_templates: Some(BTreeMap::new()),
15:         model_sources: Some(BTreeMap::from([(Capability::Embed,source)])),
16:         dense_models: Some(BTreeMap::from([("synthetic-4".into(), EmbeddingModelEntry {
17:             name:"synthetic-4".into(),dim:4,storage:StorageKind::Legacy,
18:         })])) }
19: }
20: fn input() -> DeclaredSchemaInput {
21:     DeclaredSchemaInput { entities: Some(BTreeMap::from([("Document".into(),config()),("Collection".into(),config())])),
22:         relations: Some(BTreeMap::from([("CONTAINS".into(),BackendRelation { def:RelationDef {
23:             from:"Collection".into(),to:"Document".into(),properties:Some(HashMap::from([("reason".into(),
24:                 serde_json::from_value::<FieldDef>(serde_json::json!({"type":"string"})).unwrap())])),
25:         },from_states:Some(vec!["open".into(),"review".into()]),to_states:None,description:Some("ownership".into()) })])),
26:         provider:ProviderFacts::Absent,index_signals:BTreeMap::new(),recipe:recipe() }
27: }
28: fn derived_input() -> DeclaredSchemaInput {
29:     let mut i=input();let mut derived=config();
30:     derived.derived=Some(DerivedConfig {from:"Document".into(),identity:DerivedIdentity::Content {format:"document-content-v1".into()},
31:         gather:vec![],render:BTreeMap::from([("title".into(),"{{ root.title }}".into()),("body".into(),"body-template".into())])});
32:     i.entities.as_mut().unwrap().insert("DocumentContent".into(),derived);
33:     i.recipe.resolved_templates.as_mut().unwrap().insert("body-template".into(),"{{ root.body }}".into());
34:     i
35: }
36: fn facts(source:DeclaredSchemaInput,target:DeclaredSchemaInput) -> DeclaredMigrationFacts {
37:     DeclaredMigrationFacts::new(DeclaredSchemaFacts::from_complete_input(source).unwrap(),
38:         DeclaredSchemaFacts::from_complete_input(target).unwrap()).unwrap()
39: }
40: fn formats(name:&str) -> StorageFormatFacts {
41:     StorageFormatFacts {engine_format:"synthetic-engine".into(),internal_schema:"8".into(),application_format:name.into(),
42:         generation:StorageGeneration::Unversioned}
43: }
44: fn plan(f:&DeclaredMigrationFacts) -> ResolvedMigrationPlan {
45:     ResolvedMigrationPlan::new(f.bind_inputs(formats("old"),formats("new"),BTreeMap::new()).unwrap(),vec![ResolvedMigrationStep {
46:         version:1,name:"fixture".into(),checksum:"fixture-file".into(),graph_hash:"fixture-graph".into(),
47:     }]).unwrap()
48: }
49: fn mismatch(f:&DeclaredMigrationFacts,p:&ResolvedMigrationPlan,label:&str) {
50:     assert_eq!(f.compare_with_fixed_plan(p).unwrap_err(),FactsError::Mismatch(label.into()));
51: }
52: 
53: #[test]
54: fn runner_consumes_the_producer_and_returns_only_a_supplied_facts_match() {
55:     let f=facts(input(),derived_input());let d=tempfile::tempdir().unwrap();
56:     std::fs::write(d.path().join("001_check.mmd"),"graph TD\ncheck[\"ValidateNode(query='$query', assert='count == 1')\"]\n").unwrap();
57:     let mut runner=MigrationRunner::new(Arc::new(NodeRegistry::new()));runner.add_dir(d.path().to_owned());
58:     let vars=BTreeMap::from([("query".into(),"RETURN 1".into())]);
59:     let (p,matched)=runner.resolve_plan_from_facts(&f,formats("old"),formats("new"),vars.clone()).unwrap();
60:     assert_eq!(p.inputs().variables,vars);
61:     assert_eq!(matched.referenced_plan_identity(),p.identity());
62:     assert_eq!(matched.schema_facts_fingerprint(),f.schema_facts_fingerprint());
63:     assert_eq!(matched.provenance(),FactsProvenance::SuppliedDeclarations);
64:     assert_eq!(f.compare_with_fixed_plan(&p).unwrap().referenced_plan_identity(),p.identity());
65:     assert!(p.inputs().source.config_fingerprint.starts_with(CONFIG_DOMAIN));
66:     assert_eq!(p.inputs().source.config_fingerprint,f.source().config_fingerprint());
67:     assert!(p.validate_generation_preconditions().is_err(),"facts match must not establish a generation");
68:     let report=serde_json::to_value(matched).unwrap();
69:     assert_eq!(report["provenance"],"SuppliedDeclarations");
70:     assert!(report.get("applied").is_none());assert!(report.get("storage_observed").is_none());
71:     let vars=BTreeMap::from([("query".into(),"RETURN 2".into())]);
72:     let (other,_)=runner.resolve_plan_from_facts(&f,formats("old"),formats("new"),vars).unwrap();
73:     assert_ne!(p.identity(),other.identity());assert_ne!(p.steps()[0].graph_hash,other.steps()[0].graph_hash);
74: }
75: 
76: #[test]
77: fn map_insertion_order_is_irrelevant_but_semantic_list_order_is_preserved() {
78:     let a=input();let mut b=a.clone();
79:     for c in b.entities.as_mut().unwrap().values_mut() {
80:         let mut entries=c.fields.drain().collect::<Vec<_>>();entries.reverse();c.fields=entries.into_iter().collect();
81:     }
82:     let original=facts(a.clone(),a);let reordered=facts(b.clone(),b.clone());
83:     assert_eq!(original.schema_facts_fingerprint(),reordered.schema_facts_fingerprint());
84:     assert_eq!(plan(&original).identity(),plan(&reordered).identity());
85:     b.relations.as_mut().unwrap().get_mut("CONTAINS").unwrap().from_states.as_mut().unwrap().reverse();
86:     let changed=facts(input(),b);mismatch(&changed,&plan(&original),"schema facts");
87:     assert_eq!(changed.target().config_fingerprint(),original.target().config_fingerprint());
88: }
89: 
90: #[test]
91: fn complete_empty_and_unavailable_are_different_inputs() {
92:     for part in 0..6 {
93:         let mut i=input();match part {
94:             0=>i.entities=None,1=>i.relations=None,2=>i.recipe.resolved_templates=None,
95:             3=>i.recipe.model_sources=None,4=>i.recipe.dense_models=None,
96:             _=>i.provider=ProviderFacts::Unavailable{name:"declared-provider".into()},
97:         }
98:         assert!(matches!(DeclaredSchemaFacts::from_complete_input(i),Err(FactsError::Incomplete(_))));
99:     }
100:     let mut i=input();i.entities=Some(BTreeMap::new());i.relations=Some(BTreeMap::new());
101:     let complete=DeclaredSchemaFacts::from_complete_input(i).unwrap();
102:     assert!(complete.entities().is_empty());assert!(complete.relations().is_empty());
103: }
104: 
105: #[test]
106: fn final_added_dependencies_and_overrides_use_the_existing_schema_validator() {
107:     let mut i=derived_input();i.entities.as_mut().unwrap().remove("Document");
108:     assert!(matches!(DeclaredSchemaFacts::from_complete_input(i),Err(FactsError::Invalid(_))),"compose's empty provider pass is insufficient");
109:     let mut i=input();i.relations.as_mut().unwrap().get_mut("CONTAINS").unwrap().def.to="Missing".into();
110:     assert!(matches!(DeclaredSchemaFacts::from_complete_input(i),Err(FactsError::Invalid(_))));
111:     let mut i=input();i.index_signals.insert("Missing".into(),SearchSignals::BM25);
112:     assert!(matches!(DeclaredSchemaFacts::from_complete_input(i),Err(FactsError::Invalid(_))));
113:     let mut i=input();i.entities.as_mut().unwrap().get_mut("Document").unwrap().chunked=Some(false);
114:     i.index_signals.insert("Document".into(),SearchSignals::VECTOR);
115:     assert!(matches!(DeclaredSchemaFacts::from_complete_input(i),Err(FactsError::Invalid(_))),"effective overrides must be validated");
116: }
117: 
118: #[test]
119: fn provider_definitions_are_composed_and_conflicts_are_not_hidden() {
120:     let supplied=input();
121:     let provided_entities=supplied.entities.clone().unwrap();
122:     let provided_relations=supplied.relations.as_ref().unwrap().iter().map(|(n,r)|(n.clone(),r.def.clone())).collect();
123:     let mut i=input();i.entities=Some(BTreeMap::new());i.relations=Some(BTreeMap::new());
124:     i.provider=ProviderFacts::Complete{name:"document-provider".into(),entities:provided_entities,relations:provided_relations};
125:     let composed=DeclaredSchemaFacts::from_complete_input(i.clone()).unwrap();assert_eq!(composed.entities().len(),2);
126:     assert_eq!(composed.relations()["CONTAINS"].from,"Collection");
127:     let mut c=config();c.chunking.max_size+=1;i.entities.as_mut().unwrap().insert("Document".into(),c);
128:     assert!(matches!(DeclaredSchemaFacts::from_complete_input(i),Err(FactsError::Invalid(_))));
129: }
130: 
131: #[cfg(feature="code")]
132: #[test]
133: fn actual_code_provider_and_effective_signal_overrides_are_in_the_facts() {
134:     let (entities,relations)=crate::code::schema_configs(crate::code::default_scope_chunking());
135:     let expected_entities=entities.len();let expected_relations=relations.len();
136:     let expected_relation_definitions=canonical_json(&relations).unwrap();
137:     let mut i=input();i.entities=Some(BTreeMap::new());i.relations=Some(BTreeMap::new());
138:     i.provider=ProviderFacts::Complete{name:"code".into(),entities,relations};
139:     let initial=DeclaredSchemaFacts::from_complete_input(i.clone()).unwrap();
140:     i.index_signals.insert("Scope".into(),SearchSignals::BM25);
141:     let effective=DeclaredSchemaFacts::from_complete_input(i).unwrap();
142:     assert_eq!(effective.entities().len(),expected_entities);assert_eq!(effective.relations().len(),expected_relations);
143:     assert_eq!(effective.entities()["Scope"].signals,SearchSignals::BM25);
144:     assert_ne!(effective.config_fingerprint(),initial.config_fingerprint());
145:     assert_eq!(canonical_json(effective.relations()).unwrap(),expected_relation_definitions,
146:         "all provider relation endpoints and properties must survive composition");
147: }
148: 
149: #[test]
150: fn effective_merged_schema_fields_are_not_replaced_with_a_minimal_fixture() {
151:     let initial=facts(input(),input());let p=plan(&initial);
152:     let mut i=input();i.entities.as_mut().unwrap().get_mut("Document").unwrap().fields.insert("schema_extra".into(),
153:         SimpleFieldDef{field_type:FieldType::Int64,..Default::default()});
154:     let changed=facts(input(),i);mismatch(&changed,&p,"target configs");
155:     assert!(changed.target().entities()["Document"].fields.contains_key("schema_extra"));
156: }
157: 
158: #[test]
159: fn config_roles_chunking_signals_and_defaults_are_bound_not_just_labeled() {
160:     let original=facts(input(),input());let p=plan(&original);
161:     for case in 0..5 {
162:         let mut i=input();let c=i.entities.as_mut().unwrap().get_mut("Document").unwrap();
163:         match case {
164:             0=>c.fields.get_mut("title").unwrap().is_title=false,
165:             1=>c.chunking.overlap+=1,
166:             2=>c.return_fields=Some(vec!["body".into(),"title".into()]),
167:             3=>c.fields.get_mut("body").unwrap().boost=Some(2.0),
168:             _=>{i.index_signals.insert("Document".into(),SearchSignals::HYBRID);},
169:         }
170:         let changed=facts(input(),i);mismatch(&changed,&p,"target configs");
171:         assert_ne!(plan(&changed).identity(),p.identity());
172:     }
173: }
174: 
175: #[test]
176: fn relation_direction_properties_and_policies_bind_the_full_facts_not_entity_digest() {
177:     let original=facts(input(),input());let p=plan(&original);
178:     for case in 0..4 {
179:         let mut i=input();let r=i.relations.as_mut().unwrap().get_mut("CONTAINS").unwrap();
180:         match case {
181:             0=>{std::mem::swap(&mut r.def.from,&mut r.def.to);},
182:             1=>{r.def.properties.as_mut().unwrap().get_mut("reason").unwrap().default_value=Some(serde_json::json!("changed"));},
183:             2=>r.to_states=Some(vec!["hidden".into()]),
184:             _=>{i.relations.as_mut().unwrap().remove("CONTAINS");},
185:         }
186:         let changed=facts(input(),i);
187:         assert_eq!(changed.target().config_fingerprint(),original.target().config_fingerprint());
188:         mismatch(&changed,&p,"schema facts");
189:     }
190: }
191: 
192: #[test]
193: fn named_recipe_requires_supplied_bytes_and_tracks_them_without_file_resolution() {
194:     let mut i=derived_input();i.recipe.resolved_templates.as_mut().unwrap().clear();
195:     assert_eq!(DeclaredSchemaFacts::from_complete_input(i).unwrap_err(),FactsError::UnresolvedRecipe("body-template".into()));
196:     let original=facts(input(),derived_input());let p=plan(&original);
197:     let mut i=derived_input();i.recipe.resolved_templates.as_mut().unwrap().insert("body-template".into(),"{{ root.body }}\n".into());
198:     let changed=facts(input(),i);mismatch(&changed,&p,"schema facts");
199:     assert_eq!(changed.target().config_fingerprint(),original.target().config_fingerprint());
200:     let mut i=derived_input();i.recipe.resolved_templates.as_mut().unwrap().insert("unused".into(),"bytes".into());
201:     assert!(matches!(DeclaredSchemaFacts::from_complete_input(i),Err(FactsError::Invalid(_))));
202: }
203: 
204: #[test]
205: fn models_dimensions_storage_and_reader_declarations_are_part_of_the_recipe() {
206:     let original=facts(input(),input());let p=plan(&original);
207:     for case in 0..4 {
208:         let mut i=input();match case {
209:             0=>i.recipe.dense_models.as_mut().unwrap().get_mut("synthetic-4").unwrap().storage=StorageKind::Suffixed,
210:             1=>{i.recipe.dense_models.as_mut().unwrap().get_mut("synthetic-4").unwrap().dim=8;
211:                 i.recipe.model_sources.as_mut().unwrap().get_mut(&Capability::Embed).unwrap().dimensions=Some(8);},
212:             2=>{i.recipe.model_sources.as_mut().unwrap().insert(Capability::Sparse,ModelSource::service("highlight-reader"));},
213:             _=>i.recipe.id.push_str("-changed"),
214:         }
215:         let changed=facts(input(),i);mismatch(&changed,&p,"schema facts");
216:         assert_eq!(changed.target().config_fingerprint(),original.target().config_fingerprint());
217:     }
218: }
219: 
220: #[test]
221: fn malformed_or_missing_model_facts_refuse_without_connecting_any_model() {
222:     for case in 0..4 {
223:         let mut i=input();match case {
224:             0=>i.recipe.model_sources.as_mut().unwrap().get_mut(&Capability::Embed).unwrap().dimensions=Some(8),
225:             1=>i.recipe.dense_models.as_mut().unwrap().get_mut("synthetic-4").unwrap().dim=0,
226:             2=>{i.recipe.dense_models.as_mut().unwrap().clear();i.entities.as_mut().unwrap().get_mut("Document").unwrap().signals=SearchSignals::VECTOR;},
227:             _=>{let mut m=i.recipe.dense_models.as_ref().unwrap()["synthetic-4"].clone();m.name="synthetic_4".into();
228:                 i.recipe.dense_models.as_mut().unwrap().insert(m.name.clone(),m);},
229:         }
230:         assert!(DeclaredSchemaFacts::from_complete_input(i).is_err());
231:     }
232: }
233: 
234: #[test]
235: fn opaque_legacy_fingerprints_and_unknown_contract_versions_are_not_requalified() {
236:     let f=facts(input(),input());
237:     for case in 0..3 {
238:         let mut inputs=plan(&f).inputs().clone();let steps=plan(&f).steps().to_vec();
239:         match case {0=>inputs.source.config_fingerprint="opaque-label".into(),
240:             1=>inputs.target.config_fingerprint=format!("declared-entity-configs:v2:{}","0".repeat(64)),
241:             _=>inputs.schema_facts_fingerprint="schema-name-not-facts".into(),}
242:         let p=ResolvedMigrationPlan::new(inputs,steps).unwrap();
243:         assert!(matches!(f.compare_with_fixed_plan(&p),Err(FactsError::UnknownContract(_))));
244:     }
245:     let mut inputs=plan(&f).inputs().clone();inputs.source.recipe="different-label".into();
246:     let p=ResolvedMigrationPlan::new(inputs,plan(&f).steps().to_vec()).unwrap();mismatch(&f,&p,"recipe identifier");
247: }
248: 
249: #[test]
250: fn facts_match_with_unversioned_and_foreign_generation_does_not_qualify_storage() {
251:     let f=facts(input(),input());let p=plan(&f);
252:     assert!(f.compare_with_fixed_plan(&p).is_ok());
253:     assert_eq!(p.inputs().source.generation,StorageGeneration::Unversioned);
254:     assert!(p.qualify_storage(&p.inputs().source,&PlanJournalState::Unapplied).is_err());
255:     let mut source=formats("old");source.generation=StorageGeneration::Established("explicit-synthetic-source".into());
256:     let mut target=formats("new");target.generation=StorageGeneration::Established("explicit-synthetic-target".into());
257:     let inputs=f.bind_inputs(source,target,BTreeMap::new()).unwrap();
258:     let p=ResolvedMigrationPlan::new(inputs,p.steps().to_vec()).unwrap();let id=p.identity().to_string();
259:     assert_eq!(p.qualify_storage(&p.inputs().source,&PlanJournalState::Unapplied).unwrap(),StoragePosition::Source);
260:     assert_eq!(p.qualify_storage(&p.inputs().target,&PlanJournalState::Applied{identity:id.clone()}).unwrap(),StoragePosition::PublishedTarget);
261:     let mut foreign=p.inputs().target.clone();foreign.generation=StorageGeneration::Established("foreign-generation".into());
262:     assert!(p.qualify_storage(&foreign,&PlanJournalState::Applied{identity:id}).is_err());
263:     assert!(f.compare_with_fixed_plan(&p).is_ok(),"facts correspondence is not data-generation qualification");
264: }
265: 
266: #[test]
267: fn schema_and_provider_nonfinite_values_refuse_before_json_can_turn_them_into_null() {
268:     for case in 0..3 {
269:         let mut i=input();match case {
270:             0=>i.entities.as_mut().unwrap().get_mut("Document").unwrap().fields.get_mut("body").unwrap().boost=Some(f64::NAN),
271:             1=>i.relations.as_mut().unwrap().get_mut("CONTAINS").unwrap().def.properties.as_mut().unwrap().get_mut("reason").unwrap().boost=Some(f64::INFINITY),
272:             _=>{let mut e=i.entities.clone().unwrap();e.get_mut("Document").unwrap().fields.get_mut("body").unwrap().boost=Some(f64::NEG_INFINITY);
273:                 i.provider=ProviderFacts::Complete{name:"bad-provider".into(),entities:e,relations:BTreeMap::new()};},
274:         }
275:         assert!(matches!(DeclaredSchemaFacts::from_complete_input(i),Err(FactsError::Invalid(_))));
276:     }
277: }
278: 
279: #[test]
280: fn immutable_snapshot_does_not_follow_later_mutation_of_its_source_declarations() {
281:     let mut i=input();let fixed=facts(i.clone(),i.clone());let p=plan(&fixed);let identity=p.identity().to_string();
282:     i.entities.as_mut().unwrap().get_mut("Document").unwrap().chunking.max_size+=10;
283:     let changed=facts(i.clone(),i);mismatch(&changed,&p,"source configs");
284:     assert_eq!(p.identity(),identity);assert!(fixed.compare_with_fixed_plan(&p).is_ok());
285: }
286: 
287: #[test]
288: fn canonical_encoding_mechanism_keeps_object_keys_sorted_and_list_order_exact() {
289:     let value=serde_json::json!({"z":[2,1],"a":{"b":true,"a":"é"}});
290:     assert_eq!(canonical_json(&value).unwrap(),"{\"a\":{\"a\":\"é\",\"b\":true},\"z\":[2,1]}");
291: }
292: 
293: #[test]
294: fn fusion_and_field_weight_nonfinite_values_are_rejected_in_manifest_and_provider() {
295:     for case in 0..6 {
296:         for provided in [false,true] {
297:             let mut i=input();let mut config=config();
298:             let mut fusion=crate::search::FusionConfig::default();
299:             match case {
300:                 0=>fusion.rrf_k=f64::NAN,1=>fusion.bm25.weight=f64::INFINITY,
301:                 2=>fusion.vector.weight=f64::NEG_INFINITY,3=>fusion.sparse.weight=f64::NAN,
302:                 4=>config.field_weights.push(crate::search::FieldWeight {field:"kind".into(),weights:BTreeMap::new(),default:f64::NAN}),
303:                 _=>config.field_weights.push(crate::search::FieldWeight {field:"kind".into(),weights:BTreeMap::from([("x".into(),f64::INFINITY)]),default:1.0}),
304:             }
305:             config.fusion=Some(fusion);
306:             if provided { i.provider=ProviderFacts::Complete {name:"nonfinite-provider".into(),
307:                 entities:BTreeMap::from([("Provided".into(),config)]),relations:BTreeMap::new()}; }
308:             else { i.entities.as_mut().unwrap().insert("Document".into(),config); }
309:             assert!(matches!(DeclaredSchemaFacts::from_complete_input(i),Err(FactsError::Invalid(_))),"case {case}, provider {provided}");
310:         }
311:     }
312: }

SOURCE d2cb7f0636d099823f63de2cef674b630a46bc6f:extension/rag3weaver/src/backend_schema.rs
1: //! Un schéma se monte selon ses dépendances, indépendamment de l'ordre JSON/map.
2: use super::*;
3: 
4: #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
5: pub(super) enum SchemaStep {
6:     Entity(String),
7:     Relation(String),
8: }
9: 
10: pub(super) type ProvidedSchema = (
11:     BTreeMap<String, EntityConfig>,
12:     BTreeMap<String, RelationDef>,
13: );
14: 
15: pub(super) fn provided_schema(
16:     workspace: Option<&crate::backend_code::WorkspaceConfig>,
17: ) -> ProvidedSchema {
18:     #[cfg(feature = "code")]
19:     if workspace.and_then(|w| w.index.as_deref()) == Some("code") {
20:         return crate::code::schema_configs(crate::code::default_scope_chunking());
21:     }
22:     let _ = workspace;
23:     Default::default()
24: }
25: 
26: /// Faits réels copiés sans perte : aucun schéma synthétique pour un fournisseur.
27: /// Une définition identique se partage ; deux sens différents ne s'écrasent pas.
28: pub(super) struct SchemaFacts {
29:     pub entities: HashMap<String, EntityConfig>,
30:     pub relations: HashMap<String, RelationDef>,
31: }
32: 
33: impl SchemaFacts {
34:     pub fn compose(
35:         entities: &HashMap<String, EntityConfig>,
36:         relations: &HashMap<String, BackendRelation>,
37:         provided: &ProvidedSchema,
38:     ) -> Result<Self, String> {
39:         let mut facts = Self {
40:             entities: provided
41:                 .0
42:                 .iter()
43:                 .map(|(n, c)| (n.clone(), c.clone()))
44:                 .collect(),
45:             relations: provided
46:                 .1
47:                 .iter()
48:                 .map(|(n, r)| (n.clone(), r.clone()))
49:                 .collect(),
50:         };
51:         // Le fournisseur se monte d'un bloc : ses dépendances sont internes.
52:         for (name, c) in &facts.entities {
53:             c.validate().map_err(|e| format!("{name}: {e}"))?;
54:         }
55:         facts.registration_order()?;
56:         for (name, config) in entities {
57:             if let Some(other) = facts.entities.get(name.as_str()) {
58:                 if serde_json::to_value(config).map_err(|e| e.to_string())?
59:                     != serde_json::to_value(other).map_err(|e| e.to_string())?
60:                 {
61:                     return Err(format!("schéma : définitions incompatibles de l'entité « {name} » (manifeste et fournisseur nommé)"));
62:                 }
63:             } else {
64:                 facts.entities.insert(name.clone(), config.clone());
65:             }
66:         }
67:         for (name, relation) in relations {
68:             if let Some(other) = facts.relations.get_mut(name) {
69:                 if other.from != relation.from || other.to != relation.to {
70:                     return Err(format!(
71:                         "schéma : définitions incompatibles de la relation « {name} » (extrémités)"
72:                     ));
73:                 }
74:                 let properties = other.properties.get_or_insert_with(HashMap::new);
75:                 for (field, definition) in relation.properties.as_ref().into_iter().flatten() {
76:                     if let Some(previous) = properties.get(field) {
77:                         if serde_json::to_value(previous).map_err(|e| e.to_string())?
78:                             != serde_json::to_value(definition).map_err(|e| e.to_string())?
79:                         {
80:                             return Err(format!("schéma : définitions incompatibles de la relation « {name} », propriété « {field} »"));
81:                         }
82:                     } else {
83:                         properties.insert(field.clone(), definition.clone());
84:                     }
85:                 }
86:             } else {
87:                 facts.relations.insert(name.clone(), relation.def.clone());
88:             }
89:         }
90:         Ok(facts)
91:     }
92: 
93:     pub fn registration_order(&self) -> Result<Vec<SchemaStep>, String> {
94:         use SchemaStep::*;
95:         let mut pending = BTreeMap::<SchemaStep, BTreeSet<SchemaStep>>::new();
96:         for (name, config) in &self.entities {
97:             // Une association de lecture doit être valable avant toute ouverture,
98:             // sans devenir l'arête inverse de la dépendance derived.from.
99:             config.validate_search_index_context(name, |n| self.entities.get(n))?;
100:             let mut dependencies = BTreeSet::new();
101:             if let Some(d) = &config.derived {
102:                 d.validate_context(name, |n| self.entities.get(n), |n| self.relations.get(n))
103:                     .map_err(|e| format!("{name}: {e}"))?;
104:                 dependencies.insert(Entity(d.from.clone()));
105:                 for gather in &d.gather {
106:                     if !self.relations.contains_key(gather.relation.as_str()) {
107:                         return Err(format!(
108:                             "{name}: derived : relation « {} » (règle « {} ») non déclarée",
109:                             gather.relation, gather.name
110:                         ));
111:                     }
112:                     dependencies.insert(Relation(gather.relation.clone()));
113:                 }
114:             }
115:             pending.insert(Entity(name.clone()), dependencies);
116:         }
117:         for (name, relation) in &self.relations {
118:             for endpoint in [&relation.from, &relation.to] {
119:                 if !self.entities.contains_key(endpoint.as_str()) {
120:                     return Err(format!("{name}: extrémité « {endpoint} » non déclarée"));
121:                 }
122:             }
123:             pending.insert(
124:                 Relation(name.clone()),
125:                 [Entity(relation.from.clone()), Entity(relation.to.clone())]
126:                     .into_iter()
127:                     .collect(),
128:             );
129:         }
130:         let mut done = BTreeSet::new();
131:         let mut order = Vec::with_capacity(pending.len());
132:         while !pending.is_empty() {
133:             let next = pending
134:                 .iter()
135:                 .find(|(_, dependencies)| dependencies.is_subset(&done))
136:                 .map(|(step, _)| step.clone());
137:             let Some(next) = next else {
138:                 let blocked = pending
139:                     .keys()
140:                     .map(|step| match step {
141:                         Entity(n) => format!("entité « {n} »"),
142:                         Relation(n) => format!("relation « {n} »"),
143:                     })
144:                     .collect::<Vec<_>>()
145:                     .join(", ");
146:                 return Err(format!("cycle de dépendances du schéma : {blocked}"));
147:             };
148:             pending.remove(&next);
149:             done.insert(next.clone());
150:             order.push(next);
151:         }
152:         Ok(order)
153:     }
154: }
155: 
156: // The sole bridge from migration facts to the existing schema composition and
157: // dependency validator. An inherent method avoids exposing this private backend
158: // module or broadening backend.rs just to reuse its implementation.
159: impl crate::dataflow::migrations::facts::DeclaredSchemaFacts {
160:     pub(crate) fn compose_effective(
161:         input: &crate::dataflow::migrations::facts::DeclaredSchemaInput,
162:     ) -> Result<(BTreeMap<String, EntityConfig>, BTreeMap<String, RelationDef>), String> {
163:         use crate::dataflow::migrations::facts::ProviderFacts;
164:         let provided = match &input.provider {
165:             ProviderFacts::Absent => (BTreeMap::new(), BTreeMap::new()),
166:             ProviderFacts::Complete { entities, relations, .. } => (entities.clone(), relations.clone()),
167:             ProviderFacts::Unavailable { name } => return Err(format!("provider facts unavailable: {name}")),
168:         };
169:         let entities = input.entities.as_ref().ok_or("entity facts unavailable")?
170:             .iter().map(|(n,c)| (n.clone(),c.clone())).collect();
171:         let relations = input.relations.as_ref().ok_or("relation facts unavailable")?
172:             .iter().map(|(n,r)| (n.clone(),r.clone())).collect();
173:         let mut facts = SchemaFacts::compose(&entities, &relations, &provided)?;
174:         for (name, signals) in &input.index_signals {
175:             let config = facts.entities.get_mut(name).ok_or_else(|| format!("unknown index_signals entity: {name}"))?;
176:             config.signals = *signals;
177:         }
178:         for (name, config) in &facts.entities { config.validate().map_err(|e| format!("{name}: {e}"))?; }
179:         // compose checks the provider before additions. The complete result and
180:         // effective overrides must be checked too, as in normal opening.
181:         facts.registration_order()?;
182:         Ok((facts.entities.into_iter().collect(), facts.relations.into_iter().collect()))
183:     }
184: }

SOURCE d2cb7f0636d099823f63de2cef674b630a46bc6f:extension/rag3weaver/src/dataflow/migrations.rs
1: //! Migration runner for schema migrations via dataflow graphs.
2: //!
3: //! Migrations are `.mmd` (Mermaid) files parsed into dataflow graphs and executed
4: //! via [`DataflowRuntime`]. Each migration benefits from checkpoint (crash recovery),
5: //! observability, and undo support.
6: //!
7: //! ## File convention
8: //!
9: //! ```text
10: //! migrations/
11: //!   001_add_version_field.mmd
12: //!   002_rename_entity_type.mmd
13: //! migrations/internal/
14: //!   001_create_dataflow_tables.mmd
15: //! ```
16: //!
17: //! Format: `{version}_{name}.mmd` where version is a zero-padded integer.
18: 
19: pub mod plan;
20: pub mod facts;
21: pub mod transfer;
22: 
23: use std::collections::HashMap;
24: use std::fmt;
25: use std::path::{Path, PathBuf};
26: use std::sync::Arc;
27: 
28: use super::checkpoint::{CheckpointStore, timestamp_ms};
29: use super::checkpoint_store::CypherCheckpointStore;
30: use super::graph::DataflowGraph;
31: use super::mermaid::parse_mermaid_template;
32: use super::node_registry::NodeRegistry;
33: use crate::catalog::Catalog;
34: use crate::hash::content_hash;
35: 
36: // ─── MigrationFile ──────────────────────────────────────────────────────────
37: 
38: /// A migration file discovered on disk.
39: #[derive(Debug, Clone)]
40: pub struct MigrationFile {
41:     pub version: u64,
42:     pub name: String,
43:     pub path: PathBuf,
44:     pub content: String,
45:     pub checksum: String,
46: }
47: 
48: /// Parse a migration filename like `001_add_version.mmd` into (version, name).
49: fn parse_migration_filename(filename: &str) -> Option<(u64, String)> {
50:     let stem = filename.strip_suffix(".mmd")?;
51:     let underscore = stem.find('_')?;
52:     let version_str = &stem[..underscore];
53:     let name = &stem[underscore + 1..];
54:     if name.is_empty() {
55:         return None;
56:     }
57:     let version: u64 = version_str.parse().ok()?;
58:     Some((version, name.to_string()))
59: }
60: 
61: /// Scan a directory for `.mmd` migration files, sorted by version.
62: fn scan_migration_dir(dir: &Path) -> Result<Vec<MigrationFile>, MigrationError> {
63:     if !dir.exists() {
64:         return Ok(Vec::new());
65:     }
66:     let entries = std::fs::read_dir(dir)
67:         .map_err(|e| MigrationError::IoError(format!("cannot read {}: {e}", dir.display())))?;
68: 
69:     let mut files = Vec::new();
70:     for entry in entries {
71:         let entry = entry
72:             .map_err(|e| MigrationError::IoError(format!("readdir entry: {e}")))?;
73:         let path = entry.path();
74:         if !path.is_file() {
75:             continue;
76:         }
77:         let filename = match path.file_name().and_then(|n| n.to_str()) {
78:             Some(n) => n.to_string(),
79:             None => continue,
80:         };
81:         if !filename.ends_with(".mmd") {
82:             continue;
83:         }
84:         let (version, name) = match parse_migration_filename(&filename) {
85:             Some(v) => v,
86:             None => {
87:                 return Err(MigrationError::InvalidFilename {
88:                     path: path.clone(),
89:                     detail: format!("expected format {{version}}_{{name}}.mmd, got '{filename}'"),
90:                 });
91:             }
92:         };
93:         let content = std::fs::read_to_string(&path)
94:             .map_err(|e| MigrationError::IoError(format!("cannot read {}: {e}", path.display())))?;
95:         let checksum = content_hash(&content);
96:         files.push(MigrationFile {
97:             version,
98:             name,
99:             path,
100:             content,
101:             checksum,
102:         });
103:     }
104: 
105:     files.sort_by_key(|f| f.version);
106: 
107:     // Check for duplicate versions
108:     for pair in files.windows(2) {
109:         if pair[0].version == pair[1].version {
110:             return Err(MigrationError::DuplicateVersion {
111:                 version: pair[0].version,
112:                 file_a: pair[0].path.clone(),
113:                 file_b: pair[1].path.clone(),
114:             });
115:         }
116:     }
117: 
118:     Ok(files)
119: }
120: 
121: // ─── MigrationStatus ────────────────────────────────────────────────────────
122: 
123: /// Status of a migration (file + DB state combined).
124: #[derive(Debug, Clone)]
125: pub struct MigrationStatus {
126:     pub version: u64,
127:     pub name: String,
128:     pub state: MigrationState,
129:     pub checksum: String,
130:     /// Checksum stored in DB (if applied). None if pending.
131:     pub db_checksum: Option<String>,
132:     /// Execution ID for the migration (if applied).
133:     pub execution_id: Option<String>,
134:     pub applied_at: Option<u64>,
135:     pub duration_ms: Option<u64>,
136: }
137: 
138: #[derive(Debug, Clone, PartialEq, Eq)]
139: pub enum MigrationState {
140:     Pending,
141:     Applied,
142:     Failed,
143:     RolledBack,
144: }
145: 
146: impl fmt::Display for MigrationState {
147:     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
148:         match self {
149:             Self::Pending => write!(f, "pending"),
150:             Self::Applied => write!(f, "applied"),
151:             Self::Failed => write!(f, "failed"),
152:             Self::RolledBack => write!(f, "rolled_back"),
153:         }
154:     }
155: }
156: 
157: // ─── MigrationResult ────────────────────────────────────────────────────────
158: 
159: /// Result of applying or rolling back a single migration.
160: #[derive(Debug, Clone)]
161: pub struct MigrationResult {
162:     pub version: u64,
163:     pub name: String,
164:     pub direction: MigrationDirection,
165:     pub state: MigrationState,
166:     pub duration_ms: u64,
167:     pub error: Option<String>,
168:     /// If dry_run, the parsed graph definition (for inspection).
169:     pub dry_run_plan: Option<DryRunPlan>,
170: }
171: 
172: #[derive(Debug, Clone, PartialEq, Eq)]
173: pub enum MigrationDirection {
174:     Up,
175:     Down,
176: }
177: 
178: impl fmt::Display for MigrationDirection {
179:     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
180:         match self {
181:             Self::Up => write!(f, "up"),
182:             Self::Down => write!(f, "down"),
183:         }
184:     }
185: }
186: 
187: /// Dry-run plan: parsed graph info without execution.
188: #[derive(Debug, Clone)]
189: pub struct DryRunPlan {
190:     pub node_count: usize,
191:     pub edge_count: usize,
192:     pub nodes: Vec<DryRunNode>,
193:     pub all_reversible: bool,
194: }
195: 
196: #[derive(Debug, Clone)]
197: pub struct DryRunNode {
198:     pub name: String,
199:     pub node_type: String,
200:     pub config: serde_json::Value,
201:     pub can_undo: bool,
202: }
203: 
204: // ─── MigrationError ─────────────────────────────────────────────────────────
205: 
206: #[derive(Debug)]
207: pub enum MigrationError {
208:     IoError(String),
209:     InvalidFilename { path: PathBuf, detail: String },
210:     DuplicateVersion { version: u64, file_a: PathBuf, file_b: PathBuf },
211:     ParseError { version: u64, name: String, detail: String },
212:     GraphError { version: u64, name: String, detail: String },
213:     ExecutionError { version: u64, name: String, detail: String },
214:     Locked { by: String, since: u64 },
215:     NotApplied { version: u64 },
216:     NotReversible { version: u64, name: String, nodes: Vec<String> },
217:     DbError(String),
218:     InvalidHistory { version: Option<u64>, detail: String },
219:     InvalidPlan(String),
220: }
221: 
222: impl fmt::Display for MigrationError {
223:     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
224:         match self {
225:             Self::IoError(e) => write!(f, "IO error: {e}"),
226:             Self::InvalidFilename { path, detail } => {
227:                 write!(f, "invalid migration filename '{}': {detail}", path.display())
228:             }
229:             Self::DuplicateVersion { version, file_a, file_b } => {
230:                 write!(
231:                     f,
232:                     "duplicate version {version}: '{}' and '{}'",
233:                     file_a.display(),
234:                     file_b.display()
235:                 )
236:             }
237:             Self::ParseError { version, name, detail } => {
238:                 write!(f, "migration {version}_{name}: parse error: {detail}")
239:             }
240:             Self::GraphError { version, name, detail } => {
241:                 write!(f, "migration {version}_{name}: graph error: {detail}")
242:             }
243:             Self::ExecutionError { version, name, detail } => {
244:                 write!(f, "migration {version}_{name}: execution error: {detail}")
245:             }
246:             Self::Locked { by, since } => {
247:                 write!(f, "migrations locked by '{by}' since {since}")
248:             }
249:             Self::NotApplied { version } => {
250:                 write!(f, "migration {version} is not applied")
251:             }
252:             Self::NotReversible { version, name, nodes } => {
253:                 write!(
254:                     f,
255:                     "migration {version}_{name}: not reversible — nodes without undo: {}",
256:                     nodes.join(", ")
257:                 )
258:             }
259:             Self::DbError(e) => write!(f, "database error: {e}"),
260:             Self::InvalidPlan(detail) => write!(f, "invalid migration plan: {detail}"),
261:             Self::InvalidHistory { version, detail } => match version {
262:                 Some(v) => write!(f, "invalid migration history at version {v}: {detail}"),
263:                 None => write!(f, "invalid migration history: {detail}"),
264:             },
265:         }
266:     }
267: }
268: 
269: impl std::error::Error for MigrationError {}
270: 
271: // ─── MigrationRunner ────────────────────────────────────────────────────────
272: 
273: /// Runs schema migrations defined as Mermaid dataflow graphs.
274: ///
275: /// Pure orchestrator: scans files, decides ordering, delegates all DB work
276: /// to the [`Catalog`] via its `migration_*` methods.
277: pub struct MigrationRunner {
278:     registry: Arc<NodeRegistry>,
279:     migration_dirs: Vec<PathBuf>,
280:     lock_id: String,
281: }
282: 
283: impl MigrationRunner {
284:     pub fn new(registry: Arc<NodeRegistry>) -> Self {
285:         let lock_id = format!("runner-{}", timestamp_ms());
286:         Self {
287:             registry,
288:             migration_dirs: Vec::new(),
289:             lock_id,
290:         }
291:     }
292: 
293:     /// Add a directory to scan for migration files.
294:     pub fn add_dir(&mut self, dir: PathBuf) {
295:         self.migration_dirs.push(dir);
296:     }
297: 
298:     /// Resolve declared files and template variables without a catalog, DDL or
299:     /// model access. This is a preliminary plan, never an apply authorization.
300:     pub fn resolve_plan(
301:         &self,
302:         inputs: plan::MigrationPlanInputs,
303:     ) -> Result<plan::ResolvedMigrationPlan, MigrationError> {
304:         let steps = self.resolve_steps(&inputs.variables)?;
305:         plan::ResolvedMigrationPlan::new(inputs, steps)
306:             .map_err(MigrationError::InvalidPlan)
307:     }
308: 
309:     /// Build fixed inputs from complete supplied facts, then consume the same
310:     /// producer for correspondence checking. This never qualifies storage.
311:     pub fn resolve_plan_from_facts(
312:         &self,
313:         facts: &facts::DeclaredMigrationFacts,
314:         source: facts::StorageFormatFacts,
315:         target: facts::StorageFormatFacts,
316:         variables: std::collections::BTreeMap<String, String>,
317:     ) -> Result<(plan::ResolvedMigrationPlan, facts::DeclaredFactsMatch), MigrationError> {
318:         let inputs = facts.bind_inputs(source, target, variables)
319:             .map_err(|e| MigrationError::InvalidPlan(e.to_string()))?;
320:         let plan = self.resolve_plan(inputs)?;
321:         let matched = facts.compare_with_fixed_plan(&plan)
322:             .map_err(|e| MigrationError::InvalidPlan(e.to_string()))?;
323:         Ok((plan, matched))
324:     }
325: 
326:     /// Resolve graphs for inspection without claiming known storage generations.
327:     pub fn resolve_steps(
328:         &self,
329:         variables: &std::collections::BTreeMap<String, String>,
330:     ) -> Result<Vec<plan::ResolvedMigrationStep>, MigrationError> {
331:         Self::resolve_files_steps(self.scan_all()?, variables)
332:     }
333: 
334:     /// Public-plan qualification consumes the history captured by the same
335:     /// metadata snapshot as the source descriptor. It never reloads a catalog.
336:     #[cfg(feature = "rag3db-native")]
337:     pub(crate) fn resolve_inspected_steps(
338:         &self,
339:         variables: &std::collections::BTreeMap<String, String>,
340:         history: &std::collections::BTreeMap<u64, AppliedMigration>,
341:     ) -> Result<Vec<plan::ResolvedMigrationStep>, MigrationError> {
342:         let files = self.scan_all()?;
343:         validate_declared_history(&files, history.iter())?;
344:         if let Some((&version, entry)) = history.iter().next() {
345:             // The existing journal has eight columns and records no resolved
346:             // plan identity. A matching checksum cannot supply the missing proof.
347:             let detail = if entry.status == "applied" {
348:                 "resolved identity unavailable in legacy eight-column journal".to_owned()
349:             } else {
350:                 format!("migration reconciliation required: {}", entry.status)
351:             };
352:             return Err(MigrationError::InvalidHistory { version: Some(version), detail });
353:         }
354:         Self::resolve_files_steps(files, variables)
355:     }
356: 
357:     fn resolve_files_steps(
358:         files: Vec<MigrationFile>,
359:         variables: &std::collections::BTreeMap<String, String>,
360:     ) -> Result<Vec<plan::ResolvedMigrationStep>, MigrationError> {
361:         let vars = variables.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
362:         let mut steps = Vec::new();
363:         for file in files {
364:             let definition = parse_mermaid_template(&file.content, &vars)
365:                 .map_err(|error| MigrationError::ParseError {
366:                     version: file.version, name: file.name.clone(), detail: error.to_string(),
367:                 })?;
368:             steps.push(plan::ResolvedMigrationStep {
369:                 version: file.version, name: file.name, checksum: file.checksum,
370:                 graph_hash: definition.hash(),
371:             });
372:         }
373:         if steps.is_empty() { return Err(MigrationError::InvalidPlan("no declared migration steps".into())); }
374:         Ok(steps)
375:     }
376: 
377:     // ─── Schema initialization ──────────────────────────────────────────
378: 
379:     /// Ensure migration tables exist.
380:     pub fn initialize(&self, catalog: &Catalog) -> Result<(), MigrationError> {
381:         catalog.migration_initialize()
382:     }
383: 
384:     // ─── Scanning ───────────────────────────────────────────────────────
385: 
386:     /// Scan all registered directories for migration files.
387:     fn scan_all(&self) -> Result<Vec<MigrationFile>, MigrationError> {
388:         let mut all_files = Vec::new();
389:         for dir in &self.migration_dirs {
390:             let mut files = scan_migration_dir(dir)?;
391:             all_files.append(&mut files);
392:         }
393:         all_files.sort_by_key(|f| f.version);
394: 
395:         // Check for cross-directory duplicates
396:         for pair in all_files.windows(2) {
397:             if pair[0].version == pair[1].version {
398:                 return Err(MigrationError::DuplicateVersion {
399:                     version: pair[0].version,
400:                     file_a: pair[0].path.clone(),
401:                     file_b: pair[1].path.clone(),
402:                 });
403:             }
404:         }
405: 
406:         Ok(all_files)
407:     }
408: 
409:     // ─── Status ─────────────────────────────────────────────────────────
410: 
411:     /// Le journal est une entrée du plan, pas une liste de versions à sauter.
412:     /// Même une application passée doit encore désigner le fichier déclaré.
413:     fn checked_history(
414:         &self,
415:         catalog: &Catalog,
416:         files: &[MigrationFile],
417:     ) -> Result<HashMap<u64, AppliedMigration>, MigrationError> {
418:         let history = catalog.migration_load_applied()?;
419:         validate_declared_history(files, history.iter())?;
420:         Ok(history)
421:     }
422: 
423:     /// List all migrations with their current status.
424:     pub fn status(&self, catalog: &Catalog) -> Result<Vec<MigrationStatus>, MigrationError> {
425:         let files = self.scan_all()?;
426:         let applied = self.checked_history(catalog, &files)?;
427: 
428:         let mut statuses = Vec::new();
429:         for file in &files {
430:             let (state, db_checksum, execution_id, applied_at, duration_ms) =
431:                 if let Some(am) = applied.get(&file.version) {
432:                     let state = match am.status.as_str() {
433:                         "applied" => MigrationState::Applied,
434:                         "failed" => MigrationState::Failed,
435:                         "rolled_back" => MigrationState::RolledBack,
436:                         _ => MigrationState::Pending,
437:                     };
438:                     (
439:                         state,
440:                         Some(am.checksum.clone()),
441:                         Some(am.execution_id.clone()),
442:                         Some(am.applied_at),
443:                         Some(am.duration_ms),
444:                     )
445:                 } else {
446:                     (MigrationState::Pending, None, None, None, None)
447:                 };
448: 
449:             statuses.push(MigrationStatus {
450:                 version: file.version,
451:                 name: file.name.clone(),
452:                 state,
453:                 checksum: file.checksum.clone(),
454:                 db_checksum,
455:                 execution_id,
456:                 applied_at,
457:                 duration_ms,
458:             });
459:         }
460: 
461:         Ok(statuses)
462:     }
463: 
464:     /// List pending migrations (not yet applied), sorted by version.
465:     pub fn pending(&self, catalog: &Catalog) -> Result<Vec<MigrationFile>, MigrationError> {
466:         let files = self.scan_all()?;
467:         let applied = self.checked_history(catalog, &files)?;
468: 
469:         Ok(files
470:             .into_iter()
471:             .filter(|f| {
472:                 applied
473:                     .get(&f.version)
474:                     .map_or(true, |am| am.status != "applied")
475:             })
476:             .collect())
477:     }
478: 
479:     // ─── Apply ──────────────────────────────────────────────────────────
480: 
481:     /// Apply pending migrations, optionally up to a target version.
482:     ///
483:     /// If `dry_run` is true, parse and validate each migration but do not execute.
484:     pub fn apply(
485:         &self,
486:         catalog: &mut Catalog,
487:         target_version: Option<u64>,
488:         dry_run: bool,
489:         vars: &HashMap<String, String>,
490:     ) -> Result<Vec<MigrationResult>, MigrationError> {
491:         let mut pending = self.pending(catalog)?;
492:         if let Some(target) = target_version {
493:             pending.retain(|f| f.version <= target);
494:         }
495: 
496:         if pending.is_empty() {
497:             return Ok(Vec::new());
498:         }
499: 
500:         if !dry_run {
501:             catalog.migration_acquire_lock(&self.lock_id)?;
502:         }
503: 
504:         let mut results = Vec::new();
505:         for file in &pending {
506:             let result = if dry_run {
507:                 self.dry_run_migration(file, vars)?
508:             } else {
509:                 self.apply_migration(catalog, file, vars)
510:                     .unwrap_or_else(|e| MigrationResult {
511:                         version: file.version,
512:                         name: file.name.clone(),
513:                         direction: MigrationDirection::Up,
514:                         state: MigrationState::Failed,
515:                         duration_ms: 0,
516:                         error: Some(e.to_string()),
517:                         dry_run_plan: None,
518:                     })
519:             };
520: 
521:             let failed = result.state == MigrationState::Failed;
522:             results.push(result);
523:             if failed {
524:                 break; // Stop on first failure
525:             }
526:         }
527: 
528:         if !dry_run {
529:             catalog.migration_release_lock().ok(); // Best-effort release
530:         }
531: 
532:         Ok(results)
533:     }
534: 
535:     /// Dry-run: parse + validate + check reversibility. No DB needed.
536:     fn dry_run_migration(
537:         &self,
538:         file: &MigrationFile,
539:         vars: &HashMap<String, String>,
540:     ) -> Result<MigrationResult, MigrationError> {
541:         let def = parse_mermaid_template(&file.content, vars)
542:             .map_err(|e| MigrationError::ParseError {
543:                 version: file.version,
544:                 name: file.name.clone(),
545:                 detail: e.to_string(),
546:             })?;
547: 
548:         let graph = DataflowGraph::from_definition(&def, &self.registry)
549:             .map_err(|e| MigrationError::GraphError {
550:                 version: file.version,
551:                 name: file.name.clone(),
552:                 detail: e,
553:             })?;
554: 
555:         let nodes: Vec<DryRunNode> = graph
556:             .nodes
557:             .iter()
558:             .map(|n| DryRunNode {
559:                 name: n.name().to_string(),
560:                 node_type: n.node_type().to_string(),
561:                 config: n.node_config()
562:                     .and_then(|b| b.downcast::<serde_json::Value>().ok())
563:                     .map(|v| *v)
564:                     .unwrap_or_default(),
565:                 can_undo: n.can_undo(),
566:             })
567:             .collect();
568: 
569:         let all_reversible = nodes.iter().all(|n| n.can_undo);
570: 
571:         Ok(MigrationResult {
572:             version: file.version,
573:             name: file.name.clone(),
574:             direction: MigrationDirection::Up,
575:             state: MigrationState::Pending, // dry-run — not applied
576:             duration_ms: 0,
577:             error: None,
578:             dry_run_plan: Some(DryRunPlan {
579:                 node_count: nodes.len(),
580:                 edge_count: graph.edges.len(),
581:                 nodes,
582:                 all_reversible,
583:             }),
584:         })
585:     }
586: 
587:     /// Apply a single migration via Catalog.
588:     fn apply_migration(
589:         &self,
590:         catalog: &mut Catalog,
591:         file: &MigrationFile,
592:         vars: &HashMap<String, String>,
593:     ) -> Result<MigrationResult, MigrationError> {
594:         let start = std::time::Instant::now();
595: 
596:         // Parse
597:         let def = parse_mermaid_template(&file.content, vars)
598:             .map_err(|e| MigrationError::ParseError {
599:                 version: file.version,
600:                 name: file.name.clone(),
601:                 detail: e.to_string(),
602:             })?;
603: 
604:         // Build graph
605:         let mut graph = DataflowGraph::from_definition(&def, &self.registry)
606:             .map_err(|e| MigrationError::GraphError {
607:                 version: file.version,
608:                 name: file.name.clone(),
609:                 detail: e,
610:             })?;
611: 
612:         let execution_id = format!("migration-{:03}_{}", file.version, file.name);
613: 
614:         let exec_result = catalog
615:             .migration_execute_graph(&mut graph, &execution_id);
616: 
617:         let duration_ms = start.elapsed().as_millis() as u64;
618: 
619:         match exec_result {
620:             Ok(_) => {
621:                 catalog.migration_record(
622:                     file, "applied", "up", &execution_id, duration_ms, "",
623:                 )?;
624: 
625:                 Ok(MigrationResult {
626:                     version: file.version,
627:                     name: file.name.clone(),
628:                     direction: MigrationDirection::Up,
629:                     state: MigrationState::Applied,
630:                     duration_ms,
631:                     error: None,
632:                     dry_run_plan: None,
633:                 })
634:             }
635:             Err(e) => {
636:                 catalog.migration_record(
637:                     file, "failed", "up", &execution_id, duration_ms, &e.to_string(),
638:                 )
639:                 .ok(); // Best-effort
640: 
641:                 Err(MigrationError::ExecutionError {
642:                     version: file.version,
643:                     name: file.name.clone(),
644:                     detail: e.to_string(),
645:                 })
646:             }
647:         }
648:     }
649: 
650:     // ─── Rollback ───────────────────────────────────────────────────────
651: 
652:     /// Rollback a previously applied migration.
653:     ///
654:     /// Loads the undo contexts from the checkpoint, reconstructs the nodes,
655:     /// calls undo() in reverse topological order, then auto-drains to rebuild
656:     /// chunks/embeddings/FTS for any restored entities.
657:     pub fn rollback(
658:         &self,
659:         catalog: &mut Catalog,
660:         version: u64,
661:     ) -> Result<MigrationResult, MigrationError> {
662:         let start = std::time::Instant::now();
663: 
664:         // Check it was applied
665:         let applied = catalog.migration_load_applied()?;
666:         let am = applied.get(&version).ok_or(MigrationError::NotApplied { version })?;
667:         if am.status != "applied" {
668:             return Err(MigrationError::NotApplied { version });
669:         }
670: 
671:         catalog.migration_acquire_lock(&self.lock_id)?;
672: 
673:         // Load the checkpoint for this execution
674:         let checkpoint_store = CypherCheckpointStore::new(catalog.conn_arc());
675:         let checkpoint = checkpoint_store
676:             .load_execution(&am.execution_id)
677:             .map_err(|e| MigrationError::DbError(e))?
678:             .ok_or_else(|| MigrationError::DbError(
679:                 format!("checkpoint not found for execution '{}'", am.execution_id)
680:             ))?;
681: 
682:         // Rebuild the graph to get the nodes + topological order
683:         let mut graph = DataflowGraph::from_definition(&checkpoint.graph_def, &self.registry)
684:             .map_err(|e| MigrationError::GraphError {
685:                 version,
686:                 name: am.name.clone(),
687:                 detail: e,
688:             })?;
689: 
690:         // Check all nodes are reversible
691:         let non_reversible: Vec<String> = graph
692:             .nodes
693:             .iter()
694:             .filter(|n| !n.can_undo())
695:             .map(|n| n.name().to_string())
696:             .collect();
697:         if !non_reversible.is_empty() {
698:             catalog.migration_release_lock().ok();
699:             return Err(MigrationError::NotReversible {
700:                 version,
701:                 name: am.name.clone(),
702:                 nodes: non_reversible,
703:             });
704:         }
705: 
706:         // Rollback via Catalog (undo + auto-drain)
707:         let am_name = am.name.clone();
708:         let am_execution_id = am.execution_id.clone();
709: 
710:         catalog.migration_rollback_graph(&mut graph, &checkpoint)
711:             .map_err(|e| MigrationError::ExecutionError {
712:                 version,
713:                 name: am_name.clone(),
714:                 detail: e.to_string(),
715:             })?;
716: 
717:         let duration_ms = start.elapsed().as_millis() as u64;
718: 
719:         // Record the rollback
720:         let files = self.scan_all()?;
721:         let file = files.iter().find(|f| f.version == version);
722: 
723:         if let Some(file) = file {
724:             catalog.migration_record(
725:                 file, "rolled_back", "down", &am_execution_id, duration_ms, "",
726:             )?;
727:         } else {
728:             catalog.migration_update_status(version, "rolled_back")?;
729:         }
730: 
731:         catalog.migration_release_lock().ok();
732: 
733:         Ok(MigrationResult {
734:             version,
735:             name: am_name,
736:             direction: MigrationDirection::Down,
737:             state: MigrationState::RolledBack,
738:             duration_ms,
739:             error: None,
740:             dry_run_plan: None,
741:         })
742:     }
743: 
744:     // ─── Check reversibility ────────────────────────────────────────────
745: 
746:     /// Check which pending migrations are fully reversible.
747:     pub fn check_reversible(
748:         &self,
749:         catalog: &Catalog,
750:         vars: &HashMap<String, String>,
751:     ) -> Result<Vec<(MigrationFile, bool)>, MigrationError> {
752:         let pending = self.pending(catalog)?;
753:         let mut results = Vec::new();
754: 
755:         for file in pending {
756:             let def = parse_mermaid_template(&file.content, vars)
757:                 .map_err(|e| MigrationError::ParseError {
758:                     version: file.version,
759:                     name: file.name.clone(),
760:                     detail: e.to_string(),
761:                 })?;
762: 
763:             let graph = DataflowGraph::from_definition(&def, &self.registry)
764:                 .map_err(|e| MigrationError::GraphError {
765:                     version: file.version,
766:                     name: file.name.clone(),
767:                     detail: e,
768:                 })?;
769: 
770:             let reversible = graph.nodes.iter().all(|n| n.can_undo());
771:             results.push((file, reversible));
772:         }
773: 
774:         Ok(results)
775:     }
776: }
777: 
778: // ─── Internal types ─────────────────────────────────────────────────────────
779: 
780: #[derive(Debug)]
781: pub(crate) struct AppliedMigration {
782:     pub(crate) name: String,
783:     pub(crate) status: String,
784:     pub(crate) checksum: String,
785:     pub(crate) execution_id: String,
786:     pub(crate) applied_at: u64,
787:     pub(crate) duration_ms: u64,
788:     #[allow(dead_code)]
789:     pub(crate) error: String,
790: }
791: 
792: // ─── Tests ──────────────────────────────────────────────────────────────────
793: 
794: #[cfg(test)]
795: mod tests {
796:     use super::*;
797:     const PREFLIGHT_MIGRATION_SOURCE: &str = "graph TD\ncheck[\"ValidateNode(query='RETURN 1', assert='count == 1')\"]\n";
798: 
799:     // T12 : exercer le lecteur et le runner existants avant de corriger
800:     // la qualification, sans initialiser le catalogue ni créer son schéma.
801:     fn pending_history_fixture(row: Vec<crate::connection::CypherValue>)
802:         -> (Result<Vec<MigrationFile>, MigrationError>, usize, usize)
803:     {
804:         use crate::connection::{CallbackConnection, QueryResult};
805:         use std::sync::atomic::{AtomicUsize, Ordering};
806:         let dir = tempfile::tempdir().unwrap();
807:         std::fs::write(dir.path().join("001_application.mmd"), PREFLIGHT_MIGRATION_SOURCE).unwrap();
808:         let effects = Arc::new(AtomicUsize::new(0));
809:         let model_calls = Arc::new(AtomicUsize::new(0));
810:         let e = effects.clone();
811:         let conn = CallbackConnection::new(move |query, _| {
812:             if !query.trim_start().starts_with("MATCH") || !query.contains("_DataflowMigration") {
813:                 e.fetch_add(1, Ordering::SeqCst);
814:                 return Err(crate::connection::DbError::QueryError("inspection : effet interdit".into()));
815:             }
816:             Ok(QueryResult { columns: vec![], rows: vec![row.clone()] })
817:         });
818:         let m = model_calls.clone();
819:         let embedder = crate::embedder::CallbackEmbedder::new(4, move |texts| {
820:             m.fetch_add(texts.len(), Ordering::SeqCst);
821:             Ok(texts.iter().map(|_| vec![0.0; 4]).collect())
822:         });
823:         let catalog = Catalog::new(Box::new(conn), Box::new(embedder), crate::CatalogConfig::default());
824:         let mut runner = MigrationRunner::new(Arc::new(NodeRegistry::new()));
825:         runner.add_dir(dir.path().to_path_buf());
826:         let result = runner.pending(&catalog);
827:         (result, effects.load(Ordering::SeqCst), model_calls.load(Ordering::SeqCst))
828:     }
829: 
830:     fn history_row(version: crate::connection::CypherValue, status: &str) -> Vec<crate::connection::CypherValue> {
831:         use crate::connection::CypherValue as V;
832:         vec![version, V::String("application".into()), V::String(status.into()),
833:              V::String(content_hash(PREFLIGHT_MIGRATION_SOURCE)), V::String("migration-001_application".into()),
834:              V::Int(1), V::Int(1), V::String(String::new())]
835:     }
836: 
837:     #[test]
838:     fn preflight_refuses_applied_checksum_drift_without_effects() {
839:         let mut row = history_row(crate::connection::CypherValue::Int(1), "applied");
840:         row[3] = crate::connection::CypherValue::String(content_hash("ancienne migration"));
841:         let (result, effects, models) = pending_history_fixture(row);
842:         assert_eq!((effects, models), (0, 0));
843:         assert!(result.is_err(), "une migration appliquée modifiée ne se saute pas : {result:?}");
844:     }
845: 
846:     #[test]
847:     fn preflight_accepts_identical_legacy_applied_history_without_effects() {
848:         let (result, effects, models) = pending_history_fixture(history_row(crate::connection::CypherValue::Int(1), "applied"));
849:         assert_eq!((effects, models), (0, 0));
850:         assert!(result.unwrap().is_empty(), "une application identique ne se rejoue pas");
851:     }
852: 
853:     #[test]
854:     fn preflight_refuses_orphan_history_without_effects() {
855:         let (result, effects, models) = pending_history_fixture(history_row(crate::connection::CypherValue::Int(99), "applied"));
856:         assert_eq!((effects, models), (0, 0));
857:         assert!(result.is_err(), "une version sans fichier déclaré doit être nommée : {result:?}");
858:     }
859: 
860:     #[test]
861:     fn preflight_refuses_unknown_history_state_without_effects() {
862:         let (result, effects, models) = pending_history_fixture(history_row(crate::connection::CypherValue::Int(1), "unknown_future_state"));
863:         assert_eq!((effects, models), (0, 0));
864:         assert!(result.is_err(), "un statut inconnu ne devient pas pending : {result:?}");
865:     }
866: 
867:     #[test]
868:     fn preflight_refuses_malformed_history_version_without_effects() {
869:         let (result, effects, models) = pending_history_fixture(history_row(crate::connection::CypherValue::String("not-a-version".into()), "applied"));
870:         assert_eq!((effects, models), (0, 0));
871:         assert!(result.is_err(), "une version mal typée ne devient pas zéro : {result:?}");
872:     }
873: 
874:     // ── Filename parsing ────────────────────────────────────────────────
875: 
876:     #[test]
877:     fn parse_valid_filename() {
878:         let (v, name) = parse_migration_filename("001_add_version.mmd").unwrap();
879:         assert_eq!(v, 1);
880:         assert_eq!(name, "add_version");
881:     }
882: 
883:     #[test]
884:     fn parse_zero_padded_version() {
885:         let (v, name) = parse_migration_filename("042_rename_field.mmd").unwrap();
886:         assert_eq!(v, 42);
887:         assert_eq!(name, "rename_field");
888:     }
889: 
890:     #[test]
891:     fn parse_large_version() {
892:         let (v, name) = parse_migration_filename("1000_big_change.mmd").unwrap();
893:         assert_eq!(v, 1000);
894:         assert_eq!(name, "big_change");
895:     }
896: 
897:     #[test]
898:     fn parse_invalid_no_underscore() {
899:         assert!(parse_migration_filename("001addversion.mmd").is_none());
900:     }
901: 
902:     #[test]
903:     fn parse_invalid_no_extension() {
904:         assert!(parse_migration_filename("001_add_version.txt").is_none());
905:     }
906: 
907:     #[test]
908:     fn parse_invalid_non_numeric_version() {
909:         assert!(parse_migration_filename("abc_add_version.mmd").is_none());
910:     }
911: 
912:     #[test]
913:     fn parse_invalid_empty_name() {
914:         assert!(parse_migration_filename("001_.mmd").is_none());
915:     }
916: 
917:     #[test]
918:     fn parse_name_with_underscores() {
919:         let (v, name) = parse_migration_filename("003_add_embedding_index.mmd").unwrap();
920:         assert_eq!(v, 3);
921:         assert_eq!(name, "add_embedding_index");
922:     }
923: 
924:     // ── Directory scanning ──────────────────────────────────────────────
925: 
926:     #[test]
927:     fn scan_empty_dir() {
928:         let dir = tempfile::tempdir().unwrap();
929:         let files = scan_migration_dir(dir.path()).unwrap();
930:         assert!(files.is_empty());
931:     }
932: 
933:     #[test]
934:     fn scan_nonexistent_dir() {
935:         let files = scan_migration_dir(Path::new("/nonexistent/path")).unwrap();
936:         assert!(files.is_empty());
937:     }
938: 
939:     #[test]
940:     fn scan_dir_with_migrations() {
941:         let dir = tempfile::tempdir().unwrap();
942: 
943:         // Create migration files (out of order on disk)
944:         let content_2 = "graph LR\n    step[\"CypherNode(query='RETURN 2')\"]\n";
945:         let content_1 = "graph LR\n    step[\"CypherNode(query='RETURN 1')\"]\n";
946: 
947:         std::fs::write(dir.path().join("002_second.mmd"), content_2).unwrap();
948:         std::fs::write(dir.path().join("001_first.mmd"), content_1).unwrap();
949: 
950:         let files = scan_migration_dir(dir.path()).unwrap();
951:         assert_eq!(files.len(), 2);
952:         assert_eq!(files[0].version, 1);
953:         assert_eq!(files[0].name, "first");
954:         assert_eq!(files[1].version, 2);
955:         assert_eq!(files[1].name, "second");
956:     }
957: 
958:     #[test]
959:     fn scan_ignores_non_mmd_files() {
960:         let dir = tempfile::tempdir().unwrap();
961:         std::fs::write(dir.path().join("001_first.mmd"), "graph LR\n").unwrap();
962:         std::fs::write(dir.path().join("README.md"), "# readme").unwrap();
963:         std::fs::write(dir.path().join("notes.txt"), "notes").unwrap();
964: 
965:         let files = scan_migration_dir(dir.path()).unwrap();
966:         assert_eq!(files.len(), 1);
967:     }
968: 
969:     #[test]
970:     fn scan_detects_duplicate_versions() {
971:         let dir = tempfile::tempdir().unwrap();
972:         std::fs::write(dir.path().join("001_first.mmd"), "graph LR\n").unwrap();
973:         std::fs::write(dir.path().join("001_duplicate.mmd"), "graph LR\n").unwrap();
974: 
975:         let err = scan_migration_dir(dir.path()).unwrap_err();
976:         assert!(err.to_string().contains("duplicate version 1"));
977:     }
978: 
979:     #[test]
980:     fn scan_checksum_is_content_hash() {
981:         let dir = tempfile::tempdir().unwrap();
982:         let content = "graph LR\n    step[\"CypherNode(query='RETURN 1')\"]\n";
983:         std::fs::write(dir.path().join("001_test.mmd"), content).unwrap();
984: 
985:         let files = scan_migration_dir(dir.path()).unwrap();
986:         assert_eq!(files[0].checksum, content_hash(content));
987:     }
988: 
989:     // ── MigrationState display ──────────────────────────────────────────
990: 
991:     #[test]
992:     fn migration_state_display() {
993:         assert_eq!(MigrationState::Pending.to_string(), "pending");
994:         assert_eq!(MigrationState::Applied.to_string(), "applied");
995:         assert_eq!(MigrationState::Failed.to_string(), "failed");
996:         assert_eq!(MigrationState::RolledBack.to_string(), "rolled_back");
997:     }
998: 
999:     // ── MigrationDirection display ──────────────────────────────────────
1000: 
1001:     #[test]
1002:     fn migration_direction_display() {
1003:         assert_eq!(MigrationDirection::Up.to_string(), "up");
1004:         assert_eq!(MigrationDirection::Down.to_string(), "down");
1005:     }
1006: 
1007:     // ── MigrationError display ──────────────────────────────────────────
1008: 
1009:     #[test]
1010:     fn error_display_locked() {
1011:         let err = MigrationError::Locked {
1012:             by: "runner-abc".into(),
1013:             since: 1000,
1014:         };
1015:         assert!(err.to_string().contains("locked by 'runner-abc'"));
1016:     }
1017: 
1018:     #[test]
1019:     fn error_display_not_reversible() {
1020:         let err = MigrationError::NotReversible {
1021:             version: 1,
1022:             name: "test".into(),
1023:             nodes: vec!["step1".into(), "step2".into()],
1024:         };
1025:         let s = err.to_string();
1026:         assert!(s.contains("not reversible"));
1027:         assert!(s.contains("step1, step2"));
1028:     }
1029: 
1030:     #[test]
1031:     fn error_display_duplicate_version() {
1032:         let err = MigrationError::DuplicateVersion {
1033:             version: 1,
1034:             file_a: PathBuf::from("a.mmd"),
1035:             file_b: PathBuf::from("b.mmd"),
1036:         };
1037:         assert!(err.to_string().contains("duplicate version 1"));
1038:     }
1039: 
1040:     // ── Internal migration template tests ────────────────────────────────
1041: 
1042:     fn builtin_registry() -> super::NodeRegistry {
1043:         let mut registry = super::NodeRegistry::new();
1044:         super::super::node_factories::register_builtins(&mut registry);
1045:         registry
1046:     }
1047: 
1048:     #[test]
1049:     fn internal_001_parses_and_builds() {
1050:         let mmd = include_str!("../../migrations/internal/001_create_dataflow_tables.mmd");
1051:         let def = super::parse_mermaid_template(mmd, &HashMap::new()).unwrap();
1052: 
1053:         assert_eq!(def.nodes.len(), 4, "expected 4 CypherNode declarations");
1054:         assert_eq!(def.edges.len(), 3, "expected 3 sequential edges");
1055: 
1056:         // All nodes are CypherNode
1057:         for node in &def.nodes {
1058:             assert_eq!(node.node_type, "CypherNode", "node '{}' should be CypherNode", node.name);
1059:             assert!(
1060:                 node.config["query"].as_str().unwrap().contains("CREATE NODE TABLE IF NOT EXISTS"),
1061:                 "node '{}' query should be CREATE NODE TABLE",
1062:                 node.name,
1063:             );
1064:         }
1065: 
1066:         // Build the graph via registry
1067:         let registry = builtin_registry();
1068:         let graph = super::DataflowGraph::from_definition(&def, &registry).unwrap();
1069:         assert_eq!(graph.node_names().len(), 4);
1070: 
1071:         // Topological order: sequential chain
1072:         let order = graph.topological_sort().unwrap();
1073:         assert_eq!(order[0], "create_execution");
1074:         assert_eq!(order[1], "create_node_state");
1075:         assert_eq!(order[2], "create_migration");
1076:         assert_eq!(order[3], "create_lock");
1077:     }
1078: 
1079:     #[test]
1080:     fn internal_001_not_reversible() {
1081:         let mmd = include_str!("../../migrations/internal/001_create_dataflow_tables.mmd");
1082:         let def = super::parse_mermaid_template(mmd, &HashMap::new()).unwrap();
1083:         let registry = builtin_registry();
1084:         let graph = super::DataflowGraph::from_definition(&def, &registry).unwrap();
1085: 
1086:         // DDL migrations have no capture_query → not reversible
1087:         for node in &graph.nodes {
1088:             assert!(
1089:                 !node.can_undo(),
1090:                 "DDL node '{}' should not be reversible",
1091:                 node.name(),
1092:             );
1093:         }
1094:     }
1095: 
1096:     #[test]
1097:     fn internal_001_tables_match_checkpoint_store() {
1098:         // Verify the migration creates the same tables as CypherCheckpointStore::initialize()
1099:         let mmd = include_str!("../../migrations/internal/001_create_dataflow_tables.mmd");
1100:         let def = super::parse_mermaid_template(mmd, &HashMap::new()).unwrap();
1101: 
1102:         let table_names: Vec<&str> = def
1103:             .nodes
1104:             .iter()
1105:             .filter_map(|n| {
1106:                 let query = n.config["query"].as_str()?;
1107:                 let start = query.find("_Dataflow")?;
1108:                 let end = query[start..].find('(')? + start;
1109:                 Some(&query[start..end])
1110:             })
1111:             .collect();
1112: 
1113:         assert!(table_names.contains(&"_DataflowExecution"));
1114:         assert!(table_names.contains(&"_DataflowNodeState"));
1115:         assert!(table_names.contains(&"_DataflowMigration"));
1116:         assert!(table_names.contains(&"_DataflowMigrationLock"));
1117:     }
1118: 
1119:     #[test]
1120:     fn internal_001_scan_from_disk() {
1121:         // Verify the migration file is properly scannable
1122:         let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
1123:             .join("migrations")
1124:             .join("internal");
1125:         let files = scan_migration_dir(&dir).unwrap();
1126:         assert_eq!(files.len(), 1);
1127:         assert_eq!(files[0].version, 1);
1128:         assert_eq!(files[0].name, "create_dataflow_tables");
1129:         assert!(!files[0].checksum.is_empty());
1130:     }
1131: }
1132: 
1133: /// Strict journal reader shared by catalog execution and metadata-only inspection.
1134: pub(crate) fn read_history_on(
1135:     conn: &dyn crate::connection::DbConnection,
1136:     dialect: &dyn crate::dialect::SchemaDialect,
1137: ) -> Result<HashMap<u64, AppliedMigration>, MigrationError> {
1138:         let query = dialect.select_all(
1139:             "_DataflowMigration",
1140:             &["version", "name", "status", "checksum", "execution_id", "applied_at", "duration_ms", "error"],
1141:             Some("version"),
1142:         );
1143:         let result = conn.execute(&query)
1144:             .map_err(|e| MigrationError::DbError(e.to_string()))?;
1145: 
1146:         let mut applied = HashMap::new();
1147:         for row in &result.rows {
1148:             let invalid = |detail: String| MigrationError::InvalidHistory { version: None, detail };
1149:             if row.len() != 8 {
1150:                 return Err(invalid(format!("expected 8 columns, got {}", row.len())));
1151:             }
1152:             let version = row[0].as_i64().filter(|v| *v >= 0)
1153:                 .ok_or_else(|| invalid("version must be a nonnegative integer".into()))? as u64;
1154:             let invalid = |detail: String| MigrationError::InvalidHistory { version: Some(version), detail };
1155:             let required_text = |index: usize, field: &str| -> Result<String, MigrationError> {
1156:                 row[index].as_str().filter(|s| !s.trim().is_empty()).map(str::to_string)
1157:                     .ok_or_else(|| invalid(format!("{field} must be nonempty text")))
1158:             };
1159:             let name = required_text(1, "name")?;
1160:             let status = required_text(2, "status")?;
1161:             if !matches!(status.as_str(), "pending" | "applied" | "failed" | "rolled_back") {
1162:                 return Err(invalid(format!("unknown status {status}")));
1163:             }
1164:             let checksum = required_text(3, "checksum")?;
1165:             let execution_id = required_text(4, "execution_id")?;
1166:             let nonnegative = |index: usize, field: &str| -> Result<u64, MigrationError> {
1167:                 row[index].as_i64().filter(|v| *v >= 0).map(|v| v as u64)
1168:                     .ok_or_else(|| invalid(format!("{field} must be a nonnegative integer")))
1169:             };
1170:             let applied_at = nonnegative(5, "applied_at")?;
1171:             let duration_ms = nonnegative(6, "duration_ms")?;
1172:             let error = row[7].as_str().ok_or_else(|| invalid("error must be text".into()))?.to_string();
1173:             if applied.contains_key(&version) {
1174:                 return Err(invalid("duplicate history version".into()));
1175:             }
1176: 
1177:             applied.insert(
1178:                 version,
1179:                 AppliedMigration {
1180:                     name,
1181:                     status,
1182:                     checksum,
1183:                     execution_id,
1184:                     applied_at,
1185:                     duration_ms,
1186:                     error,
1187:                 },
1188:             );
1189:         }
1190: 
1191:         Ok(applied)
1192: }
1193: 
1194: /// Shared file/history identity checks. Execution policy is separate: legacy
1195: /// pending may skip an applied row, a resolved public plan cannot qualify it.
1196: fn validate_declared_history<'a>(
1197:     files: &[MigrationFile],
1198:     history: impl IntoIterator<Item = (&'a u64, &'a AppliedMigration)>,
1199: ) -> Result<(), MigrationError> {
1200:         for (&version, entry) in history {
1201:             let invalid = |detail: String| MigrationError::InvalidHistory { version: Some(version), detail };
1202:             let file = files.iter().find(|f| f.version == version)
1203:                 .ok_or_else(|| invalid("orphan entry: no declared migration file".into()))?;
1204:             if entry.name != file.name {
1205:                 return Err(invalid(format!("name mismatch: stored={}, declared={}", entry.name, file.name)));
1206:             }
1207:             if entry.checksum != file.checksum {
1208:                 return Err(invalid("checksum mismatch: the recorded migration file has changed".into()));
1209:             }
1210:         }
1211:     Ok(())
1212: }

SOURCE d2cb7f0636d099823f63de2cef674b630a46bc6f:extension/rag3weaver/src/dataflow/migrations/plan.rs
1: //! Immutable migration identity, independent of the current storage generation.
2: //! This contract grants neither write access nor permission to publish a target.
3: use std::collections::BTreeMap;
4: use serde::{Deserialize, Serialize};
5: use crate::hash::content_hash;
6: 
7: #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
8: #[serde(tag = "state", content = "fingerprint", rename_all = "snake_case")]
9: pub enum StorageGeneration {
10:     /// Legacy metadata has no established data generation. Never means zero.
11:     Unversioned,
12:     Established(String),
13: }
14: 
15: /// Engine, internal schema and application format have separate identities.
16: #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
17: #[serde(deny_unknown_fields)]
18: pub struct StorageDescriptor {
19:     pub engine_format: String,
20:     pub internal_schema: String,
21:     pub application_format: String,
22:     pub recipe: String,
23:     pub config_fingerprint: String,
24:     pub generation: StorageGeneration,
25: }
26: 
27: impl StorageDescriptor {
28:     pub(super) fn validate(&self) -> Result<(), String> {
29:         for (name, value) in [
30:             ("engine_format", &self.engine_format),
31:             ("internal_schema", &self.internal_schema),
32:             ("application_format", &self.application_format),
33:             ("recipe", &self.recipe),
34:             ("config_fingerprint", &self.config_fingerprint),
35:         ] {
36:             if value.trim().is_empty() { return Err(format!("empty {name}")); }
37:         }
38:         if let StorageGeneration::Established(value) = &self.generation {
39:             if value.trim().is_empty() { return Err("empty generation fingerprint".into()); }
40:         }
41:         Ok(())
42:     }
43: }
44: 
45: /// Inputs fixed when planning. Do not replace source with current target metadata
46: /// when qualifying a replay after publication.
47: #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
48: #[serde(deny_unknown_fields)]
49: pub struct MigrationPlanInputs {
50:     pub source: StorageDescriptor,
51:     pub target: StorageDescriptor,
52:     pub variables: BTreeMap<String, String>,
53:     /// Fingerprint of the provider's resolved schema facts, not its name alone.
54:     pub schema_facts_fingerprint: String,
55: }
56: 
57: #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
58: pub struct ResolvedMigrationStep {
59:     pub version: u64,
60:     pub name: String,
61:     pub checksum: String,
62:     pub graph_hash: String,
63: }
64: 
65: #[derive(Debug, Clone, Serialize)]
66: pub struct ResolvedMigrationPlan {
67:     inputs: MigrationPlanInputs,
68:     steps: Vec<ResolvedMigrationStep>,
69:     identity: String,
70: }
71: 
72: #[derive(Debug, Clone, Copy, PartialEq, Eq)]
73: pub enum StoragePosition { Source, PublishedTarget }
74: 
75: /// State of this fixed plan, read from a qualified durable journal. In-flight
76: /// and failed operations cannot be represented as the absence of an applied ID.
77: /// This type does not write or upgrade the existing legacy journal.
78: #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
79: #[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
80: pub enum PlanJournalState {
81:     Unapplied,
82:     Running { identity: String },
83:     Failed { identity: String },
84:     Applied { identity: String },
85:     RolledBack { identity: String },
86: }
87: 
88: impl ResolvedMigrationPlan {
89:     pub(super) fn new(inputs: MigrationPlanInputs, steps: Vec<ResolvedMigrationStep>) -> Result<Self, String> {
90:         inputs.source.validate()?;
91:         inputs.target.validate()?;
92:         if inputs.schema_facts_fingerprint.trim().is_empty() {
93:             return Err("missing resolved schema facts fingerprint".into());
94:         }
95:         if steps.is_empty() { return Err("no declared migration steps".into()); }
96:         let encoded = serde_json::to_string(&("resolved-migration-plan-v1", &inputs, &steps))
97:             .map_err(|e| e.to_string())?;
98:         Ok(Self { inputs, steps, identity: content_hash(&encoded) })
99:     }
100: 
101:     pub fn identity(&self) -> &str { &self.identity }
102:     pub fn inputs(&self) -> &MigrationPlanInputs { &self.inputs }
103:     pub fn steps(&self) -> &[ResolvedMigrationStep] { &self.steps }
104: 
105:     /// An inspectable preliminary plan is allowed for legacy storage. Execution
106:     /// qualification requires established source AND target generations. This
107:     /// checks storage identity only, not node registry, ports, cycles or transport.
108:     pub fn validate_generation_preconditions(&self) -> Result<(), String> {
109:         if matches!(self.inputs.source.generation, StorageGeneration::Unversioned) {
110:             return Err("source generation not established".into());
111:         }
112:         if matches!(self.inputs.target.generation, StorageGeneration::Unversioned) {
113:             return Err("target generation not established".into());
114:         }
115:         if self.inputs.source == self.inputs.target {
116:             return Err("source and target descriptors are identical".into());
117:         }
118:         Ok(())
119:     }
120: 
121:     /// Qualification is pure. The caller must obtain the durable journal state
122:     /// and storage descriptor under actual exclusivity before any effects.
123:     /// The journal state refers to durable publication, never a checkpoint's
124:     /// Completed flag. This method neither reads nor creates that record.
125:     pub fn qualify_storage(&self, current: &StorageDescriptor, journal: &PlanJournalState)
126:         -> Result<StoragePosition, String>
127:     {
128:         self.validate_generation_preconditions()?;
129:         match journal {
130:             PlanJournalState::Unapplied if current == &self.inputs.source => Ok(StoragePosition::Source),
131:             PlanJournalState::Applied { identity } if identity == &self.identity && current == &self.inputs.target =>
132:                 Ok(StoragePosition::PublishedTarget),
133:             PlanJournalState::Running { .. } | PlanJournalState::Failed { .. } | PlanJournalState::RolledBack { .. } =>
134:                 Err("migration reconciliation required".into()),
135:             PlanJournalState::Applied { identity } if identity != &self.identity =>
136:                 Err("resolved migration plan changed".into()),
137:             _ => Err("storage does not match the generation expected by the fixed plan".into()),
138:         }
139:     }
140: }
141: 
142: #[cfg(test)]
143: mod tests {
144:     use super::*;
145:     use crate::dataflow::{migrations::MigrationRunner, node_registry::NodeRegistry};
146:     use std::sync::Arc;
147: 
148:     fn descriptor(format: &str, generation: &str) -> StorageDescriptor {
149:         StorageDescriptor {
150:             engine_format: "fixture-engine".into(), internal_schema: "8".into(),
151:             application_format: format.into(), recipe: "fixture-recipe".into(),
152:             config_fingerprint: format!("configs-{format}"),
153:             generation: StorageGeneration::Established(generation.into()),
154:         }
155:     }
156: 
157:     fn inputs() -> MigrationPlanInputs {
158:         MigrationPlanInputs {
159:             source: descriptor("legacy", "source-data"),
160:             target: descriptor("shared", "target-data"),
161:             variables: BTreeMap::from([("query".into(), "RETURN 1".into())]),
162:             schema_facts_fingerprint: "fixture-provider-facts".into(),
163:         }
164:     }
165: 
166:     fn resolve(inputs: MigrationPlanInputs) -> ResolvedMigrationPlan {
167:         let dir = tempfile::tempdir().unwrap();
168:         std::fs::write(dir.path().join("001_check.mmd"),
169:             "graph TD\ncheck[\"ValidateNode(query='$query', assert='count == 1')\"]\n").unwrap();
170:         let mut runner = MigrationRunner::new(Arc::new(NodeRegistry::new()));
171:         runner.add_dir(dir.path().to_owned());
172:         runner.resolve_plan(inputs).unwrap()
173:     }
174: 
175:     #[test]
176:     fn fixed_plan_qualifies_source_then_published_target_without_reresolving() {
177:         let plan = resolve(inputs());
178:         let identity = plan.identity().to_owned();
179:         assert_eq!(plan.qualify_storage(&plan.inputs().source, &PlanJournalState::Unapplied).unwrap(), StoragePosition::Source);
180:         assert_eq!(plan.qualify_storage(&plan.inputs().target, &PlanJournalState::Applied { identity: identity.clone() }).unwrap(), StoragePosition::PublishedTarget);
181:         assert_eq!(plan.identity(), identity);
182:         for journal in [PlanJournalState::Running { identity: identity.clone() },
183:                         PlanJournalState::Failed { identity: identity.clone() },
184:                         PlanJournalState::RolledBack { identity: identity.clone() }] {
185:             assert!(plan.qualify_storage(&plan.inputs().source, &journal).is_err());
186:             assert!(plan.qualify_storage(&plan.inputs().target, &journal).is_err());
187:         }
188:         // Persist original inputs, then rebuild after publication. The live
189:         // target is used only for qualification, never as the new source.
190:         let persisted = serde_json::to_string(plan.inputs()).unwrap();
191:         let restored = resolve(serde_json::from_str(&persisted).unwrap());
192:         assert_eq!(restored.identity(), identity);
193:         assert_eq!(restored.qualify_storage(&plan.inputs().target, &PlanJournalState::Applied { identity: identity.clone() }).unwrap(), StoragePosition::PublishedTarget);
194:         let mut foreign = plan.inputs().target.clone();
195:         foreign.generation = StorageGeneration::Established("foreign-write".into());
196:         assert!(plan.qualify_storage(&foreign, &PlanJournalState::Applied { identity: identity.clone() }).is_err());
197:         assert!(plan.qualify_storage(&plan.inputs().target, &PlanJournalState::Unapplied).is_err());
198:         assert!(plan.qualify_storage(&plan.inputs().source, &PlanJournalState::Applied { identity: identity.clone() }).is_err());
199:     }
200: 
201:     #[test]
202:     fn resolved_identity_covers_variables_configs_facts_and_generations() {
203:         let original = inputs();
204:         let plan = resolve(original.clone());
205:         let mut variants = Vec::new();
206:         let mut changed = original.clone(); changed.variables.insert("query".into(), "RETURN 2".into()); variants.push(changed);
207:         let mut changed = original.clone(); changed.source.config_fingerprint.push('x'); variants.push(changed);
208:         let mut changed = original.clone(); changed.schema_facts_fingerprint.push('x'); variants.push(changed);
209:         let mut changed = original.clone(); changed.target.recipe.push('x'); variants.push(changed);
210:         let mut changed = original.clone(); changed.target.generation = StorageGeneration::Established("other".into()); variants.push(changed);
211:         for changed in variants {
212:             let other = resolve(changed);
213:             assert_ne!(plan.identity(), other.identity());
214:             assert!(other.qualify_storage(&other.inputs().target, &PlanJournalState::Applied { identity: plan.identity().to_owned() }).is_err());
215:         }
216:         let mut changed_variables = original.clone();
217:         changed_variables.variables.insert("query".into(), "RETURN 2".into());
218:         assert_ne!(plan.steps()[0].graph_hash, resolve(changed_variables).steps()[0].graph_hash);
219:         assert_eq!(plan.identity(), resolve(original).identity());
220:     }
221: 
222:     #[test]
223:     fn legacy_unversioned_is_inspectable_but_never_execution_ready() {
224:         let mut inputs = inputs();
225:         inputs.source.generation = StorageGeneration::Unversioned;
226:         let plan = resolve(inputs);
227:         assert!(!plan.steps().is_empty());
228:         assert_eq!(plan.validate_generation_preconditions().unwrap_err(), "source generation not established");
229:         assert!(plan.qualify_storage(&plan.inputs().source, &PlanJournalState::Unapplied).is_err());
230:     }
231: }

SOURCE d2cb7f0636d099823f63de2cef674b630a46bc6f:extension/rag3weaver/src/dataflow/migrations/transfer.rs
1: //! Pure preview over supplied facts, never a storage-qualified execution plan.
2: //!
3: //! No connection, model, writer or publication capability enters this module.
4: //! An asserted inventory origin is not proof read from storage. In particular,
5: //! the current resolved plan does not produce a durable source instance ID.
6: use std::collections::{BTreeMap, BTreeSet};
7: 
8: use serde::{Serialize, Serializer};
9: 
10: use crate::{
11:     chunker::{Chunk, Chunker, ChunkerConfig},
12:     config::{DerivedIdentity, EntityConfig},
13:     connection::CypherValue,
14:     dataflow::derive_nodes::{derived_content_uuid, render_context, render_fields, render_inputs_hash},
15:     embedding_storage::{EmbeddingModelEntry, StorageKind, VectorStorage},
16:     hash::content_hash,
17:     scope::Scope,
18:     uuid::chunk_uuid,
19: };
20: use super::plan::{ResolvedMigrationPlan, StorageDescriptor};
21: 
22: #[derive(Debug, Clone, PartialEq, Serialize)]
23: pub struct EntityKey {
24:     pub entity: String,
25:     pub scope: Scope,
26:     pub uuid: String,
27: }
28: impl EntityKey {
29:     fn key(&self) -> (String, String, String, String) {
30:         (self.entity.clone(), self.scope.org.clone(), self.scope.project.clone(), self.uuid.clone())
31:     }
32: }
33: 
34: /// Full occurrence values (including provenance/positions) and visibility facts
35: /// are retained. Other graph entities may be included as relation endpoints.
36: #[derive(Debug, Clone, PartialEq, Serialize)]
37: pub struct InventoryEntity {
38:     pub key: EntityKey,
39:     pub values: BTreeMap<String, CypherValue>,
40:     pub masks: BTreeSet<String>,
41: }
42: 
43: /// Ordered endpoints preserve direction; repeated entries preserve multiplicity.
44: #[derive(Debug, Clone, PartialEq, Serialize)]
45: pub struct InventoryRelation {
46:     pub name: String,
47:     pub from: EntityKey,
48:     pub to: EntityKey,
49:     pub properties: BTreeMap<String, CypherValue>,
50: }
51: 
52: #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
53: pub struct ChunkShape {
54:     pub field: String,
55:     pub title: String,
56:     pub content_offset: usize,
57:     #[serde(serialize_with = "serialize_chunk")]
58:     pub chunk: Chunk,
59: }
60: fn serialize_chunk<S: Serializer>(c: &Chunk, s: S) -> Result<S::Ok, S::Error> {
61:     (&c.text, c.index, c.start_byte, c.end_byte, c.start_line, c.end_line,
62:         c.core_start_byte, c.core_end_byte, c.core_start_line, c.core_end_line).serialize(s)
63: }
64: fn serialize_bits<S: Serializer>(values: &[f32], s: S) -> Result<S::Ok, S::Error> {
65:     values.iter().map(|f| f.to_bits()).collect::<Vec<_>>().serialize(s)
66: }
67: 
68: #[derive(Debug, Clone, Serialize)]
69: pub struct KnownDenseValue {
70:     pub model: String,
71:     /// Only the default `_text` input is supported in this first preview.
72:     /// None means its historical provenance has not been established.
73:     pub input: Option<String>,
74:     pub marker: String,
75:     #[serde(serialize_with = "serialize_bits")]
76:     pub values: Vec<f32>,
77: }
78: #[derive(Debug, Clone, Serialize)]
79: pub struct InventoryChunk {
80:     pub parent: EntityKey,
81:     pub uuid: String,
82:     pub shape: ChunkShape,
83:     pub dense: Vec<KnownDenseValue>,
84: }
85: 
86: #[derive(Debug, Clone, Default, Serialize)]
87: pub struct SparseEvidence {
88:     pub materialized: bool,
89:     pub debt: bool,
90:     pub inconsistent: bool,
91:     /// A highlighting reader alone is not sparse data to transfer.
92:     pub reader_declared: bool,
93: }
94: 
95: #[derive(Debug, Clone, Serialize)]
96: pub struct TransferInventory {
97:     pub declared_source_identity: Option<String>,
98:     pub descriptor: StorageDescriptor,
99:     pub source_entity: String,
100:     pub source_config: EntityConfig,
101:     pub models: Vec<EmbeddingModelEntry>,
102:     pub sparse: SparseEvidence,
103:     pub dense_debt: bool,
104:     pub entities: Vec<InventoryEntity>,
105:     pub relations: Vec<InventoryRelation>,
106:     pub chunks: Vec<InventoryChunk>,
107: }
108: #[derive(Debug, Clone, Serialize)]
109: pub struct TransferRecipe {
110:     pub target_descriptor: StorageDescriptor,
111:     pub target_entity: String,
112:     pub target_config: EntityConfig,
113:     pub models: Vec<EmbeddingModelEntry>,
114: }
115: 
116: #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
117: pub enum RefusalCode {
118:     ExpectedIdentityMissing, SourceIdentityMissing, ForeignSourceIdentity,
119:     GenerationUnavailable, DescriptorMismatch, UnsupportedRecipe, InvalidInventory,
120:     SparseUnsupported, DenseDebt, ModelMismatch, ChunkCoverage, InputProvenanceUnavailable,
121:     DenseCoverage, InvalidVector, StaleMarker, ConflictingVectors,
122: }
123: #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
124: pub struct TransferRefusal {
125:     pub code: RefusalCode,
126:     pub location: String,
127:     pub detail: String,
128: }
129: fn refusal(code: RefusalCode, location: impl Into<String>, detail: impl Into<String>) -> TransferRefusal {
130:     TransferRefusal { code, location: location.into(), detail: detail.into() }
131: }
132: 
133: #[derive(Debug, Clone, PartialEq, Serialize)]
134: pub struct PreviewVersion {
135:     pub key: EntityKey,
136:     pub fields: BTreeMap<String, String>,
137:     /// Occurrence-local render state, not a property of the shared version.
138:     pub occurrences: BTreeMap<String, String>,
139: }
140: #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
141: pub struct DenseBinding {
142:     pub model: String,
143:     pub source_storage: VectorStorageNames,
144:     pub target_storage: VectorStorageNames,
145:     pub marker: String,
146:     pub value_bits: Vec<u32>,
147:     /// All covering source UUIDs, not an arbitrary first vector.
148:     pub sources: BTreeSet<String>,
149: }
150: #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
151: pub struct VectorStorageNames {
152:     pub column: String,
153:     pub marker: String,
154:     pub index: String,
155:     pub dim: usize,
156: }
157: fn storage(table: &str, entry: &EmbeddingModelEntry) -> VectorStorageNames {
158:     let v = VectorStorage::resolve(table, entry);
159:     VectorStorageNames { column: v.column, marker: v.marker, index: v.index, dim: v.dim }
160: }
161: #[derive(Debug, Clone, PartialEq, Serialize)]
162: pub struct PreviewChunk {
163:     pub parent: EntityKey,
164:     pub uuid: String,
165:     pub shape: ChunkShape,
166:     pub dense: BTreeMap<String, DenseBinding>,
167: }
168: 
169: /// The common plan currently has no producer binding the complete supplied
170: /// configs/schema facts to its fingerprints. A matching descriptor is only a
171: /// matching declaration, never this missing qualification.
172: #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
173: #[serde(rename_all = "snake_case")]
174: pub enum PlanQualification {
175:     UnqualifiedConfigAndSchemaFacts,
176: }
177: 
178: /// Report only. Private fields, no Deserialize, execute, apply or writer token.
179: #[derive(Debug, Clone, Serialize)]
180: pub struct KnownTransferPreview {
181:     identity: String,
182:     referenced_plan_identity: String,
183:     plan_qualification: PlanQualification,
184:     inventory_fingerprint: String,
185:     retained_entities: Vec<InventoryEntity>,
186:     retained_relations: Vec<InventoryRelation>,
187:     versions: Vec<PreviewVersion>,
188:     chunks: Vec<PreviewChunk>,
189: }
190: impl KnownTransferPreview {
191:     pub fn identity(&self) -> &str { &self.identity }
192:     pub fn referenced_plan_identity(&self) -> &str { &self.referenced_plan_identity }
193:     pub fn plan_qualification(&self) -> PlanQualification { self.plan_qualification }
194:     pub fn inventory_fingerprint(&self) -> &str { &self.inventory_fingerprint }
195:     pub fn retained_entities(&self) -> &[InventoryEntity] { &self.retained_entities }
196:     pub fn retained_relations(&self) -> &[InventoryRelation] { &self.retained_relations }
197:     pub fn versions(&self) -> &[PreviewVersion] { &self.versions }
198:     pub fn chunks(&self) -> &[PreviewChunk] { &self.chunks }
199: }
200: 
201: // Canonicalize object keys as well as collection ordering. This is a report
202: // fingerprint, never a storage generation or a durable execution identity.
203: fn encoded<T: Serialize>(value: &T) -> Result<String, TransferRefusal> {
204:     super::facts::canonical_json(value)
205:         .map_err(|e| refusal(RefusalCode::InvalidInventory, "serialization", e))
206: }
207: 
208: fn valid_values(values: &BTreeMap<String, CypherValue>) -> bool {
209:     fn valid(v: &CypherValue) -> bool {
210:         match v {
211:             CypherValue::Float(f) => f.is_finite(),
212:             CypherValue::List(v) => v.iter().all(valid),
213:             CypherValue::Map(v) => v.values().all(valid),
214:             CypherValue::Blob(_) | CypherValue::Typed { .. } => false,
215:             _ => true,
216:         }
217:     }
218:     values.values().all(valid)
219: }
220: 
221: fn models(entries: &[EmbeddingModelEntry], location: &str) -> Result<BTreeMap<String, EmbeddingModelEntry>, TransferRefusal> {
222:     let mut names = BTreeMap::new();
223:     let mut slugs = BTreeSet::new();
224:     let mut legacy = 0;
225:     for entry in entries {
226:         if entry.name.trim().is_empty() || entry.dim == 0 || !slugs.insert(entry.slug())
227:             || names.insert(entry.name.clone(), entry.clone()).is_some() {
228:             return Err(refusal(RefusalCode::ModelMismatch, location, "missing/duplicate model or slug collision"));
229:         }
230:         if entry.storage == StorageKind::Legacy { legacy += 1; }
231:     }
232:     if legacy > 1 { return Err(refusal(RefusalCode::ModelMismatch, location, "multiple models share legacy storage")); }
233:     Ok(names)
234: }
235: 
236: /// Compute a preview from supplied facts. `expected_source_identity` is an
237: /// explicit comparison input, NOT extracted from the current resolved plan.
238: /// Even matching declared values grant no proof of provenance or write access.
239: pub fn preview_known_transfer(
240:     fixed_plan: &ResolvedMigrationPlan,
241:     expected_source_identity: Option<&str>,
242:     inventory: &TransferInventory,
243:     recipe: &TransferRecipe,
244: ) -> Result<KnownTransferPreview, Vec<TransferRefusal>> {
245:     preview(fixed_plan, expected_source_identity, inventory, recipe).map_err(|e| vec![e])
246: }
247: fn preview(
248:     plan: &ResolvedMigrationPlan, expected: Option<&str>, input: &TransferInventory, recipe: &TransferRecipe,
249: ) -> Result<KnownTransferPreview, TransferRefusal> {
250:     use RefusalCode::*;
251:     let expected = expected.filter(|s| !s.trim().is_empty())
252:         .ok_or_else(|| refusal(ExpectedIdentityMissing, "origin", "no expected source instance identity supplied"))?;
253:     let declared = input.declared_source_identity.as_deref().filter(|s| !s.trim().is_empty())
254:         .ok_or_else(|| refusal(SourceIdentityMissing, "origin", "inventory declares no source instance identity"))?;
255:     if declared != expected { return Err(refusal(ForeignSourceIdentity, "origin", "declared identity differs from explicit expectation")); }
256:     plan.validate_generation_preconditions().map_err(|e| refusal(GenerationUnavailable, "plan", e))?;
257:     if input.descriptor != plan.inputs().source || recipe.target_descriptor != plan.inputs().target {
258:         return Err(refusal(DescriptorMismatch, "descriptors", "supplied descriptors differ from fixed inputs"));
259:     }
260:     let cfg = &input.source_config;
261:     let target = &recipe.target_config;
262:     if cfg.signals.sparse() || target.signals.sparse() || input.sparse.materialized || input.sparse.debt || input.sparse.inconsistent {
263:         return Err(refusal(SparseUnsupported, "sparse", "sparse/dual data, debt or inconsistent evidence cannot be transported"));
264:     }
265:     if input.dense_debt { return Err(refusal(DenseDebt, "dense", "source declares unavailable dense values")); }
266:     cfg.validate().and_then(|_| target.validate()).map_err(|e| refusal(UnsupportedRecipe, "config", e))?;
267:     if !super::facts::entity_config_is_finite(cfg) || !super::facts::entity_config_is_finite(target) {
268:         return Err(refusal(UnsupportedRecipe, "config", "non-finite config cannot be fingerprinted faithfully"));
269:     }
270:     let derived = target.derived.as_ref().ok_or_else(|| refusal(UnsupportedRecipe, "target", "content derivation required"))?;
271:     let DerivedIdentity::Content { format } = &derived.identity else {
272:         return Err(refusal(UnsupportedRecipe, "target", "content identity required"));
273:     };
274:     // Inline rendering alone: render_fields otherwise can read user templates.
275:     if derived.from != input.source_entity || !derived.gather.is_empty()
276:         || derived.render.values().any(|s| !s.contains("{{") && !s.contains("{%"))
277:         || input.source_entity.trim().is_empty() || recipe.target_entity.trim().is_empty()
278:         || input.source_entity == recipe.target_entity || cfg.derived.is_some()
279:         || cfg.chunked == Some(false) || target.chunked == Some(false)
280:         || cfg.search_index.is_some() || target.search_index.is_some()
281:         || cfg.signals != target.signals || !cfg.signals.bm25()
282:         || cfg.fields.values().filter(|f| f.is_title || f.title_for.as_deref() == Some("self")).count() > 1
283:         || target.fields.keys().collect::<BTreeSet<_>>() != derived.render.keys().collect::<BTreeSet<_>>()
284:         || format != &recipe.target_descriptor.application_format
285:         || cfg.content_fields() != target.content_fields() || cfg.title_field() != target.title_field()
286:         || encoded(&cfg.chunking)? != encoded(&target.chunking)?
287:         || target.fields.iter().any(|(name,f)| cfg.fields.get(name).map(|s| encoded(s).ok()) != Some(encoded(f).ok())) {
288:         return Err(refusal(UnsupportedRecipe, "config", "only unchanged field roles, inline renders and chunk recipe are supported"));
289:     }
290:     let source_models = models(&input.models, "source models")?;
291:     let target_models = models(&recipe.models, "target models")?;
292:     if source_models.len() != target_models.len()
293:         || source_models.iter().any(|(n,s)| target_models.get(n).is_none_or(|t| t.dim != s.dim))
294:         || (cfg.signals.vector() && source_models.is_empty()) || (!cfg.signals.vector() && !source_models.is_empty()) {
295:         return Err(refusal(ModelMismatch, "models", "complete identical model names/dimensions required"));
296:     }
297:     let mut inventory = input.clone();
298:     inventory.entities.sort_by_key(|e| e.key.key());
299:     inventory.chunks.sort_by_key(|c| (c.parent.key(), c.uuid.clone()));
300:     inventory.models.sort_by_key(|m| m.name.clone());
301:     for chunk in &mut inventory.chunks { chunk.dense.sort_by_key(|v| v.model.clone()); }
302:     // Serialized properties are validated before they are used for sorting/hashing.
303:     if inventory.entities.iter().any(|e| !valid_values(&e.values))
304:         || inventory.relations.iter().any(|e| !valid_values(&e.properties)) {
305:         return Err(refusal(InvalidInventory, "values", "non-finite or unsupported values cannot be fingerprinted faithfully"));
306:     }
307:     let mut relations = inventory.relations.iter().map(|r| encoded(r).map(|s| (s,r.clone())))
308:         .collect::<Result<Vec<_>,_>>()?;
309:     relations.sort_by(|a,b| a.0.cmp(&b.0));
310:     inventory.relations = relations.into_iter().map(|(_,r)| r).collect();
311:     let mut nodes = BTreeSet::new();
312:     for entity in &inventory.entities {
313:         if entity.key.entity.trim().is_empty() || entity.key.uuid.trim().is_empty()
314:             || entity.key.scope.validate().is_err() || !nodes.insert(entity.key.key()) {
315:             return Err(refusal(InvalidInventory, entity.key.uuid.clone(), "invalid or duplicate entity key"));
316:         }
317:         for (name, value) in [("_uuid", &entity.key.uuid), ("_org", &entity.key.scope.org), ("_project", &entity.key.scope.project)] {
318:             if entity.values.get(name).is_some_and(|v| v.as_str() != Some(value)) {
319:                 return Err(refusal(InvalidInventory, entity.key.uuid.clone(), "system columns contradict the entity key"));
320:             }
321:         }
322:         if entity.key.entity == recipe.target_entity {
323:             return Err(refusal(InvalidInventory, entity.key.uuid.clone(), "target entity already present"));
324:         }
325:     }
326:     for relation in &inventory.relations {
327:         if relation.name.trim().is_empty() || !nodes.contains(&relation.from.key()) || !nodes.contains(&relation.to.key()) {
328:             return Err(refusal(InvalidInventory, &relation.name, "relation endpoint missing from retained graph"));
329:         }
330:     }
331:     let mut observed = BTreeMap::new();
332:     for chunk in &inventory.chunks {
333:         if !nodes.contains(&chunk.parent.key()) || chunk.parent.entity != inventory.source_entity
334:             || chunk.uuid != chunk_uuid(&chunk.parent.uuid, &chunk.shape.field, chunk.shape.chunk.index)
335:             || observed.insert((chunk.parent.key(), chunk.uuid.clone()), chunk).is_some() {
336:             return Err(refusal(ChunkCoverage, &chunk.uuid, "orphan, duplicate or incorrectly identified source chunk"));
337:         }
338:     }
339:     let chunker = Chunker::new(ChunkerConfig::from(&target.chunking));
340:     let mut versions: BTreeMap<_, PreviewVersion> = BTreeMap::new();
341:     let mut chunks: BTreeMap<_, PreviewChunk> = BTreeMap::new();
342:     let mut consumed = BTreeSet::new();
343:     let mut occurrence_count = 0;
344:     for occurrence in inventory.entities.iter().filter(|e| e.key.entity == inventory.source_entity) {
345:         occurrence_count += 1;
346:         let context = render_context(&occurrence.values, &BTreeMap::new());
347:         let rendered = render_fields(derived, &context).map_err(|e| refusal(UnsupportedRecipe, &occurrence.key.uuid, e))?;
348:         if rendered.iter().any(|(name,text)| occurrence.values.get(name).and_then(|v| v.as_str()) != Some(text.as_str())) {
349:             return Err(refusal(UnsupportedRecipe, &occurrence.key.uuid, "rendered field is not byte-identical to its source field"));
350:         }
351:         let uuid = derived_content_uuid(&recipe.target_entity, format, target, &rendered, &occurrence.key.scope)
352:             .map_err(|e| refusal(UnsupportedRecipe, &occurrence.key.uuid, e))?;
353:         let key = EntityKey { entity: recipe.target_entity.clone(), scope: occurrence.key.scope.clone(), uuid };
354:         let version = versions.entry(key.key()).or_insert_with(|| PreviewVersion {
355:             key: key.clone(), fields: rendered.clone(), occurrences: BTreeMap::new(),
356:         });
357:         if version.fields != rendered { return Err(refusal(InvalidInventory, &key.uuid, "content identity collision")); }
358:         version.occurrences.insert(occurrence.key.uuid.clone(), render_inputs_hash(&context));
359:         let title = target.title_field().and_then(|f| rendered.get(f)).cloned().unwrap_or_default();
360:         let mut offset = 0;
361:         for (i, field) in target.content_fields().into_iter().enumerate() {
362:             if i > 0 { offset += 2; }
363:             let text = rendered.get(field).map(String::as_str).unwrap_or("");
364:             for chunk in chunker.chunk(text) {
365:                 let shape = ChunkShape { field: field.into(), title: title.clone(), content_offset: offset, chunk };
366:                 let source_uuid = chunk_uuid(&occurrence.key.uuid, field, shape.chunk.index);
367:                 let source_key = (occurrence.key.key(), source_uuid.clone());
368:                 let source = observed.get(&source_key).ok_or_else(|| refusal(ChunkCoverage, &source_uuid, "required source chunk missing"))?;
369:                 if source.shape != shape { return Err(refusal(ChunkCoverage, &source_uuid, "text, title or full/core coordinates differ")); }
370:                 consumed.insert(source_key);
371:                 let uuid = chunk_uuid(&key.uuid, field, shape.chunk.index);
372:                 let destination = chunks.entry((key.key(), uuid.clone())).or_insert_with(|| PreviewChunk {
373:                     parent: key.clone(), uuid, shape: shape.clone(), dense: BTreeMap::new(),
374:                 });
375:                 if destination.shape != shape { return Err(refusal(ChunkCoverage, &source_uuid, "destination shape collision")); }
376:                 let mut seen_models = BTreeSet::new();
377:                 for known in &source.dense {
378:                     let location = format!("{}/{}/{}/{}/{}", occurrence.key.scope.org,
379:                         occurrence.key.scope.project, occurrence.key.entity, source_uuid, known.model);
380:                     let entry = source_models.get(&known.model).ok_or_else(|| refusal(ModelMismatch, &location, "unknown vector model"))?;
381:                     if !seen_models.insert(known.model.clone()) { return Err(refusal(DenseCoverage, &location, "duplicate model value")); }
382:                     if known.input.as_deref() != Some(shape.chunk.text.as_str()) {
383:                         return Err(refusal(InputProvenanceUnavailable, &location, "known `_text` model input required"));
384:                     }
385:                     if known.marker != content_hash(&shape.chunk.text) { return Err(refusal(StaleMarker, &location, "model marker does not match chunk text")); }
386:                     if known.values.len() != entry.dim || known.values.iter().any(|v| !v.is_finite()) {
387:                         return Err(refusal(InvalidVector, &location, "dimension mismatch or non-finite values"));
388:                     }
389:                     let bits: Vec<_> = known.values.iter().map(|v| v.to_bits()).collect();
390:                     if let Some(binding) = destination.dense.get_mut(&known.model) {
391:                         if binding.value_bits != bits { return Err(refusal(ConflictingVectors, &location, format!("conflicting values for {}", known.model))); }
392:                         binding.sources.insert(source_uuid.clone());
393:                     } else {
394:                         destination.dense.insert(known.model.clone(), DenseBinding {
395:                             model: known.model.clone(), marker: known.marker.clone(), value_bits: bits,
396:                             source_storage: storage(&format!("{}_Chunk", inventory.source_entity), entry),
397:                             target_storage: storage(&format!("{}_Chunk", recipe.target_entity), &target_models[&known.model]),
398:                             sources: BTreeSet::from([source_uuid.clone()]),
399:                         });
400:                     }
401:                 }
402:                 if seen_models.len() != source_models.len() { return Err(refusal(DenseCoverage, &source_uuid, "not all declared models cover this source chunk")); }
403:             }
404:             offset += text.len();
405:         }
406:     }
407:     if occurrence_count == 0 || chunks.is_empty() || consumed.len() != observed.len() {
408:         return Err(refusal(ChunkCoverage, "inventory", "empty transfer or extra/uncovered source chunks"));
409:     }
410:     let inventory_fingerprint = content_hash(&encoded(&inventory)?);
411:     let versions: Vec<_> = versions.into_values().collect();
412:     let chunks: Vec<_> = chunks.into_values().collect();
413:     let mut normalized_recipe = recipe.clone();
414:     normalized_recipe.models.sort_by_key(|m| m.name.clone());
415:     let identity = content_hash(&encoded(&("known-transfer-preview-v2", plan.identity(), expected,
416:         &inventory_fingerprint, &normalized_recipe, &versions, &chunks))?);
417:     Ok(KnownTransferPreview { identity, referenced_plan_identity: plan.identity().into(),
418:         plan_qualification: PlanQualification::UnqualifiedConfigAndSchemaFacts, inventory_fingerprint,
419:         retained_entities: inventory.entities, retained_relations: inventory.relations, versions, chunks })
420: }
421: 
422: #[cfg(test)]
423: mod tests;

CONTRAT composite 777820ef docs/arbre-principal/12-11-2026/02-preparer-le-transfert-des-valeurs-connues.md
# Transfert générique de valeurs connues — tranche à relire

**État courant, relu sur master41497ba4e :** aperçu intégré sur3b2eab7ec,
passe verte et ressources retirées. La préparation initiale ci-dessous est
historique ; la section « Prochain pas » en fin de cette même page fixe la suite.
Ce chantier avance la migration1b ; les couches2a, leur résolution et leurs
écritures ne sont pas livrées par l'aperçu. Pas de nouvelle vision parallèle.

Préparation sur master **6e5f9458c**, 12 novembre 2026. Porteur : arbre-principal ;
revue commune : orchestration. [Ticket canonique](../tickets/2026-11-12-migrer-les-ancres-de-code-vers-le-contenu-partage.md),
[répartition parallèle](../../tickets/closed/2026-11-12-paralleliser-les-suites-autorisees-sans-chevauchement.md).
Aucun Rust modifié, Cargo lancé, transfert exécuté, target, base ou service créé.
Les signatures ci-dessous sont proposées, pas disponibles dans le produit.

## Résultat borné et dépendances

La première tranche de code proposée construit en mémoire un **aperçu déterministe
et non exécutable** du transfert : occurrences→versions→chunks et correspondances
de valeurs connues par cellule et modèle. Elle vérifie la couverture complète,
sans ouvrir de base, écrire de ligne ou disposer d'un embedder. Elle ne prétend
pas livrer T3–T6 sur stockage ni T7–T12 de publication/reprise.

Le préflight livré et le socle1a restent les frontières : ResolvedMigrationPlan
fixe source/cible, variables et faits ; DerivedIdentity::Content définit la version ;
MigrationRunner est l'unique orchestrateur futur. Pas de second runner, journal,
mode d'ouverture permissif ou chemin d'ingestion qui recalcule un manque.

**Deux prérequis du transfert effectif restent absents** : identité durable de
l'instance de stockage source et génération établie de ses données. Le descripteur
actuel n'a pas d'identifiant d'instance ; l'inspection peut rendre Unversioned.
Ni chemin, hash de configs, empreinte d'inventaire, checkpoint ou UUID des lignes
ne remplace ces deux preuves. Les fixtures peuvent déclarer explicitement une
identité/génération synthétique ; cela ne crée aucun producteur de métadonnées
pour une ancienne base. Tout passage vers une future écriture refusera leur absence
avant effets. Leur établissement sous exclusivité réelle et journal durable
appartient à une tranche distincte, avant tout transfert ou apply.

## Primitives relues et décisions d'API

| Besoin | Primitive existante et décision |
|---|---|
| Rendu et identité de version | derive_nodes::{render_context, render_fields, render_inputs_hash, derived_content_uuid}, EntityConfig ; réutiliser les fonctions, aucune formule parallèle de Scope. |
| Découpe et identité de chunk | Chunker::chunk, ChunkerConfig et uuid::chunk_uuid ; les conventions de record_nodes::ChunkNode portent champs, offsets et liens. Si une extraction commune est nécessaire, proposer son diff avant de modifier record_nodes.rs partagé. |
| Modèles et stockage | EmbeddingModelEntry, StorageKind, VectorStorage::resolve : nom complet+dimension qualifient le modèle ; le slug ne suffit pas (collisions possibles). Résoudre séparément source et cible, legacy ou suffixed. |
| Réutilisation actuelle | catalog/aside::KnownVectors : un slug et map par ancien UUID. Ne pas élargir silencieusement sa sémantique ni injecter dans EmbedNode pour obtenir un prétendu transfert sans calcul. |
| Porteur de valeurs | records::{EntityRecord, RecordVectors, DenseVector} : une seule valeur dense par RecordVectors actuellement. L'aperçu porte une liaison par modèle ; ne pas lui faire croire qu'un EntityRecord contient tous les modèles. Adaptateur multi-modèle d'écriture à fournir ultérieurement. |
| Recherche/occurrences | Socle1a et catalog::resolve_to_source_entities ; conserver les liens canoniques et la projection avant réduction des résultats. Ne pas recopier cette politique dans le transport. |

Surface proposée dans `src/dataflow/migrations/transfer.rs`, enfant du runner
existant, avec accès `pub(crate)` initial :

```rust
// Esquisse : noms et signatures à arrêter en revue, aucun code livré ici.
fn preview_known_transfer(
    fixed_plan: &ResolvedMigrationPlan,
    inventory: &TransferInventory,
    target: &TransferRecipe,
) -> Result<KnownTransferPreview, Vec<TransferRefusal>>;
```

TransferInventory décrit des faits fournis, sans prétendre les avoir lus d'une
base. TransferRecipe compose les EntityConfig/faits résolus réels ; les fonctions
communes de rendu/UUID/découpe produisent les destinations. KnownTransferPreview
a des champs privés, accesseurs de rapport et correspondances déterministes ;
**aucune méthode execute/apply et aucune conversion en capacité d'écriture**.
Il référence l'identité du plan fixe et les empreintes des inventaires, sans
ajouter d'identité exécutable au journal. Les refus sont typés et localisés
(cellule, entité/chunk, modèle, motif), triés pour un rapport reproductible.

Le coût de cette primitive est la conservation des métadonnées de couverture et
la comparaison des vecteurs candidats. Elle unifie les changements de parent
indexé pour toute entité déclarée ; aucune branche spéciale Scope. La fixture
reste petite et entièrement en mémoire. Streaming, limites de volume des bases
réelles et lecteur RO cohérent sont des prérequis de la tranche de transport,
pas des promesses de cet aperçu.

## Inventaire requis, sans inventer de données

- Origine : identité d'instance et génération explicitement présentes ou absentes,
  descripteurs source/cible, empreinte des configs/faits/recette, identité du plan.
  L'empreinte de l'inventaire couvre ses entrées, pas une génération durable.
- Par cellule : occurrences avec UUID, valeurs rendues, provenance et propriétés
  d'occurrence (fichier, positions), états des liens, masques/visibilité déclarés.
  Les coordonnées d'occurrence restent sur l'occurrence ; les offsets de chunks
  sont relatifs aux champs rendus de leur version. Même contenu dans deux cellules
  ne donne ni liens ni magasin de vecteurs partagés entre elles.
- Relations : inventaire canonique des extrémités, types, propriétés et
  multiplicités, avec sens ; conserver les deux représentations attendues quand
  le moteur les matérialise. Arêtes du code inchangées ; seuls liens déclarés
  version→occurrence et chunk→version sont ajoutés à l'aperçu cible.
- Chunks : parent/champ/rôle/indice, texte et titre exacts, offsets complets/core,
  lignes et content_offset. Vérifier bornes UTF-8, cohérence avec les champs,
  couverture et unicité ; pas seulement l'égalité des comptes ou des hashes.
- Dense : registre complet des modèles, nom/dimension/stockage source/cible,
  colonnes et marqueurs résolus, valeurs brutes par chunk et par modèle,
  disponibilité/dettes. Dimension exacte, valeurs finies ; aucun vecteur absent,
  partiel, périmé ou contradictoire n'est remplacé par un calcul.
- Entrée du modèle : octets effectivement désignés par le chemin d'embarquement
  qualifié, pas une chaîne reconstruite au jugé. EmbedNode utilise text_field
  (`_text` par défaut), pas automatiquement `_title`. Le titre reste à comparer
  comme donnée/recette ; si un chemin a incorporé le titre ou une autre entrée,
  prouver cette recette et cette entrée. Marqueur `_text_hash` seul ne prouve pas
  une autre entrée historique. Source ambiguë : refus InputProvenanceUnavailable.
- Sparse : croiser signaux persistés, colonnes/marqueurs, handles/magasins et
  dette. Tout sparse/dual à transférer ou contradiction est refusé avant effets.
  Une déclaration models.sparse servant seulement au surlignage ne suffit pas à
  classer le profil sparse ; ne pas faire un refus sur ce seul champ.

## Correspondances et invariants

1. Regrouper les occurrences par **identité1a calculée**, dans chaque cellule,
   sur tous les rendus/rôles et la recette ; ne pas dédupliquer au seul corps.
   Garder huit UUID d'occurrence dans la fixture canonique, six versions attendues.
2. Rejouer uniquement le rendu et la découpe déterministes. Construire les nouveaux
   UUID de chunks avec chunk_uuid(nouveau_parent, champ, indice). Une liaison
   source→destination exige mêmes octets d'entrée modèle, recette, champ/rôle,
   offsets/indice, modèle complet et dimension ; pas une copie de map UUID.
3. Plusieurs sources compatibles peuvent couvrir une destination, mais leurs
   valeurs pour un modèle doivent être identiques **bit à bit** (`f32::to_bits`),
   sans tolérance numérique ni « premier trouvé ». Même slug avec noms différents,
   doublon divergent, marqueur périmé ou absence de couverture : refus nommé.
   Le représentant de rapport est choisi par ordre canonique, tous les témoins
   de couverture sont conservés. L'ordre d'entrée n'influence pas le résultat.
4. Garder une liaison distincte par modèle. Les dimensions4 et6 ne sont jamais
   mélangées ; les valeurs et marqueurs ne deviennent disponibles qu'après future
   écriture et validation réelles, pas parce que l'aperçu les énumère.
5. Texte BM25 exact préservé. Les offsets physiques et structures FTS/HNSW ne se
   copient pas ; leur reconstruction future doit utiliser texte/vecteurs connus.
   La déduplication change le corpus BM25 : scores/rangs globaux identiques ne sont
   pas un invariant ; couverture, filtres, provenance, positions et égalités le sont.
6. Aucun état persistant ni index déclaré prêt ne sort de cette tranche. Les
   lectures masquées doivent rester une politique commune ; l'inventaire/aperçu
   conserve les occurrences et masques nécessaires sans prétendre exercer T7.

## Témoins proposés avant correction et budget

Fixture canonique du plan1b : Alpha A/B identiques, C corps différent, D docstring
différente, E nom différent, F sans docstring ; Beta deux rendus identiques à A.
Huit occurrences/six versions, graphes de code avec propriétés et multiplicités,
deux modèles synthétiques4/6 ; vecteurs littéraux, aucun serveur ni modèle.
Un deuxième nom d'entité déclaré autre que Scope vérifie la généricité.

| Cas | Verdict à exiger |
|---|---|
| Nominal, permutations, deux cellules | Mêmes aperçus/liaisons/empreintes, huit occurrences/six versions, aucun lien cross-cellule, octets/positions/provenance préservés. |
| Texte, titre, rôle, recette ou entrée modèle distincte | Classes distinctes ou refus de compatibilité, jamais réutilisation par seul hash de corps. |
| Deux modèles, legacy/suffixed, collision de slug | Résolution source/cible correcte, couverture indépendante ; collision de noms refusée. |
| Vecteur absent/dimension fausse/NaN/infini/marqueur périmé | Refus localisé, aucun succès partiel ni disponibilité inventée. |
| Deux sources d'une destination, valeurs divergentes | Refus bit à bit ; égalité exacte acceptée, ordre d'entrée sans effet. |
| Identité source ou génération absente/étrangère | Refus préalable à toute future opération ; Unversioned reste explicite. |
| Sparse/dual/dette/contradiction ; lecteur de surlignage seul | Refus du transfert sparse ; le lecteur seul n'est pas un faux positif. |
| Masques, relations et couverture manquante | Conservation exacte de l'inventaire valide ; incohérence nommée, pas de remplacement par un compte global. |

Proposition de premier diff : module pur `migrations/transfer.rs`, déclaration
`mod transfer` dans migrations.rs et tests unitaires voisins. Aucun backend.rs,
CLI, gabarit, stockage, runner supplémentaire ou intégration de service. Les
factories d'aperçu ne reçoivent ni DbConnection ni Embedder ; ce fait limite
structurellement les effets, mais ne remplace pas les futurs témoins d'exécution.

Si les conventions de ChunkNode ne sont pas réutilisables sans extraction,
signaler d'abord le petit contrat commun à orchestration ; ne pas réserver ni
modifier record_nodes.rs en parallèle d'ED sans accord sur le propriétaire.
Préparation actuelle : zéro lourd. Après revue de périmètre, proposition d'un
seul ciblé sur luciepc, target unique par session plafonné32Go, j8/Mem32, borne600s
après poste lourd ; jeu minimal à choisir d'après les dépendances effectives.
Premier rouge : diagnostic et arrêt, aucun rejeu inchangé. Passe complète et
budget d'intégration à convenir après le diff et les compteurs réellement lus.

La tranche suivante devra fournir inventaire RO lié au stockage sous exclusivité,
identité/génération durables, journal running réel, adaptateur multi-modèle,
écritures/reconstruction/checkpoints/validation et publication atomique avant
apply. L'aperçu pur ne permet de sauter aucun de ces prérequis.

## Diff pur autorisé et nuance d'identité — 12 novembre

C7 autorise module/déclaration/tests, puis ciblé600s/j8/Mem32 sous lourd après
publication de la source et argv. L'API prend explicitement expected_source_identity ;
absence d'attendu et absence de déclaré sont deux refus distincts. Le plan actuel
ne produit pas cet attendu ; égalité de deux déclarations n'est jamais une preuve
issue du stockage. KnownTransferPreview porte une empreinte de rapport distincte
de la génération et de l'identité du plan, sans exécution ni désérialisation.

Module public de rapport pour éviter une API interne inutilisée en production,
avec types d'entrée explicites et sortie à champs privés ; aucun point CLI ajouté.
Rendus inline seulement : render_fields peut sinon relire un gabarit utilisateur,
ce qui casserait la pureté promise. Chunker/UUID/rendus communs réutilisés ; aucun
changement de record_nodes, catalog, backend ou RecordVectors multi-modèle.

## Prochain pas — lier les faits déclarés au plan sans qualifier le stockage

Préparation demandée par orchestration après livraison de l'aperçu. Source de
lecture41497ba4e ; worktree neuf cx-wt-qualification-faits, aucune modification
Rust, aucun Cargo/GPU, accès base ou nouveau service. Même ticket migration.

### Le défaut précis et ce qui est déjà décidé

ResolvedMigrationPlan reçoit aujourd'hui config_fingerprint et
schema_facts_fingerprint comme des chaînes. Il fixe ces entrées, mais ne produit
pas leurs valeurs depuis les configurations examinées par l'aperçu. Modifier
une config tout en conservant les étiquettes reste donc possible ; le rapport
intégré annonce honnêtement UnqualifiedConfigAndSchemaFacts. Le prochain pas
corrige cette liaison **entre faits fournis et plan**, sans prétendre observer
l'identité ou la génération d'une base.

Restent acquis : contenu distinct d'occurrence ; génération distincte d'un hash
de schéma ; identité logique distincte d'un chemin de copie ; inputs du plan
figés, comparaison source avant opération et cible attendue après publication ;
rejeu applied identique seulement ; exclusivité réelle, running durable avant
effets et publication atomique ; sparse refusé hors transfert prouvé. Ces règles
ne sont pas des questions à reposer à Lucie. Rien ne change dans les masques,
positions, relations, modèles, découpe, rétention ou admission déjà convenus.

### Faits existants à composer, pas à reconstruire à côté

| Existant relu | Usage et limite du prochain pas |
|---|---|
| backend_schema::SchemaFacts::{compose,registration_order} | Faits complets manifeste+fournisseur et validation des dépendances. Réutiliser cette composition/validation ; pas de second fournisseur ni de copie des règles. |
| PreparedBackend::migration_plan | Produit target_entities/target_relations après surcharges index_signals ; prendre ces faits effectifs, pas uniquement le nom du fournisseur ou les champs avant surcharge. |
| StorageInspection::entity_configs / entity_config_fingerprint | Entités persistées seulement ; ni relations, ni données, ni génération. Le périmètre de cette empreinte existante reste inchangé. |
| Catalog::{persist_relation,load_relations} | Les RelationDef existent déjà sous les clés metadata relation:<nom>. Leur format est disponible : ne pas inventer un magasin de relations. Le lecteur d'inspection actuel ne les restitue pas ; ne pas traiter cette absence comme un graphe vide observé. |
| plan::{MigrationPlanInputs,ResolvedMigrationPlan}, MigrationRunner | Un constructeur commun des entrées et un vérificateur utilisant le même producteur ; conserver le runner et l'identité figée, pas un deuxième journal/exécuteur. |
| transfer::encoded, hash::content_hash, EntityConfig/RelationDef | Mécanisme de canonicalisation/hachage déjà présent ; définir une seule projection sémantique de faits pour la construction et la comparaison. Ne pas rebaptiser l'empreinte privée de rapport « contrat commun ». |

### Tranche A proposée : constructeur et vérificateur purs des faits

Entrée : ensembles **complets explicitement fournis** source et cible, chacun
avec toutes les EntityConfig et RelationDef, faits effectifs du fournisseur,
recette déclarée et définitions effectivement résolues nécessaires à cette
recette, registres des modèles/dimensions/stockages ; variables et étapes déjà
résolues du runner. Les données d'occurrences/vecteurs restent l'inventaire de
l'aperçu, pas une génération déduite de ces faits.

Un ensemble d'entités sans les relations requises, un fournisseur nommé sans ses
faits ou une recette référencée sans ses octets résolus est **incomplet**.
L'appelant doit distinguer « ensemble déclaré complet et réellement vide » de
« information absente ». Aucun remplissage par les defaults du manifeste courant
pour réparer une observation source manquante. Aucun accès fichier ou résolution
de template externe depuis la fonction pure ; les octets résolus sont fournis,
ou le contrôle refuse UnresolvedRecipe.

Surface indicative, noms et emplacement sont des choix d'implémentation :

```rust
// Proposition pour un prochain diff, pas API déjà disponible.
let facts = DeclaredMigrationFacts::from_complete_inputs(source, target)?;
let inputs = facts.bind_inputs(formats, generations, variables)?;
let plan = existing_runner.resolve_plan(inputs)?;
let check = facts.compare_with_fixed_plan(&plan)?;
```

Le constructeur et la comparaison partagent **le même producteur**. Ce qui entre
respectivement dans l'empreinte des configs d'entités, celle des faits complets
(entités/relations/fournisseur/recettes effectives) et l'identité globale est
nommé et versionné explicitement. L'empreinte entités seule ne devient pas
silencieusement une empreinte de graphe. Types/définitions normalisés par les
contrats existants, défauts effectifs inclus, clés de maps ordonnées ; l'ordre
des listes sémantiques est préservé. Valeurs non finies ou non représentables
refusées avant sérialisation, sans conversion en null.

Un ancien champ opaque dont le producteur/version/périmètre n'est pas connu ne
se requalifie pas par supposition : refus FactsContractUnknown ou état non
qualifié explicite. Le nouveau chemin construit ses empreintes depuis les faits,
pas depuis une étiquette passée à la main. Il ne réécrit aucun descripteur
stocké ni l'identité d'un plan déjà figé ; la voie préliminaire actuelle et
l'ouverture legacy restent disponibles avec leurs limites présentes.

Le résultat du contrôle veut dire **faits déclarés concordants avec les entrées
du plan**, jamais « faits lus d'une base », « plan appliqué » ou « transfert
exécutable ». Pas de conversion vers PlanJournalState::Applied ni vers une
capacité d'écriture. Première tranche : introduire et éprouver ce producteur et
sa comparaison ; ne pas ajouter prématurément un état de stockage qualifié à
KnownTransferPreview. Son statut intégré reste inchangé tant que les prérequis
d'observation/liaison ne sont pas réalisés et revus ensemble.

### Source et génération : preuves séparées, absence visible

La liaison de configs est possible même si les générations source/cible sont
Unversioned. Elle ne doit pas appeler validate_generation_preconditions pour
forcer une génération inventée : ce contrôle d'exécution reste distinct et
continue de refuser. De même, la comparaison d'une identité déclarée exige
l'attendu explicite déjà demandé par l'API ; l'égalité de deux déclarations
n'établit pas leur provenance. Absence d'attendu, origine inconnue, génération
absente ou étrangère gardent des motifs séparés.

Fixtures du prochain diff : attendu et origine synthétiques fournis explicitement,
ainsi que cas absents/étrangers ; jamais d'UUID tiré du chemin, d'un hash de configs
ou du rapport. Le contrôle ne crée aucun identifiant de stockage. Une empreinte
d'inventaire ne devient ni génération durable ni preuve que des écritures ont
cessé. Un plan figé reste le même lorsque sa cible attendue est examinée après
une publication simulée ; un avancement étranger est refusé.

### Frontière de fichiers et suite qui n'entre pas dans A

Premier diff proposé : migrations/facts.rs (contrat/producteur purs et tests),
plan.rs et point de construction dans MigrationRunner. Petit adaptateur
backend_schema.rs à convenir avec sa propriétaire mémoire et orchestration,
s'il est nécessaire pour exporter ses faits validés sans dupliquer compose.
La session choisit la forme du code et des types ; l'orchestration coordonne
la propriété de ce fichier partagé avant modification. Aucun fichier partagé
n'est encore réservé ou modifié par cette préparation.

Ne pas élargir A à backend.rs, au CLI, à Catalog::initialize, à un changement
d'ouverture legacy, à la lecture d'une base ou à une nouvelle clé metadata.
Le raccord ultérieur de l'inspection RO pourra restituer relation:<nom> dans le
même snapshot que les configs/journal, avec décodage commun et refus des lignes
ambiguës ; ses tests se feront sur données synthétiques. Ce raccord et le bootstrap
persistant ne sont pas supposés livrés par le producteur pur.

### Témoins et critère de fin de A

| Cas à écrire | Preuve attendue |
|---|---|
| Faits complets identiques, maps permutées | Même empreinte et même identité du plan, validation des dépendances existante réutilisée. |
| Config/rôle/découpe/signal/modèle ou recette modifiés, anciennes étiquettes conservées | Refus nommé de liaison ; pas un simple rapport différent annoncé cohérent au plan. |
| Relation inversée, propriété changée, relation manquante | Détection par l'empreinte complète ; empreinte entités seule incapable de qualifier ce cas. |
| Fournisseur code réel et fournisseur d'une autre entité | Faits effectifs réellement fournis, pas de raccourci Scope ou fournisseur=None de fixture. |
| Source limitée aux entités, recette non résolue, version d'empreinte inconnue | Incomplet/non qualifié ou refus explicite, jamais graphe vide/defaults réputés observés. |
| Source/génération absentes mais configs complètes | Liaison déclarée vérifiable ; readiness d'exécution toujours refusée, aucune création implicite. |
| Identité attendue absente/étrangère, génération étrangère | Motifs distincts ; aucune identité ou génération déduite des empreintes. |
| Cible attendue après publication simulée, puis avancement étranger | Plan original immuable, même identité pour le passage attendu ; refus de l'avancement étranger. |
| Valeurs non finies, listes dont l'ordre est sémantique | Refus avant perte de valeur ; ordre conservé et changement détecté. |

Fin de tranche : constructeur réellement consommé par le chemin de résolution
concerné du runner, comparaison utilisant exactement le même producteur, tests
synthétiques ci-dessus et diff relu. Pas seulement une utilité hash sans appelant.
L'aperçu intégré n'est pas rebaptisé « migration livrée ». Après revue de code,
proposition d'un ciblé600s/j8/Mem32 sur luciepc, target de session32Go/lib épinglée,
poste lourd et mesure prioritaire ; aucune réservation ni commande de test faite
pour la présente préparation. La passe d'intégration se choisit après le diff.

### Décisions autonomes et génération composite retenue par Lucie

Sont autonomes, car déjà couverts par les contrats : découpage A puis raccord RO,
noms/types, canonicalisation et version du contrat de faits en mémoire, extraction
d'une primitive commune, placement des tests et refus d'entrées incomplètes. Pas
de nouvel outil visible ni de changement de politique de stockage dans A.
La répartition d'un fichier partagé et la recette de passe sont du ressort de
l'orchestration. **A n'attend pas une décision de conception nouvelle.**

1. **Question tranchée le 12 novembre : génération de publication et empreinte
   d'état ont des rôles distincts.** Après l'explication A→B→A puis la proposition
   composite, Lucie valide la dernière recommandation : « oki pour ta
   recommandation alors ». La génération désigne une identité durable de
   publication ; l'empreinte de l'état logique permet le partage et la
   réutilisation du travail après requalification. Ce n'est pas le choix d'une
   génération définie uniquement par le contenu.

| État logique | Identité de publication | Empreinte d'état |
|---|---|---|
| A initial | P1 | H-A |
| B après nouvelle publication | P2 | H-B |
| Retour au même A | P3 | H-A |

Une publication nouvelle a une identité distincte, même si son état retrouve une
empreinte antérieure. Le rejeu de la **même** opération déjà applied conserve son
identité ; il ne crée ni nouvelle publication ni droit de répéter ses effets.
Le format de l'identifiant et son producteur durable restent à implémenter avec
le journal : aucun compteur global ou horodatage n'est choisi par cet exemple.

L'empreinte ne se réduit pas aux octets de texte : elle doit couvrir les faits
effectifs dont dépend l'opération, notamment occurrences, positions, relations,
visibilité, configurations, recettes, modèles/dimensions et références aux
couches parentes pertinentes. La portée et la complétude des dépendances doivent
être établies ; une empreinte partielle ou de provenance inconnue ne justifie
aucune réutilisation. On vise les vues/couches concernées, sans invalider toutes
les branches pour une publication étrangère au plan.

Si P1 est devenu P3 mais que les dépendances requises sont requalifiées
identiques, les calculs et étapes préparées peuvent être réutilisés. On résout
un **nouveau plan figé pour P3**, avec son identité propre ; le plan et l'identité
de P1 restent immuables. L'empreinte ne contourne ni la vérification de génération
à l'admission/application ni le journal, les annulations, les effets déjà appliqués
ou la publication atomique. Le partage des contenus/vecteurs demeure indépendant
des occurrences et des publications.

La décision retire cette question de conception des attentes ; elle n'ajoute
aucun code durable dans A et ne qualifie pas une empreinte d'inventaire existante
comme génération ou état complet. Toute opération d'établissement initial ou d'avancement durable,
transfert, apply, activation, publication, copie/lecture d'une base de Lucie ou
politique de rétention reste hors de la tranche préparée.
