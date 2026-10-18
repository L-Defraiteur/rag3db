//! **Les composants d'interface, déclarés comme des nœuds** (marche 3 de
//! l'IDE, lot 2 ; étude de codeparsers,
//! `docs/10-octobre-2026-18h20/codeparsers/07-un-composant-react-est-un-gabarit.md`).
//!
//! Un composant est un gabarit : `components/<nom>.component.json` à côté de
//! son source (`.tsx`, `.jsx`, `.ts`, `.js`). Ses **props sont des ports
//! d'entrée**, ses événements (`on*`) **des ports de sortie**, avec le même
//! JSON Schema que les nœuds scriptés (`nodes/*.node.json`) :
//!
//! ```json
//! {
//!   "name": "FicheProduit",
//!   "source": "fiche_produit.tsx",
//!   "inputs":  { "produit": { "schema": { "type": "object" }, "required": true } },
//!   "outputs": { "onEnregistrer": { "schema": { "type": "object" } } },
//!   "libs":    { "react": "19.2.0", "react-dom": "19.2.0" }
//! }
//! ```
//!
//! **Les bibliothèques sont déclarées et rangées par contenu.** `libs.json`, au
//! niveau du backend, nomme chaque paquet **une fois** — c'est ce qui garantit
//! une seule instance de React : deux versions d'un même paquet ne
//! s'écrivent pas —, avec sa version exacte, ses fichiers et leur blake3, sa
//! table `exports` et ses dépendances. Les octets vivent dans le magasin,
//! `<backend>/.libs/<blake3>` (caché, donc hors des déclarations). Le
//! magasin se remplit **sur commande seulement** ([`add_lib`], depuis un
//! paquet déballé) ; ensuite tout se sert de lui, hors réseau.
//!
//! **La résolution est une table, jamais le disque** ([`Components::resolve`]) :
//! un spécifier relatif d'un composant mène à un autre composant ; un
//! spécifier nu à la lib que l'importeur déclare, par sa table `exports`
//! (conditions `browser`, `development` ou `production`, `import`, `default`),
//! puis à un hash. Ce qui n'est pas déclaré est refusé en le nommant.

use std::collections::BTreeMap;
use std::path::{Component as PathComponent, Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::dataflow::scripted_node::DeclaredPort;

/// Le suffixe d'une déclaration de composant.
pub const SUFFIX: &str = ".component.json";
/// Le fichier des bibliothèques, au niveau du backend.
pub const LIBS_FILE: &str = "libs.json";
/// Le magasin par contenu, au niveau du backend (caché : hors des déclarations).
pub const STORE_DIR: &str = ".libs";

// ─── Les bibliothèques ──────────────────────────────────────────────────────

/// Une bibliothèque déclarée dans `libs.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lib {
    pub version: String,
    /// Chemin dans le paquet (sans `./`) → blake3 de ses octets.
    pub files: BTreeMap<String, String>,
    /// La table `exports` du `package.json`, telle quelle (absente : `main`).
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub exports: Value,
    /// `main` du `package.json`, quand il n'y a pas d'`exports`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub main: Option<String>,
    /// Les paquets que celui-ci importe, avec leur version exacte : déclarés
    /// eux aussi dans `libs.json`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub dependencies: BTreeMap<String, String>,
    /// Les dépendances pairs (`peerDependencies`), avec leur intervalle tel
    /// qu'écrit : un pair se résout vers **la** copie que le backend range,
    /// quelle que soit sa version (`react-dom` importe le `react` de l'hôte —
    /// une seule instance). Un pair absent de `libs.json` se dit à l'import.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub peers: BTreeMap<String, String>,
}

/// `libs.json` : paquet → sa bibliothèque.
pub type Libs = BTreeMap<String, Lib>;

/// Lit `libs.json` (absent : aucune bibliothèque).
pub fn read_libs(backend_dir: &Path) -> Result<Libs, String> {
    let chemin = backend_dir.join(LIBS_FILE);
    match std::fs::read_to_string(&chemin) {
        Ok(t) => serde_json::from_str(&t).map_err(|e| format!("{LIBS_FILE} : {e}")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Libs::new()),
        Err(e) => Err(format!("{LIBS_FILE} : {e}")),
    }
}

