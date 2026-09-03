# Lecteurs concurrents : la mesure

**3 septembre 2026.** Le [document 06](06-reperage-vela.md) §6 affirmait, en
lisant leur code, que le correctif de verrou de Vela ouvre la deuxième
configuration : plusieurs processus **lecteurs** aux côtés d'un écrivain. Cette
mission-ci transforme l'affirmation en fait mesuré.

Le correctif est reporté, un test à deux processus existe, il tourne. Ce qui
suit est **mesuré**, pas déduit.

## Les trois phrases du critère

### 1. Oui, un second processus peut ouvrir en lecture pendant qu'un écrivain travaille

Avant, non — et pas pour la raison qu'on pourrait croire. L'ouverture en lecture
seule ne demandait pas un verrou exclusif, elle demandait un `F_RDLCK` partagé :

```cpp
// src/storage/file_handle.cpp:36
openFlags.lockType = isLockRequired() ? FileLockType::READ_LOCK : FileLockType::NO_LOCK;
```

Mais un `F_RDLCK` entre en conflit avec le `F_WRLCK` de l'écrivain. Deux lecteurs
pouvaient donc coexister ; **un lecteur et un écrivain, jamais.** C'est
exactement ce que le relais de `rag3daemon` contourne.

Le report tient en trois lignes, dans `src/storage/storage_manager.cpp` :

```cpp
// Only the writer takes the exclusive file lock. A read-only open takes
// no lock at all, so a second process can read while a writer works.
if (!readOnly) {
    flag |= FileHandle::O_LOCKED_PERSISTENT_FILE;
}
```

Le lecteur ne prend plus **aucun** verrou. Mesuré par
`LecteursConcurrents.UnLecteurPeutOuvrirPendantQuOnEcrit` : le fils ouvre en
écriture et garde la base ; le père ouvre en lecture, interroge, obtient ses
lignes. **Le test passe.**

Le même test vérifie l'autre moitié du contrat, celle qu'on ne voulait pas
casser : **un second écrivain reste refusé.** On n'a pas levé le verrou
d'écriture, seulement cessé d'en poser un pour lire.

### 2. Ce qu'il lit est cohérent — parce que le moteur refuse plutôt que de déchirer

C'était le vrai risque : un lecteur sans verrou qui lit pendant que l'écrivain
installe un point de reprise pourrait voir un état à demi écrit.

Le test `CeQueLeLecteurVoitEstCoherent` le cherche en flagrant délit. L'écrivain
insère des lignes où `double` vaut toujours exactement `id * 2` — un invariant
qu'une lecture déchirée violerait — et pose un point de reprise tous les
vingt-cinq. Le lecteur rouvre la base en boucle et vérifie chaque ligne.

Trois exécutions, résultat stable :

| | mesuré |
|---|---:|
| tentatives d'ouverture | 60 |
| ouvertures réussies | 52 à 54 |
| lectures réussies après ouverture | **toutes** |
| **incohérences** | **0** |
| reculs (voir moins qu'au tour d'avant) | **0** |

**Zéro ligne déchirée sur environ cent quatre-vingts lectures.** Et le lecteur ne
régresse jamais : il ne voit jamais moins de lignes qu'à un tour précédent.

La raison n'est pas la chance, elle est explicite dans le code. Quand le lecteur
tombe sur un point de reprise à demi installé, il **refuse** :

```cpp
// src/storage/shadow_file.cpp:93
void ShadowFile::replayShadowPageRecords(ClientContext& context) {
    if (context.getDBConfig()->readOnly) {
        throw RuntimeException("Couldn't replay shadow pages under read-only mode. …");
```

C'est le seul refus observé, six à huit fois sur soixante. Le test l'exige
nommément : **tout autre message de refus ferait échouer le test**, parce qu'un
refus qu'on n'a pas identifié est un refus qu'on n'a pas compris.

Au passage, la reprise de journal est bien bridée en lecture seule — elle ne
supprime, ne synchronise ni ne tronque rien
(`wal_replayer.cpp`, quatre gardes). Le lecteur rejoue le journal **en mémoire**,
ce qui explique qu'il voie les écritures validées avant même leur point de
reprise.

### 3. Oui, rag3daemon peut cesser de relayer les lectures — à une condition

