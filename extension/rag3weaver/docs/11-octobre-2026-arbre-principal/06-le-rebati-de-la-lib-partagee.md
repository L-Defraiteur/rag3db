# Le rebâti de la lib partagée du poste principal

*Arbre principal, 11 octobre 2026. La façon de faire, après trois rebâtis dans la journée
(17 h 39, 20 h 08, 20 h 39). C'est l'arbre principal qui rebâtit ; les autres sessions
demandent.*

## Ce qui est rebâti

`build/lecteurs-csv` dans l'arbre principal : `src/librag3db.so` et
`extension/vector/build/libvector.rag3db_extension` (l'extension sort dans l'arbre source).
Toutes les sessions lient la lib par `RAG3DB_LIBRARY_DIR` et chargent l'extension par
`RAG3DB_ROOT` : un rebâti les touche toutes.

## Quand

- Après un lot moteur que les sessions doivent voir, sur décision de l'orchestration ; un seul
  rebâti pour plusieurs lots rapprochés.
- Toujours quand la tête de master ne compile plus contre la lib (B2 : `tools/rust_api`
  employait un champ que les en-têtes de la lib n'avaient pas).
- Entre deux rebâtis, un lot qui ne touche que le produit passe avec
  `RAG3WEAVER_MOTEUR_ANCIEN=1`, déclaré dans son annonce.

## Comment

1. Demander « fini » aux sessions qui chargent la lib (aujourd'hui : mémoire longue, recherche,
   codeparsers, embarquements ; les autres ont dit de ne pas demander). Un push en file qui
   rejouerait des suites se retire jusqu'à l'annonce : `poste lourd` est partagé et ne protège
   pas d'un bâti.
2. `git merge --ff-only` de l'arbre principal sur `FETCH_HEAD` de master (jamais rien d'autre :
   l'arbre est partagé).
3. `cmake --build build/lecteurs-csv -j 8` sous `poste lourd`
   (`~/.cache/rag3weaver-notes-A/lib/batir.sh`).
4. **Publier au magasin**, seulement si le bâti est vert (décision de l'orchestration, 12 octobre
   2026) : `python3 extension/rag3weaver/scripts/rag3db_libs.py publier build/lecteurs-csv --racine
   <arbre principal>`. Le script copie la lib, les en-têtes, `build_id.h` et l'extension dans
   `~/.cache/rag3db-libs/<identifiant>/`, vérifie que la lib et l'extension viennent du même bâti,
   et pose `courant`. Un refus (code 2) s'annonce tel quel : on ne pose rien à la main.
   `run_e2e.sh` lit le magasin par défaut : une passe e2e n'a plus à dire « fini » avant un
   rebâti. Les hôtes et les binaires installés, eux, chargent toujours `build/lecteurs-csv`.
5. Annoncer : l'heure, les deux md5, l'identifiant publié (`rag3db_libs.py courant`), la tête
   bâtie, le dernier commit moteur (les fichiers doivent être plus récents), et ce qui reste sur
   l'ancienne image.

## Ce qui tourne pendant le bâti

GNU ld écrit un **nouvel inode** à chaque édition de liens (vérifié : une `.so` liée deux fois
change d'inode). Un processus qui tient la lib (l'hôte de Lucie, les clients MCP de ses
sessions) garde l'ancienne image et ne tombe pas ; il prend la lib neuve à son redémarrage. Un
processus qui **démarre pendant** le bâti peut, lui, charger un fichier à moitié écrit : c'est
pour lui qu'on demande les « fini ».

## Quand l'ABI change

Un rebâti qui embarque un commit touchant `tools/rust_api` ou une signature exportée (B2 :
un paramètre de plus au constructeur de `SystemConfig`) **change l'ABI**. L'annonce le dit :

> ABI changée : rebâtissez vos binaires installés avant de relancer leurs hôtes.

Un binaire lié à l'ancienne lib qui redémarre tombe sur un symbole manquant (luciepc, le
11 octobre au soir). Les hôtes et les clients en cours continuent sur l'ancienne image ; on ne
les relance qu'après avoir rebâti leur binaire. Le refus nommé à la place du symbole manquant
est un ticket moteur (`docs/tickets/2026-10-11-une-lib-d-une-autre-abi-se-charge-et-tombe-sur-un-symbole.md`).
