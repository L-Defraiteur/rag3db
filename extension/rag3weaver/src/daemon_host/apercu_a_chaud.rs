//! **L'aperçu d'un composant se recharge à chaud dans un vrai navigateur**
//! (marche 3, lot 4).
//!
//! Le vrai React (react, react-dom, scheduler, react-refresh) rangé au magasin
//! d'un backend, un composant à état rendu par la page d'aperçu, cliqué ; puis
//! sa source réécrite sur le disque. La montre des composants le voit, le flux
//! le dit à la page, la page relit les modules et React Refresh remplace le
//! composant : le texte neuf est dans le DOM, **l'état est gardé**, le nœud
//! est le même, et la console n'a rien dit.
//!
//! Les paquets viennent de `RAG3WEAVER_PAQUETS_REACT` (un dossier rempli par
//! `scripts/paquets_react.sh`) ; sans eux, ou sans navigateur, le témoin se
//! saute nommément — un rouge sous `RAG3WEAVER_EXIGER_NAVIGATEUR=1`.

use super::navigateur::{navigateur_ou_saut, Navigateur};
use serde_json::{json, Value};

/// Le dossier des paquets React déballés, ou `None` après la ligne `SAUTÉ`.
fn paquets_ou_saut(temoin: &str) -> Option<std::path::PathBuf> {
    let dossier = std::env::var_os("RAG3WEAVER_PAQUETS_REACT").map(std::path::PathBuf::from);
    if let Some(d) = dossier.filter(|d| d.join("react-refresh/package.json").is_file()) {
        return Some(d);
    }
    let raison = "RAG3WEAVER_PAQUETS_REACT ne désigne pas les paquets React (scripts/paquets_react.sh)";
    if std::env::var("RAG3WEAVER_EXIGER_NAVIGATEUR").as_deref() == Ok("1") {
        panic!("{temoin} : {raison}, et RAG3WEAVER_EXIGER_NAVIGATEUR=1");
    }
    eprintln!("SAUTÉ {temoin} : {raison}");
    None
}

const COMPTEUR: &str = r#"import { useState } from "react";

export function Compteur() {
  const [n, setN] = useState(0);
  return <button id="compteur" onClick={() => setN(n + 1)}>Compte : {n}</button>;
}
"#;

/// Un backend du gabarit `code` sur un hôte gardé par le jeton `j`, ses
/// paquets React rangés et ses composants écrits, ouvert (par `describe`).
struct Banc {
    _d: tempfile::TempDir,
    composants: std::path::PathBuf,
    racine: String,
    id: String,
    fil: std::thread::JoinHandle<()>,
}

