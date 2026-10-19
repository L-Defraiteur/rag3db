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
    assert!(e.iter().any(|x| x.contains("react-dom dépend de react@^19.2.0") && x.contains("pas déclaré")), "{e:?}");
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

// ── Le lot 3 : le graphe des modules, exécuté par le registre de la page ──

/// Un backend « exécutable » : un faux React en CommonJS (useState, jsxDEV
/// qui requiert react, react-dom/client qui requiert react), et deux
/// composants TSX, `Fiche` qui importe `Bouton`.
fn backend_executable() -> (tempfile::TempDir, PathBuf) {
    let t = tempfile::tempdir().unwrap();
    let b = t.path().join("backend");
    std::fs::create_dir_all(b.join("components")).unwrap();
    let r = paquet(
        t.path(),
        "react",
        json!({
            "name": "react", "version": "19.2.0",
            "exports": {
                ".": { "development": "./cjs/react.development.js", "default": "./cjs/react.production.js" },
                "./jsx-dev-runtime": "./cjs/jsx-dev.js",
                "./jsx-runtime": "./cjs/jsx-dev.js"
            }
        }),
        &[
            ("cjs/react.development.js", "exports.useState = function (v) { return [v, function () {}]; }; exports.version = 'dev';"),
            ("cjs/react.production.js", "exports.version = 'prod';"),
            ("cjs/jsx-dev.js", "var React = require('react'); exports.jsxDEV = function (t, p) { return { t: typeof t === 'string' ? t : t.name, p: p, v: React.version }; }; exports.jsx = exports.jsxDEV; exports.jsxs = exports.jsxDEV; exports.Fragment = 'F';"),
        ],
    );
    let d = paquet(
        t.path(),
        "react-dom",
        json!({ "name": "react-dom", "version": "19.2.0", "exports": { ".": "./index.js", "./client": "./client.js" }, "dependencies": { "react": "19.2.0" } }),
        &[
            ("index.js", "module.exports = {};"),
            ("client.js", "var React = require(\"react\"); exports.createRoot = function () { return { render: function (e) { globalThis.__rendu = e; } }; }; exports.memeReact = React;"),
        ],
    );
    add_lib(&b, &r).unwrap();
    add_lib(&b, &d).unwrap();
    let ecrire = |nom: &str, fichier: &str, source: &str| {
        std::fs::write(b.join("components").join(fichier), source).unwrap();
        std::fs::write(
            b.join("components").join(format!("{}{SUFFIX}", fichier.split('.').next().unwrap())),
            json!({ "name": nom, "source": fichier, "libs": { "react": "19.2.0" } }).to_string(),
        )
        .unwrap();
    };
    ecrire("Bouton", "bouton.tsx", "export function Bouton({ libelle }: { libelle: string }) { return <button>{libelle}</button>; }\n");
    ecrire(
        "Fiche",
        "fiche.tsx",
        "import { useState } from \"react\";\nimport { Bouton } from \"./bouton\";\nexport function Fiche({ titre }: { titre: string }) {\n  const [n] = useState(3);\n  return <div>{titre}{n}<Bouton libelle=\"ok\" /></div>;\n}\n",
    );
    (t, b)
}

/// **Le graphe d'un composant tient ensemble** : le composant compilé, le
/// composant qu'il importe, les fichiers de React, et le démarrage de la page
/// (react, react-dom/client) — tout résolu, `react` au même id partout.
#[test]
fn a_component_graph_holds_together() {
    let (_t, b) = backend_executable();
    let c = Components::load(&b).unwrap();
    let g = c.module_graph(&b, "Fiche", Mode::Development).unwrap();
    assert!(g.unresolved.is_empty(), "{:?}", g.unresolved);
    let fiche = &g.modules["comp:Fiche"];
    assert_eq!(fiche.deps["./bouton"], "comp:Bouton");
    let react = &fiche.deps["react"];
    assert_eq!(react, "lib:react@19.2.0/cjs/react.development.js");
    assert_eq!(&g.boot["react"], react, "le démarrage et le composant partagent react");
    let client = &g.modules[&g.boot["react-dom/client"]];
    assert_eq!(&client.deps["react"], react, "react-dom aussi");
    assert!(g.modules.contains_key("comp:Bouton"));
    assert!(fiche.exports_only_components);
    // Chaque module de lib porte le hash du magasin.
    assert_eq!(g.modules[react].hash, c.libs["react"].files["cjs/react.development.js"]);
}

