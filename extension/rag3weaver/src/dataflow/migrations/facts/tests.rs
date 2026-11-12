use super::*;
use crate::{config::{DerivedConfig, DerivedIdentity, FieldDef, FieldType, SimpleFieldDef},
    dataflow::{migrations::{MigrationRunner, plan::{PlanJournalState, ResolvedMigrationStep, StoragePosition}}, node_registry::NodeRegistry}};
use std::{collections::HashMap, sync::Arc};

fn config() -> EntityConfig {
    EntityConfig { fields: HashMap::from([
        ("title".into(), SimpleFieldDef { field_type: FieldType::Text, is_title: true, ..Default::default() }),
        ("body".into(), SimpleFieldDef { field_type: FieldType::Text, is_content: true, ..Default::default() }),
    ]), signals: SearchSignals::BM25, ..Default::default() }
}
fn recipe() -> RecipeFactsInput {
    let mut source = ModelSource::local("synthetic-4");source.dimensions=Some(4);
    RecipeFactsInput { id: "synthetic-recipe".into(), resolved_templates: Some(BTreeMap::new()),
        model_sources: Some(BTreeMap::from([(Capability::Embed,source)])),
        dense_models: Some(BTreeMap::from([("synthetic-4".into(), EmbeddingModelEntry {
            name:"synthetic-4".into(),dim:4,storage:StorageKind::Legacy,
        })])) }
}
fn input() -> DeclaredSchemaInput {
    DeclaredSchemaInput { entities: Some(BTreeMap::from([("Document".into(),config()),("Collection".into(),config())])),
        relations: Some(BTreeMap::from([("CONTAINS".into(),BackendRelation { def:RelationDef {
            from:"Collection".into(),to:"Document".into(),properties:Some(HashMap::from([("reason".into(),
                serde_json::from_value::<FieldDef>(serde_json::json!({"type":"string"})).unwrap())])),
        },from_states:Some(vec!["open".into(),"review".into()]),to_states:None,description:Some("ownership".into()) })])),
        provider:ProviderFacts::Absent,index_signals:BTreeMap::new(),recipe:recipe() }
}
fn derived_input() -> DeclaredSchemaInput {
    let mut i=input();let mut derived=config();
    derived.derived=Some(DerivedConfig {from:"Document".into(),identity:DerivedIdentity::Content {format:"document-content-v1".into()},
        gather:vec![],render:BTreeMap::from([("title".into(),"{{ root.title }}".into()),("body".into(),"body-template".into())])});
    i.entities.as_mut().unwrap().insert("DocumentContent".into(),derived);
    i.recipe.resolved_templates.as_mut().unwrap().insert("body-template".into(),"{{ root.body }}".into());
    i
}
fn facts(source:DeclaredSchemaInput,target:DeclaredSchemaInput) -> DeclaredMigrationFacts {
    DeclaredMigrationFacts::new(DeclaredSchemaFacts::from_complete_input(source).unwrap(),
        DeclaredSchemaFacts::from_complete_input(target).unwrap()).unwrap()
}
fn formats(name:&str) -> StorageFormatFacts {
    StorageFormatFacts {engine_format:"synthetic-engine".into(),internal_schema:"8".into(),application_format:name.into(),
        generation:StorageGeneration::Unversioned}
}
fn plan(f:&DeclaredMigrationFacts) -> ResolvedMigrationPlan {
    ResolvedMigrationPlan::new(f.bind_inputs(formats("old"),formats("new"),BTreeMap::new()).unwrap(),vec![ResolvedMigrationStep {
        version:1,name:"fixture".into(),checksum:"fixture-file".into(),graph_hash:"fixture-graph".into(),
    }]).unwrap()
}
fn mismatch(f:&DeclaredMigrationFacts,p:&ResolvedMigrationPlan,label:&str) {
    assert_eq!(f.compare_with_fixed_plan(p).unwrap_err(),FactsError::Mismatch(label.into()));
}

