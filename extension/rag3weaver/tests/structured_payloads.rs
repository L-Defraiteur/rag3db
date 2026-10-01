//! Real storage/parameter/filter contract. Run against a disposable rag3daemon.
#![cfg(feature = "daemon")]
use rag3weaver::{catalog::Catalog, config::{CatalogConfig, EntityConfig}, connection::{CypherValue, DbConnection}, daemon::DaemonConnection, embedder::HashEmbedder, filter::{FilterCondition, FilterParser}, json_schema::JsonSchemaMapping, dialect::Rag3dbDialect, search::SearchSignals};
use serde_json::json;
use std::collections::HashMap;

#[test]
#[ignore = "requires RAG3WEAVER_TEST_DAEMON pointing to a disposable rag3daemon"]
fn structured_roundtrip_and_same_element_filter() {
    let address = std::env::var("RAG3WEAVER_TEST_DAEMON").unwrap();
    let conn = DaemonConnection::joindre(&address).unwrap();
    let mut cat = Catalog::new(Box::new(conn), Box::new(HashEmbedder::new(16)), CatalogConfig {allow_mock_embedder:true, embedding_dim:16, ..Default::default()});
    cat.initialize().unwrap();
    let mut mapping = JsonSchemaMapping::from_schema(&json!({"type":"object","properties":{
        "key":{"type":"string"},
        "colors":{"type":"array","items":{"type":"string"}},
        "nums":{"type":"array","items":{"type":"integer"}},
        "abilities":{"type":"array","items":{"type":"object","properties":{
            "label":{"type":"string"}, "level":{"type":"integer"}
        }}}
    }})).unwrap();
    mapping.fields.get_mut("key").unwrap().is_content = true;
    cat.register_entity("NestedProbe", EntityConfig {fields:mapping.fields.clone(), signals:SearchSignals::NONE, hashsafe:Some(vec!["key".into()]), ..Default::default()}).unwrap();
    let samples = [
        json!({"key":"split", "colors":["Blue"],"nums":[],"abilities":[{"label":"Flying","level":1},{"label":"Ward","level":3}]}),
        json!({"key":"same", "colors":["Blue","Black"],"nums":[1,2],"abilities":[{"label":"Flying","level":3}]}),
        json!({"key":"empty", "colors":[],"nums":[],"abilities":[]}),
        json!({"key":"null", "colors":null,"nums":null,"abilities":null}),
        json!({"key":"null_first", "colors":[],"nums":[null,2],"abilities":[{"label":"Flying","level":null},{"label":"Ward","level":3}]})
    ];
    let rows: Vec<_> = samples.iter().map(|r|mapping.record(r).unwrap()).collect();
    let res=cat.ingest_entities("NestedProbe",rows.clone()).unwrap();
    assert_eq!(res.failed,0,"{:?}",res.warnings);
    for row in &rows {
        let uuid=cat.entity_uuid("NestedProbe", row).unwrap();
        let got=cat.get("NestedProbe", &uuid).unwrap().unwrap();
        for (k,v) in row { assert_eq!(got.get(k),Some(v),"field {k} in {:?}", row["key"]); }
    }
    let res=cat.ingest_entities("NestedProbe",rows.clone()).unwrap();
    assert_eq!(res.failed,0); assert_eq!(res.unchanged, rows.len());
    let relations=HashMap::new();
    let mut parser=FilterParser::new(&relations,&Rag3dbDialect);
    let condition: FilterCondition=serde_json::from_value(json!({"nested":{"path":["abilities"],"condition":{"must":[
        {"field":{"key":"label","value":"Flying"}},
        {"field":{"key":"level","value":[{"op":"gte","value":3}]}}
    ]}}})).unwrap();
    let parsed=parser.parse_condition(&condition,"NestedProbe","n").unwrap();
    let conn=DaemonConnection::joindre(&address).unwrap();
    let result=conn.execute_with_params(&format!("MATCH (n:NestedProbe) WHERE {} RETURN n.key", parsed.combine_where()), &parsed.params).unwrap();
    assert_eq!(result.rows,vec![vec![CypherValue::String("same".into())]]);
    // List filters must compare complete values: 'Blue' is not 'Blu'.
    for (color,count) in [("Blue",2),("Blu",0)] {
        let c=serde_json::from_value(json!({"path":{"path":["colors"],"value":[{"op":"has_any","value":[color]}]}})).unwrap();
        let p=parser.parse_condition(&c,"NestedProbe","n").unwrap();
        let result=conn.execute_with_params(&format!("MATCH (n:NestedProbe) WHERE {} RETURN n.key",p.combine_where()), &p.params).unwrap();
        assert_eq!(result.rows.len(),count);
    }
}

