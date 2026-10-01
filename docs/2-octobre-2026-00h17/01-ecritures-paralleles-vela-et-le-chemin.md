# Écritures parallèles : ce que Vela apporte vraiment, et le chemin jusqu'à plusieurs processus

**2 octobre 2026.** Étude en lecture seule — rien bâti, rien exécuté, rien
modifié dans le code. Cartographie par cinq agents de lecture sur des arbres
extraits (`git archive`), **chaque point portant contrôlé par moi dans le
fichier cité**. Ce qui est déduit sans avoir été exécuté est étiqueté *déduit*.

Arbres lus : notre `origin/master` (`9ebcd6400`), la branche
`fin-de-journal-dechiree` (étape E, `0a35f3654`), l'amont Kuzu (`89f0263cc`),
`vela/master` (`2efa20b67`), `vela/storage/concurrent-checkpoint-recovery`
(`4945229bb`), Ladybug (`ladybug-main-2026-08-31`). Sauf mention, les
`fichier:ligne` de Vela sont ceux de leur branche, les nôtres ceux de
`origin/master`.

Décisions de Lucie qui cadrent ce rapport (2 octobre) : *« on fait ce qu'il
faut pour écritures parallèles, peu importe ce que ça coûte »* ; *« oui
jusque B, et A d'abord si dans même chemin »* ; et une piste : *« on n'est pas
obligé d'avoir tout dans un seul fichier »*, la découpe étant par genre d'index.

- **A** : plusieurs transactions d'écriture en parallèle dans le processus qui
  tient la base.
- **B** : plusieurs processus écrivains.

## La réponse en dix lignes

1. **Vela n'est pas un MVCC neuf.** C'est le mode multi-écrivains que Kuzu
   avait déjà, caché (`debug_enable_multi_writes`), allumé par défaut, plus
   **un** correctif qui le rend utilisable (le remappage des offsets de nœuds
   au commit), un point de reprise automatique en fil de fond, et une reprise
   après panne durcie. **Aucune détection de conflit n'a été ajoutée.**
2. **Leur point de reprise n'est pas « non bloquant »** : il attend toutes les
   transactions, lecteurs compris, et ferme la porte jusqu'à sa fin. Leur propre
   commit #17 l'écrit.
3. **Le coût de le prendre n'est plus une demi-journée.** Leur branche change le
   format du journal et du fichier fantôme, porte intact notre bug des 4 096
   octets, et traite une fin de journal coupée à l'inverse de notre étape E.
   Fusion à blanc : 24 fichiers en conflit, 26 avec l'étape E. **Estimation :
   5 à 8 jours pour la branche entière ; 2 jours pour n'en prendre que les deux
   morceaux utiles à A, par le contenu.**
4. **A est sur le chemin de B pour sa moitié « correction »** — validation des
   conflits au commit, remappage des offsets, vérificateur d'intégrité, banc de
   concurrence : B les réutilise tels quels. **Elle ne l'est pas pour sa moitié
   « point de reprise sans blocage dans le processus »**, que B oblige à
   concevoir autrement.
5. **A coûte environ trois semaines**, en huit marches dont chacune a son test.
   Les trois premières ferment des corruptions aujourd'hui silencieuses.
6. **B a une forme praticable sur ce code : le journal partagé, que chaque
   processus rejoue pour rattraper les autres.** Deux à trois mois, avec trois
   inconnues que je ne sais pas chiffrer (§8).
7. **La piste « un fichier par genre d'index » est, côté moteur, presque
   gratuite** et donne des écritures lourdes parallèles entre processus sans
   rien partager. Elle ne donne pas deux écrivains sur les lignes du graphe ;
   A les donne, dans le processus.
8. **Le premier cas de B est déjà rouge chez nous** : un lecteur d'un autre
   processus lit trois choses à trois instants. Une revérification à
   l'ouverture le corrige en un jour ; elle **ne suffit pas** pour un lecteur
   qui reste ouvert.
9. **Mettre les écrivains en file entre processus n'est pas « peu coûteux »**
   comme l'écrivait le document du 6 septembre : le verrou de fichier est pris
   à l'ouverture et tenu toute la vie de la base.
10. **Ordre proposé** : lecteur sûr à l'ouverture → banc de concurrence et
    vérificateur → marches de correction de A → index lourds dans leurs propres
    fichiers → B seulement s'il reste un besoin, sur des interfaces posées
    pendant A.

## 1. Où en est Vela

`vela/master` n'a pas bougé depuis le 14 juin (`2efa20b67`, identique au tag
`vela-master-2026-09-03`). La branche `storage/concurrent-checkpoint-recovery`
est passée de `373f40945` à `4945229bb` : **huit commits non fusionnés chez
eux** — les six de juillet, plus deux du 27 septembre (*« export commit and
checkpoint hooks for shared-library tests »*, *« grow data file for identified
shadow replay and test crash boundaries »*).

| | fichiers de `src/` touchés depuis l'amont | insertions | suppressions |
|---|---:|---:|---:|
| `vela/master` | 58 | 1 312 | 384 |
| la branche | 77 | 3 415 | 710 |

Leur travail de fond le plus récent — la reprise après panne — vit donc hors de
leur propre `master` depuis trois mois.

## 2. Ce que « concurrent writes » veut dire dans leur code

**Plusieurs transactions d'écriture actives en même temps dans un processus :
oui, par défaut.** Mais presque rien n'a été écrit pour ça.