#[cfg(all(feature = "code", feature = "rag3db-native"))]
fn monter(paquets: &std::path::Path, fichiers: &[(&str, &str)]) -> Banc {
    let d = tempfile::tempdir().unwrap();
    let (base, arbre) = (d.path().join("base"), d.path().join("arbre"));
    std::fs::create_dir_all(&base).unwrap();
    std::fs::create_dir_all(&arbre).unwrap();
    let gabarit = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/code");
    let manifeste = crate::gabarits::jouable::manifeste_jouable(&gabarit, &base).unwrap();
    let mut m: Value = serde_json::from_slice(&std::fs::read(&manifeste).unwrap()).unwrap();
    m["workspace"]["root"] = json!(arbre.to_string_lossy());
    std::fs::write(&manifeste, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
    let dossier = manifeste.parent().unwrap().to_path_buf();
    for p in ["react", "scheduler", "react-dom", "react-refresh"] {
        crate::components::add_lib(&dossier, &paquets.join(p)).unwrap_or_else(|e| panic!("libs add {p} : {e}"));
    }
    let composants = dossier.join("components");
    std::fs::create_dir_all(&composants).unwrap();
    for (nom, texte) in fichiers {
        std::fs::write(composants.join(nom), texte).unwrap();
    }
    // Le magasin réduit à ce que le graphe atteint (et les licences) : l'aperçu
    // à chaud doit se charger sur ce magasin-là (G, 19 octobre 2026).
    let reduit = crate::components::Components::load(&dossier).unwrap().prune_store(&dossier, &[]).unwrap();
    eprintln!("magasin de l'aperçu : {} → {} octets ({} objets effacés)", reduit.bytes_before, reduit.bytes_after, reduit.objects_removed);
    assert!(reduit.bytes_after < reduit.bytes_before, "{reduit:?}");
    let manifeste = manifeste.to_string_lossy().to_string();

    let (adresse, fil) = super::tests::hote_garde("j");
    let racine = format!("http://{adresse}");
    // `describe` ouvre le backend ; `/ide/etat` donne son id.
    let decrit: Value = {
        let mut r = crate::daemon::agent()
            .post(format!("{racine}/ide/describe"))
            .header("Cookie", "rag3weaver_ide=j")
            .header("content-type", "application/json")
            .send(json!({ "manifest": manifeste }).to_string())
            .unwrap();
        serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap()
    };
    assert_eq!(decrit["ok"], true, "{decrit}");
    let etat: Value = {
        let mut r = crate::daemon::agent()
            .get(format!("{racine}/ide/etat"))
            .header("Cookie", "rag3weaver_ide=j")
            .call()
            .unwrap();
        serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap()
    };
    let id = etat["backends"]
        .as_array()
        .and_then(|b| b.iter().find(|b| b["manifest"] == manifeste.as_str()))
        .unwrap_or_else(|| panic!("le backend n'est pas dans l'état : {etat}"))["id"]
        .as_str()
        .unwrap()
        .to_string();
    Banc { _d: d, composants, racine, id, fil }
}

#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn un_composant_modifie_se_recharge_dans_la_page_en_gardant_son_etat() {
    const TEMOIN: &str = "un_composant_modifie_se_recharge_dans_la_page_en_gardant_son_etat";
    let Some(binaire) = navigateur_ou_saut(TEMOIN) else {
        return;
    };
    let Some(paquets) = paquets_ou_saut(TEMOIN) else {
        return;
    };
    let Banc { _d, composants, racine, id, fil } = monter(
        &paquets,
        &[
            ("compteur.tsx", COMPTEUR),
            ("compteur.component.json", r#"{ "name": "Compteur", "source": "compteur.tsx", "libs": { "react": "19.2.0" } }"#),
        ],
    );

    // La route des graphes (C1), sur le même hôte : le graphe d'un outil du
    // gabarit, celui d'un composant (sans graphe de clic : vide), et le refus
    // d'une demande sans backend.
    let graphe = |requete: &str| -> (u16, Value) {
        let mut r = crate::daemon::agent()
            .get(format!("{racine}/ide/graphe?{requete}"))
            .header("Cookie", "rag3weaver_ide=j")
            .config()
            .http_status_as_error(false)
            .build()
            .call()
            .unwrap();
        (r.status().as_u16(), serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap())
    };
    let (statut, usages) = graphe(&format!("backend={id}&tool=usages"));
    assert_eq!(statut, 200, "{usages}");
    assert_eq!(usages["result"]["kind"], "tool", "{usages}");
    let noeuds = usages["result"]["nodes"].as_array().unwrap();
    assert!(!noeuds.is_empty() && noeuds.iter().all(|n| n["line"].is_u64() && n["schema"].is_object()), "{usages}");
    let (statut, compteur) = graphe(&format!("backend={id}&component=Compteur"));
    assert_eq!((statut, compteur["result"]["nodes"].clone()), (200, json!([])), "{compteur}");
    assert_eq!(graphe("tool=usages").0, 400);

    let mut n = Navigateur::lancer(&binaire);
    n.aller(&format!("{racine}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
    n.aller(
        &format!("{racine}/ide/apercu/{id}/Compteur"),
        "document.querySelector('#compteur')?.textContent === 'Compte : 0'",
    );
    assert!(n.erreurs.is_empty(), "le premier rendu : erreurs à la console : {:#?}", n.erreurs);

    // L'état : trois clics, et une marque sur le nœud pour voir qu'il reste.
    // Un clic, puis son rendu : React 19 applique la mise à jour d'un clic
    // dans une microtâche, et trois clics dans la même tâche liraient tous 0.
    for i in 1..=3 {
        n.evaluer("document.querySelector('#compteur').click()");
        n.attendre(&format!("document.querySelector('#compteur')?.textContent === 'Compte : {i}'"), "un clic");
    }
    n.evaluer("document.querySelector('#compteur').dataset.temoin = 'le-meme'");

    // La source réécrite sur le disque comme le fait un éditeur qui écrit en
    // place, au pire : le fichier vide d'abord, assez longtemps pour que la
    // montre le tienne pour établi (plus que sa fenêtre de 150 ms), puis le
    // texte. Le module vide n'est pas une frontière : la page attend avant de
    // se recharger, et la mise à jour suivante reprend le compteur.
    std::fs::write(composants.join("compteur.tsx"), "").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(600));
    std::fs::write(composants.join("compteur.tsx"), COMPTEUR.replace("Compte : ", "Total : ")).unwrap();
    // L'attente dit, en cas d'échec, ce que la page a fait : ses mises à jour
    // et la raison d'un rechargement entier.
    let debut = std::time::Instant::now();
    loop {
        let v = n.evaluer(
            "({ texte: document.querySelector('#compteur')?.textContent, apercu: window.__apercu, \
               erreurs: [...document.querySelectorAll('pre.erreur')].map(p => p.textContent) })",
        );
        if v["texte"] == "Total : 3" {
            break;
        }
        assert!(
            debut.elapsed() < std::time::Duration::from_secs(20),
            "le composant rechargé, son état gardé : jamais en 20 s ; la page : {v} ; console : {:#?}",
            n.erreurs
        );
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    let apres = n.evaluer(
        "({ meme: document.querySelector('#compteur').dataset.temoin === 'le-meme', \
           maj: window.__apercu.mises_a_jour, relances: window.__apercu.derniere.relances, \
           recharge: window.__apercu.recharge, \
           erreurs: document.querySelectorAll('pre.erreur').length })",
    );
    assert_eq!(apres["meme"], true, "le nœud a été remonté : {apres}");
    assert_eq!(apres["recharge"], Value::Null, "la page s'est rechargée entière : {apres}");
    // Une écriture en place peut donner deux mises à jour (le fichier vide,
    // puis le texte) : au moins une, la dernière reprend le compteur.
    assert!(apres["maj"].as_u64().is_some_and(|m| m >= 1), "{apres}");
    assert_eq!(apres["relances"], json!(["comp:Compteur"]), "{apres}");
    assert_eq!(apres["erreurs"], 0, "{apres}");
    // L'état vit encore : un clic de plus compte à partir de 3.
    n.evaluer("document.querySelector('#compteur').click()");
    n.attendre("document.querySelector('#compteur')?.textContent === 'Total : 4'", "un clic après le rechargement");
    assert!(n.erreurs.is_empty(), "le rechargement : erreurs à la console : {:#?}", n.erreurs);

    drop(n);
    let (statut, _) = super::tests::demander("POST", &format!("{racine}/arret"), Some("j"));
    assert_eq!(statut, 200);
    fil.join().unwrap();
}

/// La vue écrite à la main du compteur à graphe de clic : elle reçoit la case
/// `n`, le dérivé `double` et le gestionnaire `onClick` du graphe.
const VUE_DU_COMPTEUR: &str = r#"export function Compteur({ n, double, onClick }: any) {
  return <button id="compteur" onClick={() => onClick(null)}>Compte : {n} ({double})</button>;
}
"#;

/// Le comportement du compteur, en graphe de clic : un clic ajoute 1 à `n`,
/// `double` en dérive.
const GRAPHE_DU_COMPTEUR: &str = r#"graph LR
    clic["Evenement(nom='onClick')"]
    n["Etat(nom='n', schema='{"type":"number"}', initial=0)"]
    plus["Pur(expr='n + 1')"]
    double["Pur(expr='n * 2')"]
    clic -->|charge:declenche| plus
    n -->|valeur:n| plus
    plus -->|resultat:suivante| n
    n -->|valeur:n| double
"#;

/// **Le graphe de clic d'un composant, dessiné à côté de son rendu, s'allume à
/// chaque clic** (marche 3, (a)). Le composant n'a que sa vue écrite à la main ;
/// son comportement est le graphe, compilé dans le graphe des modules. Un clic
/// compte, et les nœuds traversés (`clic`, `plus`, `n`, `double`) s'allument
/// dans le panneau. Puis le graphe réécrit sur le disque (`n + 10`) se recharge
/// à chaud sans perdre l'état : le clic suivant compte de 1 à 11.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn le_graphe_de_clic_s_allume_a_chaque_clic_dans_l_apercu() {
    const TEMOIN: &str = "le_graphe_de_clic_s_allume_a_chaque_clic_dans_l_apercu";
    let Some(binaire) = navigateur_ou_saut(TEMOIN) else {
        return;
    };
    let Some(paquets) = paquets_ou_saut(TEMOIN) else {
        return;
    };
    let Banc { _d, composants, racine, id, fil } = monter(
        &paquets,
        &[
            ("compteur.tsx", VUE_DU_COMPTEUR),
            ("compteur.clic.mmd", GRAPHE_DU_COMPTEUR),
            (
                "compteur.component.json",
                r#"{ "name": "Compteur", "source": "compteur.tsx", "clic": "compteur.clic.mmd", "libs": { "react": "19.2.0" } }"#,
            ),
        ],
    );
    let mut n = Navigateur::lancer(&binaire);
    n.aller(&format!("{racine}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
    n.aller(
        &format!("{racine}/ide/apercu/{id}/Compteur"),
        "document.querySelector('#compteur')?.textContent === 'Compte : 0 (0)' && document.querySelectorAll('#clic g.etape').length === 4",
    );
    assert!(n.erreurs.is_empty(), "le premier rendu : erreurs à la console : {:#?}", n.erreurs);

    n.evaluer("document.querySelector('#compteur').click()");
    n.attendre("document.querySelector('#compteur')?.textContent === 'Compte : 1 (2)'", "le clic compté par le graphe");
    n.attendre(
        "window.__apercu.clics.length === 1 && window.__apercu.clics[0].noeuds.join(',') === 'clic,plus,n,double' \
         && document.querySelectorAll('#clic g.etape.allume').length === 4",
        "les quatre nœuds traversés, allumés",
    );

    // Le graphe réécrit : le clic suivant ajoute 10, l'état (1) est gardé.
    std::fs::write(composants.join("compteur.clic.mmd"), GRAPHE_DU_COMPTEUR.replace("n + 1", "n + 10")).unwrap();
    n.attendre("window.__apercu.mises_a_jour >= 1", "le graphe rechargé à chaud");
    n.evaluer("document.querySelector('#compteur').click()");
    n.attendre("document.querySelector('#compteur')?.textContent === 'Compte : 11 (22)'", "l'état gardé, le graphe neuf");
    let apres = n.evaluer("({ recharge: window.__apercu.recharge, erreurs: document.querySelectorAll('pre.erreur').length, clics: window.__apercu.clics.length })");
    assert_eq!(apres["recharge"], Value::Null, "la page s'est rechargée entière : {apres}");
    assert_eq!(apres["erreurs"], 0, "{apres}");
    assert_eq!(apres["clics"], 2, "{apres}");
    assert!(n.erreurs.is_empty(), "erreurs à la console : {:#?}", n.erreurs);

    drop(n);
    let (statut, _) = super::tests::demander("POST", &format!("{racine}/arret"), Some("j"));
    assert_eq!(statut, 200);
    fil.join().unwrap();
}

/// **Une proposition se voit, se joue, puis s'applique dans l'IDE** (page 08,
/// de bout en bout). Le graphe de clic du compteur est proposé autrement
/// (`n + 5`), comme un agent le proposerait par l'hôte : la proposition paraît
/// dans la liste par le flux ; son détail montre le texte neuf et le nœud
/// `plus` changé sur le dessin du graphe ; l'aperçu de la proposition compte
/// par cinq **sans que rien ne soit écrit** ; Appliquer l'écrit et le commite
/// avec sa raison. Une seconde proposition est rejetée avec son pourquoi.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn une_proposition_se_voit_se_joue_puis_s_applique_dans_l_ide() {
    const TEMOIN: &str = "une_proposition_se_voit_se_joue_puis_s_applique_dans_l_ide";
    let Some(binaire) = navigateur_ou_saut(TEMOIN) else {
        return;
    };
    let Some(paquets) = paquets_ou_saut(TEMOIN) else {
        return;
    };
    let Banc { _d, composants, racine, id, fil } = monter(
        &paquets,
        &[
            ("compteur.tsx", VUE_DU_COMPTEUR),
            ("compteur.clic.mmd", GRAPHE_DU_COMPTEUR),
            (
                "compteur.component.json",
                r#"{ "name": "Compteur", "source": "compteur.tsx", "clic": "compteur.clic.mmd", "libs": { "react": "19.2.0" } }"#,
            ),
        ],
    );
    // Le dossier du backend en dépôt : appliquer commite.
    let dossier = composants.parent().unwrap().to_path_buf();
    for args in [&["init", "-q"][..], &["config", "user.name", "Personne Test"], &["config", "user.email", "personne@example.org"]] {
        assert!(std::process::Command::new("git").arg("-C").arg(&dossier).args(args).status().unwrap().success());
    }
    let poster = |route: &str, corps: Value| -> Value {
        let mut r = crate::daemon::agent()
            .post(format!("{racine}/ide/propositions{route}"))
            .header("Cookie", "rag3weaver_ide=j")
            .header("content-type", "application/json")
            .send(corps.to_string())
            .unwrap();
        serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap()
    };
    let mut n = Navigateur::lancer(&binaire);
    n.aller(&format!("{racine}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
    n.aller(
        &format!("{racine}/ide/code/{id}/propositions/"),
        "document.querySelector('#vue h1')?.textContent === 'Propositions' && document.getElementById('lien')?.textContent === 'flux : ouvert'",
    );

    // L'agent propose ; la proposition paraît par le flux.
    let neuf = GRAPHE_DU_COMPTEUR.replace("n + 1", "n + 5");
    let p = poster("", json!({ "backend": id, "files": [{ "path": "components/compteur.clic.mmd", "content": neuf }], "reason": "compter par cinq" }));
    assert_eq!(p["ok"], true, "{p}");
    assert_eq!(p["result"]["refus"], Value::Null, "{p}");
    let pid = p["result"]["id"].as_u64().unwrap();
    n.attendre(&format!("document.querySelector('#propositions tr[data-id=\"{pid}\"]')"), "la proposition dans la liste, par le flux");

    // Le détail : le texte neuf, et le nœud changé sur le dessin.
    n.aller(
        &format!("{racine}/ide/code/{id}/propositions/{pid}"),
        "[...document.querySelectorAll('.diff .plus')].some(l => l.textContent.includes('n + 5')) \
         && document.querySelector('section.graphe-propose[data-cible=\"component:Compteur\"] g.etape.change[data-nom=\"plus\"]')",
    );

    // L'aperçu de la proposition compte par cinq ; le disque n'a pas changé.
    n.aller(
        &format!("{racine}/ide/apercu/{id}/Compteur?proposition={pid}"),
        "document.querySelector('#compteur')?.textContent === 'Compte : 0 (0)'",
    );
    n.evaluer("document.querySelector('#compteur').click()");
    n.attendre("document.querySelector('#compteur')?.textContent === 'Compte : 5 (10)'", "le comportement proposé, joué sans être appliqué");
    assert!(std::fs::read_to_string(composants.join("compteur.clic.mmd")).unwrap().contains("n + 1"), "l'aperçu a écrit");

    // Appliquer : écrit, en service, commité avec sa raison.
    n.aller(&format!("{racine}/ide/code/{id}/propositions/{pid}"), "document.querySelector('#appliquer')");
    n.evaluer("document.querySelector('#appliquer').click()");
    n.attendre("document.querySelector('#proposition-etat')?.textContent.startsWith('appliquée')", "la proposition appliquée");
    assert!(std::fs::read_to_string(composants.join("compteur.clic.mmd")).unwrap().contains("n + 5"));
    let log = std::process::Command::new("git").arg("-C").arg(&dossier).args(["log", "-1", "--format=%s"]).output().unwrap();
    assert!(String::from_utf8_lossy(&log.stdout).contains("compter par cinq"), "{log:?}");

    // Une seconde, rejetée avec son pourquoi.
    let q = poster("", json!({ "backend": id, "files": [{ "path": "components/compteur.tsx", "content": VUE_DU_COMPTEUR.replace("Compte", "Total") }], "reason": "un autre libellé" }));
    let qid = q["result"]["id"].as_u64().unwrap();
    n.aller(&format!("{racine}/ide/code/{id}/propositions/{qid}"), "document.querySelector('#rejeter')");
    n.evaluer("document.querySelector('#pourquoi').value = 'pas maintenant'; document.querySelector('#rejeter').click()");
    n.attendre("document.querySelector('#proposition-etat')?.textContent === 'rejetée : pas maintenant'", "la proposition rejetée");
    assert!(std::fs::read_to_string(composants.join("compteur.tsx")).unwrap().contains("Compte"), "un rejet n'écrit rien");
    assert!(n.erreurs.is_empty(), "erreurs à la console : {:#?}", n.erreurs);

    drop(n);
    let (statut, _) = super::tests::demander("POST", &format!("{racine}/arret"), Some("j"));
    assert_eq!(statut, 200);
    fil.join().unwrap();
}
