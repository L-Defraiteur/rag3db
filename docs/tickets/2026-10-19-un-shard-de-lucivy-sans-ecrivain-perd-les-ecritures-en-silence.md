# Un shard de lucivy sans écrivain perd les écritures en silence

- **État** : ouvert (19 octobre 2026, G, à la demande de l'orchestration). **Le correctif vit dans
  lucivy-core**, la bibliothèque de Lucie : une version à publier par elle. Rien n'est proposé
  vers son dépôt sans son mot. Correctif prêt, non poussé : commit `bb07627` de la branche locale
  `correctifs-rag3db-19-octobre` (worktree `~/git_workspaces/lucivy-figement-G`), avec le correctif
  du figement pour une seule version. Témoin `test_writes_to_a_shard_without_writer_are_reported_by_commit`,
  rouge sur v4.3.0, vert avec. Le choix revient à Lucie :
  `extension/rag3weaver/docs/optimiseur/19-octobre-2026-02h05/04-lucivy-deux-correctifs-a-ton-choix.md`.
- **Gravité** : perte silencieuse. Un document indexé ou retiré n'atteint pas l'index, et l'appel a
  rendu `Ok`.
- **Atteignable en service** : pas aujourd'hui. rag3weaver garde un écrivain ouvert sur chaque shard
  tant qu'un backend l'est. Le défaut devient atteignable dès qu'un appelant lâche un écrivain
  (`LucivyHandle::close()` d'un shard, ou `writer.lock().take()`) et que le `ShardedHandle`
  continue de servir : par exemple pour rendre la mémoire au repos, ce qu'on a envisagé le 19
  octobre.
- **Touche rag3weaver** : oui, comme appelant de lucivy-core (`fts_handle.rs`, les nœuds d'écriture).

## Ce que c'est

Dans lucivy-core 4.3.0, l'acteur d'un shard (`spawn_shard_pool`) traite les messages d'écriture
seulement s'il a un écrivain, et ne dit rien sinon.

- `src/sharded_handle.rs:865-874`, `ShardMsg::Insert` : `if let Some(writer) = guard.as_mut() {
  … }`. Sans écrivain, le document est jeté. Le routeur a déjà enregistré le `node_id` → shard
  (vers la ligne 1965) et `add_document` a rendu `Ok` à l'appelant.
- Les mêmes lignes jettent aussi le résultat de `writer.add_document(doc)` et de
  `add_document_pre_tokenized` (`let _ = …`) : une erreur d'ajout se perd même avec un écrivain.
- `src/sharded_handle.rs:989-995`, `ShardMsg::Delete` : sans écrivain, la suppression est sautée.
- `ShardMsg::Commit` (lignes 893-896 et 929-932) rend `Ok(())` sans `mark_committed`. Le shard
  reste « avec des changements non validés », et un export qui le vérifie (`snapshot.rs:26`) le
  refusera plus tard, loin de la cause.

Ce qui devrait se passer : une écriture qui ne peut pas aboutir est une erreur rendue à l'appelant
(« shard N : pas d'écrivain »), comme le fait déjà `commit_direct` (« writer closed », ligne 2959).

## Recette

En test de lucivy-core, sur un `ShardedHandle` de deux shards :

1. Ouvrir le handle, puis lâcher l'écrivain d'un shard : `handle.shard(0).close()`.
2. Ajouter, par `ShardedHandle::add_document`, des documents dont le routeur envoie au moins un au
   shard 0, puis appeler `commit()` : les deux appels rendent `Ok`.
3. Chercher le document routé vers le shard 0 : il est absent, sans erreur nulle part.
4. Même chose pour `delete_by_node_id` d'un document du shard 0 : il reste trouvable.

## Pour le fermer

Dans lucivy-core : un `Insert` ou un `Delete` sur un shard sans écrivain rend une erreur nommée à
l'appelant (la réponse passe par l'acteur, ou bien le `ShardedHandle` refuse avant d'envoyer) ; le
résultat de `add_document` n'est plus jeté ; un `Commit` sans écrivain dit qu'il n'a rien validé.
Un témoin rejoue la recette et exige l'erreur. Puis rag3weaver prend la version publiée.
