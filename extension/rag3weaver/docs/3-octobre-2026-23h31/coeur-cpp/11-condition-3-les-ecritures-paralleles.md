# Condition 3 — les écritures parallèles : une page avant le code

11 octobre 2026, matin, session cœur C++. Décision de Lucie au réveil : I2 n'est pas bloquante
(un luxe d'après coup ; I1 reste le produit, la page 10 reste prête) ; le chantier devient la
**condition 3** de la stèle : « plusieurs écrivains sur une même base, sans perte ni blocage, le
mode allumé hors du banc » — **oui jusqu'à B, et A d'abord si c'est le même chemin**. A : plusieurs
transactions d'écriture dans le processus qui tient la base. B : plusieurs processus écrivains sur
le même fichier, le second attendant sur un verrou. Comme PostgreSQL et Neo4j ; ne rendre à Lucie
que les écarts. **[lu]** : cartographie d'un agent à `69f3ff53e`/`c9362d9e8` ; **[déduit]** :
raisonné ; rien n'est mesuré. Rien à coder avant que Lucie ait vu cette page.

## 0. Ce qui est, aujourd'hui **[lu]**

- **« Un seul écrivain » est un refus, pas une attente** : `TransactionManager::beginTransaction`
  (`transaction_manager.cpp:43`) refuse la seconde transaction d'écriture (« Only one write
  transaction at a time is allowed ») sauf si `DBConfig::enableMultiWrites` — le réglage
  `debug_enable_multi_writes`, non persistant, reposé à chaque ouverture, lu à deux endroits :
  ce refus, et `Transaction::usesLocks()` qui déclenche toutes les prises de verrou de V1/A3′/A4′/
  V2/I1. **Tout ce qui a été fait depuis le 3 octobre est déjà A, derrière ce drapeau.**
- **Sous le mode, ce qui reste sérialisé** : `begin`, `commit`, `rollback`, `checkpoint`,
  `refreshSnapshot` prennent le même mutex (`mtxForSerializingPublicFunctionCalls`) ; le commit
  entier tourne dessous (stockage local → tampon d'annulation → journal local versé dans le journal
  commun sous `WAL::mtx`, **un fsync par commit**, puis point de reprise éventuel). Un commit à la
  fois ; les écritures entre les commits sont parallèles.
- **Le point de reprise** : automatique seulement si aucune transaction n'est active (sinon
  « reporté », sans attendre) ; explicite ou forcé, il pose `checkpointPending`, bloque les
  nouvelles transactions et attend le départ des autres jusqu'à 5 s (« Timeout waiting for active
  transactions… ») ; **un seul point de reprise attend à la fois** (`:180`, la cause du rouge A7).
- **Plusieurs processus** : l'écrivain pose un `fcntl(F_WRLCK)` sur tout le fichier de données à
  l'ouverture (port de Vela), le second écrivain est **refusé** (« Could not set lock on file ») ;
  un lecteur n'en prend pas et se protège par le `ReadersLock` du lecteur affamé (`flock` partagé
  sur `<base>.readers`, exclusif au point de reprise), avec une limite nommée : un lecteur resté
  ouvert pendant un point de reprise n'est pas protégé (sa « marche 5 », une époque). Dans le
  processus, deux `Database` écrivaines du même chemin sont refusées (`57c8389b4`).
- **Ce qui vit en mémoire, par processus, et ne se partage pas tel quel** : le tampon de pages, le
  catalogue et sa version, **les versions MVCC** (`VersionInfo`, `UndoBuffer`), les horodatages
  (`lastTimestamp`, des compteurs), les transactions actives, la table des verrous V1 (`std::map`,
  `std::mutex`, `condition_variable` ordinaires, identifiants de transaction du processus), l'index
  de clé primaire, les points d'entrée HNSW. **Les versions ne sont nulle part sur le disque** : une
  ligne validée n'est visible d'un autre processus qu'après rejeu du journal à l'ouverture.
- **Le produit** : un seul processus écrit, mode éteint — le serveur MCP (ou `rag3daemon`, qui tient
  la base et sert le Cypher en HTTP) ; l'indexeur est un **fil** du serveur, ses écritures
  sérialisées en Rust par un mutex ; les lecteurs d'autres processus ouvrent en lecture seule (le
  lecteur affamé), ou passent par le démon.

## 1. A — plusieurs transactions d'écriture dans le processus

### Ce que A réutilise : tout

