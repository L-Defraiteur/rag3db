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

/// **Un fichier de lib, en CommonJS** : tel quel s'il n'a ni `import` ni
/// `export`, rendu par `compile_module` sinon ; mis en cache par son hash (le
/// magasin est rangé par contenu).
fn module_en_commonjs(code: &str, hash: &str, nom: &str) -> Result<String, String> {
    static CACHE: OnceLock<Mutex<BTreeMap<String, String>>> = OnceLock::new();
    // Sans `import` ni `export` dans le texte, inutile de parser (le cas de
    // tout fichier `.cjs`).
    if !code.contains("import") && !code.contains("export") {
        return Ok(code.to_string());
    }
    let cache = CACHE.get_or_init(Default::default);
    if let Some(c) = cache.lock().unwrap_or_else(|p| p.into_inner()).get(hash) {
        return Ok(c.clone());
    }
    let rendu = match crate::script::tsx::compile_module(code, nom) {
        Ok(Some(cjs)) => cjs,
        Ok(None) => code.to_string(),
        // Un script CommonJS que le parseur d'ESM refuse (un `return` au premier
        // niveau, un `with`…) n'est pas un module : il reste tel quel.
        Err(_) if !code.contains("export ") && !code.contains("import ") => code.to_string(),
        Err(e) => return Err(format!("{nom} : un module ESM que la page ne sait pas rendre en CommonJS : {e}")),
    };
    cache.lock().unwrap_or_else(|p| p.into_inner()).insert(hash.to_string(), rendu.clone());
    Ok(rendu)
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

/// Le runtime de React Refresh, pris au démarrage de la page quand le backend
/// le range.
pub const REFRESH: &str = "react-refresh/runtime";

impl Components {
    /// **Le graphe des modules d'un composant**, depuis `backend_dir` (son
    /// dossier `components/` et son magasin). Un fichier de lib dont les
    /// octets ne redonnent pas leur hash est refusé : le magasin a été
    /// touché.
    pub fn module_graph(&self, backend_dir: &Path, entry: &str, mode: Mode) -> Result<Graph, String> {
        let c = self.components.get(entry).ok_or_else(|| format!("composant inconnu : {entry}"))?;
        // **React Refresh**, s'il est rangé : le rechargement à chaud garde
        // l'état des composants. Sans lui, la page se recharge entière.
        let facultatifs: &[&str] = if mode == Mode::Development { &[REFRESH] } else { &[] };
        self.graph_from(backend_dir, Some(c.name.clone()), &["react", "react-dom/client"], facultatifs, mode, "l'aperçu demande")
    }

    /// **Le graphe des modules de quelques libs**, par leurs spécifiers
    /// (`@codemirror/view`…), résolus comme le backend les importe : la page de
    /// l'IDE s'en sert pour son éditeur, sans composant ni React. L'entrée est
    /// le premier spécifier ; tous sont au démarrage (`boot`).
    pub fn module_graph_of(&self, backend_dir: &Path, specifiers: &[&str], mode: Mode) -> Result<Graph, String> {
        if specifiers.is_empty() {
            return Err("un graphe de libs demande au moins un spécifier".into());
        }
        self.graph_from(backend_dir, None, specifiers, &[], mode, "le graphe demande")
    }

    /// Le parcours commun : un composant de départ (ou aucun), les libs
    /// exigées au démarrage, celles qui y viennent si elles sont rangées.
    fn graph_from(
        &self,
        backend_dir: &Path,
        component: Option<String>,
        required: &[&str],
        optional: &[&str],
        mode: Mode,
        pourquoi: &str,
    ) -> Result<Graph, String> {
        let mut modules = BTreeMap::new();
        let mut unresolved = Vec::new();
        let mut a_faire: Vec<(String, Option<(String, String, String)>)> = Vec::new();
        if let Some(c) = &component {
            a_faire.push((c.clone(), None));
        }
        let mut boot = BTreeMap::new();
        for (spec, exige) in required.iter().map(|s| (*s, true)).chain(optional.iter().map(|s| (*s, false))) {
            match self.resolve(&Importer::Backend, spec, mode) {
                Ok(r) => {
                    if let Resolved::LibFile { package, file, .. } = &r {
                        a_faire.push((String::new(), Some((package.clone(), file.clone(), id_de(&r)))));
                    }
                    boot.insert(spec.to_string(), id_de(&r));
                }
                Err(e) if exige => return Err(format!("{pourquoi} {spec} dans libs.json : {e}")),
                Err(_) => {}
            }
        }
        let entry = match &component {
            Some(c) => component_id(c),
            None => boot[required[0]].clone(),
        };
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
                    let brut = String::from_utf8_lossy(&octets).into_owned();
                    // Un fichier de lib en ESM (un paquet sans build CommonJS) se
                    // rend en CommonJS pour le registre ; un script reste tel quel.
                    let code = module_en_commonjs(&brut, &hash, &format!("{package}/{file}"))?;
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
        Ok(Graph { entry, boot, modules, unresolved })
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
    // Les modules redéfinis avec un hash neuf depuis la dernière mise à
    // jour : la page les reprend (rechargement à chaud, lot 4).
    changes: [],
    define(id, hash, deps, fabrique) {
      if (fabriques[id] && hashes[id] === hash) return;
      if (fabriques[id]) R.changes.push(id);
      fabriques[id] = fabrique; hashes[id] = hash; tables[id] = deps;
      for (const cible of Object.values(deps)) (importeurs[cible] ??= new Set()).add(id);
      delete cache[id];
    },
    hash(id) { return hashes[id]; },
    importeurs(id) { return [...(importeurs[id] ?? [])]; },
    oublier(id) { delete cache[id]; },
    // **La mise à jour à chaud** : de chaque module changé, remonter les
    // importeurs jusqu'à une frontière (un module qui n'exporte que des
    // composants, que React Refresh sait remplacer en gardant l'état). Ce qui
    // est traversé est oublié, chaque frontière atteinte est réexécutée. Un
    // chemin qui arrive en haut sans frontière (une lib, l'entrée qui exporte
    // autre chose) demande de recharger la page : rend { recharger }.
    mettre_a_jour(frontieres) {
      const changes = R.changes.splice(0);
      const bornes = new Set(frontieres), relances = new Set(), vus = new Set(), pile = [...changes];
      while (pile.length) {
        const id = pile.pop();
        if (vus.has(id)) continue;
        vus.add(id);
        if (bornes.has(id)) { relances.add(id); continue; }
        const haut = R.importeurs(id);
        if (!haut.length) return { recharger: id + " a changé et rien au-dessus ne le reprend", changes };
        delete cache[id];
        pile.push(...haut);
      }
      for (const id of relances) { delete cache[id]; R.require(id); }
      return { relances: [...relances], changes };
    },
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

// ─── Le magasin réduit à ce que le graphe atteint ───────────────────────────

/// Ce qu'une réduction du magasin a fait : fichiers et octets rangés avant et
/// après, et les objets du magasin effacés.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Pruned {
    pub files_before: usize,
    pub bytes_before: u64,
    pub files_after: usize,
    pub bytes_after: u64,
    pub objects_removed: usize,
}

/// Un fichier qu'un paquet garde toujours : sa licence, quel que soit son nom.
fn licence(nom_complet: &str) -> bool {
    let nom = nom_complet.rsplit('/').next().unwrap_or(nom_complet).to_ascii_lowercase();
    nom.starts_with("license") || nom.starts_with("licence") || nom.starts_with("copying")
}

impl Components {
    /// **Réduit le magasin à ce que le graphe atteint** (19 octobre 2026) : les
    /// fichiers de chaque lib que le graphe de modules atteint depuis les
    /// entrées — les spécifiers donnés, sinon chaque composant du backend —, en
    /// développement comme en production, plus la licence de chaque paquet.
    /// `libs.json` est réécrit, et les objets du magasin que plus rien ne désigne
    /// sont effacés. Un graphe qui ne se résout pas entier refuse : on ne réduit
    /// pas un magasin d'après un graphe faux.
    pub fn prune_store(&self, backend_dir: &Path, entries: &[&str]) -> Result<Pruned, String> {
        let mut atteints: BTreeMap<&str, std::collections::BTreeSet<String>> = BTreeMap::new();
        let mut graphes = Vec::new();
        for mode in [Mode::Development, Mode::Production] {
            if entries.is_empty() {
                if self.components.is_empty() {
                    return Err("aucun composant ni spécifier : rien ne dit ce que le magasin doit garder".into());
                }
                for nom in self.components.keys() {
                    graphes.push(self.module_graph(backend_dir, nom, mode)?);
                }
            } else {
                graphes.push(self.module_graph_of(backend_dir, entries, mode)?);
            }
        }
        for g in &graphes {
            if !g.unresolved.is_empty() {
                return Err(format!("le graphe ne se résout pas entier, le magasin n'est pas réduit : {}", g.unresolved.join(" ; ")));
            }
            for id in g.modules.keys() {
                for (paquet, lib) in &self.libs {
                    if let Some(fichier) = id.strip_prefix(&format!("lib:{paquet}@{}/", lib.version)) {
                        atteints.entry(paquet.as_str()).or_default().insert(fichier.to_string());
                    }
                }
            }
        }
        let taille = |hash: &str| std::fs::metadata(super::store_path(backend_dir, hash)).map(|m| m.len()).unwrap_or(0);
        let (mut files_before, mut bytes_before, mut files_after, mut bytes_after) = (0, 0, 0, 0);
        let mut libs = self.libs.clone();
        for (paquet, lib) in libs.iter_mut() {
            files_before += lib.files.len();
            bytes_before += lib.files.values().map(|h| taille(h)).sum::<u64>();
            let garde = atteints.get(paquet.as_str());
            lib.files.retain(|f, _| licence(f) || garde.is_some_and(|g| g.contains(f)));
            files_after += lib.files.len();
            bytes_after += lib.files.values().map(|h| taille(h)).sum::<u64>();
        }
        let gardes: std::collections::BTreeSet<&String> = libs.values().flat_map(|l| l.files.values()).collect();
        let mut objects_removed = 0;
        if let Ok(entrees) = std::fs::read_dir(backend_dir.join(super::STORE_DIR)) {
            for e in entrees.flatten() {
                let nom = e.file_name().to_string_lossy().into_owned();
                if !gardes.contains(&nom) && std::fs::remove_file(e.path()).is_ok() {
                    objects_removed += 1;
                }
            }
        }
        let texte = serde_json::to_string_pretty(&libs).map_err(|e| e.to_string())? + "\n";
        std::fs::write(backend_dir.join(super::LIBS_FILE), texte).map_err(|e| e.to_string())?;
        Ok(Pruned { files_before, bytes_before, files_after, bytes_after, objects_removed })
    }
}
