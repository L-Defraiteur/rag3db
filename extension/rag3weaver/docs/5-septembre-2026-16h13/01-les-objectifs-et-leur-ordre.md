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

Mais il y a une tension à trancher, et elle est réelle : **la file existe pour
grouper**. Les écritures partent en `UNWIND`, les embarquements par lots GPU.
Rendre chaque `create` synchrone tuerait le débit d'ingestion. Deux sorties
possibles, et c'est un choix de conception, pas une évidence :

- **le lot est explicite** — `create` devient synchrone et lent, et qui veut du
  débit appelle `ingest_entities`, qui est déjà honnête ;
- **ou l'acquittement porte son état** — `create` rend un reçu qui dit « en
  attente », sur lequel on peut attendre. Le client ne croit plus rien à tort.

La seconde garde le débit et respecte la clause 2, au prix d'un type de plus
dans l'API.

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
