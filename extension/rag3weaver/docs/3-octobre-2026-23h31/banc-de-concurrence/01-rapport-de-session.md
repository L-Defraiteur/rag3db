# Banc de concurrence — rapport de session

Session « banc » (`rag3db-76`, anciennement `rag3db-19`). Mis à jour le 4 octobre 2026 après-midi, après les correctifs du planificateur et des
amonts (`8c83c3360`, puis le second lot).

## Où en est le banc

- **Sur master**, tout est fusionné. Derniers commits de la session : `67c910f0d` (cas
  longs), `a10ee1c51` (témoins des amonts), poussés en `6fe2e4fc0`.
  Chaque lot est entré en avance rapide, sans force, et la comparaison `known_red` était
  verte à chaque fois.
- **Code** : `test/transaction/concurrence/`, une seule cible, `concurrence_test`.
- **Spécification** : `docs/2-octobre-2026-00h36/01-specification-du-banc-de-concurrence.md`,
  §1 à §13 ; chaque lot y a sa section.
- **Liste des rouges** : `known_red.txt`, 87 rouges connus rangés par marche, chacun avec
  l'ensemble exact des étiquettes de son échec. `probabilistic.txt` : 6 cas. `long.txt` :
  12 cas hors de la passe par défaut.
- **Moteur** : jamais modifié par cette session.

## Ce qui a été fait, dans l'ordre

1. **Le banc et son vérificateur (étapes 1 à 3, 2 et 3 octobre).**
   - Le harnais : un scénario, des fils ou des processus, une barrière, un journal
     d'événements.
   - Le vérificateur : niveau 1 par Cypher, niveau 2 par les internes.
   - Trois corruptions déduites par le plan se sont révélées réelles : C1, la clé en
     double ; C2, la relation pendante ; C3, les relations rattachées aux mauvais nœuds.
   - C4 était un défaut du banc. C6 est un défaut du moteur, reproduit sans concurrence.
   - Première passe ThreadSanitizer.
2. **Les remarques de la relecture du cœur C++.**
   - Chaque rouge est épinglé à sa raison, par ses étiquettes `[check: …]`.
   - Un faux vert du niveau 1 est fermé : le balayage force son sens par `HINT`, vérifié
     par `EXPLAIN`.
   - Nouveaux cas : C1 avec un instantané antérieur au commit de l'autre, C2 avec la
     destination supprimée et avec `DETACH DELETE`, deux colonnes d'une même ligne.
   - La passe TSan par signatures, dans son propre build.
3. **Les cas sur une table indexée par HNSW.**
   - L'invariant de l'index et l'exécution isolée dans un processus fils.
   - H1 à H4, et l'index construit sur cent lignes qui perd le nœud 13.
4. **Les témoins des verrous, écrits avant les verrous** (§6 de la note du 3 octobre),
   avec les noms d'erreurs convenus avec le cœur C++, et le nœud-carrefour comme
   garde-fou vert.
5. **La correction de la variante Crash, et son audit.**
   - Le processus fils fermait sa base avant le SIGKILL : la variante ne rejouait aucun
     journal.
   - L'audit a nommé ce qui change, et les phrases à retirer sont corrigées par des notes
     datées.
   - Avec un vrai arrêt, une clé en double rend la base impossible à rouvrir (A3′).
6. **La famille « arrêt brutal, un seul écrivain »** : les correctifs de reprise livrés
   tiennent sous un vrai SIGKILL. Elle a fait sortir un défaut du mode en service : une
   base indexée qui fait planter le processus qui l'ouvre.
7. **La reprise avec un index d'extension**, sur la condition élargie par le cœur C++ :
   les attendus des gardes 1 et 2, et une seconde mort. La garde 1 est livrée.
8. **L'index vectoriel juste en service (4 octobre)**, témoins écrits avant le correctif du
   cœur C++, par une autre main (`vector_index_update_test.cpp`) :
   - Ses chiffres ont d'abord été reproduits hors du harnais : ils n'étaient pas
     reproductibles à l'unité, parce que le compte varie d'une passe à l'autre. Nous
     sommes convenus d'un seuil « toutes les lignes justes », sur des essais répétés.
   - Deux contrôles. Toute ligne est joignable par une recherche exhaustive, et chaque
     ligne sort première sur son propre vecteur. Le second voit des pertes que le premier
     ne voit pas.
   - Les cas : sa recette, plus 768 dimensions, dix mille lignes, une ligne mise à jour
     cinquante fois, mise à jour puis suppression, transaction annulée, ligne lointaine.
     S'y ajoute le chemin du produit, décrit par la session des embarquements.
   - Le témoin déterministe du cœur C++ sur les relations relues de travers
     (`uncommitted_relations_test.cpp`) : rouge avant `c8fdaf196`, vert après.
