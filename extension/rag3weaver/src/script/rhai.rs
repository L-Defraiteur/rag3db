//! Le branchement rhai : ce que faisait `harness::evaluate`, à l'identique —
//! une expression qui lit la constante `input` et rend sa dernière valeur ;
//! `json_string` et `content_hash` pour seules fonctions de l'hôte ; `eval`,
//! `import`, `export` coupés ; opérations, profondeurs, tailles et échéance
//! bornées. Le script est compilé une fois à la préparation.
use super::{
    check_input, check_limits, check_output, Language, PreparedScript, ScriptEngine, ScriptError,
    ScriptErrorKind, ScriptLimits,
};
use rhai::EvalAltResult;
use serde_json::Value;
use std::sync::Arc;

pub(super) struct RhaiEngine;

struct PreparedRhai {
    ast: rhai::AST,
}

impl ScriptEngine for RhaiEngine {
    fn language(&self) -> Language {
        Language::Rhai
    }

    fn prepare(
        &self,
        source: &str,
        limits: &ScriptLimits,
    ) -> Result<Arc<dyn PreparedScript>, ScriptError> {
        check_limits(Language::Rhai, limits, source)?;
        let ast = engine(limits, None)
            .compile(source)
            .map_err(|e| error(e.into()))?;
        Ok(Arc::new(PreparedRhai { ast }))
    }
}

impl PreparedScript for PreparedRhai {
    fn language(&self) -> Language {
        Language::Rhai
    }

    fn call(&self, input: &Value, limits: &ScriptLimits) -> Result<Value, ScriptError> {
        check_limits(Language::Rhai, limits, "")?;
        check_input(Language::Rhai, limits, input)?;
        let engine = engine(limits, Some(std::time::Instant::now()));
        let mut scope = rhai::Scope::new();
        scope.push_constant("input", rhai::serde::to_dynamic(input).map_err(error)?);
        let value = engine
            .eval_ast_with_scope::<rhai::Dynamic>(&mut scope, &self.ast)
            .map_err(error)?;
        let result: Value = rhai::serde::from_dynamic(&value).map_err(error)?;
        check_output(Language::Rhai, limits, result)
    }
}

/// Le moteur, réglé comme avant ; l'échéance part de `start` quand il y en a
/// un (à l'appel, pas à la préparation).
fn engine(limits: &ScriptLimits, start: Option<std::time::Instant>) -> rhai::Engine {
    let mut engine = rhai::Engine::new();
    engine.register_fn(
        "json_string",
        |value: rhai::Dynamic| -> Result<String, Box<EvalAltResult>> {
            let value: Value = rhai::serde::from_dynamic(&value)?;
            Ok(value.to_string())
        },
    );
    engine.register_fn(
        "content_hash",
        |value: rhai::Dynamic| -> Result<String, Box<EvalAltResult>> {
            let value: Value = rhai::serde::from_dynamic(&value)?;
            Ok(blake3::hash(value.to_string().as_bytes())
                .to_hex()
                .to_string())
        },
    );
    // no_module is also enabled at build time. No host I/O APIs are registered.
    for keyword in ["eval", "import", "export"] {
        engine.disable_symbol(keyword);
    }
    engine.set_max_operations(limits.operations);
    engine.set_max_call_levels(32);
    engine.set_max_expr_depths(32, 32);
    engine.set_max_string_size(limits.json_bytes);
    engine.set_max_array_size(limits.collection_items);
    engine.set_max_map_size(limits.collection_items);
    engine.on_print(|_| {});
    engine.on_debug(|_, _, _| {});
    if let Some(start) = start {
        let timeout = limits.timeout_ms;
        engine.on_progress(move |_| {
            (start.elapsed().as_millis() >= timeout as u128).then(|| "deadline exceeded".into())
        });
    }
    engine
}

/// Le message reste celui de rhai, mot pour mot ; le genre dit ce qui s'est
/// passé.
fn error(e: Box<EvalAltResult>) -> ScriptError {
    let kind = match &*e {
        EvalAltResult::ErrorTerminated(..) => ScriptErrorKind::Timeout,
        EvalAltResult::ErrorTooManyOperations(..)
        | EvalAltResult::ErrorStackOverflow(..)
        | EvalAltResult::ErrorDataTooLarge(..) => ScriptErrorKind::Limit,
        EvalAltResult::ErrorParsing(..) => ScriptErrorKind::Syntax,
        EvalAltResult::ErrorFunctionNotFound(signature, _) => ScriptErrorKind::Forbidden(
            signature
                .split(|c: char| c == ' ' || c == '(')
                .next()
                .unwrap_or(signature)
                .to_string(),
        ),
        _ => ScriptErrorKind::Runtime,
    };
    ScriptError::new(kind, e.to_string())
}
