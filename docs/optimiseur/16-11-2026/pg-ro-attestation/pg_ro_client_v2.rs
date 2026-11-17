//! V2 SOURCE ONLY. No build/runtime/PG qualification implied by this candidate.
//! Credentials stdin memory only; generic errors, no arbitrary SQL/debug/env.
#![forbid(unsafe_code)]
use std::fmt;
use std::io::{self, Read, Write};
use std::net::{IpAddr, Ipv4Addr};
use std::time::Duration;
use serde_core::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Value, json};
use tokio::time::{timeout_at, Instant};
use tokio::task::JoinHandle;
use tokio_postgres::{Client, Config, CancelToken, Error, NoTls, SimpleQueryMessage, config::{Host, SslMode}};

const IDENTITY: &str=include_str!("identity.sql");
const SESSION: &str=include_str!("readonly-session-v2.sql");
const POSTCHECK: &str=include_str!("backend-postcheck.sql");
const OPTIONS: &str="-c default_transaction_read_only=on -c statement_timeout=1000 -c lock_timeout=500";

// Library deserializer with duplicate-key rejection at every object depth.
struct Unique(Value);
impl<'de> Deserialize<'de> for Unique {
    fn deserialize<D: Deserializer<'de>>(d:D)->Result<Self,D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value=Value;
            fn expecting(&self,f:&mut fmt::Formatter)->fmt::Result{f.write_str("unique-key JSON")}
            fn visit_unit<E:de::Error>(self)->Result<Value,E>{Ok(Value::Null)}
            fn visit_none<E:de::Error>(self)->Result<Value,E>{Ok(Value::Null)}
            fn visit_bool<E:de::Error>(self,v:bool)->Result<Value,E>{Ok(Value::Bool(v))}
            fn visit_i64<E:de::Error>(self,v:i64)->Result<Value,E>{Ok(v.into())}
            fn visit_u64<E:de::Error>(self,v:u64)->Result<Value,E>{Ok(v.into())}
            fn visit_f64<E:de::Error>(self,v:f64)->Result<Value,E>{serde_json::Number::from_f64(v).map(Value::Number).ok_or_else(||E::custom("nonfinite"))}
            fn visit_str<E:de::Error>(self,v:&str)->Result<Value,E>{Ok(Value::String(v.to_owned()))}
            fn visit_string<E:de::Error>(self,v:String)->Result<Value,E>{Ok(Value::String(v))}
            fn visit_seq<A:SeqAccess<'de>>(self,mut a:A)->Result<Value,A::Error>{
                let mut out=Vec::new();while let Some(Unique(v))=a.next_element::<Unique>()?{out.push(v);}Ok(Value::Array(out))
            }
            fn visit_map<A:MapAccess<'de>>(self,mut a:A)->Result<Value,A::Error>{
                let mut out=Map::new();while let Some(k)=a.next_key::<String>()?{
                    if out.contains_key(&k){return Err(de::Error::custom("duplicate key"));}
                    let Unique(v)=a.next_value::<Unique>()?;out.insert(k,v);
                }Ok(Value::Object(out))
            }
        }
        d.deserialize_any(V).map(Unique)
    }
}
fn parse(raw:&[u8])->Result<Value,&'static str>{
    let mut d=serde_json::Deserializer::from_slice(raw);
    let Unique(v)=Unique::deserialize(&mut d).map_err(|_|"protocol")?;
    d.end().map_err(|_|"protocol")?;Ok(v)
}
fn emit(v:Value)->Result<(),&'static str>{
    let text=serde_json::to_string(&v).map_err(|_|"protocol")?;
    if text.len()>65536{return Err("protocol");}
    println!("{text}");io::stdout().flush().map_err(|_|"protocol")
}
fn name(s:&str)->bool{!s.is_empty()&&s.len()<=63&&s.bytes().enumerate().all(|(i,c)|c==b'_'||c.is_ascii_lowercase()||(i>0&&c.is_ascii_digit()))}
struct Prepared{config:Config,nonce:String,budget:u64,user:String,database:String}
fn prepare(v:&Value)->Result<Prepared,&'static str>{
    let o=v.as_object().ok_or("protocol")?;
    if o.len()!=4||!["dsn","nonce","target","budget_ms"].iter().all(|k|o.contains_key(*k)){return Err("protocol");}
    let t=v["target"].as_object().ok_or("protocol")?;
    if t.len()!=4||!["host","port","database","user"].iter().all(|k|t.contains_key(*k)){return Err("protocol");}
    let nonce=v["nonce"].as_str().ok_or("protocol")?;
    let dsn=v["dsn"].as_str().ok_or("protocol")?;
    let budget=v["budget_ms"].as_u64().filter(|x|*x>0&&*x<=8000).ok_or("protocol")?;
    let port=v["target"]["port"].as_u64().filter(|x|*x>0&&*x<=65535&&*x!=5433).ok_or("protocol")? as u16;
    let user=v["target"]["user"].as_str().filter(|x|name(x)).ok_or("protocol")?;
    let database=v["target"]["database"].as_str().filter(|x|name(x)&&x.starts_with("g_pg_probe_")).ok_or("protocol")?;
    if v["target"]["host"]!="127.0.0.1"||nonce.len()!=32||!nonce.bytes().all(|c|c.is_ascii_hexdigit()&&!c.is_ascii_uppercase())||dsn.len()>4096{return Err("protocol");}
    let mut config:Config=dsn.parse().map_err(|_|"protocol")?;
    if config.get_user()!=Some(user)||config.get_dbname()!=Some(database)||config.get_ports()!=[port]||config.get_hosts().len()!=1||!config.get_hostaddrs().is_empty()||config.get_options().is_some()||config.get_ssl_mode()!=SslMode::Disable{return Err("protocol");}
    if !matches!(&config.get_hosts()[0],Host::Tcp(h) if h=="127.0.0.1"){return Err("protocol");}
    config.hostaddr(IpAddr::V4(Ipv4Addr::LOCALHOST)).connect_timeout(Duration::from_millis(1200)).options(OPTIONS);
    Ok(Prepared{config,nonce:nonce.to_owned(),budget,user:user.to_owned(),database:database.to_owned()})
}
async fn row(client:&Client,sql:&'static str,end:Instant)->Result<Value,&'static str>{
    let messages=timeout_at(end,client.simple_query(sql)).await.map_err(|_|"timeout")?.map_err(|_|"query")?;
    let mut value=None;let mut commands=0;
    for m in messages{match m{
        SimpleQueryMessage::Row(r)=>{if value.is_some()||r.columns().len()!=1{return Err("protocol");}let s=r.try_get(0).map_err(|_|"protocol")?.ok_or("protocol")?;if s.len()>65536{return Err("protocol");}value=Some(parse(s.as_bytes())?);},
        SimpleQueryMessage::CommandComplete(n)=>{if n!=1{return Err("protocol");}commands+=1;},
        SimpleQueryMessage::RowDescription(c)=>{if c.len()!=1{return Err("protocol");}},
        _=>return Err("protocol"),
    }}
    if commands!=1{return Err("protocol");}value.ok_or("protocol")
}
struct Owner{client:Option<Client>,driver:JoinHandle<Result<(),Error>>,cancel:CancelToken,pid:Option<u64>}
impl Owner{
    async fn open(config:Config,end:Instant)->Result<Self,&'static str>{
        let (client,connection)=timeout_at(end,config.connect(NoTls)).await.map_err(|_|"connect_before_attribution")?.map_err(|_|"connect_before_attribution")?;
        let cancel=client.cancel_token();let driver=tokio::spawn(connection);
        Ok(Self{client:Some(client),driver,cancel,pid:None})
    }
    fn client(&self)->Result<&Client,&'static str>{self.client.as_ref().ok_or("protocol")}
    async fn transaction(&mut self,sql:&'static str,user:&str,database:&str,session:&str,end:Instant)->Result<Value,&'static str>{
        timeout_at(end,self.client()?.batch_execute("BEGIN READ ONLY")).await.map_err(|_|"timeout")?.map_err(|_|"query")?;
        let state=row(self.client()?,SESSION,end).await?;
        self.pid=state["backend_pid"].as_u64().filter(|n|*n>0&&*n<=i32::MAX as u64);
        if self.pid.is_none()||state.as_object().map(|x|x.len())!=Some(11)||state["user"]!=user||state["database"]!=database{return Err("readonly");}
        for k in ["transaction_read_only","default_transaction_read_only"]{if state[k].as_bool()!=Some(true){return Err("readonly");}}
        for k in ["superuser","createdb","createrole","replication","bypassrls","database_owner"]{if state[k].as_bool()!=Some(false){return Err("readonly");}}
        emit(json!({"event":"readonly","session":session,"state":state}))?;
        let value=row(self.client()?,sql,end).await?;
        timeout_at(end,self.client()?.batch_execute("COMMIT")).await.map_err(|_|"timeout")?.map_err(|_|"query")?;
        Ok(value)
    }
 }
trait SessionClose {
    fn token_acquired(&self)->bool;
    fn backend_pid(&self)->Option<u64>;
    async fn cancel_query(&mut self,end:Instant)->bool;
    fn drop_client(&mut self);
    async fn join_driver(&mut self,end:Instant)->bool;
}
impl SessionClose for Owner {
    fn token_acquired(&self)->bool{true}
    fn backend_pid(&self)->Option<u64>{self.pid}
    async fn cancel_query(&mut self,end:Instant)->bool{matches!(timeout_at(end,self.cancel.cancel_query(NoTls)).await,Ok(Ok(())))}
    fn drop_client(&mut self){drop(self.client.take());}
    async fn join_driver(&mut self,end:Instant)->bool{
        let joined=matches!(timeout_at(end,&mut self.driver).await,Ok(Ok(Ok(()))));
        if !joined{self.driver.abort();let _=timeout_at(end,&mut self.driver).await;}
        joined
    }
}
async fn finish_owned<C:SessionClose>(owner:&mut C,failed:bool,end:Instant)->Value{
    let token=owner.token_acquired();let mut attempted=false;let mut cancelled=false;
    if failed&&token&&Instant::now()<end{attempted=true;cancelled=owner.cancel_query(end).await;}
    owner.drop_client();let joined=owner.join_driver(end).await;
    json!({"cancel_token_acquired":token,"cancel_attempted":attempted,"cancel_complete":cancelled,
           "client_dropped":true,"driver_joined":joined,"backend_pid":owner.backend_pid(),"backend_absent":null})
}

fn no_owner()->Value{json!({"cancel_token_acquired":false,"cancel_attempted":false,"cancel_complete":false,"client_dropped":false,"driver_joined":false,"backend_pid":null,"backend_absent":null})}
async fn session(config:Config,sql:&'static str,p:&Prepared,kind:&str,end:Instant)->Result<(Value,Value),&'static str>{
    let mut owner=match Owner::open(config,end).await{Ok(v)=>v,Err(code)=>{emit(json!({"event":"failure","session":kind,"code":code,"cleanup":no_owner()}))?;return Err(code);}};
    let result=async {
        emit(json!({"event":"connected","session":kind,"cancel_token_acquired":true}))?;
        owner.transaction(sql,&p.user,&p.database,kind,end).await
    }.await;
    let cleanup=finish_owned(&mut owner,result.is_err(),end).await;
    match result{
        Ok(value) if cleanup["driver_joined"].as_bool()==Some(true)=>Ok((value,cleanup)),
        Ok(_)=>{emit(json!({"event":"failure","session":kind,"code":"driver_join","cleanup":cleanup}))?;Err("driver_join")},
        Err(code)=>{emit(json!({"event":"failure","session":kind,"code":code,"cleanup":cleanup}))?;Err(code)},
    }
}
async fn capture(p:Prepared)->Result<(),&'static str>{
    let end=Instant::now()+Duration::from_millis(p.budget);let application=format!("g-pg-{}",p.nonce);
    let mut config=p.config.clone();config.application_name(application.clone());
    let (identity,_)=session(config.clone(),IDENTITY,&p,"identity",end).await?;
    let pid=identity["backend_pid"].as_u64().filter(|x|*x>0&&*x<=i32::MAX as u64).ok_or("protocol")?;
    if identity.as_object().map(|x|x.len())!=Some(13)||identity["application_name"]!=application{return Err("protocol");}
    emit(json!({"event":"identity","row":identity}))?;
    emit(json!({"event":"closed","session":"identity","driver_joined":true}))?;
    config.application_name(format!("{application}:check")).options(format!("{OPTIONS} -c g_pg.expected_backend_pid={pid} -c g_pg.expected_application_name={application}"));
    let (check,_)=session(config,POSTCHECK,&p,"postcheck",end).await?;
    emit(json!({"event":"postcheck","row":check}))?;
    emit(json!({"event":"closed","session":"postcheck","driver_joined":true}))?;
    if check["backend_absent"].as_bool()!=Some(true)||Instant::now()>=end{return Err("postcheck");}
    emit(json!({"event":"complete","transport_qualified":false}))
}
fn main(){
    let args=std::env::args().skip(1).collect::<Vec<_>>();
    let validate=args==["--validate-request-v2"];
    if !validate&&args!=["--capture-readonly-v2"]{eprintln!("PREPARATION_V2 explicit mode required");std::process::exit(2);}
    let result=(||->Result<(),&'static str>{
        let mut bytes=Vec::new();io::stdin().take(8193).read_to_end(&mut bytes).map_err(|_|"protocol")?;
        if bytes.len()>8192{return Err("protocol");}let value=parse(&bytes)?;let p=prepare(&value)?;
        if validate{return emit(json!({"request_valid":true,"pg_contact":false,"schema_version":2}));}
        let runtime=tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|_|"protocol")?;
        runtime.block_on(capture(p))
    })();
    if let Err(code)=result{eprintln!("PG_V2_REFUSED:{code}");std::process::exit(1);}
}


