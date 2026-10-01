//! Single catalogue owner. JSON lines on stdio; diagnostics remain on stderr.
use std::{collections::BTreeMap, io::{self, BufRead, Write}, sync::{Arc, Mutex}};
use rag3weaver::{catalog::Catalog, config::{CatalogConfig, EntityConfig}, connection::CypherValue,
    daemon::{DaemonConnection, DaemonEmbedder}, search_backend::MoteurTexte,
    json_schema::JsonSchemaMapping, dataflow::{GraphTool, NodeTypePolicy, build_definition, builtin_graph_tools, DataflowRuntime, ServiceRegistry}};
use serde_json::{Value, json};
type Err = Box<dyn std::error::Error>;
fn string<'a>(v: &'a Value,k:&str)->Result<&'a str,Err>{v[k].as_str().ok_or_else(||format!("missing {k}").into())}
fn summary(r:rag3weaver::records::FlushResult)->Result<Value,Err>{
    if r.failed>0 {return Err(format!("{} failures: {:?}",r.failed,r.warnings).into());}
    Ok(json!({"processed":r.processed,"unchanged":r.unchanged,"warnings":r.warnings,"ready":r.rendu_pret}))
}
fn handle(v:Value, state:&mut Option<Arc<Mutex<Catalog>>>)->Result<Value,Err>{
    let op=string(&v,"op")?;
    if op=="map_schema" {
        let mapping=JsonSchemaMapping::from_schema(&v["schema"])?;
        return Ok(serde_json::to_value(mapping)?);
    }
    if op=="validate_template" {
        let (nodes,_)=builtin_graph_tools()?;
        let template=GraphTool::from_mermaid(string(&v,"template")?)?.bind(&nodes)?;
        let def=template.instantiate(&v["args"])?;
        build_definition(&def,&nodes,&NodeTypePolicy::only(["SearchSourceNode","BM25SearchNode","VectorSearchNode","FuseResultsNode","PaginateNode","ResolveParentNode","RenderResultsNode"]))?;
        return Ok(serde_json::to_value(def)?);
    }
    if op=="init" {
        if state.is_some(){return Err("already initialized".into());}
        let embed=DaemonEmbedder::joindre(string(&v,"embeddings")?)?;
        let identity=serde_json::to_value(embed.identite())?;
        if embed.identite().factice {return Err("refusing mock embeddings".into());}
        let conn=DaemonConnection::joindre(string(&v,"database")?)?;
        let mut cat=Catalog::new(Box::new(conn),Box::new(embed),CatalogConfig{embedding_dim:identity["dim"].as_u64().unwrap() as usize, checkpoint_dir:Some(string(&v,"checkpoints")?.into()), ..Default::default()});
        cat.set_moteur_texte(MoteurTexte::Lucivy);
        if let Some(path)=v["vector_extension"].as_str(){cat.execute_raw(&format!("LOAD EXTENSION '{}'",path.replace('\'',"''")))?;}
        cat.initialize()?;
        for (name,config) in v["entities"].as_object().ok_or("entities object missing")? {
            let config:EntityConfig=serde_json::from_value(config.clone())?;
            cat.register_entity(name,config)?;
        }
        for r in v["relations"].as_array().ok_or("relations missing")? {cat.register_relation(string(r,"name")?,string(r,"from")?,string(r,"to")?)?;}
        cat.regime_d_ecriture(rag3weaver::disponibilite::RegimeEcriture::ParLot);
        *state=Some(Arc::new(Mutex::new(cat)));
        return Ok(json!({"embeddings":identity,"database":"rag3db","fts":"lucivy"}));
    }
    let cat=state.as_ref().ok_or("not initialized")?;
    if op=="search" {
        let options=serde_json::from_value(v.get("options").cloned().unwrap_or(json!({})))?;
        return Ok(serde_json::to_value(Catalog::rechercher(cat,string(&v,"entity")?,string(&v,"query")?,options)?)?);
    }
    if op=="compose" {
        let (nodes,_)=builtin_graph_tools()?;
        let def=serde_json::from_value(v["graph"].clone())?;
        let policy=NodeTypePolicy::only(["SearchSourceNode","BM25SearchNode","VectorSearchNode","FuseResultsNode","PaginateNode","ResolveParentNode","RenderResultsNode","FetchRelatedNode","ComposeNode"]);
        let mut graph=build_definition(&def,&nodes,&policy)?;
        let mut services=ServiceRegistry::new();
        cat.lock().unwrap().register_search_services(&mut services);
        services.register("catalog",cat.clone());
        let output=DataflowRuntime::with_services(100,services).execute(&mut graph)?;
        let mut results=serde_json::Map::new();
        for port in v["outputs"].as_array().ok_or("outputs missing")? {
            let node=string(port,"node")?;let name=string(port,"port")?;
            let value=output.get(node,name).ok_or("missing output port")?;
            let cp=rag3weaver::dataflow::checkpoint::port_value_to_checkpoint(value)?;
            results.insert(format!("{node}.{name}"),match cp.data_json {Some(s)=>serde_json::from_str(&s)?,None=>Value::Null});
        }
        return Ok(Value::Object(results));
    }
    let mut cat=cat.lock().unwrap();
    match op {
        "ingest"=>{
            let name=string(&v,"entity")?;
            let rows:Vec<BTreeMap<String,CypherValue>>=serde_json::from_value(v["records"].clone())?;
            let ids=rows.iter().map(|r|cat.entity_uuid(name,r)).collect::<Result<Vec<_>,_>>()?;
            let report=summary(cat.ingest_entities(name,rows)?)?;
            Ok(json!({"ids":ids,"report":report}))
        }
        "inventory"=>{
            let entity=string(&v,"entity")?;
            if !cat.entity_configs().contains_key(entity){return Err("unknown entity".into());}
            let q=cat.execute_raw(&format!("MATCH (n:{entity}) RETURN n.external_id,n._uuid,n.source_hash"))?;
            Ok(serde_json::to_value(q.rows)?)
        }
        "get"=>Ok(serde_json::to_value(cat.get(string(&v,"entity")?,string(&v,"uuid")?)?)?),
        "delete"=>{
            for id in v["ids"].as_array().ok_or("ids missing")? {cat.delete(string(&v,"entity")?,id.as_str().ok_or("id must be string")?)?;}
            summary(cat.drain())
        }
        "links"=>{
            for link in v["links"].as_array().ok_or("links missing")? {
                cat.link(string(link,"relation")?,string(link,"from")?.to_string(),string(link,"to")?.to_string(),BTreeMap::new())?;
            }
            summary(cat.drain())
        }
        _=>Err(format!("unknown op {op}").into()),
    }
}
fn main(){
    let mut state=None;
    for line in io::stdin().lock().lines(){
        let result=line.map_err(|e|e.to_string()).and_then(|l|serde_json::from_str::<Value>(&l).map_err(|e|e.to_string())).and_then(|v|handle(v,&mut state).map_err(|e|e.to_string()));
        let response=match result{Ok(v)=>json!({"ok":true,"result":v}),Err(e)=>json!({"ok":false,"error":e})};
        println!("{response}");let _=io::stdout().flush();
    }
}