9. **Les cas longs hors de la passe par défaut (4 octobre)** : `long.txt`, label
   `concurrence-long`, comparés seulement avec `CONCURRENCE_LONG=1`. La comparaison par
   défaut repasse de 14 min 30 à 2 min 17 ; la longue dure 29 min.
10. **La revue des amonts (4 octobre)** : Ladybug et Vela lus en lecture seule, chaque
    correctif présenté comme manquant reproduit chez nous avant d'être rangé. Vingt défauts
    confirmés, 21 témoins rouges (`upstream_fixes_test.cpp`). Les deux défauts graves ont
    été signalés dès leur confirmation. Tableau : `03-revue-des-amonts.md`.
11. **Les tickets (4 octobre)** : `docs/tickets/`, un fichier par défaut connu, demandé par
    Lucie ; la convention et l'index dans son `README.md`. Les 22 de la revue y sont, et
    les autres sessions y écrivent les leurs.
12. **Les correctifs faciles, écrits par le banc (4 octobre)**, sur un périmètre levé par
    l'orchestration (planificateur, optimiseur, binder, UUID nul, puis `toString`, CSV,
    `MERGE`, accent grave) :
    - un défaut par commit, chacun lu sur Ladybug puis réécrit, le commit lu cité ;
    - le témoin quitte `known_red.txt` et le ticket passe à corrigé, dans le même commit ;
    - la liste C++ complète est verte avant chaque fusion.
    Treize sont corrigés :
    - premier lot (`ed2f9d5db` à `8c83c3360`) : l'UUID nul, `SKIP` sans `LIMIT`, le
      `WHERE` sur paramètres seuls, `WITH gate`, l'`OPTIONAL MATCH` doublé, l'étiquette
      ignorée, le lambda, la comparaison de relations, l'`UNION` ;
    - second lot (`27bff7d9f`, `7fc1023d1`) : `toString`, le CSV qui finit par un champ vide ;
    - troisième lot (`d5ccf5b2d`, `462300b98`, après l'accord de Lucie) : le `MERGE` d'un
      motif retrouvé dans le lot, et le champ CSV `""` lu comme la chaîne vide (un
      changement de comportement, noté au journal).
    Le `MERGE` touchait la forme `batch_link` de rag3weaver : une relation répétée dans un
    lot perdait son `SET`. Le mécanisme a été compris avant d'être corrigé : la ligne en
    double portait l'identifiant de la dernière relation insérée. Trois défauts en
    sortent, et un seul commit les ferme.
    L'accent grave reste un ticket : 17 fichiers, hors du périmètre.
13. **L'essai déterministe de la corruption d'`e2e_code` (4 octobre au soir)** : six
    variations, sans rag3weaver. L'une reproduit à coup sûr, en Release et sans ASan :
    - la recette : un `COPY` refusé, puis `DROP_VECTOR_INDEX`, réouverture et
      `CREATE_VECTOR_INDEX` ; le processus meurt (SIGSEGV) avant `1ea49837f` ;
    - la cause : le `COPY` annulé gonfle la cardinalité (401 pour 200 lignes), et la
      création bornait son parcours par elle ;
    - les témoins (`e9b925c94`) : `CreateIndexAfterARefusedCopy`, vert depuis
      `1ea49837f` ; `RefusedCopyLeavesTheCardinalityTrue`, rouge, sous un nouveau ticket.
    Les cinq autres variations n'ont rien donné.
14. **La passe courte et les correctifs du `COPY` refusé (4 octobre, fin de journée)** :
    - la sonde du test fautif d'`e2e_code`, non commitée, écarte l'hypothèse pour ce test.
      À la réouverture, aucune table indexée n'a de cardinalité gonflée ; `Scope_Chunk` a
      211 lignes, soit la taille du tableau du rapport d'ASan, dans lequel le voisin 255
      n'existe pas. Il reste un voisin faux rendu par le graphe de l'index, à chercher ;
    - la session cœur C++ a trouvé deux défauts de plus derrière le `COPY` refusé : le point
      de reprise qui suit écrit hors bloc, et la clé d'origine d'un doublon sort de l'index.
      Corrigés en `05788a868`. Témoins au banc (`93f4e81e8`) :
      `RefusedCopyKeepsTheOriginalKey`, qui tuait le processus à la fermeture avant le
      correctif, et `CheckpointAfterARefusedCopy`, garde-fou vert en Release ;
    - les 18 passes sous ASan ne sont pas lancées, puisque l'hypothèse tombe.
15. **La stèle (4 octobre au soir, « on se focus sur la stèle », Lucie)** :
    - la relecture du tri des tickets du moteur, avec deux désaccords retenus par
      l'orchestration : le `CHECKPOINT` qui retient un lecteur passe à bloque (l'écrivain
      reçoit une erreur alors que sa ligne est validée, vérifié par une sonde) ; la
      cardinalité gonflée reste confort, à condition que `STATS_INFO` dise qu'il rend une
      estimation ;
    - deux tickets ouverts (`4909f4529`) : la mise à jour massive de vecteurs, et la ligne
      lointaine, encore rouge après `1ea49837f` ;
    - premier ticket « bloque » fermé : le point de reprise après `ALTER TABLE … DROP`
      (`308ebd17e`, lu chez Vela e5e700e73). Les deux témoins rougissent sans le correctif
      et passent au vert avec. **Un écart de méthode** : le commit a été poussé sur un
      empilement qui avait reçu `5c8507577` (rag3db-e3) après ma liste C++. La liste a été
      rejouée aussitôt sur master tel que poussé, et elle est verte.
