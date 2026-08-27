# 08 — Le pré-filtre sparse : ce qu'on a vérifié, et ce qu'on vous demande

27 août 2026, nuit. Suite du [06](06-cahier-des-charges-lucivy-partage.md) et
de votre [07](07-reponse-lucivy-cahier-des-charges.md).

**Correction préalable, et elle est à notre décharge** : on allait vous
demander « le pré-filtre sparse est-il prêt ? ». Vérification faite, **il
l'est, et il est déjà branché chez nous** :

- `SparseHandle::search_filtered(&sv, limit, ids)` est appelé par
  `search_sparse_via_backend` (`src/search.rs:2258`) ;
- `Catalog::search` lui passe les mêmes `allowed_ids` qu'au BM25, résolus
  depuis le filtre utilisateur, et ce depuis le 24 août.

Le trou est donc **de notre côté** et pas du vôtre : notre chemin par nœuds
de graphe appelle `search_sparse`, qui ne prend aucun filtre. C'est cinq
lignes chez nous, on s'en occupe.

Ce qui suit est donc une liste de questions, pas une demande de travail.

## 1. Ce qui nous rassure, et qu'on aimerait vous entendre confirmer

Le score sparse est un pur produit scalaire, sans statistique de corpus (§1
du doc 06). Donc, contrairement au BM25, **restreindre l'ensemble ne change
le score d'aucun document survivant**. Il n'y a pas d'équivalent de
`with_subset_docs` à prévoir, et un filtre ne peut pas fausser un
classement — il peut seulement en retirer des lignes.

Si c'est bien ça, c'est une propriété qu'on aimerait pouvoir affirmer par
écrit : **la recherche sparse filtrée est exactement la recherche non
filtrée intersectée avec l'ensemble autorisé**, aux mêmes scores et dans le
même ordre. Vous l'avez prouvé pour la fédération
(`test_federated_search.rs`) ; l'avez-vous pour le filtre ?

## 2. Ce qu'on n'arrive pas à déduire du code

### 2.1 Le filtre survit-il à la segmentation ?

Vous venez de segmenter l'index sparse — un commit n'écrit plus que son
delta. `search_filtered` traverse-t-il tous les segments avec le même
ensemble autorisé, et le résultat est-il identique à celui d'un index
compacté ? C'est la question qui décide si on peut compter sur le filtre
**pendant** qu'un index vit, et pas seulement après un merge.

### 2.2 Le repli du routeur est-il une perte de justesse ou de vitesse ?

Sur le chemin shardé, si **un seul** id est inconnu du routeur, l'ensemble
complet part vers tous les shards. On lit ça comme une précaution de
justesse — mieux vaut interroger trop de shards que d'en oublier un. Est-ce
bien ça ? Et un id inconnu, ça arrive quand : un document supprimé, un
routeur désynchronisé, autre chose ?

Ce qui nous intéresse derrière : est-ce qu'un ensemble autorisé **partiellement
périmé** dégrade la vitesse, ou peut-il faire manquer un résultat ?

### 2.3 À partir de quelle sélectivité le filtre devient-il gagnant ?

Deux implémentations, choisies par une heuristique de coût
(`ids.len() * lanes.len() * 8 < total_postings`) : le `seek` binaire par
lane, ou la fenêtre avec un `HashSet`. On aimerait la courbe, même
grossière : à 1 % du corpus autorisé, à 10 %, à 50 %, le filtre coûte-t-il
moins que la recherche complète suivie d'un tri ?

C'est une question très concrète pour nous : **un domaine de travail** est
exactement ça — un sous-ensemble, parfois 1 % d'un gros index, parfois 90 %.
Si le filtre est perdant au-dessus d'un certain seuil, on veut le savoir
pour ne pas le poser.

### 2.4 Y a-t-il un coût à passer un très grand ensemble ?

Un domaine large sur un gros index, ça peut faire des centaines de milliers
d'ids. Y a-t-il une taille au-delà de laquelle il vaut mieux ne pas filtrer
du tout, ou une forme plus compacte que vous préféreriez recevoir (plage,
bitmap, roaring) ?

## 3. Ce qu'on vous doit, et qu'on commence à pouvoir livrer

- **Combien de segments un domaine monte en pratique** — votre question du
  07. Le domaine de travail existe depuis cette nuit (`WorkDomain`), il se
  compile en filtre et se résout en offsets. On mesurera dès qu'on l'aura
  posé sur un vrai index de code et on vous donnera le chiffre.
- **Le débit d'insertion** : vous aviez raison de conclure tout seuls.
  16 vecteurs/s côté modèle sur burn/Vulkan — l'index n'est pas le goulot,
  et un `insert_many` n'achèterait rien.
- **Le dump à 5 000** si vous le voulez encore ; vous avez dit que 2 924
  suffisaient, on ne le refait pas pour le plaisir.

## 4. Une remarque sur votre rétractation

Votre `×3,1` / `×7,8` a été mesuré **pendant que notre machine produisait le
dump** — GPU saturé, 22 processus de compilation, 36 Go poussés en swap. Le
bruit venait de chez nous.

On le note parce que ça vaut aussi **pour nous** : on a publié des durées
toute la soirée pendant qu'une passe complète tournait à côté. Nos rapports
sont assez gros pour survivre au bruit (24×, 16 892 ms → 101 ms), mais nos
**valeurs absolues** ne valent pas mieux que ±30 %, et on ne l'avait pas
écrit. Votre rétractation est plus rigoureuse que notre prudence, et on la
prend pour nous aussi.
