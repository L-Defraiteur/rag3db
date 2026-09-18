# Un défaut qui ne coûtait qu'une économie

18 septembre 2026. Trouvé en cherchant autre chose, corrigé avec la garde du
[doc 01](01-la-machine-a-etats-a-l-ecriture.md) (`b31713596`).

**Aucun chiffre ici, et c'est délibéré** : au moment où j'écris, aucune entité
livrée n'était touchée. Un « avant / après » demanderait que je fabrique
l'entité de démonstration, et mesurerait alors ma mise en scène.

## 1. Le mécanisme

`split_unchanged` court-circuite les enregistrements déjà en base, identiques
et complets. Pour juger « complet », il relit la table de chunks et compte,
par parent, combien portent leur marqueur dense et leur marqueur sparse.

Il réclamait ces deux marqueurs **sans regarder les signaux déclarés**. Sur une
entité qui ne veut pas de vecteur, la colonne dense n'existe pas sur sa table
de chunks : le moteur refuse la requête —

```
Binder exception: Cannot find property _embed_hash__<modèle> for n.
```

— et la fonction sortait par l'échec, en rendant « rien n'a été court-circuité ».

## 2. Pourquoi il pouvait dormir indéfiniment

Parce que **sa conséquence était correcte**.

« Rien n'a été court-circuité » veut dire « tout est réécrit ». Le résultat est
juste ; seul le temps est perdu. Il n'y a ni erreur remontée, ni donnée fausse,
ni test rouge — le seul symptôme est une lenteur qu'on attribue au travail
lui-même, puisqu'on ne sait pas qu'une partie était évitable.

> **Un défaut dont le seul coût est une économie manquée ne se voit pas.** Il
> n'a pas de symptôme, il a une absence de gain — et on ne remarque pas
> l'absence d'un gain qu'on n'a jamais eu.

## 3. Ce qui l'a rendu visible : un changement de prix

Rien dans ce code n'a changé. Ce qui a changé, c'est **ce qui s'y appuyait**.

La garde de la machine à états avait besoin de l'**état d'avant**, et l'endroit
juste pour le lire était précisément cette fonction, qui relit déjà la ligne —
le lire ailleurs aurait coûté une seconde lecture, et cette seconde lecture
aurait menti sur les écritures en attente du même lot.

Du jour où l'état d'avant sort d'ici, la même sortie en échec ne coûte plus une
économie : **elle laisse passer une transition interdite**. Le défaut n'a pas
changé de nature, il a changé de prix.

C'est pour ça qu'il s'est fait prendre en une soirée après des semaines de
sommeil, et c'est une leçon transposable : *un silence tolérable devient
intolérable dès que quelqu'un décide à partir de lui*. Quand on branche une
décision sur une information existante, il faut relire ce que cette information
fait en cas d'échec — pas seulement ce qu'elle dit en cas de succès.

## 4. La population exacte

| condition | touchée ? |
|---|---|
| signaux **sans vecteur** **et** entité **découpée** | **oui** |
| signaux sans vecteur, `chunked: Some(false)` | non — le bloc entier est sauté |
| signaux avec vecteur | non — la colonne existe |

Conséquences, et elles vont à rebours de ce que j'avais annoncé d'abord :

- **`Symbol` n'était pas touché.** Il est en BM25 seul, mais il déclare
  `chunked: Some(false)` : la requête n'était jamais exécutée. Je l'avais cité
  comme exemple, à tort.
- **Aucune entité livrée n'était touchée.** `File`, `Scope` et `Library`
  partent du défaut `HYBRID`, donc elles ont leur colonne dense.

**Le défaut était donc devant nous, pas derrière** — et pour une raison précise :
`EntityConfig::declared()`, le point de départ d'un schéma produit par un
modèle, vaut **`BM25` seul exprès**, pour qu'une déclaration automatique
n'embarque pas de vecteurs que personne n'a demandés
([doc 07 §3.1](../27-aout-2026-13h01/07-le-langage-de-declaration.md)). Toute
entité née par cette porte, dès qu'elle est découpée, tombait dans le cas. Le
schéma d'un utilisateur, ou celui qu'un modèle écrit.

Deux décisions saines se rencontraient et produisaient un défaut : ne pas
embarquer par défaut, et supposer que la colonne dense existe toujours.

## 5. Ce qui a été corrigé

- **La requête ne demande que les marqueurs des signaux déclarés.** Les
  positions suivent la liste, qui n'est plus fixe.
- **Des chunks illisibles ne font plus tout abandonner.** Cette lecture ne sert
  qu'à une question — les artefacts sont-ils complets ? — et la réponse
  prudente en cas d'échec est « non », ce que donne un `complete` vide. Rendre
  « rien n'est relu » faisait la même chose côté court-circuit **et** jetait au
  passage la ligne parente déjà lue, donc l'état d'avant.
- **Quatre sorties muettes parlent**, dont une qui n'existait pas : *N lignes
  demandées dont M avec un uuid, 0 relue*. Celle-là ne lève rien et ne
  correspond à aucune erreur — c'est exactement le genre qu'on ne trouve jamais.

## 6. La mesure à faire, le jour venu

Le jour où une entité **sans vecteur et découpée** existe pour de vrai —
premier schéma déclaré par un modèle, ou par un utilisateur :

1. l'ingérer une fois, puis la **réingérer à contenu identique** ;
2. lire `FlushResult` : `unchanged` doit valoir le nombre de lignes, et
   `processed` ne pas croître en travail dérivé ;
3. comparer les durées des deux ingestions.

Avant la correction, la seconde ingestion coûtait autant que la première :
découpage et indexation refaits pour rien. Le gain attendu est celui du
court-circuit de l'inchangé mesuré le 27 août sur une autre entité —
**19,5 s → 2,1 s** — mais *ce chiffre-là n'est pas celui-ci*, et il ne doit pas
être cité comme tel. Il dit seulement l'ordre de grandeur de ce qu'un
court-circuit qui fonctionne fait gagner.