- **Le réglage existait en amont.** `debug_enable_multi_writes`, défaut faux
  (`kuzu-base/src/main/db_config.cpp:30`), saute le refus du second écrivain ;
  l'amont a déjà neuf tests `Concurrent*` dans `transaction_test.cpp` qui
  tournent avec. Vela l'a renommé `concurrent_writes` /
  `experimental_concurrent_writes` et mis à vrai (`db_config.cpp:31`,
  `db_config.h:76-79`). Le refus reste, conditionné au réglage
  (`transaction_manager.cpp:45-50`).
- **Les commits sont sérialisés par un verrou**
  (`transaction_manager.cpp:73-88`), comme chez nous. L'horloge reste deux
  entiers non atomiques protégés par ce verrou ; l'offset de nœud est toujours
  `getNumTotalRows()` au commit (`node_table.cpp:630`).
- **Leur seul ajout pour la concurrence d'écriture** : le remappage des offsets
  de nœuds locaux dans les relations locales, au commit
  (`local_storage.cpp:58-86`, `local_rel_table.cpp:151-200`). Chaque
  transaction numérote ses nouveaux nœuds à partir de la taille de la table à
  son démarrage ; deux transactions concurrentes ont donc des offsets locaux
  qui se chevauchent, et sans remappage leurs relations pointent vers de
  mauvais nœuds. Absent de l'amont et de chez nous (zéro occurrence de
  `remapNodeOffsets`).
- **La détection de conflit est celle de l'amont, à l'octet** —
  `version_info.cpp`, `update_info.cpp`, `chunked_node_group.cpp` et tout
  `src/storage/index/` sont identiques à Kuzu :

| cas | ce que fait le code | référence |
|---|---|---|
| deux mises à jour de la même ligne | exception « Write-write conflict », premier écrivain gagnant | `update_info.cpp:31-38` |
| deux suppressions de la même ligne | exception, sur un chemin sans verrou (création paresseuse de `versionInfo`) | `version_info.cpp:103-118`, `chunked_node_group.cpp:392-396` |
| deux insertions de la même clé primaire | **rien de fiable** : la clé n'est tenue pour existante que si sa ligne est *visible* pour la transaction ; une clé validée après le début de l'autre est insérée en double — *déduit* | `node_table.cpp:392-403`, `in_mem_hash_index.h:284-296` (les nôtres, identiques) |
| une relation vers un nœud que l'autre supprime | **rien** : relation pendante possible — *déduit* | `delete_executor.cpp:32-41`, `rel_table.cpp:398+` |

- **Leurs tests évitent ces cas.** `DefaultConcurrentAgentMemoryUnderAutoCheckpoint`
  donne à chaque fil des clés disjointes ; aucun test, chez eux ni en amont,
  n'oppose deux écrivains sur la même clé ou une suppression à une insertion de
  relation sous concurrence réelle.

## 3. Ce que ça donnerait à rag3weaver

| question | aujourd'hui chez nous | avec Vela |
|---|---|---|
| le second écrivain | refusé, exception immédiate (`transaction_manager.cpp:35-39`) | **démarre en parallèle** — ni refusé, ni mis en attente |
| le point de reprise bloque-t-il les lecteurs ? | oui | **oui, toujours** : il attend `activeTransactionCount == 0`, lecteurs compris (`:162-174`), et tient la porte fermée pendant toutes ses phases (`:301-373`) |
| un lecteur long le fait-il échouer ? | oui, après 5 s | **oui, après 5 s** ; un point de reprise automatique note l'erreur et réessaie au commit suivant — *déduit* : blocages répétés de 5 s sous ingestion continue |
| une transaction ouverte peut-elle se fermer pendant l'attente ? | **non** quand le point de reprise vient d'un commit : il tient le verrou dont elle a besoin (`:57, 72-73`) — l'attente ne peut alors qu'expirer | oui (verrou dédié `mtxForCheckpoint`) |
| le commit qui déclenche le point de reprise automatique | le fait lui-même, en bloquant | pose un drapeau, un fil de fond s'en charge (`:213-258`) |
| `COPY FROM` | force un point de reprise synchrone | idem (`client_context.cpp:522-524`) |

La lecture de l'orchestrateur se confirme : ce qui gêne le produit, c'est le
second écrivain refusé et le point de reprise qui bloque. Vela règle le premier,
améliore le second sans le régler.

## 4. Le coût de fusion aujourd'hui

**À blanc, par `git merge-tree --write-tree`, rien dans un arbre de travail.**
Base commune : `89f0263cc`.

| contre | `vela/master` | la branche |
|---|---:|---:|
| `origin/master` | 18 fichiers en conflit | 24 |
| `origin/master` + étape E | 19 | **26** |