#[test]
fn runner_consumes_the_producer_and_returns_only_a_supplied_facts_match() {
    let f=facts(input(),derived_input());let d=tempfile::tempdir().unwrap();
    std::fs::write(d.path().join("001_check.mmd"),"graph TD\ncheck[\"ValidateNode(query='$query', assert='count == 1')\"]\n").unwrap();
    let mut runner=MigrationRunner::new(Arc::new(NodeRegistry::new()));runner.add_dir(d.path().to_owned());
    let vars=BTreeMap::from([("query".into(),"RETURN 1".into())]);
    let (p,matched)=runner.resolve_plan_from_facts(&f,formats("old"),formats("new"),vars.clone()).unwrap();
    assert_eq!(p.inputs().variables,vars);
    assert_eq!(matched.referenced_plan_identity(),p.identity());
    assert_eq!(matched.schema_facts_fingerprint(),f.schema_facts_fingerprint());
    assert_eq!(matched.provenance(),FactsProvenance::SuppliedDeclarations);
    assert_eq!(f.compare_with_fixed_plan(&p).unwrap().referenced_plan_identity(),p.identity());
    assert!(p.inputs().source.config_fingerprint.starts_with(CONFIG_DOMAIN));
    assert_eq!(p.inputs().source.config_fingerprint,f.source().config_fingerprint());
    assert!(p.validate_generation_preconditions().is_err(),"facts match must not establish a generation");
    let report=serde_json::to_value(matched).unwrap();
    assert_eq!(report["provenance"],"SuppliedDeclarations");
    assert!(report.get("applied").is_none());assert!(report.get("storage_observed").is_none());
    let vars=BTreeMap::from([("query".into(),"RETURN 2".into())]);
    let (other,_)=runner.resolve_plan_from_facts(&f,formats("old"),formats("new"),vars).unwrap();
    assert_ne!(p.identity(),other.identity());assert_ne!(p.steps()[0].graph_hash,other.steps()[0].graph_hash);
}

#[test]
fn map_insertion_order_is_irrelevant_but_semantic_list_order_is_preserved() {
    let a=input();let mut b=a.clone();
    for c in b.entities.as_mut().unwrap().values_mut() {
        let mut entries=c.fields.drain().collect::<Vec<_>>();entries.reverse();c.fields=entries.into_iter().collect();
    }
    let original=facts(a.clone(),a);let reordered=facts(b.clone(),b.clone());
    assert_eq!(original.schema_facts_fingerprint(),reordered.schema_facts_fingerprint());
    assert_eq!(plan(&original).identity(),plan(&reordered).identity());
    b.relations.as_mut().unwrap().get_mut("CONTAINS").unwrap().from_states.as_mut().unwrap().reverse();
    let changed=facts(input(),b);mismatch(&changed,&plan(&original),"schema facts");
    assert_eq!(changed.target().config_fingerprint(),original.target().config_fingerprint());
}

#[test]
fn complete_empty_and_unavailable_are_different_inputs() {
    for part in 0..6 {
        let mut i=input();match part {
            0=>i.entities=None,1=>i.relations=None,2=>i.recipe.resolved_templates=None,
            3=>i.recipe.model_sources=None,4=>i.recipe.dense_models=None,
            _=>i.provider=ProviderFacts::Unavailable{name:"declared-provider".into()},
        }
        assert!(matches!(DeclaredSchemaFacts::from_complete_input(i),Err(FactsError::Incomplete(_))));
    }
    let mut i=input();i.entities=Some(BTreeMap::new());i.relations=Some(BTreeMap::new());
    let complete=DeclaredSchemaFacts::from_complete_input(i).unwrap();
    assert!(complete.entities().is_empty());assert!(complete.relations().is_empty());
}

#[test]
fn final_added_dependencies_and_overrides_use_the_existing_schema_validator() {
    let mut i=derived_input();i.entities.as_mut().unwrap().remove("Document");
    assert!(matches!(DeclaredSchemaFacts::from_complete_input(i),Err(FactsError::Invalid(_))),"compose's empty provider pass is insufficient");
    let mut i=input();i.relations.as_mut().unwrap().get_mut("CONTAINS").unwrap().def.to="Missing".into();
    assert!(matches!(DeclaredSchemaFacts::from_complete_input(i),Err(FactsError::Invalid(_))));
    let mut i=input();i.index_signals.insert("Missing".into(),SearchSignals::BM25);
    assert!(matches!(DeclaredSchemaFacts::from_complete_input(i),Err(FactsError::Invalid(_))));
    let mut i=input();i.entities.as_mut().unwrap().get_mut("Document").unwrap().chunked=Some(false);
    i.index_signals.insert("Document".into(),SearchSignals::VECTOR);
    assert!(matches!(DeclaredSchemaFacts::from_complete_input(i),Err(FactsError::Invalid(_))),"effective overrides must be validated");
}

