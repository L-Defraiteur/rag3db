use super::*;
use crate::{config::{ChunkStrategy, DerivedConfig, FieldType, SimpleFieldDef},
    dataflow::migrations::plan::{MigrationPlanInputs, ResolvedMigrationStep, StorageGeneration},
    search::SearchSignals};
use std::collections::HashMap;

struct Fixture { plan: ResolvedMigrationPlan, inventory: TransferInventory, recipe: TransferRecipe }
fn descriptor(format: &str, generation: &str) -> StorageDescriptor {
    StorageDescriptor { engine_format: "synthetic-engine".into(), internal_schema: "8".into(),
        application_format: format.into(), recipe: "synthetic-recipe".into(),
        config_fingerprint: format!("synthetic-config-{format}"),
        generation: StorageGeneration::Established(generation.into()) }
}
fn fixture(entity: &str) -> Fixture {
    let source = descriptor("old", "synthetic-source-generation");
    let target_descriptor = descriptor("shared-v1", "synthetic-target-generation");
    let plan = ResolvedMigrationPlan::new(MigrationPlanInputs { source: source.clone(), target: target_descriptor.clone(),
        variables: BTreeMap::new(), schema_facts_fingerprint: "synthetic-facts".into() }, vec![ResolvedMigrationStep {
            version: 1, name: "synthetic".into(), checksum: "file".into(), graph_hash: "graph".into(),
        }]).unwrap();
    let mut config = EntityConfig {
        fields: HashMap::from([
            ("name".into(), SimpleFieldDef { field_type: FieldType::Text, is_title: true, ..Default::default() }),
            ("content".into(), SimpleFieldDef { field_type: FieldType::Text, is_content: true, ..Default::default() }),
            ("docstring".into(), SimpleFieldDef { field_type: FieldType::Text, is_content: true, ..Default::default() }),
        ]), signals: SearchSignals::HYBRID, ..Default::default()
    };
    config.chunking.max_size = 64;
    config.chunking.overlap = 8;
    config.chunking.title_max_chars = 0;
    config.chunking.strategy = ChunkStrategy::Lines;
    config.chunking.max_lines = 2;
    config.chunking.overlap_lines = 0;
    let mut target = config.clone();
    target.derived = Some(DerivedConfig { from: entity.into(), identity: DerivedIdentity::Content { format: "shared-v1".into() },
        gather: vec![], render: ["name", "content", "docstring"].into_iter()
            .map(|f| (f.into(), format!("{{{{ root.{f} }}}}"))).collect() });
    let models = vec![
        EmbeddingModelEntry { name: "model-4".into(), dim: 4, storage: StorageKind::Legacy },
        EmbeddingModelEntry { name: "model-6".into(), dim: 6, storage: StorageKind::Suffixed },
    ];
    let mut target_models = models.clone();
    target_models[0].storage = StorageKind::Suffixed;
    let mut inventory = TransferInventory { declared_source_identity: Some("synthetic-instance".into()), descriptor: source,
        source_entity: entity.into(), source_config: config.clone(), models,
        sparse: SparseEvidence::default(), dense_debt: false, entities: vec![], relations: vec![], chunks: vec![] };
    for (cell, labels) in [("Alpha", vec!["A","B","C","D","E","F"]), ("Beta", vec!["G","H"])] {
        for (i,label) in labels.into_iter().enumerate() {
            let key = EntityKey { entity: entity.into(), scope: Scope::new("fixture",cell), uuid: label.into() };
            let file = EntityKey { entity: "File".into(), scope: key.scope.clone(), uuid: format!("file-{label}") };
            let name = if label == "E" { "different" } else { "name" };
            let content = if label == "C" { "pub fn café() { 2 }" } else { "pub fn café() { 1 }" };
            let doc = if label == "D" { "different documentation" } else if label == "F" { "" } else { "documentation" };
            let values = BTreeMap::from([
                ("name".into(), CypherValue::String(name.into())), ("content".into(), CypherValue::String(content.into())),
                ("docstring".into(), CypherValue::String(doc.into())),
                ("file_path".into(), CypherValue::String(format!("{label}.rs"))),
                ("start_line".into(), CypherValue::Int((i*10) as i64)),
                ("_uuid".into(), CypherValue::String(label.into())),
            ]);
            inventory.entities.push(InventoryEntity { key: key.clone(), values: values.clone(),
                masks: if label=="B" { BTreeSet::from(["hidden-by-task".into()]) } else { BTreeSet::new() } });
            inventory.entities.push(InventoryEntity { key: file.clone(), values: BTreeMap::from([
                ("path".into(), CypherValue::String(format!("{label}.rs"))),
            ]), masks: BTreeSet::new() });
            let relation = InventoryRelation { name: "DEFINES".into(), from: file.clone(), to: key.clone(),
                properties: BTreeMap::from([("kind".into(), CypherValue::String("function".into()))]) };
            inventory.relations.push(relation.clone());
            if label == "A" { inventory.relations.push(relation); } // multiplicity matters
            inventory.relations.push(InventoryRelation { name: "DEFINED_IN".into(), from: key.clone(), to: file,
                properties: BTreeMap::new() });
            let chunker = Chunker::new(ChunkerConfig::from(&config.chunking));
            let mut offset=0;
            for (field_i,field) in config.content_fields().into_iter().enumerate() {
                if field_i>0 { offset+=2; }
                let text=values[field].as_str().unwrap();
                for chunk in chunker.chunk(text) {
                    let uuid=chunk_uuid(label,field,chunk.index);
                    let dense=inventory.models.iter().enumerate().map(|(m,model)| KnownDenseValue {
                        model:model.name.clone(), input:Some(chunk.text.clone()), marker:content_hash(&chunk.text),
                        values:vec![(m+1) as f32;model.dim],
                    }).collect();
                    inventory.chunks.push(InventoryChunk { parent:key.clone(), uuid,
                        shape:ChunkShape { field:field.into(),title:name.into(),content_offset:offset,chunk }, dense });
                }
                offset+=text.len();
            }
        }
    }
    Fixture { plan, inventory, recipe:TransferRecipe { target_descriptor,
        target_entity:format!("{entity}Content"),target_config:target,models:target_models } }
}
fn run(f: &Fixture) -> Result<KnownTransferPreview, Vec<TransferRefusal>> {
    preview_known_transfer(&f.plan,Some("synthetic-instance"),&f.inventory,&f.recipe)
}
fn rejects(f: &Fixture, code: RefusalCode) {
    let before=format!("{:?}",f.inventory);
    let errors=run(f).expect_err("invalid supplied facts must refuse, not partially succeed");
    assert_eq!(errors.len(),1);
    assert_eq!(errors[0].code,code,"{errors:?}");
    assert!(!errors[0].location.is_empty());
    assert_eq!(format!("{:?}",f.inventory),before,"preview mutated its input");
}

