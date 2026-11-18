# La pose des liens de la résolution repasse par MERGE

- **État** : corrigé (18 novembre 2026, branche `codeparsers/link-copy-split`) : dans un groupe, seules les paires déjà en base passent par lots ; les neuves partent par COPY
- **Famille** : produit
- **Gravité** : lenteur d'un premier index : environ 4,4 s sur 7 de `drain · liens` sur rag3weaver/src, 10,6 s sur
  environ 20 sur le dépôt entier
- **Atteignable en service** : oui, à chaque premier index et à chaque réindexation complète
- **Touche rag3weaver** : oui, `src/code.rs` (`materialiser_les_symboles`) et le nœud des liens
  (`dataflow/record_nodes.rs`)
- **Pour** : session codeparsers (la part code.rs du lot C, décision de Lucie du 18 novembre)
- **Signalé par** : session codeparsers, sur les journaux profilés de recherche (`probe-index-c-rw-src.log` et
  `probe-index-c-depot.log` à luciepc, lib 3d706f709, `RAG3WEAVER_INGEST_PROFILE=1`)

## Constat

Le profil par relation (`[link-profile]`) montre que les COPY ne coûtent presque rien : tous ensemble, environ
1,5 s sur rag3weaver/src. En revanche, au dernier drain des liens, celui des arêtes que pose la résolution
(`materialiser_les_symboles`), le nœud des liens écrit :

« CONSUMES : des paires déjà en base reçoivent leurs propriétés : chemin par lots » (et CONSUMED_BY, et sur le
dépôt INHERITS_FROM).

Ces relations repassent alors par `MERGE … SET`, paire par paire :

| arbre | nœud | paires | dont COPY | durée |
|---|---|---|---|---|
| rag3weaver/src | `links` de la fin | 64 979 | 349 | 4 368 ms |
| dépôt entier | `links` de la fin | 157 830 | 9 283 | 10 594 ms |

Le repli est voulu (`record_nodes.rs`, ticket `2026-10-18-les-relations-d-un-index-dependent-du-chemin-d-ecriture`) :
une paire déjà en base doit recevoir les propriétés du lot, comme sous MERGE. La question est pourquoi la
résolution renvoie autant de paires **déjà posées** : les relations de l'analyse dans le fichier, posées plus tôt
dans le même index, pour les mêmes couples ? ou une même paire posée deux fois dans la résolution ?

## Ce qu'il faudrait

Lire d'où viennent les paires déjà là (un relevé : combien de CONSUMES de la résolution existent déjà, et posés
par quoi), puis soit ne pas les renvoyer, soit fondre leurs propriétés avant l'écriture pour rester sur le COPY.
Témoin : la même base, arête pour arête et propriété pour propriété (la sonde de marque), avant et après ;
la durée de `drain · liens` mesurée sur les deux arbres.

## Fermeture, 18 novembre 2026

**La cause** était dans le nœud des liens, pas dans la résolution : `copier_les_liens` (`dataflow/record_nodes.rs`)
renvoyait **tout le groupe** au chemin par lots dès qu'**une seule** de ses paires était déjà en base avec des
propriétés à écrire. Désormais il copie les paires neuves et rend à l'appelant les seules paires déjà là (toutes
leurs occurrences), que le chemin par lots met à jour à la valeur de leur dernière occurrence, comme avant. La
ligne `[link-profile]` et la métrique `to_update_by_batch` comptent ces paires.

Témoin : `dataflow::copie_liens_tests::a_link_group_with_one_pair_already_linked_still_copies_its_new_pairs`. Une paire
`CONSUMES` posée, puis un groupe de 300 qui la répète avec un autre usage. Il était rouge avant (aucun COPY, tout le
groupe par MERGE), il est vert après (COPY des 299 neuves, 300 arêtes, la paire répétée prend la valeur du groupe).
Les témoins de parité du chemin d'écriture (`chemin_d_ecriture_tests`) et du COPY vers un nœud déjà lié
(`copie_liens_tests`) restent verts.

**La mesure** : sonde `probe_index_timeline` de recherche, `RAG3WEAVER_INGEST_PROFILE=1`, arbre figé, lib 3d706f709,
`record_nodes.rs` de master contre la branche :

| arbre | `drain · liens` avant | après | repli « chemin par lots » avant → après | `usages` répond, avant → après |
|---|---|---|---|---|
| rag3weaver/src | 7 457 ms | 4 577 ms | 2 → 0 | 50,2 s → 45,8 s |
| dépôt entier | 26 529 ms | 17 832 ms | 5 → 0 | 116 s → 139,5 s |

Sur le dépôt, l'index complet n'a pas baissé (217,7 s puis 232,5 s). L'écart vient des « dérivations », 106 s puis
124 s : une étape que ce lot ne touche pas, et dont les liens n'ont pas de propriétés, donc restaient déjà sur le
COPY. Cette variation n'est pas attribuée : **à revérifier sur un poste calme** (c7, 18 novembre), avant de
la dire due à la charge du poste. Le gain mesurable est celui de `drain · liens`.

**La parité** : sonde de marque, seuil du COPY à 1 (tout groupe par COPY), avant contre après. Les TSV sont
identiques au couple et à la marque près sur rag3weaver/src (290 fichiers) et sur `src/` du moteur (1 550 fichiers).