Dans le cœur, pour la branche contre master + étape E : 16 fichiers, dont deux
gros — **`shadow_file.cpp`, 4 zones, 207 lignes entre marqueurs** ;
**`wal_replayer.cpp`, 4 zones, 73 lignes**. Les autres zones font 5 à 23
lignes. Plusieurs conflits sont notre propre travail récent (dépôt
d'extensions : `extension.cpp`, `extension.h`, `transform_extension.cpp`).

**Nos deux correctifs du journal contre leur code :**

- **Le bug des 4 096 octets est intact chez eux.** `checksum_writer.cpp`,
  `checksum_reader.cpp` et `buffered_file.cpp` : **zéro ligne d'écart** avec
  l'amont, dans `master` comme dans la branche ; `resizeBufferIfNeeded` y
  remplace toujours le tampon sans recopier (`checksum_writer.cpp:17-24`). Pas
  de conflit textuel avec notre étape D — notre correctif survit à la fusion,
  et c'est le leur qui manque.
- **`wal_record.cpp`** : chez eux le `default:` est toujours `KU_UNREACHABLE`
  (`:87-89`) ; ils touchent le fichier (43 lignes) pour deux nouveaux types,
  sans conflit avec nous.
- **La fin de journal coupée : sémantiques opposées.** Eux : avec le défaut
  `throwOnWalReplayFailure=true`, refus d'ouvrir ; et même avec `false`, refus
  dès que la coupure touche un enregistrement de point de reprise
  (`wal_replayer.cpp:420-431`, tests `…FailsClosed`). Nous (étape E) : on rouvre
  au dernier COMMIT et on met le reste de côté. Les deux réécrivent `replay` et
  `dryReplay`. Fusionner demande de décider cas par cas : notre branche « fin de
  fichier » doit respecter leurs marqueurs de point de reprise, et notre mise
  de côté doit être reproduite à leurs quatre sites de troncature, sur deux
  fichiers de journal au lieu d'un.
- **Le format change, dans les deux sens.** La branche ajoute
  `CHECKPOINT_BEGIN_RECORD = 252` et `CHECKPOINT_RECORD_V2 = 253`
  (`wal_record.h:41-42`), un second fichier `<base>.wal.checkpoint`, et un
  nouvel en-tête de fichier fantôme. Un journal écrit par eux fait lever notre
  `default:` ; un journal écrit par nous, terminé par l'ancien marqueur sans
  fichier fantôme, est refusé par eux (`wal_replayer.cpp:164-165`). Pas de
  longueur par enregistrement, pas de numéro de séquence, d'un côté comme de
  l'autre.

**Mon estimation, honnête :**

- **La branche entière : 5 à 8 jours.** Deux jours de conflits, deux à trois
  pour réconcilier le rejeu du journal avec l'étape E et porter notre correctif
  des 4 096 octets dans leurs chemins, un à deux pour leurs 2 658 lignes de
  tests de point de reprise à faire tourner chez nous, et le reste pour le
  changement de format (une base existante doit se rouvrir).
- **`vela/master` seul : 2 à 3 jours**, sans leur reprise après panne — mais
  c'est une version dont ils corrigeaient encore la reprise trois mois plus
  tard.
- **Ce que je recommande : ne pas fusionner, prendre par le contenu.** Deux
  morceaux servent A : le remappage d'offsets (≈ 150 lignes, `local_storage.cpp`
  et `local_rel_table.cpp`) et le gestionnaire de transactions (verrou de point
  de reprise dédié, fil de fond). **2 jours.** Leur reprise après panne est un
  troisième morceau, utile, indépendant de la concurrence, à évaluer à part
  parce qu'il change deux formats.

Le chiffre du 3 septembre (une demi-journée, 55 fichiers sur 58) mesurait
`vela/master` contre notre arbre d'alors, avant nos propres travaux sur le
journal et le dépôt d'extensions, et avant que leur branche ne grossisse.

## 5. Les risques chez eux

- **Aucune détection de conflit ajoutée** (§2) : leur mode par défaut peut
  produire des doublons de clé primaire et des relations pendantes — *déduit*.
- **Reprise « à redémarrage »** : après un échec de point de reprise ou de
  commit, la base refuse tout jusqu'au redémarrage
  (`transaction_manager.cpp:27-34`, `wal.cpp:84-86`). Sûr, mais un démon qui
  tient la base doit savoir redémarrer.
- **Ce qu'ils testent bien** : la panne injectée à chaque frontière de
  durabilité (`CheckpointCrashTest.RecoversAfterProcessExit`, quinze scénarios),
  une trentaine de tests `…FailsClosed`.
- **Ce qu'ils ne testent pas** : un lecteur long au-delà du délai ; la famine
  des nouvelles transactions pendant l'attente ; le point de reprise
  automatique qui échoue en boucle sous ingestion ; la panne avec plusieurs
  écrivains actifs ; l'ouverture d'une base au nouveau format par un binaire
  plus ancien ; **tout conflit réel entre écrivains**.
- **Leur `master` est en retard de trois mois sur leur propre branche.**

## 6. Le premier cas de B, déjà rouge : le lecteur d'un autre processus

### Le lecteur rejoue le journal d'un écrivain vivant

`database.cpp:136` appelle `StorageManager::recover` sans condition sur la
lecture seule → `storage_manager.cpp:81-85` → `WALReplayer::replay`
(`wal_replayer.cpp:84`). Les gardes de lecture seule (`:556, 567, 575, 582`)
n'empêchent que de supprimer, synchroniser et tronquer. Le rejeu a lieu, dans la
mémoire du lecteur, par le chemin d'insertion normal — index de clé primaire
compris (`:386-408`). Donc oui : avant l'étape D, un lecteur rejouait nos
enregistrements abîmés.

### La course

Le lecteur fait trois lectures non atomiques, sans verrou :

1. `dryReplay` parcourt le journal et décide « le dernier enregistrement n'est
   pas un CHECKPOINT » (`wal_replayer.cpp:111-113`) ;
2. `readCheckpoint` lit le fichier de données (`:123`) ;
3. il rejoue le journal jusqu'au dernier COMMIT vu en 1 (`:137-141`).

L'écrivain, lui : CHECKPOINT journalisé, pages appliquées et synchronisées,
puis seulement troncature du journal (`checkpointer.cpp:158-162`,
`buffered_file.cpp:45-47`). S'il journalise et applique entre 1 et 2, le lecteur
charge un fichier qui contient déjà les transactions et les rejoue par-dessus :
**« Found duplicated primary key »**. C'est le rouge de
`LecteursConcurrents.CeQueLeLecteurVoitEstCoherent` sur le nouveau poste. Le
refus de pages fantômes ne dépend que de ce que 1 a vu ; il n'est jamais
revérifié.