16. **Les tickets « bloque » suivants (4 octobre, soirée)** :
    - le balayage de plusieurs tables de relations dans une transaction : fermé
      (`88d937160`) ;
    - l'erreur rendue sur une écriture validée (le `CHECKPOINT` qui retient un lecteur, le
      `COPY` validé qui rend une erreur) : fermé (`df689b522`, `68ff9d5e2`). Le gestionnaire
      de transactions attend sur une variable de condition sans tenir le verrou public, et
      un point de reprise au seuil se reporte au lieu d'attendre. Relu par la session cœur
      C++ ;
    - les arrêts au mauvais instant, deux lignes du ticket de durabilité (`a86d4e6a8`,
      `9e9296422`, relus par la session cœur C++, liste C++ verte). À la reprise, le journal
      est supprimé avant le fichier fantôme ; un journal clos par `CHECKPOINT` sans fichier
      fantôme est toléré ; les pages rejouées sont synchronisées avant toute suppression
      (environ 20 ms à la médiane, 45 ms au pire, sur btrfs, 200 000 lignes). Crochet de test
      `WALReplayer::setRecoveryHookForTesting`, deux points nommés. Témoins
      `RecoveryDeath.*`, `CheckpointDeath/AfterCheckpointLogged` et `/AfterJournalCleared`.
      Le ticket reste ouvert pour ses autres lignes ;
    - le témoin de la garde de double ouverture (57c8389b4) :
      `SecondDatabaseOnTheSamePathInOneProcessIsRefused`, vert sur le message nommé ;
    - la réouverture intermittente sous charge : fermée sous surveillance (`21ae489d9`).
      C'était la double ouverture dans un même processus, par le dernier `Arc<Database>`
      que lucivy lâchait sur un fil de fond (ad220d0eb).
