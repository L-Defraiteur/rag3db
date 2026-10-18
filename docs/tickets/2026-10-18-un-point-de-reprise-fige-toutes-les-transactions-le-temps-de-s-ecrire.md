# Un point de reprise fige toutes les transactions le temps de s'écrire, lecteurs compris, et il s'écrit toutes les trois secondes sous charge

- **État** : ouvert (mesuré par le banc le 18 octobre 2026 ; à placer dans la file du cœur C++
  par l'orchestration)
- **Gravité** : attente, pour tout le monde : aucun BEGIN n'entre et chaque validation attend
  que le point de reprise soit écrit ; sous vingt écrivains, 1,2 à 1,4 s par point de reprise,
  un toutes les 3 s au seuil. Ni perte ni réponse fausse.
- **Atteignable en service** : oui, c'est le régime ordinaire d'un hôte qui indexe (le journal
  atteint le seuil sans cesse).
- **Touche rag3weaver** : oui (chaque appel d'outil pendant une indexation peut attendre un
  point de reprise entier).
- **Ouvert le** : 18 octobre 2026, session cœur C++, sur demande de l'orchestration, d'après la
  mesure du banc.
- **Pour** : cœur C++.

## Ce qu'on voit (banc, `DdlEveryMinute`, luciepc, 6ff805fcb sur master 248bf03cc, 2 Gio,
`RAG3DB_PROFILE_CHECKPOINT=1`, vert)

- Les `ADD COLUMN` ne coûtent rien : `ADD note` 7,9 ms, `ADD rang DEFAULT 0` 9,1 ms ; pendant
  chacun, p99 des écrivains de `Doc` 39 et 32 ms, de `Note` 19 et 15 ms. L'attente de 1,3 s « sur
  toutes les tables » de la première passe était un point de reprise au seuil tombé au même
  moment (page `coeur-cpp/16`, §2, départagé).
- Sur la passe : **129 points de reprise « au seuil », environ un toutes les 3 s**, et un
  « forcé » (celui de `CREATE_VECTOR_INDEX`, 862 ms : `Doc` 764, `Link` 48, `Note` 10). Le
  `CHECKPOINT` explicite de la minute 5 dure 1,24 s, dont 1,07 s pour `Doc`.
- Journal : `~/.cache/rag3weaver-build/jeu-vingt/minute-profil-6ff805fcb.log` sur luciepc.

## Le seuil, et ce que coûte chaque étape (lu dans le journal du banc)

- **Le seuil** : `checkpointThreshold`, 16 MiB par défaut (`database.h:78`, taille du journal :
  `taille du journal local de la validation + taille du fichier`, `Checkpointer::canAutoCheckpoint`),
  réglable par `CALL checkpoint_threshold=<octets>` (`settings.cpp:196`). Sous vingt écrivains de
  vecteurs, le journal atteint 16 MiB toutes les 3 s.
- **129 validations ont dépassé le seuil**, mais la plupart n'ont fait que poser la demande (A8 :
  d'autres transactions ouvertes, « Checkpoint postponed … the journal holds 16 777 779 bytes ») ;
  **14 points de reprise ont été écrits** sur la passe, dont 10 par la validation qui franchissait
  le seuil (`point-de-reprise=` de 0,1 à 1 819 ms, médiane 909 ms) et les autres par la dernière
  transaction à partir.
- **Les étapes** (`[checkpoint-profile]`) : `tables` (réécriture en place des groupes modifiés)
  fait 85 à 97 % du temps : 85 ms (2 381 pages), 1 179 ms (5 732 pages), 1 239 ms (8 719 pages),
  1 693 ms (14 642 pages) ; `journal+pages-ombres` 45 à 110 ms ; `catalogue+metadonnees` moins
  d'une milliseconde ; `fin` 0,3 à 12 ms ; `attente des lecteurs` 0 ms (c'est le verrou des
  lecteurs du Checkpointer, pas l'attente des BEGIN, qui n'est pas mesurée par ce profil).
  Dans le plus long, `Doc` (la table à vecteurs) pèse 764 ms, `Link` 48, `Note` 10.
- Donc : **le temps d'un point de reprise est le temps de réécrire les tables**, et il croît avec
  le nombre de pages touchées depuis le précédent ; le seuil en octets de journal le déclenche
  toutes les 3 s sous écriture soutenue, et chaque écriture fige tous les BEGIN pendant 1 à 1,9 s.

