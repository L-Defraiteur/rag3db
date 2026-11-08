# Un test de l'hôte attend son hôte par un client sans délai

- **État** : ouvert (8 novembre 2026), pour l'arrière-plan (relevé par l'arbre principal dans la pile d'un figement).
- **Famille** : produit
- **Gravité** : passe bloquée (un rouge devient un figement : le binaire de tests ne finit plus).
- **Atteignable en service** : non (le test) ; le client est celui des tests de l'hôte.
- **Touche rag3weaver** : oui (`src/daemon_host/taches/tests.rs`, et les tests qui joignent un hôte par `crate::daemon::agent()`).

## Ce que c'est

`daemon_host::taches::tests::une_tache_ouverte_survit_a_l_arret_de_l_hote_et_se_fusionne` joint
l'hôte qu'il a lancé par un client `ureq` **sans délai** (`crate::daemon::agent()`). Quand l'hôte ne
répond plus, le test attend sans fin au lieu de rougir : la pile du figement du 8 novembre le montre
en `RequestBuilder::send` → `TcpStream::read` → `recv`, le seul fil de la lib dans un client HTTP.
D'autres tests de l'hôte prennent le même agent (`daemon_host.rs`, `daemon_host/chat_qui_code.rs`,
`daemon_host/apercu_a_chaud.rs`, `daemon_host/navigateur.rs`).

## Le témoin à écrire

Un hôte de test qui accepte et ne répond plus ; le test qui le joint doit rougir en un délai borné
(quelques secondes), avec un message qui nomme l'hôte et la route.

## Ce qu'il faut pour le fermer

Un délai de lecture sur le client des tests de l'hôte (`crate::daemon::agent_with_timeout`, une
valeur dite), et le témoin ci-dessus.