Le refus est-il un mur ou une gêne ? C'est ce qui décide, et c'est mesuré par
`LeRefusSeResoutParUneNouvelleTentative`, avec un point de reprise tous les cinq
enregistrements pour élargir au maximum la fenêtre :

| | mesuré |
|---|---:|
| cycles de lecture | 80 |
| réussis du premier coup | 74 à 75 |
| refusés au moins une fois | 5 à 6 |
| **sauvés par une nouvelle tentative** | **tous** |
| **jamais obtenus** | **0** |
| reprises consommées | 15 à 19, soit environ trois par refus |

**Aucun refus n'a résisté.** Le test exige `perdus == 0` : si un seul refus
survivait à cinq tentatives, il échouerait.

> **Donc `rag3daemon` peut cesser de relayer les lectures, à deux conditions —
> et les deux sont désormais tenues.** La première est ici : réessayer sur ce
> refus précis. Une poignée de reprises espacées de quelques millisecondes suffit
> — ce n'est pas une lecture qui échoue, c'est une lecture qui attend. La seconde
> n'était pas dans le périmètre de cette mesure ; elle est décrite juste en
> dessous, et elle a été construite et éprouvée le soir même.
>
> Ce qui reste n'est plus un préalable technique, c'est le retrait lui-même —
> une décision de conception.

### La seconde condition : le contrat de cohérence d'ingestion, qui se dégrade en silence

Signalée par la session qui tient `extension/rag3weaver/`, et vérifiée.

Ce que cette mission mesure, c'est la cohérence **de la page lue** : le lecteur
ne voit jamais une ligne à demi écrite. C'est établi. Mais il existe une seconde
cohérence, celle **de ce qui a été ingéré**, et elle ne survit pas au passage
entre processus.

`SearchOptions` porte un `Consistency` à trois valeurs, et `Strict` vide toute la
file d'ingestion avant de chercher :

```rust
// extension/rag3weaver/src/catalog.rs:3983
search::Consistency::Strict  => { self.drain(); }
search::Consistency::Eventual => { if self.has_pending() { self.flush_insertions(); } }
search::Consistency::Immediate => {}
```

Or `self.pending` est un champ **en mémoire du catalogue**. Un lecteur dans un
autre processus a son propre catalogue, dont la file est vide : `has_pending()`
répond faux, `drain()` ne fait rien. Pour lui, `Strict` et `Eventual` n'ont plus
de sens — **il ne lui reste que `Immediate`, sans que rien ne le lui dise.**

C'est la forme de défaut qu'on passe nos journées à débusquer : un appelant
demande la garantie la plus forte et reçoit la plus faible, en silence.

**Le verrou n'a jamais protégé de ça**, et il faut être juste sur ce point : il
rendait l'accès concurrent *impossible*, pas *ordonné*. On ne perd donc rien
qu'on avait — on découvre une garantie qui n'a jamais franchi la frontière du
processus, et qui devient visible maintenant qu'on peut la franchir.