## Deux voies à comparer (sans rien écrire pour l'instant)

1. **Le seuil** : plus haut (128 MiB ?) ou en temps (toutes les 30 s) : moins de points de
   reprise, mais chacun réécrit plus de pages et fige plus longtemps, et le rejeu après un arrêt
   brutal rejoue plus de journal ; une croissance du seuil avec la charge (doubler tant que les
   points de reprise se suivent à moins de N s) lisse le régime sans changer la forme. À mesurer
   avec le même scénario : p99 des écrivains et lecteurs, et temps de rejeu.
2. **La forme de PostgreSQL** : le point de reprise ne retient aucune transaction ; il écrit les
   pages sales pendant que les autres continuent (les versions validées avant son départ, les
   suivantes iront au prochain), et tronque le journal après. Chez nous, cela demande les trois
   pièces de la section suivante ; c'est la seule voie qui rende « aucun bloqué ».

## La première voie, mesurée (banc, 18 octobre 2026)

`TwentyWriters.DdlEveryMinute` sur luciepc, à 35795bbd7 (master 8f4779bdc, avec le correctif du
BEGIN refusé), sous la charge des compilations voisines. Tampon de 2 Gio, point de reprise
automatique en marche, `RAG3DB_PROFILE_CHECKPOINT=1`, seuil réglé par
`SCENARIO_CHECKPOINT_THRESHOLD`. Journaux : `~/.cache/rag3weaver-build/jeu-vingt/seuil-*.log`
sur luciepc.

| seuil | points de reprise au seuil | pic du tampon | refus « Unable to allocate memory » | p99 hors DDL (Doc / Note / lecteurs) | verdict |
|---|---|---|---|---|---|
| 16 Mio (défaut) | 67, et 1 forcé | 1 125 Mio | aucun | 445 / 38 / 41 ms (max Doc 4,7 s) | vert |
| 64 Mio | 0 | 2 049 Mio | 30 137 Doc, 4 355 Note, 8 587 lecteurs | 492 / 328 / 309 ms | rouge |
| 256 Mio | 0 | 2 050 Mio | plus de 300 000 par acteur | sans objet | rouge |

À 16 Mio, sur les 12 points de reprise profilés en détail : médiane 1,22 s, p90 2,02 s,
max 2,10 s. À 64 et à 256 Mio, le seul contrôle rouge est `index-creation-refuses-no-reader` :
les lecteurs sont refusés par le tampon plein pendant `CREATE_VECTOR_INDEX`.

**Le tampon est plein avant que le journal atteigne le seuil.** Avec vingt écrivains de
vecteurs sous HNSW, la mémoire tenue par les pages validées mais pas encore reportées croît plus
de trente fois plus vite que le journal. Relever le seuil ne réduit pas le nombre de points de
reprise : il remplace leur attente par des refus.

**Le fond.** Le tampon ne peut pas rendre une page validée tant qu'elle n'est pas reportée, et le
seuil compte les octets du journal au lieu de la mémoire tenue. Chez PostgreSQL, quand le tampon
manque de place, il écrit ces pages sur disque, ce que le journal permet : une page écrite avant
le checkpoint reste rejouable. Le checkpoint n'y sert qu'à borner la reprise. Avec le point de
reprise automatique coupé (`SCENARIO_NO_AUTO_CHECKPOINT=1`), le même tampon de 2 Gio déborde avant
la minute 5 : 58 598 refus, et `CREATE_VECTOR_INDEX` comme `CHECKPOINT` refusés tant que les
écrivains tournent.

## Effet de bord mesuré : le convoi après le point de reprise (banc, 18 octobre, 8 fils, 9936a4e6d)

Un `CHECKPOINT` de 786 ms retient vingt écrivains de `Doc` (table sous HNSW) qui repartent
ensemble à sa fin et se sérialisent un par un sur le verrou INDEX exclusif de l'insertion dans
le graphe : ils finissent de 1 080 à 1 218 ms, soit 300 à 430 ms de convoi après le point de
reprise ; les écrivains de `Note` (sans index) finissent à 787 ms. Ce surplus n'est pas le point
de reprise mais le prix du verrou INDEX exclusif par écrivain (lot de la rétention, forme (α)) ;
il tombera avec un verrou plus fin sur l'insertion HNSW. La borne du scénario du banc en tient
compte (durée + 50 ms + écrivains retenus × médiane hors DDL pour les tables indexées).

## Le point 3 du ticket du ROLLBACK sous tampon plein (007b2c3e7), rangé ici

