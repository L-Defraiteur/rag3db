# BM25 au rang visible : la longueur moyenne lue sur les admis (conception, 16 novembre 2026)

Pour c7, ED et banc, avant tout code. Ticket d'ED : e47736a79. L'écrivain d'ED garde les versions fermées au plein
texte : 136 181 morceaux de `ScopeContent`, contre 114 854 dans un index neuf. Le banc voit 23 cas sur 30 changer de
rang. Son rappel à 10 vaut 260/289, soit 0,90 ; 18 cas sur 30 ont un sortant, 25 sur 30 gardent leur premier résultat.

## Ce qui se passe, et ce qui reste à lucivy

- **La plus grande part est chez ED, et il la corrige.** `Visibilite::tout_visible` était posé vrai à chaque
  ouverture. Une base avec de l'histoire, rouverte, cherchait donc **sans `allowed_ids`**, sur les 136 181 morceaux,
  et N, les fréquences et la longueur moyenne portaient toute l'histoire. Le juge de banc était dans ce cas.
- **Ce que lucivy fait déjà sous un `allowed_ids`** (`lucivy_core/src/bm25_global.rs`, depuis 4.3.7) :
  - N est la taille de l'ensemble permis ;
  - les fréquences de documents sont comptées dans la vue (préscan sur les lecteurs filtrés), pour `term`,
    `contains`, `phrase` et `parse` (témoin `test_filtered_scores_match_the_subset`) ;
  - le témoin compare au sous-ensemble indexé seul, mais ses documents font tous quatre tokens : il ne pouvait pas
    voir la longueur moyenne.
- **Ce qui fuit : la longueur moyenne du champ.** `AggregatedBm25StatsOwned::total_num_tokens` met le total du corpus
  à l'échelle de N_vue / N_corpus. La moyenne reste donc celle du corpus entier, histoire comprise.

## Pourquoi elle avait été gardée sur le corpus

Le choix date du 26 août 2026 (5ede7b9, « a filtered search is a pre-filter »). Le commentaire dit : « the average
field length stays the corpus's — a property of the text, not of the filter ». L'idée : un filtre (un dossier, un
langage) est une loupe sur le même corpus, et la longueur typique d'un document ne dépend pas de la loupe. Ajoutons
qu'aucune somme des longueurs de la vue n'était à portée.

Avec les générations, le raisonnement ne tient plus : une version fermée **ne fait pas partie du corpus visible**,
un index neuf ne l'aurait pas. Et même pour un filtre ordinaire, mêler N et fréquences de la vue avec une longueur
du corpus n'est pas cohérent avec le principe de 4.3.7 : une recherche filtrée classe comme le sous-ensemble
indexé seul.

## Le dessin proposé

1. **La somme des longueurs des admis, au même passage que le masque.** `build_node_filter_bitset` lit déjà la
   colonne `_node_id` de chaque document du segment. Il lit en plus, pour chaque document admis et chaque champ
   texte, sa longueur codée (`FieldNormReader::fieldnorm`) et la somme.
2. **La correction du codage.** La longueur d'un document n'est gardée exacte que jusqu'à 40 tokens, puis par
   paliers. Le segment, lui, garde le total exact de chaque champ. La somme des admis est donc multipliée, segment par
   segment, par `total exact du segment / somme des longueurs codées de tous ses documents`. Cette somme se calcule
   une fois par lecteur de segment et se garde. Quand la vue est le segment entier, le total redevient exact.
3. **Les statistiques de la vue.** `with_subset_docs(N)` devient `with_subset(N, tokens par champ)` : la somme des
   shards remplace la mise à l'échelle. Une recherche sans filtre ne change pas.
4. **Pour toutes les recherches filtrées**, pas seulement la visibilité : c'est le principe de 4.3.7 poussé au bout.
   Les scores des formes filtrées du banc (`scope`, `language`) bougeront un peu ; c'est mesuré, pas supposé.

Écartés :
- **une table de comptes par génération** côté ED : lucivy tient la somme au même passage que le masque ;
- **un filtre dans le scorer** : il ne change pas la moyenne, qui est un scalaire de la requête.

La vue nommée (`define_view`, convenue avec ED le 14 novembre) pourra garder ces sommes avec ses bitsets. Ce n'est
pas nécessaire pour ce lot.

## Le prix

- **À la requête** : une lecture de longueur par document admis et par champ texte, dans la boucle qui bâtit déjà
  le masque. Le masque coûte aujourd'hui 1,2 à 1,6 ms par passe sur 72 652 ids et 14 segments (mesure du 14
  novembre) ; la somme devrait en ajouter autant au plus. C'est à mesurer par le banc du coût du filtre
  (`test_filter_cost`) avant et après.
- **Une fois par lecteur de segment** : la somme des longueurs codées de tous ses documents, qui sert à la
  correction.
- **Le format ne change pas** : version 4.3.9. Lucie autorise sa publication après le correctif (16 novembre).

## Le témoin rouge

- **Le corpus** : V, des documents visibles courts ; H, des versions fermées plus longues des mêmes contenus, qui
  emportent la moyenne. Les longueurs sont au-delà de 40 tokens, pour que la correction du codage joue.
- **Deux index** : A, l'index de V seul ; B, l'index de V et H.
- **La règle** : sur une série de requêtes (`term`, `contains`, `parse`, `phrase`), `search_filtered(B, admis = V)`
  rend **le même rang** que `search(A)`, et les mêmes scores à une tolérance relative mesurée.
- **Rouge aujourd'hui** : la moyenne vient de V ∪ H.
- **Ensuite l'oracle du banc** : ses 30 cas (`jeu-recherche-2a2.json`, `parite_search_code.py --regime reponses`)
  sur une copie reflink de `ecr-base-ecrivain-neuf-5-lot-reindex-20261115`, après le correctif d'ED, contre
  `parent-reindex`. Attendu : rappel à 10 de 1,0, même ordre. L'écart de score toléré sera celui que mesure le
  témoin.
