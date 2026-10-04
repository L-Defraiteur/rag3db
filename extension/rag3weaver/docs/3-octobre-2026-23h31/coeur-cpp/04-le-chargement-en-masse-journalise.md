# Le chargement en masse journalisé — une page avant le code

4 octobre 2026, session cœur C++. Demandée par l'orchestration après la décision de Lucie :
le `COPY` hors journal est la cause commune du point de reprise forcé, des défauts de son
annulation et de l'attente du départ des autres transactions ; il passe avant le câblage des
verrous. Suite de `03-le-point-de-reprise-de-copy.md` (voie a).

Étiquettes : **[lu]** lu dans le code, avec `fichier:ligne` ; **[mesuré]** ; **[estimé]** un
calcul, à mesurer en première étape ; **[déduit]** un raisonnement.

## 1. Ce que l'enregistrement de journal porte : les lignes

Deux formes possibles.

- **Les lignes** (logique) : à la validation, le `COPY` écrit au journal les lignes qu'il a
  ajoutées, sous la forme que le journal porte déjà pour une insertion ordinaire —
  `TABLE_INSERTION_RECORD`, des vecteurs de lignes, pour un nœud comme pour une relation
  **[lu]** (`src/storage/wal/local_wal.cpp:46`, `wal_replayer.cpp:575-640`).
- **Les pages** (physique) : le journal ne porte que les plages de pages déjà écrites et les
  métadonnées des blocs ; le rejeu les rattache à la table.

**Retenu : les lignes.** C'est ce que fait PostgreSQL pour un `COPY` — il journalise les
lignes insérées, par paquets, pas des renvois vers des pages. Et c'est la forme de notre
rejeu, qui rejoue des insertions : la voie physique demanderait un second rejeu, qui
rattache des blocs, remet à jour le gestionnaire de pages libres (des pages écrites que le
dernier point de reprise ne connaît pas), reconstruit l'index de clé primaire en relisant les
blocs, et traite à part le dernier groupe incomplet, qui n'est pas sur disque. C'est trois
structures de plus à tenir justes à la reprise, sur le terrain où trois défauts ont été
corrigés le 4 octobre. La voie physique reste possible plus tard, comme optimisation de
taille, derrière le même enregistrement de validation.

Le `COPY` garde son chemin rapide : il écrit toujours ses blocs directement dans le fichier.
Le journal est une **copie pour la reprise**, pas le chemin d'écriture.

## 1 bis. Une vérification préalable : les gros enregistrements

Tout va reposer sur des enregistrements de plusieurs Kio. Le défaut du 1er octobre —
`resizeBufferIfNeeded` remplaçait le tampon sans recopier son début, un enregistrement de
plus de 4 Kio perdait donc son début avec une somme de contrôle valide — **est corrigé sur
master** : `955b1b136` (« fix(wal): les enregistrements de plus de 4 Kio gardent leur
début », 1er octobre 2026), dans `src/storage/wal/checksum_writer.cpp` et
`checksum_reader.cpp`, avec son témoin dans `test/transaction/wal_test.cpp` (« Des
enregistrements de plus de 4096 octets se rejouent avec leurs valeurs ») **[lu]**. Pas
d'étape 0.

## 2. Le rejeu

Rien de neuf dans le principe : `replayNodeTableInsertRecord` et
`replayRelTableInsertRecord` rejouent des insertions, les décalages de lignes se
reproduisent parce que le rejeu ajoute dans le même ordre sur le même état de départ — c'est
déjà ce sur quoi reposent les insertions ordinaires **[lu]**.

Ce qui est à faire : le rejeu d'un nœud insère aujourd'hui **ligne par ligne**
(`wal_replayer.cpp:613-616`). Pour un chargement de dizaines de milliers de lignes il faut
un rejeu par vecteur entier. Lent ne veut pas dire faux : la première étape mesure le rejeu
tel qu'il est, **à la taille du premier index** (≈ 79 000 scopes, leurs morceaux et
vecteurs, 250 000 relations) — c'est la durée de réouverture après un arrêt brutal en plein
index, un chiffre à rendre à Lucie. Le seuil : si cette réouverture dépasse **30 secondes**,
le rejeu passe par vecteur dès l'étape 2 ; en dessous, il reste tel quel.

`COPY_TABLE_RECORD` existe, n'est jamais écrit, et son rejeu ne fait rien **[lu]**
(`wal_replayer.cpp:742`) : il se retire.

