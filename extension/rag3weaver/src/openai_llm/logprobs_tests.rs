//! The witness of the lot `llm-logprobs` (22 November 2026): the trait `Llm` gives the most likely tokens and passes
//! the chat template's parameters, so that the gate's judge can go through `connect_llm`.
use super::*;
use std::io::BufReader;

/// **Asked, the most likely tokens are sent for and read back, position by position; the chat template's parameters
/// go as they are; nothing is sent when nothing is asked.**
#[test]
fn the_most_likely_tokens_are_asked_and_read_back_and_the_template_parameters_go() {
    let llm = OpenAiLlm::new("http://127.0.0.1:1/v1", "qwen3.5-9b");
    let turns = [Turn::user("Y or N?")];
    let plain = llm.request_body(&turns, &GenOptions::default());
    assert!(plain.get("logprobs").is_none() && plain.get("top_logprobs").is_none() && plain.get("chat_template_kwargs").is_none(), "{plain}");
    let opts = GenOptions {
        max_tokens: 1,
        top_logprobs: Some(20),
        template_kwargs: Some(serde_json::json!({ "enable_thinking": false })),
        ..Default::default()
    };
    let body = llm.request_body(&turns, &opts);
    assert_eq!((body["logprobs"].clone(), body["top_logprobs"].clone()), (serde_json::json!(true), serde_json::json!(20)), "{body}");
    assert_eq!(body["chat_template_kwargs"], serde_json::json!({ "enable_thinking": false }));

    #[derive(Default)]
    struct Sink {
        text: String,
        seen: Vec<(usize, Vec<(String, f64)>)>,
    }
    impl TokenSink for Sink {
        fn on_token(&mut self, t: &str) -> Flow {
            self.text.push_str(t);
            Flow::Continue
        }
        fn on_top_logprobs(&mut self, position: usize, alternatives: &[(String, f64)]) {
            self.seen.push((position, alternatives.to_vec()));
        }
    }
    let frames = [
        r#"{"choices":[{"index":0,"delta":{"content":"Y"},"logprobs":{"content":[{"token":"Y","logprob":-0.1,"top_logprobs":[{"token":"Y","logprob":-0.1},{"token":"N","logprob":-2.4}]}]},"finish_reason":null}]}"#,
        r#"{"choices":[{"index":0,"delta":{"content":"."},"logprobs":{"content":[{"token":".","logprob":-0.01,"top_logprobs":[{"token":".","logprob":-0.01}]}]},"finish_reason":null}]}"#,
        r#"{"choices":[{"index":0,"delta":{},"finish_reason":"length"}]}"#,
    ];
    let body: String = frames.iter().map(|f| format!("data: {f}\n\n")).collect();
    let mut sink = Sink::default();
    read_sse(&mut BufReader::new(body.as_bytes()), &opts, &mut sink).unwrap();
    assert_eq!(sink.text, "Y.");
    assert_eq!(
        sink.seen,
        vec![(0, vec![("Y".to_string(), -0.1), ("N".to_string(), -2.4)]), (1, vec![(".".to_string(), -0.01)])]
    );
}