**Vela a la même forme** (`wal_replayer.cpp:106-302` : `dryReplay` des deux
journaux, puis `readCheckpoint`, puis rejeu) et suppose qu'aucun écrivain ne
vit ; en lecture seule son `rollbackCheckpoint` sort sans rien faire
(`shadow_file.cpp:502`). Il ne traite pas cette fenêtre.

### Rectification de ma mesure du 18 septembre

Le 3 septembre le test exigeait que tout refus soit celui des pages fantômes,
et il passait, sur l'ancien poste. Le 18, je n'ai rejoué que
`LeRefusSeResoutParUneNouvelleTentative`, qui **compte les refus sans lire leur
message**. Le pic à 567 ms et le « 2/80 » du test Rust peuvent avoir été cette
course. Le titre de `20a8f6ee8` (« une hypothèse sur l'ordonnanceur ») n'est
plus soutenu.

### Deux conséquences *déduites, non exécutées*, et le test qui les tranche

- **Des relations en double chez le lecteur, sans erreur.** La même course avec
  des enregistrements sans clé primaire ne lève rien. *Test* : un crochet qui
  arrête le lecteur entre `dryReplay` et `readCheckpoint` ; l'écrivain valide
  des relations puis fait son point de reprise ; le lecteur reprend ; on compte
  les relations. Attendu rouge : le double.
- **Un mélange d'anciennes et de nouvelles pages** pour un lecteur qui lit
  pendant la boucle d'application (`shadow_file.cpp:71-78`, page par page, sans
  barrière). *Test* : un crochet dans `applyShadowPages` après la première page
  — Vela en a un, `setPhaseHookForTesting` — puis ouverture d'un lecteur et
  vérification d'un invariant porté par deux pages.

### La revérification : suffisante à l'ouverture, pas pour un lecteur qui reste

**À l'ouverture, oui.** Relire l'identité du journal (taille, premier octet
après le dernier COMMIT vu, présence d'un CHECKPOINT) après avoir lu le fichier
de données, et recommencer si elle a changé. **Une marche d'un jour**, avec le
test déterministe ci-dessus, rouge avant.

**Pour un lecteur qui reste ouvert, non — et rien ne le protège aujourd'hui.**
Lu dans notre code :

- après l'ouverture, le lecteur ne relit jamais l'en-tête ni le catalogue ; ses
  plages de pages, ses statistiques et son nombre de pages sont figés
  (`checkpointer.cpp:203-221`, `file_handle.cpp:44-45`) ;
- un point de reprise **réécrit en place** les segments de colonnes qui tiennent
  dans leurs pages (`column.cpp:479-527`), les pages de tableaux sur disque, les
  en-têtes d'index ;
- les pages libérées par un point de reprise redeviennent allouables **à la fin
  de ce même point de reprise** (`free_space_manager.cpp:194-202`), et un `COPY`
  peut y écrire directement ;
- aucune page de données n'a de somme de contrôle ni de version sur disque ; le
  cache du lecteur ne sait pas qu'une page a changé ;
- **l'en-tête de base ne porte aucune époque** (`database_header.h:12-18` : deux
  plages de pages et un identifiant fixe) ; une lecture courte en fin de
  fichier est acceptée en silence (`local_file_system.cpp:380-388`).

| un lecteur ouvert avant un point de reprise, puis… | classement — *déduit* |
|---|---|
| requête sur une table non modifiée | sûr, mais figé à l'ouverture |
| table dont des lignes ont été mises à jour en place | **faux en silence possible** |
| deux points de reprise avec suppressions puis insertions | **faux en silence possible** (pages réallouées, décodées avec les anciennes métadonnées) |
| lecture pendant l'application des pages | **faux en silence possible** |

`lecteurs_concurrents_test.cpp` rouvre la base à chaque lecture : il n'a jamais
couvert ce cas.

**Ce qu'il faut pour le fermer : une époque.** Un compteur écrit par l'écrivain
— impair pendant qu'il applique, pair sinon — que le lecteur lit avant et après.
Une propriété rend la chose simple : entre deux points de reprise, l'écrivain
n'écrit que dans des pages que l'état du dernier point de reprise ne référence
pas (pages libres ou neuves) et dans le journal. Un lecteur dont la requête
commence et finit dans la même époque paire lit donc un instantané cohérent ;
si l'époque a bougé, il jette son résultat et rouvre. **Trois jours**, test
rouge avant par les mêmes crochets. C'est le protocole du lecteur de B — et il
ne demande rien de A.

## 7. La cible A : plusieurs écrivains dans le processus

### Ce qui suppose un écrivain unique, ce que Vela règle, ce qui reste

