//! **Les modules d'un composant, prêts pour la page** (marche 3, lot 3).
//!
//! Le graphe part d'un composant : chaque module (un composant compilé par
//! le moteur `tsx` en CommonJS, ou un fichier d'une lib lu au magasin et
//! vérifié par son blake3) porte un id stable, son hash et la table de ses
//! `require` résolus par [`super::Components::resolve`]. La page les reçoit
//! enveloppés dans une fabrique, `__rag3weaver.define(id, hash, deps,
//! function (require, module, exports, process) { … })`, et les exécute par
//! le registre [`REGISTRE`] : des fabriques remplaçables à chaud (le lot 4),
//! ce qu'un module ES ne permet pas.
//!
//! Ids : `comp:<Nom>` pour un composant (stable d'une édition à l'autre : le
//! nom de famille de React Refresh) ; `lib:<paquet>@<version>/<fichier>` pour
//! un fichier de lib.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

use serde::Serialize;

use super::{Components, Importer, Mode, Resolved};
use crate::script::tsx::{compile_component, CompiledComponent, ComponentMode, ModuleFormat};

/// Un module du graphe.
#[derive(Debug, Clone, Serialize)]
pub struct Module {
    pub id: String,
    pub hash: String,
    /// Spécifier tel qu'écrit → id du module visé.
    pub deps: BTreeMap<String, String>,
    #[serde(skip)]
    pub code: String,
    /// Toutes ses exportations sont des composants (le lot 4 : il s'accepte lui-même).
    pub exports_only_components: bool,
}

/// Le graphe d'un composant : son module d'entrée, tous ses modules, et les
/// `require` qu'aucune table ne résout (dits, pas tus : le registre les
/// refusera en les nommant s'ils s'exécutent).
#[derive(Debug, Clone, Serialize)]
pub struct Graph {
    pub entry: String,
    /// Les modules de démarrage de la page, par le backend : `react`,
    /// `react-dom/client`.
    pub boot: BTreeMap<String, String>,
    pub modules: BTreeMap<String, Module>,
    pub unresolved: Vec<String>,
}

pub fn component_id(name: &str) -> String {
    format!("comp:{name}")
}

fn lib_id(package: &str, version: &str, file: &str) -> String {
    format!("lib:{package}@{version}/{file}")
}

fn id_de(r: &Resolved) -> String {
    match r {
        Resolved::Component(n) => component_id(n),
        Resolved::LibFile { package, version, file, .. } => lib_id(package, version, file),
    }
}

/// Les compilations, par empreinte du source, du nom et du mode : une page
/// rechargée ne recompile rien de ce qui n'a pas changé.
fn compiler(source: &str, name: &str, mode: Mode) -> Result<CompiledComponent, String> {
    static CACHE: OnceLock<Mutex<BTreeMap<String, CompiledComponent>>> = OnceLock::new();
    let cle = blake3::hash(format!("{name}\0{mode:?}\0{source}").as_bytes()).to_hex().to_string();
    let cache = CACHE.get_or_init(Default::default);
    if let Some(c) = cache.lock().unwrap_or_else(|p| p.into_inner()).get(&cle) {
        return Ok(c.clone());
    }
    let m = if mode == Mode::Development { ComponentMode::Development } else { ComponentMode::Production };
    let c = compile_component(source, name, m, ModuleFormat::CommonJs).map_err(|e| format!("{name} : {e}"))?;
    cache.lock().unwrap_or_else(|p| p.into_inner()).insert(cle, c.clone());
    Ok(c)
}

/// Les `require("…")` littéraux d'un fichier CommonJS de lib, dans l'ordre.
fn requires(code: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut reste = code;
    while let Some(i) = reste.find("require(") {
        let apres = &reste[i + "require(".len()..];
        let apres = apres.trim_start();
        let guillemet = apres.chars().next();
        if let Some(q @ ('"' | '\'' | '`')) = guillemet {
            if let Some(fin) = apres[1..].find(q) {
                let spec = &apres[1..1 + fin];
                if !spec.is_empty() && !out.iter().any(|s| s == spec) {
                    out.push(spec.to_string());
                }
            }
        }
        reste = &reste[i + "require(".len()..];
    }
    out
}

