//! `OpenAiLlm` contre un serveur SSE local écrit à la main : socket réelle,
//! **aucun réseau, aucun secret**. Ce que les tests unitaires du module ne
//! peuvent pas prouver, c'est ce qui se passe sur le transport — surtout que
//! `Flow::Stop` coupe bien la connexion.
#![cfg(feature = "openai-llm")]

mod common;

use std::sync::atomic::Ordering;

use common::fake_sse::*;
use rag3weaver::llm::{
    CountingSink, Finish, FinishReason, GenOptions, Llm, LlmError, StringSink, Turn,
};
use rag3weaver::openai_llm::OpenAiLlm;
use serde_json::Value;

fn hello() -> Vec<Turn> {
    vec![Turn::system("tu es utile"), Turn::user("bonjour")]
}

#[test]
fn streams_over_a_real_socket() {
    let srv = FakeServer::start(text_frames(), false);
    let llm = OpenAiLlm::new(&srv.url, "google/gemini-2.5-flash");
    let mut sink = StringSink::default();
    let (finish, usage) = llm.generate(&hello(), &GenOptions::default(), &mut sink).unwrap();
    assert_eq!(sink.text, "Bonjour le monde");
    assert_eq!(finish, Finish::eos());
    assert_eq!(usage.prompt_tokens, 11);
    assert_eq!(usage.completion_tokens, 3);
    assert_eq!(llm.name(), "google/gemini-2.5-flash");
}

#[test]
fn the_wire_carries_the_openai_shape() {
    let srv = FakeServer::start(text_frames(), false);
    let llm = OpenAiLlm::new(&srv.url, "gpt-x");
    let mut sink = StringSink::default();
    llm.generate(&hello(), &GenOptions::default().with_max_tokens(64), &mut sink).unwrap();

    let body: Value = serde_json::from_str(&srv.request.recv().unwrap()).unwrap();
    assert_eq!(body["stream"], true);
    assert_eq!(body["stream_options"]["include_usage"], true);
    assert_eq!(body["max_tokens"], 64);
    assert_eq!(body["messages"][0]["role"], "system");
}

#[test]
fn flow_stop_closes_the_connection() {
    // Le serveur réémet la dernière trame 100 000 fois : sans annulation
    // réelle, ce test ne se terminerait pas.
    let srv = FakeServer::start(text_frames(), true);
    let llm = OpenAiLlm::new(&srv.url, "m");
    let mut sink = CountingSink::stopping_after(2);
    let (finish, usage) = llm.generate(&hello(), &GenOptions::default(), &mut sink).unwrap();

    assert_eq!(finish, Finish::cancelled());
    assert!(!finish.is_complete(), "annulé : réponse incomplète");
    assert_eq!(sink.tokens, 2, "le générateur s'arrête net");
    assert_eq!(sink.finished, Some(Finish::cancelled()), "on_finish appelé quand même");
    assert_eq!(usage.completion_tokens, 2);

    let written = srv.written.load(Ordering::SeqCst);
    assert!(written < 10_000, "{written} trames écrites : la socket n'a pas été coupée");
}

#[test]
fn tool_call_is_accumulated_across_deltas() {
    let srv = FakeServer::start(tool_frames(), false);
    let llm = OpenAiLlm::new(&srv.url, "m");
    let mut sink = StringSink::default();
    let (finish, usage) = llm.generate(&hello(), &GenOptions::default(), &mut sink).unwrap();
    assert_eq!(sink.text, "");
    assert_eq!(finish.reason, FinishReason::ToolCall, "eu {finish:?}");
    let calls = &finish.tool_calls;
    assert_eq!(calls[0].id, "call_a1");
    assert_eq!(calls[0].name, "KBQuerySourceNode");
    let args: Value = serde_json::from_str(&calls[0].arguments).unwrap();
    assert_eq!(args["kb_name"], "docs");
    assert_eq!(args["query"], "luciole");
    assert_eq!(usage.completion_tokens, 24);
}

#[test]
fn a_client_side_stop_sequence_closes_the_socket() {
    // C'est ce qui justifie de ne plus envoyer `stop` au fournisseur : on
    // coupe nous-mêmes, et la socket se ferme immédiatement — seuls les
    // fragments déjà en vol sont facturés, pas la fin de la génération.
    // Le serveur réémet la dernière trame 100 000 fois : sans coupure réelle,
    // ce test ne se terminerait pas.
    let srv = FakeServer::start(stop_frames(), true);
    let llm = OpenAiLlm::new(&srv.url, "m");
    let opts = GenOptions::default().with_stop(vec!["Observation:".into()]);
    let mut sink = StringSink::default();
    let (finish, usage) = llm.generate(&hello(), &opts, &mut sink).unwrap();

    assert_eq!(finish, Finish::stop("Observation:"));
    assert!(finish.is_complete(), "l'appelant avait demandé ce stop");
    assert_eq!(sink.text, "pensée utile ", "préfixe verbatim, séquence non émise");
    // Le chunk final `usage` n'arrive jamais : on rend nos propres fragments.
    assert_eq!(usage.completion_tokens, 2);
    assert_eq!(usage.prompt_tokens, 0, "inconnu, non inventé");

    let written = srv.written.load(Ordering::SeqCst);
    assert!(written < 10_000, "{written} trames écrites : la socket n'a pas été coupée");
    eprintln!("[preuve] {written} trames écrites par le serveur avant coupure (sur 100 000)");
}

#[test]
fn an_http_error_becomes_a_model_error_without_the_secret() {
    let srv = FakeServer::start_error(401, r#"{"error":{"message":"API key not valid"}}"#);
    let llm = OpenAiLlm::ai_studio("SECRET-DE-LUCIE", "m").with_base_url(&srv.url);
    let mut sink = StringSink::default();
    let err = llm.generate(&hello(), &GenOptions::default(), &mut sink).unwrap_err();
    let m = err.to_string();
    assert!(matches!(err, LlmError::Model(_)));
    assert!(m.contains("401"), "{m}");
    assert!(!m.contains("SECRET-DE-LUCIE"), "secret fuité dans l'erreur : {m}");
}

#[test]
fn an_unreachable_endpoint_is_a_model_error() {
    let llm = OpenAiLlm::new("http://127.0.0.1:1/v1", "m");
    let mut sink = StringSink::default();
    assert!(matches!(
        llm.generate(&hello(), &GenOptions::default(), &mut sink).unwrap_err(),
        LlmError::Model(_)
    ));
}
