# Un binaire lié à une autre lib échoue sur un symbole manquant au lieu d'un refus nommé

- **État** : ouvert (11 octobre 2026, 21 h ; relevé par codeparsers sur l'hôte de luciepc, après le
  rebâti de la lib commune sur B2). Propriétaire : le cœur C++, après les petites cardinalités.
- **Gravité** : confort / exploitation — l'hôte de luciepc (binaire lié à la lib de b4655da39) a
  bouclé au démarrage sur « undefined symbol: _ZN6rag3db4main12SystemConfigC1Emmbbmbmbbb » après que
  la lib commune a été rebâtie sur 0a630b764 (B2 ajoute `fileLockWaitMs` au constructeur de
  `SystemConfig`) ; l'unité systemd relançait en boucle ; personne ne savait d'où venait le
  symbole sans lire le mangling.
- **Atteignable en service** : oui, à chaque lot qui change une signature exportée (`SystemConfig`,
  `Database`, `Connection`, le C API) suivi d'un rebâti de la lib sous un binaire installé.
- **Touche rag3weaver** : oui (l'hôte, le client, le paquet npm, python, node : tout ce qui lie la lib).

## Ce que c'est

Le contrôle d'identifiant de bâti (`cmake/build_id.cmake`, `RAG3DB_BUILD_ID`) protège les
**extensions** : `LOAD EXTENSION` refuse en nommant les deux identifiants. Il ne protège pas les
**binaires qui lient la lib** : l'éditeur de liens dynamique découvre l'écart au premier symbole
absent, sans dire ni quel bâti le binaire attendait ni lequel la lib porte.

## Forme proposée

Le binaire lie l'identifiant du bâti contre lequel il a été compilé (l'en-tête généré le donne
déjà aux bindings, `rag3db_rs.cpp` peut le porter), et une fonction de la lib —
`rag3db::main::checkLinkedBuild(const char* expected)` ou le `CALL build_id()` d'hotfixs lu avant
tout — compare à l'ouverture de la première `Database` : refus nommé « ce binaire a été bâti contre
rag3db-build-X, la lib chargée est rag3db-build-Y : rebâtir le binaire », avant tout symbole
manquant (le symbole manquant restera quand la signature change, mais le message nommé vient en
premier si la fonction de contrôle existe dans les deux bâtis). Côté exploitation : l'annonce
« lib faite » dit **« ABI changée »** quand le lot touche `tools/rust_api` ou une signature exportée,
et les binaires installés se rebâtissent avant la relance de leurs hôtes (règle de l'orchestration,
11 octobre, 21 h). Et garder l'ancienne signature en surcharge quand on ajoute un paramètre à
`SystemConfig` (elle délègue à la neuve) : un binaire d'avant continue de se lier.

## Témoin

À écrire : un petit programme lié à la lib avec un identifiant de bâti forcé différent, lancé
contre la lib : il sort avec le refus nommé, code non nul, avant d'ouvrir une base.
