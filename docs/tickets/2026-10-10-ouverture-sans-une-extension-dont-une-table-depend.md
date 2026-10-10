# Une base s'ouvre sans dire qu'une extension dont une table dépend n'a pas pu être chargée

- **État** : ouvert — proposition de l'orchestration (10 octobre 2026), rien de codé ; Lucie peut
  renverser
- **Gravité** : perte d'un index (une donnée dérivée, à rebâtir), sans que l'ouverture le dise
- **Atteignable en service** : oui, si le fichier d'une extension manque ou ne se charge pas au
  moment d'un rejeu
- **Touche rag3weaver** : oui (index vectoriel ; il lit `getRecoveryLoadFailures` en
  avertissement seulement, d'après la session de l'arbre principal)
- **Ouvert le** : 10 octobre 2026, seconde session cœur C++, sur relevé de rag3db-96 et de
  l'orchestration
- **Pour** : cœur C++

## Ce que c'est

Quand le rejeu ne peut pas charger une extension (fichier absent ou refusé), la base s'ouvre
quand même ; l'index qui en dépend est détaché et devient « à rebâtir ». L'ouverture réussit
sans erreur : l'échec n'est lisible que par `ExtensionManager::getRecoveryLoadFailures`.

## Ce qui est vérifié dans le code (à `ba11e003b` et suivants)

- `WALReplayer::replayLoadExtensionRecord` (`wal_replayer.cpp:788-795`) charge l'extension par
  `loadExtensionForRecovery` : un échec n'interrompt pas la reprise ; il est rangé dans
  `recoveryLoadFailures` (`extension_manager.cpp:121-127`, lu par `getRecoveryLoadFailures`,
  `extension_manager.h:59`).
- C'est la garde 1 (`fcd9a7882`) : l'index non chargé est détaché (`IndexHolder::detach`), n'est
  plus écrit par le point de reprise ; son entrée reste au catalogue — l'état « à rebâtir ». La
  recherche le refuse par son nom (« is behind its table »), `DROP_VECTOR_INDEX` fonctionne.
- Les lignes de la table ne sont pas perdues. Ce qui l'est, c'est l'index : le point de reprise
  qui suit la reprise écrit la table sans lui, et le journal qui aurait permis de le tenir à jour
  est vidé ; il faut le rebâtir (`DROP_VECTOR_INDEX` puis `CREATE_VECTOR_INDEX`).

## Rapporté, non vérifié ici

- Neo4j refuserait de démarrer une base quand le fournisseur d'un index manque (orchestration) :
  à vérifier dans sa documentation avant de s'y appuyer.

## La proposition

Refuser l'ouverture en écriture quand le rejeu n'a pas pu charger une extension dont un index
d'une table dépend, par une erreur nommée qui dit l'extension et la table ; une option explicite
pour ouvrir quand même (en lecture seule, ou en acceptant de rebâtir). Ce qui change pour
rag3weaver : il reconnaît l'erreur, propose le rebâti, et ouvre avec l'option.

## Le témoin à écrire

Une base avec un index vectoriel, un journal non vide qui porte le `LOAD EXTENSION`, le fichier
de l'extension rendu introuvable, un processus neuf : aujourd'hui l'ouverture réussit et
`getRecoveryLoadFailures` n'est pas vide ; après le correctif, l'ouverture est refusée par son
nom, et l'option l'ouvre. La famille `ExtensionIndexRecovery` du banc a déjà des formes proches.

## Voir aussi

`2026-10-05-extension-chargee-au-rejeu-sans-controle-de-bati.md` (une extension chargée au rejeu
sans contrôle de bâti).
