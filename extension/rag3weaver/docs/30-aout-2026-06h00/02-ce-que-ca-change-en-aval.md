# Ce que ça change en aval

Suite du [cahier des charges](../../codeparsers/docs/30-aout-2026-06h00/01-cahier-des-charges-tout-le-fichier-est-couvert.md),
passé dans le dépôt `codeparsers` avec le code qu'il spécifie.
Celui-ci décrit ce qui bouge **hors de `codeparsers`** — dans l'ingestion, dans
le schéma, dans la recherche et dans le rendu. À lire avant d'implémenter :
plusieurs de ces points changent la forme de ce que `codeparsers` doit rendre.

## 1. Le schéma du code : un genre de plus, pas une entité de plus

L'entité `Scope` existe (`src/code.rs`), avec un champ `scope_type` déjà
présent et déjà indexé. **Les deux nouveaux genres passent par là.** Il ne faut
pas créer une entité `TextBlock` à côté :

- une seconde entité voudrait dire une seconde recherche, un second rendu, un
  second jeu de relations — et l'agent devrait savoir laquelle interroger ;
- alors que la question qu'il pose est toujours la même : *« où est-il question
  de ça ? »*.

`scope_type` prend donc deux valeurs de plus, et tout le reste — `DEFINED_IN`,
la résolution vers le fichier parent, `search(target="Scope")` — continue de
marcher sans une ligne de changement.

## 2. Le découpage : ce qui est compris et ce qui ne l'est pas ne se coupent pas pareil

Une fonction est une unité de sens : on l'embarque entière, et c'est ce que
fait le découpage actuel des scopes (`default_scope_chunking`).

Un passage non compris n'a pas d'unité. Il doit se découper comme un document
ordinaire — le `Chunker` existe et sert déjà aux bases de connaissances.

**Conséquence à ne pas rater** : un scope non compris de 20 000 lignes ne doit
pas produire **un** chunk de 20 000 lignes. Sinon il pèse dans la recherche
vectorielle comme un document entier, et il sortira devant les vraies fonctions
sur presque toutes les requêtes.

## 3. La recherche : filtrable, et honnête dans le rendu

Deux besoins distincts, et il ne faut pas les confondre :

**Filtrer.** Un agent qui cherche une fonction doit pouvoir dire « pas de texte
non compris ». Le mécanisme existe (`FilterParser`, les filtres sur les champs
indexés) : `scope_type` est déjà un champ, donc c'est de la configuration, pas
du code.

**Le dire dans la fiche.** Un résultat `TexteNonCompris` ne doit pas se
présenter comme une fonction. Le gabarit de rendu affiche `kind` quand il
existe (`### 3. nom (function)`) — il suffit que le genre soit juste. Mais il
faut aller plus loin : un passage non compris devrait porter une mention
explicite, du genre *« passage non analysé — le parseur n'a pas compris cette
région »*, parce qu'un agent qui lit `(texte_non_compris)` sans explication
supposera que c'est nous qui l'avons voulu.

## 4. La pondération : à mesurer, pas à supposer

Un passage non compris est du **texte libre**, donc riche en mots, donc
avantagé par BM25 comme par le vecteur : il ressemble à de la prose, et nos
requêtes sont souvent en prose.

C'est le risque le plus concret de tout le chantier. Deux façons de le tenir,
et il faut mesurer avant de choisir :

1. **Un facteur de pondération par genre** à la fusion. Le mécanisme des poids
   existe (`FuseResultsNode(weights=…)`), mais il pèse par **signal**, pas par
   genre — ce serait donc un ajout.
2. **Rien du tout**, et on regarde. Peut-être que le rerank suffit : un
   cross-encoder qui compare la requête au passage devrait naturellement
   préférer la fonction qui répond.

**La mesure d'abord** : reprendre le banc des trois questions françaises
(`e2e_catalogue_gabarits`), ajouter un fichier volontairement mal parsé au
corpus, et regarder si les vraies réponses reculent. Sans ce banc, on réglera
un poids à l'aveugle.

## 5. L'ingestion : ce qui devient possible

Aujourd'hui `analyze_source` rend des fichiers et des scopes, et l'ingestion
enregistre les seconds. Avec la couverture :

- **Un fichier sans parseur entre dans l'index** au lieu d'être ignoré. C'est
  un changement de volume : compter avant, sur ce dépôt, combien de fichiers
  sont aujourd'hui hors index.
- **Le `stale` par fichier continue de marcher** tel quel : il compare des
  empreintes de contenu, pas des scopes.
- **Le rapport d'ingestion doit dire la couverture**, sinon on aura les
  données sans jamais les regarder. Une ligne par langage, comme au §6 du
  cahier des charges.

## 6. Ce qu'il ne faut **pas** faire

**Ne pas cacher les passages non compris par défaut.** La tentation sera forte
— « ça pollue les résultats » — et ce serait revenir exactement à l'état
d'aujourd'hui, en pire : le texte serait indexé, coûterait de la place, et
resterait invisible.

Si le bruit est réel, la réponse est la pondération ou le filtre **explicite**,
pas un masquage silencieux. La règle du dépôt vaut ici comme ailleurs :
distinguer « ça n'existe pas » de « je ne te le montre pas ».

**Ne pas faire dépendre l'ingestion du taux de couverture.** « En dessous de
40 %, indexer comme texte » serait une seconde politique à régler, et elle
serait fausse la moitié du temps. Le fichier porte déjà les deux : du code là
où on a compris, du texte ailleurs. Il n'y a pas de choix à faire.
