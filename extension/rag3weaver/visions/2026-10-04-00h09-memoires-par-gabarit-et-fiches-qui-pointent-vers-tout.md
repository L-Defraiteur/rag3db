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
longue (`extension/rag3weaver/visions/2026-10-03-20h37-memoire-longue.md`) et sa proposition
(`../docs/3-octobre-2026-20h41/01`).

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

### 9.3 Les trois étages du classeur (Lucie, 4 octobre)

> « À 100 % du contexte, tous les sujets touchés cette session sont à
> re-résumer, en tenant compte des autres L1 du même jour (ou des L2 déjà
> faits ce jour) : le sujet garde un article résumant le jour, c'est le L2.
> Et quand trop de L2 sont présents dans un sujet, en caractères ou en jetons,
> un crochet appelle un archiviste neutre pour un résumé global, intemporel
> même si on lui donne la date du jour : un L3, l'article principal du sujet. »

| Étage | Ce que c'est | Quand il s'écrit | Ce qu'il lit |
|---|---|---|---|
| **L1** | le résumé d'une tranche de session | à 30 % du contexte | les tours de la tranche |
| **L2** | l'article **du jour** d'un sujet | à 100 % du contexte, pour chaque sujet touché dans la session | les L1 du jour rangés sous ce sujet, et le L2 du jour s'il existe déjà |
| **L3** | l'article **principal** du sujet, intemporel | quand les L2 du sujet dépassent un budget (caractères ou jetons) | les L2 du sujet, et le L3 précédent |

Ce que cette forme règle :

- **Un seul L2 par sujet et par jour.** Une seconde session du même jour ne
  crée pas un second article : elle refait celui du jour avec ce qu'elle
  apporte.
- **Le L3 est ce qu'on compare** pour ranger un texte neuf et pour répondre
  au premier temps de la porte : c'est « le contenu complet du sujet » que la
  mesure demandait, en un seul texte borné. Sans L3 encore, le dernier L2 en
  tient lieu.
- **L'archiviste du L3 est neutre** : il ne sort pas de la session en cours,
  il n'a que les articles. C'est un appel isolé, déclenché par un crochet sur
  la taille, pas par un tour.
- **Le coût est borné par construction** : un L1 par tranche, un L2 par sujet
  touché et par jour, un L3 seulement au dépassement du budget.

Ce qu'il faut tenir, parce qu'un résumé de résumés dérive :

1. **Rien ne se perd en montant.** Chaque L2 garde l'adresse de ses L1, chaque
   L1 celle de ses tours ; le L3 cite les L2 dont il vient. On redescend
   toujours. Un L2 ou un L3 refait **remplace** le précédent sans l'effacer.
2. **Le L3 dit ce qui a changé, pas seulement ce qui est.** « Intemporel » ne
   veut pas dire sans histoire : une décision renversée doit rester lisible
   comme renversée (« on faisait X jusqu'au 3 octobre ; depuis, Y, parce
   que… »), sinon l'article principal efface justement ce qu'une mémoire doit
   garder. La date du jour lui sert à cela.
3. **Ce que la personne a dit ne se résume pas.** Ses mots restent cités tels
   quels d'un étage à l'autre ; ce que l'archiviste en déduit est marqué.
4. **Le L3 se refait à partir des L2, pas à partir de l'ancien L3 seul**, pour
   qu'une erreur de résumé ne se recopie pas indéfiniment ; l'ancien L3 sert
   de point de départ, les L2 de preuve.
5. **Un sujet qui se divise** : quand un L3 devient lui-même trop gros ou
   couvre deux choses, l'archiviste propose de le scinder — comme pour la
   création d'un sujet, il propose, il ne tranche pas seul.

Encore flou :

- Ce que devient un L1 touchant **plusieurs sujets** : rangé sous chacun, ou
  découpé par sujet avant d'être rangé.
