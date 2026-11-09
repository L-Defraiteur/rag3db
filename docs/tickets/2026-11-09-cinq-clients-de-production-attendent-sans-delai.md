# Cinq clients de production attendent sans délai

- **État** : ouvert (9 novembre 2026), relevé par archi en bornant les témoins de l'hôte
  (`closed/2026-11-08-un-test-de-l-hote-attend-son-hote-par-un-client-sans-delai.md`). Une
  question de conception attend Lucie.
- **Famille** : produit
- **Gravité** : figement possible. Un hôte qui accepte la connexion et ne répond plus tient
  l'appelant sans fin : outil MCP, CLI, sonde de cycle de vie.
- **Atteignable en service** : oui en principe, par exemple un hôte qui s'arrête au milieu d'une
  requête ou un fil de service bloqué. Aucun figement en service n'a encore été relevé sur ces
  chemins ; c'est la forme du code qui le permet, et un témoin l'a montré pour les tests le
  9 novembre.
- **Touche rag3weaver** : oui (`src/code_tree.rs`, `src/daemon_client.rs`,
  `src/daemon_lifecycle.rs`, `src/daemon/db.rs`).

## Ce que c'est

Ces clients prennent `crate::daemon::agent()`, qui n'a aucun délai, alors que
`crate::daemon::agent_with_timeout` existe à côté (ticket du 11 octobre 2026, un `/embed` muet) :

- `code_tree.rs:382`, `host_state` : `GET /etat`, dont la réponse vaut `None` quand l'hôte ne
  répond pas. Sans délai, un hôte muet n'est jamais « ne répond pas ».
- `daemon_client.rs:177`, `attach_as_sent` : l'agent du client MCP de l'hôte, gardé pour **tous**
  les appels qui suivent. Certains durent légitimement longtemps, comme un index ou un tour qui
  attend l'embarquement.
- `daemon_lifecycle.rs:468`, `answers` : la sonde `/sante` qui décide si l'hôte de `daemon.json`
  répond. Sans délai, elle peut tenir le démarrage d'un client.
- `daemon_lifecycle.rs:605`, `attendre_l_unite` : la sonde qui réveille l'unité systemd, avant la
  boucle bornée par `LAUNCH_WAIT`. La boucle est bornée, la sonde ne l'est pas.
- `daemon/db.rs:320`, `depuis` : l'agent du client du démon de base, gardé pour tous ses appels.

## La question pour Lucie

Faut-il un délai de connexion, d'envoi et d'attente de la réponse partout, en laissant la lecture
du corps libre pour les flux (la forme d'`agent_de_temoin` dans les tests) ? Et faut-il, pour les
sondes (`answers`, `host_state`, la sonde de réveil), un délai global court de l'ordre de
quelques secondes ? Pour les deux agents gardés (`daemon_client`, `daemon/db`), l'attente de la
réponse d'un appel long est le point à trancher : un délai par outil, ou une réponse tôt suivie de
`wait_output`.

## Le témoin à écrire

Le faux hôte du témoin des tests (`daemon_host::tests::la_sonde_d_un_temoin_rend_la_main_devant_un_hote_muet`),
qui accepte et ne répond jamais. Chacun des cinq chemins doit rendre la main dans son délai, en
le disant.

## Ce qu'il faut pour le fermer

La décision de Lucie, les délais posés selon elle, et le témoin pour chaque chemin.
