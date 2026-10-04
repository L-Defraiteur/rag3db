# lucivy : une fermeture qui rend la main quand tout est lâché

Demi-page pour Lucie : lucivy est ton dépôt, publié ; rien n'y change sans ton accord. Écrite le 4 octobre 2026 par la session de l'arbre principal, à la demande de l'orchestration.

## Le défaut qu'elle explique

La corruption d'e2e_code venait de deux instances de la même base ouvertes dans un même processus (diagnostic de la session cœur C++). La seconde s'ouvrait alors que la première n'était pas encore détruite, et leurs points de reprise s'entrelaçaient.

Ce qui retenait la première instance, c'est la chaîne :

```
acteur de shard (scheduler luciole) → Arc<LucivyHandle> → BlobDirectory → magasin de blobs → connexion → Database
```

Le moteur refuse désormais une seconde ouverture en écriture. Mais tant que la chaîne tient, une réouverture rapide reçoit ce refus.

## Ce que `ShardedHandle::close()` fait aujourd'hui

Code lu dans lucivy-core 4.3.0, `sharded_handle.rs` :

1. Il commite les shards, ferme chacun et prévient le routeur, puis seulement coupe les pools. Les trois premières étapes passent par `?` : au moindre échec, les pools ne sont pas coupés. Les acteurs gardent leur `Arc<LucivyHandle>` jusqu'à la fin du processus, et la base reste ouverte.
2. Quand tout réussit, `Pool::shutdown` attend la réponse du shard (`ack`), puis le shard rend `Stop`. Mais la boîte de l'acteur, qui tient l'`Arc`, n'est lâchée qu'ensuite, par le fil du scheduler. `close()` rend donc la main avant que le dernier `Arc` tombe, et la destruction finit sur un autre fil.
3. `Drop for ShardedHandle` ne coupe pas les pools.

## Ce qu'il devrait garantir

- **Couper les pools en toute circonstance.** Même si un commit ou une fermeture de shard échoue, les pools sont coupés et l'erreur est rendue ensuite. Pas de `?` avant la coupure.
- **Ne rendre la main qu'une fois chaque boîte d'acteur lâchée.** Après le `Stop`, attendre que le scheduler ait retiré la boîte. Par exemple : un `Weak<LucivyHandle>` pris avant la coupure, et une attente bornée jusqu'à ce qu'il ne se relève plus. Ou un accusé envoyé par le scheduler après le retrait de la boîte.
- **Couper aussi dans `Drop`**, si `close()` n'a pas été appelé : un handle lâché ne doit pas laisser d'acteur vivant.

## Le changement minimal

Dans `close()` :
- remplacer les `?` par une collecte des erreurs ;
- couper les pools toujours ;
- attendre, de façon bornée, que le `Weak` du handle soit mort ;
- rendre la première erreur.

Dans `Drop` : couper les pools si ce n'est pas déjà fait.

Ni le format d'index ni l'API ne changent.

## Ce que ça coûte aux autres usagers

- **Durée de `close()`** : elle s'allonge du temps de retrait des boîtes par le scheduler, de l'ordre de la milliseconde. L'attente est bornée.
- **Un `close()` sur un handle encore cloné ailleurs** : l'attente ne peut pas aboutir tant que le clone vit. Il faut alors une erreur nommée (« handle encore détenu »), pas une attente infinie. C'est un changement de comportement visible, mais il révèle un usage déjà fautif.
- **Usagers sans base derrière** (stockage fichiers) : aucun effet, sauf la fermeture plus nette.

## En attendant, côté rag3weaver

- Un registre de processus (chemin → `Weak<Database>`) dans `Rag3dbConnection` : une ouverture en écriture attend, de façon bornée, la mort de l'instance précédente. Sinon elle rend une erreur nommée qui dit ce qui la retient. Cela couvre la fin d'acteur tardive.
- `Catalog::drop` ferme vraiment les handles. Plus de simple commit quand un clone survit.

## Les deux questions de l'orchestration

**(a) Le mode fichiers (`FtsStorage::Files` / `LocalFs`) coupe-t-il la chaîne ?** Oui. Ses handles ne portent que des chemins (`fts_directory.rs`) : ni magasin de blobs, ni connexion. Un acteur lucivy qui survit à la fermeture n'y retient plus la Database. Il peut encore retenir des fichiers de l'index plein texte, ce qui est sans danger pour la base. C'est un argument de plus pour ce mode.

**(b) Le fil détaché de `spawn_index`**, qui garde le catalogue au-delà de `drop(backend)` : à l'arrêt du backend, il faut **lui demander de s'arrêter, puis attendre sa fin, de façon bornée**. L'indexation s'arrête à la fin de son paquet en cours : l'arrêt entre deux paquets est déjà sûr, puisque la reprise sait finir. Au-delà de la borne, le backend sort quand même en le disant. La reprise dans un processus neuf retrouve alors un état connu.
