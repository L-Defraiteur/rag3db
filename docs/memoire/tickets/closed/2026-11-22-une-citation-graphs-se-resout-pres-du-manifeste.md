# Une citation `@graphs/` se résolvait près du manifeste, pas près du graphe qui cite

- **État** : corrigé (23 novembre 2026), branche `cx-memoire-cited-graphs`.
- **Famille** : produit
- **Gravité** : une copie locale d'un gabarit cassait à chaque pose d'un hôte qui changeait un graphe livré. Le
  22 novembre au soir, la mémoire longue de Lucie (`memoire-longue`) ne se chargeait plus, et tous ses outils avec elle.
- **Atteignable en service** : oui, avant ce lot. Tout manifeste local qui cite un graphe livré (`builtin:`), quand
  ce graphe cite ses voisins en `@graphs/…`.
- **Touche rag3weaver** : oui. La lecture des sous-graphes cités (`dataflow/foreach_node.rs`) et le chargeur
  (`PreparedBackend::load`).
- **Pour** : la session mémoire (96).
- **Signalé** : le 22 novembre 2026, après la pose de l'hôte ef1cddb8c9 à 21 h 12.

## Ce qu'on voyait

`recall_fiches` échouait : « le nœud « recall » : graphs/archive_recall.mmd :
…/memoire-longue/backends/memory/graphs/archive_recall.mmd : No such file or directory ». La config de Lucie est une
copie du gabarit memory datée du 15 novembre. Son `graphs/archive.mmd` cite le graphe livré `archive_decide.mmd`, qui
cite depuis le 20 novembre son voisin `@graphs/archive_recall.mmd`. La citation se résolvait dans le dossier du
manifeste local, où ce fichier n'existe pas.

## Le remède du soir (réversible)

Le dossier entier a été gardé dans `~/.config/rag3weaver/memoire-longue.avant-22-11-2026`. `backends/memory`,
`backends/notebook` et `tools` ont été rafraîchis depuis les gabarits de l'hôte posé, et les réglages de Lucie
(`database`, `files_root`) remis.

## Le correctif

Une citation `@graphs/…` se résout près de la racine du gabarit du graphe qui cite (`resolve_cited_graphs_from`,
`cited_path`) : un graphe livré lit ses voisins livrés, un graphe local les siens. Le chargeur passe le chemin du
graphe de l'outil. `@builtin:…` et un chemin absolu se lisent tels quels.

## Les témoins (vus rouges d'abord)

- `a_shipped_graph_cites_its_siblings_in_the_shipped_template` et `a_local_graph_cites_its_siblings_next_to_the_manifest`
  (`dataflow/foreach_node/tests/cited_from_their_template.rs`).
- `a_memory_whose_graphs_are_all_shipped_loads_and_answers` (`tests/e2e_archiviste/follows_the_binary.rs`) : un
  manifeste dont tous les graphes sont livrés, sans `graphs/` local, se charge, et `put_fiche`, `get_fiche` et
  `recall_fiches` répondent. Avant le correctif, il rougissait sur la même erreur que la mémoire de Lucie.

## Ce qui reste

Le manifeste de Lucie peut maintenant passer en références `builtin:` (« suit le binaire » pour de bon), avec ses
seuls réglages en local.
