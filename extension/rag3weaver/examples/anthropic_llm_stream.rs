//! Premier appel réel à l'API Messages d'Anthropic — la réponse s'affiche au
//! fil de l'eau, la réflexion résumée à part sur la sortie d'erreur.
//!
//! ```text
//! export ANTHROPIC_API_KEY=…        # lue, jamais écrite ni affichée
//! cargo run --features anthropic-llm --example anthropic_llm_stream -- [prompt]
//! ```
//!
//! Réglages, tous facultatifs :
//! - `ANTHROPIC_MODEL` — `claude-opus-5` par défaut ;
//! - `REASONING_EFFORT` — `low|medium|high`, ou `none` pour laisser le
//!   fournisseur décider (son défaut est `high`) ;
//! - `MAX_TOKENS` — 16 000 par défaut : la réflexion compte dans ce plafond,
//!   un petit plafond tronque la réponse avant qu'elle commence ;
//! - `ANTHROPIC_BASE_URL` — un mandataire ou un serveur de test ;
//! - `RAG3WEAVER_SSE_DUMP=<fichier>` — le flux brut, pour comparer la forme
//!   réelle aux transcriptions des tests.

use std::io::Write;

use rag3weaver::anthropic_llm::{AnthropicLlm, DEFAULT_MODEL};
use rag3weaver::llm::{Finish, Flow, GenOptions, Llm, ReasoningEffort, TokenSink, Turn};

struct StdoutSink {
    chars: usize,
    reasoning_chars: usize,
}

impl TokenSink for StdoutSink {
    fn on_token(&mut self, delta: &str) -> Flow {
        print!("{delta}");
        let _ = std::io::stdout().flush();
        self.chars += delta.chars().count();
        Flow::Continue
    }
    fn on_reasoning(&mut self, delta: &str) -> Flow {
        // Sur la sortie d'erreur : la réponse seule va sur la sortie standard.
        eprint!("{delta}");
        self.reasoning_chars += delta.chars().count();
        Flow::Continue
    }
    fn on_finish(&mut self, finish: &Finish) {
        println!();
        eprintln!("── fin : {:?}", finish.reason);
        for c in &finish.tool_calls {
            eprintln!("── outil demandé : {} {} ({})", c.id, c.name, c.arguments);
        }
    }
}

fn env_or(var: &str, default: &str) -> String {
    std::env::var(var).ok().filter(|v| !v.trim().is_empty()).unwrap_or_else(|| default.into())
}

fn reasoning() -> Result<Option<ReasoningEffort>, String> {
    match std::env::var("REASONING_EFFORT").ok().filter(|v| !v.trim().is_empty()) {
        None => Ok(None),
        Some(v) => match v.trim() {
            "low" => Ok(Some(ReasoningEffort::Low)),
            "medium" => Ok(Some(ReasoningEffort::Medium)),
            "high" => Ok(Some(ReasoningEffort::High)),
            "none" | "off" => Ok(None),
            other => Err(format!(
                "REASONING_EFFORT={other:?} inconnu.\n  Attendu : low, medium, high — ou `none`."
            )),
        },
    }
}

fn main() {
    let prompt = std::env::args().skip(1).collect::<Vec<_>>().join(" ");
    let prompt = if prompt.trim().is_empty() {
        "Explique en trois phrases ce qu'est un index inversé.".to_string()
    } else {
        prompt
    };

    // Validé avant de chercher la clé : une faute de configuration doit se
    // voir même sans identifiants.
    let effort = match reasoning() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    let max_tokens: usize = match env_or("MAX_TOKENS", "16000").parse() {
        Ok(n) => n,
        Err(_) => {
            eprintln!("MAX_TOKENS doit être un entier");
            std::process::exit(2);
        }
    };

    let llm = match AnthropicLlm::from_env(env_or("ANTHROPIC_MODEL", DEFAULT_MODEL)) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("{e}\n  export ANTHROPIC_API_KEY=…");
            std::process::exit(1);
        }
    };
    let llm = match std::env::var("ANTHROPIC_BASE_URL") {
        Ok(url) if !url.trim().is_empty() => llm.with_base_url(url),
        _ => llm,
    };

    eprintln!("── modèle : {} (contexte {})", llm.name(), llm.context_len());
    match effort {
        Some(e) => eprintln!("── effort : {e}"),
        None => eprintln!("── effort : réglage non envoyé (défaut du fournisseur)"),
    }
    eprintln!("── prompt : {prompt}\n");

    let turns = vec![
        Turn::system("Tu réponds en français, brièvement et sans fioritures."),
        Turn::user(prompt),
    ];
    let mut opts = GenOptions::default().with_max_tokens(max_tokens);
    if let Some(e) = effort {
        opts = opts.with_reasoning(e);
    }
    let mut sink = StdoutSink { chars: 0, reasoning_chars: 0 };

    match llm.generate(&turns, &opts, &mut sink) {
        Ok((_, usage)) => {
            eprintln!(
                "── {} jetons en {} ms ({:.1} jetons/s), {} jetons de prompt dont {} au cache, \
                 {} caractères de réponse, {} de réflexion",
                usage.completion_tokens,
                usage.ms,
                usage.tokens_per_s(),
                usage.prompt_tokens,
                usage.cached_prompt_tokens,
                sink.chars,
                sink.reasoning_chars
            );
        }
        Err(e) => {
            eprintln!("\n── échec : {e}");
            std::process::exit(1);
        }
    }
}
