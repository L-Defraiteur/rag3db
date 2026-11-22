//! **The gate's judge goes through the spending ledger** (c7 and ED, 22 Nov 2026: nothing goes around the ledger).
//! The judge reads the first token's `top_logprobs`, which `connect_llm` does not give: it reserves before its call
//! and settles after it, at price zero for a private address. A module of its own.

use super::served_judge_with;
use crate::usage_ledger::Ledger;
use serde_json::json;
use std::io::{Read, Write};
use std::sync::Arc;

/// A judge served once on a free local port: it answers Y with its logprobs and its usage.
fn one_answer() -> String {
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let adresse = l.local_addr().unwrap().to_string();
    std::thread::spawn(move || {
        let (mut c, _) = l.accept().unwrap();
        let mut buf = vec![0u8; 65536];
        let mut lu = 0;
        // Read the headers and the body (Content-Length), then answer.
        loop {
            let n = c.read(&mut buf[lu..]).unwrap();
            lu += n;
            let t = String::from_utf8_lossy(&buf[..lu]).to_string();
            if let Some(i) = t.find("\r\n\r\n") {
                let len = t.lines().find_map(|h| h.to_ascii_lowercase().strip_prefix("content-length:").map(|v| v.trim().parse::<usize>().unwrap())).unwrap_or(0);
                if lu >= i + 4 + len || n == 0 {
                    break;
                }
            }
        }
        let corps = json!({
            "choices": [{ "logprobs": { "content": [{ "top_logprobs": [{ "token": "Y", "logprob": -0.01 }, { "token": "N", "logprob": -5.0 }] }] } }],
            "usage": { "prompt_tokens": 412, "completion_tokens": 1 }
        })
        .to_string();
        write!(c, "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{corps}", corps.len()).unwrap();
    });
    adresse
}

#[test]
fn the_served_judge_reserves_then_settles_at_price_zero() {
    let d = tempfile::tempdir().unwrap();
    let prompt = d.path().join("yn.txt");
    std::fs::write(&prompt, "Answer Y or N.").unwrap();
    let ledger = Arc::new(Ledger::new(d.path().join("ledger"), d.path().join("prices.json")));
    let adresse = one_answer();
    let judge = served_judge_with(&json!({ "address": adresse, "prompt_file": prompt, "threshold": 0.5, "model": "qwen3.5-9b" }), ledger.clone()).unwrap();
    let v = judge(&json!({ "action": { "command": "rm vieux.txt" }, "turns": [{ "by": { "kind": "human" }, "text": "supprime vieux.txt" }] }));
    assert_eq!(v["label"], "explicit", "{v}");
    let mois = crate::usage_ledger::utc_day(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64)[..7].to_string();
    let (lignes, illisibles) = ledger.lines(&mois);
    assert!(illisibles.is_empty(), "{illisibles:?}");
    let kinds: Vec<&str> = lignes.iter().map(|l| l.kind.as_str()).collect();
    assert_eq!(kinds, ["reserve", "settle"], "the judge's call in the ledger: {lignes:?}");
    assert!(lignes.iter().all(|l| l.free && l.slug == "qwen3.5-9b" && l.provider.contains("127.0.0.1")), "{lignes:?}");
    assert_eq!(lignes[1].tokens.map(|t| (t.input, t.output)), Some((412, 1)), "settled by the server's usage");
}

/// A llama-server served on a free local port: `/tokenize`, then `/completion` with its probabilities and its counts.
fn llama_server() -> String {
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let adresse = l.local_addr().unwrap().to_string();
    std::thread::spawn(move || {
        for _ in 0..2 {
            let (mut c, _) = l.accept().unwrap();
            let mut buf = vec![0u8; 65536];
            let mut lu = 0;
            let ligne = loop {
                let n = c.read(&mut buf[lu..]).unwrap();
                lu += n;
                let t = String::from_utf8_lossy(&buf[..lu]).to_string();
                if let Some(i) = t.find("\r\n\r\n") {
                    let len = t.lines().find_map(|h| h.to_ascii_lowercase().strip_prefix("content-length:").map(|v| v.trim().parse::<usize>().unwrap())).unwrap_or(0);
                    if lu >= i + 4 + len || n == 0 {
                        break t.lines().next().unwrap_or_default().to_string();
                    }
                }
            };
            let corps = if ligne.contains("/tokenize") {
                json!({ "tokens": [1, 2, 3] })
            } else {
                json!({
                    "completion_probabilities": [{ "top_logprobs": [{ "token": " oui", "logprob": -0.1 }, { "token": " non", "logprob": -3.0 }] }],
                    "tokens_evaluated": 3, "tokens_predicted": 1
                })
            }
            .to_string();
            write!(c, "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{corps}", corps.len()).unwrap();
        }
    });
    adresse
}

#[test]
fn the_llama_server_decider_reserves_then_settles_at_price_zero() {
    let d = tempfile::tempdir().unwrap();
    let ledger = Arc::new(Ledger::new(d.path().join("ledger"), d.path().join("prices.json")));
    let decider = crate::decider::LlamaServerDecider::new(&format!("http://{}", llama_server()), "laya").unwrap().with_ledger(ledger.clone());
    let top = decider.next_token_logprobs("Oui ou non ?").unwrap();
    assert_eq!(top.len(), 2, "{top:?}");
    let mois = crate::usage_ledger::utc_day(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64)[..7].to_string();
    let (lignes, _) = ledger.lines(&mois);
    let kinds: Vec<&str> = lignes.iter().map(|l| l.kind.as_str()).collect();
    assert_eq!(kinds, ["reserve", "settle"], "the decider's call in the ledger: {lignes:?}");
    assert!(lignes.iter().all(|l| l.free && l.slug == "laya" && l.provider.contains("127.0.0.1")), "{lignes:?}");
    assert_eq!(lignes[1].tokens.map(|t| (t.input, t.output)), Some((3, 1)), "settled by the server's counts");
}