#[test]
fn provider_definitions_are_composed_and_conflicts_are_not_hidden() {
    let supplied=input();
    let provided_entities=supplied.entities.clone().unwrap();
    let provided_relations=supplied.relations.as_ref().unwrap().iter().map(|(n,r)|(n.clone(),r.def.clone())).collect();
    let mut i=input();i.entities=Some(BTreeMap::new());i.relations=Some(BTreeMap::new());
    i.provider=ProviderFacts::Complete{name:"document-provider".into(),entities:provided_entities,relations:provided_relations};
    let composed=DeclaredSchemaFacts::from_complete_input(i.clone()).unwrap();assert_eq!(composed.entities().len(),2);
    assert_eq!(composed.relations()["CONTAINS"].from,"Collection");
    let mut c=config();c.chunking.max_size+=1;i.entities.as_mut().unwrap().insert("Document".into(),c);
    assert!(matches!(DeclaredSchemaFacts::from_complete_input(i),Err(FactsError::Invalid(_))));
}

#[cfg(feature="code")]
#[test]
fn actual_code_provider_and_effective_signal_overrides_are_in_the_facts() {
    let (entities,relations)=crate::code::schema_configs(crate::code::default_scope_chunking());
    let expected_entities=entities.len();let expected_relations=relations.len();
    let mut i=input();i.entities=Some(BTreeMap::new());i.relations=Some(BTreeMap::new());
    i.provider=ProviderFacts::Complete{name:"code".into(),entities,relations};
    let initial=DeclaredSchemaFacts::from_complete_input(i.clone()).unwrap();
    i.index_signals.insert("Scope".into(),SearchSignals::BM25);
    let effective=DeclaredSchemaFacts::from_complete_input(i).unwrap();
    assert_eq!(effective.entities().len(),expected_entities);assert_eq!(effective.relations().len(),expected_relations);
    assert_eq!(effective.entities()["Scope"].signals,SearchSignals::BM25);
    assert_ne!(effective.config_fingerprint(),initial.config_fingerprint());
    assert_eq!(effective.relations()["DEFINES"].to,"Scope");
}

#[test]
fn effective_merged_schema_fields_are_not_replaced_with_a_minimal_fixture() {
    let initial=facts(input(),input());let p=plan(&initial);
    let mut i=input();i.entities.as_mut().unwrap().get_mut("Document").unwrap().fields.insert("schema_extra".into(),
        SimpleFieldDef{field_type:FieldType::Int64,..Default::default()});
    let changed=facts(input(),i);mismatch(&changed,&p,"target configs");
    assert!(changed.target().entities()["Document"].fields.contains_key("schema_extra"));
}

#[test]
fn config_roles_chunking_signals_and_defaults_are_bound_not_just_labeled() {
    let original=facts(input(),input());let p=plan(&original);
    for case in 0..5 {
        let mut i=input();let c=i.entities.as_mut().unwrap().get_mut("Document").unwrap();
        match case {
            0=>c.fields.get_mut("title").unwrap().is_title=false,
            1=>c.chunking.overlap+=1,
            2=>c.return_fields=Some(vec!["body".into(),"title".into()]),
            3=>c.fields.get_mut("body").unwrap().boost=Some(2.0),
            _=>{i.index_signals.insert("Document".into(),SearchSignals::HYBRID);},
        }
        let changed=facts(input(),i);mismatch(&changed,&p,"target configs");
        assert_ne!(plan(&changed).identity(),p.identity());
    }
}

#[test]
fn relation_direction_properties_and_policies_bind_the_full_facts_not_entity_digest() {
    let original=facts(input(),input());let p=plan(&original);
    for case in 0..4 {
        let mut i=input();let r=i.relations.as_mut().unwrap().get_mut("CONTAINS").unwrap();
        match case {
            0=>{std::mem::swap(&mut r.def.from,&mut r.def.to);},
            1=>{r.def.properties.as_mut().unwrap().get_mut("reason").unwrap().default_value=Some(serde_json::json!("changed"));},
            2=>r.to_states=Some(vec!["hidden".into()]),
            _=>{i.relations.as_mut().unwrap().remove("CONTAINS");},
        }
        let changed=facts(input(),i);
        assert_eq!(changed.target().config_fingerprint(),original.target().config_fingerprint());
        mismatch(&changed,&p,"schema facts");
    }
}