17. **La fin de la soirée du 4 octobre** :
    - le lecteur refusé 1 à 3 fois sur 80 (`e2e_prise_atomique`) : le refus nommé
      `CHECKPOINT_CROSSED_READ_ONLY_OPEN`, déjà présent avant la garde et les remèdes
      (ancêtre `df689b522^` rebâti, 20 passes alternées, écart non établi). Témoin
      probabiliste `ReadOnlyReader`, ticket « lecteur affamé » (`fdf6d81c6`), confort ;
    - le témoin du COPY après des insertions de la même transaction (`7e4100576`), rouge
      jusqu'au refus de la session cœur C++ (`0f4a54b2c`), vert depuis ;
    - l'ordre de synchronisation au point de reprise : le doute tenait, pour une coupure.
      Le fichier de données est synchronisé avant la marque CHECKPOINT (`552443817`, relu
      par la session cœur C++), + 22 ms après 200 000 lignes, + 1,4 ms après une ligne ;
    - `STATS_INFO` dit qu'il rend des estimations (`d5fa21152`) ;
    - le COPY journalisé, cas 1 et 2 du §7 (`c45095c55`) : le cas 1 tient le tout ou rien
      mais fuit des pages (rouge connu `data-file-bounded`, sur l'ancien chemin aussi) ;
    - les propriétés d'une relation qui diffèrent selon le sens (CONSUMES, bloque la
      stèle) : base gardée lue par le stockage, 140 relations contiguës ; nouvel invariant
      `stored-rel-properties-agree` au niveau 2 et témoin générique `RelationPropertiesBothWays`
      (`c45095c55`). Cause trouvée par la session cœur C++ (le point de reprise d'une
      colonne de chaînes de relations) ; son correctif relu ; le témoin avant et après le
      point de reprise est rouge sans lui, vert avec, et part après son push.
18. **La reprise après la panne de mémoire (5 octobre, nuit)**, par une conversation neuve
    (`rag3db-36`), l'ancienne ne se rouvrant plus :
    - la panne de 22 h 51 venait du banc : `NodeListsKeepTheirStrings` comparait par
      `EXPECT_EQ` deux textes de 400 000 lignes, et gtest en calcule un diff quadratique
      quand il est rouge. Les deux vidages sont maintenant des lignes triées, comparées
      par `==` puis résumées par `compareDumps`, qui est borné ;
    - `ListOfStringsAcrossCheckpoints` vu rouge une fois, le correctif `25b3b45dc` retiré
      de `src/` dans l'arbre du banc, sous `poste` à 4 Go sans échange : 2 lignes sur
      400 000 changées sur les nœuds (au point de reprise et à la réouverture), 1 614
      valeurs changées sur les relations ; 2,4 s, sans pression de mémoire. Vert avec le
      correctif, comme `CheckpointKeepsEveryStoredString` ;
    - le tout est poussé en `eef731081`, après une comparaison `known_red` verte.
19. **La suite de la nuit du 5 octobre** :
    - les pages d'un COPY tué (ticket de durabilité, `a4442df7e`, `a517dcc36`) : la fuite
      n'est pas bornée. Chaque mort laisse 0,3 à 3,4 Mo, sans palier, et ni la réouverture
      ni le point de reprise ni les écritures suivantes ne les rendent. Mesuré par une sonde
      non commitée (`~/.cache/rag3db-banc-notes/sonde-fuite-copy.patch`). Classé confort par
      l'orchestration ; la déduction « reprises naturellement » de la page du chargement
      journalisé est réfutée ;
    - les témoins d'A3′ (`218a0fa8a`) trouvent plus grave que la limite écrite. En mode
      multi-écrivains, l'annulation d'un COPY copié le premier efface les **lignes** et les
      clés d'un COPY copié après lui dans le même bloc, que l'autre écrivain valide sans
      erreur. `Order/RollbackOfACopy.RemovesOnlyItsOwnKeys/RolledBackCopiedFirst` et
      `TwoCommitsThatEachWantACheckpointDoNotWaitForTheTimeout` sont rouges, stables, dans
      `known_red.txt` sous A3′. Ticket
      `2026-10-05-annulation-d-un-copy-efface-les-lignes-d-un-autre-ecrivain.md`, non
      atteignable en service ;
    - relecture croisée du correctif de la session cœur C++ (le COPY après des insertions de
      la même transaction, par versement du stockage local) : rien de bloquant. Deux cas
      proposés et écrits chez elle (une clé versée, supprimée puis réinsérée ; une mise à
      jour après le COPY rejouée). Son versement hérite du défaut d'A3′ ci-dessus, et c'est
      écrit dans son message ;
    - l'index des tickets : les propriétés selon le sens passent à corrigé (`25b3b45dc`).
