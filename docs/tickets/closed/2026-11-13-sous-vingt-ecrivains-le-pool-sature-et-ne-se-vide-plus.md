# Sous vingt écrivains, le pool sature et ne se vide plus

- **État** : corrigé sur master `2385b5f616` (16 novembre 2026, cœur c++) : une version de mise à jour naît à 4 lignes et double à la demande ; le pic du pool sous `DdlEveryMinute` passe de 2 000 à 331-445 Mio
- **Famille** : moteur
- **Gravité** : indisponibilité partielle — la plupart des écritures et une partie des lectures
  sont refusées tant que le pool reste plein ; la base reste ouverte et intègre
- **Atteignable en service** : à établir. Le banc le reproduit seul ; un hôte au pool de tampons
  serré sous de nombreux écrivains concurrents et des DDL y ressemble.
- **Touche rag3weaver** : par le moteur (refus « buffer pool is full » sous charge)
- **Signalé par** : cœur c++ — banc décrit dans ce ticket
- **Pour** : cœur c++

## Ce que c'est

`TwentyWriters.DdlEveryMinute` (`test/transaction/concurrence/twenty_writers_scenario_test.cpp`,
cas long listé dans `long.txt`) fait tourner vingt écrivains et des DDL toutes les minutes. Joué à
luciepc le 13 novembre 2026 sur la branche `annulation-pool-plein-2` (`aab8ad0ba`, le correctif de
l'annulation inachevée sous pool plein reposé sur master), il reste rouge, 377 s.

Avant ce correctif, le premier pool plein faisait échouer une annulation à mi-chemin et la base se
mettait en état « à rouvrir » (ticket fermé
`docs/tickets/closed/2026-11-09-sous-vingt-ecrivains-un-pool-plein-laisse-une-annulation-inachevee.md`).
Avec lui, plus aucune annulation n'échoue, la base reste utilisable, et la saturation du pool se voit
seule :

| Erreur dans le journal du scénario | Nombre |
|---|---|
| `Buffer manager exception: Unable to allocate memory! The buffer pool is full and no memory could be freed!`, hors DDL | 76 176 |
| `Buffer manager exception: No more frame groups can be added to the allocator.` | 20 |
| `could not serialize access due to concurrent update` (attendu) | 10 |
| `Found duplicated primary key value` (attendu) | 2 |

Les contrôles qui échouent :
- `index-creation-refuses-no-reader` : 14 lectures refusées pendant la création d'un index ;
- une exception non rattrapée à la fin du scénario (étiquette `untagged`) : `MATCH (d:Doc) WHERE
  d.id < 10 SET d.b = …` lève « Cannot find property b for d », parce que le `ALTER TABLE … ADD b`
  du scénario a été refusé par le pool plein. C'est une conséquence de la saturation, et le scénario
  devrait la dire par une étiquette au lieu de lever.

La lib du banc : celle de `aab8ad0ba`, bâtie dans `~/git_workspaces/rag3db-colonnes/build/moteur`
(Release) à luciepc ; journal `~/.cache/rag3db-moteur-notes/annulation-pool-plein/concurrence.log`.

## Les questions

1. Pourquoi le pool ne se vide pas : des pages validées pas encore reportées par un point de reprise
   (le point de reprise automatique est-il reporté sans fin sous vingt écrivains ?), des blocs en
   mémoire de transactions longues, ou le DDL qui retient tout.
2. « No more frame groups can be added to the allocator » : la réserve d'adresses du gestionnaire de
   tampons épuisée, une limite distincte de la taille du pool.
3. Le scénario : une étiquette pour le DDL refusé au lieu de l'exception de fin.

## Témoin

`TwentyWriters.DdlEveryMinute` lui-même, rouge seul, inscrit dans
`test/transaction/concurrence/known_red.txt` avec les étiquettes
`index-creation-refuses-no-reader,untagged`.

## Pour le fermer

La cause de la saturation, le correctif, `DdlEveryMinute` vert et sa ligne retirée de
`known_red.txt` dans le même commit.

## Joué seul trois fois (13 novembre 2026, soir)

Sur `d5aa49c49` (master `2ac8080ca` plus deux témoins de test), à luciepc : rouge au tour 1
(442 BEGIN refusés « buffer pool is full », 31 512 erreurs hors DDL, le ADD refusé puis l'exception
« Cannot find property rang »), vert au tour 2, SIGSEGV au tour 3 (ticket
`2026-11-13-sous-vingt-ecrivains-un-lecteur-meurt-dans-l-index-de-cle-primaire-sur-disque.md`). Le
banc entier du même soir l'avait vu vert. Le cas est donc variable ; sa ligne de `known_red.txt`
reste telle quelle jusqu'à une mesure qui décide entre `known_red.txt` et `probabilistic.txt`.

## Ce que coûte une écriture (15 novembre 2026, cœur c++)

Une sonde hors dépôt, sur master `2c11a7995f` à luciepc : une table Doc(id, titre STRING, corps
STRING, vec FLOAT[64]) de 50 000 lignes par COPY, puis CHECKPOINT, sans point de reprise
automatique ; 2 000 à 4 000 écritures d'une ligne ; le pool relevé par
`BufferManager::getUsedMemory` avant et après. Journaux :
`~/.cache/rag3db-moteur-notes/sonde-pool/`.

| Écriture | Pool gardé par écriture |
|---|---|
| SET d'une chaîne (titre) sur un nœud existant, déjà sur disque | 8,5 Ko |
| SET d'un FLOAT[64] (256 octets de donnée) sur un nœud existant | 25 Ko |
| SET de titre, corps et vec, par MERGE ou par MATCH | 42 Ko |
| la même, suivie de la création d'une relation (l'instruction du scénario) | 35 à 42 Ko |
| MERGE d'un nœud neuf | 300 octets |
| CREATE d'une relation | 59 octets |

