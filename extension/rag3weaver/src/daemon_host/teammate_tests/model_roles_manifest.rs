//! **The folder's manifest says which model plays a role, over the user's configuration** (28 Nov 2026, the third
//! level of the roles: shipped, the user's configuration, then the folder). Its key `roles` takes the place of the
//! roles it names. By the host's own path, the helper of `model_roles`. A module of its own.

use super::model_roles::{a_thanks, equipier, fake_sse, knows_it};
use serde_json::json;

#[test]
fn the_manifests_roles_take_the_place_of_the_users() {
    let users = fake_sse::FakeServer::start_sequence(vec![knows_it()]);
    let folders = fake_sse::FakeServer::start_sequence(vec![knows_it()]);
    a_thanks(
        json!([equipier(), { "name": "y", "access": "compatible", "provider": "openai", "model": "faux-y", "address": folders.url }]),
        json!({ "small": ["x"] }),
        json!([{ "name": "x", "access": "compatible", "provider": "openai", "model": "faux-x", "address": users.url }]),
        json!({ "small": ["y"] }),
    );
    assert_eq!(folders.request.try_iter().count(), 1, "the folder's manifest says small → y");
    assert_eq!(users.request.try_iter().count(), 0, "over the user's small → x");
}
