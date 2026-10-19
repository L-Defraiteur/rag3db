//! **Le composant React d'un gabarit, compilé en module ES pour le navigateur** (marche 3 de
//! l'IDE, niveau 1 ; étude `docs/10-octobre-2026-18h20/codeparsers/07-un-composant-react-est-un-gabarit.md`).
//!
//! Contrairement aux branchements `typescript` et `javascript`, rien ne s'exécute ici : le
//! module rendu tourne dans la page, et l'hôte ne fait que le compiler. La chaîne est celle de
//! swc, déjà compilé pour `swc_ts_fast_strip` (aucune crate nouvelle, mêmes versions) :
//! lecture TSX → résolution des portées → effacement des types → JSX automatique
//! (`react/jsx-dev-runtime` en développement, `react/jsx-runtime` en production) → **React
//! Refresh** en développement (`$RefreshReg$` / `$RefreshSig$`, la transformation de
//! `swc_ecma_transforms_react`) → génération, avec sa source map.
//!
//! **Le nom de famille de React Refresh est l'id du gabarit** : le module se termine, en
//! développement, par deux fonctions `$RefreshReg$` et `$RefreshSig$` qui préfixent chaque
//! composant de l'id du gabarit et délèguent au runtime posé par la page
//! (`globalThis.__rag3weaverRefresh`, ses `register(type, id)` et `signature()`). Ce sont des
//! déclarations de fonction, donc hissées, et ajoutées en fin de module : la source map n'est
//! pas décalée. Sans runtime posé, elles ne font rien.
//!
//! Un même source, un même id et un même mode rendent toujours les mêmes octets, donc le même
//! `hash` (blake3) : c'est la version du module, et sa clé de cache.
//!
//! **Deux formats** : le module ES tel quel, ou CommonJS (`require`, `exports`, interop par
//! défaut comprise : `swc_ecma_transforms_module`), pour le registre de modules de la page, qui
//! exécute aussi les paquets npm de React, déjà en CommonJS. Les spécifiers importés sont rendus
//! à part (`imports`) : la page les résout par sa table, et l'hôte en tire les importeurs.
use super::{ScriptError, ScriptErrorKind};
use std::sync::{Arc, Mutex};
use swc_common::comments::SingleThreadedComments;
use swc_common::errors::{DiagnosticBuilder, Emitter, Handler, HANDLER};
use swc_common::source_map::DefaultSourceMapGenConfig;
use swc_common::sync::Lrc;
use swc_common::{FileName, Globals, Mark, SourceMap, Span, GLOBALS};
use swc_ecma_ast::{
    Decl, DefaultDecl, EsVersion, ExportSpecifier, ModuleDecl, ModuleExportName, ModuleItem, Pat,
    Program,
};
use swc_ecma_parser::{lexer::Lexer, EsSyntax, Parser, StringInput, Syntax, TsSyntax};
use swc_ecma_transforms_base::fixer::fixer;
use swc_ecma_transforms_base::helpers::{inject_helpers, Helpers, HELPERS};
use swc_ecma_transforms_base::hygiene::hygiene;
use swc_ecma_transforms_base::resolver;
use swc_ecma_transforms_module::common_js::{common_js, FeatureFlag};
use swc_ecma_transforms_module::import_analysis::import_analyzer;
use swc_ecma_transforms_module::path::Resolver;
use swc_ecma_transforms_module::util::ImportInterop;
use swc_ecma_transforms_react::{react, Options as ReactOptions, RefreshOptions, Runtime};
use swc_ecma_transforms_typescript::typescript;

/// Développement (l'IDE : `jsxDEV`, React Refresh) ou production (l'application empaquetée).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentMode {
    Development,
    Production,
}

/// Le format du module rendu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleFormat {
    /// `import` / `export`.
    Esm,
    /// `require` / `exports`, pour une fabrique `function (require, module, exports)`.
    CommonJs,
}

