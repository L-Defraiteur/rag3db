# Expliquer le code d'un diff, îlot par îlot, à qui apprend

- **État** : ouvert (21 novembre 2026, matin)
- **Famille** : produit (un outil déclaré, la vue des diffs de l'IDE)
- **Gravité** : une fonctionnalité neuve, voulue par Lucie. Son rang (c7) : après la liste des agents et les diffs des vues.
- **Atteignable en service** : non : elle n'existe pas.
- **Touche rag3weaver** : oui (un gabarit d'outil sous `templates/tools/`, l'appel au modèle du produit).
- **Pour** : everything declarative (l'outil) ; recherche (le bouton, dans sa tranche des diffs)
- **Signalé par** : Lucie, relayée par c7 (21 novembre 2026, 8 h 2x).

## Ce que veut Lucie

« Une feature expliquer le code sur les diffs, ça vient prendre chaque îlot du diff et t'expliquer ce que fait le code,
comme à quelqu'un qui apprend à coder, pour ramener les devs à une capacité de comprendre les boucles, etc., sur clic
sur un bouton. »

## La forme

**Un outil déclaré**, `explain_diff` : un gabarit `templates/tools/explain_diff.mmd`, qui appelle le modèle du produit
par `LlmNode` (Gemini par défaut, le modèle déclaré à `models.llm`). Aucun code nouveau n'est attendu, sauf si un nœud
manque pour découper le diff en îlots ; dans ce cas, il est petit et pur.

- **L'entrée** : un diff unifié entier, ou un seul îlot (un bloc `@@ … @@` et ses lignes), avec le chemin du fichier et
  son langage quand on les connaît. En option : le niveau du lecteur (`débutant` par défaut) et la langue de
  l'explication (le français par défaut).
- **La sortie** : une explication par îlot, dans l'ordre du diff, chacune liée à son îlot (le fichier, l'en-tête
  `@@`, les lignes). Pour chaque îlot : ce que faisait le code avant, ce qu'il fait après, et les notions qu'il
  emploie, expliquées simplement (une boucle, une condition, un appel, une erreur rendue…).
- **L'invite** : **au gabarit**, dans le `.mmd` ou un fichier de texte qu'il cite, pour qu'on la change sans rebâtir.
  Elle dit au modèle de parler comme à quelqu'un qui apprend à coder, d'expliquer les notions plutôt que de les
  nommer, et de ne jamais inventer ce que le diff ne montre pas.
- **Le coût, par îlot** : un appel au modèle par îlot, borné par `max_tokens` (de l'ordre de 600 en sortie). L'entrée
  est l'îlot et ses quelques lignes de contexte, soit quelques centaines de jetons. Un diff de dix îlots coûte donc
  dix appels courts, environ 10 000 jetons au total. Les appels d'un même diff peuvent partir en parallèle. Au-delà
  d'un nombre d'îlots déclaré (20 par défaut), l'outil explique les premiers et dit combien il en reste.
- **Chaque explication cite les lignes qu'elle commente** (c7) : les numéros de l'îlot, ancien et nouveau côté, pour
  qu'une erreur du modèle se voie en relisant. Une explication qui ne cite aucune ligne est refusée par le schéma de
  sortie.
- **Le contexte donné au modèle est la fonction ou le scope entier autour de l'îlot** (c7), pas l'îlot seul : lu par
  le backend de code quand le dépôt est indexé (le scope qui contient les lignes de l'îlot), sinon les lignes voisines
  du fichier, bornées (par exemple 40 de chaque côté). Le coût par îlot monte d'autant : de l'ordre de 1 000 à
  3 000 jetons d'entrée.
- **Deux usages** : apprendre, et **relire ce qu'un agent vient d'écrire**. Le bouton sert aussi sur le diff d'une
  tâche de l'équipier.
- **Pas de chemin chaud** : c'est un clic. Rien ne s'appelle à l'index ni à la recherche, hors de la lecture du scope
  qui entoure un îlot.

## Ce qu'il faut pour le fermer

1. Le gabarit `explain_diff.mmd`, son invite au gabarit, ses bornes déclarées (`max_tokens`, le nombre d'îlots), et
   son exposition dans le manifeste `code`.
2. Un témoin, rouge d'abord, par le faux modèle des témoins (`fake_sse`) : un diff de deux îlots donne deux
   explications, dans l'ordre, chacune liée à son îlot et citant ses lignes ; le modèle reçoit le scope entier autour
   de l'îlot (un dépôt indexé) ou ses lignes voisines bornées (un dépôt sans index) ; et l'invite servie est celle du
   gabarit (la changer change ce que reçoit le modèle, sans rebâti).
3. Le bouton dans la vue des diffs, par recherche, sur cet outil.
