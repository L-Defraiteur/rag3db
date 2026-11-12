Revue statique ciblée de A, déjà intégré et testé. A lie des DÉCLARATIONS fournies à un plan figé : aucune observation de stockage, génération durable ou apply n’est revendiquée. Le runner peut lire ses fichiers, son adaptateur ne prétend pas être pur. Le contrat composite distingue publication P1/P3 et empreinte état H-A ; producteur durable et réutilisation future restent hors A. Cherche au maximum TROIS défauts concrets de complétude/canonicalisation/configurations relations/recettes/modèles ou liaison plan/provenance/immutabilité. Pour chacun : chemin/lignes, mécanisme, contre-exemple test et hypothèses manquantes. Si rien de démontré : « aucun constat établi ». Pas de long préambule, maximum 1200 mots. Attention : canonical_json appelle récursivement canonical(v) pour CHAQUE valeur objet, et pour les tableaux ; un avis précédent tronqué alléguait à tort le contraire. Ne confonds pas durcissement futur et bug de cette API de déclarations. Tests déjà verts, aucun besoin de réclamer une passe.

SOURCE d2cb7f0636d099823f63de2cef674b630a46bc6f:extension/rag3weaver/src/dataflow/migrations/facts.rs lignes 1-253
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

SOURCE d2cb7f0636d099823f63de2cef674b630a46bc6f:extension/rag3weaver/src/backend_schema.rs lignes 160-190
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

SOURCE d2cb7f0636d099823f63de2cef674b630a46bc6f:extension/rag3weaver/src/dataflow/migrations.rs lignes 307-324
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

SOURCE d2cb7f0636d099823f63de2cef674b630a46bc6f:extension/rag3weaver/src/dataflow/migrations/facts/tests.rs lignes 1-132
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

SOURCE d2cb7f0636d099823f63de2cef674b630a46bc6f:extension/rag3weaver/src/dataflow/migrations/facts/tests.rs lignes 175-220
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

SOURCE d2cb7f0636d099823f63de2cef674b630a46bc6f:extension/rag3weaver/src/dataflow/migrations/facts/tests.rs lignes 249-293
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

SOURCE 777820ef79d26af91b93687953bb8c06f790fbb8:docs/arbre-principal/12-11-2026/02-preparer-le-transfert-des-valeurs-connues.md lignes 340-380
340: 1. **Question tranchée le 12 novembre : génération de publication et empreinte
341:    d'état ont des rôles distincts.** Après l'explication A→B→A puis la proposition
342:    composite, Lucie valide la dernière recommandation : « oki pour ta
343:    recommandation alors ». La génération désigne une identité durable de
344:    publication ; l'empreinte de l'état logique permet le partage et la
345:    réutilisation du travail après requalification. Ce n'est pas le choix d'une
346:    génération définie uniquement par le contenu.
347: 
348: | État logique | Identité de publication | Empreinte d'état |
349: |---|---|---|
350: | A initial | P1 | H-A |
351: | B après nouvelle publication | P2 | H-B |
352: | Retour au même A | P3 | H-A |
353: 
354: Une publication nouvelle a une identité distincte, même si son état retrouve une
355: empreinte antérieure. Le rejeu de la **même** opération déjà applied conserve son
356: identité ; il ne crée ni nouvelle publication ni droit de répéter ses effets.
357: Le format de l'identifiant et son producteur durable restent à implémenter avec
358: le journal : aucun compteur global ou horodatage n'est choisi par cet exemple.
359: 
360: L'empreinte ne se réduit pas aux octets de texte : elle doit couvrir les faits
361: effectifs dont dépend l'opération, notamment occurrences, positions, relations,
362: visibilité, configurations, recettes, modèles/dimensions et références aux
363: couches parentes pertinentes. La portée et la complétude des dépendances doivent
364: être établies ; une empreinte partielle ou de provenance inconnue ne justifie
365: aucune réutilisation. On vise les vues/couches concernées, sans invalider toutes
366: les branches pour une publication étrangère au plan.
367: 
368: Si P1 est devenu P3 mais que les dépendances requises sont requalifiées
369: identiques, les calculs et étapes préparées peuvent être réutilisés. On résout
370: un **nouveau plan figé pour P3**, avec son identité propre ; le plan et l'identité
371: de P1 restent immuables. L'empreinte ne contourne ni la vérification de génération
372: à l'admission/application ni le journal, les annulations, les effets déjà appliqués
373: ou la publication atomique. Le partage des contenus/vecteurs demeure indépendant
374: des occurrences et des publications.
375: 
376: La décision retire cette question de conception des attentes ; elle n'ajoute
377: aucun code durable dans A et ne qualifie pas une empreinte d'inventaire existante
378: comme génération ou état complet. Toute opération d'établissement initial ou d'avancement durable,
379: transfert, apply, activation, publication, copie/lecture d'une base de Lucie ou
380: politique de rétention reste hors de la tranche préparée.
