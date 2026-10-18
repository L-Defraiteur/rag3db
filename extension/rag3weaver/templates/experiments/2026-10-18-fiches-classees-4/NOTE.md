# Tentative 4 des fiches classées — préparée le 18 octobre 2026, pas encore jouée

*Les deux retours de la tentative 3 : **une décision par fiche** au lieu d'un oui / non
par candidat, et **l'instruction sur les fiches de règle**. Voie du LLM seule (Gemini 3.5
Flash par Vertex, graphe `decide_subject_llm.json`), même jeu (`tests/data/banc-fiches-classees/v1.json`), mêmes trois
candidats par `recall_subjects`.*

## Ce qui change

- **La forme** (`tentative.json` : `"forme": "par_fiche"`). L'élément porte
  `{text, candidats}` : la fiche, son pourquoi et les trois sujets candidats avec leur
  description, en un seul texte ; `candidats` est la liste de leurs noms. Le `decide`
  prend ses étiquettes dans l'élément (`labels_from='candidats'`) et ajoute `aucun` en
  fixe (`labels='aucun'`) ; le banc refuse toute étiquette hors de cette liste en le
  disant. 39 décisions au lieu de 117 ; deux sujets pour une fiche ne sont plus possibles.
- **L'instruction** : « une fiche de règle va sous le domaine où la règle s'applique »,
  pour les quatre manqués de la tentative 3 (« commiter par chemins listés » sous
  *historique git*, etc.) ; « partager un mot ne suffit pas » reste, pour garder le faux
  ancrage au compteur.
- **Le compte** est le même pour les deux formes (un sujet retenu est un ancrage juste ou
  faux ; le bon sujet présent et pas retenu, un manqué), ce qui rend les tentatives 3 et
  4 comparables ligne à ligne.

## Ce qu'elle attend

- **`labels_from` dans `Decide`** : promis par la session du proto (12 oct.), pas encore
  sur master au 18 oct. (`git grep labels_from` vide sur toutes les branches). Sans lui,
  le graphe refuse de se charger.
- **Le jeton du poste** (règle du 18 oct.) pour la compilation de l'e2e et l'appel à
  Vertex ; la passe passe après le cœur.

## À comparer à la tentative 3

| | tentative 3 (par candidat) | tentative 4 (par fiche) |
|---|---|---|
| fiches justes | 32 / 39 | — |
| abstentions justes | 8 / 8 | — |
| ancrages justes / faux / manqués | 25 / 1 / 4 | — |
| décisions, temps par décision | 117, 5,35 s | — |
