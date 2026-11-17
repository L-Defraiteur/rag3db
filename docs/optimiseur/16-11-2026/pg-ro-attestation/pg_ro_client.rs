//! Candidat NON COMPILÉ/NON QUALIFIÉ. Parent hard5 et autorité C7/G requis.
//! Aucun protocole PostgreSQL maison : bibliothèques de production archivées.
#![forbid(unsafe_code)]

use std::io::{self, Read, Write};
use std::net::{IpAddr, Ipv4Addr};
use std::time::Duration;
use serde_json::Value;
use tokio::time::{timeout_at, Instant};
use tokio_postgres::{config::{Host, SslMode}, Config, NoTls, SimpleQueryMessage};

const IDENTITY: &str = include_str!("identity.sql");
const POSTCHECK: &str = include_str!("backend-postcheck.sql");
const OPTIONS: &str = "-c default_transaction_read_only=on -c statement_timeout=1000 -c lock_timeout=500";

fn emit(phase: &str, raw: &str) -> Result<(), &'static str> {
    let mut frame = serde_json::Map::new();
    frame.insert("phase".into(), Value::String(phase.into()));
    frame.insert("raw_stdout".into(), Value::String(raw.into()));
    frame.insert("connection_driver_joined".into(), Value::Bool(true));
    let line = serde_json::to_string(&frame).map_err(|_| "frame encoding failed")?;
    if line.len() > 65536 { return Err("frame exceeds bound"); }
    println!("{line}");
    io::stdout().flush().map_err(|_| "frame flush failed")
}

async fn query_closed(config: Config, sql: &'static str, deadline: Instant) -> Result<String, &'static str> {
    let (client, connection) = timeout_at(deadline, config.connect(NoTls)).await
        .map_err(|_| "connect deadline")?.map_err(|_| "connect refused")?;
    let driver = tokio::spawn(connection);
    let messages = timeout_at(deadline, client.simple_query(sql)).await
        .map_err(|_| "query deadline")?.map_err(|_| "query refused")?;
    let mut row = None;
    let mut commands = 0;
    for message in messages {
        match message {
            SimpleQueryMessage::Row(value) => {
                if row.is_some() || value.columns().len() != 1 { return Err("unexpected row count or shape"); }
                let text = value.try_get(0).map_err(|_| "column conversion refused")?
                    .ok_or("null SQL JSON")?;
                if text.len() > 65536 { return Err("SQL JSON exceeds bound"); }
                row = Some(text.to_owned());
            }
            SimpleQueryMessage::CommandComplete(count) => {
                if count != 1 { return Err("unexpected command count"); }
                commands += 1;
            }
            SimpleQueryMessage::RowDescription(columns) => {
                if columns.len() != 1 { return Err("unexpected column count"); }
            }
            _ => return Err("unexpected query message"),
        }
    }
    if commands != 1 { return Err("missing or extra command completion"); }
    drop(client);
    // Piloter la fermeture de la liaison AVANT toute preuve et avant postcheck.
    timeout_at(deadline, driver).await.map_err(|_| "connection shutdown deadline")?
        .map_err(|_| "connection driver task failed")?.map_err(|_| "connection shutdown refused")?;
    row.ok_or("missing SQL row")
}

async fn capture(request: Value) -> Result<(), &'static str> {
    let object = request.as_object().ok_or("request must be an object")?;
    if object.len() != 3 || !["dsn", "nonce", "budget_ms"].iter().all(|k| object.contains_key(*k)) {
        return Err("unexpected request fields");
    }
    let dsn = request["dsn"].as_str().ok_or("missing in-memory DSN")?;
    let nonce = request["nonce"].as_str().ok_or("missing nonce")?;
    if dsn.len() > 4096 || nonce.len() != 32 || !nonce.bytes().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()) {
        return Err("DSN or nonce refused");
    }
    let budget = request["budget_ms"].as_u64().filter(|n| (1..=4000).contains(n)).ok_or("budget refused")?;
    // Une seule échéance intérieure ; le parent inclut startup/stdin/hash/reap dans hard5.
    let deadline = Instant::now() + Duration::from_millis(budget);
    let mut config: Config = dsn.parse().map_err(|_| "canonical DSN parse refused")?;
    if config.get_user() != Some("rag3weaver") || config.get_dbname() != Some("rag3weaver_test")
        || config.get_ports() != [5433] || config.get_hosts().len() != 1
        || config.get_options().is_some() || !config.get_hostaddrs().is_empty() {
        return Err("foreign PostgreSQL target refused");
    }
    match &config.get_hosts()[0] {
        Host::Tcp(host) if host == "127.0.0.1" => (),
        _ => return Err("foreign host or Unix socket refused"),
    }
    let application = format!("g-pg-{nonce}");
    config.hostaddr(IpAddr::V4(Ipv4Addr::LOCALHOST)).ssl_mode(SslMode::Disable)
        .connect_timeout(Duration::from_millis(1200)).application_name(application.clone()).options(OPTIONS);
    let identity = query_closed(config.clone(), IDENTITY, deadline).await?;
    let typed: Value = serde_json::from_str(&identity).map_err(|_| "invalid identity JSON")?;
    let pid = typed["backend_pid"].as_u64().filter(|n| *n > 0 && *n <= i32::MAX as u64).ok_or("backend PID refused")?;
    if typed["application_name"].as_str() != Some(application.as_str()) { return Err("backend nonce differs"); }
    emit("identity", &identity)?;
    config.application_name(format!("{application}:check")).options(format!(
        "{OPTIONS} -c g_pg.expected_backend_pid={pid} -c g_pg.expected_application_name={application}"));
    let postcheck = query_closed(config, POSTCHECK, deadline).await?;
    let checked: Value = serde_json::from_str(&postcheck).map_err(|_| "invalid postcheck JSON")?;
    emit("postcheck", &postcheck)?;
    if checked["backend_absent"].as_bool() != Some(true) { return Err("original backend still present"); }
    if Instant::now() > deadline { return Err("capture exceeded original deadline"); }
    println!("{{\"capture_complete\":true,\"transport_qualified\":false}}");
    io::stdout().flush().map_err(|_| "completion flush failed")
}

fn main() {
    // Pas d'action implicite, de DSN en argv, de psqlrc, de shell ou d'outil Docker.
    if std::env::args().skip(1).collect::<Vec<_>>() != ["--capture-readonly"] {
        eprintln!("UNQUALIFIED preparation candidate; explicit authorized mode required");
        std::process::exit(2);
    }
    let result = (|| -> Result<(), &'static str> {
        let mut raw = Vec::new();
        io::stdin().take(8193).read_to_end(&mut raw).map_err(|_| "request read failed")?;
        if raw.len() > 8192 { return Err("request exceeds bound"); }
        let request = serde_json::from_slice(&raw).map_err(|_| "request JSON refused")?;
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|_| "runtime refused")?;
        runtime.block_on(capture(request))
    })();
    if let Err(reason) = result {
        // Erreur de phase, jamais le DSN/password ni un Debug du client.
        eprintln!("PG read-only candidate refused: {reason}");
        std::process::exit(1);
    }
}
