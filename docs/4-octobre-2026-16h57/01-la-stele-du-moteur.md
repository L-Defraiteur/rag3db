# La stèle du moteur — où l'on s'arrête d'y regarder

4 octobre 2026. Décision de Lucie :

> « Cette liste exactement définit un point d'arrêt de version pour le moteur :
> une stèle, pour savoir continuer sans y regarder encore ad vitam æternam. »

La liste : **les verrous, le chargement en masse journalisé, les écritures
parallèles**, et les défauts que les bancs ont trouvés. Quand elle est
tenue, le moteur est déclaré « assez bon » : on pose la version, et on n'y
revient que sur un défaut **constaté** dans le produit — plus par
précaution, plus pour l'améliorer.

Pourquoi : « le moteur nous aveugle sur la complexité ; c'est lui qui nous
prend beaucoup de temps à régler ; quand on l'aura bien fait, ce sera plus
simple de faire les autres sujets. » Sans point d'arrêt écrit, il absorbe
l'effort sans fin.

## 1. Ce qui doit être vrai

| # | Condition | Ce que « fait » veut dire | La preuve |
|---|---|---|---|
| 1 | **Les verrous** | le gestionnaire de verrous (V1 : ressource générique, genres ligne et index), puis A3′, A4′, V2 de la note de conception | les témoins du banc de concurrence (§6 de la note des verrous) verts |
| 2 | **Le chargement en masse journalisé** | un `COPY` n'impose plus son point de reprise ; ses lignes survivent à un arrêt brutal par le journal | arrêt brutal au milieu d'un chargement, processus neuf, comptes justes ; le premier index n'y perd plus de temps |
| 3 | **Les écritures parallèles** | plusieurs écrivains sur une même base, sans perte ni blocage, le mode allumé hors du banc | le banc de concurrence entier, vert, mode allumé ; deux agents qui écrivent en même temps dans un test du produit |
| 4 | **Plus de défaut connu qui corrompt ou qui perd** | chaque ticket de `docs/tickets/` qui touche la mémoire, la durabilité ou la justesse d'un résultat est fermé | le ticket fermé avec son témoin au banc |

Ce que la stèle **n'exige pas** : les défauts de confort (un message, un cas
de syntaxe contourné, une lenteur sans perte), les correctifs d'amont non
reproduits, toute optimisation. Ils restent en tickets et n'empêchent pas de
poser la version.

## 2. Où l'on en est (4 octobre, 17 h)

- **1, les verrous** : conçus (`docs/3-octobre-2026-15h47/01-note-de-conception-les-verrous.md`),
  témoins écrits au banc ; V1 pas commencée.
