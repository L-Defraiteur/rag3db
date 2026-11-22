//! The witness of the lot `llm-logprobs` (22 November 2026): the web search goes through the spending ledger.
use super::*;

/// **The web search is reserved, then settled with Gemini's `usageMetadata`**: a call to `generateContent` that went
/// around the ledger is now written, at the tokens the reply says (the prompt less its cache, the cache read, the
/// candidates and the thoughts).
#[test]
fn the_web_search_is_reserved_and_settled_with_its_usage_metadata() {
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let address = format!("http://{}", server.server_addr().to_ip().unwrap());
    let thread = std::thread::spawn(move || {
        let request = server.recv().unwrap();
        let reply = json!({
            "candidates": [{ "content": { "parts": [{ "text": "an answer" }] } }],
            "usageMetadata": { "promptTokenCount": 120, "cachedContentTokenCount": 20, "candidatesTokenCount": 30, "thoughtsTokenCount": 5 }
        });
        request.respond(tiny_http::Response::from_string(reply.to_string())).unwrap();
    });
    let model = format!("gemini-web-search-witness-{}", std::process::id());
    let policy = WebSearchPolicy { model: Some(model.clone()), endpoint: Some(address.clone()), ..Default::default() };
    let got = web_search(&policy, &json!({ "query": "the weather" })).expect("the search");
    assert_eq!(got["answer"], json!("an answer"), "{got:#}");
    thread.join().unwrap();
    let ledger = crate::usage_ledger::user_ledger();
    let day = crate::usage_ledger::utc_day(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64);
    let (lines, _) = ledger.lines(&day[..7]);
    let mine: Vec<_> = lines.iter().filter(|l| l.slug == model).collect();
    assert_eq!(mine.iter().map(|l| l.kind.as_str()).collect::<Vec<_>>(), ["reserve", "settle"], "{mine:#?}");
    assert_eq!(mine[1].provider, address);
    assert_eq!(mine[1].tokens, Some(crate::usage_ledger::Tokens { input: 100, cache_read: 20, output: 35, ..Default::default() }));
}