/// Ce que la compilation rend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledComponent {
    /// Le module ES : ses `import` (`react`, `react/jsx-runtime`, les autres composants) restent
    /// à résoudre par la page, contre les libs déclarées.
    pub code: String,
    /// La source map (JSON v3) du module vers le TSX tel qu'écrit.
    pub source_map: String,
    /// blake3 du module : sa version.
    pub hash: String,
    /// Toutes les exportations sont des composants (un nom en majuscule, ce qu'en juge React
    /// Refresh) : le module peut s'accepter lui-même ; sinon la mise à jour remonte à ses
    /// importeurs.
    pub exports_only_components: bool,
    /// Les spécifiers importés, dans l'ordre du module et sans doublon : `react`,
    /// `react/jsx-dev-runtime` (posé par le JSX), `./bouton`… Les imports de types sont effacés.
    pub imports: Vec<String>,
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

/// Compile le TSX d'un composant. `template_id` nomme le gabarit (stable d'une édition à
/// l'autre : c'est le nom de famille de React Refresh).
pub fn compile_component(
    source: &str,
    template_id: &str,
    mode: ComponentMode,
    format: ModuleFormat,
) -> Result<CompiledComponent, ScriptError> {
    let dev = mode == ComponentMode::Development;
    let cm: Lrc<SourceMap> = Default::default();
    let collected = Collected::default();
    let handler = Handler::with_emitter(false, false, Box::new(collected.clone()));
    let fm = cm.new_source_file(
        FileName::Custom(format!("{template_id}.tsx")).into(),
        source.to_string(),
    );
    let comments = SingleThreadedComments::default();

    let sortie = GLOBALS.set(&Globals::new(), || {
        HANDLER.set(&handler, || -> Result<(String, String, bool, Vec<String>), ()> {
            let syntax = Syntax::Typescript(TsSyntax {
                tsx: true,
                decorators: true,
                ..Default::default()
            });
            let lexer = Lexer::new(syntax, EsVersion::latest(), StringInput::from(&*fm), Some(&comments));
            let mut parser = Parser::new_from(lexer);
            let lu = parser.parse_module();
            for e in parser.take_errors() {
                e.into_diagnostic(&handler).emit();
            }
            let module = match lu {
                Ok(m) => m,
                Err(e) => {
                    e.into_diagnostic(&handler).emit();
                    return Err(());
                }
            };
            if handler.has_errors() {
                return Err(());
            }
            let mut program = Program::Module(module);
            let unresolved_mark = Mark::new();
            let top_level_mark = Mark::new();
            let mut seulement_des_composants = true;
            let mut importes = Vec::new();
            HELPERS.set(&Helpers::new(false), || {
                program.mutate(&mut resolver(unresolved_mark, top_level_mark, true));
                program.mutate(&mut typescript::typescript(
                    Default::default(),
                    unresolved_mark,
                    top_level_mark,
                ));
                seulement_des_composants = exports_only_components(&program);
                program.mutate(&mut react(
                    cm.clone(),
                    Some(comments.clone()),
                    ReactOptions {
                        next: Some(true),
                        runtime: Some(Runtime::Automatic),
                        development: Some(dev),
                        refresh: dev.then(RefreshOptions::default),
                        ..Default::default()
                    },
                    top_level_mark,
                    unresolved_mark,
                ));
                importes = imported_specifiers(&program);
                if format == ModuleFormat::CommonJs {
                    program.mutate(&mut import_analyzer(ImportInterop::Swc, true));
                    program.mutate(&mut inject_helpers(unresolved_mark));
                    program.mutate(&mut common_js(
                        Resolver::Default,
                        unresolved_mark,
                        Default::default(),
                        FeatureFlag { support_block_scoping: true, support_arrow: true },
                    ));
                } else {
                    program.mutate(&mut inject_helpers(unresolved_mark));
                }
                program.mutate(&mut hygiene());
                program.mutate(&mut fixer(Some(&comments)));
            });
            if handler.has_errors() {
                return Err(());
            }
            let mut code = Vec::new();
            let mut correspondances = Vec::new();
            {
                let mut emitter = swc_ecma_codegen::Emitter {
                    cfg: swc_ecma_codegen::Config::default(),
                    comments: Some(&comments),
                    cm: cm.clone(),
                    wr: swc_ecma_codegen::text_writer::JsWriter::new(
                        cm.clone(),
                        "\n",
                        &mut code,
                        Some(&mut correspondances),
                    ),
                };
                if emitter.emit_program(&program).is_err() {
                    return Err(());
                }
            }
            let carte = cm.build_source_map(&correspondances, None, DefaultSourceMapGenConfig);
            let mut source_map = Vec::new();
            if carte.to_writer(&mut source_map).is_err() {
                return Err(());
            }
            let code = String::from_utf8(code).map_err(|_| ())?;
            let source_map = String::from_utf8(source_map).map_err(|_| ())?;
            Ok((code, source_map, seulement_des_composants, importes))
        })
    });

    let (mut code, source_map, exports_only_components, imports) = match sortie {
        Ok(fait) => fait,
        Err(()) => return Err(premiere_erreur(&cm, &collected)),
    };
    if dev {
        code.push_str(&refresh_bindings(template_id));
    }
    let hash = crate::hash::content_hash(&code);
    Ok(CompiledComponent { code, source_map, hash, exports_only_components, imports })
}

