# L'index vectoriel (HNSW) sous plusieurs écrivains

Session « cœur C++ », 3 octobre 2026. Suite de
`01-note-de-conception-les-verrous.md`, qui laissait l'index vectoriel de côté.

Étiquettes : **[exécuté]** prouvé par un test que j'ai fait tourner ; **[lu]** lu dans
le code, à la ligne citée ; **[déduit]** raisonnement, à valider.

## En cinq lignes

1. L'insertion dans l'index se fait **au commit**, et les commits sont en file : c'est
   sain. La suppression et la mise à jour, que notre fork a ajoutées, travaillent
   **pendant** la transaction : c'est de là que viennent tous les défauts.
2. Quatre défauts trouvés et corrigés aujourd'hui, **tous visibles avec un seul
   écrivain ou sans aucune course** : trois sur master (`b15f92a7a`), un en attente de
   relecture (`hnsw-point-d-entree-apres-rollback`).
3. Un cinquième, avec deux écrivains, faisait planter le processus au second commit ;
   corrigé dans la marche A2 (`f8d73398a`, en attente de relecture).
4. Ce qui reste est une question de conception, pas un défaut isolé : **les points
   d'entrée de l'index vivent hors transaction**, et une suppression réécrit les arêtes
   de nœuds que la transaction n'a pas verrouillés.
5. Proposition : faire toute la maintenance de l'index au commit, comme l'insertion.
   Environ trois jours. Rien n'est codé là-dessus.

---

## 1. Comment l'index est tenu à jour

| Opération | Quand | Ce qu'elle touche |
|---|---|---|
| insertion d'un nœud | **au commit** — `needCommitInsert()` vrai, `NodeTable::commit` puis `OnDiskHNSWIndex::commitInsert` | arêtes des deux couches ; points d'entrée si la couche est vide |
| `COPY` sur une table déjà indexée | à la fin du `COPY`, `OnDiskHNSWIndex::finalize` : tout ce qui dépasse `numCheckpointedNodes` | idem |
| suppression | **pendant l'instruction** — `delete_`, puis `finalizeDelete` en fin d'instruction | arêtes du nœud, arêtes de ses voisins, points d'entrée |
| mise à jour d'un vecteur | **pendant l'instruction** — retrait puis réinsertion | idem |

**[lu]** Les arêtes sont des relations dans deux tables internes (`upperRelTable`,
`lowerRelTable`), rangées dans un seul sens, écrites sans journal : un rejeu les
reconstruit par l'index. Elles sont **transactionnelles** — un `ROLLBACK` les rend.

**[lu]** Les points d'entrée et le compteur `numCheckpointedNodes` sont de simples
champs de `HNSWStorageInfo` (`hnsw_index.h:60-82`), sérialisés au point de reprise. Ils
ne sont **ni transactionnels ni protégés par un verrou**.

**[lu]** Les commits sont en file : `TransactionManager::commit` tient
`mtxForSerializingPublicFunctionCalls`. Deux insertions dans l'index ne se croisent
donc jamais.

---

## 2. Ce qui a été trouvé et corrigé

Tous ces défauts viennent de la suppression dans l'index, ajoutée par notre fork
(`98e35566a`) ; l'amont ne supprime rien dans le HNSW.

| # | Défaut | Preuve | État |
|---|---|---|---|
| 1 | Le nettoyage de fin d'instruction n'était **jamais exécuté** : `finalize()` est appelé sur l'opérateur d'origine, alors que la requête tourne sur sa copie, seule à porter l'état de suppression. | **[exécuté]** zéro appel sous trace | sur master, `b15f92a7a` |
| 2 | Le point d'entrée supprimé était remplacé par un voisin **déjà mort**. Après une suppression en lot, toute recherche rendait vide, et les insertions suivantes n'étaient pas retrouvées. | **[exécuté]** table vidée puis `COPY` : zéro résultat | sur master, `b15f92a7a` |
| 3 | Les arêtes vers les supprimés étaient retirées sans rien mettre à la place : le graphe se coupait. | **[exécuté]** 899 suppressions sur 1000 : 62 survivants atteignables sur 101, trois plus proches faux | sur master, `b15f92a7a` |
| 4 | Un `DELETE` annulé par `ROLLBACK` rendait les lignes mais pas le point d'entrée : l'index se croyait vide. | **[exécuté]** mille lignes, zéro résultat | `hnsw-point-d-entree-apres-rollback`, `d64c74b18`, à relire |
| 5 | Deux transactions ouvertes ensemble insèrent des vecteurs : **plantage au second `COMMIT`**, dans un seul fil. L'index lisait les nouveaux nœuds par leurs offsets définitifs, que le moteur prenait pour des lignes locales. | **[exécuté]** SIGSEGV, pile lue | `a2-remappage-des-offsets-au-commit`, `f8d73398a`, à relire |

Le défaut 5 n'est pas dans l'extension : c'est le cœur, et il toucherait tout index qui
insère au commit.

---

## 3. Ce qui se passe aujourd'hui avec deux écrivains

Trois scénarios à deux connexions, dans un seul fil, sur un index de mille vecteurs,
avec la marche A2 complète :

| Scénario | Résultat |
|---|---|
| les deux insèrent cinq vecteurs, puis valident | **[exécuté]** vert : les dix retrouvés |
| l'un supprime un nœud, l'autre insère, l'inséreur valide d'abord | **[exécuté]** vert |
| les deux suppriment chacun un nœud, voisins dans le graphe | **[exécuté]** vert sur cet essai |

Ces verts ne prouvent qu'une chose : il n'y a plus de défaut **déterministe** sur ces
trois chemins. Ils ne disent rien des vraies courses, que seul un banc sous fils et
sous ThreadSanitizer montrerait. Ce qui reste, par lecture :