/// Le chemin d'un objet du magasin.
pub fn store_path(backend_dir: &Path, hash: &str) -> PathBuf {
    backend_dir.join(STORE_DIR).join(hash)
}

/// **Range un paquet déballé dans le magasin, et le déclare** : chaque fichier
/// haché (blake3) et copié sous son hash, l'entrée de `libs.json` écrite d'après
/// le `package.json` (nom, version, `exports`, `main`, `dependencies` à leur
/// version exacte). C'est la commande explicite qui remplit le magasin : rien
/// d'autre n'y écrit, et rien ne télécharge en silence. Un paquet déjà
/// déclaré sous une autre version est remplacé, et la réponse le dit : un
/// backend n'en garde qu'une.
pub fn add_lib(backend_dir: &Path, package_dir: &Path) -> Result<String, String> {
    let pj: Value = serde_json::from_str(
        &std::fs::read_to_string(package_dir.join("package.json")).map_err(|e| format!("{} : package.json : {e}", package_dir.display()))?,
    )
    .map_err(|e| format!("package.json : {e}"))?;
    let nom = pj["name"].as_str().ok_or("package.json sans « name »")?.to_string();
    let version = pj["version"].as_str().ok_or("package.json sans « version »")?.to_string();
    let mut files = BTreeMap::new();
    let mut a_voir = vec![package_dir.to_path_buf()];
    while let Some(dir) = a_voir.pop() {
        for entree in std::fs::read_dir(&dir).map_err(|e| format!("{} : {e}", dir.display()))? {
            let chemin = entree.map_err(|e| e.to_string())?.path();
            let nom_f = chemin.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if chemin.is_dir() {
                if nom_f != "node_modules" && !nom_f.starts_with('.') {
                    a_voir.push(chemin);
                }
                continue;
            }
            let octets = std::fs::read(&chemin).map_err(|e| format!("{} : {e}", chemin.display()))?;
            let hash = blake3::hash(&octets).to_hex().to_string();
            let cible = store_path(backend_dir, &hash);
            if !cible.exists() {
                std::fs::create_dir_all(cible.parent().unwrap()).map_err(|e| e.to_string())?;
                let provisoire = cible.with_extension("part");
                std::fs::write(&provisoire, &octets).map_err(|e| e.to_string())?;
                std::fs::rename(&provisoire, &cible).map_err(|e| e.to_string())?;
            }
            let relatif = chemin.strip_prefix(package_dir).unwrap().to_string_lossy().replace('\\', "/");
            files.insert(relatif, hash);
        }
    }
    let dependencies = pj["dependencies"]
        .as_object()
        .map(|o| o.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").trim_start_matches(['^', '~', '=']).to_string())).collect())
        .unwrap_or_default();
    let peers = pj["peerDependencies"]
        .as_object()
        .map(|o| o.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string())).collect())
        .unwrap_or_default();
    let lib = Lib {
        version: version.clone(),
        files,
        exports: pj["exports"].clone(),
        main: pj["main"].as_str().map(str::to_string),
        dependencies,
        peers,
    };
    let mut libs = read_libs(backend_dir)?;
    let avant = libs.insert(nom.clone(), lib).map(|l| l.version);
    let texte = serde_json::to_string_pretty(&libs).map_err(|e| e.to_string())? + "\n";
    std::fs::write(backend_dir.join(LIBS_FILE), texte).map_err(|e| e.to_string())?;
    Ok(match avant {
        Some(v) if v != version => format!("{nom}@{version} rangé ; remplace {nom}@{v} (un backend n'en garde qu'une version)"),
        _ => format!("{nom}@{version} rangé"),
    })
}