**Construit et éprouvé le soir même**, par la session qui tient le crate
(`25dca75ce`, `de4fe5d5a`). L'écrivain pose une marque `_ingestion/pending/{son
id}` quand sa file passe de vide à non vide, et l'efface après chaque vidange
réussie. Un lecteur qui demande `Strict` vide sa propre file, puis attend que
plus aucun écrivain ne soit marqué.

Les trois façons d'échouer sont nommées et aucune ne se déguise en succès : délai
expiré, marque périmée au-delà d'une minute — l'attendre transformerait une panne
en gel — et marques illisibles. La vidange porte en plus un filet : si elle
trouve du travail sans marque publiée, elle la pose **et le signale**, pour qu'un
chemin de mise en file oublié demain se voie au lieu de mentir.

Et le test qui compte est à deux vrais processus, contre notre moteur : zéro
marque au repos, une sous travail non publié, zéro après la vidange. Contre une
bibliothèque d'avant le correctif de verrou, l'enfant est refusé et le test le
dit — il ne confond pas « aucune marque » avec « je n'ai pas pu regarder ».

## Ce que ça change, et ce que ça ne change pas

**Ce qu'on gagne.** Les lectures peuvent sortir du relais. Un processus tiers
ouvre la base directement, en lecture seule, pendant que l'écrivain travaille —
et ce qu'il lit est bon, page par page, et cohérent avec ce qui a été ingéré
depuis que la marque d'eau existe. « Peuvent » et non « sortent » : le retrait
lui-même reste à décider.

**Ce qu'on n'a pas pris.** Rien de la couche transactionnelle de Vela : ni leur
contrôle de concurrence multi-version, ni leur rotation de journal, ni leurs
points de reprise non bloquants. Trois lignes, pas trente-quatre commits. Leur
travail de fond reste non relu, comme le document 06 §8 le disait.

**Ce qui reste interdit.** Deux processus **écrivains**. Le verrou exclusif est
intact pour l'écriture, et le test le vérifie.

**Un contrat qui change, et qu'il faut dire.** Avant, un lecteur actif tenait un
`F_RDLCK` qui empêchait un écrivain de s'installer. Ce n'est plus vrai : le
lecteur ne prend aucun verrou, donc **un écrivain peut désormais ouvrir pendant
que des lecteurs travaillent**. C'est précisément la situation que le test 2
éprouve, et elle ne produit pas d'incohérence. Mais c'est une exclusion perdue,
et quelqu'un qui comptait dessus doit le savoir.

## Le contrat public, lui, ne change pas — il est mieux tenu

Ce que `src/include/main/database.h` promet à qui ouvre une base n'a pas bougé :

> *« Multiple read-only `Database` objects can be created with the same database
> path. If false, the database is opened read-write. Under this mode, there must
> not be multiple `Database` objects created with the same database path. »*

Les deux moitiés restent vraies, et la première l'est **davantage** qu'avant :
plusieurs lecteurs pouvaient déjà coexister, ils peuvent désormais le faire même
si quelqu'un écrit. La seconde est intacte, et le test la vérifie. Le contrat
n'a jamais promis qu'un lecteur empêcherait un écrivain de s'installer — c'était
un effet du verrou, pas une promesse.

Le drapeau de verrouillage n'est d'ailleurs consulté qu'à un seul endroit,
`storage_manager.cpp`, et appliqué à un seul autre, `file_handle.cpp:36`. Aucun
autre code ne s'appuyait sur le verrou du lecteur.

## Un test mort qui portait l'ancien contrat

`test/api/db_locking_test.cpp` affirme l'inverse de ce qu'on vient de mesurer :

```cpp
// try to open db for reading, this should fail
systemConfig->readOnly = true;
EXPECT_ANY_THROW(createDBAndConn());
```

Ce fichier **n'est compilé par personne** : il n'est pas dans
`test/api/CMakeLists.txt`, et il ne l'était déjà pas en amont à `89f0263cc`.
C'est du code de test mort hérité de Kuzu, pas de nous. Il n'a donc pas été
« ajusté » — il n'a jamais tourné. Il est laissé en l'état, et le nouveau fichier
`lecteurs_concurrents_test.cpp`, lui, est déclaré et s'exécute.

## Non-régression

Toutes les suites touchées de près ou de loin par le gestionnaire de stockage
ont été exécutées, et toutes passent.

| suite | tests | résultat | durée |
|---|---:|---|---:|
| `api_test` (dont les trois nouveaux) | 101 | **tous** | 40 s |
| `transaction_test` | 49 | **tous** | 13 min |
| `buffer_manager_test` | 3 | **tous** | < 1 s |
| `node_update_test` | 2 | **tous** | 1 s |
| `node_insertion_deletion_test` | 2 | **tous** | < 1 s |

`transaction_test` est la plus intéressante des cinq : elle contient les
`FlakyCheckpointerTest`, qui simulent une panne au milieu d'un point de reprise —
`RecoverFromCheckpointFlushingShadowFailure`, entre autres — et donc exactement
la machinerie de pages fantômes sur laquelle bute notre lecteur. Elle passe
entièrement. Elle prend treize minutes : ne pas la lancer avec un délai
d'attente court, elle n'est pas bloquée, elle est lente.

## Confirmé de l'extérieur, par un second harnais

La mesure ci-dessus est en C++, dans le même processus de test que le moteur. La
session qui tient `extension/rag3weaver/` l'a refaite depuis Rust, avec son
propre montage à deux processus, contre la même bibliothèque : quatre-vingts
ouvertures en lecture seule pendant qu'un écrivain écrit sans relâche, point de
reprise toutes les cinq écritures.

| | contre une bibliothèque d'avant le correctif | contre celle d'après |
|---|---:|---:|
| ouvertures refusées | **80** | **0** |
| lectures obtenues | 0 | **80** |
| lectures incohérentes | — | **0** |

Deux choses en sortent, et la seconde ne se déduisait pas de ma mesure seule.

**Le régime bascule entièrement**, il ne s'améliore pas à la marge. Quatre-vingts
refus deviennent quatre-vingts lectures. C'est bien le verrou qui décidait, et
rien d'autre.

**Et le zéro refus est le vrai résultat.** Ma mesure brute en comptait cinq à six
sur quatre-vingts. Leur chemin de lecture réessaie dans un budget court, et il
n'en laisse passer *aucun*. Ce que mon test
`LeRefusSeResoutParUneNouvelleTentative` établissait dans son propre montage est
donc confirmé de bout en bout, par du code appelant qui n'a pas été écrit pour la
démonstration : **le transitoire est absorbé, l'appelant ne le voit jamais.**

La deuxième condition de la troisième phrase — réessayer sur ce refus — n'est
plus une recommandation, elle est éprouvée.

## Attention au build : un artefact périmé dit le contraire

Un désaccord est apparu entre cette mesure et un test Rust du crate, qui
affirmait l'inverse — qu'un lecteur reste refusé — **et qui passait**. Ce n'était
pas une contradiction : les deux mesures portaient sur des bibliothèques
différentes.

```
build/native-test/src/librag3db.a        24 août 2026
le report du correctif                    3 septembre 2026, 17h14
build/lecteurs/src/librag3db.a            3 septembre 2026, 17h21
```

`build/native-test` précède le correctif de dix jours. Il ne peut pas le
contenir, et l'exclusion qu'il fait observer est l'ancien comportement,
correctement observé sur un artefact périmé.

**La leçon, pour la prochaine fois :** avant de conclure d'un test qui contredit
une mesure, comparer la date de la bibliothèque liée à celle du changement. Un
`stat` sur le `.a` coûte une seconde et évite une enquête.

`build/lecteurs` porte le correctif et a été complété pour être interchangeable —
même type Release, même liaison statique, mêmes extensions `vector;geo`, et un
jeu de bibliothèques statiques désormais identique à celui de `build/native-test`
(il manquait `brotlienc` et les extensions, ajoutés depuis). Pour l'employer sans
recompiler quoi que ce soit :

```sh
export RAG3DB_LIBRARY_DIR=$PWD/build/lecteurs/src
export RAG3DB_INCLUDE_DIR=$PWD/src/include
```

## Comment refaire la mesure

```sh
cmake -S . -B build/lecteurs -DCMAKE_BUILD_TYPE=Release -DBUILD_TESTS=TRUE \
      -DBUILD_SHELL=FALSE -G Ninja
