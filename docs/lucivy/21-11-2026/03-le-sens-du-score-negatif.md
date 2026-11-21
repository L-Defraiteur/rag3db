# Le sens du score négatif de lucivy

Session lucivy, 21 novembre 2026, à la demande de Lucie : « le score négatif, il faut l'interpréter correctement :
vérifier exactement le sens du score négatif, et si on peut le transformer ». Lu dans lucivy 4.4.1 (main 31d6b3d).

## 1. D'où il vient

Un `contains` flou (`distance > 0`) score un document ainsi (`src/query/phrase_query/suffix_contains_query.rs:733-740`,
même formule dans `sfx_scoring.rs:252`) :

```
score = −1000 × manqués + BM25
```

- **BM25** : le score ordinaire du terme dans ce champ (`bm25_weight.score(fieldnorm, tf)`), positif, de l'ordre de 1
  à 15.
- **manqués** : les trigrammes de la requête collée (« rag3db is cool » devient `rag3dbiscool`) absents de la meilleure
  correspondance du document, au minimum sur ses correspondances (`fuzzy_contains.rs:637-655`). Une correspondance
  n'existe que si elle trouve au moins `seuil = trigrammes − 3 × distance − coupés par les frontières` trigrammes, avec
  un plancher de 1 ou 2 (`fuzzy_contains.rs:461-462`). Une édition casse jusqu'à 3 trigrammes : une faute coûte donc 1 à
  3 manqués, pas un.
- Un `contains` exact (`distance = 0`) ne porte aucune pénalité : son score est le BM25 seul.

Les mesures du 29 août (`BM25Mode::resolve` dans rag3weaver) se lisent donc : −996, c'est 1 manqué et un BM25 de 4 ;
−1 992, 2 manqués et 8 ; −4 991, 5 manqués et 9. La pénalité n'est ni par clause non satisfaite, ni par édition : elle
est **par trigramme manqué**, et vaut 1 000.

**Dans un booléen** (`ContainsSplit`, ou le `parse` de mots simples : un `should` de `contains` par mot et par champ),
les scores des clauses satisfaites s'additionnent ; une clause non satisfaite n'ajoute rien, ni bonus ni pénalité.

## 2. Ce qu'il signifie

- **Négatif veut dire « trouvé, avec des trigrammes manqués »**, jamais « pas trouvé » : un document sans correspondance
  n'est pas rendu.
- **Le millier dit les manqués, le reste dit le BM25.** Deux documents à −996 sont au même palier (1 manqué) avec à peu
  près le même BM25 : équivalents.
- **Le défaut, c'est la somme.** Dans un booléen, chaque clause floue apporte sa pénalité. Un document qui trouve trois
  mots avec un manqué chacun vaut environ −2 985 ; un document qui ne trouve qu'un mot, exact, vaut environ +5 et passe
  devant. Plus on trouve de mots avec une faute, plus on descend. C'est le 0/3 du 29 août.
- **Dans rag3weaver** : la fusion par défaut est RRF (`search.rs:63-69`), qui ne lit que les rangs. Seul l'ordre
  interne de la liste BM25 compte, et le défaut de la somme le fausse déjà. Avec la fusion pondérée et la normalisation
  min-max (`search.rs:96-103`), les paliers de 1 000 écrasent toute différence de BM25 à l'intérieur d'un palier.

## 3. Le transformer

Une grandeur lisible et comparable existe déjà dans les données : la **couverture** d'une clause, la part des trigrammes
trouvés, `1 − manqués / trigrammes`, dans [0, 1]. Elle vaut 1 pour une correspondance exacte.

Proposition, à arbitrer :
- **le score d'une clause floue devient `BM25 × couverture`** : positif, nul quand rien n'est trouvé, un peu plus bas
  quand des trigrammes manquent. La somme sur les clauses récompense alors les mots trouvés au lieu de les punir ;
- **lucivy rend à côté** de chaque résultat, par clause ou en somme : les manqués, les trigrammes de la requête, les
  clauses satisfaites. rag3weaver peut ainsi dire « trouvé avec une faute » sans le lire dans un millier ;
- **pour la fusion** : un score positif et borné par document, que min-max et RRF lisent sans palier. Le classement
  « exact avant flou » reste vrai à BM25 égal, puisque la couverture d'une correspondance exacte est 1.

Cela change le score de lucivy, donc ses classements : c'est une **4.4.2**, publiée au mot de Lucie, et le banc de
recherche doit en mesurer l'effet (nom exact, noms courts, questions en français) avant la pose.

## 4. Le témoin

Dans lucivy, rouge aujourd'hui :
- un document trouve trois mots de la requête avec une faute chacun, un autre un seul mot, exact. Le premier doit passer
  devant ;
- un document exact passe devant le même document avec une faute, à BM25 égal ;
- tout score de `contains` est positif ou nul.

Rien n'est codé : à relire d'abord.
