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

> **Donc `rag3daemon` peut cesser de relayer les lectures, à condition de
> réessayer sur ce refus précis.** Une poignée de reprises espacées de quelques
> millisecondes suffit. Ce n'est pas une lecture qui échoue, c'est une lecture
> qui attend.

## Ce que ça change, et ce que ça ne change pas

**Ce qu'on gagne.** Les lectures sortent du relais. Un processus tiers ouvre la
base directement, en lecture seule, pendant que l'écrivain travaille — et ce
qu'il lit est bon.

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

| suite | résultat |
|---|---|
| `api_test` (dont les trois nouveaux) | **101 / 101** |

Les suites `transaction_test`, `node_insertion_deletion_test`, `node_update_test`
et `buffer_manager_test` ont été lancées ; elles sont longues — les tests de
reprise après panne prennent près de trente secondes chacun. Leur résultat est
reporté au §« Reste à faire » s'il n'est pas consigné ici.

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
  relayer. Le faire est un autre chantier, et il appartient à la session qui
  tient `extension/rag3weaver/`.