20. **Après la fusion des deux commits de la session cœur C++ (`f1d8c7190`, `e1049934e`)** :
    - relu `e1049934e`, sans objection. Deux remarques : le filet compare au compte de la
      transaction, lignes locales comprises ; et la course sur `numCheckpointedNodes`,
      pour la passe TSan avec l'extension ;
    - `efd76bdc9` :
      - le filet témoigné par les internes, `IndexCountingMoreRowsThanItsTableIsRefusedByName` :
        un miroir de `HNSWStorageInfo` écrit le compte à 210 pour 200 lignes. Trois refus
        nommés, puis un rebâti juste ;
      - le remappage d'un versement après un autre écrivain : vert ;
      - `CopyAfterLocalInserts` exige le COPY accepté ;
    - `77ee3f986`, la reprise de rag3weaver après un paquet défait (`ProductReloadRecovery`) :
      - la forme : index cosinus de 64 dimensions posé d'avance, paquets de 32 par COPY
        journalisé, reprise dans un fils ;
      - aucun plantage, le SIGSEGV de l'arbre principal n'est pas reproduit ;
      - mais, après trois COPY annulés, de 1 à 9 lignes validées avant l'annulation ne
        sortent plus pour leur vecteur exact, 5 à 6 fois sur 10. C'est vrai aussi sans
        rejeu, après une fermeture propre. Sans annulation, vert 10 fois sur 10 ;
      - ticket `2026-10-05-annulation-d-un-copy-abime-l-index-vectoriel.md`, la cause à la
        session cœur C++ ; les deux cas sont probabilistes ;
    - `data-file-bounded` est passé vert une fois (rouge 4 fois sur 5) : il est déplacé de
      `known_red.txt` vers `probabilistic.txt`, sans correctif. L'écart à la règle est
      signalé à l'orchestration.
21. **La nuit du 5 octobre, suite** :
    - `ProductReloadRecovery` rendu presque constant par un seul fil (`45aca6dc1`) : 29 rouges
      sur 30, toujours sur la ligne 15. Le hasard qui reste est la graine de l'index.
      Une quatrième fin, sans annulation, est rouge 10 fois sur 10 : ce n'est pas
      l'annulation, c'est le graphe bâti par des COPY successifs. Mon hypothèse des arêtes
      de retour était fausse, la session cœur C++ l'avait établi dans un seul processus. Les
      tickets ont été réécrits par elle. La règle « jamais retirée en silence » est en tête
      de `known_red.txt` ;
    - un rouge isolé de `DeathDuringDeletesAndVectorSearchKeepsTheIndexExact`
      (`vector-extension-loaded`), le 5 octobre vers 1 h 20, sous une forte charge du poste :
      rejoué vert. S'il revient, le délai de mort doit dépendre de la préparation au lieu
      d'être fixe (décision de l'orchestration) ;
    - le seul COPY qui reste forcé, `ForcedCopyCheckpointDeath` (`8900138c8`), trois formes ×
      sept instants de mort du point de reprise de la validation. Douze rouges dans
      `known_red.txt`, bloque la stèle :
      - le COPY sous IGNORE_ERRORS qui écarte une clé en double rend la base inouvrable :
        sa suppression est journalisée sans son insertion ;
      - une transaction qui mêle des écritures et un COPY revient à moitié, même au réglage
        par défaut (105 lignes). Le COMMIT n'avait pas rendu la main ;
      - le correctif de la session cœur C++ pour le premier défaut (`9c0c6532d`) a été joué
        ici, appliqué temporairement : il ne suffit pas pour le second. Le remède retenu
        est qu'une transaction forcée n'écrive rien au fichier du journal ;
      - ticket `2026-10-05-transaction-a-moitie-apres-une-mort-pendant-son-point-de-reprise.md`.