#[test]
fn named_recipe_requires_supplied_bytes_and_tracks_them_without_file_resolution() {
    let mut i=derived_input();i.recipe.resolved_templates.as_mut().unwrap().clear();
    assert_eq!(DeclaredSchemaFacts::from_complete_input(i).unwrap_err(),FactsError::UnresolvedRecipe("body-template".into()));
    let original=facts(input(),derived_input());let p=plan(&original);
    let mut i=derived_input();i.recipe.resolved_templates.as_mut().unwrap().insert("body-template".into(),"{{ root.body }}\n".into());
    let changed=facts(input(),i);mismatch(&changed,&p,"schema facts");
    assert_eq!(changed.target().config_fingerprint(),original.target().config_fingerprint());
    let mut i=derived_input();i.recipe.resolved_templates.as_mut().unwrap().insert("unused".into(),"bytes".into());
    assert!(matches!(DeclaredSchemaFacts::from_complete_input(i),Err(FactsError::Invalid(_))));
}

#[test]
fn models_dimensions_storage_and_reader_declarations_are_part_of_the_recipe() {
    let original=facts(input(),input());let p=plan(&original);
    for case in 0..4 {
        let mut i=input();match case {
            0=>i.recipe.dense_models.as_mut().unwrap().get_mut("synthetic-4").unwrap().storage=StorageKind::Suffixed,
            1=>{i.recipe.dense_models.as_mut().unwrap().get_mut("synthetic-4").unwrap().dim=8;
                i.recipe.model_sources.as_mut().unwrap().get_mut(&Capability::Embed).unwrap().dimensions=Some(8);},
            2=>{i.recipe.model_sources.as_mut().unwrap().insert(Capability::Sparse,ModelSource::service("highlight-reader"));},
            _=>i.recipe.id.push_str("-changed"),
        }
        let changed=facts(input(),i);mismatch(&changed,&p,"schema facts");
        assert_eq!(changed.target().config_fingerprint(),original.target().config_fingerprint());
    }
}

#[test]
fn malformed_or_missing_model_facts_refuse_without_connecting_any_model() {
    for case in 0..4 {
        let mut i=input();match case {
            0=>i.recipe.model_sources.as_mut().unwrap().get_mut(&Capability::Embed).unwrap().dimensions=Some(8),
            1=>i.recipe.dense_models.as_mut().unwrap().get_mut("synthetic-4").unwrap().dim=0,
            2=>{i.recipe.dense_models.as_mut().unwrap().clear();i.entities.as_mut().unwrap().get_mut("Document").unwrap().signals=SearchSignals::VECTOR;},
            _=>{let mut m=i.recipe.dense_models.as_ref().unwrap()["synthetic-4"].clone();m.name="synthetic_4".into();
                i.recipe.dense_models.as_mut().unwrap().insert(m.name.clone(),m);},
        }
        assert!(DeclaredSchemaFacts::from_complete_input(i).is_err());
    }
}

#[test]
fn opaque_legacy_fingerprints_and_unknown_contract_versions_are_not_requalified() {
    let f=facts(input(),input());
    for case in 0..3 {
        let mut inputs=plan(&f).inputs().clone();let steps=plan(&f).steps().to_vec();
        match case {0=>inputs.source.config_fingerprint="opaque-label".into(),
            1=>inputs.target.config_fingerprint=format!("declared-entity-configs:v2:{}","0".repeat(64)),
            _=>inputs.schema_facts_fingerprint="schema-name-not-facts".into(),}
        let p=ResolvedMigrationPlan::new(inputs,steps).unwrap();
        assert!(matches!(f.compare_with_fixed_plan(&p),Err(FactsError::UnknownContract(_))));
    }
    let mut inputs=plan(&f).inputs().clone();inputs.source.recipe="different-label".into();
    let p=ResolvedMigrationPlan::new(inputs,plan(&f).steps().to_vec()).unwrap();mismatch(&f,&p,"recipe identifier");
}

