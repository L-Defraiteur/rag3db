//! Le branchement JavaScript, par QuickJS (`rquickjs`, qui embarque
//! quickjs-ng sans `quickjs-libc` : aucun module de fichiers ni de système
//! n'existe dans le binaire). Un `Runtime` neuf par appel, avec trois gardes :
//! une échéance (le gestionnaire d'interruption, appelé par le moteur pendant
//! l'exécution, rend `true` passé le délai et lève une exception que le script
//! ne peut pas attraper), un plafond de mémoire, une pile bornée.
//!
//! Le script définit `function run(input)` ; la valeur rendue passe par JSON.
//! Une promesse n'est pas une sortie : rien d'asynchrone n'est donné au
//! script, il n'a rien à attendre.
use super::{
    check_input, check_limits, check_output, Language, PreparedScript, ScriptEngine, ScriptError,
    ScriptErrorKind, ScriptLimits,
};
use rquickjs::{context::EvalOptions, Context, Ctx, Function, Runtime};
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Le nom du source dans les piles d'appel, d'où l'on relit ligne et colonne.
const FILENAME: &str = "script";
const STACK_BYTES: usize = 256 * 1024;
const ENTRY: &str = "run";
const MAX_JOBS: usize = 10_000;

/// Ce qu'un script d'ailleurs (Node, Deno, navigateur, qjs) s'attend à
/// trouver pour toucher aux fichiers, au réseau ou au système : demandé ici,
/// c'est un refus qui le nomme, pas une variable inconnue.
const HOST_NAMES: &[&str] = &[
    "require",
    "process",
    "fetch",
    "XMLHttpRequest",
    "WebSocket",
    "Deno",
    "Bun",
    "std",
    "os",
    "setTimeout",
    "setInterval",
];

pub(super) struct QuickJsEngine;

impl ScriptEngine for QuickJsEngine {
    fn language(&self) -> Language {
        Language::JavaScript
    }

    fn prepare(
        &self,
        source: &str,
        limits: &ScriptLimits,
    ) -> Result<Arc<dyn PreparedScript>, ScriptError> {
        prepare_js(Language::JavaScript, source.to_string(), limits)
    }
}

/// Prépare du JavaScript ; `language` est celui que le script a déclaré
/// (TypeScript arrive ici déjà effacé, lignes et colonnes gardées).
pub(super) fn prepare_js(
    language: Language,
    source: String,
    limits: &ScriptLimits,
) -> Result<Arc<dyn PreparedScript>, ScriptError> {
    check_limits(language, limits, &source)?;
    let script = PreparedJs {
        language,
        source: source.into(),
    };
    // Une première évaluation, dans un moteur jetable aux mêmes bornes : la
    // syntaxe, le code de premier niveau et la présence de `run` sont vérifiés
    // à la préparation, pas au premier appel.
    script.with_entry(limits, |_, _| Ok(()))?;
    Ok(Arc::new(script))
}

struct PreparedJs {
    language: Language,
    source: Arc<str>,
}

impl PreparedScript for PreparedJs {
    fn language(&self) -> Language {
        self.language
    }

    fn call(&self, input: &Value, limits: &ScriptLimits) -> Result<Value, ScriptError> {
        check_limits(self.language, limits, "")?;
        check_input(self.language, limits, input)?;
        let output = self.with_entry(limits, |ctx, run| {
            let argument = ctx.json_parse(input.to_string())?;
            let result: rquickjs::Value = run.call((argument,))?;
            if let Some(promise) = result.as_promise() {
                // Un rejet immédiat (`import(…)` sans chargeur) se dit par sa
                // raison ; les tâches sont vidées en nombre borné, sous la même
                // échéance.
                for _ in 0..MAX_JOBS {
                    if !ctx.execute_pending_job() {
                        break;
                    }
                }
                if let Some(Err(rejected)) = promise.result::<rquickjs::Value>() {
                    return Err(rejected);
                }
                return Ok(Err(ScriptError::new(
                    ScriptErrorKind::Output,
                    format!("{ENTRY} returned a promise: scripts are synchronous, nothing is given to await"),
                )));
            }
            Ok(match ctx.json_stringify(result)? {
                Some(text) => Ok(text.to_string()?),
                None => Err(ScriptError::new(
                    ScriptErrorKind::Output,
                    format!("{ENTRY} returned undefined or a value JSON cannot hold"),
                )),
            })
        })??;
        let value: Value = serde_json::from_str(&output)
            .map_err(|e| ScriptError::new(ScriptErrorKind::Output, e.to_string()))?;
        check_output(self.language, limits, value)
    }
}

impl PreparedJs {
    /// Un moteur neuf, borné ; le script évalué ; `body` reçoit `run`.
    fn with_entry<T>(
        &self,
        limits: &ScriptLimits,
        body: impl for<'js> FnOnce(&Ctx<'js>, Function<'js>) -> rquickjs::Result<T>,
    ) -> Result<T, ScriptError> {
        let runtime = Runtime::new().map_err(internal)?;
        runtime.set_memory_limit(limits.memory_bytes);
        runtime.set_max_stack_size(STACK_BYTES);
        let interrupted = Arc::new(AtomicBool::new(false));
        let deadline = Instant::now() + Duration::from_millis(limits.timeout_ms);
        let flag = interrupted.clone();
        runtime.set_interrupt_handler(Some(Box::new(move || {
            let late = Instant::now() >= deadline;
            if late {
                flag.store(true, Ordering::Relaxed);
            }
            late
        })));
        let context = Context::full(&runtime).map_err(internal)?;
        context.with(|ctx| {
            let mut options = EvalOptions::default();
            options.global = true;
            options.strict = true;
            options.filename = Some(FILENAME.into());
            let fail = |e: rquickjs::Error| self.error(&ctx, e, &interrupted, limits);
            ctx.eval_with_options::<(), _>(self.source.as_bytes(), options)
                .map_err(fail)?;
            let entry: rquickjs::Value = ctx.globals().get(ENTRY).map_err(fail)?;
            let Some(run) = entry.into_function() else {
                return Err(ScriptError::new(
                    ScriptErrorKind::MissingEntry,
                    format!("the script must define `function {ENTRY}(input)`, which returns the output"),
                ));
            };
            body(&ctx, run).map_err(fail)
        })
    }

