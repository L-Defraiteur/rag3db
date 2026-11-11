# Un refus 4xx du modèle ne se relit pas

- **État** : corrigé le 9 novembre 2026 (session mémoire, 96), dans le lot qui l'ouvre.
- **Famille** : produit
- **Gravité** : un appel refusé sans que personne puisse dire pourquoi.
- **Atteignable en service** : oui, par tout appelant du client du modèle (chat, équipier, juge,
  archiviste).
- **Touche rag3weaver** : oui, `src/openai_llm.rs`, `src/builtin_templates.rs` (`state_root`).

## Ce qu'on voyait

L'archivage à blanc du 9 novembre (`archive` sur une copie de la mémoire de Lucie) : Vertex a
rendu « HTTP 400 INVALID_ARGUMENT, Request contains an invalid argument » à cinq décisions sur
sept du premier document. Le client ne gardait pas la requête, et coupait le corps de l'erreur à
512 caractères ; les essais refaits à la main ne reproduisaient pas le refus. Personne ne pouvait
relire la requête fautive.

## Le correctif

Sur un refus 4xx qui n'est pas rejoué (un 429 l'est), le client range dans
`<racine d'état>/requetes-refusees/<instant>-<statut>-<pid>.json` l'URL, le modèle, le statut, le
corps de la requête tel qu'envoyé et le corps entier de l'erreur. Les entêtes ne sont pas gardés :
l'authentification n'y est jamais. L'erreur rendue se termine par « requête gardée : <chemin> »,
et une ligne le dit au journal de qui appelle. La racine d'état est celle du gabarit vivant
(`builtin_templates::state_root`) : `RAG3WEAVER_STATE_DIR`, sinon `~/.local/state/rag3weaver`.

## Le témoin

`a_4xx_refusal_keeps_the_faulty_request_and_the_whole_error_body` (`tests/openai_llm_retry.rs`) :
un faux serveur rend un 400 dont le corps dépasse 512 caractères ; l'erreur donne le chemin, le
fichier porte la requête et la fin du corps, jamais le jeton. Rouge sur master 3c0ec28b5 (« l'erreur
donne le chemin de la requête gardée »), vert après. Le binaire de ce test pose son dossier d'état
avant `main`, comme le binaire e2e.
