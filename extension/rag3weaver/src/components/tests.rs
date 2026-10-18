//! Les témoins des composants déclarés et des bibliothèques rangées par
//! contenu (marche 3, lot 2).
use super::*;
use serde_json::json;

/// Un faux paquet npm déballé : son `package.json` et ses fichiers.
fn paquet(racine: &Path, nom: &str, pj: Value, fichiers: &[(&str, &str)]) -> PathBuf {
    let dir = racine.join("paquets").join(nom.replace('/', "_"));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("package.json"), pj.to_string()).unwrap();
    for (chemin, texte) in fichiers {
        let p = dir.join(chemin);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, texte).unwrap();
    }
    dir
}

fn react(racine: &Path, version: &str) -> PathBuf {
    paquet(
        racine,
        "react",
        json!({
            "name": "react", "version": version,
            "exports": {
                ".": { "development": "./cjs/react.development.js", "default": "./cjs/react.production.js" },
                "./jsx-runtime": { "development": "./cjs/react-jsx-runtime.development.js", "default": "./cjs/react-jsx-runtime.production.js" },
                "./package.json": "./package.json"
            }
        }),
        &[
            ("cjs/react.development.js", &format!("// react {version} dev")),
            ("cjs/react.production.js", &format!("// react {version} prod")),
            ("cjs/react-jsx-runtime.development.js", "// jsx dev"),
            ("cjs/react-jsx-runtime.production.js", "// jsx prod"),
        ],
    )
}

fn react_dom(racine: &Path) -> PathBuf {
    paquet(
        racine,
        "react-dom",
        json!({ "name": "react-dom", "version": "19.2.0", "main": "index.js", "dependencies": { "react": "^19.2.0" } }),
        &[("index.js", "module.exports = require('./cjs/react-dom.js')"), ("cjs/react-dom.js", "require('react')")],
    )
}

fn composant(backend: &Path, nom: &str, source: &str, libs: Value) {
    let dir = backend.join("components");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(source), format!("export function {nom}() {{ return null }}")).unwrap();
    std::fs::write(
        dir.join(format!("{}{SUFFIX}", source.split('.').next().unwrap())),
        json!({
            "name": nom, "source": source,
            "inputs": { "titre": { "schema": { "type": "string" }, "required": true } },
            "outputs": { "onClic": { "schema": { "type": "object" } } },
            "libs": libs
        })
        .to_string(),
    )
    .unwrap();
}

/// Un backend avec react et react-dom rangés, et deux composants.
fn backend() -> (tempfile::TempDir, PathBuf) {
    let t = tempfile::tempdir().unwrap();
    let b = t.path().join("backend");
    std::fs::create_dir_all(&b).unwrap();
    add_lib(&b, &react(t.path(), "19.2.0")).unwrap();
    add_lib(&b, &react_dom(t.path())).unwrap();
    composant(&b, "Fiche", "fiche.tsx", json!({ "react": "19.2.0" }));
    composant(&b, "Liste", "liste.tsx", json!({ "react": "19.2.0", "react-dom": "19.2.0" }));
    (t, b)
}

fn hash_de(r: Resolved) -> String {
    match r {
        Resolved::LibFile { hash, .. } => hash,
        autre => panic!("un fichier de lib attendu : {autre:?}"),
    }
}

/// **`react` résout au même hash partout** : depuis deux composants, et depuis
/// react-dom qui le requiert — une seule instance, par construction (un
/// paquet, une entrée de `libs.json`).
#[test]
fn react_resolves_to_the_same_hash_everywhere() {
    let (_t, b) = backend();
    let c = Components::load(&b).unwrap();
    let dev = Mode::Development;
    let de_fiche = hash_de(c.resolve(&Importer::Component("Fiche"), "react", dev).unwrap());
    let de_liste = hash_de(c.resolve(&Importer::Component("Liste"), "react", dev).unwrap());
    let de_dom = hash_de(c.resolve(&Importer::LibFile { package: "react-dom", file: "cjs/react-dom.js" }, "react", dev).unwrap());
    assert_eq!(de_fiche, de_liste);
    assert_eq!(de_fiche, de_dom);
    assert_eq!(de_fiche, c.libs["react"].files["cjs/react.development.js"]);
    // La production prend l'autre condition ; jsx-runtime passe par son sous-chemin.
    let prod = hash_de(c.resolve(&Importer::Component("Fiche"), "react", Mode::Production).unwrap());
    assert_eq!(prod, c.libs["react"].files["cjs/react.production.js"]);
    let jsx = hash_de(c.resolve(&Importer::Component("Fiche"), "react/jsx-runtime", dev).unwrap());
    assert_eq!(jsx, c.libs["react"].files["cjs/react-jsx-runtime.development.js"]);
    // Un relatif dans une lib, sans extension : main, puis le fichier requis.
    let dom = hash_de(c.resolve(&Importer::Component("Liste"), "react-dom", dev).unwrap());
    assert_eq!(dom, c.libs["react-dom"].files["index.js"]);
    let interne = c.resolve(&Importer::LibFile { package: "react-dom", file: "index.js" }, "./cjs/react-dom", dev).unwrap();
    assert_eq!(hash_de(interne), c.libs["react-dom"].files["cjs/react-dom.js"]);
    // Les octets sont au magasin, sous leur hash.
    assert_eq!(std::fs::read_to_string(store_path(&b, &de_fiche)).unwrap(), "// react 19.2.0 dev");
}

