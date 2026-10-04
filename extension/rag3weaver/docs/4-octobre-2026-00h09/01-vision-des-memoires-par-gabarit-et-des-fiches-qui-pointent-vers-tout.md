# Vision — des mémoires que l'agent conçoit, et des fiches qui pointent vers tout

4 octobre 2026, seconde version. La première proposait un modèle fixe (« le
fil de travail » et trois genres d'ancre). Lucie l'a élargie :

> « C'est justement le challenge : il faut qu'il ait autant de mémoires qu'il
> veut, et qu'il puisse en concevoir par gabarit, avec des gabarits tout faits
> proposés par défaut — fil de travail, sujets, dossiers. Et au final ça reste
> toujours ça : soit il lie telle plage de données, ou une requête peut-être
> préconstruite, à sa fiche ; il peut y lier aussi un lien externe, vers un
> site ou un fichier local ; il peut y lier tel schéma de base, tel graphe de
> DAG, tel gabarit… pensons vraiment boucle étrange, même là-dedans. »

Rien n'est décidé ni codé. Ce document complète la vision de la mémoire
longue (`../3-octobre-2026-20h37/02`) et sa proposition
(`../3-octobre-2026-20h41/01`).

## 1. L'idée en trois phrases

1. **Une mémoire est une base de fiches déclarée par un gabarit**, et l'agent
   en a autant qu'il veut : il en ouvre une depuis un gabarit proposé, ou il
   conçoit le sien.
2. **Une fiche pointe vers n'importe quoi d'adressable** : des données, une
   requête, un lien externe, un fichier, un morceau du schéma, un graphe, un
   gabarit, une autre fiche, une autre mémoire.
3. **Ce dont le système est fait est adressable comme le reste** : c'est la
   boucle étrange — la mémoire peut parler de ses propres gabarits, de ses
   propres graphes, d'elle-même.

## 2. Des mémoires par gabarit

Le moteur sait déjà le faire, sans code : le gabarit `notebook` et le gabarit
`memory` déclarent une entité, ses champs, ce qui se cherche par mots et par
sens, une machine à états, des outils — par un manifeste.

Ce que la vision ajoute :

- **Des gabarits proposés par défaut**, chacun une petite forme de fiche :
  - *fil de travail* — une PR, un chantier : but, état, ce qui reste ;
  - *sujets* — un thème et ce qu'on en sait ;
  - *dossier* — une chose suivie dans le temps, avec ses pièces ;
  - *décisions* — ce qui a été tranché, par qui, pourquoi ;
  - *préférences* — ce que la personne veut, avec ses mots.
- **Ouvrir une mémoire** : l'agent en crée une depuis un gabarit, lui donne
  un nom et une portée (la personne, le projet, l'équipe). Rien ne limite
  leur nombre.
- **Concevoir un gabarit** : l'agent décrit une forme de fiche — ses champs,
  ses états, ses genres — et le système en fait un gabarit, vérifié comme un
  manifeste l'est déjà (un champ inconnu, une transition impossible sont
  refusés en disant quoi faire). Un gabarit conçu se réutilise et se partage.

Le garde-fou contre le désordre est le même que pour les sujets : avant
d'ouvrir une mémoire ou de créer un gabarit, le système montre ceux qui
ressemblent et demande de préciser lequel on retient. Ce n'est pas un refus.

## 3. Une fiche pointe vers tout

« Au final ça reste toujours ça. » Une fiche a un texte, et des **liens**. Un
lien a une **adresse**, et l'adresse a un genre :

| Genre | Ce qu'il désigne | Exemple | Ce qui le fait vieillir |
|---|---|---|---|
| ligne | une ligne d'une entité | une fonction, une carte, une autre fiche | l'ingestion ou la synchronisation dit qu'elle a changé |
| plage | un ensemble de lignes, figé | « les 12 fonctions touchées par cette PR » | une des lignes change ou disparaît |
| requête | une question enregistrée, rejouée à la lecture | « les fonctions qui appellent `begin_snapshot` » | jamais périmée : elle rend l'état du jour ; on garde le compte vu pour dire ce qui a bougé |
| externe | une adresse hors du système | un site, une PR, une issue | la date seule |
| fichier | un chemin local | `docs/users.md` | l'empreinte de son contenu |
| schéma | une entité, une relation, un champ | `User`, `User.email`, `FOLLOWS` | une migration, une redéclaration |
| graphe | un graphe de traitement, un outil | `search_workspace`, `usages` | le fichier du graphe change |
| gabarit | un gabarit de mémoire ou de backend | le gabarit *fil de travail* | le gabarit change |
| mémoire | une mémoire entière | « la mémoire des décisions du projet » | — |

Trois choses tiennent pour tous les genres :

- **Une seule forme d'adresse**, lisible : `row:Scope/…`, `query:…`,
  `url:…`, `path:…`, `schema:User.email`, `graph:usages`,
  `template:fil-de-travail`, `kb:decisions`. L'agent n'apprend qu'une
  syntaxe ; un genre nouveau s'ajoute sans changer les fiches.
- **Le lien se lit dans les deux sens.** Depuis la fiche : ce qu'elle
  concerne. Depuis la cible : les fiches qui en parlent — c'est ce qui arrive
  avec le résultat d'un outil quand l'agent touche la cible.
- **Chaque genre dit comment il vieillit.** Une fiche dont une cible a bougé
  passe « à revoir » et le dit ; elle n'est jamais fausse en silence.

**La requête préconstruite** mérite un mot : c'est le lien le plus puissant.
Une plage figée décrit le passé ; une requête décrit une intention (« tout ce
qui dépend de `User` ») et se rejoue. Le langage de requête structuré et les
outils `usages` et `impact` existent : une requête enregistrée est un appel
d'outil avec ses arguments, gardé sous un nom.

## 4. La boucle étrange

Les genres *schéma*, *graphe*, *gabarit* et *mémoire* désignent ce dont le
système lui-même est fait. Conséquences, toutes voulues :

- **Le système se documente en lui-même.** La raison d'être d'une entité, le
  pourquoi d'un seuil dans un graphe, la décision derrière un gabarit sont
  des fiches liées à ces objets. L'agent qui ouvre le schéma d'une entité lit
  le pourquoi avec.
- **Un gabarit de mémoire est décrit par des fiches** d'une mémoire qui a,
  elle aussi, un gabarit. La mémoire « gabarits » contient la fiche du
  gabarit qui la définit.
- **L'agent qui conçoit laisse sa trace dans ce qu'il conçoit** : créer un
  gabarit écrit la fiche qui dit pourquoi il existe ; modifier un graphe fait
  passer « à revoir » les fiches qui l'expliquent.
- **Une fiche peut pointer vers une requête qui rend des fiches**, vers la
  mémoire qui la contient, vers elle-même.

Pour que la boucle reste un outil et pas un piège :

- **Une adresse se résout paresseusement** : suivre un lien est un geste
  demandé, jamais une expansion automatique sans fin. Un rendu suit au plus
  un niveau et dit ce qu'il n'a pas suivi.
- **Ce qui est du système est en lecture pour la mémoire** : une fiche parle
  d'un graphe, elle ne le modifie pas. Concevoir un gabarit ou un graphe
  passe par les outils faits pour cela, avec leurs vérifications.
- **Un registre unique des objets du système** : pour être adressables, les
  entités, relations, graphes, gabarits et mémoires doivent être des lignes
  quelque part. C'est « le catalogue comme graphe »
  (`vision_roadmap_09_2026/04`), que cette vision rend nécessaire : une
  mémoire du système, tenue par le catalogue, où chaque objet déclaré a sa
  ligne.

## 5. Ce que l'agent a en main

Peu de verbes, qui valent pour toutes les mémoires :

- **ouvrir** une mémoire depuis un gabarit ; **lister** les mémoires et les
  gabarits ;
- **concevoir** un gabarit ;
- **noter** une fiche, avec ses liens (le `remember` de la proposition, avec
  la demande de précision quand un proche existe) ;
- **lier** et **délier** une fiche et une adresse ;
- **retrouver** : par question, par adresse (« que sait-on de `schema:User` ? »),
  par mémoire ;
- et le rappel qui vient seul, par le crochet après outil, quand l'agent
  touche une cible liée.

## 6. Ce que le moteur donne et ce qui manque

| Besoin | État |
|---|---|
| Une mémoire déclarée par gabarit, sans code | livré (`notebook`, `memory`) |
| Ouvrir une mémoire à la demande, plusieurs par base | à faire : aujourd'hui un manifeste se charge au démarrage |
| Concevoir un gabarit par un outil, vérifié | à faire ; la vérification des manifestes existe |
| Lien vers une ligne, « à revoir » quand elle change | proposé ; l'événement d'ingestion est livré |
| Plage et requête enregistrée | à faire ; le langage de requête et les outils existent |
| Lien externe, fichier avec empreinte | à faire, petit |
| Les objets du système comme lignes (le registre) | à faire : la pièce qui ouvre la boucle |
| Le rappel par ce qu'on touche | crochet après outil : mécanique livrée |
| Plusieurs agents qui notent en même temps | attend les écritures parallèles |

## 7. Par où commencer

1. **L'adresse** : sa syntaxe et ses genres, écrits une fois. C'est elle qui
   décide de tout le reste, et elle ne coûte rien à dessiner large.
2. **Deux gabarits par défaut** (*fil de travail*, *sujets*) et le lien vers
   une ligne, un fichier et une adresse externe : assez pour l'exemple de
   Lucie (« ma PR users, tel design, tel md »).
3. **Le registre des objets du système**, puis les genres *schéma*, *graphe*
   et *gabarit* : la boucle.
4. **La requête enregistrée.**
5. **Ouvrir et concevoir** des mémoires par un outil.

Le banc de la mémoire longue mesure le désordre (doublons, abandons après une
demande de précision) : il vaudra pour les mémoires et les gabarits comme
pour les fiches.

## 9. L'archiviste (idée de Lucie, 4 octobre — pour plus tard)

> « Un agent différent pourrait se brancher sur ce qui se passe chez un autre
> et lui enregistrer ses mémoires sans qu'il s'en rende compte, ou lui en
> injecter — sauf demande explicite de la personne, en mode "note ça en
> mémoire". Genre un archiviste. »

L'agent qui travaille ne tient pas sa mémoire : un second agent, l'archiviste,
regarde passer la conversation et les appels d'outils, et fait les deux
gestes à sa place.

- **Il note.** Il lit le fil (les tours, les résultats d'outils, les
  corrections de la personne) et écrit les fiches : les décisions, les
  préférences, ce qui a été essayé sans succès. L'agent qui travaille n'a
  rien à décider ni à interrompre.
- **Il rappelle.** Avant un tour ou avec un résultat d'outil, il glisse les
  fiches utiles — par le crochet après outil, qui est déjà la porte prévue
  pour cela.
- **La personne garde la main directe** : « note ça en mémoire » s'adresse à
  l'agent qui travaille, qui écrit alors lui-même, avec ses mots à elle.

Pourquoi cela vaut d'être gardé : les passes d'agent ont montré qu'un modèle
qui travaille n'écrit ni ne relaie ce qu'on ne l'oblige pas à écrire ; un
agent dont c'est le seul métier le fera. Et la séparation règle la question
de l'origine : ce que l'archiviste déduit est marqué comme tel, ce que la
personne a dit reste sa parole.

Ce qu'il demande, et qui existe en partie : le fil d'événements d'une session
(le chat le journalise déjà), le crochet après outil (livré), les mémoires
par gabarit, et un modèle peu coûteux qui tourne à côté. Ce qu'il faudra
tenir : il ne modifie jamais le travail de l'autre ; ce qu'il injecte se voit
(une section nommée, pas un texte fondu) ; il a un budget ; et la personne
peut lire, corriger et effacer ce qu'il a retenu.

### 9.1 La difficulté, dite par Lucie

> « Le truc ne doit pas travailler à chaque tour, H24, ni injecter H24 du
> contexte que l'autre a peut-être déjà en tête. Ce n'est pas à faire à la
> légère : il faut bien organiser ça. »

Deux coûts à tenir, et ils se règlent séparément.

**Quand il travaille.** Pas à chaque tour : sur des **événements**, que le
harnais sait reconnaître sans modèle.

| Déclencheur | Ce qu'il fait |
|---|---|
| la personne corrige l'agent, ou tranche un choix | note la décision, avec ses mots |
| un lot se termine (une fusion, un test qui passe du rouge au vert) | note ce qui a été fait et pourquoi |
| un échec répété, puis un changement d'approche | note ce qui n'a pas marché |
| le contexte de l'agent va être résumé ou la session se ferme | une passe de rattrapage sur ce qui n'a pas été noté |
| rien de tout cela | rien : il dort |

Entre deux déclencheurs il ne lit rien et n'appelle aucun modèle. Le fil des
événements est gardé ; il le reprend là où il s'était arrêté, par lots.

**Quand il injecte.** Trois filtres, dans cet ordre, et le silence par défaut :

1. **Pertinence** : seulement ce qui est accroché à ce que l'agent touche
   maintenant (le fichier lu, l'entité interrogée) — jamais « ce qui pourrait
   servir ».
2. **Nouveauté pour cet agent** : ne pas redire ce qu'il a déjà. Le harnais
   tient le registre de ce qui a été montré dans cette session (les fiches
   injectées, les fichiers lus, ce que l'agent a lui-même écrit) ; une fiche
   déjà montrée ne revient pas, sauf si elle a changé ou si le contexte a été
   résumé depuis.
3. **Valeur** : une fiche qui contredit ce que l'agent s'apprête à faire, ou
   qui est marquée « à revoir », passe avant une fiche qui confirme.

Et un budget par session, pas seulement par appel : quelques lignes à la
fois, un plafond sur l'ensemble. Une section vide ne s'affiche pas.

**Ce qu'on ne sait pas, et qu'il faudra mesurer avant de l'allumer** : ce que
« l'agent l'a déjà en tête » veut dire quand son contexte est long (une chose
lue il y a deux cents tours est-elle encore là ?) ; le taux d'injections
inutiles qu'un agent tolère avant d'ignorer la section — les passes d'agent
ont montré qu'une ligne répétée finit ignorée ; et ce que coûte l'archiviste
en appels pour ce qu'il rapporte. Le banc de la mémoire le dira : injections
faites, injections qui ont changé ce que l'agent a fait, redites.

**L'ordre pour y arriver** : d'abord la moitié « noter », hors ligne, en fin
de session, sur le journal d'événements — elle ne gêne personne et se juge
à froid. Puis les déclencheurs. L'injection en dernier, éteinte par défaut,
allumée fiche par fiche.

### 9.2 Seconde forme : le classeur et la porte à deux temps (Lucie, 4 octobre midi)

> « Il faut qu'il travaille quand même chaque tour ou chaque compression de
> contexte, à la manière d'une mémoire hiérarchique : chaque fois qu'on
> atteint 30 % du contexte, un résumé est généré, un L1 ; l'archiviste le
> classe, avec un sujet en plus de la date — il organise comme un classeur.
> Et à chaque tour il décide, mais on l'encourage à ne pas chercher plusieurs
> fois la même chose. Peut-être d'abord une décision structurée pour savoir
> s'il faut chercher, et après la recherche, s'il faut injecter. Peut-être
> des outils plutôt que des appels structurés. C'est encore flou. »

Cela corrige §9.1 sur un point : l'archiviste a deux rythmes, pas des
déclencheurs épars.

**Rythme lent — classer, à chaque compression.**

- Quand le contexte de l'agent atteint un seuil (30 %), la tranche écoulée
  est résumée : un **L1**.
- L'archiviste **classe** le L1 : sa date, et un ou plusieurs **sujets**. Le
  classement d'un texte parmi des sujets existants est ce que la mesure a
  montré de plus sûr (la similarité contre le contenu complet du sujet) ; la
  création d'un sujet neuf suit la règle de la mémoire : montrer les proches,
  demander lequel on retient.
- Le classeur est **hiérarchique** : un sujet qui accumule des L1 reçoit un
  **L2**, son résumé à lui, refait quand il a assez changé — c'est le sujet
  « dérivé de ce qu'il porte ». Chaque L1 garde l'adresse de la tranche brute
  d'où il vient : on peut toujours redescendre du L2 au L1, du L1 aux tours.
- Un **sommaire** du classeur — les sujets, une ligne chacun — est petit et
  stable.

**Rythme rapide — rappeler, à chaque tour, par une porte à deux temps.**

```
tour ──▶ faut-il chercher ? ──non──▶ rien
              │ oui
              ▼
         chercher dans le classeur (borné)
              │
              ▼
         faut-il injecter ? ──non──▶ rien (mais on retient qu'on a cherché)
              │ oui
              ▼
         une section courte, nommée, avec ce que l'agent n'a pas déjà
```

- **Premier temps, sans modèle si possible.** « Chaque tour » n'est tenable
  que si ce temps ne coûte presque rien : le tour comparé au sommaire du
  classeur (un embarquement, quelques millisecondes), plus le registre de la
  session. Un modèle n'est appelé que quand ce score laisse un doute.
- **Ne pas chercher deux fois.** Le registre de la session garde ce qui a été
  cherché (le sujet, le tour, ce que ça a rendu) et ce qui a été montré. Une
  recherche déjà faite sur un sujet qui n'a pas changé ne se refait pas : la
  porte le voit et répond non.
- **Second temps : injecter ou non.** Trois refus avant un oui : l'agent l'a
  déjà (montré, ou lu par lui, depuis le dernier résumé) ; ce n'est pas
  accroché à ce qu'il fait ; cela ne change rien à ce qu'il s'apprête à faire.

**Décision structurée ou outils ?** Les deux, chacun à sa place — c'est la
forme que le système a déjà.

- Les deux portes sont des **décisions fermées** (chercher ou non ; injecter
  ou non) : un nœud de décision, pas une boucle d'agent. Elles se mesurent.
- La recherche dans le classeur est un **graphe**, borné, pas une exploration
  libre.
- L'archiviste entier est donc un **graphe de traitement**, déclaré comme un
  outil : porte, recherche, porte, rendu. Pas un second agent qui converse.
- Et l'agent qui travaille garde un **outil** pour tirer lui-même sur le
  classeur (« rappelle ce qu'on sait de tel sujet »), avec le sommaire sous
  les yeux : ce qu'il sait vouloir, il le demande ; l'archiviste ne pousse que
  ce qu'il ne sait pas devoir demander.

**Ce qui reste flou, à trancher par la mesure et pas par l'intuition** :

1. Qui écrit le L1 : le résumé que le harnais produit déjà en compressant, ou
   un résumé fait pour le classeur (plus factuel : décisions, échecs, état) ?
2. « 30 % » : du contexte total, ou depuis le dernier L1 ?
3. La première porte tient-elle sans modèle ? La mesure des modèles de
   décision dit qu'un score ordonne bien et juge mal, et qu'un seuil ne se
   transporte pas d'un jeu à l'autre : il faudra un banc de tours réels,
   étiquetés « il fallait chercher » ou non.
4. Le sommaire sous les yeux de l'agent suffit-il à ce qu'il tire de
   lui-même ? Les passes d'agent diront s'il s'en sert.

## 8. Ce qui attend un choix de Lucie

1. L'ordre du §7, ou la boucle (le registre) d'abord.
2. Qui peut concevoir un gabarit : tout agent, ou seulement avec l'accord de
   la personne.
3. Une mémoire par base, ou toutes les mémoires d'une personne dans une même
   base (ce qui rend les liens entre mémoires directs).
