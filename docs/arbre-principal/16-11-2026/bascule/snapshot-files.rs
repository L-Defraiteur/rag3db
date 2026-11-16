// Private immutable Lucivy snapshot reader: filesystem inputs are never opened by a writer.
use std::{collections::BTreeSet,env,fs,path::Path};
use lucivy_core::{query::SchemaConfig,sharded_handle::ShardedHandle,snapshot::{SnapshotIndex,export_snapshot_sharded}};
fn flat_files(path:&Path)->Result<Vec<(String,Vec<u8>)>,String>{
 let mut files=Vec::new();
 for entry in fs::read_dir(path).map_err(|e|e.to_string())?{
  let entry=entry.map_err(|e|e.to_string())?;
  let kind=entry.file_type().map_err(|e|e.to_string())?;
  if !kind.is_file(){return Err("non-regular shard entry".into());}
  let name=entry.file_name().into_string().map_err(|_|"non-UTF8 file")?;
  files.push((name,fs::read(entry.path()).map_err(|e|e.to_string())?));
 }
 files.sort_by(|a,b|a.0.cmp(&b.0));
 if files.is_empty(){return Err("empty shard".into());}
 Ok(files)
}
fn run()->Result<(),String>{
 let args:Vec<_>=env::args_os().collect();
 let list_ids=args.len()==3&&args[2]=="--live-ids";
 if args.len()!=2&&!list_ids{return Err("usage: snapshot-files EXISTING_INDEX_DIRECTORY [--live-ids]".into());}
 let path=Path::new(&args[1]);
 let meta=fs::symlink_metadata(path).map_err(|e|e.to_string())?;
 if !meta.is_dir(){return Err("index directory absent or symlink".into());}
 let config_path=path.join("_shard_config.json");
 if !fs::symlink_metadata(&config_path).map_err(|e|e.to_string())?.is_file(){return Err("config not regular".into());}
 let config=SchemaConfig::from_stored_json(&fs::read(&config_path).map_err(|e|e.to_string())?)?;
 let count=config.shards.unwrap_or(1);
 if !(1..=4096).contains(&count){return Err("invalid shard count".into());}
 let paths:Vec<_>=(0..count).map(|i|format!("shard_{i}")).collect();
 let mut roots=Vec::new();
 for entry in fs::read_dir(path).map_err(|e|e.to_string())?{
  let entry=entry.map_err(|e|e.to_string())?;let kind=entry.file_type().map_err(|e|e.to_string())?;
  let name=entry.file_name().into_string().map_err(|_|"non-UTF8 root")?;
  if kind.is_file(){roots.push((name,fs::read(entry.path()).map_err(|e|e.to_string())?));}
  else if !kind.is_dir()||!paths.contains(&name){return Err("unexpected root entry".into());}
 }
 roots.sort_by(|a,b|a.0.cmp(&b.0));
 let mut shards=Vec::new();
 for name in &paths{
  let shard=path.join(name);
  if !fs::symlink_metadata(&shard).map_err(|e|e.to_string())?.is_dir(){return Err("shard not regular directory".into());}
  shards.push(SnapshotIndex{path:name.as_str(),files:flat_files(&shard)?});
 }
 // Eager full visible inventory: includes obsolete files; this is not a live-only export.
 let blob=export_snapshot_sharded(&shards,&roots);
 let bytes=blob.len();
 let handle=ShardedHandle::open_snapshot(ld_lucivy::directory::OwnedBytes::new(blob))?;
 if !handle.is_read_only(){return Err("snapshot is writable".into());}
 if handle.commit().is_ok(){return Err("snapshot accepted commit".into());}
 let mut ids=BTreeSet::new();
 if list_ids{
  for i in 0..handle.num_shards(){
   let shard=handle.shard(i).ok_or("missing shard")?;
   let searcher=shard.reader.searcher();
   for segment in searcher.segment_readers(){
    let column=segment.fast_fields().u64(lucivy_core::handle::NODE_ID_FIELD).map_err(|e|e.to_string())?;
    let alive=segment.alive_bitset();
    for doc in 0..segment.max_doc(){
     if alive.is_some_and(|bits|!bits.is_alive(doc)){continue;}
     let mut values=column.values_for_doc(doc);
     let id=values.next().ok_or("live document lacks _node_id")?;
     if values.next().is_some(){return Err("live document has multiple _node_id values".into());}
     if !ids.insert(id){return Err("duplicate live _node_id".into());}
    }
   }
  }
  if ids.len() as u64!=handle.num_docs(){return Err("live ID count differs from documents".into());}
 }
 println!("IMMUTABLE_SNAPSHOT\tshards={}\tdocuments={}\tbytes={}\tcommit=REFUSED",handle.num_shards(),handle.num_docs(),bytes);
 if list_ids{for id in ids{println!("LIVE_ID\t{id}");}}
 drop(handle);
 Ok(())
}
fn main(){if let Err(error)=run(){eprintln!("SNAPSHOT_REFUSED\t{error}");std::process::exit(1)}}