- **2, le chargement journalisé : fait, `ff9bad960` (10 octobre 2026).** Le `COPY`
  journalisé est le défaut du moteur : plus de point de reprise forcé par `COPY`, plus
  d'attente du départ des autres à sa validation, ses lignes durables par le journal ; au-delà
  de 256 Mio de journal par transaction, repli sur le point de reprise forcé ; l'indexation
  avec le plein texte en base demande elle-même le réglage forcé. Les étapes :
  `0f4a54b2c` (nœuds), `1cfba2d6a` (relations), `1177f5794` (statistiques dans la
  transaction), `1c232f318` (tableaux au journal en octets bruts), `71cffbc4b` (le repli),
  `fb98852e1` (transaction forcée sans journal en mémoire), `0aed3c4b5` (l'étendue du fichier :
  les pages d'un `COPY` tué ne sont plus perdues ; version de stockage 40). Preuves : la liste
  complète verte sous ce défaut, contrôle de fuite de pages compris ; la série de confirmation
  des embarquements (fichiers +1 %, blobs −2 %, 12 passes au calme). Pages :
  `extension/rag3weaver/docs/3-octobre-2026-23h31/coeur-cpp/04`, `05`, `06`.
- **3, les écritures parallèles** : T0, A2, A5, A5 bis livrées ; le mode
  reste éteint hors du banc ; la suite attend les verrous.
- **4, les défauts** : la corruption de mémoire d'`e2e_code` a sa cause et son correctif
  (`57c8389b4`, 4 octobre au soir) : deux exemplaires de la même base ouverts en écriture dans le
  même processus — rag3weaver rouvrait avant la fin de la fermeture, et le verrou du fichier
  n'exclut qu'un autre processus. Le moteur refuse maintenant la seconde ouverture par son
  nom ; rag3weaver doit encore rendre sa fermeture synchrone. En chemin, la traque a donné
  trois refus nommés à la place d'écritures hors bloc et l'index vectoriel dimensionné par
  le nombre de lignes (`1ea49837f`), et deux défauts d'origine de l'annulation d'un `COPY`
  (`05788a868`, `5c8507577`). Les tickets du moteur sont triés au §3.1.
- **4, l'index vectoriel (11 octobre)** : la mise à jour massive de vecteurs est fermée. Deux
  défauts en étaient la cause : un contrôle qui n'entrait pas dans le graphe comme une requête,
  et une réinsertion qui partait de la ligne elle-même (ticket fermé
  `closed/2026-10-04-mise-a-jour-massive-de-vecteurs-lignes-injoignables.md`). Deux témoins
  restent probabilistes : `TenThousandRowsInBatchesOf512`, jusqu'à sa mesure, et
  `TwentyRowsToTheSameVectorInOneStatement`, qui relève des groupes de copies.
  **La condition 4 n'est pas fermée.** Côté moteur, six tickets restent ouverts avec la gravité
  « perte » ou « réponse fausse » :
  - la recherche ne rend plus qu'une ligne après des annulations et des mises à jour
    (`2026-10-11-recherche-vectorielle-une-seule-ligne-apres-annulations-et-mises-a-jour.md`) ;
  - le CALL de la recherche rend des nœuds supprimés
    (`2026-10-11-query-vector-index-rend-des-noeuds-supprimes-au-call.md`) ;
  - la validation à moitié appliquée
    (`2026-10-10-validation-a-moitie-appliquee-sur-echec-apres-l-ajout-des-lignes.md`) ;
  - C7 après un arrêt brutal (`2026-10-10-c7-apres-arret-brutal-relations-pendantes-nees-au-rejeu.md`) ;
  - le COPY refusé pour mémoire, avec un filet côté rag3weaver
    (`2026-10-05-copy-refuse-pour-memoire-table-faussee-en-memoire.md`) ;
  - les grands groupes de copies (`2026-10-10-un-groupe-de-copies-bien-plus-grand-que-le-degre.md`).
  Les 21 autres tickets de même gravité sont côté produit (rendez-vous, codeparsers). Lucie dira
  s'ils comptent pour la stèle du moteur ou pour celle du produit.

## 3. Ce qui reste à faire pour que la stèle soit utilisable

1. **Trier les tickets** : une ligne par ticket du moteur — bloque la stèle
   (mémoire, durabilité, résultat faux) ou non. Fait au §3.1, à relire par le banc.
2. **L'ordre** : tranché au §3.2.
3. **Poser la version** quand les quatre conditions sont tenues : une
   étiquette git, la date au journal des chantiers, et la règle « on n'y
   revient que sur un défaut constaté ».

### 3.1 Le tri des tickets du moteur (session cœur C++, 4 octobre, à relire par le banc)

Seuls les tickets **ouverts** qui touchent le moteur. Les tickets fermés ne bloquent plus
rien ; ceux de l'analyse de code, de codeparsers et de rag3weaver ne sont pas l'affaire de
la stèle.

| Ticket | Verdict | Pourquoi |
|---|---|---|
| Sous `IGNORE_ERRORS`, un doublon de clé fait supprimer une ligne innocente (10 octobre) | corrigé (`de8fc8c0f`) | perte et réponse fausse en silence quand l'index de clé a des entrées sur disque ; témoin `IgnoreErrorsDuplicateKeyTest` ; non atteint par rag3weaver |
| Une corruption de mémoire tue `e2e_code` | corrigé dans le moteur (`57c8389b4`) | la cause : une double ouverture en écriture dans un même processus, maintenant refusée par son nom ; reste à rag3weaver d'attendre la fin de la fermeture |
| Le point de reprise plante, à jamais, après `ALTER TABLE … DROP` | **bloque** | mémoire (SIGSEGV) et durabilité : la base ne peut plus écrire de point de reprise ; rag3weaver ne supprime pas de colonne, mais le défaut corrompt |
| Dans une transaction, après des insertions puis un `COPY` dans la même table, la clé d'une ligne mène à une autre ligne | **bloque** — refusé par son nom (`0f4a54b2c`), le vrai correctif après le chargement journalisé des relations | résultat faux et écriture sur la mauvaise ligne, validés en silence (défaut d'origine, exécuté le 4 octobre) ; les tables de relations ne sont pas touchées |
| Dans une transaction, un balayage de plusieurs tables de relations relit celles d'une autre table | **bloque** | résultat faux |
| Un `COPY` rend une erreur alors qu'il est validé | **bloque** | résultat faux sur la durabilité : l'appelant qui recommence écrit deux fois |
| L'ordre de synchronisation au point de reprise | **bloque, à éprouver** | durabilité, si le doute est fondé ; un arrêt au bon instant le dit |
| Durabilité sur faute d'entrée-sortie, coupure ou mort au mauvais instant | **bloque, borné** | la stèle exige les cas qu'un arrêt au mauvais instant suffit à déclencher (l'ordre des suppressions à la reprise, le rejeu des pages d'ombre non synchronisé) et, pour le reste, « détecter et refuser de continuer » ; le disque plein et la coupure de courant viennent après la stèle, avec un crochet de faute |
| La réouverture d'une base échoue par intermittence, sous charge | **bloque, tant que la cause manque** | une réouverture qui échoue sur un journal de taille nulle touche à la durabilité ; quatre hypothèses écartées |
| Mise à jour massive de vecteurs : des lignes restent injoignables dans l'index | **bloque — sans ticket, à ouvrir** | résultat faux (journal des chantiers §6) |
| Une ligne très loin des autres est injoignable dans un index bâti d'un coup | **bloque — sans ticket, à ouvrir** | résultat faux (journal des chantiers §6) ; à vérifier depuis `1ea49837f`, qui borne autrement le parcours de la construction |
| Un doublon de clé au journal empêche de rouvrir (H4) | **bloque — porté par la condition 1** | durabilité ; c'est A3′ |
| Un `CHECKPOINT` retient la validation d'un lecteur jusqu'à son délai | **bloque** (relecture du banc) | l'écrivain reçoit une erreur de délai alors que sa ligne est validée : même classe que le `COPY` validé qui rend une erreur — un appelant qui recommence écrit deux fois |
| Le point de reprise échoue quand le tampon du moteur est petit | confort, sous condition | à condition que l'erreur soit nommée et qu'aucune écriture validée ne soit perdue, avec un témoin au banc de cette condition |
| Un `COPY` refusé laisse la cardinalité gonflée | confort, sous condition | ne sert plus qu'au planificateur depuis `1ea49837f` ; à condition que `STATS_INFO` dise qu'il rend une estimation |
| Le point de reprise réécrit en entier une table de blobs | hors stèle | une optimisation (ticket du 4 octobre) ; contournée dans rag3weaver |
| Un `NULL` en tête d'une liste de paramètres type sa colonne en `STRING` | confort | un refus nommé, contourné dans rag3weaver |
| Un accent grave doublé dans un nom n'est pas réduit | confort | un cas de syntaxe |
| Deux défauts de chaînes annoncés par Ladybug, non reproduits | hors stèle | correctifs d'amont non reproduits |

