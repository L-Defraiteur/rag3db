# Un test de l'hôte attend son hôte par un client sans délai

- **État** : corrigé (9 novembre 2026, branche delai-des-temoins-de-l-hote, archi), demandé par
  c7 après un second figement de la lib entière, le 9 (voir « La fermeture »).
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

## La fermeture (9 novembre 2026)

Second figement le 9 : la lib entière tenue 25 min par
`daemon_host::tests::l_hote_inactif_s_arrete_seul_et_une_sonde_ne_le_retient_pas`. Ce témoin sonde
`/sante` par `demander` pendant les 600 ms qu'un hôte met à s'arrêter seul. Sous la charge, une
sonde retardée tombe dans l'arrêt même : connexion acceptée, réponse jamais venue, et l'agent sans
délai attend sans fin. Joué seul, le témoin est vert (5 sur 5) : le figement demande la charge.

- **Le témoin** : `daemon_host::tests::la_sonde_d_un_temoin_rend_la_main_devant_un_hote_muet`. Un
  faux hôte accepte et ne répond jamais. Rouge avant le correctif (la sonde attend encore à
  20 s), vert après : elle panique en nommant la route et son délai.
- **`demander`** est borné à 10 s en tout (`DELAI_D_UNE_SONDE`), et ses paniques disent
  « `<méthode> <url>` : sans réponse après 10 s ».
- **Les autres clients des témoins de l'hôte** passent par `crate::daemon::agent_de_temoin()`
  (`cfg(test)`). La connexion, l'envoi et l'attente de la réponse y sont bornés à 120 s ; la
  lecture du corps ne l'est pas, pour ne pas couper un flux de l'IDE qu'un témoin lit. Cela
  couvre `daemon_host.rs`, `taches/tests.rs`, `chat/tests.rs`, `chat_qui_code.rs`,
  `apercu_a_chaud.rs`, `navigateur.rs`, `ide_sur_un_depot.rs`, `mes_recherches.rs`,
  `teammate_tests.rs`, `symboles_cites.rs` et `worktrees.rs`.
- **Le témoin de l'hôte inactif** ne dépend plus d'une sonde qui tombe dans l'arrêt. Une sonde en
  erreur arrête les sondes, et le délai d'arrêt se juge depuis la dernière sonde.
- **Hors de ce ticket** : les clients de production sans délai (`code_tree.rs`,
  `daemon_client.rs`, `daemon_lifecycle.rs`, `daemon/db.rs`) restent tels quels. Ce sont des
  choix de conception, à trancher à part.
