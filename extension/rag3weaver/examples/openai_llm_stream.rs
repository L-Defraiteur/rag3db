//! Premier appel réel à un fournisseur distant — la réponse s'affiche au fil
//! de l'eau sur la sortie standard.
//!
//! ```text
//! cargo run --features openai-llm --example openai_llm_stream -- <cible> [prompt]
//! ```
//!
//! # Les deux voies Google
//!
//! **AI Studio — deux minutes.** Une clé d'API sur
//! <https://aistudio.google.com/apikey> (le projet Cloud et la clé sont créés
//! à l'acceptation des CGU), puis :
//!
//! ```text
//! export GEMINI_API_KEY=…
//! cargo run --features openai-llm --example openai_llm_stream -- ai-studio
//! ```
//!
//! ⚠ En compte **Prepay** (le défaut depuis mars 2026), il faut garder un
//! solde prépayé strictement positif : à zéro, les crédits startup ne se
//! consomment pas et toutes les clés s'arrêtent d'un coup.
//!
//! **Vertex AI — un quart d'heure, mais c'est là que vont les crédits
//! startup.** Projet avec facturation activée, API `aiplatform.googleapis.com`
//! activée, puis soit un compte de service (rôle `roles/aiplatform.user`, clé
//! JSON), soit `gcloud auth application-default login` :
//!
//! ```text
//! export GOOGLE_CLOUD_PROJECT=mon-projet
//! export GOOGLE_APPLICATION_CREDENTIALS=/chemin/compte-de-service.json  # ou ADC
//! cargo run --features openai-llm --example openai_llm_stream -- vertex
//! ```
//!
//! **N'importe quel autre endpoint compatible OpenAI**, y compris un modèle
//! local sans clé (llama.cpp `--server`, vLLM, Ollama) :
//!
//! ```text
//! cargo run --features openai-llm --example openai_llm_stream -- \
//!     http://127.0.0.1:8080/v1
//! # avec une clé : export OPENAI_API_KEY=… (et OPENAI_MODEL pour le modèle)
//! ```
//!
//! # L'effort de raisonnement, et pourquoi il est réglé ici
//!
//! `REASONING_EFFORT` (`minimal|low|medium|high`) borne la réflexion du
//! modèle. Sur Gemini 3.x, **ne rien régler n'est pas neutre** : la réflexion
//! s'étend jusqu'à saturer `max_tokens` et tronque la vraie réponse. Mesuré
//! sur Vertex avec 34 375 jetons d'entrée : sans réglage, 90 s, 11 520 jetons
//! de réflexion, réponse **tronquée**, ~0,050 $ ; avec `low`, 9 s, réponse
//! complète, 0,0149 $. D'où le défaut `low` sur les cibles Google.
//!
//! Pour un fournisseur générique, rien n'est envoyé par défaut : on ne change
//! pas le comportement de qui ne connaît pas ce réglage.
//!
//! Aucune clé n'est jamais écrite en dur ni affichée.

use std::io::Write;

use rag3weaver::llm::{Finish, Flow, GenOptions, Llm, ReasoningEffort, TokenSink, Turn};
use rag3weaver::openai_llm::{secret_from_env, Auth, OpenAiLlm};

/// Écrit chaque fragment tout de suite : c'est le point de l'exercice.
struct StdoutSink {
    chars: usize,
}