/// **Ce qui n'est pas déclaré est refusé en le nommant** : un paquet hors des
/// libs du composant, un sous-chemin non exporté, un composant relatif absent.
#[test]
fn what_is_not_declared_is_refused_by_name() {
    let (_t, b) = backend();
    let c = Components::load(&b).unwrap();
    let dev = Mode::Development;
    let e = c.resolve(&Importer::Component("Fiche"), "lodash", dev).unwrap_err();
    assert!(e.contains("Fiche") && e.contains("lodash") && e.contains("libs"), "{e}");
    let e = c.resolve(&Importer::Component("Fiche"), "react-dom", dev).unwrap_err();
    assert!(e.contains("react-dom") && e.contains("Fiche"), "déclaré au backend mais pas par ce composant : {e}");
    let e = c.resolve(&Importer::Component("Fiche"), "react/cjs/react.development.js", dev).unwrap_err();
    assert!(e.contains("n'exporte pas"), "{e}");
    let e = c.resolve(&Importer::Component("Fiche"), "./absent", dev).unwrap_err();
    assert!(e.contains("aucun composant"), "{e}");
    assert_eq!(c.resolve(&Importer::Component("Fiche"), "./liste", dev).unwrap(), Resolved::Component("Liste".into()));
    let e = c.resolve(&Importer::Component("Fiche"), "../hors", dev).unwrap_err();
    assert!(e.contains("sort du dossier"), "{e}");
}

/// **Le chargement vérifie tout, et nomme chaque défaut** : une version de lib
/// qui ne concorde pas, une dépendance non déclarée, un fichier absent du
/// magasin, un port de sortie qui n'est pas un événement, un schéma invalide.
#[test]
fn loading_checks_everything_and_names_each_fault() {
    let (t, b) = backend();
    composant(&b, "Vieille", "vieille.tsx", json!({ "react": "18.3.1" }));
    let e = Components::load(&b).unwrap_err();
    assert!(e.iter().any(|x| x.contains("vieille") && x.contains("react@18.3.1") && x.contains("19.2.0")), "{e:?}");
    std::fs::remove_file(b.join("components/vieille.component.json")).unwrap();

    // Une sortie qui n'est pas un événement ; un schéma invalide.
    std::fs::write(b.join("components/fiche.component.json"), json!({
        "name": "Fiche", "source": "fiche.tsx", "outputs": { "valeur": { "schema": { "type": "string" } } }
    }).to_string()).unwrap();
    std::fs::write(b.join("components/liste.component.json"), json!({
        "name": "Liste", "source": "liste.tsx", "inputs": { "x": { "schema": { "type": 12 } } }
    }).to_string()).unwrap();
    let e = Components::load(&b).unwrap_err();
    assert!(e.iter().any(|x| x.contains("valeur") && x.contains("événement")), "{e:?}");
    assert!(e.iter().any(|x| x.contains("« x »") && x.contains("schéma invalide")), "{e:?}");

    // Une dépendance non déclarée, un fichier absent du magasin.
    let (_t2, b2) = backend();
    let mut libs = read_libs(&b2).unwrap();
    libs.remove("react");
    std::fs::write(b2.join(LIBS_FILE), serde_json::to_string(&libs).unwrap()).unwrap();
    std::fs::remove_dir_all(b2.join("components")).unwrap();
    let e = Components::load(&b2).unwrap_err();
    assert!(e.iter().any(|x| x.contains("react-dom dépend de react@19.2.0") && x.contains("pas déclaré")), "{e:?}");
    let (_t3, b3) = backend();
    let h = read_libs(&b3).unwrap()["react"].files["cjs/react.production.js"].clone();
    std::fs::remove_file(store_path(&b3, &h)).unwrap();
    let e = Components::load(&b3).unwrap_err();
    assert!(e.iter().any(|x| x.contains("react/cjs/react.production.js") && x.contains("absent du magasin")), "{e:?}");
    drop(t);
}

