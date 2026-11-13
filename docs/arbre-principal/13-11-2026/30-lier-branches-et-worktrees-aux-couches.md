# Lier les branches et les worktrees aux couches (le pas des vues après 1a et 1b)

13 novembre 2026, arbre principal, à la demande de c7. Page de conception : aucun code avant le mot de
Lucie. Relue par recherche le 13 novembre (ses points sont reportés et attribués) ; ED à venir. Elle s'appuie sur la conception du 8 novembre
(`extension/rag3weaver/docs/08-novembre-2026-arbre/01-conception-des-vues-marche-1.md`, §§ 2 à 5),
sur le résolveur de couches 2a (`src/dataflow/migrations/publication/layers.rs`) et sur la décision
de Lucie du 13 novembre : les vues se bâtissent sur l'index de code réel, sans second index.

## Ce qu'il faut relier

Le résolveur 2a sait composer une pile de couches épinglées. Pour chaque occurrence, la couche la
plus proche gagne, et un tombstone masque le dessous. Mais une couche n'y est qu'un nom libre, plus
une publication tirée au hasard : rien ne la relie à Git. Cette page dit comment une branche, un
worktree et un commit deviennent des couches et des générations.

## 1. Une couche par branche, une ancre par dépôt

- **L'ancre** est la branche principale du dépôt (`master` ou `main`), indexée en entier une fois.
  C'est la couche du bas.
- **Une couche = une branche du dépôt**, nommée `<dépôt>:<branche>`. Elle ne porte que les
  occurrences qu'elle change par rapport à sa parente, plus ses tombstones (scopes et fichiers
  retirés).
- **Sa parente** est la couche d'où elle est née, l'ancre ou une autre branche (la pyramide de
  Lucie). Git ne garde pas cette information : il faut la noter quelque part.
  - Une tâche ouverte par l'atelier la note à sa création : `ouvrir_une_tache` connaît déjà sa
    branche de départ et son commit (`depart`).
  - Pour une branche venue d'ailleurs, je propose une inférence prudente : parmi les couches
    connues, celle dont la pointe est l'ancêtre le plus proche du point de fourche. À défaut,
    l'ancre. L'inférence est notée comme telle, jamais confondue avec une déclaration. **Question 1.**
- **Sa base** est le commit de la fourche. La couche se définit contre sa base, pas contre la pointe
  du moment de sa parente (la mesure du 7 novembre : 0 à 8 chemins contre la base, 5 à 53 contre le
  master du moment).

## 2. Une génération par commit indexé

