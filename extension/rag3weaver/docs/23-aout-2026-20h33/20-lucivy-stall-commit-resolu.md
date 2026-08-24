# Le stall de 25-40 s après réouverture : ce n'était pas un `Reply` perdu, c'étaient 480 aller-retours

Réponse au doc 19. Le rapport était excellent — le dump luciole, la lecture de
`collect_replies_to`, l'ordre-dépendance — et il pointait un vrai trou dans
luciole. Mais le trou n'était pas votre stall : un `Reply` lâché sous un pipe
bloque **pour toujours** (il n'y a aucun timeout de repli dans `wait`), or vos
commits finissaient par aboutir. Et votre dump le disait : *un* thread en
`TASK`. Il ne dormait pas, il travaillait — dans votre store.

## Ce qui se passait

J'ai instrumenté un `BlobStore` qui compte ses appels
(`lucivy_core/tests/test_commit_floor.rs`, `commit_floor_store_calls`). Un
commit de **9 documents sur 2 shards**, montage identique au vôtre :

| appel au store | avant | après |
|---|---|---|
| `save` fichiers de segment | 340 | **81** |
| `save .managed.json` | 135 | **2** |
| `save meta.json` | 2 | 2 |
| `delete` (à la fermeture) | 232 | **6** |
| **total par commit sale** | **~480** | **~91** |

Avec `MemBlobStore` c'est invisible (9,6 ms). Avec une base derrière
`CypherBlobStore`, à 50-80 ms l'aller-retour, 480 appels = 25-40 s. C'est
votre chiffre. Le thread en `TASK` était la finalisation du segment, bloquée
dans `store.save` en FFI, un fichier après l'autre.

Trois causes empilées, trois correctifs (tous poussés, `e6176f5` en tête) :

1. **`.managed.json` réécrit dans le store à chaque fichier enregistré**
   (`e8ace07`). Le registre de fichiers de l'index est réécrit par
   `atomic_write` une fois par fichier créé ; `BlobDirectory` poussait chaque
   version dans le store. Il est maintenant écrit en local et sauvé dans le
   store **au point de commit** (juste avant `meta.json`, pour que le store
   n'ait jamais un `meta.json` en avance sur son registre), à
   `sync_directory` et à la fermeture. 135 → 2.

2. **9 documents = 8 segments** (`8e2db07`). Le writer distribuait les
   documents un par un, en round-robin, sur ses 8 indexeurs — chacun produit
   son propre segment (~35 fichiers, un `save` chacun), puis la policy les
   fusionnait et supprimait les miettes (les 232 `delete` de la fermeture).
   Les documents vont maintenant par tranches de 64 à un même indexeur avant
   de passer au suivant : un petit lot fait **un** segment par shard, un
   chargement massif utilise toujours tous les indexeurs. 340 → 81.

3. **La corrélation avec `9a66fbf`** : pas causale. Le retrait des fsync a
   rendu tout le reste plus rapide, sauf ce qui était borné par votre base —
   d'où l'impression d'un chemin inverse. Le suite complète avant faisait
   déjà 33 s ; votre soupçon « ces trois tests perdaient peut-être déjà 8 s
   chacun » est le bon.

Chronos après correctifs (`test_commit_floor`, MemBlobStore) : 2 shards 9 docs
**5,6 ms**, 4 shards 900 docs **18,7 ms**, réouverture puis commit **7-10 ms**,
fermeture < 1 ms.

## Ce que la chasse a révélé en plus — un vrai bug moteur

Le routage par tranches met les 8 documents d'un test dans un seul segment
là où ils en occupaient 8. Un test écrit ce matin
(`v3_strict_sep_head_three_chunks`) est tombé au premier essai :
`<binder::Expression` en strict ne trouvait plus rien. Cause : quand une clé
du FST **avale** tout le reste de la requête (`Expressi`+`on` dans un
document), la construction des chaînes s'arrêtait là et n'explorait jamais la
forme découpée du même texte dans un autre document (`Expres|si` puis
`sion>`). Le même mot se chunke différemment selon le document ; les deux
branches sont réelles. Corrigé (`4d00531`) : l'avalement est une branche
parmi les autres. Panel kernel 50k rejoué : 12 requêtes, spans exacts,
aucun changement de temps (`include` 45,7 ms, plancher 24-27 ms).

C'est le second bug de cette famille (« la branche la plus longue exclut les
autres ») après celui du 23 août — et il aurait touché votre production sur
des segments réels bien remplis. Votre doc 19 a permis de le trouver.

## Votre demande luciole

Faite en partie (`e6176f5`), honnêtement : on ne peut pas « honorer le pipe »
sans valeur à livrer, et changer `collect_replies_to` pour transporter des
`Option<T>` toucherait tous les appelants. Ce qui est fait : `Reply::drop`
avec un pipe attaché, et `set_pipe` sur un `Reply` déjà fermé, **avertissent
sur stderr** au lieu de se taire. Un collect bloqué avec tous les threads
idle porte maintenant un message qui dit pourquoi. La vraie garantie reste
la règle de conception : un acteur répond toujours, ou meurt bruyamment.

## À faire chez vous

- Épingler `e6176f5`, rejouer `e2e_idempotent_registration`. Attendu :
  `kb_and_relation_persist_and_reopen` autour de la seconde, la suite
  autour de vos 35 s de référence ou moins.
- Si un commit reste au-dessus de ~100 ms × (nombre d'appels), mesurez le
  coût unitaire de `save` dans `CypherBlobStore` : à ~91 appels par commit
  de petit lot, c'est lui qui fixe le plancher maintenant. Le palier
  suivant chez nous serait de regrouper les ~35 fichiers d'un segment en un
  seul blob — dites-nous si vos chiffres le justifient.

## Point ouvert chez nous, pour information

`test_luce_playground_search` échoue (pré-existant, indépendant de tout ceci,
découvert parce que `cargo test` s'arrêtait avant lui) : un highlight fuzzy
qui finit au milieu d'un `→`. À examiner ; sans rapport avec vos chemins.
