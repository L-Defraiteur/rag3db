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
  son langage quand on les connaît. En option : le niveau du lecteur et la langue de l'explication (le français par
  défaut). **Le niveau prend trois valeurs** : `débutant` (par défaut : les notions expliquées, comme à qui apprend),
  `technique` (le comportement, sans expliquer les notions de base), `architecture` (les invariants que l'îlot touche :
  un verrou, un format, une frontière entre modules, ce qui doit rester vrai).
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
- **Chaque explication sépare trois parties, explicitement** (un avis que Lucie a fait lire, relayé par c7) ; c'est la
  parade à l'explication fausse et sûre d'elle :
  1. **ce que le diff montre** : des faits, chacun avec les lignes citées ;
  2. **ce qu'on en déduit** : l'intention probable, dite comme une hypothèse (« probablement », « semble ») ;
  3. **ce qui est vérifié** : les tests qui couvrent ces lignes, s'il y en a (par l'outil `impact` du backend de code
     quand le dépôt est indexé), sinon « non vérifié ».
  Le schéma de sortie exige les trois parties ; une explication à laquelle il en manque une est refusée.
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
   explications, dans l'ordre, chacune liée à son îlot et citant ses lignes, avec ses trois parties (montré, déduit,
   vérifié) et une réponse sans l'une d'elles refusée ; les trois niveaux changent l'invite servie ; le modèle reçoit le scope entier autour
   de l'îlot (un dépôt indexé) ou ses lignes voisines bornées (un dépôt sans index) ; et l'invite servie est celle du
   gabarit (la changer change ce que reçoit le modèle, sans rebâti).
3. Le bouton dans la vue des diffs, par recherche, sur cet outil.

## Précision de Lucie (21 novembre 2026, 8 h 4x) : des vues, et les justifications en entrée

**Les niveaux deviennent des vues.** Elles remplacent les trois niveaux écrits plus haut :
1. **« Comprendre le code »** : le déroulé, pour qui apprend (« la boucle `for` permet de… ») ;
2. **« Choix techniques »** : pour un humain expérimenté, le pourquoi (« `React.memo` parce que… ») ;
3. **« Architecture »** (l'abstraction, le flux des données) : **seulement si elle se sépare proprement de (2)**, sinon
   elle est fondue dedans.

Les deux premières se font d'abord. **Les trois parties (montré, déduit, vérifié) restent dans chaque vue.**

**L'entrée s'enrichit.** En plus du code de l'îlot et de son scope, le modèle reçoit les **justifications techniques**
attachées à l'îlot, quand il y en a, **citées comme sources** :
- celles du **relecteur**, qui soutient un îlot ou s'y oppose (leur format est fait par archi, côté relecteur) ;
- celles de **l'agent auteur**, si son run les a dites.

Une justification citée entre dans « ce qu'on en déduit » avec sa source ; elle n'y passe jamais pour un fait.
L'explication ne la reprend pas comme vraie sans la confronter au diff.

**Le témoin** vérifie en plus : les deux vues donnent deux invites distinctes, servies du gabarit ; une justification
donnée en entrée est citée comme source dans la sortie ; une sortie qui présente une justification comme un fait
montré est refusée.