// ─── Les composants ─────────────────────────────────────────────────────────

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ComponentFile {
    name: String,
    source: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    inputs: Map<String, Value>,
    #[serde(default)]
    outputs: Map<String, Value>,
    #[serde(default)]
    libs: BTreeMap<String, String>,
    /// Les outils que ses effets appellent (marche 3, niveau 2).
    #[serde(default)]
    effets: Vec<String>,
    /// Son graphe de clic, relatif à `components/` (`<nom>.clic.mmd`).
    #[serde(default)]
    clic: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PortFile {
    schema: Value,
    #[serde(default)]
    required: bool,
}

/// Un composant déclaré.
#[derive(Debug, Clone)]
pub struct ComponentDecl {
    pub name: String,
    pub description: String,
    /// Le source, relatif au dossier `components/`.
    pub source: String,
    pub inputs: Vec<DeclaredPort>,
    pub outputs: Vec<DeclaredPort>,
    /// Paquet → version exacte, déclarée dans `libs.json`.
    pub libs: BTreeMap<String, String>,
    /// Les outils que ses effets appellent, chacun appelable d'un écran.
    pub effets: Vec<String>,
    /// Son graphe de clic validé (`dataflow::clic`), relatif à `components/`.
    pub clic: Option<String>,
}

/// **Ce que le backend permet à un écran** : ses outils exposés, avec ce que chacun permet
/// (`policy.ecran`), et le lieu des cases de portée `hôte` (`ui_state`). Le contexte contre
/// lequel se vérifient les effets et le graphe de clic d'un composant.
#[derive(Debug, Clone, Default)]
pub struct EcranDuBackend {
    pub outils: BTreeMap<String, Option<crate::dataflow::clic::Ecran>>,
    pub etat_hote: Option<String>,
}

/// Ce que compile un mode : les conditions d'`exports` qu'il prend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Development,
    Production,
}

/// Qui importe : un composant (par son nom), un fichier d'une lib, ou le
/// backend lui-même (le démarrage de la page d'aperçu : `react`,
/// `react-dom/client`, pris dans `libs.json`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Importer<'a> {
    Component(&'a str),
    LibFile { package: &'a str, file: &'a str },
    Backend,
}

/// Où mène un import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolved {
    /// Un autre composant du backend.
    Component(String),
    /// Un fichier d'une lib, par son hash.
    LibFile { package: String, version: String, file: String, hash: String },
}

/// **Les composants d'un backend et ses bibliothèques**, vérifiés ensemble.
#[derive(Debug, Clone, Default)]
pub struct Components {
    pub components: BTreeMap<String, ComponentDecl>,
    pub libs: Libs,
}

impl Components {
    /// **Découvre et vérifie** : chaque `components/*.component.json` (son
    /// source présent, ses ports en JSON Schema valide, ses libs déclarées à
    /// la même version), `libs.json` (chaque dépendance déclarée à sa
    /// version, chaque fichier présent au magasin). Une erreur par défaut,
    /// toutes rendues ensemble.
    pub fn load(backend_dir: &Path) -> Result<Self, Vec<String>> {
        Self::load_in(backend_dir, &EcranDuBackend::default())
    }

    /// La même, contre ce que le backend permet à un écran : les effets et le graphe de clic
    /// de chaque composant s'y vérifient ([`EcranDuBackend`]).
    pub fn load_in(backend_dir: &Path, ecran: &EcranDuBackend) -> Result<Self, Vec<String>> {
        let mut errors = Vec::new();
        let libs = match read_libs(backend_dir) {
            Ok(l) => l,
            Err(e) => return Err(vec![e]),
        };
        for (nom, lib) in &libs {
            for (dep, version) in &lib.dependencies {
                match libs.get(dep) {
                    None => errors.push(format!("{LIBS_FILE} : {nom} dépend de {dep}@{version}, qui n'est pas déclaré")),
                    Some(d) if &d.version != version => errors.push(format!(
                        "{LIBS_FILE} : {nom} dépend de {dep}@{version}, déclaré en {} (une seule version par paquet)",
                        d.version
                    )),
                    _ => {}
                }
            }
            for (fichier, hash) in &lib.files {
                if !store_path(backend_dir, hash).exists() {
                    errors.push(format!("{LIBS_FILE} : {nom}/{fichier} absent du magasin ({STORE_DIR}/{hash}) — le ranger par la commande des libs"));
                }
            }
        }
        let dir = backend_dir.join("components");
        let mut components = BTreeMap::new();
        let mut chemins: Vec<PathBuf> = match std::fs::read_dir(&dir) {
            Ok(e) => e.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.to_string_lossy().ends_with(SUFFIX)).collect(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(e) => return Err(vec![format!("components/ : {e}")]),
        };
        chemins.sort();
        for chemin in chemins {
            match read_component(&chemin, &dir, &libs, ecran) {
                Ok(c) => {
                    if components.contains_key(&c.name) {
                        errors.push(format!("components/ : le composant « {} » est déclaré deux fois", c.name));
                    } else {
                        components.insert(c.name.clone(), c);
                    }
                }
                Err(e) => errors.push(e),
            }
        }
        if errors.is_empty() {
            Ok(Self { components, libs })
        } else {
            Err(errors)
        }
    }