/// **Un module JavaScript en ESM, rendu en CommonJS** (19 octobre 2026) : pour un
/// fichier de lib que son paquet ne publie qu'en ESM, le registre de la page
/// n'exécutant que du CommonJS. Ni types, ni JSX, ni React Refresh : le code
/// tiers ne passe que par `common_js`. `Ok(None)` quand le fichier n'a ni
/// `import` ni `export` (il est déjà un script, rien à faire) — la détection est
/// celle du parse, pas du nom du fichier.
pub fn compile_module(source: &str, name: &str) -> Result<Option<String>, ScriptError> {
    let cm: Lrc<SourceMap> = Default::default();
    let collected = Collected::default();
    let handler = Handler::with_emitter(false, false, Box::new(collected.clone()));
    let fm = cm.new_source_file(FileName::Custom(name.to_string()).into(), source.to_string());
    let comments = SingleThreadedComments::default();
    let sortie = GLOBALS.set(&Globals::new(), || {
        HANDLER.set(&handler, || -> Result<Option<String>, ()> {
            let lexer = Lexer::new(Syntax::Es(EsSyntax::default()), EsVersion::latest(), StringInput::from(&*fm), Some(&comments));
            let mut parser = Parser::new_from(lexer);
            let lu = parser.parse_module();
            for e in parser.take_errors() {
                e.into_diagnostic(&handler).emit();
            }
            let module = match lu {
                Ok(m) => m,
                Err(e) => {
                    e.into_diagnostic(&handler).emit();
                    return Err(());
                }
            };
            if handler.has_errors() {
                return Err(());
            }
            if !module.body.iter().any(|i| matches!(i, ModuleItem::ModuleDecl(_))) {
                return Ok(None);
            }
            let mut program = Program::Module(module);
            let unresolved_mark = Mark::new();
            let top_level_mark = Mark::new();
            HELPERS.set(&Helpers::new(false), || {
                program.mutate(&mut resolver(unresolved_mark, top_level_mark, false));
                program.mutate(&mut import_analyzer(ImportInterop::Swc, true));
                program.mutate(&mut inject_helpers(unresolved_mark));
                program.mutate(&mut common_js(
                    Resolver::Default,
                    unresolved_mark,
                    Default::default(),
                    FeatureFlag { support_block_scoping: true, support_arrow: true },
                ));
                program.mutate(&mut hygiene());
                program.mutate(&mut fixer(Some(&comments)));
            });
            if handler.has_errors() {
                return Err(());
            }
            let mut code = Vec::new();
            {
                let mut emitter = swc_ecma_codegen::Emitter {
                    cfg: swc_ecma_codegen::Config::default(),
                    comments: Some(&comments),
                    cm: cm.clone(),
                    wr: swc_ecma_codegen::text_writer::JsWriter::new(cm.clone(), "\n", &mut code, None),
                };
                if emitter.emit_program(&program).is_err() {
                    return Err(());
                }
            }
            String::from_utf8(code).map(Some).map_err(|_| ())
        })
    });
    sortie.map_err(|()| premiere_erreur(&cm, &collected))
}