/// **Le registre de la page exécute le graphe** (QuickJS, sans navigateur) :
/// le composant compilé par `tsx` se rend en son élément, son enfant aussi, et
/// react-dom voit le même React que le composant — une seule instance.
#[test]
fn the_page_registry_runs_the_graph() {
    let (_t, b) = backend_executable();
    let c = Components::load(&b).unwrap();
    let g = c.module_graph(&b, "Fiche", Mode::Development).unwrap();
    let programme = format!(
        "{}{}\nconst F = __rag3weaver.require({entree}).Fiche;\nconst e = F({{ titre: \"T\" }});\nconst dom = __rag3weaver.require({client});\ndom.createRoot(null).render(e);\nJSON.stringify({{ e: globalThis.__rendu, un: dom.memeReact === __rag3weaver.require({react}) }});",
        super::modules::REGISTRE,
        g.script(),
        entree = serde_json::to_string(&g.entry).unwrap(),
        client = serde_json::to_string(&g.boot["react-dom/client"]).unwrap(),
        react = serde_json::to_string(&g.boot["react"]).unwrap(),
    );
    let rt = rquickjs::Runtime::new().unwrap();
    let ctx = rquickjs::Context::full(&rt).unwrap();
    let rendu: String = ctx.with(|ctx| match ctx.eval::<String, _>(programme.clone()) {
        Ok(s) => s,
        Err(e) => {
            let detail = ctx.catch();
            panic!("{e} — {detail:?}\n{programme}")
        }
    });
    let v: Value = serde_json::from_str(&rendu).unwrap();
    assert_eq!(v["e"]["t"], "div", "{rendu}");
    assert_eq!(v["e"]["v"], "dev", "le React de développement : {rendu}");
    assert_eq!(v["un"], true, "une seule instance de React : {rendu}");
    assert!(rendu.contains("\"Bouton\"") && rendu.contains("\"T\"") && rendu.contains('3'), "{rendu}");
}

/// **Un magasin touché est refusé** : les octets d'une lib qui ne redonnent
/// plus leur hash.
#[test]
fn a_tampered_store_is_refused() {
    let (_t, b) = backend_executable();
    let c = Components::load(&b).unwrap();
    let h = c.libs["react"].files["cjs/react.development.js"].clone();
    std::fs::write(store_path(&b, &h), "exports.useState = null; // touché").unwrap();
    let e = c.module_graph(&b, "Fiche", Mode::Development).unwrap_err();
    assert!(e.contains("react/cjs/react.development.js") && e.contains("touché"), "{e}");
}

// ── Le lot 4 : la mise à jour à chaud, et la montre des composants ──

/// Le registre et les modules d'un graphe, évalués dans un contexte QuickJS ;
/// rend ce que le dernier programme rend.
fn evaluer(ctx: &rquickjs::Context, programme: &str) -> String {
    ctx.with(|ctx| match ctx.eval::<String, _>(programme.to_string()) {
        Ok(s) => s,
        Err(e) => {
            let detail = ctx.catch();
            panic!("{e} — {detail:?}\n{programme}")
        }
    })
}

/// Les modules d'un graphe, puis la mise à jour du registre, en JSON.
fn mise_a_jour(c: &Components, b: &Path) -> String {
    let g = c.module_graph(b, "Fiche", Mode::Development).unwrap();
    let frontieres: Vec<&str> = g.modules.values().filter(|m| m.exports_only_components).map(|m| m.id.as_str()).collect();
    format!(
        "{}\nJSON.stringify(__rag3weaver.mettre_a_jour({}));",
        g.script(),
        serde_json::to_string(&frontieres).unwrap()
    )
}

