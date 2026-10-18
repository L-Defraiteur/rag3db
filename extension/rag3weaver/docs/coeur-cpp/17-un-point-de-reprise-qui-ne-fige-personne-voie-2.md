# Un point de reprise qui ne fige personne — la voie 2, à la manière de PostgreSQL (19 octobre 2026)

Page de conception, pas de code. Elle répond au ticket « un point de reprise fige toutes les
transactions le temps de s'écrire » (`docs/tickets/2026-10-18-un-point-de-reprise-fige-toutes-les-transactions-le-temps-de-s-ecrire.md`),
mesuré par le banc (16 MiB de journal toutes les 3 s sous vingt écrivains, 1 à 1,9 s de gel
chacun ; et, le seuil relevé, un tampon de 2 Gio plein de pages validées non reportées, 43 000
refus). Les témoins de la voie 2 sont sur master (`test/transaction/concurrence/checkpoint_no_stall_test.cpp`,
23583b75b, known_red), quatre, chacun d'un seul contrôle ; ils nomment ce que chaque lot doit
rendre vert.

## 1. Ce que le moteur fait aujourd'hui, étape par étape

**Le déclenchement.** Une validation dont le journal (local + fichier) dépasse `checkpoint_threshold`
(16 MiB) écrit le point de reprise elle-même si elle est seule, sinon pose la demande (A8) et la
dernière transaction à partir l'écrit ; un `CHECKPOINT` explicite ou forcé (COPY hors journal,
index) le fait aussi. Dans tous les cas, `stopNewTransactionsAndWaitForOthersNoLock` : **aucun
BEGIN n'entre, lecteurs compris**, et on attend le départ de tous (borné). C'est le gel mesuré.

**L'écriture** (`Checkpointer::writeCheckpoint`, profil `RAG3DB_PROFILE_CHECKPOINT`) :
1. `checkpointStorage` : chaque table modifiée réécrit **en place**, sous les états de balayage
   (d'où l'arrêt) : les groupes en mémoire sont fusionnés aux blocs persistants par une copie
   (`scanAllInsertedAndVersions`, l'allocation qui manque sous tampon plein), les colonnes des
   blocs persistants sont réécrites par le fichier ombre (une copie de page par page touchée),
   les blocs et groupes sont échangés, les versions aplaties (`checkpointVersionInfo` avec la
   transaction de point de reprise : tout ce qui est validé), les colonnes supprimées retirées et
   **renumérotées** (`vacuumColumnIDs`), les index écrits (clé primaire, HNSW), les tables et
   index retirés récupérés. C'est 85 à 97 % du temps (1,2 à 1,7 s pour 6 000 à 15 000 pages).
2. `serializeCatalogAndMetadata` : catalogue et métadonnées (gestionnaire de pages, en-têtes de
   colonnes) dans des pages neuves ; < 1 ms.
3. `writeDatabaseHeader` : l'en-tête désigne les nouvelles étendues.
4. `logCheckpointAndApplyShadowPages` : l'enregistrement de point de reprise au journal, fsync,
   puis les pages ombres appliquées au fichier de données ; 45 à 110 ms.
5. `finalizeCheckpoint` : pages libérées rendues, journal et fichier ombre remis à zéro, version
   du catalogue remise. Puis `allowNewTransactionsNoLock`.

**Le journal est logique.** Il porte des enregistrements d'instructions (insertion d'une table,
suppression, mise à jour, COPY, entrées de catalogue, séquences, extension), pas des pages. Au
rejeu, le moteur rejoue ces instructions sur le fichier de données tel que le dernier point de
reprise l'a laissé ; les pages ombres ne s'appliquent que si l'enregistrement de point de reprise
qui les couvre est au journal (atomicité).

**Le tampon ne rend pas une page validée non reportée.** Entre deux points de reprise, les
lignes validées vivent dans des blocs en mémoire (groupes en mémoire, versions, mises à jour) ;
l'éviction ne touche que les pages lues depuis le disque. Quand les blocs en mémoire remplissent
le tampon, c'est « Unable to allocate memory » pour tout le monde (le tableau des seuils du banc :
à 64 et 256 MiB, 0 point de reprise et des dizaines de milliers de refus). Le seuil de journal
n'est donc pas une borne de mémoire : il ne compte que le journal, et un vecteur FLOAT[64] sous
HNSW pèse trente fois plus en mémoire qu'au journal.