/// Les deux fonctions qui donnent à React Refresh le nom de famille `"<gabarit> <composant>"`.
fn refresh_bindings(template_id: &str) -> String {
    let id = serde_json::to_string(&format!("{template_id} ")).unwrap_or_else(|_| "\"\"".into());
    format!(
        "\nfunction $RefreshReg$(type, id) {{ const r = globalThis.__rag3weaverRefresh; if (r) r.register(type, {id} + id); }}\n\
         function $RefreshSig$() {{ const r = globalThis.__rag3weaverRefresh; return r ? r.signature() : (type) => type; }}\n"
    )
}

/// La première erreur de swc, avec sa ligne et sa colonne dans le TSX.
fn premiere_erreur(cm: &Lrc<SourceMap>, collected: &Collected) -> ScriptError {
    let first = collected.0.lock().ok().and_then(|all| all.first().cloned());
    match first {
        Some((message, span)) => {
            let (line, column) = match span {
                Some(span) => {
                    let at = cm.lookup_char_pos(span.lo);
                    (Some(at.line as u32), Some(at.col_display as u32 + 1))
                }
                None => (None, None),
            };
            ScriptError::new(ScriptErrorKind::Syntax, message).at(line, column)
        }
        None => ScriptError::new(ScriptErrorKind::Syntax, "le TSX ne se compile pas"),
    }
}

/// Les spécifiers d'`import` et de réexportation (`export … from`), dans l'ordre, sans doublon.
fn imported_specifiers(program: &Program) -> Vec<String> {
    let Program::Module(module) = program else { return Vec::new() };
    let mut vus = Vec::new();
    for item in &module.body {
        let ModuleItem::ModuleDecl(decl) = item else { continue };
        let src = match decl {
            ModuleDecl::Import(i) => Some(&i.src),
            ModuleDecl::ExportAll(e) => Some(&e.src),
            ModuleDecl::ExportNamed(n) => n.src.as_ref(),
            _ => None,
        };
        if let Some(src) = src {
            let s = src.value.to_string_lossy().to_string();
            if !vus.contains(&s) {
                vus.push(s);
            }
        }
    }
    vus
}

/// Un nom de composant pour React Refresh : il commence par une majuscule.
fn est_un_composant(nom: &str) -> bool {
    nom.starts_with(|c: char| c.is_ascii_uppercase())
}

/// Toutes les exportations du module (types effacés) sont-elles des composants ? Une
/// exportation par défaut anonyme, une réexportation entière (`export * from`) ou une valeur
/// en minuscule ne le sont pas.
fn exports_only_components(program: &Program) -> bool {
    let Program::Module(module) = program else { return false };
    let mut au_moins_une = false;
    for item in &module.body {
        let ModuleItem::ModuleDecl(decl) = item else { continue };
        let noms: Vec<String> = match decl {
            ModuleDecl::ExportDecl(e) => match &e.decl {
                Decl::Fn(f) => vec![f.ident.sym.to_string()],
                Decl::Class(c) => vec![c.ident.sym.to_string()],
                Decl::Var(v) => {
                    let mut noms = Vec::new();
                    for d in &v.decls {
                        match &d.name {
                            Pat::Ident(i) => noms.push(i.id.sym.to_string()),
                            _ => return false,
                        }
                    }
                    noms
                }
                _ => return false,
            },
            ModuleDecl::ExportDefaultDecl(e) => match &e.decl {
                DefaultDecl::Fn(f) => match &f.ident {
                    Some(i) => vec![i.sym.to_string()],
                    None => return false,
                },
                DefaultDecl::Class(c) => match &c.ident {
                    Some(i) => vec![i.sym.to_string()],
                    None => return false,
                },
                _ => return false,
            },
            ModuleDecl::ExportNamed(n) => {
                if n.src.is_some() {
                    return false;
                }
                let mut noms = Vec::new();
                for s in &n.specifiers {
                    match s {
                        ExportSpecifier::Named(s) => {
                            let nom = s.exported.as_ref().unwrap_or(&s.orig);
                            match nom {
                                ModuleExportName::Ident(i) => noms.push(i.sym.to_string()),
                                ModuleExportName::Str(_) => return false,
                            }
                        }
                        _ => return false,
                    }
                }
                noms
            }
            ModuleDecl::ExportDefaultExpr(_) | ModuleDecl::ExportAll(_) => return false,
            _ => continue,
        };
        for nom in noms {
            au_moins_une = true;
            if !est_un_composant(&nom) {
                return false;
            }
        }
    }
    au_moins_une
}

