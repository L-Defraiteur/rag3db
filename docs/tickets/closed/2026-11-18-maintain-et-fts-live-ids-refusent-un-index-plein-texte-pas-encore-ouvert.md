# `maintain` et `fts-live-ids` refusent un index plein texte qui n'est pas encore ouvert

- **État** : corrigé (18 novembre 2026), sur master en 9bb06d5c80 (lot de lucivy, branche `fts-open-on-demand-6`).
  `Catalog::fts_handle_nomme` ouvre l'index plein texte à la demande, par `open_fts_handles_for` comme la recherche ;
  `fts_live_ids` et `fts_releve_a_blanc` prennent `&mut self`. Une table sans plein texte est refusée par son nom
  (« la table « X » n'a pas d'index plein texte »), et un index à rebâtir n'est pas maintenu. La maintenance nocturne,
  de même racine, ouvre d'abord les index du mode fichiers sans rien rebâtir (`open_fts_files`) : juste après un
  redémarrage, elle ne voyait aucune table. Témoins `e2e_demon_hote::maintenance_routes_open_the_index_after_a_host_restart`
  et `nightly_maintenance_covers_tables_no_search_has_opened`. Ils étaient rouges sur f2bb4c1ba5 (« pas d'index plein
  texte ouvert pour « Fiche » (ouverts : []) » ; la nuit ne maintenait aucune base), et le témoin de nuit corrigé à
  30 fiches a été revu rouge contre le catalog.rs de master. Passe sur la lib 3d706f709 sans drapeau, tout vert,
  e2e_code_sync comprise.
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