    /// **Résout un import** : relatif d'un composant → un autre composant ;
    /// relatif d'un fichier de lib → un fichier de la même lib ; nu → la lib
    /// que l'importeur déclare, par ses `exports`. Refusé en le nommant sinon.
    pub fn resolve(&self, importer: &Importer<'_>, specifier: &str, mode: Mode) -> Result<Resolved, String> {
        let relatif = specifier.starts_with("./") || specifier.starts_with("../");
        match importer {
            Importer::Component(nom) => {
                let c = self.components.get(*nom).ok_or_else(|| format!("composant inconnu : {nom}"))?;
                if relatif {
                    let vise = normaliser(&parent_de(&c.source), specifier)
                        .ok_or_else(|| format!("{nom} : « {specifier} » sort du dossier components/"))?;
                    let sans_ext = |s: &str| s.rsplit_once('.').map(|(a, _)| a.to_string()).unwrap_or_else(|| s.to_string());
                    return self
                        .components
                        .values()
                        .find(|x| x.source == vise || sans_ext(&x.source) == vise)
                        .map(|x| Resolved::Component(x.name.clone()))
                        .ok_or_else(|| format!("{nom} importe « {specifier} » : aucun composant déclaré n'a ce source"));
                }
                let (paquet, sous) = parse_package_name(specifier);
                let version = c.libs.get(paquet).ok_or_else(|| {
                    format!("{nom} importe « {specifier} » : le paquet « {paquet} » n'est pas dans ses libs (component.json)")
                })?;
                self.dans_la_lib(paquet, version, &sous, mode, specifier)
            }
            Importer::Backend => {
                if relatif {
                    return Err(format!("le backend n'importe pas de relatif (« {specifier} »)"));
                }
                let (paquet, sous) = parse_package_name(specifier);
                let lib = self.libs.get(paquet).ok_or_else(|| format!("« {paquet} » n'est pas déclaré dans {LIBS_FILE}"))?;
                self.dans_la_lib(paquet, &lib.version, &sous, mode, specifier)
            }
            Importer::LibFile { package, file } => {
                let lib = self.libs.get(*package).ok_or_else(|| format!("lib inconnue : {package}"))?;
                if relatif {
                    let vise = normaliser(&parent_de(file), specifier)
                        .ok_or_else(|| format!("{package}/{file} : « {specifier} » sort du paquet"))?;
                    return fichier_de(lib, &vise)
                        .map(|(f, hash)| Resolved::LibFile { package: package.to_string(), version: lib.version.clone(), file: f, hash })
                        .ok_or_else(|| format!("{package}/{file} importe « {specifier} » : absent du paquet"));
                }
                let (paquet, sous) = parse_package_name(specifier);
                if paquet == *package {
                    return self.dans_la_lib(paquet, &lib.version, &sous, mode, specifier);
                }
                if let Some(version) = lib.dependencies.get(paquet) {
                    return self.dans_la_lib(paquet, version, &sous, mode, specifier);
                }
                if lib.peers.contains_key(paquet) {
                    let pair = self.libs.get(paquet).ok_or_else(|| {
                        format!("{package}/{file} importe « {specifier} » : le pair « {paquet} » n'est pas déclaré dans {LIBS_FILE}")
                    })?;
                    return self.dans_la_lib(paquet, &pair.version, &sous, mode, specifier);
                }
                Err(format!("{package}/{file} importe « {specifier} » : « {paquet} » n'est ni dans ses dependencies ni dans ses peerDependencies"))
            }
        }
    }