- **Une génération est l'état publié d'une couche pour un commit** : `(couche, commit)`, plus un rang
  si le même commit est réindexé, par exemple après un changement de recette. L'identité vient du
  commit, pas d'un tirage au hasard : deux processus qui indexent le même commit parlent de la
  même génération. Le registre de Codex (chaîne de publications, refus d'une tête qui a bougé)
  s'adopte pour la publication. Ses identifiants tirés au hasard ne s'adoptent pas.
- **Quand une génération se publie.** Je propose : quand l'index a fini un commit, c'est-à-dire
  quand la montre voit `HEAD` bouger et que les versions neuves sont écrites. La génération
  précédente reste lisible jusqu'à la fin des lectures en cours (épinglage L6). **Question 2** :
  à chaque commit, ou seulement à la demande, au push, ou à la fin d'une tâche ?
- **La fusion** d'une branche dans sa parente publie une génération de la parente au commit de
  fusion. Les versions de la couche y descendent sans réembarquement, ses tombstones s'appliquent,
  et la couche se ferme (§ 5 de la conception du 8 novembre).
- **Un rebase** déplace la base : c'est une génération neuve de la couche sur une autre génération
  de sa parente, publiée explicitement. Jamais un glissement silencieux. **Question 4.**

## 3. La couche de travail : le non-commité d'un worktree

- **Attacher un worktree, c'est monter la couche de sa branche**, plus une **couche de travail** :
  ce qui est modifié et pas encore commité.
- **La couche de travail n'est pas une génération.** Elle est vivante : la montre la réécrit à chaque
  édition, elle n'a pas d'identité stable, elle ne se publie pas. Elle appartient au worktree.
  - Au commit, ses occurrences entrent dans la génération neuve de la branche, et elle se vide.
  - À l'effacement du worktree, elle part.
- **Question 3** : la couche de travail d'un worktree est-elle visible des autres sessions (un agent
  qui lit le worktree d'un autre), ou seulement de celle qui le tient ?

## 4. Ce qu'on lit, et ce que l'IDE montre

- **La vue courante se choisit par le contexte** (Lucie, 11 novembre) :
  - une session attachée à un worktree lit sa branche plus sa couche de travail ;
  - sinon, l'ancre.
  - Les outils prennent un paramètre `view` pour changer de vue ; une vue inconnue est refusée avant
    toute recherche.
- **Une version ancienne se demande explicitement** : `view: <branche>@<commit>`. Si ce commit a une
  génération, elle se lit sans rien indexer. Sinon, on matérialise ce seul état, sans indexer les
  commits intermédiaires.
- **Le coût d'une version ancienne** (recherche, relecture du 13 novembre) :
  - Avec 1a, ce n'est pas un index entier. Un vieux commit retombe surtout sur des versions déjà
    embarquées ; seules les versions jamais vues coûtent du dense. L'analyse se fait en delta contre
    l'ancêtre le plus proche qui a une génération, par `git show`, sans checkout.
  - **Le texte d'abord** : la réponse sert le plein texte tout de suite, la dette dense se paie hors
    du verrou, et la réponse le dit (« dense partiel : N versions en cours »). Une lecture n'attend
    jamais l'embarqueur.
  - **Une seule matérialisation par (couche, commit)**, sous un verrou par génération : deux lectures
    simultanées se partagent le travail.
  - **Ces générations de lecture sont jetables**, distinctes de celles qu'un commit publie, et
    évincées en premier (un cas de plus pour la question 5).
- **Lire une génération jusqu'au bout pendant qu'un commit publie la suivante** (recherche) :
  l'épinglage L6 ne suffit pas, car lucivy, et peut-être l'index dense, vivent hors de la MVCC de
  rag3db. L'instantané composé reste petit :
  1. un seul searcher lucivy pour toute la lecture, pris au début, jamais rechargé au milieu ;
  2. chaque résultat, plein texte ou dense, se résout par l'instantané épinglé : de la version à
     l'occurrence visible dans la génération lue. Un résultat d'une version neuve n'y a pas
     d'occurrence et tombe ;
  3. aucun effacement physique d'une version qu'une lecture peut voir : le ramasse-miettes (M5)
     attend les lectures épinglées, sous un bail ;
  4. le masque avant le top-k (2b-B), ou un sur-tirage, faute de quoi le rappel baisse en silence.
  - À vérifier avec le cœur avant de l'écrire comme acquis : l'index HNSW suit-il l'instantané de la
    transaction de lecture ? À défaut, le point 2 s'applique aussi au dense.
- **Les trois lunettes de l'IDE** (« voir en tant que… ») :
  - **worktree** : la branche plus la couche de travail, ce que l'agent voit ;
  - **branche** : la dernière génération publiée de la branche, sans le non-commité ;
  - **pyramide** : la même vue, chaque scope marqué de sa couche (de la vue, de la parente, de
    l'ancre). On remonte la pile scope par scope, comme Lucie l'a décrite le 8 novembre.
- **Deux ajouts de recherche** :
  - la lunette worktree marque le non-commité à part, puisqu'il n'a pas de génération, et dit ce
    qu'il n'a pas encore : une édition pas encore embarquée, trouvable par mots seulement (la dette,
    montrée scope par scope) ;
  - la pyramide montre, à la demande, ce qu'un tombstone masque (le scope de la parente ou de
    l'ancre retiré par la branche). Sinon, « pourquoi je ne vois plus `f` » n'a pas de réponse.
- **La provenance** reste à la demande pour un agent (`provenance: true`), sans coût sinon. L'IDE la
  montre toujours.

## 5. Ce qui s'appuie sur 1a et 1b

- **Sur 1a** (la version de scope, en cours) : une couche ne porte que des références d'occurrences
  vers des versions partagées. Ouvrir une couche ne coûte que les versions neuves, embarquées une
  fois (12 à 151 par branche au lieu de 88 à 1 278 scopes).
- **Sur 1b** (la migration, à son mot) : une base de la forme d'avant n'a pas de versions, donc pas
  de couches. Tant qu'elle n'est pas migrée, ses vues sont indisponibles, et elle le dit (« à
  migrer ») au lieu de servir une vue fausse.
- **Le pas suivant** dépend de celui-ci : le chargeur durable des couches (2b-B), le masque avant le
  top-k dans les signaux réels, puis le paramètre `view` des outils (2b-C). Le témoin cruel (aucune
  réponse ne mélange deux générations) se joue dès qu'une vue est interrogeable.

## Les questions pour Lucie

1. **La parente d'une branche venue d'ailleurs** : l'inférer par le point de fourche (notée comme
   inférée), ou exiger qu'elle soit déclarée ?
2. **Quand publier une génération** : à chaque commit indexé, ou seulement à la demande, au push,
   ou à la fin d'une tâche ?
3. **La couche de travail** : privée à la session qui tient le worktree, ou lisible des autres ?
4. **Un rebase** : la couche suit d'elle-même (génération neuve publiée par l'hôte), ou on attend un
   geste ?
5. **L'histoire** : combien de générations passées garder par couche (toutes, les N dernières, ou
   celles qu'une fiche cite) ? Et des points de reprise complets avec des deltas, comme tu l'as
   demandé le 11 novembre ?

## Les témoins, quand le code viendra (rouges d'abord)

- Une tâche ouverte depuis une branche note sa parente et sa base. Sa vue rend `f` de la parente, `g`
  d'elle, `h` de l'ancre.
- Un commit dans un worktree publie une génération. Une lecture commencée avant voit l'ancienne
  jusqu'au bout.
- Une édition non commitée se voit dans la vue du worktree, pas dans celle de la branche.
- La fusion fait descendre les versions sans réembarquement et ferme la couche.
- **Le témoin cruel** (recherche) : une génération se publie pendant une lecture tenue, dans les trois
  signaux à la fois, et aucune réponse ne mélange deux générations.
- `view: <branche>@<commit>` d'un commit jamais indexé matérialise ce seul état.