Relu par la session du banc le 4 octobre au soir ; les quatre lignes « relecture » et
« sous condition » sont ses corrections, tranchées par l'orchestration. Le partage : les
tickets « bloque » qui ne sont portés par aucune condition vont au banc (point de reprise
après `DROP` de colonne, balayage de plusieurs tables de relations, `COPY` validé qui rend
une erreur et `CHECKPOINT` qui retient un lecteur, ordre de synchronisation, réouverture
intermittente, durabilité à l'arrêt) ; le terrain du `COPY` et de son annulation, la
corruption d'`e2e_code` et les verrous restent au cœur C++. Les deux défauts de l'index
vectoriel sans ticket sont à ouvrir par le banc, qui en a les témoins.

### 3.2 L'ordre (tranché le 4 octobre ; revu le même soir, choix réversibles)

Lucie : « on se focus sur la stèle ». Puis, à 20 h 40, sur la question « c'était quoi le
remède, on l'a attaqué du bon côté ? » : non. **Le `COPY` hors journal est la cause commune**
de ce qui a été payé le 4 octobre — le point de reprise forcé, les trois correctifs de
l'annulation, la perte après un délai, et un cycle d'attente qui aurait demandé un genre de
verrou de plus. On dessinait les verrous autour d'une verrue qui disparaît avec le journal.
Le chargement journalisé passe donc avant le câblage des verrous.

1. l'annulation d'un `COPY` (clés de l'index, point de reprise) — fait ;
2. la corruption d'`e2e_code` — fait : c'était une double ouverture de la base ;
3. le cœur du gestionnaire de verrous (V1), sans câblage — il ne dépend pas du reste ;
4. **le chargement en masse journalisé — fait, `ff9bad960` (10 octobre)** : la validation d'un `COPY` durable par le journal,
   plus de point de reprise forcé, plus d'attente du départ des autres (le remède 2a se
   retire alors, avec ses témoins adaptés). Une page de conception d'abord
   (`extension/rag3weaver/docs/3-octobre-2026-23h31/coeur-cpp/04-le-chargement-en-masse-journalise.md`) ;
   **Précisé le 5 octobre, par lecture puis par la liste d'origine jouée défaut basculé** :
   - *Ce qui se retire est le `COPY` de la liste des appelants, pas le mécanisme.* Le point de
     reprise forcé et son attente restent pour les créations d'index
     (`CREATE_VECTOR_INDEX`, `CREATE_FTS_INDEX`, la création et la suppression d'un index
     spatial) et pour le seul `COPY` qui ne sait pas encore se journaliser : un `COPY` de
     nœuds qui écarte réellement des lignes (`IGNORE_ERRORS`, clé en double ou nulle — il
     faut au journal une forme pour « un trou »). Ces créations d'index sont refusées dans
     une transaction explicite. Conséquence pour A3′ : le cycle d'attente vu avec les
     verrous (une validation qui attend le départ des autres pendant qu'une autre attend
     un verrou qu'elle tient) existe encore pour ces instructions ; acceptable pour un DDL,
     mais A3′ doit le savoir — un genre de verrou « base » pour elles seules, ou le refus
     nommé d'un DDL d'index quand un verrou est tenu.
   - *Une transaction forcée n'écrit rien au journal* (`37608cf4b`) : son point de reprise
     est sa seule durabilité, elle revient entière ou pas du tout. Avant, ses écritures
     ordinaires étaient durables avant son `COPY` — le défaut du chemin par défaut.
   - *Le réglage `force_checkpoint_on_copy` reste*, à `false` par défaut une fois l'étape
     livrée : un chemin de sortie pendant une version.
   - *Quatre conditions du basculement, trouvées en le tentant* : les plantages du chemin
     journalisé sous la liste d'origine ; la borne de mémoire du journal d'une transaction
     (un gros `COPY` ne tient pas dans un petit tampon) ; les pages d'un `COPY` rejoué,
     perdues à chaque reprise après arrêt brutal ; puis la liste d'origine verte jusqu'au
     bout, défaut basculé.
5. le câblage des verrous : A3′, A4′, V2, la maintenance de l'index vectoriel au commit.
   Deux témoins attendus d'A3′, nés des limites écrites le 4 octobre : à l'annulation, une
   transaction ne retire de l'index de clé primaire que les clés de **ses** lignes
   (aujourd'hui : de toute ligne non validée du bloc, celles d'un autre écrivain comprises) ;
   et deux validations qui veulent chacune leur point de reprise ne s'attendent pas jusqu'au
   délai ;
6. les écritures parallèles.

V1 se prouve par les tests du gestionnaire lui-même, joués aussi sous ThreadSanitizer ; les
premiers verts du banc sur les verrous viennent avec A3′. Les durées (session cœur C++,
incertitude d'un facteur 1,5) : chargement journalisé, à redonner après sa page ; câblage
des verrous, 11 à 16 jours ; écritures parallèles, 10 à 20 jours.

## 4. Ce qui n'attend pas la stèle

Un agent seul sur un dépôt marche déjà : l'interface des fiches de contexte
et un premier utilisateur extérieur ne dépendent ni des verrous ni des
écritures parallèles.