    fn dans_la_lib(&self, paquet: &str, version: &str, sous: &str, mode: Mode, specifier: &str) -> Result<Resolved, String> {
        let lib = self.libs.get(paquet).ok_or_else(|| format!("« {paquet} » n'est pas déclaré dans {LIBS_FILE}"))?;
        if lib.version != version {
            return Err(format!("« {specifier} » veut {paquet}@{version}, {LIBS_FILE} déclare {}", lib.version));
        }
        let cible = cible_exports(lib, sous, mode).ok_or_else(|| format!("« {specifier} » : {paquet}@{version} n'exporte pas « {sous} »"))?;
        let vise = normaliser("", &cible).ok_or_else(|| format!("« {specifier} » : cible hors du paquet ({cible})"))?;
        fichier_de(lib, &vise)
            .map(|(f, hash)| Resolved::LibFile { package: paquet.to_string(), version: version.to_string(), file: f, hash })
            .ok_or_else(|| format!("« {specifier} » mène à {paquet}/{vise}, absent du paquet"))
    }
}

fn read_component(chemin: &Path, dir: &Path, libs: &Libs, ecran: &EcranDuBackend) -> Result<ComponentDecl, String> {
    let montre = format!("components/{}", chemin.file_name().unwrap().to_string_lossy());
    let texte = std::fs::read_to_string(chemin).map_err(|e| format!("{montre} : {e}"))?;
    let f: ComponentFile = serde_json::from_str(&texte).map_err(|e| format!("{montre} : {e}"))?;
    let source = Path::new(&f.source);
    if source.is_absolute() || source.components().any(|c| !matches!(c, PathComponent::Normal(_))) {
        return Err(format!("{montre} : le source « {} » doit être dans components/", f.source));
    }
    if !matches!(source.extension().and_then(|e| e.to_str()), Some("tsx" | "jsx" | "ts" | "js")) {
        return Err(format!("{montre} : le source « {} » n'est ni .tsx, ni .jsx, ni .ts, ni .js", f.source));
    }
    if !dir.join(source).is_file() {
        return Err(format!("{montre} : le source « {} » est absent", f.source));
    }
    let ports = |cote: &str, map: Map<String, Value>| -> Result<Vec<DeclaredPort>, String> {
        map.into_iter()
            .map(|(nom, v)| {
                let p: PortFile = serde_json::from_value(v).map_err(|e| format!("{montre} : port {cote} « {nom} » : {e}"))?;
                jsonschema::validator_for(&p.schema).map_err(|e| format!("{montre} : port {cote} « {nom} » : schéma invalide : {e}"))?;
                Ok(DeclaredPort { name: nom, schema: p.schema, required: p.required })
            })
            .collect()
    };
    let inputs = ports("d'entrée", f.inputs)?;
    let outputs = ports("de sortie", f.outputs)?;
    if let Some(o) = outputs.iter().find(|p| !p.name.starts_with("on")) {
        return Err(format!("{montre} : le port de sortie « {} » n'est pas un événement (on…)", o.name));
    }
    for (paquet, version) in &f.libs {
        match libs.get(paquet) {
            None => return Err(format!("{montre} : la lib « {paquet} » n'est pas déclarée dans {LIBS_FILE}")),
            Some(l) if &l.version != version => {
                return Err(format!("{montre} : veut {paquet}@{version}, {LIBS_FILE} déclare {}", l.version))
            }
            _ => {}
        }
    }
    // Les effets : chacun un outil exposé, appelable d'un écran (`policy.ecran`).
    for effet in &f.effets {
        match ecran.outils.get(effet) {
            None => return Err(format!("{montre} : l'effet « {effet} » n'est pas un outil exposé de ce backend")),
            Some(None) => {
                return Err(format!(
                    "{montre} : l'outil « {effet} » n'est pas appelable d'un écran ; son \"policy\" doit dire \"ecran\": \"lecture\" ou \"ecriture\""
                ))
            }
            Some(Some(_)) => {}
        }
    }
    // Le graphe de clic : dans `components/`, validé contre les effets et le backend.
    if let Some(clic) = &f.clic {
        let p = Path::new(clic);
        if p.is_absolute() || p.components().any(|c| !matches!(c, PathComponent::Normal(_))) || !clic.ends_with(".clic.mmd") {
            return Err(format!("{montre} : le graphe de clic « {clic} » doit être un <nom>.clic.mmd de components/"));
        }
        let texte = std::fs::read_to_string(dir.join(p)).map_err(|e| format!("{montre} : le graphe de clic « {clic} » : {e}"))?;
        let ctx = crate::dataflow::clic::ContexteClic {
            effets: f.effets.clone(),
            outils: ecran.outils.clone(),
            etat_hote: ecran.etat_hote.clone(),
        };
        if let Err(refus) = crate::dataflow::clic::valider_graphe_de_clic(&texte, &ctx) {
            let dits: Vec<String> = refus
                .iter()
                .map(|r| match r.ligne {
                    Some(l) => format!("components/{clic}:{l} : {}", r.message),
                    None => format!("components/{clic} : {}", r.message),
                })
                .collect();
            return Err(dits.join(" ; "));
        }
    }
    Ok(ComponentDecl {
        name: f.name,
        description: f.description,
        source: f.source,
        inputs,
        outputs,
        libs: f.libs,
        effets: f.effets,
        clic: f.clic,
    })
}