- « 100 % du contexte » quand une session s'arrête avant : la fermeture de la
  session vaut 100 %.
- Le budget qui déclenche le L3, et s'il dépend du sujet.

### 9.4 Des résumés structurés, et des genres de référence qui s'apprennent (Lucie, 4 octobre)

> « Les résumés sont structurés quand même : on extrait en plus des infos —
> nom de fichier mentionné, site web, des références abstraites avec des
> règles de format. Si ce type de référence n'existe pas encore, on demande à
> l'archiviste de créer la règle, en la pensant générique pour ce type de
> référence, pour les usages futurs. »

Un résumé n'est pas qu'un texte : c'est un texte **et la liste de ce qu'il
cite**. Chaque chose citée est une référence d'un **genre**, écrite dans une
forme réglée. Ce sont les adresses du §3 : un L1 qui cite `docs/users.md`, la
PR 42 et l'entité `User` porte trois liens, et se retrouve depuis chacun des
trois.

**Ce que le résumé structuré apporte.**

- Le rappel par ce qu'on touche devient exact : l'agent lit un fichier, et
  les L1, L2 et L3 qui le citent se trouvent par l'adresse, sans recherche
  par sens.
- Le « à revoir » monte les étages : un fichier change, les articles qui le
  citent le savent.
- Deux articles qui citent les mêmes choses sont proches, même s'ils n'en
  parlent pas avec les mêmes mots : un signal de plus pour ranger.

**Un genre de référence est une règle déclarée**, pas du code :

| Partie de la règle | Exemple pour « fichier » | Exemple pour « commit » |
|---|---|---|
| son nom et ce qu'il désigne | un chemin dans l'espace de travail | une révision d'un dépôt |
| comment on le reconnaît | un chemin relatif avec une extension | 7 à 40 chiffres hexadécimaux, près de « commit » |
| sa forme normale | `path:docs/users.md` | `commit:<dépôt>@<sha>` |
| comment on le vérifie | le fichier existe | la révision existe |
| ce qui le fait vieillir | l'empreinte du contenu | rien, il est immuable |

Les genres de départ : fichier, adresse web, ligne d'une entité, élément du
schéma, graphe, gabarit, sujet, commit, ticket.

**Quand un genre manque, l'archiviste le crée — pour tous les usages à venir.**
Il rencontre une chose citée qu'aucune règle ne reconnaît (un numéro de
facture, une référence d'article, un identifiant de carte) : il écrit la
règle, générique, et elle rejoint le registre. C'est la boucle du §4 : les
genres de référence sont eux-mêmes des fiches d'une mémoire, avec leur
gabarit, et le prochain résumé s'en sert.

**La forme exacte, dite par Lucie** : « ça se rajoute à un enum global de
types de référence ; nous on en fait une dizaine intégrés — document local,
fichier de code local, méthode ou symbole de code, lien web… — et le
structuré est contraint dessus, ou bien choisit `new` comme valeur, et dans
ce cas doit renseigner le champ optionnel `newRefType`, avec description et
règle. »

```json
{
  "summary": "…",
  "refs": [
    { "type": "code_file",  "value": "src/catalog.rs" },
    { "type": "web_link",   "value": "https://…" },
    { "type": "new",        "value": "FAC-2026-0412",
      "newRefType": { "name": "invoice",
                      "description": "un numéro de facture",
                      "rule": "FAC-<année>-<4 chiffres>" } }
  ]
}
```

**Des outils plutôt qu'une sortie structurée** (Lucie, dans la foulée) :
« je dis structuré, mais je pense mieux des outils : il essaie avec tel enum,
il se rend compte que ça n'existe pas encore — normalement il voit direct
qu'il n'a pas la valeur dans l'enum —, et donc il appelle un outil
`create_ref_type`. »

La forme ci-dessus devient donc deux outils, et le champ `new` disparaît :