#[cfg(test)]
mod tests {
    use super::*;
    fn request(port:u16)->Value{json!({"dsn":format!("host=127.0.0.1 port={port} user=g_pg_observer dbname=g_pg_probe_fake password=RUST_FAKE_CANARY sslmode=disable"),"nonce":"a".repeat(32),"budget_ms":1000,"target":{"host":"127.0.0.1","port":port,"database":"g_pg_probe_fake","user":"g_pg_observer"}})}
    #[test] fn duplicate_json_is_rejected_without_network(){assert!(parse(br#"{"nonce":"a","nonce":"b"}"#).is_err());}
    #[test] fn private_fixture_is_syntax_only_until_capture(){assert!(prepare(&request(15433)).is_ok());}
    #[test] fn shared_bootstrap_port_is_never_fallback(){assert!(prepare(&request(5433)).is_err());}
    #[test] fn parse_error_never_contains_secret(){let mut value=request(15433);value["nonce"]=json!("bad");let reason=prepare(&value).err().unwrap();assert!(!reason.contains("RUST_FAKE_CANARY"));}
    struct FakeClose{token:bool,pid:Option<u64>,joined:bool,calls:Vec<&'static str>,deadlines:Vec<Instant>}
    impl SessionClose for FakeClose{
        fn token_acquired(&self)->bool{self.token}
        fn backend_pid(&self)->Option<u64>{self.pid}
        async fn cancel_query(&mut self,end:Instant)->bool{self.calls.push("cancel");self.deadlines.push(end);true}
        fn drop_client(&mut self){self.calls.push("drop");}
        async fn join_driver(&mut self,end:Instant)->bool{self.calls.push("join");self.deadlines.push(end);self.joined}
    }
    fn fake(token:bool)->FakeClose{FakeClose{token,pid:None,joined:true,calls:vec![],deadlines:vec![]}}
    fn run_close(owner:&mut FakeClose,end:Instant)->Value{tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(finish_owned(owner,true,end))}
    #[test] fn no_token_never_attempts_cancel(){let mut o=fake(false);let v=run_close(&mut o,Instant::now()+Duration::from_secs(1));assert_eq!(o.calls,vec!["drop","join"]);assert_eq!(v["cancel_attempted"],false);}
    #[test] fn first_failure_cleanup_uses_cancel_drop_join_and_same_end(){let mut o=fake(true);let end=Instant::now()+Duration::from_secs(1);run_close(&mut o,end);assert_eq!(o.calls,vec!["cancel","drop","join"]);assert!(o.deadlines.iter().all(|x|*x==end));}
    #[test] fn expired_end_is_never_renewed_for_cancel(){let mut o=fake(true);let end=Instant::now()-Duration::from_secs(1);run_close(&mut o,end);assert_eq!(o.calls,vec!["drop","join"]);assert!(o.deadlines.iter().all(|x|*x==end));}
    #[test] fn unknown_pid_or_unjoined_driver_is_not_absence_proof(){let mut o=fake(true);o.joined=false;let v=run_close(&mut o,Instant::now()+Duration::from_secs(1));assert_eq!(v["backend_pid"],Value::Null);assert_eq!(v["backend_absent"],Value::Null);assert_eq!(v["driver_joined"],false);}

}