#[test]
#[ignore = "requires disposable RAG3WEAVER_TEST_DAEMON and real RAG3WEAVER_TEST_EMBEDDINGS"]
fn mermaid_dag_filters_with_lucivy_and_local_dense() {
    use std::sync::{Arc,Mutex};
    use rag3weaver::{daemon::DaemonEmbedder, dataflow::{GraphTool, builtin_graph_tools, build_definition, NodeTypePolicy, ServiceRegistry, DataflowRuntime, resultat::UnifiedResult}, search_backend::MoteurTexte};
    let address=std::env::var("RAG3WEAVER_TEST_DAEMON").unwrap();
    let embed=DaemonEmbedder::joindre(&std::env::var("RAG3WEAVER_TEST_EMBEDDINGS").unwrap()).unwrap();
    assert!(!embed.identite().factice);
    let dim=embed.identite().dim;
    eprintln!("Real dense model: {} ({dim})",embed.identite().modele);
    let mut cat=Catalog::new(Box::new(DaemonConnection::joindre(&address).unwrap()),Box::new(embed),CatalogConfig {embedding_dim:dim,..Default::default()});
    cat.set_moteur_texte(MoteurTexte::Lucivy);
    let extension=std::env::var("RAG3WEAVER_TEST_VECTOR_EXTENSION").unwrap();
    cat.execute_raw(&format!("LOAD EXTENSION '{}'",extension.replace('\'',"''"))).unwrap();
    cat.initialize().unwrap();
    let mut mapping=JsonSchemaMapping::from_schema(&json!({"type":"object","properties":{
        "key":{"type":"string"},"text":{"type":"string"},"colors":{"type":"array","items":{"type":"string"}},
        "abilities":{"type":"array","items":{"type":"object","properties":{"label":{"type":"string"},"level":{"type":"integer"}}}}
    }})).unwrap();
    mapping.fields.get_mut("key").unwrap().is_title=true;
    mapping.fields.get_mut("text").unwrap().is_content=true;
    cat.register_entity("DenseProbe",EntityConfig {fields:mapping.fields.clone(),hashsafe:Some(vec!["key".into()]),return_fields:Some(vec!["key".into(),"colors".into(),"abilities".into()]),..Default::default()}).unwrap();
    let rows:Vec<_>=[
        json!({"key":"split", "text":"Flying creature with ward. Créature volante protégée.", "colors":["Blue"],"abilities":[{"label":"Flying","level":1},{"label":"Ward","level":3}]}),
        json!({"key":"same", "text":"Flying creature. Créature avec le vol.", "colors":["Blue","Black"],"abilities":[{"label":"Flying","level":3}]}),
        json!({"key":"empty", "text":"Flying creature. Créature volante.", "colors":[],"abilities":[]})
    ].iter().map(|r|mapping.record(r).unwrap()).collect();
    let expected=cat.entity_uuid("DenseProbe",&rows[1]).unwrap();
    let report=cat.ingest_entities("DenseProbe",rows).unwrap();
    assert_eq!(report.failed,0,"{:?}",report.warnings);
    let cat=Arc::new(Mutex::new(cat));
    let (nodes,_)=builtin_graph_tools().unwrap();
    let template=GraphTool::from_mermaid(include_str!("../templates/tools/search_structured.mmd")).unwrap().bind(&nodes).unwrap();
    let filter=json!({"nested":{"path":["abilities"],"condition":{"must":[
        {"field":{"key":"label","value":"Flying"}},{"field":{"key":"level","value":[{"op":"gte","value":3}]}}
    ]}}});
    for signals in [json!(["bm25"]),json!(["vector"]),json!(["bm25","vector"])] {
        let def=template.instantiate(&json!({"target":"DenseProbe","query":"Flying","options":{"signals":signals,"limit":1,"filter_condition":filter}})).unwrap();
        let mut graph=build_definition(&def,&nodes,&NodeTypePolicy::All).unwrap();
        let mut services=ServiceRegistry::new();
        cat.lock().unwrap().register_search_services(&mut services);
        services.register("catalog",cat.clone());
        let output=DataflowRuntime::with_services(100,services).execute(&mut graph).unwrap();
        let results=output.get("render","results").unwrap().downcast::<Vec<UnifiedResult>>().unwrap();
        assert_eq!(results.len(),1,"{signals}: {results:?}");
        assert_eq!(results[0].uuid,expected,"{signals}");
        assert!(matches!(&results[0].data.as_ref().unwrap()["abilities"],CypherValue::List(_)));
    }
    // Bad paths must fail before a signal can return unrestricted results.
    let bad=template.instantiate(&json!({"target":"DenseProbe","query":"Flying","options":{"filter_condition":{"path":{"path":["abilities","level"],"value":3}}}})).unwrap();
    let mut graph=build_definition(&bad,&nodes,&NodeTypePolicy::All).unwrap();
    let mut services=ServiceRegistry::new();cat.lock().unwrap().register_search_services(&mut services);services.register("catalog",cat);
    assert!(DataflowRuntime::with_services(100,services).execute(&mut graph).is_err());
}