/// **Une édition se reprend à sa frontière.** Le registre ne redéfinit que le
/// module dont le hash a changé ; un composant seul se réexécute lui-même ; un
/// module qui exporte autre chose qu'un composant fait remonter jusqu'au
/// composant qui l'importe ; un chemin qui n'en trouve aucun demande de
/// recharger la page.
#[test]
fn an_edit_is_taken_back_at_its_boundary() {
    let (_t, b) = backend_executable();
    let c = Components::load(&b).unwrap();
    let g = c.module_graph(&b, "Fiche", Mode::Development).unwrap();
    let rt = rquickjs::Runtime::new().unwrap();
    let ctx = rquickjs::Context::full(&rt).unwrap();
    evaluer(
        &ctx,
        &format!(
            "{}{}\n__rag3weaver.require({});\nJSON.stringify(__rag3weaver.changes);",
            super::modules::REGISTRE,
            g.script(),
            serde_json::to_string(&g.entry).unwrap()
        ),
    );
    let bouton = b.join("components/bouton.tsx");

    // Le bouton seul : il est sa propre frontière.
    std::fs::write(&bouton, "export function Bouton({ libelle }: { libelle: string }) { return <i>{libelle}</i>; }\n").unwrap();
    let v: Value = serde_json::from_str(&evaluer(&ctx, &mise_a_jour(&c, &b))).unwrap();
    assert_eq!(v, json!({ "relances": ["comp:Bouton"], "changes": ["comp:Bouton"] }));

    // Le même graphe relu : rien n'a changé, rien ne se redéfinit.
    let v: Value = serde_json::from_str(&evaluer(&ctx, &mise_a_jour(&c, &b))).unwrap();
    assert_eq!(v, json!({ "relances": [], "changes": [] }));

    // Le bouton exporte aussi une constante : la fiche, qui l'importe, est
    // la frontière.
    std::fs::write(
        &bouton,
        "export const taille = 2;\nexport function Bouton({ libelle }: { libelle: string }) { return <b>{libelle}</b>; }\n",
    )
    .unwrap();
    let v: Value = serde_json::from_str(&evaluer(&ctx, &mise_a_jour(&c, &b))).unwrap();
    assert_eq!(v, json!({ "relances": ["comp:Fiche"], "changes": ["comp:Bouton"] }));

    // La fiche, l'entrée, exporte une constante : rien au-dessus ne la reprend.
    let fiche = b.join("components/fiche.tsx");
    let source = std::fs::read_to_string(&fiche).unwrap();
    std::fs::write(&fiche, format!("export const nombre = 1;\n{source}")).unwrap();
    let v: Value = serde_json::from_str(&evaluer(&ctx, &mise_a_jour(&c, &b))).unwrap();
    assert_eq!(v["changes"], json!(["comp:Fiche"]), "{v}");
    assert!(v["recharger"].as_str().is_some_and(|r| r.contains("comp:Fiche")), "{v}");
}

/// **La montre des composants dit ce qui change** sur le sujet `components` :
/// le code d'un composant, puis une déclaration (que la porte de l'index
/// aurait écartée : une donnée JSON).
#[cfg(feature = "code")]
#[test]
fn the_components_watch_says_what_changed() {
    let (_t, b) = backend_executable();
    let bus = crate::events::EventBus::new(64);
    let mut rx = bus.subscribe(crate::events::topic::COMPONENTS);
    let montre = super::a_chaud::spawn(&b, bus.clone()).unwrap();
    let attendre = |rx: &mut async_broadcast::Receiver<crate::events::Event>| {
        let fin = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            if let Ok(crate::events::Event::ComponentsChanged { paths, declarations, .. }) = rx.try_recv() {
                return (paths, declarations);
            }
            assert!(std::time::Instant::now() < fin, "rien sur le sujet components en 10 s");
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    };
    std::fs::write(b.join("components/bouton.tsx"), "export function Bouton() { return null; }\n").unwrap();
    assert_eq!(attendre(&mut rx), (vec!["components/bouton.tsx".to_string()], false));
    std::fs::write(
        b.join("components/bouton.component.json"),
        json!({ "name": "Bouton", "source": "bouton.tsx", "description": "neuf", "libs": { "react": "19.2.0" } }).to_string(),
    )
    .unwrap();
    assert_eq!(attendre(&mut rx), (vec!["components/bouton.component.json".to_string()], true));
    drop(montre);
}