impl Components {
    /// **Le graphe des modules d'un composant**, depuis `backend_dir` (son
    /// dossier `components/` et son magasin). Un fichier de lib dont les
    /// octets ne redonnent pas leur hash est refusé : le magasin a été
    /// touché.
    pub fn module_graph(&self, backend_dir: &Path, entry: &str, mode: Mode) -> Result<Graph, String> {
        let c = self.components.get(entry).ok_or_else(|| format!("composant inconnu : {entry}"))?;
        let mut modules = BTreeMap::new();
        let mut unresolved = Vec::new();
        let mut a_faire: Vec<(String, Option<(String, String, String)>)> = vec![(c.name.clone(), None)];
        let mut boot = BTreeMap::new();
        for spec in ["react", "react-dom/client"] {
            match self.resolve(&Importer::Backend, spec, mode) {
                Ok(r) => {
                    if let Resolved::LibFile { package, file, .. } = &r {
                        a_faire.push((String::new(), Some((package.clone(), file.clone(), id_de(&r)))));
                    }
                    boot.insert(spec.to_string(), id_de(&r));
                }
                Err(e) => return Err(format!("l'aperçu demande {spec} dans libs.json : {e}")),
            }
        }
        while let Some((nom, lib)) = a_faire.pop() {
            let (id, hash, code, specs, seul, importeur): (String, String, String, Vec<String>, bool, Importer<'_>) = match &lib {
                None => {
                    let decl = &self.components[&nom];
                    let id = component_id(&nom);
                    if modules.contains_key(&id) {
                        continue;
                    }
                    let source = std::fs::read_to_string(backend_dir.join("components").join(&decl.source))
                        .map_err(|e| format!("{} : {e}", decl.source))?;
                    let compile = compiler(&source, &nom, mode)?;
                    let importeur = Importer::Component(self.components.get_key_value(&nom).unwrap().0.as_str());
                    (id, compile.hash, compile.code, compile.imports, compile.exports_only_components, importeur)
                }
                Some((package, file, id)) => {
                    if modules.contains_key(id) {
                        continue;
                    }
                    let lib = &self.libs[package];
                    let hash = lib.files[file].clone();
                    let octets = std::fs::read(super::store_path(backend_dir, &hash)).map_err(|e| format!("{package}/{file} : {e}"))?;
                    if blake3::hash(&octets).to_hex().as_str() != hash {
                        return Err(format!("{package}/{file} : le magasin ne redonne pas son hash ({hash}) — il a été touché"));
                    }
                    let code = String::from_utf8_lossy(&octets).into_owned();
                    let specs = requires(&code);
                    let paquet = self.libs.get_key_value(package).map(|(k, _)| k.as_str()).unwrap();
                    (id.clone(), hash, code, specs, false, Importer::LibFile { package: paquet, file: file.as_str() })
                }
            };
            let mut deps = BTreeMap::new();
            for spec in specs {
                match self.resolve(&importeur, &spec, mode) {
                    Ok(r) => {
                        let cible = id_de(&r);
                        match &r {
                            Resolved::Component(n) => a_faire.push((n.clone(), None)),
                            Resolved::LibFile { package, file, .. } => a_faire.push((String::new(), Some((package.clone(), file.clone(), cible.clone())))),
                        }
                        deps.insert(spec, cible);
                    }
                    Err(e) => unresolved.push(format!("{id} : {e}")),
                }
            }
            modules.insert(id.clone(), Module { id, hash, deps, code, exports_only_components: seul });
        }
        Ok(Graph { entry: component_id(entry), boot, modules, unresolved })
    }
}

impl Graph {
    /// **Le script des modules** : chaque module enveloppé dans sa fabrique,
    /// enregistré auprès du registre de la page.
    pub fn script(&self) -> String {
        let mut out = String::new();
        for m in self.modules.values() {
            out.push_str(&define(m));
        }
        out
    }
}

/// Un module enveloppé dans sa fabrique.
pub fn define(m: &Module) -> String {
    format!(
        "__rag3weaver.define({}, {}, {}, function (require, module, exports, process) {{\n{}\n}});\n",
        serde_json::to_string(&m.id).unwrap(),
        serde_json::to_string(&m.hash).unwrap(),
        serde_json::to_string(&m.deps).unwrap(),
        m.code
    )
}

/// **Le registre de modules de la page** : des fabriques, un cache, et un
/// `require` qui suit la table de chaque module. Un module redéfini (le lot
/// 4) remplace sa fabrique et sort du cache ; ses importeurs sont connus.
pub const REGISTRE: &str = r#"(function () {
  if (globalThis.__rag3weaver) return;
  const fabriques = {}, hashes = {}, tables = {}, cache = {}, importeurs = {};
  const R = globalThis.__rag3weaver = {
    mode: "development",
    define(id, hash, deps, fabrique) {
      fabriques[id] = fabrique; hashes[id] = hash; tables[id] = deps;
      for (const cible of Object.values(deps)) (importeurs[cible] ??= new Set()).add(id);
      delete cache[id];
    },
    hash(id) { return hashes[id]; },
    importeurs(id) { return [...(importeurs[id] ?? [])]; },
    oublier(id) { delete cache[id]; },
    require(id) {
      if (cache[id]) return cache[id].exports;
      const fabrique = fabriques[id];
      if (!fabrique) throw new Error("module inconnu du registre : " + id);
      const module = { exports: {} };
      cache[id] = module;
      const table = tables[id];
      const requis = (spec) => {
        const cible = table[spec];
        if (!cible) throw new Error(id + " requiert « " + spec + " », que la table ne résout pas");
        return R.require(cible);
      };
      try {
        fabrique(requis, module, module.exports, { env: { NODE_ENV: R.mode } });
      } catch (e) { delete cache[id]; throw e; }
      return module.exports;
    },
  };
})();
"#;