## 2. Ce que fait PostgreSQL, et ce qui nous sépare

Un checkpoint PostgreSQL n'arrête aucune transaction : il note une position de journal, écrit en
arrière-plan les pages sales (lissé sur `checkpoint_completion_target`), puis fsync et tronque le
journal avant la position. Deux choses le permettent : les pages sales d'une transaction validée
peuvent être écrites **à tout moment** par le bgwriter ou une éviction (« steal »), parce que le
journal est **physique** (redo par page, full page writes) et que le rejeu refait l'état des
pages depuis le dernier checkpoint ; et les lecteurs ne voient jamais une page réécrite sous eux,
parce que les versions (MVCC) sont dans les pages et qu'on n'y renumérote rien.

Nous : journal logique (pas de redo par page), réécriture en place par copie, et des pointeurs de
blocs tenus par les balayages. La voie 2 ne copie pas PostgreSQL : elle lui emprunte les deux
propriétés avec nos moyens.

## 3. La forme cible, en trois pièces

### 3.1 L'écriture à la demande des pages validées (« steal » par le fichier ombre)

Quand le tampon manque, une page d'un bloc en mémoire **validé** (sa transaction est partie) peut
être écrite dans le **fichier ombre**, à une place neuve, et son cadre rendu ; `ShadowFile` sait
déjà retrouver une page par (fichier, page) et l'appliquer plus tard. Relire la page la ramène
du fichier ombre. Ce que cela exige du journal : **rien de plus**, et c'est le point qui rend la
forme simple chez nous. Au crash, les pages ombres d'avant un enregistrement de point de reprise
ne s'appliquent pas (règle existante) : le rejeu logique reconstruit les blocs validés depuis le
journal, comme aujourd'hui. Au point de reprise suivant, ces pages sont déjà écrites : il les
applique (elles font partie des pages à reporter) au lieu de les copier.

Ce qu'il faut : (a) un bloc en mémoire dont les pages peuvent partir et revenir (aujourd'hui un
bloc en mémoire est un tampon épinglé, pas une page du gestionnaire) : ranger les blocs validés
dans des pages du tampon, évictables, lues par le gestionnaire ; (b) une borne de mémoire tenue
(pages validées non reportées) qui déclenche l'écriture à la demande, puis le point de reprise ;
(c) la lecture d'un bloc en mémoire qui passe par le gestionnaire (épingler) et plus par un
pointeur nu. C'est le lot le plus lourd ; il rend verts
`CommittedPagesAreWrittenOutWhenTheBufferIsFull` (400 000 lignes FLOAT[64] sous 64 Mo, aucun refus,
tout se relit) et `CrashAfterWrittenOutPagesReplaysExactly` (le fils tué base ouverte, la base
rouverte porte exactement ce que son journal dit validé : le rejeu logique l'assure déjà, la
pièce ne doit pas le casser).

### 3.2 Un point de reprise qui n'arrête ni les lecteurs ni les écrivains

Trois conditions, dans l'ordre des lots :

1. **Les blocs remplacés restent vivants tant qu'un balayage les tient.** C'est fait depuis le
   lot du rollback sous lecteurs (5e3df4563) : la collection range ses blocs et groupes en
   propriété partagée, les balayages en gardent une référence. Un point de reprise peut donc
   échanger un bloc sous un lecteur : l'ancien vit jusqu'au dernier lecteur, le neuf sert aux
   suivants. Reste à retirer de la réécriture en place ce qui écrit **dans** l'ancien bloc
   (colonnes réécrites par pages ombres) : écrire un bloc neuf, échanger, et laisser l'ancien
   mourir.
2. **Les colonnes ne sont plus renumérotées sous les plans liés.** `vacuumColumnIDs` renumérote
   le catalogue pendant que des plans liés portent les anciens numéros : impossible sous
   lecteurs. Forme : un numéro de colonne ne change jamais ; une colonne supprimée laisse un trou
   (entrée vide dans `columns`, numéro gardé au catalogue comme retiré) ; le trou se comble au
   prochain ADD ou se compacte dans un point de reprise fait sans transaction ouverte (le chemin
   d'aujourd'hui, devenu l'exception). C'est aussi la forme qui simplifie l'ADD paresseux (page
   16, forme (b)) : un bloc absent vaut le défaut, un numéro ne bouge pas.
