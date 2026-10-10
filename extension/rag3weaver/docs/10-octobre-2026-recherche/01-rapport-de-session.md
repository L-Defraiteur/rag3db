# Session recherche — rapport (chantier C : l'exécution asynchrone des graphes)

## État à la pause du 10 octobre (redémarrage du poste, ~12h30)

**Rien n'est codé.** La branche `execution-asynchrone` est poussée telle
quelle (= master `27eeb6a7f`), worktree `rag3db-async`, target sous
`~/.cache/rag3weaver-build/target-async`. Les mesures AVANT (2 bancs en une
tenue de mesure, 2 batteries backend en lourd) ont été interrompues par la
pause : À REJOUER EN PREMIER à la reprise, avant toute modification.

## Les décisions actées (toutes validées par l'orchestration)

- **Deux écarts de la page 02 corrigés avant de coder** (je dois éditer la
  page dans mon commit, §3.2, avec « la page disait X, le code faisait Y ») :
  (1) run_level PARALLÉLISE DÉJÀ les niveaux par threads de portée — la
  forme : niveau tout-sync = run_level inchangé ; niveau avec un nœud async =
  voie async (join des execute_async, les sync séquentialisés sous
  block_in_place, dit) ; un TÉMOIN À NIVEAU MIXTE exigé (la séquentialisation
  vue, pas supposée). (2) `execute` GARDE son nom (pont synchrone, block_on
  d'un runtime global paresseux — multi-fil, 2 fils, né au premier graphe,
  coût à noter au rapport) ; `execute_async` est le cœur ; aucun appelant ne
  bouge (12 sites de production, dizaines de tests).
- **Le trait** : execute à défaut-erreur « ni execute ni execute_async »,
  execute_async à défaut block_in_place(execute), marqueur is_async() ; test
  du registre : tout nœud marqué async rend l'erreur-défaut sur execute (le
  mensonge inverse non détectable sans exécuter, documenté). Essayer
  #[async_trait] d'abord ; repli : défaut manuel Pin<Box<dyn Future>> si les
  78 impl rechignent.
- **Postgres (5 block_on)** : le RUNTIME DÉDIÉ possédé par la connexion
  (current_thread) — constructible hors tokio, sûr sous block_in_place et
  dans execute ; le pool INCHANGÉ (dit à F : sa proposition §4.1 de session
  tenue reste valable). PLUS les deux bras `CypherValue::Typed` manquants
  (lignes 117 cypher_to_json et 155 cypher_to_pg_param — `--features
  postgres` ne compile pas sur master, signalé par F, je le prends).
- **Le réacteur** : tâche sur le runtime global, son block_on disparaît,
  JoinHandle tokio dans ReactorHandle.
- **La boucle d'agent en tâche + les trois pièces du chantier A** (forme (b)
  actée avec lui — son nœud rend tout de suite poignée/chemins/argv, le
  moniteur poste la fin) : ctx poignée d'appel (« #cmd-N », la boucle
  numérote CHAQUE appel), le poste-à-la-boîte (Sender/bus en service du
  graphe), le RunScope (objet de portée, fermetures de mort vidées à la fin
  du run AVANT le join). Le trait ToolBox (agent.rs, à moi) gagne call_with
  (invocation) à défaut. Llm::generate dans la boucle : ponté par
  block_in_place. chat.rs branche with_events/with_inbox en fin de lot
  (zones disjointes du chantier E, acté avec rag3db-83).
- **Mesure avant/après** : 2 bancs (une tenue de mesure) + 2 batteries
  backend (lourd), même régime avant et après.

## Comment reprendre

1. Rejouer les mesures AVANT (la chaîne est dans l'historique de session :
   bancs granite+creux ×2 en RAG3WEAVER_MESURE=1 enchaînés, batteries
   test_backend_code.py ×2 en poste lourd, worktree rag3db-async,
   RAG3DB_ROOT=arbre principal).
2. Étape 1 : rt.rs (runtime global paresseux) + trait + runtime async +
   execute pont + témoin async + témoin niveau mixte + test du registre.
3. Étape 2 : boucle en tâche + 3 pièces d'A (lui envoyer « prêt » avec les
   noms) + réacteur en tâche.
4. Étape 3 : postgres (runtime dédié + bras Typed) ; prévenir F au push.
5. chat.rs (with_events/with_inbox) ; batterie complète ; mesures APRÈS ;
   corriger la page 02 §3.2 dans le commit ; rapport.

Les coordinations à jour : A attend mon « prêt » (il code sa part
indépendante sur sa branche) ; E part de master (zones disjointes) ; F
attend mon push postgres.