#[cfg(test)]
mod tests {
    use super::*;

    const COMPTEUR: &str = r#"import { useState } from "react";
import { Bouton } from "./bouton";

type Props = { libelle: string; depart?: number };

export function Compteur({ libelle, depart = 0 }: Props) {
  const [n, setN] = useState<number>(depart);
  return (
    <div className="compteur">
      <span>{libelle} : {n}</span>
      <Bouton onClick={() => setN(n + 1)}>+1</Bouton>
    </div>
  );
}
"#;

    #[test]
    fn un_meme_source_rend_le_meme_module() {
        let a = compile_component(COMPTEUR, "boutique/compteur", ComponentMode::Development, ModuleFormat::Esm).unwrap();
        let b = compile_component(COMPTEUR, "boutique/compteur", ComponentMode::Development, ModuleFormat::Esm).unwrap();
        assert_eq!(a, b, "deux compilations du même source diffèrent");
        assert_eq!(a.hash.len(), 64);
        let autre = compile_component(&COMPTEUR.replace("+1", "+2"), "boutique/compteur", ComponentMode::Development, ModuleFormat::Esm).unwrap();
        assert_ne!(a.hash, autre.hash, "un source changé doit changer le hash");
    }

    #[test]
    fn le_developpement_porte_react_refresh_au_nom_du_gabarit() {
        let c = compile_component(COMPTEUR, "boutique/compteur", ComponentMode::Development, ModuleFormat::Esm).unwrap();
        assert!(c.code.contains("$RefreshReg$("), "pas d'enregistrement Refresh :\n{}", c.code);
        assert!(c.code.contains("$RefreshSig$("), "pas de signature de hooks :\n{}", c.code);
        assert!(c.code.contains("\"Compteur\""), "le composant n'est pas enregistré sous son nom :\n{}", c.code);
        assert!(c.code.contains("\"boutique/compteur \""), "le nom de famille ne porte pas l'id du gabarit :\n{}", c.code);
        assert!(c.code.contains("react/jsx-dev-runtime"), "le JSX de développement attendu :\n{}", c.code);
        assert!(c.exports_only_components);
    }

    #[test]
    fn la_production_n_a_ni_refresh_ni_types() {
        let c = compile_component(COMPTEUR, "boutique/compteur", ComponentMode::Production, ModuleFormat::Esm).unwrap();
        assert!(!c.code.contains("$Refresh"), "Refresh en production :\n{}", c.code);
        assert!(c.code.contains("react/jsx-runtime"), "{}", c.code);
        assert!(!c.code.contains("Props") && !c.code.contains("useState<number>"), "des types restent :\n{}", c.code);
        assert!(c.code.contains("from \"./bouton\""), "l'import d'un autre composant reste à résoudre :\n{}", c.code);
    }

    #[test]
    fn la_source_map_vise_le_tsx() {
        let c = compile_component(COMPTEUR, "boutique/compteur", ComponentMode::Development, ModuleFormat::Esm).unwrap();
        let carte: serde_json::Value = serde_json::from_str(&c.source_map).unwrap();
        assert_eq!(carte["version"], 3);
        assert!(carte["sources"][0].as_str().unwrap_or("").contains("boutique/compteur.tsx"), "{carte}");
        assert!(!carte["mappings"].as_str().unwrap_or("").is_empty());
    }

    #[test]
    fn une_erreur_de_syntaxe_dit_sa_ligne() {
        // Une balise non fermée se lit jusqu'au `}` qui la trahit (ligne 3) : swc dit où il bute.
        let e = compile_component("export function A() {\n  return <div>;\n}\n", "a", ComponentMode::Development, ModuleFormat::Esm).unwrap_err();
        assert_eq!((e.kind.clone(), e.line, e.column), (ScriptErrorKind::Syntax, Some(3), Some(1)), "{e:?}");
        let e = compile_component("export function B() {\n  const x = ;\n  return <b/>;\n}\n", "b", ComponentMode::Development, ModuleFormat::Esm).unwrap_err();
        assert_eq!((e.kind.clone(), e.line), (ScriptErrorKind::Syntax, Some(2)), "{e:?}");
    }