- **Les points d'entrée se modifient sans verrou, pendant les transactions.** **[lu]**
  `deleteFromGraph` les écrit (`hnsw_index.cpp`, remplacement du point d'entrée) pendant
  qu'une recherche d'une autre connexion les lit. C'est une course de données au sens
  strict. **[déduit]** Sa conséquence la plus probable est une recherche qui part d'un
  nœud en cours de suppression — tolérée par le code — plutôt qu'un plantage.
- **Une suppression réécrit les arêtes de nœuds qu'elle n'a pas verrouillés.** **[lu]**
  Supprimer A retire puis réécrit les arêtes de chaque voisin de A
  (`cleanEdgesForNode`). Les verrous de ligne de la note précédente protègent A, pas
  ses voisins. **[déduit]** Deux écrivains qui touchent deux nœuds voisins dans le
  graphe — sans aucun rapport pour l'utilisateur — se rencontrent donc sur les mêmes
  arêtes ; l'un des deux reçoit un « Write-write conflict » de suppression de relation,
  au milieu de son instruction ou, pire, **au milieu de son commit** quand c'est une
  insertion qui rétrécit le voisinage d'un nœud (`shrinkForNode`). Non reproduit : mon
  essai n'a pas tiré deux nœuds dont les voisinages se recouvrent ainsi.
- **Un commit qui échoue au milieu de l'insertion dans l'index.** **[déduit]** Les
  nœuds sont déjà dans la table quand l'index lève son erreur ; ce que devient la
  transaction à cet instant n'a pas été suivi.
- **La seconde transaction ne voit pas les arêtes de la première.** **[déduit]** Son
  instantané précède leur commit : ses nouveaux nœuds se relient à l'ancien graphe, pas
  aux nœuds que l'autre vient d'ajouter. C'est une perte de qualité de voisinage, pas
  une erreur : les deux groupes restent atteignables par l'ancien graphe.

---

## 4. Proposition : toute la maintenance de l'index au commit

Aujourd'hui l'insertion est au commit et le reste est pendant la transaction. La
proposition est de tout mettre au commit :

- pendant la transaction, une suppression ou une mise à jour de vecteur **ne touche pas
  le graphe** : elle note seulement le nœud ;
- au commit, sous le verrou de commit, l'index retire les nœuds supprimés, recoud leurs
  voisins, réinsère les vecteurs modifiés, puis insère les nouveaux nœuds — dans cet
  ordre.

Ce que cela règle, d'un coup :

| Problème | Pourquoi il disparaît |
|---|---|
| le `ROLLBACK` qui laisse l'index abîmé (défaut 4) | une transaction annulée n'a jamais touché l'index |
| la course sur les points d'entrée | ils ne changent plus que sous le verrou de commit |
| les conflits entre voisins du graphe | plus aucune arête n'est réécrite par deux transactions à la fois |
| l'échec au milieu d'une instruction | il n'y a plus de travail d'index dans l'instruction |

Ce que cela coûte :

- **Une recherche dans la même transaction voit encore le nœud supprimé dans le
  graphe.** Elle ne le rend pas — une ligne supprimée n'a plus de vecteur lisible, la
  recherche la saute déjà — mais elle traverse ses arêtes jusqu'au commit. C'est ce que
  fait déjà l'insertion, dans l'autre sens : un nœud créé n'est trouvé par l'index
  qu'après le commit, et par balayage avant (`searchFromUnCheckpointed`).
- Le commit d'une grosse suppression porte tout le travail de recouture. Il le portait
  déjà en fin d'instruction ; il se déplace, il ne grossit pas.
- Environ **trois jours**, tests compris **[déduit]** : la liste des nœuds à retirer
  par transaction, l'ordre au commit, la reprise des six cas de `delete.test`, et un
  cas par ligne du tableau ci-dessus.

Ce que cela ne règle pas : la seconde transaction qui ne voit pas les arêtes de la
première. Il faudrait que l'insertion au commit lise le graphe dans son dernier état
validé plutôt que dans l'instantané de la transaction — à regarder dans le même
chantier.

**Place dans l'ordre des marches** : après le gestionnaire de verrous et avant
d'allumer le mode multi-écrivains par défaut (A6). Tant que cette marche n'est pas
faite, une table qui porte un index vectoriel ne devrait pas recevoir deux écrivains à
la fois ; avec un seul écrivain, l'index est juste, branches en attente comprises.

---

## 5. Ce que le banc de concurrence doit ajouter

Le banc n'a aucun cas sur une table indexée. Quatre cas, sur le modèle des siens :

1. deux écrivains insèrent des vecteurs dans des transactions ouvertes ensemble ; tous
   retrouvés (c'est le défaut 5 ; déterministe) ;
2. un écrivain supprime pendant qu'un autre insère, dans les deux ordres de commit ;
3. deux écrivains suppriment des nœuds dont les voisinages se recouvrent — à
   construire en lisant le graphe, pour être sûr du recouvrement ; c'est le cas que je
   n'ai pas su tirer ;
4. le mélange aléatoire de C7 sur une table indexée, sous ThreadSanitizer.

Et un invariant de plus pour le vérificateur, propre à l'index : **tout nœud visible
qui porte un vecteur est rendu par une recherche exhaustive** (`k` égal au nombre de
lignes). C'est lui qui a montré les défauts 2, 3 et 4.

Attention en l'écrivant : `QUERY_VECTOR_INDEX … RETURN count(*)` rend toujours `k`,
même quand il y a moins de `k` résultats **[exécuté]** ; il faut compter
`count(node.id)`.
