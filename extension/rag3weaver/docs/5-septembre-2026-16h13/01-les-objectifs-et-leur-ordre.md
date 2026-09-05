# Les objectifs, et leur ordre

**5 septembre 2026.** Écrit après la relecture de la série vision
([`vision_roadmap_09_2026`](../vision_roadmap_09_2026/00-index.md)), qui a dit
où on en est. Celui-ci dit ce qu'on fait ensuite, et **pourquoi dans cet
ordre** — c'est l'ordre qui a été discuté, pas la liste.

## L'ordre, et le renversement qui le décide

J'avais proposé la lecture des documents en premier. Lucie a inversé :

> les écrivains multi-processus d'abord — fiabiliser la faisabilité d'une
> solution cloud avant de faire cinquante passes de features.

**Elle a raison, et la raison est structurelle.** La lecture des documents est
**additive** : elle ajoute de la matière à un moteur qui marche. Les écrivains
multi-processus sont **porteurs** : c'est une hypothèse d'architecture. Si elle
ne tient pas, tout ce qu'on empile au-dessus est à reprendre — et cinquante
passes de features, c'est exactement l'empilement qui rend une reprise
impossible.

C'est le même principe qui a fait passer le second backend devant le reste :
*prouver ce qui porte avant d'ajouter ce qui pèse*. Je l'avais appliqué aux
backends et sous-appliqué ici.

## 1. La vraie concurrence, **sur rag3db**

### Ce que « vraie » veut dire, en trois clauses

Lucie, mot pour mot :

> deux choses qui sont ok d'écrire en même temps le sont, et les opérations ne
> sont pas mises en queue alors que les clients pensent que c'est fini — c'est
> fait à la demande sur le coup, malgré un truc qui arbitre « tiens vous écrivez
> la même ressource, attendez, je drive ça l'un après l'autre ».
>
> et je le veux pour rag3db, c'est ça le challenge.

Trois exigences, et aucune n'est celle que j'avais notée :

1. **Le parallélisme est réel, pas simulé.** Deux écritures qui ne se gênent pas
   se font **en même temps**. Pas de sérialisation globale.
2. **Un acquittement veut dire fait.** Aucune file qui rend la main pendant que
   le client croit l'opération terminée.
3. **L'arbitrage est à la ressource, pas à la base.** Le conflit se détecte là où
   il est — même ressource — et se règle en faisant attendre, pas en faisant
   attendre tout le monde.

**Et c'est sur rag3db**, la base embarquée. Ma note précédente proposait de
regarder « sur quel backend tourne le cloud » et concluait que PostgreSQL
réglerait la moitié du sujet : **c'est à côté de la question.** Contourner le mur
n'est pas l'abattre, et c'est l'abattre qui est l'objectif.

### La clause 2 est déjà violée, et c'est chez nous

Avant tout travail sur le cœur C++, il faut voir que **notre propre contrat
d'écriture ment aujourd'hui** :

| appel | ce qu'il fait | honnête ? |
|---|---|---|
| `ingest_entities` | exécute son graphe et rend un `FlushResult` | **oui** — quand il rend, c'est écrit |
| `create` / `update` / `delete` | poussent dans `Catalog::pending` **en mémoire** et rendent | **non** — le client croit que c'est fait |
| `drain()` | fait le travail | — |

`create` rend un `EntityRef` après une mise en file. Rien n'est en base. Le
défaut est de la famille qu'on passe nos journées à sortir : **ce n'est pas une
erreur, c'est un silence** — et il a déjà produit sa conséquence, la marque
d'eau d'ingestion, qui existe précisément parce qu'un lecteur d'un autre
processus ne pouvait pas savoir qu'une file invisible existait.

**C'est réparable indépendamment du C++, et ça doit l'être en premier** : ça ne
demande rien au moteur de stockage, et ça rend la suite mesurable. Un
acquittement qui ment fausserait toute mesure de concurrence qu'on ferait
ensuite.

### Le faux dilemme, et la sortie — **deux niveaux de disponibilité**

J'avais posé le choix comme « honnêteté **contre** débit » : rendre `create`
synchrone tuerait l'ingestion, donc il faudrait un reçu. Lucie a écarté le
dilemme en observant qu'il repose sur une confusion :

> pour une ingestion c'est compréhensible que ce soit en arrière-plan, mais pour
> un truc genre « tiens, mon consumer a acheté un produit aujourd'hui », bof.
>
> les gens devraient pouvoir se dire « j'ai besoin juste des données, pas de la
> recherche pour cette requête, le moteur est donc prêt » ou « j'ai besoin de la
> recherche, est-ce que le moteur est prêt, j'attends jusqu'à ce que… » — et le
> « j'attends que ce soit prêt » devient un forçage synchrone si on veut
> vraiment.