| # | hypothèse d'écrivain unique | où | Vela | reste à écrire |
|---|---|---|---|---|
| 1 | le second écrivain est refusé | `transaction_manager.cpp:35-39` | réglé (réglage à vrai) | l'allumer après 3 à 6 |
| 2 | offsets locaux de nœuds qui se chevauchent entre transactions | `local_node_table.cpp:32` | **réglé** (remappage au commit) | le reprendre, ≈ 150 lignes |
| 3 | unicité de clé primaire jugée sur l'instantané de la transaction | `node_table.cpp:392-403`, `in_mem_hash_index.h:284-296` | non | validation au commit contre le dernier état validé |
| 4 | suppression d'un nœud contre insertion d'une relation vers lui | `delete_executor.cpp:35-39`, `rel_table.cpp:324, 409` | non | validation au commit des extrémités |
| 5 | chemin de suppression sans verrou | `node_group.cpp:370-379`, `chunked_node_group.cpp:392` | non | verrou ou initialisation atomique |
| 6 | horloge : deux entiers non atomiques | `transaction_manager.h:70-71` | inchangé — protégée par le verrou de commit | rien pour A ; une interface pour B (§8) |
| 7 | offset de nœud dérivé au commit, pas réservé | `node_table.cpp:608` | inchangé — sûr parce que les commits passent un par un | rien pour A |
| 8 | `LocalStorage`, `UndoBuffer`, `VersionRecordHandler` non sûrs entre fils | annotations | inchangé | rien : ils sont par transaction, et une transaction écrit depuis un fil |
| 9 | journal : un verrou, un fichier | `wal.cpp` | inchangé | rien pour A |
| 10 | le point de reprise attend tout le monde en tenant le verrou public | `transaction_manager.cpp:57, 72-73, 119-137` | amélioré (verrou dédié, fil de fond) ; **attend toujours tout le monde** | le prendre ; le vrai « sans blocage » est un autre chantier |
| 11 | ramassage des versions au seul point de reprise | `chunked_node_group.cpp:125-132` | inchangé | rien pour A |
| 12 | index d'extension (HNSW) sous plusieurs écrivains | `extension/vector` | non étudié | **non étudié** |

### Les marches, chacune testable seule

| marche | ce qu'elle fait | le test qui la prouve | si elle est fausse | coût |
|---|---|---|---|---:|
| **A1** | le banc de concurrence et le vérificateur d'intégrité (§10), **avant tout code** | il est rouge aujourd'hui sous `debug_enable_multi_writes` sur les cas 3 et 4 — ce qui tranche les deux *déduits* du §2 | — | 4 à 6 j |
| **A2** | remappage des offsets locaux au commit (repris de Vela) | deux fils créent chacun des nœuds puis des relations entre eux ; vérifier chaque extrémité | **silencieux** : relations vers de mauvais nœuds | 1 j |
| **A3** | unicité de clé primaire validée au commit, sous le verrou de commit, contre le dernier état validé ; le second arrivé échoue | deux fils insèrent la même clé ; exactement un réussit ; `count = count(distinct)` après point de reprise et réouverture | **silencieux** avant, erreur franche après | 2 j |
| **A4** | intégrité des relations validée au commit : une extrémité supprimée entre-temps fait échouer l'insertion ; une suppression échoue s'il existe une relation validée | un fil supprime un nœud, l'autre y accroche une relation ; aucune relation pendante | **silencieux** avant, erreur franche après | 3 j |
| **A5** | le chemin de suppression rendu sûr entre fils | le banc sous ThreadSanitizer | comportement indéfini, non déterministe | 2 j |
| **A6** | allumer le mode par défaut, renommer le réglage | les neuf tests amont sans le réglage de débogage | erreur franche | 0,5 j |
| **A7** | le point de reprise laisse les transactions ouvertes se fermer ; le point de reprise automatique passe en fil de fond (repris de Vela) | un point de reprise demandé pendant une transaction ouverte réussit quand elle se ferme ; le commit déclencheur ne bloque pas | erreur franche (délai) | 2 à 3 j |
| **A8** | *(optionnel, à part)* le second écrivain peut **attendre** au lieu de démarrer en parallèle, pour qui veut l'ordre | §9 | erreur franche | 0,5 j |

**Total : environ trois semaines.** A2, A3 et A4 ferment des corruptions
silencieuses ; elles passent avant A6.

**Ce que A ne donne pas** : un point de reprise qui ne bloque pas les lecteurs.
Ni Vela ni l'amont ne l'ont. Il demande un point de reprise entièrement hors
place, des instantanés épinglés et une libération de pages retardée jusqu'au
départ du plus vieux lecteur. Deux à quatre semaines, **que je ne sais pas
chiffrer mieux**, et à ne pas faire dans le processus si l'on va jusqu'à B (§8).

## 8. La cible B : plusieurs processus écrivains

### Ce qu'elle demande en plus

Entre deux points de reprise, les données validées vivent **dans la mémoire du
processus qui les a écrites** : groupes de nœuds en mémoire, index local,
versions, catalogue. Un autre processus ne les voit pas. C'est le vrai mur, plus
que l'horloge.

| besoin | pourquoi |
|---|---|
| voir les commits des autres | l'état non encore passé au point de reprise est en mémoire, par processus |
| un ordre total des commits | l'instantané repose sur `lastTimestamp`, local |
| une allocation de pages commune | la liste des pages libres est en mémoire, par processus ; un `COPY` écrit directement dans le fichier |
| un point de reprise coordonné | il réécrit des pages en place sous les autres |
| un cache cohérent | aucune invalidation n'existe |

### Les formes possibles

**Forme 1 — tout l'état en mémoire partagée.** Caches, versions, index, verrous
dans un segment partagé. Les structures du moteur sont des conteneurs C++ avec
des pointeurs ; les y loger revient à réécrire le stockage. **Des mois, le
risque de corruption silencieuse le plus élevé. Je la déconseille.**