    /// Ce que le moteur a levé, dit par son genre, avec la ligne du source.
    fn error(
        &self,
        ctx: &Ctx<'_>,
        error: rquickjs::Error,
        interrupted: &AtomicBool,
        limits: &ScriptLimits,
    ) -> ScriptError {
        if interrupted.load(Ordering::Relaxed) {
            return ScriptError::new(
                ScriptErrorKind::Timeout,
                format!("deadline exceeded ({} ms)", limits.timeout_ms),
            );
        }
        let memory = || {
            ScriptError::new(
                ScriptErrorKind::Memory,
                format!("memory ceiling reached ({} bytes)", limits.memory_bytes),
            )
        };
        match error {
            rquickjs::Error::Allocation => memory(),
            rquickjs::Error::Exception => {
                let thrown = ctx.catch();
                let (name, message, stack) = match thrown.as_exception() {
                    Some(exception) => (
                        exception.get::<_, String>("name").unwrap_or_default(),
                        exception.message().unwrap_or_default(),
                        exception.stack().unwrap_or_default(),
                    ),
                    None => (
                        String::new(),
                        ctx.json_stringify(thrown)
                            .ok()
                            .flatten()
                            .and_then(|s| s.to_string().ok())
                            .unwrap_or_else(|| "a non-error value was thrown".into()),
                        String::new(),
                    ),
                };
                let (line, column) = position(&stack, &message);
                let text = if name.is_empty() {
                    message.clone()
                } else {
                    format!("{name}: {message}")
                };
                let mut kind = classify(&name, &message);
                // Hors module, `import x from …` bute sur le mot suivant
                // (« Unexpected identifier 'x' ») : c'est la ligne qui dit
                // qu'un module était demandé.
                if kind == ScriptErrorKind::Syntax {
                    let written = line
                        .and_then(|l| self.source.lines().nth(l.saturating_sub(1) as usize))
                        .unwrap_or_default()
                        .trim_start();
                    for keyword in ["import", "export"] {
                        if written.starts_with(keyword)
                            && !written[keyword.len()..]
                                .starts_with(|c: char| c.is_alphanumeric() || c == '_' || c == '$')
                        {
                            kind = ScriptErrorKind::Forbidden(keyword.into());
                        }
                    }
                }
                if kind == ScriptErrorKind::Memory {
                    return memory();
                }
                let text = match &kind {
                    ScriptErrorKind::Forbidden(host) => format!(
                        "`{host}` is not given to scripts: no files, no network, no system ({text})"
                    ),
                    _ => text,
                };
                ScriptError::new(kind, text).at(line, column)
            }
            other => ScriptError::new(ScriptErrorKind::Runtime, other.to_string()),
        }
    }
}

fn classify(name: &str, message: &str) -> ScriptErrorKind {
    let lower = message.to_lowercase();
    if lower.contains("out of memory") {
        return ScriptErrorKind::Memory;
    }
    if lower.contains("stack overflow") {
        return ScriptErrorKind::Limit;
    }
    // `import(…)` sans chargeur, ou un message qui nomme `import` : un module
    // est demandé, et aucun n'est donné.
    if (name == "SyntaxError" && lower.contains("import"))
        || lower.contains("could not load module")
        || lower.contains("dynamic import")
    {
        return ScriptErrorKind::Forbidden("import".into());
    }
    match name {
        "SyntaxError" => ScriptErrorKind::Syntax,
        "ReferenceError" => match HOST_NAMES.iter().find(|host| names(message, host)) {
            Some(host) => ScriptErrorKind::Forbidden((*host).into()),
            None => ScriptErrorKind::Runtime,
        },
        _ => ScriptErrorKind::Runtime,
    }
}

/// « 'require' is not defined » ou « require is not defined ».
fn names(message: &str, host: &str) -> bool {
    message
        .split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '$'))
        .any(|word| word == host)
}

/// La première position `script:ligne:colonne` d'une pile ou d'un message.
fn position(stack: &str, message: &str) -> (Option<u32>, Option<u32>) {
    let marker = format!("{FILENAME}:");
    for text in [stack, message] {
        for (at, _) in text.match_indices(&marker) {
            let rest = &text[at + marker.len()..];
            let mut numbers = rest
                .split(|c: char| !c.is_ascii_digit())
                .take(2)
                .map(|n| n.parse::<u32>().ok());
            if let Some(Some(line)) = numbers.next() {
                return (Some(line), numbers.next().flatten());
            }
        }
    }
    (None, None)
}

fn internal(error: rquickjs::Error) -> ScriptError {
    ScriptError::new(ScriptErrorKind::Runtime, format!("script engine: {error}"))
}