impl TokenSink for StdoutSink {
    fn on_token(&mut self, delta: &str) -> Flow {
        print!("{delta}");
        let _ = std::io::stdout().flush();
        self.chars += delta.chars().count();
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

/// Lit `REASONING_EFFORT`. `google` porte le défaut : `low` pour Gemini (voir
/// l'en-tête), rien pour un fournisseur générique.
fn reasoning(google: bool) -> Result<Option<ReasoningEffort>, String> {
    match std::env::var("REASONING_EFFORT").ok().filter(|v| !v.trim().is_empty()) {
        None => Ok(google.then_some(ReasoningEffort::Low)),
        Some(v) => match v.trim() {
            "minimal" => Ok(Some(ReasoningEffort::Minimal)),
            "low" => Ok(Some(ReasoningEffort::Low)),
            "medium" => Ok(Some(ReasoningEffort::Medium)),
            "high" => Ok(Some(ReasoningEffort::High)),
            "none" | "off" => Ok(None),
            other => Err(format!(
                "REASONING_EFFORT={other:?} inconnu.\n  \
                 Attendu : minimal, low, medium, high — ou `none` pour ne rien envoyer."
            )),
        },
    }
}

fn build(target: &str) -> Result<(OpenAiLlm, bool), String> {
    match target {
        "ai-studio" | "gemini" => {
            let key = secret_from_env("GEMINI_API_KEY").map_err(|_| {
                "GEMINI_API_KEY n'est pas définie.\n  \
                 Prends une clé sur https://aistudio.google.com/apikey (deux minutes),\n  \
                 puis : export GEMINI_API_KEY=…"
                    .to_string()
            })?;
            Ok((OpenAiLlm::ai_studio(key, env_or("GEMINI_MODEL", "gemini-3.5-flash")), true))
        }
        "vertex" => {
            let project = std::env::var("GOOGLE_CLOUD_PROJECT").map_err(|_| {
                "GOOGLE_CLOUD_PROJECT n'est pas définie.\n  \
                 export GOOGLE_CLOUD_PROJECT=mon-projet"
                    .to_string()
            })?;
            // Le jeton dure une heure ; `TokenSource` le renouvelle seul.
            let source = rag3weaver::gcp_auth::TokenSource::from_env().map_err(|e| {
                format!(
                    "{e}\n  Deux voies : un compte de service\n    \
                     export GOOGLE_APPLICATION_CREDENTIALS=/chemin/compte-de-service.json\n  \
                     ou les identifiants par défaut\n    \
                     gcloud auth application-default login"
                )
            })?;
            let token = source.token().map_err(|e| e.to_string())?;
            Ok((
                OpenAiLlm::vertex(
                    &project,
                    &env_or("GOOGLE_CLOUD_LOCATION", "global"),
                    token,
                    env_or("VERTEX_MODEL", "google/gemini-3.5-flash"),
                ),
                true,
            ))
        }
        url if url.starts_with("http://") || url.starts_with("https://") => {
            let llm = OpenAiLlm::new(url, env_or("OPENAI_MODEL", "gpt-4o-mini"));
            // Une clé est facultative : un llama.cpp local n'en veut pas.
            Ok((
                match secret_from_env("OPENAI_API_KEY") {
                    Ok(k) => llm.with_auth(Auth::Bearer(k)),
                    Err(_) => llm,
                },
                false,
            ))
        }
        other => Err(format!(
            "cible inconnue : {other:?}\n  \
             Attendu : `ai-studio`, `vertex`, ou une URL d'endpoint compatible OpenAI."
        )),
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(target) = args.next() else {
        eprintln!(
            "usage : cargo run --features openai-llm --example openai_llm_stream \
             -- <ai-studio|vertex|URL> [prompt]\n\n\
             Voir l'en-tête de examples/openai_llm_stream.rs pour la mise en place \
             des deux voies Google."
        );
        std::process::exit(2);
    };
    let prompt = args.collect::<Vec<_>>().join(" ");
    let prompt = if prompt.trim().is_empty() {
        "Explique en trois phrases ce qu'est un index inversé.".to_string()
    } else {
        prompt
    };

    // Validé AVANT de construire le client : une faute de frappe dans la
    // configuration doit se voir même sans identifiants.
    let google = matches!(target.as_str(), "ai-studio" | "gemini" | "vertex");
    let effort = match reasoning(google) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };

    let (llm, _) = match build(&target) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    eprintln!("── modèle : {} (contexte {})", llm.name(), llm.context_len());
    match effort {
        Some(e) => eprintln!("── raisonnement : {e}"),
        None => eprintln!("── raisonnement : réglage non envoyé"),
    }
    eprintln!("── prompt : {prompt}\n");

    let turns = vec![
        Turn::system("Tu réponds en français, brièvement et sans fioritures."),
        Turn::user(prompt),
    ];
    let mut opts = GenOptions::default().with_max_tokens(512);
    if let Some(e) = effort {
        opts = opts.with_reasoning(e);
    }
    let mut sink = StdoutSink { chars: 0 };

    match llm.generate(&turns, &opts, &mut sink) {
        Ok((_, usage)) => {
            eprintln!(
                "── {} jetons en {} ms ({:.1} jetons/s), {} jetons de prompt, {} caractères",
                usage.completion_tokens,
                usage.ms,
                usage.tokens_per_s(),
                usage.prompt_tokens,
                sink.chars
            );
        }
        Err(e) => {
            eprintln!("\n── échec : {e}");
            std::process::exit(1);
        }
    }
}