#[test]
fn canonical_fixture_preserves_occurrences_graph_models_and_local_coordinates() {
    let f=fixture("Scope");
    let result=run(&f).unwrap();
    assert_eq!(result.referenced_plan_identity(),f.plan.identity());
    assert_eq!(result.plan_qualification(),PlanQualification::UnqualifiedConfigAndSchemaFacts);
    assert_eq!(result.versions().len(),6);
    assert_eq!(result.retained_entities().len(),16);
    assert_eq!(result.retained_relations().len(),17);
    assert_eq!(result.versions().iter().map(|v|v.occurrences.len()).sum::<usize>(),8);
    let shared=result.versions().iter().find(|v|v.occurrences.contains_key("A")).unwrap();
    assert_eq!(shared.occurrences.keys().cloned().collect::<Vec<_>>(),vec!["A","B"]);
    assert_ne!(shared.occurrences["A"],shared.occurrences["B"],"render input state belongs to each occurrence");
    for entity in &f.inventory.entities { assert!(result.retained_entities().contains(entity)); }
    for relation in &f.inventory.relations { assert!(result.retained_relations().contains(relation)); }
    assert_eq!(result.retained_relations().iter().filter(|r|r.name=="DEFINES" && r.to.uuid=="A").count(),2);
    for chunk in result.chunks() {
        assert_eq!(chunk.uuid,chunk_uuid(&chunk.parent.uuid,&chunk.shape.field,chunk.shape.chunk.index));
        assert_eq!(chunk.dense.len(),2);
        let d=&chunk.dense["model-4"];
        assert_eq!(d.value_bits,vec![1f32.to_bits();4]);
        assert_eq!(chunk.dense["model-6"].value_bits,vec![2f32.to_bits();6]);
        assert_eq!(d.source_storage.column,"embedding");
        assert_eq!(d.target_storage.column,"embedding__model_4");
        assert_eq!(d.source_storage.marker,"_embed_hash");
        assert_eq!(d.target_storage.marker,"_embed_hash__model_4");
        let version=result.versions().iter().find(|v|v.key==chunk.parent).unwrap();
        for source in &d.sources {
            assert!(f.inventory.chunks.iter().any(|c|c.uuid==*source && c.parent.scope==chunk.parent.scope
                && version.occurrences.contains_key(&c.parent.uuid) && c.shape==chunk.shape));
        }
    }
    let d=result.chunks().iter().find(|c|c.parent==shared.key).unwrap();
    assert_eq!(d.dense["model-4"].sources.len(),2);
    assert!(!result.identity().is_empty());
    assert!(!result.inventory_fingerprint().is_empty());
}