- **`add_ref(type, value)`** — `type` est l'enum, dans le schéma de l'outil :
  l'archiviste voit la liste avant d'appeler. Un genre qui n'y est pas est
  refusé avec la liste et l'appel exact à faire : « ce genre n'existe pas ;
  crée-le par `create_ref_type(name, description, rule)`, puis rappelle
  `add_ref` ».
- **`create_ref_type(name, description, rule)`** — montre les genres proches
  (« `web_link` existe déjà : est-ce lui ? »), éprouve la règle sur la valeur
  qui l'a fait naître, puis l'ajoute à l'enum. L'appel suivant d'`add_ref`
  porte le genre neuf dans son schéma.

Pourquoi c'est mieux qu'un seul objet structuré : chaque geste a son refus,
qui dit quoi faire — la forme que les passes d'agent ont montrée suivie, même
par un modèle faible ; la création d'un genre est un acte à part, visible au
journal, et pas un champ optionnel qu'on remplit en passant ; et l'enum est
relu à chaque appel, donc un genre créé à l'instant sert tout de suite. Le
résumé lui-même reste un texte ; ses références s'y ajoutent une à une.

Ce qui reste vrai de la première forme :

- **Un enum global**, que la sortie structurée du modèle est **contrainte** à
  respecter : il ne peut pas inventer un genre en l'écrivant de travers. Le
  moteur sait déjà faire d'une liste fermée un `enum` de schéma, refusé avec
  la liste quand la valeur n'y est pas.
- **Une dizaine de genres intégrés** : document local, fichier de code local,
  méthode ou symbole de code, lien web, ligne d'une entité, élément du
  schéma, graphe, gabarit, sujet, commit, ticket.
- **`new` est la seule porte de sortie**, et elle se paie : qui choisit `new`
  doit remplir `newRefType` (nom, description, règle). Le schéma l'exige — un
  `new` sans sa règle est refusé.
- **L'enum grandit** : un `newRefType` admis (points 1 et 2 ci-dessous)
  entre dans l'enum, et le résumé suivant le voit parmi ses choix. En
  attendant, la référence garde son genre proposé : elle n'est pas perdue.
- **La liste doit rester lisible par un modèle** : quand elle dépasse ce
  qu'un schéma porte bien, on ne montre que les intégrés et les genres déjà
  vus dans le sujet ou la mémoire en cours — le reste se retrouve par la
  recherche avant création.

**La règle porte un script** (Lucie) : « peut-être que la règle doit contenir
un script rhai pour se valider, qui sert de crochet aux prochains ».

La règle d'un genre n'est donc pas une phrase qu'un modèle interprète à
chaque fois : c'est un petit script, écrit une fois à la création du genre,
et rejoué ensuite **sans modèle** à chaque `add_ref` de ce genre.

```rhai
// genre « invoice » : FAC-<année>-<4 chiffres>
fn validate(value) {
    let v = value.trim().to_upper();
    if !v.starts_with("FAC-") || v.len() != 13 { return #{ ok: false, why: "attendu FAC-AAAA-NNNN" }; }
    #{ ok: true, normal: "invoice:" + v }
}
```

- Le moteur a déjà la pièce : rhai est embarqué, `RhaiNode` et les nœuds de
  validation existent, et un backend déclare ses scripts au manifeste. Un
  genre de référence est un script de plus dans ce registre.
- Le script fait **trois choses au plus**, chacune une fonction facultative :
  reconnaître et normaliser (`validate`) ; dire si la chose existe encore
  (`resolve`) ; dire ce qui la fait vieillir (`fingerprint`). Le crochet des
  « prochains » est là : tout `add_ref` du genre, et plus tard l'extraction
  déterministe dans un texte, passent par lui.
- `create_ref_type` **éprouve le script avant de l'admettre** : il doit
  accepter la valeur qui l'a fait naître, refuser des contre-exemples, et ne
  pas accepter ce qu'un genre existant accepte déjà.

