# Session recherche — rapport (chantier C : l'exécution asynchrone des graphes)

## État au 10 octobre au soir

Branche `execution-asynchrone` (worktree `rag3db-async`, target sous
`~/.cache/rag3weaver-build/target-async`), deux commits poussés, étape 3 en
cours :

- **Étape 1 — poussée (`d19eb903d`)** : le trait à deux méthodes
  (`execute` à défaut-erreur « ni execute ni execute_async » ;
  `execute_async` à défaut `block_in_place(execute)`, forme manuelle
  `Pin<Box<dyn Future>>`, AUCUN attribut sur les 78 impl ; `is_async()`) ;
  le runtime du crate (`dataflow/rt.rs` : paresseux, multi-fil, 2 fils) et
  son pont `bloquer` à trois bras — fil ordinaire : `block_on` ; runtime
  multi-fil : `block_in_place` + `block_on` (la réentrance du « graphe dans
  le graphe » de l'ingestion est LÉGALE) ; fil unique : refus nommé, jamais
  la panique. `execute`/`execute_as`/`execute_with_report`/
  `execute_with_checkpoint(_mode)` gardent leurs noms en ponts ; un niveau
  tout-sync garde `run_level` au fil près ; un niveau avec un nœud async
  passe par `run_level_async` (séquentiel, dit et VU par le témoin « niveau
  mixte »). Page 02 §3.2 corrigée aux deux endroits. En ligne à part : la
  fixture de `scripts/test_backend_code.py`, rouge sur master depuis le
  filtre des arêtes devinées (forme `use` + appel nu validée par l'arbre
  principal).
- **Étape 2 — poussée (`da25d8e0f`)** : les trois pièces d'A en forme (b) —
  `RunScope` (`defer`/`vider`, vidé par garde de Drop AVANT le join des
  fils ; témoin `la_portee_du_run_tue_avant_de_joindre`), `ToolInvocation
  { handle "#outil-N", bus, inbox, scope }` par `ToolBox::call_with` (au
  défaut → `call_in`, enveloppes relayées) et services de couche
  `tool_handle`/`agent_bus`/`agent_inbox`/`run_scope` ;
  `NodeContext::tool_handle()`. Le réacteur en tâche (`stop()` = arrêt
  ATTENDU par le pont, Drop-abort en filet) ; `watch_bound(tool, arguments,
  nodes_policy)` pour la session mémoire (`watch` inchangé ; la politique
  reste un argument du montage, jamais un champ de manifeste — convenu avec
  rag3db-96). Page 02 §3.3 corrigée : la boucle d'agent RESTE un fil ce lot
  (48 sites `Agent::new` pour un gain nul ; en tâche au run en fond §4.3),
  les trois pièces survivront au changement de véhicule. Lib complète :
  1250/1250. « Prêt » envoyé à rag3db-73 avec les noms définitifs ; signe
  envoyé à rag3db-96.
- **Étape 3 — codée, compilation en cours** : `postgres_connection.rs` — le
  runtime DÉDIÉ possédé par la connexion (current_thread : il ne vit que
  pendant les appels), constructeurs devenus SYNCHRONES (`new`/`connect` —
  plus aucun contexte tokio exigé des appelants), pont à trois bras local
  (refus nommé en `DbError` sur fil unique), pool inchangé ; les bras
  `CypherValue::Typed` manquants (cypher_to_json, cypher_to_pg_param) +
  `denuder()` aux deux points d'inspection qui perdaient un `Typed` en
  silence (`est_liste_de_lignes`, le `filter_map` des identifiants). Le
  harnais de `tests/e2e_postgres.rs` simplifié : `Contexte` (runtime fuité
  + `rt.enter()`) supprimé, 6 sites de construction directs. Prévenir F
  (rag3db-6f) au push.
- **Étape 4 — codée avec l'étape 3** : `chat.rs` branche
  `.with_events(EventBus::new(64)).with_inbox()` sur son agent — le bus par
  lequel un outil en fond poste sa fin, la boîte qui la ramène au tour
  suivant.

## Mesures : l'AVANT est à REJOUER (banc réparé)

rag3db-96 a réparé `e2e_banc_etage` (rouge : la crate `rag3weaver-ir`,
`41b869ba4`, a déplacé 4 des 63 aiguilles hors de `src/`) et prévenu : mes
chiffres AVANT (banc 444,4/429,7 s ; batterie backend 9,93/10,24 s) ne sont
PAS comparables — le corpus `src/` a changé, 2 des 43 questions changent de
cible. À la fin du lot : rejouer « avant » AU COMMIT D'AVANT mon changement
ET « après » en tête de branche, tous deux sur SON banc réparé (attendre son
signe + le hash de son push master), bancs ×2 en une tenue de mesure
exclusive, batteries ×2 en lourd, même montage.

## Ce qui reste

1. Vert de `--features postgres` (check en cours), lib complète, commits
   étape 3 (postgres_connection.rs + e2e_postgres.rs) et étape 4 (chat.rs),
   push ; prévenir rag3db-6f (F).
2. Batterie complète du crate une fois (e2e_code, e2e_dataflow_observe,
   e2e_checkpoint, suites backend et chat) sous poste lourd.
3. Mesures avant/après sur le banc réparé (voir ci-dessus), au signe de
   rag3db-96.
4. Fusion master en avance rapide ; rapport final.

Coordinations : rag3db-73 a mes noms définitifs (« prêt » envoyé) ; F attend
le push postgres ; rag3db-96 convenu sur watch_bound et me fait signe pour
le banc.
