# `maintain` et `fts-live-ids` refusent un index plein texte qui n'est pas encore ouvert

- **État** : ouvert (18 novembre 2026), relevé par arbre principal dans le travail de Codex du 16 novembre.
- **Famille** : produit
- **Gravité** : la maintenance de lucivy (purge des orphelins, compact) est le premier pas de la bascule des bases de
  Lucie ; elle échoue en HTTP 422 sur un backend fraîchement ouvert par l'hôte, et la bascule s'arrête là.
- **Atteignable en service** : oui (le 16 novembre, `code-rag3db-depot` au portable, juste après la pose de la lib
  59ac9c86bc et le redémarrage de l'hôte).
- **Touche rag3weaver** : oui (`src/catalog.rs` `fts_handle_nomme`, ~1419, appelé par `fts_live_ids` et
  `fts_releve_a_blanc` ; les routes `POST /backend/fts-live-ids` et `POST /backend/maintain` de `src/daemon_host.rs`,
  ~1014-1060).
- **Pour** : lucivy (les routes de maintenance), avec G (l'hôte).
- **Signalé par** : la session Codex d'arbre principal, le 16 novembre (branche `cx-arbre-bascule-bases-20261116-r1`,
  ticket `docs/arbre-principal/tickets/2026-11-16-basculer-les-trois-bases-vers-les-generations.md` de cette
  branche) ; requalifié par arbre principal le 18.

## Ce que c'est

Le catalogue ouvre ses index plein texte à la demande : `fts_handles` ne contient une table qu'après une lecture qui
l'a ouverte. `fts_handle_nomme` ne fait que chercher dans cette table, et refuse sinon :
« pas d'index plein texte ouvert pour « File » (ouverts : []) ». Les routes `fts-live-ids` et `maintain` (y compris
en essai à blanc, `dry_run: true`) passent par lui. Sur un hôte qui vient d'ouvrir le backend, sans recherche
encore faite, toute maintenance est donc refusée en 422.

Une note de recette, de même racine, qui n'est pas un défaut : juste après un redémarrage de l'hôte, aucun backend
n'est hébergé, et `POST /backend/close` rend `NOT_HOSTED`. La recette de la bascule ouvre le backend (un appel) avant
de le fermer.

## Le correctif attendu

Les routes de maintenance ouvrent l'index plein texte de la table demandée s'il ne l'est pas encore, par le même
chemin que la recherche, au lieu de refuser. Une table qui n'a pas d'index plein texte du tout reste un refus nommé,
distinct (« la table « X » n'a pas d'index plein texte »).

## Le témoin

Rouge d'abord : un backend ouvert sans aucune recherche, puis `fts_live_ids("File")` (et `fts_releve_a_blanc`) ;
aujourd'hui « pas d'index plein texte ouvert ». Puis le même appel rend les vivants et le jeton. Le lot touche
`src/catalog.rs` : la passe joue `e2e_code_sync` (`tools/poste/required_suites.txt`).