**La description toujours, les harnais au mieux** (Lucie) : « c'est les deux :
description de règle, et script si possible, ou même schéma — c'est encore
flou. L'agent a consigne de renseigner au mieux des harnais pour son type de
référence. »

Un genre n'exige donc pas un script pour exister. Il a une description, qui
est obligatoire, et autant de **harnais** que son créateur sait en donner :

| Niveau | Ce que le genre porte | Qui vérifie une référence |
|---|---|---|
| 0 | une description en clair, avec deux ou trois exemples | un modèle, en lisant la description — coûteux, variable |
| 1 | un **schéma** : la forme de la valeur (champs, motif, bornes) | le moteur, sans modèle |
| 2 | un **script** : normaliser, résoudre, dire ce qui fait vieillir | le moteur, sans modèle |

- La consigne donnée à l'archiviste : monter aussi haut qu'il le peut
  honnêtement. Un numéro de facture se met en schéma ; « le design d'hier »
  ne se met pas en script, et ce n'est pas une faute.
- Un genre peut **monter d'un niveau plus tard** : quand assez de références
  l'ont utilisé, le jardinier (ou l'archiviste neutre) propose le schéma ou
  le script qu'elles permettent d'écrire, éprouvé sur elles.
- Le niveau se voit : un genre de niveau 0 est marqué comme tel, et ses
  références sont dites « non vérifiées » plutôt que tenues pour sûres.
- Ce qui reste flou, et qui se décidera en essayant : si le schéma et le
  script sont deux harnais ou un seul (un script peut tout faire, un schéma
  se lit mieux) ; et ce qu'on demande au minimum pour qu'un genre entre dans
  l'enum.

Un script écrit par un modèle et rejoué pour tous est une surface à tenir :

1. **Pur par défaut** : rien que des chaînes en entrée et une table en
   sortie. Pas de fichier, pas de réseau, pas de commande. `resolve` ne voit
   le monde que par des fonctions que l'hôte lui prête, bornées à l'espace de
   travail et au catalogue, en lecture.
2. **Borné** : un plafond d'opérations, de profondeur et de taille de chaîne
   par appel (le moteur rhai sait le faire) ; un script qui dépasse échoue,
   et la référence reste du texte.
3. **Une erreur du script n'est jamais un refus muet** : elle se dit, et le
   genre est marqué en défaut.
4. **Versionné** : un script corrigé remplace l'ancien sans réécrire les
   références déjà validées ; on peut les repasser.

Ce qu'il faut tenir, parce qu'une règle créée à la volée par un modèle est
exactement l'endroit où le désordre entre :

1. **Chercher avant de créer** : montrer les genres proches et demander
   lequel on retient — la même règle que pour un sujet. « Référence
   d'article » et « DOI » ne doivent pas devenir deux genres.
2. **Une règle s'éprouve avant d'entrer** : sur les exemples qui l'ont fait
   naître, et sur des contre-exemples — elle ne doit pas attraper ce que les
   genres existants attrapent déjà. Une règle qui échoue reste une
   proposition.
3. **L'extraction est déterministe d'abord** : ce qu'une règle sait
   reconnaître seule (un chemin, une adresse, une révision) n'a pas besoin de
   modèle ; le modèle ne sert qu'aux références dites en langue (« le design
   d'hier », « la PR sur les users ») et à la création d'un genre.
4. **Une référence non résolue se dit** : « cité, introuvable » est une
   information ; elle n'est ni jetée ni inventée.
5. **Un genre créé est daté, attribué, et se retire** : ses références
   redeviennent du texte, rien n'est perdu.

## 8. Ce qui attend un choix de Lucie

1. L'ordre du §7, ou la boucle (le registre) d'abord.
2. Qui peut concevoir un gabarit : tout agent, ou seulement avec l'accord de
   la personne.
3. Une mémoire par base, ou toutes les mémoires d'une personne dans une même
   base (ce qui rend les liens entre mémoires directs).