**Les deux coûts ne sont pas dans la même partie de l'écriture.** Poser une
ligne est bon marché ; la rendre *trouvable* — chunker, embarquer, indexer — est
cher, et c'est ça qui a besoin de lots. Les séparer ne sacrifie donc rien : ça
récupère le groupage là où il compte et le supprime là où il ne servait à rien.

| niveau | ce qui est vrai | coût | qui en a besoin |
|---|---|---|---|
| **donnée** | la ligne est en base, lisible, cohérente | faible | « mon client a acheté un produit » |
| **recherchable** | les index dérivés sont à jour — plein texte, vecteur, sparse | élevé, par lots | « trouve-moi les clients qui… » |

Trois conséquences :

- **Le niveau « donnée » devient synchrone par défaut.** C'est le cas que Lucie
  nomme, et c'est celui où la file est indéfendable : un achat qu'on acquitte
  sans l'écrire est un bug, pas une optimisation.
- **Le niveau « recherchable » reste en arrière-plan**, légitimement, et
  l'appelant peut **attendre** qu'il soit atteint. C'est ce qu'elle appelle le
  forçage synchrone : non pas un mode, mais une **attente choisie**.
- **Le contrat cesse de mentir sans rien coûter au débit.** Ce qui était un
  compromis devient une distinction.

### La symétrie qu'on n'avait pas vue

C'est **le même axe que `Consistency`, par l'autre bout.**

| | qui parle | ce qu'il dit |
|---|---|---|
| écriture | l'écrivain | *ce qu'il a rendu prêt* |
| lecture (`Consistency`) | le lecteur | *ce dont il a besoin d'être prêt* |

Deux notions bâties séparément, à des semaines d'intervalle, qui posent la même
question. Et la **marque d'eau** est ce qui les relie à travers la frontière du
processus — sauf qu'aujourd'hui elle est **binaire** : « cet écrivain a du
travail non publié », sans dire lequel.

Elle devrait publier **par niveau** : les lignes sont posées jusqu'à ici, les
index sont bâtis jusque-là. Un lecteur qui ne veut que la donnée n'attendrait
alors pas l'indexation — ce qu'il fait aujourd'hui, et qui est du gaspillage
autant que du mensonge.

**Et la granularité y revient, exactement comme dans la clause 3.** Deux entités
peuvent être à deux niveaux différents : l'une posée et indexée, l'autre posée
seulement. Une marque globale répondrait « pas prêt » pour tout le monde à cause
d'une seule. La même idée que l'arbitrage à la ressource plutôt qu'à la base, et
c'est la deuxième fois qu'elle sort la même journée — c'est probablement qu'elle
est structurelle.

### Quatre disponibilités, pas deux — et ce ne sont pas des niveaux

Lucie, en corrigeant mon « commencer à deux » :

> on peut autant à la lecture qu'à l'écriture attendre « data / textsearch /
> sparse / dense ».

C'est la bonne forme, et elle n'est pas un **niveau** mais un **ensemble** :
`data` précède tout — on n'indexe pas ce qui n'est pas posé — mais les trois
index sont des **frères parallèles**, pas des étages. Ils se commitent
séparément ; c'était la question que je laissais ouverte, et elle est tranchée
par la structure plutôt que par un cas d'usage.

Conséquence sur la surface : on n'attend pas « jusqu'au niveau N », on attend
**les signaux qu'on nomme**. Qui ne veut que du plein texte n'attend pas
l'embarquement GPU.

### L'invariant qui ordonne tout le chantier

> si les gens font des trucs lourdingues c'est leur responsabilité, mais nous on
> optimise pour que **jamais deux ressources qui ne sont pas en lien ne soient
> bloquées l'une par l'autre**.

Cette phrase n'est pas une exigence de plus : **c'est celle dont les autres
découlent**. Elle trace aussi le partage des responsabilités, ce qui est rare et
utile — une requête coûteuse est le problème de qui l'écrit ; un **couplage
faux** est le nôtre.

Elle explique les trois clauses d'un coup, et c'est la même idée que la
granularité de l'arbitrage et celle de la disponibilité. Trois apparitions par
des chemins indépendants dans la même journée : **la ressource, et non la base,
est l'unité de tout ce chantier.**

### Ce que ça condamne, et ce que ça ne condamne pas

Le couplage global existe aujourd'hui, et il est facile à nommer :
`Catalog::pending` est **une** file, `drain()` **une** barrière. Une recherche
sur l'entité A peut attendre des écritures en attente sur l'entité B, qui n'a
rien à voir — c'est le couplage faux, dans le chemin de lecture, aujourd'hui.

**Mais `drain` n'est pas le tort, et il ne faut pas le retirer.** Lucie :