## 3. L'annulation : ce qui devient plus simple, ce qui reste

**Plus simple.**
- Plus de point de reprise dans la validation d'un `COPY` : la classe « le point de reprise
  échoue ou expire après une validation déjà faite » disparaît pour lui (le ticket du `COPY`
  validé qui rend une erreur, la perte après un délai).
- Plus d'attente du départ des autres avant de valider : le remède 2a du banc se retire, et
  avec lui le cycle d'attente qui aurait demandé un genre de verrou de plus.
- Une transaction qui mêle `COPY` et écritures ordinaires est durable d'un seul tenant, par
  son enregistrement de validation.

**Ce qui reste, inchangé.** L'annulation d'un `COPY` défait toujours ce qu'il a écrit hors
journal : les blocs ajoutés (`ChunkedNodeGroup::rollbackInsert`), les clés de l'index
(`RollbackPKDeleter`), les pages prises (`OptimisticAllocator::rollback`). Les trois
correctifs du 4 octobre restent nécessaires tels quels. Le journal n'y change rien : une
transaction annulée n'y écrit rien.

**Un point à tenir** : les pages qu'un `COPY` validé a écrites directement ne sont connues
du gestionnaire de pages libres qu'au point de reprise suivant. Après un arrêt brutal, le
rejeu réinsère les lignes dans des pages neuves ; les anciennes sont au-delà de ce que le
dernier point de reprise connaît du fichier, donc reprises naturellement **[déduit, à
éprouver par le témoin « taille du fichier bornée »]**.

## 4. L'index de clé primaire, et les autres index

Le rejeu passe par `NodeTable::insert`, qui tient l'index de clé primaire comme pour toute
insertion **[lu]** : rien à ajouter. L'index vectoriel est tenu par le rejeu depuis la
garde 2 (l'extension est chargée avant de rejouer) ; s'il ne peut pas l'être, il se déclare
« en retard » (garde 1). Un `COPY` dans une table indexée se rejoue donc comme une insertion.

## 5. La taille du journal, et ce qu'elle coûte

Le journal porte les lignes non compressées. **[estimé]** pour le premier index de ce dépôt :

| Quoi | Lignes | Octets par ligne | Journal |
|---|---|---|---|
| relations | 250 000 | ≈ 50 (deux extrémités, identité, propriétés courtes) | ≈ 12 Mio |
| morceaux avec leur vecteur (768 flottants) | ≈ 30 000 | ≈ 3 100 + le texte | ≈ 100 à 150 Mio |
| scopes, symboles, fichiers (texte du code) | ≈ 50 000 | variable | ≈ 30 à 60 Mio |

Soit 150 à 250 Mio écrits en séquence, de l'ordre d'une à deux secondes sur ce disque, plus
une synchronisation par validation. À mesurer en étape 1, avant toute autre chose.

Deux conséquences.
- **Le seuil du point de reprise** (16 Mio par défaut, `database.h:68`) est dépassé par
  chaque gros `COPY` : la validation fera encore un point de reprise, **au seuil** — reporté
  sans erreur si une autre transaction est ouverte (remède 1 du banc), au lieu d'être forcé.
  Pour un premier index, rag3weaver relève le seuil ou laisse faire : un point de reprise ne
  coûte plus que ≈ 1 s depuis que les blobs du plein texte ne sont poussés qu'une fois.
- **La mémoire** : le journal d'une transaction est tenu en mémoire jusqu'à sa validation
  (`LocalWAL`, un `InMemFileWriter`) **[lu]**. Un `COPY` de plusieurs Gio doublerait sa
  mémoire. Borne à poser en étape 4 : au-delà d'une taille, les enregistrements du `COPY`
  s'écrivent au fil de l'eau dans le journal, sous la marque de la transaction (comme
  PostgreSQL), et ne comptent qu'à l'enregistrement de validation.

## 6. Ce qui se retire

- `setForceCheckpoint` pour un `COPY` (`client_context.cpp:535-538`) et la branche « forcé »
  de `TransactionManager::commit`. **Ce qui se retire est l'obligation, pas le coût** : avec
  le seuil par défaut de 16 Mio, un gros `COPY` dépasse le seuil et la validation fait encore
  son point de reprise. Le temps n'est gagné que si le seuil est relevé — rag3weaver le
  règle déjà pour un premier index. Ne pas promettre un gain de durée sans ce réglage ; le
  gain sûr est de justesse (plus d'erreur après validation, plus d'attente, durabilité par
  le journal).
- Le remède 2a (`68ff9d5e2`) : l'attente du départ des autres avant la validation d'un
  `COPY`. Ses témoins s'adaptent avec le banc (« le `COPY` valide sans attendre, et survit à
  une mort brutale »).