V1 (gestionnaire, deux genres), A3′ (insertion : index partagé + clé exclusive, unicité contre le
dernier validé), A4′ (mise à jour, suppression, extrémités des relations, l'erreur de
sérialisation après attente, rel2), V2 (l'annonce en tête, instantané repris), I1 (un écrivain à
la fois sur une table indexée). A = **allumer le mode par défaut**, et fermer ce que le banc
montre encore. Les rouges du banc qui touchent A, dans l'ordre où je les prendrais :

| # | Rouge du banc | Marche | Ce que ça prouve |
|---|---|---|---|
| 1 | `LockBench.FailedTransactionReleasesItsLocksAtRollback` (`waited`) | **T0** : une transaction explicite en échec reste ouverte, marquée en échec, refuse toute instruction jusqu'au `ROLLBACK`, et **garde ses verrous** (note des verrous §2, « préalable, pas un à-côté » — PostgreSQL) | aujourd'hui les verrous tombent à l'échec et le client repasse en auto-commit par morceaux |
| 2 | `Copies/TwoCommitsThatWantACheckpoint.DoNotWaitForTheTimeout/Forced` | **A7** : deux validations qui veulent un point de reprise ne s'attendent pas jusqu'au délai — le second se range derrière le premier (une file, pas « un seul attend ») ; depuis le COPY journalisé, le cas ne reste que pour l'index créé et le repli | les deux valident en < 2,5 s |
| 3 | `LockBench.RecoveryOfAJournalWithADuplicateKey…` | **A3′ reprise** : la base se rouvre malgré un doublon validé au journal, la transaction fautive refusée en nommant la clé | la durabilité sous deux écrivains |
| 4 | `C7_RandomMix/Thread_Crash` (+ `Thread_Hot`/`Reopen` probabilistes, H4) | **C7** : après un arrêt brutal, plus de relation vers un nœud supprimé par un autre écrivain (relations pendantes nées au rejeu : ticket ouvert) | l'intégrité après rejeu sous mélange |

Puis deux marches sans rouge aujourd'hui, parce qu'aucun témoin ne les joue :

