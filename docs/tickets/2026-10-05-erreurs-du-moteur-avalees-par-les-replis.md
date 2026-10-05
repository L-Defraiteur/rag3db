# Des erreurs du moteur avalées par des replis, et l'écriture continue

- **État** : ouvert ; relevé par lecture, rien n'est éprouvé.
- **Gravité** : réponse fausse possible, en silence (selon les cas, voir la liste).
- **Atteignable en service** : oui, sur une erreur interne du moteur (mémoire, état faussé)
  que rien ne reconnaît comme « à rouvrir ».
- **Touche rag3weaver** : oui.

## Ce que c'est

Plusieurs chemins d'écriture transforment une erreur rendue par le moteur en avertissement,
en compte d'échecs ou en `let _ =`, puis continuent d'écrire dans la même base. Seules deux
erreurs arrêtent tout : `REOPEN_AFTER_FAILED_CHECKPOINT` et, depuis le 5 octobre 2026,
`BUFFER_POOL_FULL` (ticket `2026-10-05-copy-refuse-pour-memoire-table-faussee-en-memoire`).
Une autre erreur interne du moteur, qui laisserait son état faux, passerait.

## La liste (lecture du 5 octobre 2026, lignes à cette date)

**L'écriture continue, et c'est risqué :**

1. `dataflow/record_nodes.rs` : le MERGE d'un groupe d'`InsertRecordNode` qui échoue est
   compté (`consigner_l_echec`), ses refs passent en échec, et les autres groupes et les
   nœuds suivants (morceaux, liens, vecteurs, plein texte) continuent. Sous transaction
   par paquet, l'instruction échouée défait la transaction et le COMMIT est refusé, ce qui
   est sûr. Hors transaction, la suite s'écrit.
2. `dataflow/record_nodes.rs` : le MERGE d'une tranche de 5 000 liens qui échoue est
   compté, et la tranche suivante part.
3. `catalog.rs`, `drainer` : un graphe en erreur devient `FlushResult { failed }`, puis
   `flush_blob_store` pousse quand même les blobs du plein texte produits par ce graphe.
4. `catalog.rs`, `embarquer_le_retard` : une réclamation ou un graphe qui échoue sur une
   table passe à la table suivante, puis les blobs sont poussés.
5. `catalog.rs`, `reprendre_une_promesse` : `let _ = mark_fts_pending(..)`, puis
   `let _ = persist_meta_key(FTS_PROMISE, "")`. Si la marque échoue et que l'effacement
   réussit, la promesse disparaît sans marque de rebâti. **C'est un défaut probable** :
   un plein texte en retard serait servi, puis complété. De plus, la promesse est lue par
   `.ok()…unwrap_or_default()`, si bien qu'une lecture qui échoue fait passer une
   promesse orpheline inaperçue.
6. `catalog.rs`, `check_fts_counts` et l'ouverture des fichiers du plein texte : l'index
   ou le dossier est supprimé **avant** la marque, et la marque est posée par
   `let _ = mark_fts_pending`. Si la marque échoue, il reste un plein texte vide ou partiel
   qu'aucune marque durable ne fait rebâtir (seul l'ensemble en mémoire du processus le
   fait).
7. `catalog.rs` : `fts_pending_entities().unwrap_or(false)`. Si la lecture échoue, un
   rebâti interrompu n'est pas repris.

**Moins risqué :**

- `catalog.rs`, `ensure_vector_index` : un CREATE refusé (autre que « detached ») devient
  un avertissement, et l'ingestion continue sans index vectoriel. Les données restent
  justes, mais la recherche par vecteur se dégrade en silence.
- `catalog.rs`, la migration des colonnes de portée : tout message qui contient « exist »,
  « not found » ou « does not » est absorbé, y compris une erreur du moteur qui
  emploierait ces mots.
- Les marques de travail en attente et `relations_pending` sont posées par
  `let _ = persist_meta_key` : une marque périmée, pas une donnée fausse.
- Les COPY refusés pour une autre raison que la mémoire se replient sur MERGE, qui est
  idempotent. C'est sûr tant que l'état du moteur est juste.

**Sûrs**, parce qu'une lecture qui échoue fait seulement réécrire davantage :
`split_unchanged`, le contrôle des empreintes d'`EmbedNode`, le compte de la dette de
vecteurs, et la lecture de `fts_generation`.

## Ce qu'il faudrait

- Pour les points 5 à 7 : propager l'erreur, ou poser la marque **avant** de supprimer,
  pour qu'une marque manquante ne laisse jamais un index faux sans trace.
- Pour les points 1 à 4 : décider si une erreur du moteur qui n'est pas une erreur de
  données (une erreur interne, la mémoire, une exception du gestionnaire de tampon) doit
  arrêter le drain et empoisonner la base, comme les deux erreurs déjà reconnues. C'est un
  choix de conception, pour Lucie et l'orchestration.

## Témoin

Aucun. Pour en écrire un, il faut un crochet de test qui injecte une erreur du moteur à un
endroit donné, dans la lignée de `ReopenState::inject_after`.