    #[test]
    fn un_module_qui_exporte_autre_chose_qu_un_composant_le_dit() {
        let c = compile_component(
            "export const formater = (n: number) => `${n}`;\nexport function Etiquette() { return <b/>; }\n",
            "x",
            ComponentMode::Development,
            ModuleFormat::Esm,
        )
        .unwrap();
        assert!(!c.exports_only_components);
        let d = compile_component("export default function () { return <b/>; }\n", "y", ComponentMode::Development, ModuleFormat::Esm).unwrap();
        assert!(!d.exports_only_components, "une exportation par défaut anonyme n'a pas de nom de famille");
    }

    #[test]
    fn les_imports_sont_rendus_a_part_avec_celui_du_jsx() {
        let c = compile_component(COMPTEUR, "boutique/compteur", ComponentMode::Development, ModuleFormat::Esm).unwrap();
        assert_eq!(c.imports, vec!["react/jsx-dev-runtime", "react", "./bouton"], "{:?}", c.imports);
        let typee = compile_component(
            "import type { Props } from \"./types\";\nexport function A(p: Props) { return <b/>; }\n",
            "a",
            ComponentMode::Production,
            ModuleFormat::Esm,
        )
        .unwrap();
        assert_eq!(typee.imports, vec!["react/jsx-runtime"], "un import de type est effacé : {:?}", typee.imports);
    }

    #[test]
    fn le_commonjs_requiert_et_exporte_sans_import_ni_export() {
        let c = compile_component(COMPTEUR, "boutique/compteur", ComponentMode::Development, ModuleFormat::CommonJs).unwrap();
        for s in ["require(\"react\")", "require(\"react/jsx-dev-runtime\")", "require(\"./bouton\")"] {
            assert!(c.code.contains(s), "{s} manque :\n{}", c.code);
        }
        assert!(c.code.contains("exports"), "{}", c.code);
        assert!(!c.code.contains("import ") && !c.code.contains("export "), "il reste de l'ESM :\n{}", c.code);
        assert!(c.code.contains("$RefreshReg$("), "Refresh perdu en CommonJS :\n{}", c.code);
        // Un helper d'interop employé est défini dans le module (aucun import de helpers).
        for helper in ["_interop_require_default", "_interop_require_wildcard"] {
            if c.code.contains(&format!("{helper}(")) {
                assert!(c.code.contains(&format!("function {helper}")), "{helper} employé sans être défini :\n{}", c.code);
            }
        }
        let esm = compile_component(COMPTEUR, "boutique/compteur", ComponentMode::Development, ModuleFormat::Esm).unwrap();
        assert_ne!(c.hash, esm.hash);
        assert_eq!(c.imports, esm.imports);
    }

    #[test]
    fn le_commonjs_s_execute_avec_un_require_donne() {
        // Le module CommonJS, exécuté dans QuickJS avec un `require` qui rend un faux React :
        // le composant exporté se rend en un élément.
        let c = compile_component(COMPTEUR, "boutique/compteur", ComponentMode::Production, ModuleFormat::CommonJs).unwrap();
        let rt = rquickjs::Runtime::new().unwrap();
        let ctx = rquickjs::Context::full(&rt).unwrap();
        let rendu: String = ctx.with(|ctx| {
            let faux = r#"
                const libs = {
                  "react": { useState: (v) => [v, () => {}] },
                  "react/jsx-runtime": {
                    jsx: (t, p) => ({ t: typeof t === "string" ? t : t.name, p }),
                    jsxs: (t, p) => ({ t: typeof t === "string" ? t : t.name, p }),
                    Fragment: "F",
                  },
                  "./bouton": { Bouton: function Bouton(p) { return null; } },
                };
                const module = { exports: {} };
                (function (require, module, exports) {
            "#;
            let fin = r#"
                })((s) => libs[s], module, module.exports);
                JSON.stringify(module.exports.Compteur({ libelle: "n" }));
            "#;
            ctx.eval::<String, _>(format!("{faux}{}{fin}", c.code)).unwrap()
        });
        assert!(rendu.contains("\"t\":\"div\""), "{rendu}");
        assert!(rendu.contains("Bouton"), "{rendu}");
    }
}