- **A8, le point de reprise sous écrivains permanents** **[déduit]** : avec des écrivains qui se
  succèdent sans trou, le point de reprise automatique est « reporté » à chaque fois et le journal
  grossit sans fin (aujourd'hui le produit n'a qu'un écrivain, le trou existe toujours). Forme :
  le point de reprise automatique pose `checkpointPending` comme l'explicite, borne son attente, et
  les nouvelles transactions attendent le temps de l'écriture. **Écart avec PostgreSQL** : chez lui
  le point de reprise ne bloque jamais un écrivain (les pages sales s'écrivent en arrière-plan,
  les versions sont dans le fichier) ; chez nous il bloque brièvement, parce que les versions sont
  en mémoire et qu'il les plie. Témoin : N écrivains en boucle, le journal reste borné, aucun ne
  reçoit le délai.
- **A9, le mode allumé par défaut** : le réglage perd son `debug_`, devient `enable_multi_writes`
  à `true` ; le banc, les e2e et le produit tournent dessus ; un `CALL enable_multi_writes=false`
  reste pour l'ancienne sémantique. Son coût **[déduit, à mesurer]** : par ligne écrite, une clé
  imprimée (`lockKeyOf` → `toString`) et une entrée de `std::map` sous un mutex, sans attente quand
  on est seul ; le COPY ne prend qu'un verrou d'index. À mesurer sur luciepc avant d'allumer :
  `SetToAnotherVectorLineByLine`, un COPY de 200 000 lignes, et une insertion ligne à ligne de
  100 000 lignes, mode éteint contre allumé — si le ligne à ligne perd plus de quelques pour cent,
  la clé imprimée se remplace par la clé brute (entier ou chaîne) avant d'allumer.
  **Mesuré le 11 octobre (§5) : le coût est nul.**

Ce que A ne change pas : le commit reste sérialisé (un fsync par commit ; le regroupement des
commits est une optimisation, pas une condition) ; I1 garde un écrivain à la fois par table
indexée.

Ce que A donne au produit : l'indexeur (un fil) et les écritures de l'agent ne s'attendent plus
que sur les clés qu'ils partagent ; le mutex Rust du catalogue peut se relâcher — témoin côté
produit : deux agents qui écrivent en même temps (c'est la preuve que la stèle demande).

## 2. B — plusieurs processus écrivains sur le même fichier

### Ce que B réutilise de A

L'interface du gestionnaire (`acquire(clés triées, mode, échéance)`, `releaseAll`) ; l'annonce en
tête (V2), forme naturelle d'un processus qui prend tout d'un coup dans l'ordre ; l'erreur de
sérialisation après attente (A4′) ; le `ReadersLock` et le rejeu à l'ouverture (le lecteur affamé)
; le refus `F_WRLCK` comme filet.

### Ce que B ne peut pas réutiliser, et pourquoi c'est la question entière **[déduit]**

PostgreSQL fait B parce que **tout l'état partagé vit en mémoire partagée** (tampon, table des
verrous, horodatages, et les versions sont dans les pages). Neo4j **ne fait pas B** : un seul
processus tient le magasin (le mode embarqué refuse le second), les autres lui parlent par le
réseau. Chez nous, les versions MVCC, le catalogue, les horodatages, l'index de clé, les points
d'entrée HNSW sont de la mémoire privée du processus, et le journal n'est lu par un autre
processus qu'à son ouverture. Deux processus qui écriraient le même fichier en même temps ne se
verraient pas ; un verrou par clé entre eux (la piste `fcntl` par plage de la note des verrous)
ordonnerait des écritures que l'autre ne peut pas relire. **B « à la PostgreSQL » est une refonte**
(versions et tampon en mémoire partagée, ou expédition du journal entre processus et relecture
avant chaque écriture) — ce n'est pas sur le chemin de la stèle, et je ne le propose pas.

### B tel qu'il est atteignable, marche par marche

- **B1, le lecteur d'un autre processus** : fait (le lecteur affamé, `ReadersLock`), avec sa
  marche 5 à finir — un lecteur resté ouvert pendant un point de reprise lit une époque et le
  sait ; `ReadOnlyReader.InsistingReaderIsNotStarvedByCheckpoints` sort du probabiliste.
- **B2, le second écrivain attend au lieu d'être refusé** : le verrou de fichier devient une
  **attente bornée** (`F_SETLKW` ou boucle sur `F_SETLK`, délai nommé, le drapeau d'interruption
  regardé) ; le premier écrivain, en fermant, laisse la place ; le second ouvre, rejoue le journal,
  écrit. C'est le relais, pas la simultanéité : « le second écrivain attend sur un verrou » au grain
  du fichier. Témoin : `C9_SecondWriterProcessRefused` retourné (il attend puis écrit, les deux
  écritures sont là), le lanceur `Process` du banc en marche pour les cas qui ne demandent pas la
  simultanéité. Petit (un jour).
- **B3, un processus tient la base, les autres lui parlent** : c'est la réponse de Neo4j, et elle
  existe déjà chez nous — `rag3daemon`. Avec A allumé, le démon sert plusieurs écrivains à la
  fois ; B3 est côté produit (le démon par défaut, le `takeover` des sessions remplacé par un
  bail), pas côté moteur.
- **B4, le vrai multi-processus** : la refonte ci-dessus. À ne décider que si B2 + B3 ne suffisent
  pas à un usage réel ; **[écart à rendre à Lucie]** : PostgreSQL le fait, Neo4j ne le fait pas, je
  recommande la voie Neo4j (B3), qui est déjà la nôtre.

### Où vit la table des verrous pour B

Pour B2 et B3 : là où elle est, dans la `Database` du processus qui écrit (il n'y en a qu'un à la
fois). Pour B4 seulement : en mémoire partagée ou derrière un arbitre, avec des identifiants de
transaction globaux et la libération à la mort d'un processus — la piste `fcntl` par plage de la
note des verrous en est la moitié (le noyau libère et détecte l'interblocage), l'autre moitié est
l'état partagé qui n'existe pas.

## 3. L'ordre, testable une marche à la fois

1. **T0** (rouge 1) — préalable des verrous depuis le 3 octobre, un jour.
2. **A7** (rouge 2) — la file des points de reprise, un jour.
3. **A3′ reprise** (rouge 3) — un jour.
4. **A9, la mesure puis le mode par défaut** — une demi-journée de mesure, puis le basculement
   avec la liste complète et les e2e du produit dessus.
5. **A8**, le point de reprise sous écrivains permanents — un témoin et une forme, un à deux jours.
6. **C7** (rouge 4) — le plus incertain (chemin non trouvé le 10 octobre), deux à trois jours.
7. **B2**, l'attente du second écrivain au fichier — un jour.
8. Le produit : deux agents qui écrivent en même temps, et le démon par défaut (B3, arbre
   principal / mémoire longue).

Ordres de grandeur d'agent, à lire comme des rapports entre eux, pas comme des délais.

## 4. Les écarts à rendre à Lucie

1. **Le point de reprise bloque brièvement les écrivains** (A8) là où PostgreSQL ne bloque jamais —
   parce que nos versions sont en mémoire. Accepter, ou mettre les versions dans le fichier (une
   refonte).
2. **B = relais + démon, pas simultanéité entre processus** (B2 + B3, la voie Neo4j) — le vrai B
   (B4) est une refonte que je ne recommande pas.
3. **Le commit reste sérialisé avec un fsync par commit** — PostgreSQL regroupe les commits ; nous
   pas, tant que personne ne le mesure comme un manque.
4. **Le mode allumé par défaut change le nom d'une erreur** : « Only one write transaction… »
   disparaît ; un second écrivain attend sur ses clés, et peut recevoir « lock timeout », « deadlock
   detected » ou « could not serialize » — les clients qui comptaient sur le refus immédiat doivent
   le savoir (rag3weaver n'en dépend pas : il sérialise lui-même).

## 5. A9, la mesure du coût du mode **[lu]** (11 octobre 2026, 13 h 47)

Protocole : `test/transaction/concurrence/lock_cost_test.cpp` (`Mode/LockCost.*`, joué seulement
sous `CONCURRENCE_MEASURE=1`), sur luciepc en tenue exclusive (`poste mesure`), moteur Release bâti
de `rag3db-a9` — identifiant de bâti `rag3db-build-2c7704b27…-Release-p12-v11-g17-s18`, imprimé sur
chaque ligne de mesure, et porté par l'extension vector chargée (LOAD EXTENSION la refuse sinon).
Trois cas, chacun sur une base neuve, mode éteint (`debug_enable_multi_writes=false`, le défaut)
puis allumé, les deux modes alternés cas par cas, trois répétitions (`--gtest_repeat=3`) ; chaque
cas vérifie son compte de lignes après la mesure (18 verts).

| cas | éteint (ms, trois passes) | allumé (ms, trois passes) | lecture |
|---|---|---|---|
| mille SET d'un vecteur vers un autre, ligne à ligne, sous index HNSW (1 000 lignes) | 7 262 · 10 523 · 10 274 | 8 099 · 11 110 · 7 766 | bruit de ±30 % d'une passe à l'autre **dans les deux modes**, sans sens constant (somme : 28,1 s contre 27,0 s) |
| COPY de 200 000 lignes (id, chaîne, double) depuis un CSV | 181 · 181 · 184 | 178 · 175 · 187 | nul : un verrou d'index par COPY |
| 100 000 insertions ligne à ligne, auto-commit, instruction préparée | 59 877 · 56 782 · 56 964 | 59 569 · 57 160 · 58 068 | +0,7 % sur la somme (173,6 s contre 174,8 s), sous le bruit entre passes (±3 %) |

Lecture : par ligne écrite, la clé imprimée et l'entrée de la table des verrous coûtent moins que
le bruit de mesure — l'insertion ligne à ligne est à ~570 µs par ligne dans les deux modes, dominée
par le commit (un fsync par instruction en auto-commit). **La condition de la page (« si le ligne à
ligne perd plus de quelques pour cent ») n'est pas atteinte** : rien à remplacer avant d'allumer.
Le mode par défaut se bascule en lot à part (le réglage perd son `debug_`, `enable_multi_writes`
à `true`, `CALL enable_multi_writes=false` pour l'ancienne sémantique, la liste complète et les e2e
du produit dessus), sur le mot de Lucie, et se renverse par le même réglage.

**La décision (11 octobre, 16 h 10)** — le mot de Lucie, au matin, rendu par l'orchestration :
« A9 mesure puis bascule du défaut sur son coût dit », la condition pour s'abstenir étant « plus
de quelques pour cent » sur le ligne à ligne. La mesure qui déclenche : +0,7 % sur 100 000
insertions ligne à ligne, sous le bruit de ±3 % entre passes ; nul sur le COPY ; sans sens sur le
SET sous index. La condition n'est pas atteinte : **le mode est le défaut**, lot `verrous-a9b`
(le réglage `enable_multi_writes`, `true` dans `DBConfig`, les témoins de l'ancienne sémantique
la rendent par `CALL enable_multi_writes=false`, deux commentaires Rust mis à jour), renversable
par le même réglage. Lucie a levé en séance la permission qui bloquait l'édition.
