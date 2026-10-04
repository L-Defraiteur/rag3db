# Le point de reprise que `COPY` force — demi-page avant de coder

4 octobre 2026, session cœur C++. Demandée par l'orchestration : avec la transaction par
paquet, le premier index de ce dépôt fait 177 s sur disque, dont 66 s de `COMMIT` des 13
paquets (5 s par paquet), mesure de la session des embarquements.

## 1. Pourquoi `COPY` force son point de reprise

Parce que **ses lignes ne sont pas dans le journal**. Un `COPY` écrit ses blocs de colonnes
directement dans le fichier de données, sur des pages prises à part
(`OptimisticAllocator`, `src/storage/optimistic_allocator.cpp`) ; ce qui dit où sont ces
pages — les métadonnées des groupes de nœuds, le nombre de lignes, l'index de clé primaire,
les statistiques — ne vit qu'en mémoire jusqu'au point de reprise. Le journal ne reçoit que
le début et la fin de la transaction : l'enregistrement `COPY_TABLE_RECORD` existe mais
n'est jamais écrit, et son rejeu ne fait rien (`src/storage/wal/wal_replayer.cpp:742`).
D'où `setForceCheckpoint` à chaque `COPY` (`src/main/client_context.cpp:535-538`), honoré au
commit (`src/transaction/transaction_manager.cpp:71-76`).

Sans ce point de reprise, un arrêt brutal perdrait les lignes d'un `COPY` validé, et le
journal pourrait porter ensuite des écritures qui s'y réfèrent par décalage de ligne
(mises à jour, suppressions, relations) : un rejeu incohérent, pas seulement une perte.

Ce qui existe déjà : plusieurs `COPY` dans une transaction explicite ne font qu'un point de
reprise, au `COMMIT`. C'est ce que la transaction par paquet exploite ; il reste un point
de reprise par paquet.

## 2. Trois voies

Les voies a/b/c d'hier ne sont dans aucun de mes fichiers ; je les repose ici.

- **(c) Rendre le point de reprise moins cher — à mesurer d'abord.** 5 s pour un paquet est
  suspect : si le coût suit la taille de la base et non celle du paquet (catalogue et
  métadonnées réécrits en entier, index de clés, synchronisations), c'est un défaut en soi.
  Aucune sémantique ne change, aucune sûreté n'est perdue. Une demi-journée de mesure dit si
  le gain est là.
- **(b) « Chargement initial » : un seul point de reprise, à la fin.** Une fenêtre déclarée
  dans le journal ; les `COMMIT` qui portent un `COPY` n'y forcent plus rien ; la clôture
  écrit le point de reprise. À l'arrêt brutal dans la fenêtre, la reprise voit la fenêtre
  ouverte et **écarte tout le journal depuis le dernier point de reprise** : la base revient
  à l'état d'avant la fenêtre, entière et cohérente, et le chargement se refait. Coût en
  sûreté : on perd toute la fenêtre, pas le seul paquet en cours — acceptable pour un
  premier index, qui se refait ; à refuser hors de ce cas (d'où la déclaration explicite).
  Deux à trois jours avec ses témoins. Les pages écrites par la fenêtre perdue restent
  dans le fichier jusqu'à ce qu'on les reprenne : à mesurer.
- **(a) Journaliser le `COPY`.** Au commit, synchroniser les pages déjà écrites et écrire
  dans le journal de quoi retrouver les métadonnées (plages de pages, groupes, clés). Le
  point de reprise retombe au seuil comme pour toute écriture ; rien n'est perdu d'un paquet
  validé. C'est le vrai remède et le plus cher : il touche le rejeu, l'index de clés et la
  récupération des pages — une semaine au moins.

Ma recommandation : (c) d'abord, parce qu'elle peut suffire et ne coûte rien en sûreté ;
(b) si elle ne suffit pas ; (a) plus tard, quand un chargement en service en aura besoin.
Une question à poser avant tout : qu'est-ce qui oblige à 13 paquets plutôt qu'une seule
transaction ? Si c'est la mémoire, (b) est la bonne voie ; si rien n'y oblige, un seul
`COMMIT` règle la chose sans toucher au moteur.

## 3. PostgreSQL et Neo4j — les écarts seulement

- **PostgreSQL** journalise les lignes d'un `COPY` ; le point de reprise reste au seuil.
  Avec `wal_level = minimal`, un `COPY` dans une table créée ou vidée par la même
  transaction saute le journal et synchronise les fichiers de la table au commit — sûr
  parce qu'un fichier neuf, non validé, est invisible au catalogue. Écart : nous sautons le
  journal pour **tout** `COPY`, table neuve ou non, et nous payons un point de reprise de
  toute la base là où il synchronise les seuls fichiers de la table. La voie (a) est son
  équivalent.
- **Neo4j** fait son import initial hors ligne, sans journal ; la base n'est utilisable
  qu'à la fin, et un arrêt en cours se refait depuis le début. Écart : la voie (b) est la
  même idée, mais base ouverte et fenêtre déclarée.

## 4. La preuve exigée

Un processus enfant ouvre la base, valide *k* paquets (chacun : `COPY` de nœuds, `COPY` de
relations, puis une écriture journalisée qui s'y réfère), et reçoit SIGKILL **au milieu du
paquet *k* + 1**, base ouverte, journal non vide (vérifié avant de rouvrir). Un processus
neuf rouvre. Ce qui doit tenir :
- les comptes par table : exactement *k* paquets pour (a) et (c), zéro ligne de la fenêtre
  pour (b) — jamais un demi-paquet ;
- chaque clé se retrouve par l'index, les deux sens de chaque table de relations rendent le
  même compte ;
- un `COPY` de plus aboutit, et le chargement refait donne les comptes complets ;
- la taille du fichier après reprise et rechargement reste sous une borne (pas de pages
  perdues sans fin).

Le témoin que le banc écrirait : ce scénario, par `fork` et SIGKILL, joué sur plusieurs
points de mort (pendant le `COPY`, entre le `COPY` et le `COMMIT`, pendant le `COMMIT`,
pendant le point de reprise final), rouge sur le code d'aujourd'hui dès que le point de
reprise forcé est retiré sans rien mettre à sa place.

Rien de cela ne va aux amonts.
