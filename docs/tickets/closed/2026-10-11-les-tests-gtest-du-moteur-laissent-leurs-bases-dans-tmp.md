# Les tests gtest du moteur laissent leurs bases dans /tmp/rag3db, qui est de la mémoire vive

- **État** : corrigé par le commit qui déplace ce ticket (11 octobre 2026, 15 h ; ouvert le matin,
  relevé par la session « e2e et disque », rag3db-97). Forme : le parent par passe décrit plus bas
  — `TestHelper::getRootTempDir()` rend `<commun>/<pid du lanceur>` (pid pris à l'initialisation
  statique, avant tout fork), un `::testing::Environment` (`TempDirPerRun`, `test_helper.cpp`)
  efface ce parent au début et à la fin du lanceur, et les parents des passes dont le lanceur
  n'existe plus (`removeDeadRunTempDirs`, `kill(pid, 0)`). Témoin
  `HarnessMechanics.WhatAKilledChildLeavesLivesUnderTheRunsParent` (banc). **Ce qui reste à la
  main, une fois** : les dossiers d'avant (`<Groupe.Nom><suffixe>` à la racine de `/tmp/rag3db`)
  ne sont pas effacés par le moteur — un binaire d'avant la correction, dans un autre arbre, y
  écrit encore ; les effacer quand tous les arbres sont rebâtis. Côté Rust (les e2e de
  rag3weaver), rien ici.
- **Gravité** : confort, mais sur ce poste `/tmp` est un tmpfs : 6 500 entrées, 8,6 Go de RAM sur
  le poste principal et 10,9 Go sur luciepc, effacés à la main (seulement ce qu'aucun processus
  vivant n'ouvrait). Le dossier se remplit de nouveau à chaque passe.
- **Atteignable en service** : non (tests seulement).
- **Touche rag3weaver** : non ; ses e2e ont le même défaut, côté Rust (même session).

## Ce que c'est

`TestHelper::getRootTempDir()` donne `/tmp/rag3db` (ou `$RUNNER_TEMP/rag3db`), et chaque test y
crée `<Groupe.Nom><suffixe>`. `BaseGraphTest::TearDown` efface le dossier parent du chemin de la
base (`removeParentDirectoryOfDBPath`) — mais pas tout : un test qui meurt (les témoins d'arrêt
brutal, `fork` + `SIGKILL`, les lanceurs `Crash` et `Process` du banc), un test qui échoue avant
son `TearDown`, un test qui ouvre une seconde base à un chemin à lui (`.reference`, les fixtures
copiées, `ApiTest.DBFileUnderNonExistingDir…`), un `TearDown` surchargé qui oublie le parent : tout
cela reste. Des milliers de dossiers s'accumulent, en RAM.

## Pour le fermer

Un **parent par passe** : `getRootTempDir()` rend `/tmp/rag3db/<pid du lanceur>/` ; un
`::testing::Environment` global efface ce parent à la fin du processus (et à son début, s'il
existait d'une passe tuée au même pid — rare) ; les enfants `fork` écrivent sous le même parent
(même pid de lanceur dans le chemin, pris avant le `fork`), donc une base laissée par une mort
voulue est effacée avec la passe. Et au `SetUp` du lanceur, effacer les parents de passes mortes
(un pid qui n'existe plus). Témoin : après `transaction_test` entier, `/tmp/rag3db` ne contient
que ce que la passe en cours ouvre ; après sa fin, rien. Propriétaire : le cœur C++ (tests du
moteur) ; un petit lot, à prendre entre deux marches.
