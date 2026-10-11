# A8, le point de reprise sous écrivains permanents — le dernier partant écrit

Pour la session cœur C++ (relecture) et l'orchestration. Seconde session cœur C++, 11 octobre
2026. Suite de la page 05, qui dit pourquoi la première forme bloquait le planificateur. La forme
ci-dessous a été relue par la session cœur C++ avant le code.

## Le défaut

Au seuil du journal, le point de reprise automatique n'avait lieu que si aucune autre transaction
n'était ouverte ; sinon il était reporté. Sous des écrivains qui se succèdent sans trou, il était
reporté à chaque validation, et le journal grossissait sans fin.

## La forme

- **La validation qui atteint le seuil**, s'il reste d'autres transactions ouvertes, pose la
  demande et rend la main, sans attendre. La demande, c'est `checkpointPending`, qui retient les
  nouvelles transactions, plus `autoCheckpointRequested` et l'heure. Attendre dans une validation
  tiendrait un fil du planificateur dont les autres ont besoin pour partir.
- **Le dernier à partir** (`writeRequestedCheckpointIfLastNoLock`, après toute validation, lecture
  seule comprise) écrit le point de reprise. S'il échoue, son erreur dit que sa transaction est
  validée au journal et que c'est le point de reprise d'après qui a échoué.
- **Une annulation n'écrit jamais**, parce qu'elle peut venir de la fermeture d'une connexion ou de
  la base. Partie la dernière, elle renonce à la demande, et le point de reprise est reporté.
- **Un BEGIN retenu plus de 250 ms** par une demande automatique y renonce, et le point de reprise
  est reporté comme avant (une transaction longue la tient). Les lecteurs attendent aussi, 250 ms
  au plus. La borne valait d'abord une seconde.
- **Sous un lecteur long, les écrivains n'attendent qu'une fois.** Une demande abandonnée note le
  plus grand identifiant des transactions alors ouvertes (`lastBlockingTransactionID`). Aucune
  nouvelle demande n'est posée tant qu'une transaction d'identifiant inférieur ou égal reste
  ouverte : c'est le report d'avant A8, jusqu'à son départ. La liste complète l'a exigé : sans
  cela, chaque validation sous un lecteur ouvert reposait une demande, et l'écriture suivante
  l'attendait. Dix écritures prenaient 9 s (`SingleWriterCrash.BurstOfWritesUnderAnOpenReaderDoesNotWait`).
- **Le BEGIN explicite attend dans le fil de l'appelant**
  (`TransactionManager::waitUntilNewTransactionsAreAllowed`, appelé par
  `ClientContext::executeNoLock`), avant de passer au planificateur. Une instruction BEGIN
  s'exécute sur un fil du planificateur, et y attendre tenait un fil dont les transactions ouvertes
  avaient besoin pour valider : une passe sur six restait bloquée jusqu'au garde du banc.
- **Un point de reprise explicite ou forcé reprend une demande en attente** et la remplit. Une
  validation forcée qui trouve une demande la lève d'abord, pour ne pas se ranger dans une file
  sans porteur (A7).

## L'écart avec PostgreSQL

Le point de reprise retient les nouvelles transactions, lecteurs compris, le temps que les
transactions ouvertes partent et que le point de reprise s'écrive : 250 ms au plus, puis il se
reporte. PostgreSQL ne bloque jamais, parce que ses versions sont dans le fichier.

- **Sous un lecteur long**, un écrivain attend une fois, 250 ms au plus, au BEGIN qui suit la
  demande. Chez PostgreSQL, il n'attend jamais.
- **Sous une transaction longue** (un lecteur, un gros COPY journalisé), le point de reprise reste
  reporté tant qu'elle dure. Si plus aucune validation ne vient après l'abandon de la demande, le
  journal n'est replié qu'à la fermeture de la base, **comme avant A8**.

**Un test existant a changé de moment** :
`SingleWriterCrash.AutoCheckpointPostponedWhileATransactionStaysOpen`. C'est désormais le départ de
la transaction restée ouverte, dernière à partir, qui écrit le point de reprise demandé ; avant,
c'était la validation suivante. Le test mesure le journal avant la fermeture de cette transaction,
et non plus après. L'attendu ne change pas : une fois la transaction fermée, le journal est replié,
au plus tard à la validation suivante.

## Le témoin

`LockBench.AutoCheckpointIsNotPostponedForeverUnderWritersWithoutAGap` : 8 écrivains sans pause
(BEGIN, une ligne d'1 Kio, COMMIT), 300 transactions chacun, un seuil de 64 Kio. Le journal reste
sous un plafond, aucun écrivain ne reçoit le délai, et toutes les lignes sont validées.

Le plafond a été fixé par la mesure, sur luciepc, en Release :

| | passes | journal au plus haut |
|---|---|---|
| master | 6 | 210 à 358 Kio |
| avec le correctif | 6, puis 10 | 74 à 91 Kio |

**Deux fois le seuil (128 Kio)** sépare franchement les deux séries. Le témoin rougit sur master
par cette séparation, pas par un seuil réglé après coup. Deux premiers jets ont été écartés (page
05) : le premier laissait des trous où le point de reprise se glissait, et le second rendait le
point calme impossible par construction.

## Ce qui reste (relecture du cœur)

- **Deux attentes sur un fil du planificateur, d'avant ce lot** : le CHECKPOINT explicite
  (`TransactionManager::checkpoint`, dans l'opérateur Transaction) et la validation forcée (COPY
  forcé, index créé : `stopNewTransactionsAndWaitForOthersNoLock` dans le commit, sur le fil du
  COMMIT). Toutes deux sont bornées par le délai de 5 s, et c'était le 5 014 ms d'A7 avant la file.
  **Fait dans un lot d'après, sous une autre forme** (relue et préférée par le cœur) : elles ne
  sont pas déplacées. Pendant qu'elles attendent, le fil ouvrier rend sa place à un remplaçant,
  par le `TaskScheduler::BlockingWait` de l'attente des verrous (A3′). Cela couvre d'un coup
  le CHECKPOINT, la validation forcée et la file d'A7, dont les attentes étaient sans délai.
  Témoin : `LockBench.CheckpointsWaitingForAnOpenWriterLeaveAThreadForItsCommit`.
- **Le dernier partant peut être une connexion en lecture seule** du même processus (la validation
  d'une transaction READ_ONLY) : c'est son contexte client qui sert au Checkpointer. Le `readOnly`
  de DBConfig est celui de la base, faux ici, donc l'écriture passe d'après la lecture. **Non
  éprouvé** : aucun lecteur dans le témoin.

