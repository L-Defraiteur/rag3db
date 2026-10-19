# Knowledge dump de la session mémoire longue — 19 octobre 2026

Ce qu'une session qui reprend doit savoir, et que le code ne dit pas seul.

## Le journal des écritures et la publication

- `src/ecritures.rs` : le journal du drain. `InsertRecordNode`, `LinkRecordNode` et
  `DeleteRecordNode` y notent ce qu'ils ont écrit.
- Le catalogue le prend au début de chaque drain, pour le vider, et le jette à l'échec. Au
  succès, il le publie (`publier_le_journal`).
- Il n'est prêté qu'aux graphes d'écriture, par `enregistrer_les_services_d_ingestion` et par le
  graphe de `poser_la_donnee`. Il n'est **jamais** prêté par `register_search_services`, qui
  sert aussi la vue de lecture.
- `Catalog::publier` :
  - pose le run du fil s'il manque ;
  - retient sous une transaction **ou** sous une fenêtre ouverte par
    `retenir_les_evenements` (hors transaction) ;
  - fond les `RelationsChanged` retenus d'une même relation en état net
    (`Event::fondre_relations`).
- Au commit, `publier_les_evenements_retenus` ; au rollback, `jeter_les_evenements_retenus`.
- Tronquées, les listes ne se fondent qu'en ce qu'elles montrent : `count` devient un majorant,
  et `truncated` dit de relire par la base. Le miroir régénère alors tout.

## Les liens neufs

- **Chemin COPY** (au moins 200 paires d'une relation à bouts déclarés dans un drain) :
  `copier_les_liens` rend les indices écrits.
- **Chemin par lots** : `paires_posees` (`existing_links` par départs) avant et après chaque
  tranche. Seules les relations de `config.relations` paient ces lectures.
- Pour savoir quel chemin a servi : `RAG3WEAVER_INGEST_PROFILE=1` imprime les lignes
  `[link-profile]`.
- Piège : `link` sous l'exigence par défaut pose tout de suite, une paire par drain, donc
  jamais par COPY. Un témoin du chemin en masse met en file par `link_jusqu_a(…,
  Disponibilites::AUCUNE)`, puis draine.

## La déliaison

- `RelationRecord.retirer` : la même file et le même nœud que le lien. `LinkRecordNode` traite
  les retraits après les poses. Il ne retire que ce qui existait, et l'annulation le repose
  (clé `__retires` de l'undo).
- `Catalog::annuler_en_file` : un lien et une déliaison d'une même paire résolue s'annulent ; la
  dernière demande gagne. Une paire dont un bout est une référence non résolue n'est pas
  comparée : l'ordre du drain (poses, puis retraits) suffit.
- `code_sync` : `oublier_les_aretes_du_fichier` relit par `edges_to_detach`, détache, puis
  publie par `publier_les_deliaisons`. `reingest_file` et `reingest_files` sont enveloppés
  d'une fenêtre de retenue.

## Le run

- `events::run_du_fil` / `pendant_le_run` : une variable de fil, posée autour de l'exécution de
  chaque nœud à trois endroits :
  - l'`execute_async` par défaut (`node.rs`) ;
  - le chemin synchrone de `concurrent.rs` ;
  - `run_level` (`runtime.rs`).
- Un nœud asynchrone qui rend la main entre deux `await` ne la garde pas. Aucun nœud
  d'écriture ne l'est aujourd'hui.

## Les postes

- À luciepc, `RAG3DB_LIB=courant` vaut cf71c929d (finalisation du COPY, COPY de relations
  annulé). C'est celle qu'il faut pour juger le chemin des liens ; 87a362e30 ne l'a pas.
- Un target repris peut rendre un vert vide. Exiger la ligne `Checking rag3weaver` (`touch
  src/lib.rs` avant) : le 19 à 03 h, un premier contrôle n'avait vérifié que les dépendances.
- Sous fish (le shell de luciepc), `$|` dans un motif de grep casse la commande. Copier le
  journal ici par scp, puis le lire.
- L'arbre de code-lourd est `~/git_workspaces/rag3db-lourd`. Il n'est que lu par la mesure ;
  son HEAD et son état sont relevés avant et après.