Un `CHECKPOINT` refusé faute de mémoire condamne la base (`checkpointFailed`) même s'il n'a
encore rien écrit : `writeCheckpoint` réécrit en place dès la première table (les colonnes sont
déplacées avant les groupes), et le code ne distingue pas « refusé avant la première mutation »
de « cassé au milieu ». Hotfixs pose un drapeau « l'état en mémoire a changé » au premier pas
irréversible (après avoir déplacé les colonnes seulement une fois les groupes écrits) ; un point
de reprise qui n'écrit pas en place (voie 2) n'a plus ce problème.

## Ce que fait le code

`TransactionManager::checkpointNoLock` : `stopNewTransactionsAndWaitForOthersNoLock` (aucun
BEGIN n'entre, lecteurs compris ; attente bornée du départ de tous), puis `writeCheckpoint`
(chaque table réécrit ses groupes modifiés en place, les colonnes sont renumérotées
`vacuumColumnIDs`, les tables retirées sont récupérées, catalogue et métadonnées sérialisés,
fsync), puis `allowNewTransactionsNoLock`. La validation qui dépasse le seuil l'écrit elle-même
si elle est seule, sinon pose la demande (A8) et la dernière à partir l'écrit ; une demande qui
attend plus de `AUTO_CHECKPOINT_WAIT` est abandonnée. Pendant l'écriture, tout attend.

Chez PostgreSQL, un checkpoint n'arrête aucune transaction : il écrit les pages sales en arrière-
plan, les lecteurs et les écrivains continuent sur les versions en place, et le journal borne la
reprise. Chez nous, l'arrêt tient à la réécriture en place sous les états de balayage (les
pointeurs de blocs et de groupes des lecteurs, les plans liés aux numéros de colonnes).

## Ce qu'il faudrait (voie 2)

Un point de reprise qui ne fige pas tout le monde. Trois pièces, à étudier avant toute écriture :
1. **ne plus arrêter les lecteurs** : les blocs qu'un point de reprise remplace restent vivants
   tant qu'un balayage les tient (la même garde que pour le rollback qui détruit un bloc sous un
   lecteur, ticket du 18 octobre) ; la renumérotation des colonnes ne se fait que sans lecteur, ou
   se fait sans renuméroter ;
2. **ne plus arrêter les écrivains** : la part du point de reprise qui ne dépend que des versions
   validées avant son départ s'écrit pendant que les écrivains continuent, comme un instantané ;
   ce qui arrive après ira au suivant ;
3. **moins souvent** : le seuil (taille du journal) déclenche un point de reprise toutes les 3 s
   sous vingt écrivains ; un seuil en temps ou un seuil plus haut quand le journal rejoue vite
   change déjà le régime (à mesurer avec le même scénario).

Le « forcé » du `CREATE_VECTOR_INDEX` est le sujet du ticket du CREATE concurrent (hotfixs).

**Un refus avant la première écriture n'a pas à condamner la base.** Aujourd'hui, un point de
reprise refusé par le tampon plein, avant d'avoir rien écrit, condamne la base (« A checkpoint of
this database failed, so it must be closed and reopened »). C'est la règle prudente tant que le
point de reprise réécrit en place : le code ne sait pas dire s'il a déjà touché la mémoire. Un
point de reprise qui n'écrit pas en place peut faire cette distinction, et le refus redevient une
erreur nommée. Le banc l'a vu le 18 octobre (ticket
`2026-10-18-un-rollback-sous-un-tampon-plein-bloque-la-base-et-la-fermeture-tue-le-processus`,
point 3).

## Témoin

Le scénario du banc avec `RAG3DB_PROFILE_CHECKPOINT=1` : le p99 des écrivains et des lecteurs
pendant les fenêtres des points de reprise au seuil, avant et après. Fermé quand un point de
reprise au seuil ne fait plus attendre les lecteurs, et que l'attente des écrivains tient dans
une borne dite.

Pour la voie 2 : `FullBufferRollback.RefusedCheckpointLeavesTheDatabaseOpen`
(`test/transaction/concurrence/full_buffer_rollback_test.cpp`, dans `known_red.txt`, contrôle
`refused-checkpoint-leaves-the-database-open`). Un fils remplit le tampon de 64 Mo de pages validées
non reportées, point de reprise automatique coupé ; le CHECKPOINT est refusé par le tampon ; un
BEGIN doit ensuite passer.