- Le même coût à une connexion et à vingt, avec ou sans table de relations, à 2 ou à 8 fils.
- Le CHECKPOINT rend tout (le pool revient à 37 Mio).
- **L'écart avec la toute première sonde, expliqué** : son COPY échouait (« Copy exception: Found
  NULL, which violates the non-null constraint of the primary key column »), sa table était vide
  (0 ligne), et ses SET ne modifiaient rien : d'où +4 Kio. Les mesures ci-dessus sont les bonnes.

**La lecture** : de l'ordre de 25 Ko de pool pour 256 octets de donnée ressemble à des tampons
d'une mise à jour arrondis à la page. Sous vingt écrivains à 42 Ko par écriture, 2 Gio sont pleins
en une cinquantaine de milliers d'écritures. Le journal ne grossit que de quelques centaines
d'octets par écriture : le point de reprise automatique, déclenché par la taille du journal (16 Mio
par défaut), vient bien après le pool plein, s'il n'est pas en plus reporté par les transactions
ouvertes.

**Deux voies à trancher** (non mesurées) :
- (a) une mise à jour garde un tampon à sa taille, et non à la page ;
- (b) le point de reprise automatique se déclenche aussi sur la pression du pool, et non sur le seul
  journal.

## La cause (15 novembre 2026, cœur c++)

Lue dans `UpdateInfo::update` (`src/storage/table/update_info.cpp`). Chaque transaction qui met à
jour une ligne déjà écrite crée une version du vecteur de 2 048 lignes de cette ligne
(`VectorUpdateInfo`, chaînée aux versions d'avant jusqu'au point de reprise). Son tampon naissait à
pleine capacité (`DEFAULT_VECTOR_CAPACITY`, 2 048 lignes) pour une seule ligne changée. Les petites
allocations passent par `MemoryManager::mallocBuffer`, à leur taille exacte, que le pool réserve :
une version gardait donc 2 048 lignes de pool. Cela donne 8,5 Ko pour une chaîne (2 048 × 4 octets
de positions) et 25 Ko pour un FLOAT[64].

**Le correctif** (branche `mise-a-jour-a-sa-taille`) : la version naît à 4 lignes et double à la
demande, jusqu'au vecteur entier.

**Le témoin** `UpdateMemory` (`test/transaction/update_memory_test.cpp`) : le pool gardé par mise à
jour d'une ligne, sous 4 Kio pour un FLOAT[64] et sous 1 Kio pour une chaîne ; et une mise à jour
d'un vecteur entier relue avant et après une réouverture. Rouge sur `abc5117fd` : 25 136 et
8 528 octets. Vert sur `ec7a0bf30`, `transaction_test` entier vert (390).

**(b), le point de reprise sous la pression du pool**, ne s'ajoute que si les ×10 de
`DdlEveryMinute` le demandent, avec sa mesure (c7).

## Fermé (16 novembre 2026)

Sur master `2385b5f616` (voie des suites C++ transférables). Le pool gardé par écriture, même sonde
avant et après (`abc5117fd` → `ec7a0bf30`) : SET des trois colonnes 42 095 → 471 octets ; SET du
vecteur 25 149 → 373 ; SET du titre 8 526 → 102 ; l'instruction du scénario à vingt connexions
42 101 → 477. `DdlEveryMinute` ×10 (CONCURRENCE_LONG=1) : aucun refus du pool plein (des dizaines
de milliers avant), pic de 331 à 445 Mio (2 000 à 2 052 avant), 10 tours sur 10 jusqu'aux comptes,
justes. `index-creation-refuses-no-reader` : 0 sur 10. Le point de reprise sous la pression du pool,
voie (b), n'est pas demandé par ces mesures.

Sa ligne de `known_red.txt` est retirée dans le même lot : le cas passe dans `probabilistic.txt`,
pour l'intermittent d'avant qui reste (ticket
`2026-11-16-un-create-rel-table-retient-un-ecrivain-au-dela-de-sa-duree.md`, 1 tour sur 10).