/// **Un pair se résout vers la copie que le backend range** : `react-dom`
/// déclare `react` en `peerDependencies` (le vrai 19.2.0 le fait), et prend
/// le `react` de `libs.json`, quelle que soit l'écriture de l'intervalle. Un
/// pair absent se dit en le nommant.
#[test]
fn a_peer_resolves_to_the_copy_the_backend_holds() {
    let t = tempfile::tempdir().unwrap();
    let b = t.path().join("backend");
    std::fs::create_dir_all(&b).unwrap();
    let r = paquet(t.path(), "react", json!({ "name": "react", "version": "19.2.1", "main": "index.js" }), &[("index.js", "")]);
    let d = paquet(
        t.path(),
        "react-dom",
        json!({ "name": "react-dom", "version": "19.2.0", "main": "index.js", "peerDependencies": { "react": "^19.2.0" } }),
        &[("index.js", "require('react');")],
    );
    add_lib(&b, &d).unwrap();
    let c = Components::load(&b).unwrap();
    assert_eq!(c.libs["react-dom"].peers["react"], "^19.2.0");
    let importeur = Importer::LibFile { package: "react-dom", file: "index.js" };
    let e = c.resolve(&importeur, "react", Mode::Development).unwrap_err();
    assert!(e.contains("le pair « react »"), "{e}");
    add_lib(&b, &r).unwrap();
    let c = Components::load(&b).unwrap();
    match c.resolve(&importeur, "react", Mode::Development).unwrap() {
        Resolved::LibFile { package, version, file, .. } => assert_eq!((package.as_str(), version.as_str(), file.as_str()), ("react", "19.2.1", "index.js")),
        autre => panic!("{autre:?}"),
    }
}

/// Exécute un graphe de libs dans rquickjs : le registre, les modules, puis
/// `expression` (du JavaScript qui rend une chaîne).
fn executer(g: &super::modules::Graph, expression: &str) -> String {
    let programme = format!("{}{}\n{expression}", super::modules::REGISTRE, g.script());
    let rt = rquickjs::Runtime::new().unwrap();
    let ctx = rquickjs::Context::full(&rt).unwrap();
    ctx.with(|ctx| match ctx.eval::<String, _>(programme.clone()) {
        Ok(s) => s,
        Err(e) => {
            let detail = ctx.catch();
            panic!("{e} — {detail:?}\n{programme}")
        }
    })
}

