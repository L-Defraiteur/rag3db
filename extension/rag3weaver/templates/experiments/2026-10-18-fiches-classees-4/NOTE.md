# Tentative 4 des fiches classées — 18 octobre 2026, jouée sur luciepc à 18 h 13

*Les deux retours de la tentative 3 : **une décision par fiche** au lieu d'un oui / non
par candidat, et **l'instruction sur les fiches de règle**. Voie du LLM seule (Gemini par
Vertex, graphe `decide_subject_llm.json`), même jeu (`tests/data/banc-fiches-classees/v1.json`),
mêmes trois candidats par `recall_subjects`. Poussée depuis luciepc en 5b5ce8688 ; les
décisions brutes sont dans `resultats/llm-gemini.json`.*

## Ce qui change

- **La forme** (`tentative.json` : `"forme": "par_fiche"`). L'élément porte
  `{text, candidats}` : la fiche, son pourquoi et les trois sujets candidats avec leur
  description, en un seul texte. Le `decide` prend ses étiquettes dans l'élément
  (`labels_from='candidats'`, 4c3e8179a) et ajoute `aucun` en fixe ; le banc refuse toute
  étiquette hors de cette liste. 39 décisions au lieu de 117 ; deux sujets pour une fiche
  ne sont plus possibles.
- **L'instruction** : « une fiche de règle va sous le domaine où la règle s'applique » ;
  « partager un mot ne suffit pas » reste.
- **Le compte** est le même pour les deux formes, ce qui rend les tentatives comparables
  ligne à ligne.

## Ce qui a tourné

| | tentative 3 (par candidat, 12 oct.) | **tentative 4 (par fiche, 18 oct.)** |
|---|---|---|
| modèle | `gemini-3.5-flash` | **`gemini-3.8-flash`** (le défaut depuis f1d575b50, le même jour) |
| rappel des candidats | 29 / 31 | 29 / 31 |
| fiches justes | 32 / 39 | **36 / 39** |
| abstentions justes | 8 / 8 | **8 / 8** |
| ancrages justes / **faux** / manqués | 25 / 1 / 4 | **28 / 0 / 1** |
| décisions, temps par décision | 117, 5,35 s | **39, 9,6 s** (13,9 s au premier essai) |
| temps total des décisions | 626 s | **374 s** |

Aucune réponse tronquée, aucune étiquette hors liste.

## Les trois fiches qui ne sont pas justes

- **Deux viennent du rappel**, pas du juge, comme à la tentative 3 : le bon sujet n'était
  pas parmi les candidats (« une suite nouvelle se monte par le script de génération… »,
  « bge-m3 rend des vecteurs de 1024 dimensions… ») ; Gemini répond **aucun**, la bonne
  réponse sur ces candidats.
- **Un manqué** : « une estimation en jours d'agent se rend comme un ordre de grandeur »,
  refusée sous *préférences de communication* alors que le sujet est candidat. Des quatre
  manqués de la tentative 3 (des fiches de règle), il ne reste que celui-là : les trois
  sur *historique git* et *modèles d'embarquement* sont maintenant rangés.

## Ce qu'on peut en dire, et ce qu'on ne peut pas encore

**Deux choses ont changé à la fois** : la forme (et l'instruction) et le modèle. Le gain
(32 → 36, 4 → 1 manqués, 1 → 0 faux) ne se partage pas encore entre elles.

- Le rejeu de la tentative 4 en `gemini-3.5-flash` est tombé **deux fois sur un 503**
  (« service unavailable »), chacun après dix minutes de reprises ;
- le rejeu de la tentative 3 en `gemini-3.8-flash`, pour comparer les deux formes au même
  modèle, est tombé sur un **429** (« resource exhausted ») : le quota de Vertex du projet
  était épuisé le 18 octobre au soir, ce qui explique sans doute aussi les 503.

**À refaire quand le quota le permet** : la tentative 3 en `3.8-flash` (la forme par
candidat, au modèle d'aujourd'hui). C'est la comparaison qui tranche ; elle coûte 117
appels.

Ce qui est sûr dès maintenant : **la forme par fiche coûte trois fois moins d'appels**
(39 au lieu de 117) et **ne peut pas ranger une fiche sous deux sujets** (le faux ancrage
de la tentative 3 était un double oui). C'est la forme que reprend l'archiviste de la
mémoire (`docs/18-octobre-2026-memoire-longue/04-…`, §2) : une décision fermée par
morceau — une fiche candidate, aucune, ou en créer une.

## Le retour

1. **Une reprise sur 429 et 503** dans la voie LLM de `Decide`, avec un délai croissant,
   et le banc qui garde les décisions déjà rendues : un quota épuisé après 30 décisions ne
   devrait pas perdre les 30 (aujourd'hui le banc tombe et n'écrit rien).
2. **Le rappel** (2 / 31 absents des candidats) reste à régler côté `recall_subjects`.
3. **L'étiquette « créer »** pour l'archiviste : la tentative 5, avec des fiches dont le
   sujet n'existe pas encore dans le jeu.
