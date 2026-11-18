# La pose des liens de la résolution repasse par MERGE

- **État** : ouvert (18 novembre 2026) — cause mesurée, pas encore lue dans le code
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