/// **Un graphe de libs, sans composant ni React** (l'éditeur de l'IDE) : une
/// lib qui ne publie que de l'ESM est rendue en CommonJS pour le registre, sa
/// dépendance en `.cjs` est prise par la condition `require`, et une dépendance
/// en intervalle (`^1.0.0`) se résout vers la copie rangée qui le tient.
#[test]
fn a_graph_of_libs_runs_esm_and_cjs_through_the_registry() {
    let t = tempfile::tempdir().unwrap();
    let b = t.path().join("backend");
    std::fs::create_dir_all(&b).unwrap();
    let outil = paquet(
        t.path(),
        "outil",
        json!({ "name": "outil", "version": "1.4.2", "type": "module",
                "exports": { "import": "./dist/index.js", "require": "./dist/index.cjs" } }),
        &[
            ("dist/index.js", "export function double(x) { return 2 * x; }"),
            ("dist/index.cjs", "exports.double = function (x) { return 2 * x; };"),
        ],
    );
    let editeur = paquet(
        t.path(),
        "editeur",
        json!({ "name": "editeur", "version": "6.0.0", "type": "module", "exports": { "import": "./dist/index.js" },
                "dependencies": { "outil": "^1.0.0" } }),
        &[("dist/index.js", "import { double } from \"outil\";\nexport const nom = \"éditeur\";\nexport default function quatre() { return double(2); }\n")],
    );
    add_lib(&b, &outil).unwrap();
    add_lib(&b, &editeur).unwrap();
    let c = Components::load(&b).unwrap();
    assert_eq!(c.libs["editeur"].dependencies["outil"], "^1.0.0", "l'intervalle reste tel qu'écrit");
    let g = c.module_graph_of(&b, &["editeur"], Mode::Development).unwrap();
    assert!(g.unresolved.is_empty(), "{:?}", g.unresolved);
    assert!(g.modules.keys().any(|k| k == "lib:outil@1.4.2/dist/index.cjs"), "la condition require : {:?}", g.modules.keys());
    let rendu = executer(&g, &format!(
        "const m = __rag3weaver.require({}); JSON.stringify({{ nom: m.nom, quatre: m.default() }});",
        serde_json::to_string(&g.boot["editeur"]).unwrap()
    ));
    assert_eq!(rendu, r#"{"nom":"éditeur","quatre":4}"#);
    // Un intervalle que la copie rangée ne tient pas se refuse en le nommant.
    let vieil = paquet(t.path(), "outil-0", json!({ "name": "outil", "version": "0.9.0", "main": "index.js" }), &[("index.js", "")]);
    add_lib(&b, &vieil).unwrap();
    let e = Components::load(&b).unwrap_err();
    assert!(e.iter().any(|x| x.contains("editeur dépend de outil@^1.0.0") && x.contains("0.9.0")), "{e:?}");
}

/// **Un pair hors de son intervalle se refuse**, en le nommant, comme une
/// dépendance (le pair est la copie du backend — si elle tient l'intervalle).
#[test]
fn a_peer_outside_its_range_is_refused() {
    let t = tempfile::tempdir().unwrap();
    let b = t.path().join("backend");
    std::fs::create_dir_all(&b).unwrap();
    let r = paquet(t.path(), "react", json!({ "name": "react", "version": "18.3.1", "main": "index.js" }), &[("index.js", "")]);
    let d = paquet(
        t.path(),
        "react-dom",
        json!({ "name": "react-dom", "version": "19.2.0", "main": "index.js", "peerDependencies": { "react": "^19.2.0" } }),
        &[("index.js", "require('react');")],
    );
    add_lib(&b, &r).unwrap();
    add_lib(&b, &d).unwrap();
    let c = Components::load(&b).unwrap();
    let e = c.resolve(&Importer::LibFile { package: "react-dom", file: "index.js" }, "react", Mode::Development).unwrap_err();
    assert!(e.contains("react@^19.2.0") && e.contains("18.3.1"), "{e}");
}

/// **Le magasin ne range pas ce qu'un navigateur ne charge pas** : types,
/// cartes de sources et notes partent ; la licence reste.
#[test]
fn the_store_keeps_code_and_licence_not_types_maps_or_notes() {
    let t = tempfile::tempdir().unwrap();
    let b = t.path().join("backend");
    std::fs::create_dir_all(&b).unwrap();
    let p = paquet(
        t.path(),
        "leger",
        json!({ "name": "leger", "version": "1.0.0", "main": "dist/index.cjs" }),
        &[
            ("dist/index.cjs", "module.exports = 1;"),
            ("dist/index.d.ts", "export {};"),
            ("dist/index.d.cts", "export {};"),
            ("dist/index.cjs.map", "{}"),
            ("CHANGELOG.md", "# 1.0.0"),
            ("README.md", "# leger"),
            ("LICENSE", "MIT"),
            ("LICENSE.md", "MIT"),
        ],
    );
    add_lib(&b, &p).unwrap();
    let fichiers: Vec<String> = read_libs(&b).unwrap()["leger"].files.keys().cloned().collect();
    assert_eq!(fichiers, vec!["LICENSE", "LICENSE.md", "dist/index.cjs", "package.json"]);
}