#[test]
fn facts_match_with_unversioned_and_foreign_generation_does_not_qualify_storage() {
    let f=facts(input(),input());let p=plan(&f);
    assert!(f.compare_with_fixed_plan(&p).is_ok());
    assert_eq!(p.inputs().source.generation,StorageGeneration::Unversioned);
    assert!(p.qualify_storage(&p.inputs().source,&PlanJournalState::Unapplied).is_err());
    let mut source=formats("old");source.generation=StorageGeneration::Established("explicit-synthetic-source".into());
    let mut target=formats("new");target.generation=StorageGeneration::Established("explicit-synthetic-target".into());
    let inputs=f.bind_inputs(source,target,BTreeMap::new()).unwrap();
    let p=ResolvedMigrationPlan::new(inputs,p.steps().to_vec()).unwrap();let id=p.identity().to_string();
    assert_eq!(p.qualify_storage(&p.inputs().source,&PlanJournalState::Unapplied).unwrap(),StoragePosition::Source);
    assert_eq!(p.qualify_storage(&p.inputs().target,&PlanJournalState::Applied{identity:id.clone()}).unwrap(),StoragePosition::PublishedTarget);
    let mut foreign=p.inputs().target.clone();foreign.generation=StorageGeneration::Established("foreign-generation".into());
    assert!(p.qualify_storage(&foreign,&PlanJournalState::Applied{identity:id}).is_err());
    assert!(f.compare_with_fixed_plan(&p).is_ok(),"facts correspondence is not data-generation qualification");
}

#[test]
fn schema_and_provider_nonfinite_values_refuse_before_json_can_turn_them_into_null() {
    for case in 0..3 {
        let mut i=input();match case {
            0=>i.entities.as_mut().unwrap().get_mut("Document").unwrap().fields.get_mut("body").unwrap().boost=Some(f64::NAN),
            1=>i.relations.as_mut().unwrap().get_mut("CONTAINS").unwrap().def.properties.as_mut().unwrap().get_mut("reason").unwrap().boost=Some(f64::INFINITY),
            _=>{let mut e=i.entities.clone().unwrap();e.get_mut("Document").unwrap().fields.get_mut("body").unwrap().boost=Some(f64::NEG_INFINITY);
                i.provider=ProviderFacts::Complete{name:"bad-provider".into(),entities:e,relations:BTreeMap::new()};},
        }
        assert!(matches!(DeclaredSchemaFacts::from_complete_input(i),Err(FactsError::Invalid(_))));
    }
}

#[test]
fn immutable_snapshot_does_not_follow_later_mutation_of_its_source_declarations() {
    let mut i=input();let fixed=facts(i.clone(),i.clone());let p=plan(&fixed);let identity=p.identity().to_string();
    i.entities.as_mut().unwrap().get_mut("Document").unwrap().chunking.max_size+=10;
    let changed=facts(i.clone(),i);mismatch(&changed,&p,"source configs");
    assert_eq!(p.identity(),identity);assert!(fixed.compare_with_fixed_plan(&p).is_ok());
}

#[test]
fn canonical_encoding_mechanism_keeps_object_keys_sorted_and_list_order_exact() {
    let value=serde_json::json!({"z":[2,1],"a":{"b":true,"a":"é"}});
    assert_eq!(canonical_json(&value).unwrap(),"{\"a\":{\"a\":\"é\",\"b\":true},\"z\":[2,1]}");
}

#[test]
fn fusion_and_field_weight_nonfinite_values_are_rejected_in_manifest_and_provider() {
    for case in 0..6 {
        for provided in [false,true] {
            let mut i=input();let mut config=config();
            let mut fusion=crate::search::FusionConfig::default();
            match case {
                0=>fusion.rrf_k=f64::NAN,1=>fusion.bm25.weight=f64::INFINITY,
                2=>fusion.vector.weight=f64::NEG_INFINITY,3=>fusion.sparse.weight=f64::NAN,
                4=>config.field_weights.push(crate::search::FieldWeight {field:"kind".into(),weights:BTreeMap::new(),default:f64::NAN}),
                _=>config.field_weights.push(crate::search::FieldWeight {field:"kind".into(),weights:BTreeMap::from([("x".into(),f64::INFINITY)]),default:1.0}),
            }
            config.fusion=Some(fusion);
            if provided { i.provider=ProviderFacts::Complete {name:"nonfinite-provider".into(),
                entities:BTreeMap::from([("Provided".into(),config)]),relations:BTreeMap::new()}; }
            else { i.entities.as_mut().unwrap().insert("Document".into(),config); }
            assert!(matches!(DeclaredSchemaFacts::from_complete_input(i),Err(FactsError::Invalid(_))),"case {case}, provider {provided}");
        }
    }
}
