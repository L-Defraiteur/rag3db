# Une lib d'une autre ABI se charge et tombe sur un symbole manquant

- **État** : ouvert (11 octobre 2026) — au nom du cœur C++, ouvert par l'arbre principal à la demande de l'orchestration
- **Gravité** : blocage (un hôte installé ne démarre plus, avec un message qui ne dit pas pourquoi)
- **Atteignable en service** : oui, à chaque rebâti de la lib partagée qui change une signature exportée, pour tout binaire lié à la lib d'avant
- **Touche rag3weaver** : oui (les binaires installés : `rag3weaver-backend`, les hôtes)

## Ce que c'est

B2 (`0a630b764`) a ajouté un paramètre au constructeur de `SystemConfig`, que `tools/rust_api`
emploie. Après le rebâti de la lib partagée du poste principal (20 h 39), un binaire lié à la lib
d'avant qui démarre charge la lib neuve par son chemin et tombe sur un symbole manquant (sur
luciepc, l'hôte s'est éteint et ne redémarrait plus). Rien ne dit « ce binaire a été bâti contre
une autre lib » : on lit une erreur de l'éditeur de liens dynamique.

## Recette minimale

Bâtir `rag3weaver-backend` contre une lib d'avant `0a630b764`, rebâtir la lib après, relancer le
binaire sans le rebâtir.

## Ce qu'il faut pour le fermer

L'identifiant de bâti existe désormais (`CALL build_id()`, `48b74ead1`). Le binaire garde
l'identifiant de la lib contre laquelle il a été bâti (constante écrite au bâti par
`tools/rust_api/build.rs`), et à l'ouverture de la première base il le compare à celui de la
lib chargée : différents, un refus nommé (« ce binaire a été bâti contre la lib <id>, la lib
chargée est <id> : rebâtissez-le ») au lieu d'un symbole manquant. Le magasin de libs par
identifiant de bâti, en discussion à l'orchestration, ferait disparaître le cas.

Témoin : un binaire de test bâti avec un identifiant attendu faux refuse en nommant les deux.

## Réserve lue le 12 octobre 2026 (arbre principal, 2 h 34)

La surcharge de l'ancienne signature de `SystemConfig` (lot ABI, master 2bc7b4a3b) fait **se
charger** un binaire d'avant B2, pas **tourner** : l'ancien binaire de Lucie du 11 octobre 13 h 21
(`~/.local/bin/rag3weaver-backend.avant-marche-4`), lancé sur une lib qui porte la surcharge, plante
en `malloc(): invalid size (unsorted)` à l'ouverture — la **disposition** de `SystemConfig` a changé
avec `fileLockWaitMs`, et le binaire d'avant la lit avec son ancienne taille. Règle : **un changement
de disposition d'un type exporté est un changement d'ABI ; la surcharge n'y suffit pas ; le refus
nommé fait foi** (la section ELF `.rag3db_build_id` de la recherche et `rag3db-check-lib`, qui
refusent en nommant les deux identifiants avant le chargeur, plus le filet dans le processus). La
surcharge reste utile aux binaires qui ne touchent pas à la disposition (ceux qui ne construisent
pas `SystemConfig` eux-mêmes). Rien à changer dans le code.