22. **Le matin du 5 octobre** :
    - l'atomicité : la relecture de `82bb0b2c5` (rien n'est écrit au journal par une
      transaction forcée), sans objection ; le correctif est sur master (`37608cf4b`). Le COPY
      de relations qui écarte des lignes est ajouté au témoin, vert ;
    - le DROP d'une colonne qui en précède d'autres (ticket de la session cœur C++, confié au
      banc) :
      - la passe de lecture systématique a trouvé une dizaine d'endroits, dans deux fenêtres ;
        quatorze témoins rouges (`1e41b33a0`) ;
      - fenêtre A, entre le DROP et le point de reprise : une seule conversion,
        `TableCatalogEntry::getPropertyPosition` (`99059fb69`) ; puis la relecture
        indépendante de la session cœur C++, dont un défaut bloquant (`ff76b1ee3`) ;
      - fenêtre B, après le point de reprise : la clé et les index retrouvent leurs colonnes
        par le catalogue, au point de reprise et à l'ouverture. Une base abîmée par l'ancien
        moteur est gardée en témoin (`dataset/databases/stale-index-columns`). Commit prêt,
        en attente de relecture ;
    - le point de reprise sans fin après UN COPY annulé ou refusé (ticket
      `2026-10-05-point-de-reprise-sans-fin-apres-un-copy-annule.md`), trouvé en cherchant la
      seconde porte de l'index de clé qui enfle. Le témoin tourne dans un fils borné (en temps,
      tampon, réservation, `RLIMIT_DATA`), sans quoi la portée de `poste` emportait la passe.
      Le correctif de la session cœur C++ (`f3a581fb8`), joué ici, le verdit ;
    - la joignabilité « fond » contre « masse », sur les vrais vecteurs de l'arbre principal :
      pas d'écart au banc. Le premier compte était faussé par 159 lignes dans des groupes de
      plus de dix vecteurs identiques.

## Ce qui m'attend

- le push de la fenêtre B du DROP après la relecture de la session cœur C++ ;
- l'étape 4 du chargement journalisé : réécrire mes deux témoins de l'attente (2a) le jour du
  basculement, et sortir `TwoCommitsThatEachWantACheckpointDoNotWaitForTheTimeout` ;
- une graine réglable de l'index vectoriel, demandée à la session cœur C++ : elle rendrait
  `ProductReloadRecovery` déterministe ;
- la mesure du coût d'insertion demandée par la relecture de la fenêtre A (un million de
  CREATE sur trente colonnes, avant et après) : pas encore faite.

## Décisions et pourquoi

- **La raison du rouge est épinglée.** Sans cela, une marche qui échange une corruption
  contre une autre passe inaperçue (remarque de la relecture).
- **Un rouge probabiliste n'est jamais comparé.** Un test qui rougit une fois sur deux ne
  se lit jamais comme un vert.
- **La comparaison n'exécute pas les cas probabilistes**, parce qu'un plantage de C5
  emportait toute la passe.
- **Le second écrivain attend.** Les scripts y sont prêts (`writeInTurn`,
  `commitInOrderByEvents`) ; ceux que A4′ devra réécrire sont notés dans les cas : C5,
  C6_UpdateCommitsFirst, les trois C2_*RelationCommitsFirst.
- **Réouverture dans un processus neuf après chaque mort.** Un processus qui a déjà
  chargé l'extension vector la garde chargée, et cache le plantage d'un service qui
  redémarre.
- **Isoler le banc avant d'accuser le moteur, dans les deux sens.** Le banc a déjà
  accusé le moteur à tort (C4, les vecteurs mis à jour), et l'a aussi innocenté à tort
  (la fausse variante Crash).

## Ce qui attend quelqu'un

- **La session cœur C++** :
  - les marches V1, A3′, A4′ et V2 font passer les témoins des verrous au vert ; chaque
    marche retire ses lignes de `known_red.txt` dans son commit ;
  - la garde 2 retirera aussi les témoins de la garde 1 (`IndexToRebuildIsNamed`), qui
    deviendront rouges pour une autre raison ;
  - la question de ce que la reprise doit faire d'un journal qui porte un doublon (second
    témoin d'A3′) est chez l'orchestration ;
  - un crochet de test dans l'allocateur, si l'on veut la mort à chaque allocation pendant
    la phase de stockage d'un point de reprise. C'est non couvert ;
  - savoir si le graphe HNSW connaît la nouvelle position d'un vecteur mis à jour (le
    banc ne vérifie que ce que rend la recherche).
- **La session cœur C++, sur l'index vectoriel** :
  - la marche « mise à jour de l'index : contrôle en fin d'instruction » (quatre rouges
    dans `known_red.txt`, plus le ligne à ligne en probabiliste) ;
  - la ligne lointaine d'un nuage serré, sous la construction de l'index ;
  - **le ralentissement de la mise à jour depuis `c8fdaf196`** : dix mille lignes par
    lots de 512 passent de 25 s à 12–27 min. C'est signalé ; le cas est probabiliste (vert
    une fois sur sept essais).
