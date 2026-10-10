# Après un COPY annulé ou refusé, le point de reprise suivant ne finit pas

- **État** : corrigé le 5 octobre 2026 par la session cœur C++ (« fix(index de clé): le compte de
  l'index en mémoire recule au retrait d'une clé, et sa réservation ne passe plus sous zéro »).
  Les deux témoins du banc sont sortis de `known_red.txt` dans ce commit. Le détail de la cause,
  la taille mesurée et l'essai sur une base déjà abîmée sont au ticket jumeau,
  `2026-10-05-index-de-cle-primaire-apres-un-copy-a-court-de-memoire.md` ; la base abîmée par
  l'ancien moteur est gardée hors dépôt, aux annexes de la session
  (`~/.cache/rag3db-moteur-notes/etape-4/base-abimee-par-l-ancien-moteur.rag3db`).
- **Gravité** : blocage, et panne de mémoire du poste. Le point de reprise boucle et prend la
  mémoire : de 4 à 8 Gio en une demi-seconde. Sans plafond, c'est une panne de la machine.
  Aucune donnée perdue : avant lui, toutes les clés sont retrouvées.
- **Atteignable en service** : oui. C'est le chemin d'échec d'un paquet de rag3weaver :
  des COPY annulés, ou un COPY refusé pour une clé en double, suivis d'un point de reprise.
- **Touche rag3weaver** : oui (la transaction par paquet, et tout COPY refusé dans une
  table à clé primaire).

## Ce que c'est

Un COPY de 100 000 lignes (moins d'un groupe) est annulé, puis le même COPY est validé.
Les 100 002 lignes de la table sont toutes retrouvées par leur clé. Pourtant, le
`CHECKPOINT` suivant ne finit pas : plus de 240 s sous un plafond de 2 Go, ou tué par le
plafond sous 4 Go. La pile de la session cœur C++ : `HashIndex::checkpoint` →
`splitSlots`, en boucle. `STATS_INFO` compte aussi les lignes annulées (200 002) ; c'est
une estimation, et peut-être l'ombre du même défaut (ticket
`2026-10-04-copy-refuse-gonfle-la-cardinalite.md`).

Rejoué sur master sans aucun correctif du banc, le 5 octobre. La panne de mémoire au milieu
d'un COPY, trouvée par la session cœur C++, mène au même état : ce ticket en est une autre
porte, la plus simple.

## Recette minimale

`auto_checkpoint=false` :

```
CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, name STRING);
BEGIN TRANSACTION; COPY Doc FROM '<100 000 lignes, id 0..99999>'; ROLLBACK;
COPY Doc FROM '<les mêmes 100 000 lignes>';
CREATE (:Doc {id: 500000, name: 'a'});
CHECKPOINT;   -- ne finit pas
```

Même effet si l'on remplace le COPY annulé par un COPY de 100 000 lignes refusé pour une
clé en double au milieu de son lot, ou par trois COPY annulés.

Le seuil, mesuré par la session cœur C++ sur l'ancien moteur, table vide, clé INT64 : un COPY
annulé de 16 000 lignes passe, un de 18 000 ne finit plus.

Une base déjà abîmée par l'ancien moteur (index écrit trop grand) s'ouvre, s'écrit et fait
ses points de reprise sous le moteur corrigé (session cœur C++). Base gardée :
`~/.cache/rag3db-moteur-notes/etape-4/base-abimee-par-l-ancien-moteur.rag3db`.

## Cause (session cœur C++, par lecture, non jouée)

- `InMemHashIndex::deleteKey` (`in_mem_hash_index.h:167`) retire la clé sans reculer
  `indexHeader.numEntries`. L'annulation d'un COPY retire ses clés par là : l'index en
  mémoire, vide, se croit à 100 000 entrées.
- `HashIndex::reserve` (`hash_index.cpp:234`) appelle `splitSlots` avec « cases requises −
  cases existantes » en non signé. Quand les cases requises sont moins nombreuses, la
  soustraction reboucle, et `splitSlots` part pour 2^64 cases.
- Le compte de la table (`NodeGroupCollection::numTotalRows`) et le curseur de réservation
  (`NodeGroup::nextRowToAppend`) ne reculaient pas non plus de tout ce que l'annulation
  retire. Correction écrite chez la session cœur C++.

## Témoin

`upstream_fixes_test.cpp`, `Copies/RolledBackCopyThenCheckpoint.*` (`RolledBack`,
`Refused`) : le COPY annulé ou refusé, le COPY validé, deux créations, les recherches par
clé, le point de reprise, puis la réouverture.

Le travail tourne dans un fils, avec une échéance de 60 s, un tampon de 512 Mo, une
réservation virtuelle de 1 Go et `RLIMIT_DATA` à 3 Go. Un point de reprise sans fin ou qui
enfle échoue seul, sans emporter la passe ni le poste. Rouge dans `known_red.txt`
(`checkpoint-finishes`).

## Pour le fermer

Les deux corrections de l'index et celles du compte, commit f3a581fb8 de la session cœur
C++. Ses cinq fichiers, appliqués sans commit dans l'arbre du banc, verdissent les deux
formes du témoin aux mêmes plafonds (5 octobre) : le point de reprise finit en 0,3 s, et
toutes les clés sont retrouvées après la réouverture. `STATS_INFO` reste gonflé (200 002) :
c'est un autre défaut, au ticket `2026-10-04-copy-refuse-gonfle-la-cardinalite.md`.