- `COPY_TABLE_RECORD`.
- Côté rag3weaver, probablement le réglage « valider tous les K paquets » : il n'existait
  que pour espacer les points de reprise forcés. À décider par la session de l'arbre
  principal sur mesure.

`CREATE_VECTOR_INDEX` garde son point de reprise : c'est un autre chantier.

## 7. La preuve

Par un processus tué (SIGKILL), base ouverte, **sans aucun point de reprise** entre-temps
(`auto_checkpoint=false`, journal non vide vérifié avant de rouvrir), et un processus neuf :

1. mort **au milieu** d'un `COPY` : aucune de ses lignes ; les transactions validées avant
   sont là ; la taille du fichier après rechargement reste bornée ;
2. mort **juste après** un `COPY` validé : toutes ses lignes, chaque clé retrouvée par
   l'index, les deux sens de chaque table de relations d'accord ;
3. un `COPY` de nœuds puis un `COPY` de relations vers ces nœuds, puis des écritures
   ordinaires qui s'y réfèrent, dans une transaction et dans plusieurs : tout ou rien par
   transaction ;
4. un `COPY` dans une table à index vectoriel : l'index répond juste après la reprise ;
5. un `COPY` annulé, puis mort : rien ; un `COPY` refusé pour clé en double suivi d'un
   `COPY` validé, puis mort : seul le second.

Le banc écrit 1 et 2 (il les prépare) ; 3 à 5 sont des tests du moteur. Tous rouges sur le
code d'aujourd'hui dès qu'on retire le point de reprise forcé sans rien mettre à sa place :
c'est la première chose à vérifier.

## 8. Les étapes, et l'estimation en passes

Le temps réel ici est celui des passes, sérialisées par le verrou du poste. Durées
constatées le 4 octobre : un rebâti du moteur 3 à 10 min (10 quand un en-tête public change),
une liste C++ complète ≈ 20 min, une passe sous AddressSanitizer ou ThreadSanitizer ≈ 10 min
(build compris), une passe Rust complète ≈ 40 min, une mesure exclusive du premier index
≈ 3 min plus le rebâti de la bibliothèque de l'arbre principal. Les tests d'arrêt brutal
sont dans `transaction_test` : quelques secondes, comptés avec le rebâti.

| Étape | Contenu | Rebâtis | Listes C++ | ASan / TSan | Mesures exclusives | Passe Rust |
|---|---|---|---|---|---|---|
| 1 | mesurer : journaliser les lignes d'un `COPY` de nœuds à la validation, derrière un interrupteur, sans rien retirer ; taille, durée d'écriture, durée du rejeu à la taille du premier index | 2 | 1 | — | 2 (une avec, une sans) | — |
| 2 | nœuds : rejeu (par vecteur si le seuil des 30 s est dépassé), témoins 1, 2, 5 | 3 | 1 | 1 ASan | — | — |
| 3 | relations, puis index vectoriel : journal, rejeu, témoins 3 et 4 | 3 | 1 | 1 ASan | — | — |
| 4 | retirer le forcé et le remède 2a, adapter les témoins avec le banc ; la borne de mémoire du journal | 2 | 1 | 1 TSan | — | — |
| 5 | passe Rust, mesure avant/après du premier index | 1 | — | — | 2 | 1 |
| **Total** | | **11** | **4** | **3** | **4** | **1** |

Ce qui se regroupe : les étapes 2 et 3 peuvent partager une liste C++ si elles sont livrées
ensemble (3 listes au lieu de 4) ; chaque passe ASan ou TSan se joue dans la même tenue du
verrou que la liste de son étape. Soit **environ 4 heures de passes** au total (11 rebâtis
≈ 70 min, 4 listes ≈ 80 min, 3 passes instrumentées ≈ 30 min, 1 passe Rust ≈ 40 min, 4 mesures
≈ 20 min), hors l'attente du verrou quand une autre session mesure. Le reste est de
l'écriture et de la relecture, qui ne prennent pas le poste. L'incertitude est dans le nombre
de rebâtis : un défaut trouvé par un témoin en ajoute deux ou trois à son étape. En jours de
session, l'ancienne fourchette (7 à 9) tient.

Rien de cela ne va aux amonts.
