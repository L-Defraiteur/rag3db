# 09 — Réponse de lucivy sur le pré-filtre sparse

Réponse au [08](08-questions-a-lucivy-prefiltre-sparse.md), 27 août au
matin. Vos questions ont trouvé un défaut que nos tests ne voyaient pas et
ont provoqué une accélération de ×27 sur le cas qui vous intéresse. Merci —
et voici tout, dans l'ordre.

## D'abord : votre question 2.3 a trouvé un bug

Vous demandiez à partir de quelle sélectivité le filtre devient gagnant.
En allant mesurer, on a découvert que **sur un index segmenté, l'heuristique
ne jouait plus du tout**. Notre `search_segments` (écrit la nuit dernière)
testait un prédicat par document au lieu de passer l'ensemble autorisé aux
résolveurs : le chemin `seek` — recherche binaire par lane — était devenu
inatteignable. Une régression introduite par la segmentation, invisible pour
nos tests parce qu'ils vérifiaient les *réponses*, pas le *chemin*.

Corrigé : l'ensemble descend maintenant dans chaque segment, et le choix
seek/fenêtre se refait segment par segment.

En le corrigeant, on a trouvé deux coûts payés **à chaque requête** pour un
ensemble qui ne change pas :

1. `allowed.to_vec()` + `sort_unstable()` + `dedup()` — une copie et un tri
   à chaque appel ;
2. sur le chemin fenêtre, la construction d'un `HashSet` de tout l'ensemble,
   également à chaque appel.

Les deux sont partis. Un ensemble **trié et sans doublon** est lu sur place
(vérification linéaire, aucune allocation), et l'appartenance devient une
recherche binaire au lieu d'une table qu'il faut bâtir d'abord.

## 1. Oui, et c'est maintenant écrit noir sur blanc

> la recherche sparse filtrée est exactement la recherche non filtrée
> intersectée avec l'ensemble autorisé, aux mêmes scores et dans le même
> ordre

Confirmé, et il y a désormais un test dédié :
`sparse_vector/tests/test_filter_truth.rs`. Il compare, sur de vrais
vecteurs BGE-M3 (votre dump), la recherche filtrée à la recherche complète
intersectée — pour sept formes d'ensemble : 3 ids, 1 %, 50 %, tout, non
trié avec doublons, des ids inexistants, moitié réels moitié inconnus.
Sur les deux chemins de code, puisque la sélectivité en choisit un.

**Une nuance à connaître avant de comparer des scores** : les documents et
leur ordre sont identiques, les scores le sont *à quelques ULP près*, pas
au bit. Les deux chemins additionnent les lanes d'un document dans un ordre
différent, et l'addition flottante n'est pas associative — on a mesuré
`0.043053508` contre `0.04305351` pour le même document. Comparez avec une
tolérance, jamais avec `==`, dès que vous croisez deux chemins.

Et votre raisonnement de fond est juste : pas de statistique de corpus, donc
pas d'équivalent de `with_subset_docs` à prévoir. Un filtre ne peut retirer
que des lignes. C'est exactement l'inverse du BM25, où `search_filtered`
**rescore** sur le sous-ensemble (`N` = sa taille) — deux réponses
différentes, pas un arrondi.

## 2.1 Oui, le filtre survit à la segmentation

`search_filtered` traverse tous les segments avec le même ensemble, et
`test_segments.rs::a_filtered_search_is_the_same_before_and_after_a_merge`
le verrouille : cinq segments, des suppressions au milieu, cinq ensembles
autorisés (de 3 ids à « tout, y compris les supprimés », plus un qui ne
contient que des ids inexistants), et la même réponse — mêmes documents,
mêmes scores, même ordre — **avant** et **après** `compact()`.

Donc oui : comptez sur le filtre pendant qu'un index vit, pas seulement
après un merge.

Détail d'implémentation qui vous concerne : un segment qui porte des
tombstones filtre d'abord l'ensemble (une passe linéaire), les autres
reçoivent vos ids tels quels. Le cas courant — pas de suppression — ne paie
rien.

## 2.2 Le repli du routeur : justesse, jamais un résultat manqué

Votre lecture est la bonne. Dans `ShardedSparseHandle::search_filtered` :
le routeur groupe les ids par shard ; si **un seul** est inconnu, on
abandonne le groupage et on envoie l'ensemble complet à tous les shards
(`sharded.rs:576-597`).

C'est une précaution de **justesse** : envoyer trop est un sur-ensemble du
routage correct, donc aucun résultat ne peut manquer. Ce qu'on perd, c'est
la vitesse — tous les shards travaillent au lieu de ceux qui portent
quelque chose.

Un id inconnu du routeur, ça arrive quand :

- il n'a jamais été inséré (ACL périmée, id venant d'un autre index,
  document supprimé puis routeur recompacté) ;
- l'index est antérieur à la persistance du routage.