**Forme 2 — le journal partagé, que chaque processus rejoue.** *C'est celle que
je recommande.* Le journal devient la seule vérité entre deux points de
reprise. Pour valider, un processus : prend un verrou inter-processus ;
**rattrape** en rejouant dans sa mémoire les enregistrements que les autres ont
ajoutés depuis sa dernière position ; valide ses conflits contre cet état à
jour ; ajoute ses enregistrements ; relâche. Le code de rattrapage existe : le
rejeu applique déjà chaque transaction du journal par les chemins normaux,
index compris (`wal_replayer.cpp:203-218, 386-408`) — aujourd'hui seulement à
l'ouverture.

- *l'ordre total* tombe du journal : la position d'un COMMIT est son estampille ;
- *les offsets* sont attribués au commit, sous le verrou, sur l'état rattrapé —
  le code de A, inchangé ;
- *le cache* reste juste entre deux points de reprise, puisque l'écrivain n'écrit
  alors que dans des pages non référencées ; au changement d'époque (§6) chacun
  vide son cache et recharge ;
- *le point de reprise* : un seul processus à la fois, sous le verrou ; les
  autres rouvrent ;
- *l'allocation de pages* : par extension du fichier sous un compteur commun ;
  la réutilisation des pages libres reste au seul point de reprise ;
- *`COPY`* force déjà un point de reprise (`client_context.cpp:523`) et n'est pas
  rejoué (`wal_replayer.cpp:531-533`) : il devient « un point de reprise, tout
  le monde recharge ».

Un petit segment partagé suffit : le verrou de commit, l'époque, la position
validée du journal, le compteur d'allocation, le registre des lecteurs et
écrivains vivants. C'est la forme du mode journal de SQLite — plusieurs
processus, des commits sérialisés, des lecteurs qui ne bloquent pas l'écrivain.

*Où elle peut corrompre en silence* : un processus qui valide sans avoir
rattrapé ; une page allouée deux fois ; un cache non vidé au changement
d'époque. Les trois se testent par le banc de A lancé avec des processus.

**Forme 3 — un seul processus qui tient la base, les autres sont ses clients.**
C'est `rag3daemon`. Avec A, elle donne déjà des écritures parallèles à
plusieurs clients.

**Forme 4 — plusieurs fichiers, un écrivain par fichier.** §9.

### A est-elle sur le chemin de B ?

**Oui pour la correction, non pour le point de reprise.**

| de A | dans B (forme 2) |
|---|---|
| A2 remappage au commit | **réutilisé tel quel** |
| A3, A4 validation des conflits au commit | **réutilisées telles quelles** — c'est précisément ce qu'un processus fait après avoir rattrapé |
| A1 banc et vérificateur | **réutilisés**, des fils remplacés par des processus |
| A5 sûreté entre fils | réutilisée, sans plus |
| le verrou de commit | **à refaire** : mutex de processus → verrou inter-processus |
| l'horloge | **à refaire** : compteur en mémoire → position dans le journal |
| l'allocation de pages | **à refaire** |
| A7 point de reprise | **à refaire** : « attendre les transactions du processus » → « attendre celles de tous », par le registre partagé |
| un point de reprise sans blocage *dans le processus* | **non réutilisable** : ses instantanés épinglés vivraient en mémoire locale |

**Ce qu'il faut concevoir pour B dès A, pour ne rien construire deux fois :**

1. **Le verrou de commit et l'horloge derrière une interface** (un
   « séquenceur » : prendre le tour, obtenir l'estampille, rendre le tour) —
   aujourd'hui un mutex et deux entiers ; demain un verrou partagé et la
   position du journal. À poser pendant A3. Coût : négligeable.
2. **L'époque du §6 dans le segment partagé ou dans l'en-tête dès le départ**,
   pas dans la mémoire de l'écrivain.
3. **Ne pas bâtir le point de reprise sans blocage dans le processus.** S'en
   tenir à A7.
4. **Écrire le banc de A paramétré par « fil ou processus ».**

### Mon avis net sur B

B n'apporte au produit que trois choses que A plus un processus serveur
n'apportent pas : **pas de démon obligatoire** (chaque outil ouvre le fichier),
**l'isolement des pannes** (une ingestion qui meurt ne tue pas le service), et
**un processus d'ingestion distinct du processus qui sert**. Les lecteurs d'un
autre processus, eux, viennent de la marche d'époque (§6), sans B ; et les
écritures lourdes parallèles viennent de la forme 4, sans B. **B reste le plus
gros chantier de ce rapport, et le seul dont trois morceaux m'échappent.** Je ne
contredis pas la recommandation « A d'abord, B s'il reste un besoin » ; je la
précise : A, la marche d'époque et les fichiers par genre d'index couvrent le
besoin décrit — un agent qui déclare des sources, une ingestion lourde en fond,
des recherches pendant ce temps.

**Estimation de B (forme 2) : deux à trois mois.** Séquenceur et segment
partagé, 1 semaine ; rattrapage par le journal, catalogue compris, 2 à 3
semaines ; allocation de pages commune, 1 à 2 semaines ; point de reprise
coordonné et rechargement, 2 semaines ; banc multi-processus et pannes
injectées, 2 semaines.

**Ce que je ne sais pas chiffrer :** l'index HNSW, dont l'état en mémoire par
processus n'est pas dans le journal sous une forme que j'ai lue ; le coût du
rattrapage quand un processus a beaucoup de retard ; le comportement de l'index
de hachage local sous rattrapage.

## 9. Plusieurs fichiers, un écrivain par fichier

La piste de Lucie : ne pas tout mettre dans un fichier ; la découpe par genre
d'index, indépendante des sources.

### Ce que le moteur sait déjà faire avec plusieurs fichiers : presque rien

