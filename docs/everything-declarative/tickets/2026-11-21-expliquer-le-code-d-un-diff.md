# Expliquer le code d'un diff, îlot par îlot, à qui apprend

- **État** : ouvert (21 novembre 2026, matin)
- **Famille** : produit (un outil déclaré, la vue des diffs de l'IDE)
- **Gravité** : une fonctionnalité neuve, voulue par Lucie.
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
- **Pas de chemin chaud** : c'est un clic. Rien ne s'appelle à l'index ni à la recherche.

## Ce qu'il faut pour le fermer

1. Le gabarit `explain_diff.mmd`, son invite au gabarit, ses bornes déclarées (`max_tokens`, le nombre d'îlots), et
   son exposition dans le manifeste `code`.
2. Un témoin, rouge d'abord, par le faux modèle des témoins (`fake_sse`) : un diff de deux îlots donne deux
   explications, dans l'ordre, chacune liée à son îlot ; et l'invite servie est celle du gabarit (la changer change
   ce que reçoit le modèle, sans rebâti).
3. Le bouton dans la vue des diffs, par recherche, sur cet outil.
