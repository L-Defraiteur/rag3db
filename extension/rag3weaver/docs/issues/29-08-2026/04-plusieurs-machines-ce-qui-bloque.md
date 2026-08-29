# Plusieurs machines : ce qui bloque, et ce que ça coûterait

**Ouvert le 29 août 2026**, après rag3daemon ([issue 03 §9](03-un-demon-et-la-fin-du-tout-synchrone.md)).

## 0. Une phrase à retirer

J'ai écrit que l'écrivain unique n'était pas notre goulot, « parce que le
nôtre est le GPU ». Lucie : *« ça veut pas dire que demain ce qui intéresse les
gens est d'ingérer des choses sans embedding »*. Exact. Une ingestion pure
graphe ou plein-texte, sans embarquement, et l'écrivain unique **est** le mur.
L'ordre des goulots dépend de l'usage, pas de celui qu'on a aujourd'hui.

## 1. Ce qui ne bloque plus : les clients

rag3daemon écoute sur une adresse. `--adresse 0.0.0.0:7979`, et n'importe quelle
machine lui parle : `DaemonConnection` est un `DbConnection` ordinaire. Dire
« Postgres le jour où plusieurs machines » était un réflexe, pas un constat.

**Mais pas encore** : voir [issue 05](05-rag3daemon-execute-du-cypher-pour-qui-atteint-le-port.md).
Le démon exécute du Cypher arbitraire pour quiconque atteint le port. C'est le
vrai préalable, avant toute question d'échelle.

## 2. Ce que le moteur partage déjà : la lecture

Une base ouverte en lecture seule pose un `F_RDLCK` — **partagé**. Mesuré
(`tests/e2e_prise_atomique.rs`) : deux processus lisent la même base en même
temps ✓ ; un lecteur est refusé pendant qu'un écrivain la tient ✓. Donc N
machines lectrices sur un montage commun, sur une base **figée**, ça marche
aujourd'hui. `Rag3dbConnection::read_only` l'expose depuis le 29 août.

## 3. La bonne nouvelle : le WAL est logique

`src/include/storage/wal/wal_record.h:21` — `TABLE_INSERTION_RECORD`,
`NODE_UPDATE_RECORD`, `REL_DELETION_RECORD`, `CREATE_CATALOG_ENTRY_RECORD`,
`UPDATE_SEQUENCE_RECORD`. Des **opérations**, pas des pages. C'est exactement la
matière d'une réplication logique à la Postgres, et c'est la question
architecturale qui décide de tout : elle est tranchée dans le bon sens.

Existent déjà : un `WALReplayer` (592 lignes) qui rejoue ces records, des sommes
de contrôle (`checksum_reader/writer`), et un `databaseID` dans l'en-tête du WAL
— un suiveur peut donc refuser un WAL étranger.

N'existe pas du tout : `replication`, `replica`, `standby`, `failover` — zéro
fichier dans tout le moteur.

## 4. Point par point : faisabilité et travail

| | Faisabilité | Travail |
|---|---|---|
| Bascule — panne de **processus** | déjà faisable | **jours**, zéro C++ |
| Bascule — panne de **machine**, volume réattachable | dépend de l'infra | **jours** de tuyauterie |
| Réplication logique, **premier suiveur** | faisable | **2–4 semaines** de C++ |
| Bascule à laquelle on **confie des données** | faisable, mais | **mois** — ou on emprunte |
| Réplique de lecture **par instantané** | faisable | **jours**, zéro C++ |
| Partitionnement **dans le moteur** | non | projet de recherche |
| Partitionnement **par locataire** | faisable | **jours** |

### Bascule sur panne de processus

**Le verrou de fichier est déjà une élection de chef correcte** : `F_SETLK` est
atomique (`src/common/file_system/local_file_system.cpp:142`), un seul l'obtient,
et l'OS le rend quand le processus meurt. Il manque un superviseur qui relance et
des clients qui se reconnectent. `Serveur` en fait déjà la moitié : `Occupe` dit
« quelqu'un d'autre tient, ne le tue pas ».

### Réplication logique, par ordre de difficulté

1. **Le point d'accroche** — `WAL::logCommittedWAL` (`src/storage/wal/wal.cpp:28`)
   fait dix lignes et a déjà le lot de records en mémoire au moment du flush.
   Y brancher un abonné : **quelques heures**.
2. **Une position reprenable** — `dryReplay` calcule déjà un offset ; l'exposer
   et le persister côté suiveur : **jours**.
3. **Un mode « suivre »** — `replayWALRecord` est déjà par-record ; il faut le
   sortir du chemin de démarrage, qui tronque et supprime le WAL : **une
   semaine**.
4. **La rétention — la partie invasive.** `wal->clear()`
   (`src/storage/checkpointer.cpp:162`) tronque le WAL à zéro au checkpoint. Un
   suiveur en retard perd le fil. Il faut des segments, ou retenir jusqu'à
   l'accusé des suiveurs : **semaines**.
5. **Ce qui coûte vraiment** — promotion, clôture de l'ancien chef, split-brain.
   Ce n'est pas du code, c'est de la conception distribuée, et c'est là que les
   implémentations naïves perdent des données **en silence**. Soit des mois, soit
   on délègue l'élection (etcd, Raft) et on ne garde que le transport.

### Réplique de lecture sans réplication

`CHECKPOINT` est une instruction du langage
(`src/parser/transform/transform_transaction.cpp:25`). Donc : checkpoint, copier
le dossier, basculer un lien, les lecteurs rouvrent. Le §2 prouve que N lecteurs
partagent une base figée. La fraîcheur vaut l'intervalle, et **aucune ligne de
C++**. C'est 80 % du besoin pour quelques jours.

### Partitionnement

Dans le moteur : planificateur, jointures et stockage sont tous mono-nœud. Ce
n'est pas des semaines, c'est un projet de recherche. **Non.**

Au-dessus : N rag3daemons, un par domaine, et rag3weaver qui route. Pas de
requête inter-partitions — mais si la clé est « un projet par base », il n'y en
a pas, et `WorkDomain` est déjà une notion de premier rang chez nous.

## 5. Ce dont je suis le moins sûr

**Les chiffres C++.** J'ai lu les en-têtes, le `WALReplayer` et le
`TransactionManager` ; je n'ai pas écrit une ligne dans ce moteur. Les *jours*
en Rust, je les tiens ; les *semaines* en C++ sont une lecture de code, pas une
expérience. À réviser dès la première demi-journée passée dedans.

## 6. Ce que ça dessine

Trois choses réelles, **sans toucher au moteur**, pour des jours chacune : la
bascule sur panne de processus, la réplique de lecture par instantané, le
partitionnement par locataire. La réplication continue est le vrai chantier, et
elle n'est pas le premier.