#[test]
#[ignore = "requires disposable daemon and RAG3WEAVER_TEST_MTGA snapshot directory"]
fn composed_magic_snapshot() {
    use std::{collections::{BTreeMap,HashSet}, sync::{Arc,Mutex}};
    use rag3weaver::dataflow::{GraphTool,builtin_graph_tools,build_definition,NodeTypePolicy,ServiceRegistry,DataflowRuntime,resultat::UnifiedResult};
    let root=std::path::PathBuf::from(std::env::var("RAG3WEAVER_TEST_MTGA").unwrap());
    let cards:Vec<serde_json::Value>=std::fs::read_to_string(root.join("cards.jsonl")).unwrap().lines().map(|s|serde_json::from_str(s).unwrap()).collect();
    let decks:Vec<serde_json::Value>=serde_json::from_str(&std::fs::read_to_string(root.join("decks.json")).unwrap()).unwrap();
    let deck=decks.iter().find(|d|d["name"]=="mycotyrant_insidious TRIM (2)").unwrap();
    let entries=deck["piles"]["MainDeck"].as_array().unwrap();
    let wanted:HashSet<i64>=entries.iter().map(|e|e["cardId"].as_i64().unwrap()).collect();
    let glossary:Vec<serde_json::Value>=serde_json::from_str(&std::fs::read_to_string(root.join("mechanics.json")).unwrap()).unwrap();
    let address=std::env::var("RAG3WEAVER_TEST_DAEMON").unwrap();
    let mut cat=Catalog::new(Box::new(DaemonConnection::joindre(&address).unwrap()),Box::new(HashEmbedder::new(16)),CatalogConfig{allow_mock_embedder:true,embedding_dim:16,..Default::default()});
    cat.initialize().unwrap();
    let mapping=JsonSchemaMapping::from_schema(&json!({"type":"object","properties":{"key":{"type":"string"},"text":{"type":"string"},"quantity":{"type":"integer"},"deck_id":{"type":"string"}}})).unwrap();
    for entity in ["MagicCard","MagicAbility","MagicMechanic","MagicEntry"] {
        let mut fields=mapping.fields.clone();fields.get_mut("text").unwrap().is_content=true;
        cat.register_entity(entity,EntityConfig{fields,hashsafe:Some(vec!["key".into()]),signals:SearchSignals::NONE,..Default::default()}).unwrap();
    }
    for (name,from,to) in [("MAGIC_HAS_ABILITY","MagicCard","MagicAbility"),("MAGIC_HAS_MECHANIC","MagicAbility","MagicMechanic"),("MAGIC_ENTRY_CARD","MagicEntry","MagicCard")] {cat.register_relation(name,from,to).unwrap();}
    let mut ids:HashMap<(String,String),String>=HashMap::new();
    let mut insert=|cat:&mut Catalog,entity:&str,key:String,text:String,quantity:i64,deck_id:&str| {
        if let Some(id)=ids.get(&(entity.into(),key.clone())) {return id.clone();}
        let row=mapping.record(&json!({"key":key,"text":text,"quantity":quantity,"deck_id":deck_id})).unwrap();
        let id=cat.entity_uuid(entity,&row).unwrap();
        let report=cat.ingest_entities(entity,vec![row]).unwrap();assert_eq!(report.failed,0,"{:?}",report.warnings);
        ids.insert((entity.into(),key),id.clone());id
    };
    let mut mechanic_ids=HashMap::new();let mut mechanic_texts=HashMap::new();
    for m in &glossary {
        let text=format!("{}\n{}",m["definition_en"].as_str().unwrap_or(""),m["definition_fr"].as_str().unwrap_or(""));
        let key=m["key"].as_str().unwrap();
        let Some(external)=cards.iter().flat_map(|c|c["mechanics"].as_array().unwrap()).find(|m|m["key"]==key).and_then(|m|m["mechanic_id"].as_str()).map(str::to_owned) else {continue;};
        let id=insert(&mut cat,"MagicMechanic",key.into(),text.clone(),0,"");
        mechanic_ids.insert(external.clone(),id);mechanic_texts.insert(external,text);
    }
    let mut expected_collection=HashSet::new();
    let mut expected=HashSet::new();let mut in_scope=HashSet::new();let mut rites=None;
    // Include a card outside the deck to verify scope exclusion before pagination.
    for c in cards.iter().filter(|c|wanted.contains(&c["arena_id"].as_i64().unwrap()) || c["arena_id"]==78541) {
        let key=c["arena_id"].to_string();
        let text=format!("{}\n{}",c["text_en"].as_str().unwrap(),c["text_fr"].as_str().unwrap());
        let cid=insert(&mut cat,"MagicCard",key.clone(),text.clone(),0,"");
        let mut matches=text.contains("graveyard");
        for a in c["abilities"].as_array().unwrap() {
            let text=format!("{}\n{}",a["text_en"].as_str().unwrap(),a["text_fr"].as_str().unwrap());
            matches |= text.contains("graveyard");
            let aid=insert(&mut cat,"MagicAbility",format!("{}:{}",a["ability_id"],a["text_id"]),text,0,"");
            cat.link("MAGIC_HAS_ABILITY",cid.clone(),aid.clone(),BTreeMap::new()).unwrap();
            for gid in a["glossary_ids"].as_array().unwrap() {
                let gid=gid.as_str().unwrap();
                matches |= mechanic_texts[gid].contains("graveyard");
                cat.link("MAGIC_HAS_MECHANIC",aid.clone(),mechanic_ids[gid].clone(),BTreeMap::new()).unwrap();
            }
        }
        let owned=c["owned"].as_i64().unwrap_or(0);
        let holding=insert(&mut cat,"MagicEntry",format!("holding:{key}"),key.clone(),owned,"collection");
        cat.link("MAGIC_ENTRY_CARD",holding,cid.clone(),BTreeMap::new()).unwrap();
        if matches && owned>0 {expected_collection.insert(cid.clone());}
        if wanted.contains(&c["arena_id"].as_i64().unwrap()) {
            in_scope.insert(cid.clone());if matches {expected.insert(cid.clone());}
            let quantity=entries.iter().find(|e|e["cardId"]==c["arena_id"]).unwrap()["quantity"].as_i64().unwrap();
            let entry=insert(&mut cat,"MagicEntry",key.clone(),key,quantity,"current");
            cat.link("MAGIC_ENTRY_CARD",entry,cid.clone(),BTreeMap::new()).unwrap();
        }
        if c["arena_id"]==86649 {rites=Some(cid);}
    }
    let cat=Arc::new(Mutex::new(cat));let (nodes,_)=builtin_graph_tools().unwrap();
    let tool=GraphTool::from_mermaid(include_str!("../templates/tools/search_related_scoped.mmd")).unwrap().bind(&nodes).unwrap();
    let contains=json!({"field":{"key":"text","value":[{"op":"contains","value":"graveyard"}]}});
    let args=json!({"mechanic_entity":"MagicMechanic","ability_entity":"MagicAbility","card_entity":"MagicCard","scope_entity":"MagicEntry","mechanic_filter":contains,"ability_filter":contains,"card_filter":contains,"scope_filter":{"field":{"key":"deck_id","value":"current"}},"ability_mechanic_relation":"MAGIC_HAS_MECHANIC","card_ability_relation":"MAGIC_HAS_ABILITY","scope_card_relation":"MAGIC_ENTRY_CARD","options":{"limit":500},"duplicates":"merge"});
    let run=|args:&serde_json::Value| {
        let def=tool.instantiate(args).unwrap();let mut graph=build_definition(&def,&nodes,&NodeTypePolicy::All).unwrap();
        let mut services=ServiceRegistry::new();cat.lock().unwrap().register_search_services(&mut services);services.register("catalog",cat.clone());
        let runtime=DataflowRuntime::with_services(100,services);
        let out=runtime.execute(&mut graph).unwrap();
        out.get("page","results").unwrap().downcast::<Vec<UnifiedResult>>().unwrap().clone()
    };
    let merged=run(&args);assert!(!expected.is_empty());
    let actual:HashSet<_>=merged.iter().map(|r|r.uuid.clone()).collect();
    for missing in expected.difference(&actual) {eprintln!("MISSING {:?}",ids.iter().find(|(_,v)|*v==missing));}
    assert_eq!(actual,expected);
    assert_eq!(merged.len(),expected.len());
    let rites=merged.iter().find(|r|Some(&r.uuid)==rites.as_ref()).unwrap();
    assert!(serde_json::to_string(rites).unwrap().contains("Flashback"),"native mechanic proof missing");
    let mut keep=args.clone();keep["duplicates"]=json!("keep");let kept=run(&keep);
    assert!(kept.len()>merged.len());assert!(kept.iter().all(|r|in_scope.contains(&r.uuid)));
    let mut page=args.clone();page["options"]=json!({"limit":1,"offset":1});
    assert_eq!(run(&page)[0].uuid,merged[1].uuid);
    let mut collection=args.clone();collection["scope_filter"]=json!({"must":[{"field":{"key":"deck_id","value":"collection"}},{"field":{"key":"quantity","value":[{"op":"gt","value":0}]}}]});
    let owned=run(&collection);
    assert_eq!(owned.iter().map(|r|r.uuid.clone()).collect::<HashSet<_>>(),expected_collection);
    assert!(owned.iter().any(|r|r.data.as_ref().unwrap()["key"].as_str()==Some("78541")),"owned Dryad must survive collection scope but not deck scope");
    assert!(owned.iter().all(|r|r.other_children.as_ref().is_some_and(|v|!v.is_empty())));
    let mut absent=args.clone();absent["scope_filter"]=json!({"field":{"key":"deck_id","value":"absent"}});
    assert!(run(&absent).is_empty());
    if let Ok(path)=std::env::var("RAG3WEAVER_TEST_REPORT") {
        let report=json!({"deck_id":deck["id"],"deck_name":deck["name"],"distinct_cards":wanted.len(),"merged_count":merged.len(),"kept_count":kept.len(),"owned_matches_in_test_corpus":owned.len(),"query":"graveyard","engine":"rag3db exact predicates + Mermaid traversal/fusion; no dense in this exhaustive query","results":merged});
        std::fs::write(path,serde_json::to_string_pretty(&report).unwrap()).unwrap();
    }
    eprintln!("Actual deck: {} distinct cards, {} merged matches, {} kept occurrences; native Flashback evidence retained",wanted.len(),merged.len(),kept.len());
}