> je pense que drain a son usage quand même dans certains cas, faut pas non plus
> enlever ce mode de fonctionnement, mais permettre un plus ou moins au cas par
> cas — « au tick » ? — et un autre avec drain quand on sait très volontairement
> vouloir faire une grosse opération d'ingestion.

Le lot n'est un tort **que lorsqu'il n'a pas été choisi**. Une grosse ingestion
veut grouper : c'est ce qui la rend possible. Deux régimes, donc, déclarés :

| régime | ce qu'il fait | pour qui |
|---|---|---|
| **au tick** | chaque écriture passe à sa disponibilité déclarée, arbitrée **par ressource** | « mon client a acheté un produit » |
| **par lot** | on accumule volontairement, on vide quand on le dit | une ingestion massive |

**Et la forme existe déjà à moitié.** `ingest_entities` *est* le verbe de lot, et
il est honnête : il exécute et rend quand c'est fait. Le défaut n'est donc pas
qu'il y ait un lot, c'est que **le verbe par item se comporte comme le verbe de
lot, en silence** — `create` alimente une file globale sans le dire, et seul
`drain` la vide.

C'est la troisième fois qu'on retrouve cette forme après `MoteurTexte` et
`Acces` : **une option, pas un remplacement**, avec un défaut qui s'explique. Et
la même règle qu'ailleurs — le régime se déclare, il ne se devine pas, et le
choix se **dit** dans ce que l'appel rend.

### Ce que ça coûte, et ce qui reste ouvert

- Une **API qui change** : les écritures prennent un niveau, ou rendent de quoi
  attendre. C'est un vrai changement de surface, pas un ajout.
- ~~« Recherchable » est-il une seule chose ?~~ **Tranché** : non. Quatre
  disponibilités — `data`, `textsearch`, `sparse`, `dense` — dont trois
  parallèles. Voir plus haut.
- **Où vit l'arbitrage du régime « au tick » ?** Faire passer chaque écriture
  sans coupler les ressources demande de savoir quelles ressources une écriture
  touche. C'est trivial pour une entité, moins pour ses chunks, ses relations et
  les lignes d'index KB qu'elle alimente — une écriture sur une entité en touche
  plusieurs par ricochet. **C'est là que le sujet est vraiment difficile**, et
  c'est le même problème que la granularité des verrous : la réponse ne peut pas
  être différente des deux côtés.
- L'idée vaut **au-delà de l'écriture**, et c'est Lucie qui le note : un agent
  aussi devrait pouvoir dire « je n'ai pas besoin de la recherche pour cette
  requête ». Une requête déclare la disponibilité qu'elle exige — c'est déjà ce
  que `Consistency` fait, en plus grossier.

### Ce que la clause 1 et la clause 3 demandent au cœur

Là, c'est le C++, et c'est le gros morceau. Aujourd'hui un second processus **ne
peut pas ouvrir** la base en écriture : `LocalFileSystem::openFile` pose un
`F_WRLCK` en `F_SETLK`, refus immédiat ; et `TransactionManager::beginTransaction`
refuse une seconde transaction d'écriture — le seul réglage qui le relâche
s'appelle `debug_enable_multi_writes`, ce qui dit son statut.

Trois pièces, dans cet ordre :

1. **Relire le MVCC de Vela.** C'est le plancher, et personne ne l'a fait :
   rotation de WAL, points de reprise non bloquants. C'est lui qui décide si on
   bâtit dessus ou si on le remplace, et on ne pose pas d'arbitrage fin sur une
   couche qui suppose un écrivain unique. **Préalable, pas première tâche.**
2. **Un gestionnaire de verrous inter-processus.** Le verrou de fichier actuel
   rend l'accès *impossible*, pas *ordonné* — c'est déjà ce qu'on a écrit à
   propos de `Consistency`. Il faut un arbitre qui vive hors des processus.
3. **La granularité.** C'est elle qui distingue la clause 1 de la clause 3 :
   sans granularité fine, tout arbitrage dégénère en verrou global, et « deux
   choses qui peuvent s'écrire en même temps » ne s'écrivent jamais en même
   temps. C'est ce que le MVCC donne, et c'est pour ça que le point 1 le précède.

### L'ordre, et ce qui est mesurable tout de suite

| | qui | dépend de |
|---|---|---|
| **a.** L'acquittement cesse de mentir | Rust, ici | rien |
| **b.** Relire le MVCC de Vela | cœur C++ | rien |
| **c.** Verrous inter-processus | cœur C++ | b |
| **d.** Arbitrage à la ressource | cœur C++ + Rust | b, c |
| **e.** Mesurer : deux écrivains disjoints en parallèle, deux écrivains en conflit sérialisés | ici | a, d |

**a** et **b** peuvent partir en même temps, sur deux terrains disjoints.