- **Un seul fichier de données par base**, chez nous, en amont et chez Vela
  (`storage_manager.h:50, 92` : un unique `dataFH`). Aucun stockage par table.
- **`ATTACH` d'une base native ne fédère pas, il remplace.** La base attachée est
  ouverte en lecture seule (`attached_database.cpp:55`), son journal doit être
  vide (`:22-34`), et `ATTACH` fait d'elle la base courante
  (`attach_database.cpp:37`) : la principale n'est plus interrogeable jusqu'au
  `DETACH`. **Dès qu'une base native est attachée, plus aucune écriture n'est
  permise, même dans la principale** (`client_context.cpp:597-610`). Un nom de
  table n'a pas de qualificatif de base : une requête ne voit qu'une base. Pas
  de relation entre deux bases, pas de transaction sur deux bases. Identique à
  l'amont et à Vela.
- **Ladybug va plus loin en lecture, pas en écriture.** Les noms qualifiés
  `base.Table` existent (`bind_graph_pattern.cpp:969-980`), sans remplacement
  de la base courante ; mais une base attachée reste non inscriptible, et une
  relation ne traverse pas deux bases. Leur partitionnement donne bien **un
  fichier par partition**, avec **un seul catalogue, un seul gestionnaire de
  transactions et un seul journal** (`wal.cpp:271-273`) : atomique, mais un
  seul écrivain pour l'ensemble. Environ 1 500 lignes propres et une vingtaine
  de sites d'appel — et ce n'est pas « un écrivain par fichier ».

**Le moteur n'offre donc rien pour « plusieurs fichiers, chacun son écrivain,
interrogés ensemble ».** La réunion se fait au-dessus de lui.

### La découpe par genre d'index

Le graphe — lignes et relations — dans un fichier ; chaque index dérivé dans le
sien, avec son propre écrivain.

- **Le plein texte et le sparse y sont déjà.** Ils vivent en Rust, dans le
  processus, hors du cœur C++ (`extension_config.cmake` le dit en commentaire).
- **L'index vectoriel peut vivre dans une autre base rag3db**, à une condition
  lue dans notre extension : **le HNSW doit être dans le même fichier que la
  table qu'il indexe.** Son graphe est rangé dans deux tables de relations
  internes du même catalogue (`create_hnsw_index.cpp:164-173`), et l'entrée
  d'index est rattachée à la table par son identifiant. Notre greffe exige la
  même chose : `vector_search` cherche l'index dans le catalogue courant et rend
  des offsets de nœuds de cette table (`vector_search_function.cpp:70, 149-152,
  193`), que l'opérateur de scan relit dans la même table
  (`index_scan_node_table.cpp:23-36`). Donc : **une table mince
  `(uuid, embedding)` dans une base à part**, écrite par un autre processus ; la
  descente de prédicat y fonctionne entièrement et rend `uuid` et score.
- **Ce qu'on perd** :
  - la jointure par `uuid` vers le graphe se fait hors du moteur ;
  - un filtre sur une propriété du graphe ne peut plus descendre avec la
    recherche vectorielle, sauf à recopier cette propriété dans le fichier de
    l'index ;
  - l'atomicité entre une ligne et son vecteur — acceptée par construction,
    l'index étant une donnée dérivée dont rag3weaver tient déjà le retard ;
  - une lecture simultanée cohérente des deux fichiers : ils sont lus à deux
    instants.
- **Coût côté moteur : à peu près nul.** Chaque fichier est une base ordinaire à
  écrivain unique. Le travail est dans rag3weaver.
- **Une conséquence à ne pas manquer** : plus de fichiers lus par d'autres
  processus que celui qui les écrit, c'est plus de lecteurs exposés à la course
  du §6. **La marche d'époque devient un préalable de cette découpe**, pas une
  amélioration.

### Les deux autres découpages

**Une base par source, fédérée au-dessus.** Le moteur le permet sans rien
changer : ce sont des bases indépendantes. Ce qui devient impossible dans le
moteur : une relation entre deux sources (à porter par `uuid` déterministe,
au-dessus), une requête Cypher qui traverse, une transaction qui touche deux
bases. Les index sont par base ; fusionner des « K meilleurs » de plusieurs
bases est simple au-dessus. Coût moteur : nul. Coût réel : chez rag3weaver.

**Une base, des fichiers par table, un verrou d'écrivain par fichier, un
catalogue commun.** Personne ne l'a : Ladybug a les fichiers sans les écrivains.
Un catalogue commun à plusieurs écrivains exige une horloge et un journal
communs — **c'est B sous un autre nom**, avec en plus un stockage à réécrire.
**Je la déconseille.**

### Sa place par rapport à A et B

- **La découpe par genre d'index est un vrai palier, pas un détour.** Elle
  donne dès maintenant ce pour quoi on regardait B — des écritures lourdes en
  parallèle, dans des processus distincts, pendant qu'on cherche — sans
  partager ni horloge, ni allocateur, ni cache.
- **Le fichier du graphe à écrivain unique suffit-il ?** Oui, comme palier, et
  je le crois durable : les écritures de lignes sont courtes, le lourd est dans
  les index, et A donne plusieurs transactions parallèles à l'intérieur de ce
  seul écrivain. **Elle repousse B plutôt qu'elle ne la remplace** dans un seul
  cas : s'il faut que plusieurs *processus* écrivent des lignes du graphe. Le
  besoin décrit ne le demande pas.
- **Elle ne dispense de rien dans A** : deux transactions concurrentes sur le
  graphe se disputent toujours des clés et des nœuds.

## 10. Mettre les écrivains en file

