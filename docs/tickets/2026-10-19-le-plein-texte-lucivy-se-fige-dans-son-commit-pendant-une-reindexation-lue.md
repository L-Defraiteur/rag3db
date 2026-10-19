# Le plein texte lucivy se fige dans son commit pendant une réindexation lue

- **État** : ouvert — vu le 19 octobre 2026, trois passes sur trois (une avec L6, deux sur master 745989605 sans L6).
- **Gravité** : blocage — la montre ne réindexe plus rien, sans erreur ; les lectures continuent sur l'état d'avant (« Une écriture de l'index est en cours »).
- **Atteignable en service** : oui, a priori. C'est le régime ordinaire d'un hôte de code : un agent lit pendant que des fichiers changent sur le disque.
- **Touche rag3weaver** : oui (la montre, `reindex_on_change`) ; le correctif vit sans doute dans lucivy (`lucivy-core` 4.3.0, l'ordonnanceur « luciole »).

## Ce que c'est

Un dossier de code réel (`extension/rag3weaver/src`, 213 fichiers) indexé par le gabarit de code
livré, servi par un `DaemonHost`. Quatre lecteurs bouclent sur `search_code` et `usages`, et tous
les fichiers sont modifiés deux fois de suite.

- Le premier tour se réindexe en 2,8 s.
- Au second, la montre réindexe fichier par fichier (environ 1 s de verrou chacun), puis s'arrête
  après une soixantaine de fichiers. Le journal répète, toutes les 10 s jusqu'à la fin du délai
  du test (600 s) :

```
[luciole] WARNING: wait("flush_shard") blocked 530s (warn #53)
  ActorId(68) shard: BUSY processing (529.2s) q:0
WaitGraph (10 edges):
  unknown --[flush_shard]--> waiting
  shard_68 --[shard_0_commit]--> waiting
  scheduler-8 --[schedule_commit]--> waiting
  rag3weaver-graphes --[drain_shards]--> waiting
```

L'acteur du shard 68 est occupé sans fin. Il attend `shard_0_commit`, le flush attend le shard, et
le drain du graphe de réindexation attend `drain_shards`.

## La recette

Le témoin ci-dessous, sur luciepc :

- lib cf71c929d ;
- services d'embarquement 7878-7880 ;
- `RAG3WEAVER_MESURE_ARBRE` pointant sur un dossier de code.

Le rouge se reproduit trois fois sur trois. Il **n'est pas encore départagé** :

- les quatre lecteurs y sont-ils nécessaires ?
- le second tour suffit-il seul ?

Un rejeu sans lecteurs le dira. C'est la première chose à faire, avant de lire le code de
lucivy, et la pile d'abord : `eu-stack` avec `LD_PRELOAD` de `ptracer-any.so` sur le processus
figé.

## Le témoin

`e2e_lectures::mesure_des_instantanes_sur_un_vrai_depot` (arrive avec L6), qui échoue sur « le
tour 1 n'est pas réindexé ». Il se saute sans `RAG3WEAVER_MESURE_ARBRE` et le dit.

## Pour le fermer

Départager (lecteurs ou non), puis prendre la pile du shard figé. Le correctif va dans lucivy
(nouvelle version publiée, puis prise ici), avec un témoin déterministe côté lucivy. Le témoin
de mesure doit ensuite passer ses trois tours.