**Donc : un ensemble autorisé partiellement périmé dégrade la vitesse, il ne
peut pas faire manquer un résultat.** Le pire cas est « on interroge tous
les shards », c'est-à-dire le comportement d'avant le routage.

## 2.3 La courbe — et elle ne ressemble plus à ce qu'elle était

`bench_filter_selectivity.rs`, 40 000 documents de vos vecteurs BGE-M3, 200
de vos requêtes, machine au repos, deux runs identiques au centième :

| autorisé | ids | temps | rapport à la recherche complète |
|---|---|---|---|
| 0,1 % | 40 | 0,009 ms | **×0,15** — le filtre *gagne* |
| 1 % | 400 | 0,073 ms | ×1,27 |
| 5 % | 2 000 | 0,073 ms | ×1,27 |
| 10 % | 4 000 | 0,070 ms | ×1,22 |
| 25 % | 10 000 | 0,065 ms | ×1,14 |
| 50 % | 20 000 | 0,066 ms | ×1,15 |
| 90 % | 36 000 | 0,068 ms | ×1,18 |
| 100 % | 40 000 | 0,069 ms | ×1,21 |

(sans filtre : 0,057 ms)

**La réponse à « à partir de quelle sélectivité le filtre devient-il
perdant » est : nulle part.** Il coûte au pire 30 % au-dessus d'une
recherche complète, quelle que soit la taille du domaine, et il *rapporte*
en dessous de ~1 % du corpus. Posez-le sans arrière-pensée.

Avant le correctif, la même courbe montait à ×7,7 à 100 % : c'est ce que
vous auriez mesuré si vous aviez posé la question sans qu'on aille voir.

## 2.4 Un très grand ensemble ne coûte plus rien — s'il est trié

| ensemble | avant | après |
|---|---|---|
| 40 000 ids (tout l'index) | 0,443 ms | **0,069 ms** |
| 140 000 ids (100 000 inexistants) | 1,495 ms | **0,100 ms** |
| 540 000 ids (500 000 inexistants) | 6,004 ms | **0,220 ms** |

Donc non : pas de taille au-delà de laquelle il vaut mieux ne pas filtrer.

**La condition, et c'est le contrat d'API à retenir** : donnez des ids
**triés et sans doublon**. Ils sont alors lus sur place, sans allocation. Un
ensemble non trié est copié, trié et dédupliqué à chaque requête — pour un
domaine de plusieurs centaines de milliers d'ids, c'est ce coût-là qui
domine tout le reste. Vos domaines sortent probablement d'une requête ou
d'une colonne triée, donc c'est gratuit chez vous ; il suffit de ne pas les
mélanger en route.

Sur votre suggestion d'une forme plus compacte (plage, bitmap, roaring) :
pas nécessaire pour l'instant. À 540 000 ids on est à 0,22 ms, dont
l'essentiel est la recherche elle-même. Si un jour vous avez des domaines de
plusieurs millions, un bitmap deviendrait intéressant — dites-le-nous à ce
moment-là, l'interface prend une tranche, elle prendrait un `impl DocFilter`
aussi bien.

## 3. Ce qu'on note de votre côté

- **Le débit** : 16 vecteurs/s côté modèle, donc l'index n'est pas le
  goulot et `insert_many` n'achète rien. C'est classé, on ne le fait pas.
- **Le nombre de segments par domaine** : toujours la question ouverte, sans
  urgence — notre seuil de compactage est à huit et il est justifié par le
  nombre de fichiers, pas par la vitesse de recherche (voir plus bas).
- **Le dump** : 2 924 suffisent, ne le refaites pas.

## 4. Sur votre remarque, et une seconde rétractation

Vous notez que notre ×3,1 / ×7,8 a été mesuré pendant que votre machine
saturait. C'est exact, et c'est la moitié de l'histoire — l'autre moitié est
pire : **avant celui-là, un premier chiffre (×5,3) venait d'un corpus
synthétique** à poids plats, où le WAND ne peut rien élaguer. Deux chiffres
faux, deux causes différentes, tous deux reproductibles au moment où ils ont
été pris.

Et il y a une troisième chose qu'on a dû retirer : entre les deux, on avait
*expliqué* l'écart par la taille du vocabulaire (« un modèle a des
dimensions partagées, les mots sont quasi uniques »). L'explication était
jolie et elle expliquait du bruit — au repos, les deux corpus donnent le
même résultat. Une explication convaincante d'un artefact est plus
dangereuse que l'artefact.

Ce qu'on en retient, et qui vaut pour vos rapports aussi : les **rapports**
survivent au bruit (vos 24×, notre ×27 ci-dessus), les **valeurs absolues**
non. Et une explication ne vaut que si elle survit à une nouvelle mesure.

---

Tout ce qui précède est sur `main` de lucivy, non publié, et partira en
3.0.6. Rien à faire de votre côté sinon, éventuellement, vérifier que vos
domaines arrivent triés.
