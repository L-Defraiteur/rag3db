//! **The IDE's routes that grant answer only a human** (c7, 22 Nov 2026, 22 h 45: the IDE's gate lets the host's token in,
//! and an agent that holds it could accept a card, apply a proposal or merge a task). A yes on `/ide/accords`,
//! `/ide/propositions/appliquer` and `/ide/taches/fusionner` asks for the user's title (her cookie or the terminal's
//! header); without it, the route refuses, before anything else. A module of its own.

use serde_json::{json, Value};

const REFUS: &str = "réservé à une réponse humaine";

#[test]
fn the_routes_that_grant_refuse_a_caller_without_the_users_title() {
    let (adresse, _fil) = super::tests::hote_garde_avec("j", |h| h.with_user_token("u".to_string()));
    let racine = format!("http://{adresse}");
    let poster = |route: &str, corps: Value, humain: bool| -> String {
        let mut r = crate::daemon::agent_de_temoin().post(format!("{racine}{route}")).header("Cookie", "rag3weaver_ide=j").header("content-type", "application/json");
        if humain {
            r = r.header("X-Rag3weaver-User", "u");
        }
        let mut reponse = r.send(corps.to_string()).unwrap();
        reponse.body_mut().read_to_string().unwrap()
    };
    let routes = [
        ("/ide/accords", json!({ "id": "x", "decision": "accorder" })),
        ("/ide/propositions/appliquer", json!({ "backend": "x", "id": 1 })),
        ("/ide/taches/fusionner", json!({ "backend": "x", "nom": "t", "message": "m" })),
    ];
    for (route, corps) in &routes {
        let agent = poster(route, corps.clone(), false);
        assert!(agent.contains(REFUS), "{route} with the host's token only: {agent}");
        let humain = poster(route, corps.clone(), true);
        assert!(!humain.contains(REFUS), "{route} with the user's title passes the check: {humain}");
    }
    // A no is not a grant: refusing stays open to any caller.
    let refuser = poster("/ide/accords", json!({ "id": "x", "decision": "refuser" }), false);
    assert!(!refuser.contains(REFUS), "{refuser}");
}