- **La garde 2** est livrée (`15fcc3474`) : ses cinq rouges sont verts et retirés, et les
  `IndexToRebuildIsNamed` effacent la liste des extensions pour éprouver la garde 1.
  `IndexExactWithoutRebuild` exige désormais que la liste existe avant la réouverture
  (`9e3b03c63`).
- **Les tickets « bloque » de la stèle confiés au banc** : faits. Les propriétés selon le
  sens (CONSUMES) sont corrigées par `25b3b45dc` ; leurs témoins sont sur master
  (`eef731081`).
- **Les pages d'un COPY tué jamais récupérées** (ticket de durabilité, rouge connu
  `data-file-bounded`) : pour la session cœur C++, avec le chargement journalisé.
- **Le remède 2a** (l'attente du départ des autres avant la validation d'un `COPY`) se
  retirera avec le chargement journalisé ; la session cœur C++ préviendra avant de toucher
  à ses témoins.
- **La corruption d'`e2e_code`** : l'hypothèse du `COPY` refusé est écartée pour le test
  fautif ; la piste qui reste est un voisin faux dans le graphe de l'index (ticket de la
  corruption).
- **La stèle du moteur** (`docs/4-octobre-2026-16h57/01-la-stele-du-moteur.md`, §3.1) : le
  tri des tickets de la session cœur C++ attend ma relecture.
- **La cardinalité après un `COPY` refusé** (ticket
  `2026-10-04-copy-refuse-gonfle-la-cardinalite.md`) : pour la session cœur C++.
- **La session cœur C++, sur les amonts** : la perte de relations au point de reprise et
  la lecture après une relation créée puis supprimée sont corrigées (`80e3f2c32`). Restent
  sous la marche : le point de reprise après `ALTER … DROP`, le lecteur retenu par un
  `CHECKPOINT`, le balayage de plusieurs tables de relations dans une transaction.
- **L'accent grave dans un nom** : un correctif sur 17 fichiers (catalogue, export, un
  fichier de stockage), à attribuer.
- **Ce que la revue n'a pas couvert** :
  - deux recettes non reproduites (`254a7444d`, `a10cecbc7`) ;
  - dix défauts qui demandent une faute d'E/S, une coupure ou une mort placée au bon
    instant (§3 du tableau) ;
  - les défauts d'API sans témoin (§4).
- **TSan avec l'extension vector** : pas fait. L'extension se construit à un chemin fixe
  de l'arbre source, qu'un build TSan écraserait (§11).

## Comment reprendre

```bash
cd ../rag3db-banc                       # worktree du banc, branche locale banc-verrous
git fetch origin && git rebase origin/master
cmake -S . -B build/release -G Ninja -DCMAKE_BUILD_TYPE=Release -DBUILD_TESTS=TRUE \
  -DBUILD_SHELL=FALSE -DBUILD_BENCHMARK=OFF -DBUILD_EXTENSIONS=vector
cmake --build build/release -j 8 --target rag3db_vector_extension concurrence_test   # jamais l'alias ninja
ctest --test-dir build/release/test -R known_red --output-on-failure   # la comparaison seule, 2 min 17
CONCURRENCE_LONG=1 ctest --test-dir build/release/test -R known_red   # avec les cas longs, 29 min
ctest --test-dir build/release/test -L concurrence-rouge-connu -N      # les rouges connus
```

- La comparaison : `test/transaction/concurrence/compare_known_red.cmake`. Elle lance le
  banc entier sans les cas probabilistes, lit le JSON de gtest et compare les étiquettes
  de chaque rouge à `known_red.txt`. Elle garde le JSON d'une passe en échec
  (`…result.json.failed-<horodatage>`).
- Les passes se lancent en série (`ctest -j 1`), jamais deux en parallèle.
- La passe TSan : un build à part, `build/tsan` (§9 et §10 de la spécification), puis
  `ctest --test-dir build/tsan/test -L concurrence-tsan`.
- Après un rebase qui touche `src/` ou `extension/vector/`, rebâtir et rejouer la
  comparaison avant de pousser master.