/// **Une version par paquet** : ranger une autre version remplace la première,
/// et la réponse le dit ; un backend sans composant ni lib se charge vide.
#[test]
fn one_version_per_package_and_an_empty_backend_loads() {
    let (t, b) = backend();
    let dit = add_lib(&b, &react(&t.path().join("autre"), "19.3.0")).unwrap();
    assert!(dit.contains("react@19.3.0") && dit.contains("remplace react@19.2.0"), "{dit}");
    assert_eq!(read_libs(&b).unwrap()["react"].version, "19.3.0");
    let vide = tempfile::tempdir().unwrap();
    let c = Components::load(vide.path()).unwrap();
    assert!(c.components.is_empty() && c.libs.is_empty());
}

#[test]
fn package_names_split_like_node() {
    assert_eq!(parse_package_name("react"), ("react", ".".to_string()));
    assert_eq!(parse_package_name("react/jsx-runtime"), ("react", "./jsx-runtime".to_string()));
    assert_eq!(parse_package_name("@scope/p"), ("@scope/p", ".".to_string()));
    assert_eq!(parse_package_name("@scope/p/x/y"), ("@scope/p", "./x/y".to_string()));
}

/// Un backend d'un composant à graphe de clic : `effets` et `clic` déclarés.
fn composant_a_clic(effets: &str, clic: &str) -> tempfile::TempDir {
    let t = tempfile::tempdir().unwrap();
    let c = t.path().join("components");
    std::fs::create_dir_all(&c).unwrap();
    std::fs::write(c.join("fiche.tsx"), "export function Fiche() { return null; }\n").unwrap();
    std::fs::write(
        c.join("fiche.component.json"),
        format!(r#"{{ "name": "Fiche", "source": "fiche.tsx", "effets": {effets}, "clic": "fiche.clic.mmd" }}"#),
    )
    .unwrap();
    std::fs::write(c.join("fiche.clic.mmd"), clic).unwrap();
    t
}

fn ecran(etat_hote: Option<&str>) -> EcranDuBackend {
    use crate::dataflow::clic::Ecran;
    EcranDuBackend {
        outils: BTreeMap::from([
            ("search_code".to_string(), Some(Ecran::Lecture)),
            ("run_command".to_string(), None),
        ]),
        etat_hote: etat_hote.map(str::to_string),
    }
}

const CHERCHER: &str = "graph LR\n    saisie[\"Evenement(nom='onSaisie')\"]\n    chercher[\"Effet(outil='search_code')\"]\n    saisie -->|charge:query| chercher\n";

/// **Un composant à graphe de clic se déclare** : ses effets et son graphe y sont gardés.
#[test]
fn un_composant_a_graphe_de_clic_se_declare() {
    let t = composant_a_clic(r#"["search_code"]"#, CHERCHER);
    let c = Components::load_in(t.path(), &ecran(None)).unwrap_or_else(|e| panic!("{e:?}"));
    let f = &c.components["Fiche"];
    assert_eq!(f.effets, vec!["search_code"]);
    assert_eq!(f.clic.as_deref(), Some("fiche.clic.mmd"));
}

/// **Le contrat se vérifie à la déclaration** : un effet vers un outil sans `ecran` est refusé
/// en disant quoi déclarer ; un effet du graphe que le composant ne déclare pas l'est à sa
/// ligne, avec le chemin du `.clic.mmd`.
#[test]
fn un_effet_hors_contrat_est_refuse_a_la_declaration() {
    let t = composant_a_clic(r#"["run_command"]"#, CHERCHER);
    let e = Components::load_in(t.path(), &ecran(None)).unwrap_err().join("\n");
    assert!(e.contains("« run_command » n'est pas appelable d'un écran") && e.contains("\"ecran\""), "{e}");
    let t = composant_a_clic("[]", CHERCHER);
    let e = Components::load_in(t.path(), &ecran(None)).unwrap_err().join("\n");
    assert!(e.contains("components/fiche.clic.mmd:3") && e.contains("ne déclare pas"), "{e}");
}

/// **Une case de portée `hôte` exige son lieu** : sans `ui_state`, refusée en disant quoi
/// déclarer ; avec, acceptée.
#[test]
fn une_case_de_portee_hote_exige_ui_state() {
    let g = "graph LR\n    commun[\"Etat(nom='filtre', schema='{\"type\":\"string\"}', initial='\"\"', portee='hôte')\"]\n";
    let t = composant_a_clic("[]", g);
    let e = Components::load_in(t.path(), &ecran(None)).unwrap_err().join("\n");
    assert!(e.contains("\"ui_state\""), "{e}");
    Components::load_in(t.path(), &ecran(Some("EtatEcran"))).unwrap_or_else(|e| panic!("{e:?}"));
}