**Dans un processus : une demi-journée.** Le refus est à
`transaction_manager.cpp:35-39`. Le remplacer par une attente sur une variable
de condition signalée à la fin de chaque transaction d'écriture, avec un délai
au-delà duquel l'exception actuelle part. Vela a déjà la variable de condition
et le compteur (`cvActiveTransactionsChanged`, `activeWriteTransactionCount`).
C'est A8 : utile à qui veut l'ordre plutôt que le parallélisme, inutile une
fois A6 allumée.

**Entre processus : ce n'est pas le « gain de confort peu coûteux » qu'annonçait
le document du 6 septembre.** Le verrou n'est pas pris par transaction mais **à
l'ouverture de la base** (`storage_manager.cpp:40-56`, appelé une fois depuis
`checkpointer.cpp:195`) et tenu jusqu'à la fermeture. Passer `F_SETLK` en
`F_SETLKW` (`local_file_system.cpp:142`) ferait attendre le second processus
jusqu'à ce que le premier **ferme sa base** — jamais, pour un démon. Une file
entre processus n'a de sens que pour des écrivains courts (ouvrir, écrire,
fermer, chacun payant un rejeu et un point de reprise). Pour un verrou par
transaction il faut que le second processus voie ce que le premier a écrit :
c'est B.

## 11. Les garde-fous

**Ce qui existe.** Neuf tests `Concurrent*` en amont et chez nous
(`transaction_test.cpp`), tous sur des clés disjointes ; les `WWConflict*` des
fichiers `.test`, séquentiels (deux connexions, un fil) ; chez Vela, cinq tests
de plus dont deux de concurrence, toujours sur clés disjointes, et quinze
scénarios de panne injectée. Chez nous, `lecteurs_concurrents_test.cpp`, à
réouverture par lecture.

**Ce qu'il faut écrire avant de toucher au code (A1).**

1. **Un banc à invariants**, pas à résultats attendus : N fils (puis N
   processus) qui se disputent exprès les mêmes clés et les mêmes nœuds —
   virements entre comptes (la somme ne bouge pas), insertions de clés tirées
   dans un petit domaine (`count = count(distinct)`), suppressions de nœuds
   contre insertions de relations (aucune extrémité absente). Chaque invariant
   est revérifié trois fois : à chaud, après point de reprise et réouverture,
   après arrêt brutal et rejeu du journal.
2. **Un vérificateur d'intégrité**, appelable (`CALL check_integrity()`) :
   chaque ligne visible a une et une seule entrée dans l'index de clé primaire ;
   chaque relation a ses deux extrémités ; les deux directions d'une table de
   relations portent les mêmes arêtes ; les compteurs de lignes s'accordent avec
   les balayages. **C'est lui qui détecte une corruption silencieuse** : aucun
   test n'attrape un doublon qu'il ne cherche pas.
3. **Les crochets de test de Vela** (`setPhaseHookForTesting`, crochets de
   commit) pour rendre les courses déterministes : sans eux, un test de
   concurrence prouve seulement qu'on n'a pas eu de chance.
4. **Le banc sous ThreadSanitizer**, pour A5.

**Le signal d'alarme à garder en tête** : sur ce chantier, l'échec normal n'est
pas une erreur, c'est un résultat faux. Une marche n'est finie que quand son
test était rouge avant.

## 12. L'ordre proposé et les chiffres

| ordre | marche | coût | donne |
|---:|---|---:|---|
| 1 | lecteur : revérification à l'ouverture (§6) | 1 j | le test rouge redevient vert, pour la bonne raison |
| 2 | A1 : banc et vérificateur | 4 à 6 j | la preuve des deux corruptions *déduites* ; le filet de tout le reste |
| 3 | A2 → A6 | 8 à 9 j | **des écritures parallèles sûres dans le processus** |
| 4 | A7 | 2 à 3 j | un point de reprise qui n'échoue plus dès qu'une transaction est ouverte |
| 5 | lecteur : l'époque (§6) | 3 j | des lecteurs d'autres processus corrects même s'ils restent ouverts |
| 6 | index lourds dans leurs propres fichiers (§9) | côté rag3weaver surtout | des écritures lourdes parallèles entre processus |
| 7 | B, forme 2, **s'il reste un besoin** | 2 à 3 mois | plusieurs processus écrivains sur un fichier |

Les lignes 1 à 5 font **quatre à cinq semaines**. Rien n'est à coder avant que
Lucie ait vu ce plan.

## 13. Ce qui n'est pas vérifié

- **Rien n'a été compilé ni exécuté.** Tout vient de la lecture.
- **Le doublon de clé primaire et la relation pendante sous deux écrivains** :
  déduits ; A1 les tranche.
- **Les relations en double et le mélange de pages chez un lecteur** : déduits ;
  les tests du §6 les tranchent.
- **Le classement des scénarios du lecteur resté ouvert** : déduit.
- **L'index HNSW sous plusieurs écrivains, dans un processus comme entre
  processus** : non étudié. C'est le trou le plus important de ce rapport pour
  A6 — l'extension a ses propres hypothèses, et notre greffe de suppression par
  lots (`finalizeDelete`) en fait partie.
- **Conflit suppression contre mise à jour de la même ligne ; `COPY` concurrent
  à un commit** : non examinés.
- **Les changements de Vela dans `node_group*.cpp` et `csr_node_group.cpp`** :
  survolés.
- **La compatibilité du fichier fantôme** entre leur format et le nôtre : non
  étudiée.
- **Les estimations** sont les miennes, à la lecture ; aucune n'a été confrontée
  à un essai.