### Ce qui est déjà là et qui servira

- La **marque d'eau d'ingestion** est écrite pour **plusieurs** écrivains — une
  marque par écrivain, avec péremption pour qu'un processus mort ne gèle
  personne. Elle n'a été **éprouvée qu'à un seul**.
- La **lecture concurrente** est acquise et mesurée : 80 ouvertures pendant
  qu'on écrit, zéro refus.
- Le **choix de chemin d'un lecteur** — direct ou par le relais — existe déjà,
  avec la règle qui empêchera la même souplesse de devenir un piège côté
  écriture : jamais de repli silencieux.

## 2. La lecture des documents

pdf, docx, pptx, html, csv. Sans bibliothèque lourde : les formats Office sont
du ZIP + XML, et l'OCR livré couvre déjà les PDF scannés.

C'est le plus vieux point non tenu de la feuille de route, et le seul dont
l'absence rend inutile une bonne part du reste : on a rendu la recherche juste,
mesurée, cloisonnée et portable **sur du contenu qu'on ne sait pas ingérer**.

Additif, donc second — mais premier parmi les additifs.

## 3. `Catalog::search` devient un gabarit

Pas pour l'élégance. **La dette a un prix mesuré** : 409 lignes dans un
`catalog.rs` de 6 373, et le chemin composable existe en parallèle sans être
emprunté. Cette semaine, chaque correction de recherche a dû être portée aux
**deux** chemins — le filtre utilisateur, la marque de confiance, le dialecte
pour la résolution des chunks. L'une l'a été à moitié, et ça s'est trouvé par
accident.

Le bon moment n'est pas « maintenant » dans l'absolu : c'est **la prochaine fois
qu'on rouvre la recherche**. Le noter pour que ce moment ne passe pas inaperçu.

## 4. Le graphe de normalisation des tableurs

La porte d'entrée de toute la moitié KB. Un tableur est la forme sous laquelle
un catalogue arrive, neuf fois sur dix.

La spécification de février existe, et la pratique la corrige sur quatre points
([03](../vision_roadmap_09_2026/03-normaliser-des-tableurs.md) §9) : phases
persistées, deux représentations, renversement d'échelle, et une décision
explicite sur l'identité des lignes. C'est du travail **conçu**, pas à
concevoir.

## 5. Avaler une base étrangère

Le [13](../vision_roadmap_09_2026/13-avaler-une-base-existante.md) attendait
qu'un backend SQL existe pour cesser d'être une idée. Il existe.

**Après les tableurs**, et c'est une décision de Lucie déjà prise : la question
« quelles colonnes portent du texte cherchable » se rencontre d'abord là, sur un
terrain plus simple.

## Les petits, à faire au passage

Trois choses qui ne méritent pas leur journée mais qui mordront :

- **La troncature d'embedding.** Le dernier silence pur qui reste après une
  semaine passée à les supprimer : un chunker qui ignore la fenêtre du modèle
  tronque sans un mot. La cause est nommée dans le code — le trait `Embedder`
  n'expose ni fenêtre ni compte de jetons, le budget est en caractères parce que
  le tokenizer vit ailleurs. **Un avertissement avant une correction.**
- **`crate::acces` n'a aucun appelant en production.** Écrit hier, éprouvé par
  des tests, et rien d'autre ne s'en sert : c'est le « construit et jamais
  appelé » qu'on vient d'inscrire comme corollaire d'invariant dans le
  [01](../vision_roadmap_09_2026/01-la-vision.md) §4. Son appelant naturel
  manque aussi — rien ne permet d'ouvrir un catalogue **en lecture**.
- **Le miroir Rust de `search_base`.** Dernière duplication tenue à la main :
  70 lignes de test qui recopient `search_base.mmd` nœud par nœud. Le
  **générer serait pire** — il prouverait que l'analyseur est d'accord avec
  l'analyseur. Le bon geste est un fixture minimal découplé du graphe de
  production.

## Ce qui n'est pas dans cette liste, et pourquoi

- **La parole (TTS/STT/G2P)** — reportée avec l'**interface** du produit « agent
  de code ». Décision de Lucie, pas un oubli.
- **Neo4j** — en dernier, pas en deuxième. Il parle Cypher, donc il masquerait
  les fuites de représentation qu'un backend SQL révèle ; il restera bon marché
  précisément pour ça
  ([15](../vision_roadmap_09_2026/15-le-moteur-cesse-d-etre-mono-backend.md) §2).
- **Le renommage `kuzu` → `rag3db`** — 2 899 fichiers à chaque reprise de
  l'amont, pour un bénéfice que personne n'a énoncé. Tant qu'il ne l'est pas, ça
  reste une dépense récurrente sans contrepartie.