#[test]
fn permutations_and_config_map_order_do_not_change_preview_identity() {
    let mut f=fixture("Scope");let original=run(&f).unwrap();
    f.inventory.entities.reverse();f.inventory.relations.reverse();f.inventory.chunks.reverse();
    f.inventory.models.reverse();f.recipe.models.reverse();
    for c in &mut f.inventory.chunks { c.dense.reverse(); }
    let mut fields=f.recipe.target_config.fields.drain().collect::<Vec<_>>();fields.reverse();
    f.recipe.target_config.fields=fields.into_iter().collect();
    assert_eq!(encoded(&run(&f).unwrap()).unwrap(),encoded(&original).unwrap());
}
#[test]
fn arbitrary_declared_entity_uses_the_same_identity_and_coverage_primitives() {
    let f=fixture("Document");let p=run(&f).unwrap();
    assert_eq!(p.versions().len(),6);
    assert!(p.versions().iter().all(|v|v.key.entity=="DocumentContent"));
    assert!(p.chunks().iter().all(|c|c.parent.entity=="DocumentContent"));
}
#[test]
fn expected_identity_is_explicit_not_produced_by_the_fixed_plan() {
    let f=fixture("Scope");
    assert_eq!(preview_known_transfer(&f.plan,None,&f.inventory,&f.recipe).unwrap_err()[0].code,RefusalCode::ExpectedIdentityMissing);
    let mut f=fixture("Scope");f.inventory.declared_source_identity=None;rejects(&f,RefusalCode::SourceIdentityMissing);
    f.inventory.declared_source_identity=Some("foreign-instance".into());rejects(&f,RefusalCode::ForeignSourceIdentity);
}
#[test]
fn unversioned_generation_and_foreign_supplied_descriptor_refuse() {
    let mut f=fixture("Scope");f.inventory.descriptor.generation=StorageGeneration::Unversioned;
    rejects(&f,RefusalCode::DescriptorMismatch);
    let mut inputs=f.plan.inputs().clone();inputs.source.generation=StorageGeneration::Unversioned;
    f.plan=ResolvedMigrationPlan::new(inputs,f.plan.steps().to_vec()).unwrap();rejects(&f,RefusalCode::GenerationUnavailable);
    let mut f=fixture("Scope");f.recipe.target_descriptor.recipe.push_str("-foreign");rejects(&f,RefusalCode::DescriptorMismatch);
}
#[test]
fn sparse_signals_materialization_debt_and_contradictions_refuse_but_reader_alone_does_not() {
    let mut f=fixture("Scope");f.inventory.sparse.reader_declared=true;assert!(run(&f).is_ok());
    for case in 0..4 {
        let mut f=fixture("Scope");
        match case {0=>f.inventory.source_config.signals=SearchSignals::HYBRID|SearchSignals::SPARSE,
            1=>f.inventory.sparse.materialized=true,2=>f.inventory.sparse.debt=true,_=>f.inventory.sparse.inconsistent=true}
        rejects(&f,RefusalCode::SparseUnsupported);
    }
    let mut f=fixture("Scope");f.inventory.dense_debt=true;rejects(&f,RefusalCode::DenseDebt);
}
#[test]
fn all_declared_models_must_cover_every_source_even_if_another_occurrence_covers_destination() {
    let mut f=fixture("Scope");f.inventory.chunks.iter_mut().find(|c|c.parent.uuid=="B").unwrap().dense.pop();
    rejects(&f,RefusalCode::DenseCoverage);
    let mut f=fixture("Scope");let duplicate=f.inventory.chunks[0].dense[0].clone();f.inventory.chunks[0].dense.push(duplicate);
    rejects(&f,RefusalCode::DenseCoverage);
}
#[test]
fn missing_input_stale_marker_wrong_dimensions_and_nonfinite_values_are_named() {
    for case in 0..6 {
        let mut f=fixture("Scope");let v=&mut f.inventory.chunks[0].dense[0];
        let code=match case {
            0=>{v.input=None;RefusalCode::InputProvenanceUnavailable},
            1=>{v.input=Some("title plus text".into());RefusalCode::InputProvenanceUnavailable},
            2=>{v.marker="stale".into();RefusalCode::StaleMarker},
            3=>{v.values.pop();RefusalCode::InvalidVector},
            4=>{v.values[0]=f32::NAN;RefusalCode::InvalidVector},
            _=>{v.values[0]=f32::INFINITY;RefusalCode::InvalidVector},
        };rejects(&f,code);
    }
}
#[test]
fn conflicting_known_values_never_choose_first_and_zero_sign_is_not_approximated() {
    for pair in [(1.0,1.0000001),(0.0,-0.0)] {
        let mut f=fixture("Scope");
        for chunk in f.inventory.chunks.iter_mut().filter(|c|c.parent.uuid=="A" || c.parent.uuid=="B") {
            chunk.dense[0].values[0]=if chunk.parent.uuid=="A" {pair.0}else{pair.1};
        }
        rejects(&f,RefusalCode::ConflictingVectors);
        f.inventory.chunks.reverse();rejects(&f,RefusalCode::ConflictingVectors);
    }
}
#[test]
fn model_name_dimension_collision_and_legacy_aliasing_are_refused() {
    for case in 0..5 {
        let mut f=fixture("Scope");match case {
            0=>f.recipe.models[0].name="another".into(),1=>f.recipe.models[0].dim=8,
            2=>f.inventory.models[1].name="model_4".into(),
            3=>f.inventory.models[1].storage=StorageKind::Legacy,
            _=>f.inventory.chunks[0].dense[0].model="unknown".into(),
        }rejects(&f,RefusalCode::ModelMismatch);
    }
}
#[test]
fn incomplete_extra_duplicate_and_corrupt_source_chunks_refuse() {
    for case in 0..7 {
        let mut f=fixture("Scope");match case {
            0=>{f.inventory.chunks.pop();},
            1=>{f.inventory.chunks.push(f.inventory.chunks[0].clone());},
            2=>f.inventory.chunks[0].shape.chunk.core_start_byte+=1,
            3=>f.inventory.chunks[0].shape.chunk.end_line+=1,
            4=>f.inventory.chunks[0].shape.content_offset+=1,
            5=>f.inventory.chunks[0].shape.title.push('!'),
            _=>f.inventory.chunks[0].parent.uuid="missing".into(),
        }rejects(&f,RefusalCode::ChunkCoverage);
    }
}
#[test]
fn relation_endpoints_system_columns_and_unfingerprintable_values_refuse() {
    for case in 0..5 {
        let mut f=fixture("Scope");match case {
            0=>f.inventory.relations[0].to.uuid="missing".into(),
            1=>{f.inventory.entities.push(f.inventory.entities[0].clone());},
            2=>{f.inventory.entities[0].values.insert("_org".into(),CypherValue::String("foreign".into()));},
            3=>{f.inventory.entities[0].values.insert("opaque".into(),CypherValue::Blob(vec![1]));},
            _=>{f.inventory.relations[0].properties.insert("weight".into(),CypherValue::Float(f64::NAN));},
        }rejects(&f,RefusalCode::InvalidInventory);
    }
}
#[test]
fn recipe_changes_and_named_templates_refuse_without_reading_a_template() {
    for case in 0..8 {
        let mut f=fixture("Scope");match case {
            0=>f.recipe.target_config.chunking.overlap+=1,
            1=>f.recipe.target_config.fields.get_mut("name").unwrap().is_title=false,
            2=>{f.recipe.target_config.derived.as_mut().unwrap().render.insert("content".into(),"{{ root.content }}changed".into());},
            3=>{f.recipe.target_config.derived.as_mut().unwrap().render.insert("content".into(),"unresolved-user-template".into());},
            4=>f.recipe.target_config.chunked=Some(false),
            5=>f.recipe.target_config.fields.get_mut("name").unwrap().boost=Some(f64::NAN),
            6=>f.recipe.target_config.fusion=Some(crate::search::FusionConfig {rrf_k:f64::NAN,..Default::default()}),
            _=>f.recipe.target_config.field_weights.push(crate::search::FieldWeight {
                field:"kind".into(),weights:BTreeMap::new(),default:f64::INFINITY,
            }),
        }rejects(&f,RefusalCode::UnsupportedRecipe);
    }
}
#[test]
fn occurrence_visibility_and_graph_changes_are_retained_and_fingerprinted() {
    let mut f=fixture("Scope");let before=run(&f).unwrap();
    f.inventory.entities[0].masks.insert("new-mask".into());
    let after=run(&f).unwrap();assert_ne!(after.identity(),before.identity());
    assert_ne!(after.inventory_fingerprint(),before.inventory_fingerprint());
    assert_eq!(after.chunks(),before.chunks());
    assert_eq!(after.versions(),before.versions());
    f.inventory.relations[0].properties.insert("changed".into(),CypherValue::Bool(true));
    assert_ne!(run(&f).unwrap().identity(),after.identity());
}