cmake --build build/lecteurs --target api_test -j $(( $(nproc) - 2 ))
./build/lecteurs/test/api/api_test --gtest_filter='LecteursConcurrents.*'
```

Les trois tests écrivent leurs chiffres sur la sortie d'erreur, y compris quand
ils passent : une mesure qui ne s'affiche pas est une mesure qu'on ne relit pas.

## Ce que cette mission n'a pas fait

- **Le fond de la concurrence de Vela reste non relu.** On a pris trois lignes
  qui ne touchent qu'au verrou de fichier. Leur contrôle multi-version et leur
  rotation de journal sont l'autre moitié, et l'enjeu est maintenant clair : si
  on veut plusieurs processus **écrivains**, c'est là qu'il faut aller.
- **Le refus n'a pas été mesuré sous charge réelle.** Six à huit refus sur
  soixante, dans une boucle hostile qui rouvre toutes les deux millisecondes et
  pose un point de reprise tous les cinq enregistrements. Un usage normal en
  verra beaucoup moins ; personne ne l'a vérifié.
- **Rien n'a été changé dans `rag3daemon`.** La mesure dit qu'il *peut* cesser de
  relayer, sous deux conditions désormais tenues toutes les deux. Le faire reste
  un autre chantier, et une décision de conception, pas une mesure.