3. **Le point de reprise lit un instantané et les écrivains continuent.** Il prend un
   horodatage T (tout ce qui est validé avant T est à lui, comme un lecteur) ; il n'écrit que ce
   qui est validé à T (`checkpointVersionInfo` avec un instantané, pas « tout le validé ») ; les
   validations après T vont dans des blocs en mémoire neufs et au journal, pour le suivant. Le
   journal n'est plus remis à zéro mais **tronqué à la position de T** (l'enregistrement de point
   de reprise porte cette position ; le rejeu repart de là). Les lignes validées après T et
   touchant un bloc que le point de reprise réécrit vont dans un bloc en mémoire qui s'ajoute au
   groupe (comme aujourd'hui entre deux points de reprise) : aucune écriture ne croise la copie.
   Les versions antérieures au plus ancien instantané ouvert s'aplatissent ; les autres restent.
   C'est ce que fait déjà la compaction des listes CSR (page 13 §5) avec `oldestActiveStartTS`.

Avec 1 et 2, un **lecteur** n'attend plus : `ReadDuringACheckpointDoesNotWait` passe (lecture par
clé en automatique pendant un point de reprise de 1,1 s, en moins de 100 ms). Avec 3, un
**écrivain** n'attend plus : `BeginDuringACheckpointDoesNotWait` passe (un BEGIN 30 ms après le
départ du point de reprise rend la main en moins de 100 ms). Les validations forcées (COPY hors
journal, index) gardent leur attente : leur durabilité est leur propre point de reprise.

### 3.3 Moins souvent, et borné par la mémoire

Le seuil de journal reste, mais la **mémoire tenue** par les blocs validés devient le premier
déclencheur (3.1 b) ; le point de reprise automatique se fait en arrière-plan sur son instantané,
et une validation ne l'attend plus que pour une raison nommée (forcée). Le convoi des écrivains
d'une table HNSW après le point de reprise (verrou INDEX exclusif, 300 à 430 ms mesurés) relève
d'E11 (forme I2), pas d'ici.

## 4. L'ordre des lots, et ce que chacun rend vert

| lot | contenu | témoin du banc rendu vert |
|---|---|---|
| V2-1 | les colonnes ne sont plus renumérotées (trous, compaction seulement sans transaction) ; le point de reprise n'écrit plus dans l'ancien bloc : bloc neuf, échange, l'ancien vit tant qu'un lecteur le tient ; **les lecteurs ne sont plus arrêtés** (seuls les écrivains attendent encore, comme une validation forcée aujourd'hui) | `ReadDuringACheckpointDoesNotWait` |
| V2-2 | le point de reprise à instantané T : versions écrites à T, journal tronqué à la position de T, blocs neufs pour ce qui vient après ; **plus aucun BEGIN n'attend** ; les validations forcées gardent leur attente | `BeginDuringACheckpointDoesNotWait` |
| V2-3 | les blocs validés en pages du tampon, évictables vers le fichier ombre ; la borne de mémoire tenue ; le déclenchement par la mémoire ; relecture par le gestionnaire | `CommittedPagesAreWrittenOutWhenTheBufferIsFull`, `CrashAfterWrittenOutPagesReplaysExactly` |

V2-1 avant V2-2, parce que l'instantané n'a de sens que si l'ancien bloc survit. V2-3 est
indépendant des deux et le plus lourd ; il peut venir en dernier ou en parallèle, par une autre
session. Chaque lot garde les témoins d'aujourd'hui verts (checkpoint_test, les Flaky*, les
points de reprise ratés et leur condamnation, la rétention des index, le rejeu du journal), et
rejoue le scénario des vingt clients du banc avec le profil.

## 5. Ce qui reste à trancher avant V2-1

- La forme des trous de colonnes au catalogue (une propriété « retirée, numéro gardé » dans
  `PropertyDefinitionCollection`, et la lecture d'un bloc qui n'a pas la colonne : la même branche
  que l'ADD paresseux).
- Le bloc neuf à la place de la réécriture en place : coût mémoire (deux blocs le temps de
  l'échange) et le gestionnaire de pages (les pages de l'ancien bloc rendues quand il meurt, pas
  au point de reprise : `freePageRange` différé, à la mort du dernier lecteur).
- Le point de reprise de fermeture et les ouvertures en lecture seule d'un autre processus
  (`ReadersLock`) : inchangés, ils ne tiennent pas de lecteur en mémoire.
