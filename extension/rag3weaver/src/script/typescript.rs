//! Le branchement TypeScript : les types sont **effacés**, jamais vérifiés ni
//! compilés. `swc_ts_fast_strip` en `StripOnly` (le retrait de types de Node)
//! remplace chaque annotation par des blancs — mêmes lignes, mêmes colonnes —,
//! puis le JavaScript obtenu va au branchement QuickJS : une erreur levée à
//! l'exécution porte donc la ligne du source tel qu'écrit, sans carte de
//! source. Ce qui ne s'efface pas (enum, namespace porteur de code, propriété
//! de paramètre de constructeur, `<T>expr`) est refusé en le nommant, avec sa
//! ligne : c'est le « TypeScript effaçable » de TypeScript 5.8.
use super::{
    check_limits, quickjs, Language, PreparedScript, ScriptEngine, ScriptError, ScriptErrorKind,
    ScriptLimits,
};
use std::sync::{Arc, Mutex};
use swc_common::errors::{DiagnosticBuilder, Emitter, Handler, HANDLER};
use swc_common::sync::Lrc;
use swc_common::{Globals, SourceMap, Span, GLOBALS};
use swc_ts_fast_strip::{operate, ErrorCode, Mode, Options};

pub(super) struct TypeScriptEngine;

impl ScriptEngine for TypeScriptEngine {
    fn language(&self) -> Language {
        Language::TypeScript
    }

    fn prepare(
        &self,
        source: &str,
        limits: &ScriptLimits,
    ) -> Result<Arc<dyn PreparedScript>, ScriptError> {
        check_limits(Language::TypeScript, limits, source)?;
        quickjs::prepare_js(Language::TypeScript, strip_types(source)?, limits)
    }
}

/// Les diagnostics de swc, gardés au lieu d'être écrits.
#[derive(Clone, Default)]
struct Collected(Arc<Mutex<Vec<(String, Option<Span>)>>>);

impl Emitter for Collected {
    fn emit(&mut self, db: &mut DiagnosticBuilder<'_>) {
        if let Ok(mut all) = self.0.lock() {
            all.push((db.message(), db.span.primary_span()));
        }
    }
}

/// Le JavaScript du source, positions gardées.
pub(super) fn strip_types(source: &str) -> Result<String, ScriptError> {
    let map: Lrc<SourceMap> = Default::default();
    let collected = Collected::default();
    let handler = Handler::with_emitter(false, false, Box::new(collected.clone()));
    let options = Options {
        filename: Some("script.ts".into()),
        mode: Mode::StripOnly,
        ..Default::default()
    };
    let result = GLOBALS.set(&Globals::new(), || {
        HANDLER.set(&handler, || {
            operate(&map, &handler, source.to_string(), options)
        })
    });
    let error = match result {
        Ok(output) => return Ok(output.code),
        Err(error) => error,
    };
    let kind = match error.code {
        ErrorCode::InvalidSyntax => ScriptErrorKind::Syntax,
        _ => ScriptErrorKind::Unsupported,
    };
    let first = collected.0.lock().ok().and_then(|all| all.first().cloned());
    Err(match first {
        Some((message, span)) => {
            let (line, column) = match span {
                Some(span) => {
                    let at = map.lookup_char_pos(span.lo);
                    (Some(at.line as u32), Some(at.col_display as u32 + 1))
                }
                None => (None, None),
            };
            ScriptError::new(kind, message).at(line, column)
        }
        None => ScriptError::new(kind, error.message),
    })
}