#[test]
fn unchanged_descriptor_never_qualifies_altered_configs_or_recipe_against_the_plan() {
    let mut f=fixture("Scope");let original=run(&f).unwrap();
    // The descriptor labels remain unchanged; this module has no common
    // producer proving they cover these full configs or the provider's facts.
    f.inventory.source_config.return_fields=Some(vec!["name".into()]);
    let changed_source=run(&f).unwrap();
    assert_eq!(changed_source.plan_qualification(),PlanQualification::UnqualifiedConfigAndSchemaFacts);
    assert_eq!(changed_source.referenced_plan_identity(),original.referenced_plan_identity());
    assert_ne!(changed_source.identity(),original.identity());
    assert_ne!(changed_source.inventory_fingerprint(),original.inventory_fingerprint());
    f.recipe.target_config.return_fields=Some(vec!["content".into()]);
    let changed_target=run(&f).unwrap();
    assert_eq!(changed_target.plan_qualification(),PlanQualification::UnqualifiedConfigAndSchemaFacts);
    assert_ne!(changed_target.identity(),changed_source.identity());
    f.inventory.source_config.chunking.title_max_chars+=1;
    f.recipe.target_config.chunking.title_max_chars+=1;
    let changed_recipe=run(&f).unwrap();
    assert_eq!(changed_recipe.plan_qualification(),PlanQualification::UnqualifiedConfigAndSchemaFacts);
    assert_ne!(changed_recipe.identity(),changed_target.identity());
    assert_eq!(f.inventory.descriptor,f.plan.inputs().source);
    assert_eq!(f.recipe.target_descriptor,f.plan.inputs().target);
    let report=serde_json::to_value(&changed_recipe).unwrap();
    assert_eq!(report["plan_qualification"],"unqualified_config_and_schema_facts");
    assert!(report.get("fixed_plan_identity").is_none());
}
