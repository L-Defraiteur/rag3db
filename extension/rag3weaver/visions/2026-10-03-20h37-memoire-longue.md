# Vision — une mémoire longue pour des agents

3 octobre 2026. Demande de Lucie : « un produit, si on peut dire "long term
memory", à travailler en parallèle : comment laisser des agents noter des
mémoires sans que ce soit un bordel ».

Rien n'est décidé ni codé. Ce document dit pourquoi cela devient un bordel, et
ce qui l'empêche.

## 1. Pourquoi cela devient un bordel

Observé sur une mémoire de travail faite de fichiers de texte et d'un index
d'une ligne par fichier, tenue plusieurs semaines :

- **Des doublons.** Deux notes disent que `/tmp` est de la mémoire vive, deux
  autres donnent le même régime GPU, deux la même règle sur les tests. Chaque
  auteur a écrit sans voir la note de l'autre.
- **Du périmé qui parle comme du vrai.** Une notice disait un modèle « publié
  nulle part » alors qu'il l'était ; rien ne signale qu'une note a vieilli.
- **Des règles trop fortes.** Une interdiction née d'un excès de prudence,
  écrite comme une règle, qu'il a fallu défaire.
- **Tout est chargé, tout le temps.** L'index entier entre dans chaque
  session ; il grossit sans fin.
- **Pas d'ancrage.** Une note qui nomme un fichier ne sait pas que le fichier
  a changé.
- **Pas d'origine.** On ne distingue pas ce que la personne a dit de ce que
  l'agent en a déduit.

## 2. Huit règles

1. **Une mémoire est une entité typée, pas un texte libre.** Une affirmation
   courte ; un genre (fait, préférence, décision, pointeur) ; un pourquoi ;
   une portée ; une origine ; une date.

2. **Écrire, c'est d'abord chercher.** L'outil d'écriture cherche les mémoires
   proches (par sens et par ancre) et les montre : l'agent doit choisir entre
   compléter, remplacer, ou créer. Le doublon se traite à l'écriture, pas au
   ménage.

3. **On ne réécrit pas, on remplace.** Une mémoire nouvelle garde un lien vers
   celle qu'elle remplace. La lecture ne rend que la courante ; l'histoire
   reste consultable. Tout est daté.

4. **Une mémoire s'accroche à ce dont elle parle.** Un fichier, une fonction,
   une personne, un projet, une carte : une relation. Quand la chose change,
   la mémoire passe « à revoir ». Une mémoire sans ancre est globale, et rare.

5. **L'origine se garde.** « Dit par la personne » (avec ses mots) pèse plus
   que « déduit par l'agent ». Une déduction se marque comme telle, et
   n'écrase jamais une parole.

6. **Des portées nettes.** La personne, le projet, l'équipe, la session. Ce
   qui ne vaut que pour la conversation en cours ne s'écrit pas.

7. **Le rappel est une recherche, pas un chargement.** Ce qui revient dépend
   de la tâche et de ce que l'agent touche : il lit un fichier, les mémoires
   accrochées à ce fichier arrivent avec le résultat de l'outil (le crochet
   après outil du document 01). Avec un budget.

8. **Un jardinier.** Une passe périodique, faite par un agent : fusionner les
   proches, signaler les contradictions (à la personne, jamais tranchées
   seules), laisser s'éteindre ce qui n'est plus rappelé. Rien n'est détruit :
   mis de côté, avec un délai.

## 3. Ce que le moteur donne déjà

| Règle | Pièce existante |
|---|---|
| Entité typée, portée | `EntityConfig`, les cellules organisation/projet |
| Chercher avant d'écrire | la recherche fusionnée (mots, sens) |
| Remplacer sans perdre | une relation entre deux mémoires ; la mise de côté de 7 jours |
| Ancrage, « à revoir » | les relations ; la synchronisation par périmètre, qui sait ce qui a changé |
| Origine, date | des champs ; des propriétés d'arête |
| Rappel par ce qu'on touche | les relations, le crochet après outil (à faire) |
| Plusieurs agents qui notent | les écritures parallèles (en cours) |

Ce qui manque : le crochet après outil ; la marque « à revoir » déclenchée par
une synchronisation ; le jardinier ; la détection de contradiction.

## 4. La forme du produit

Un serveur que n'importe quel agent branche (MCP), avec peu de verbes :

- `remember` : propose une mémoire ; rend les proches ; exige le choix.
- `recall` : par question, par ancre, ou les deux.
- `revise` : remplace, avec la raison.
- `forget` : met de côté.
- une page lisible par la personne, où elle corrige et supprime.

Le même moteur sert le produit code (les décisions accrochées au code sont des
mémoires ancrées à des scopes) et un usage sans code (un assistant qui se
souvient d'une personne, d'un dossier).

## 5. Comment le prouver

Un banc, avant tout réglage : un scénario de plusieurs sessions où un agent
reçoit des faits, des corrections, des redites et un fait qui se périme. On
mesure : le nombre de doublons créés, les rappels utiles, les rappels périmés
servis comme vrais, les contradictions vues.

Selon la règle de Lucie : la tuyauterie avec un modèle local (l'outil est
appelé, le choix est exigé), la qualité avec un modèle de la classe visée.

## 6. Ce qui attend un choix de Lucie

1. Le premier usage visé : la mémoire d'un agent de code (ancrée au dépôt), ou
   une mémoire générale d'assistant.
2. Qui peut écrire sans confirmation : tout agent, ou seulement après une
   parole de la personne.
3. Ce que la personne voit : chaque écriture au fil de l'eau, ou la page à la
   demande.