/// `react/jsx-runtime` → (`react`, `./jsx-runtime`) ; `@scope/p/x` → (`@scope/p`, `./x`).
pub fn parse_package_name(specifier: &str) -> (&str, String) {
    let parts: Vec<&str> = specifier.splitn(3, '/').collect();
    let n = if specifier.starts_with('@') { 2.min(parts.len()) } else { 1 };
    let fin = parts[..n].iter().map(|p| p.len()).sum::<usize>() + n - 1;
    let paquet = &specifier[..fin];
    let reste = &specifier[fin..];
    (paquet, if reste.is_empty() { ".".to_string() } else { format!(".{reste}") })
}

/// La cible d'un sous-chemin par la table `exports` (ou `main`, ou `index.js`).
fn cible_exports(lib: &Lib, sous: &str, mode: Mode) -> Option<String> {
    let conditions = ["browser", if mode == Mode::Development { "development" } else { "production" }, "import", "default"];
    fn choisir(v: &Value, conditions: &[&str]) -> Option<String> {
        match v {
            Value::String(s) => Some(s.clone()),
            Value::Array(a) => a.iter().find_map(|x| choisir(x, conditions)),
            // **Par notre ordre de conditions, pas par l'ordre de l'objet** :
            // serde_json range les clés (« default » passerait avant
            // « development »). `default` vient toujours en dernier.
            Value::Object(o) => conditions.iter().find_map(|c| o.get(*c)).and_then(|x| choisir(x, conditions)),
            _ => None,
        }
    }
    match &lib.exports {
        Value::Null => {
            if sous == "." {
                Some(lib.main.clone().unwrap_or_else(|| "index.js".to_string()))
            } else {
                Some(sous.to_string())
            }
        }
        Value::String(_) | Value::Array(_) if sous == "." => choisir(&lib.exports, &conditions),
        Value::Object(o) => {
            // Une table de sous-chemins (clés en `.`) ou de conditions pour `.`.
            if o.keys().any(|k| k.starts_with('.')) {
                o.get(sous).and_then(|v| choisir(v, &conditions))
            } else if sous == "." {
                choisir(&lib.exports, &conditions)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Un fichier d'une lib par son chemin, en essayant `.js` et `/index.js`.
fn fichier_de(lib: &Lib, vise: &str) -> Option<(String, String)> {
    [vise.to_string(), format!("{vise}.js"), format!("{vise}/index.js")]
        .into_iter()
        .find_map(|c| lib.files.get(&c).map(|h| (c.clone(), h.clone())))
}

fn parent_de(fichier: &str) -> String {
    fichier.rsplit_once('/').map(|(p, _)| p.to_string()).unwrap_or_default()
}

/// Joint et normalise `base/spec` ; `None` s'il sort de la racine.
fn normaliser(base: &str, spec: &str) -> Option<String> {
    let mut pile: Vec<&str> = base.split('/').filter(|s| !s.is_empty()).collect();
    for part in spec.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                pile.pop()?;
            }
            p => pile.push(p),
        }
    }
    Some(pile.join("/"))
}

pub mod modules;
/// La montre des composants : `components/` et `libs.json` changés, sur le bus.
pub mod a_chaud;

#[cfg(test)]
mod tests;
