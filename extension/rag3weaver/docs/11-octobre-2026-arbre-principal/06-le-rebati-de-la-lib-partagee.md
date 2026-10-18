# Le rebâti de la lib partagée du poste principal

*Arbre principal, 11 octobre 2026. La façon de faire, après trois rebâtis dans la journée
(17 h 39, 20 h 08, 20 h 39). C'est l'arbre principal qui rebâtit ; les autres sessions
demandent.*

*Revue le 18 octobre 2026 : depuis la règle de Lucie du 13 octobre, son arbre
(`~/git_workspaces/rag3db`) reste sur master et ne porte aucun bâti. Le rebâti part d'un worktree
de l'arbre principal et ne sert plus que par le magasin. Le jeton du lourd (une compilation à la
fois par poste) se demande à l'orchestration.*

## Ce qui est rebâti

`build/lib` dans le worktree `~/.cache/rag3weaver-build/wt-lib` de l'arbre principal, détaché sur
le commit à bâtir : `src/librag3db.so` et `extension/vector/build/libvector.rag3db_extension`
(l'extension sort dans l'arbre source, donc dans le worktree). La configuration est celle de
l'ancien `build/lecteurs-csv` : Release, l'extension vector, l'en-tête unique, ni tests ni shell.

Personne ne lie ce dossier. Les sessions lisent le **magasin**
(`~/.cache/rag3db-libs/<identifiant>/`, `courant` par défaut) par `scripts/cargo_magasin.sh`,
`run_e2e.sh` et `check_features.sh`. Les binaires installés suivent un identifiant fixe par leur
RUNPATH. Un rebâti ne touche donc personne avant que `courant` ne change, et ne touche pas les
binaires installés du tout.

Avant le 13 octobre, c'était `build/lecteurs-csv` dans l'arbre partagé.

## Quand

- Après un lot moteur que les sessions doivent voir, sur décision de l'orchestration ; un seul
  rebâti pour plusieurs lots rapprochés.
- Toujours quand la tête de master ne compile plus contre la lib (B2 : `tools/rust_api`
  employait un champ que les en-têtes de la lib n'avaient pas).
- Entre deux rebâtis, un lot qui ne touche que le produit passe avec
  `RAG3WEAVER_MOTEUR_ANCIEN=1`, déclaré dans son annonce.

## Comment

La recette est `~/.cache/rag3weaver-notes-A/lib/batir.sh [<commit>]`, origin/master par défaut.

1. **Le jeton du lourd**, demandé à l'orchestration. Aucun « fini » à demander aux sessions :
   personne ne charge le dossier de bâti, et une passe e2e a résolu son identifiant au départ.
2. **Le worktree**, créé détaché au premier rebâti puis replacé (`checkout --detach`) sur le
   commit voulu. Il refuse de bâtir s'il porte des modifications de `src/`, `extension/vector/` ou
   `tools/`. L'arbre de Lucie n'est jamais touché : ni checkout, ni merge, ni fichier.
3. **`cmake --build build/lib -j 8` sous `poste lourd`**, configuré au premier passage.
4. **Les symboles, par nom**, contre la lib `courant` du magasin. Des ajouts seuls laissent
   l'ABI tenir. Un retrait change l'ABI : la lib est alors publiée **sans poser `courant`**
   (`--sans-courant`), et `courant` ne se pose qu'après l'annonce.
5. **Publier au magasin**, seulement si le bâti est vert : `rag3db_libs.py publier build/lib
   --racine <worktree>`. Le script copie la lib, les en-têtes, `build_id.h` et l'extension dans
   `~/.cache/rag3db-libs/<identifiant>/`, vérifie que la lib et l'extension viennent du même bâti,
   et pose `courant`. Un refus (code 2) s'annonce tel quel : on ne pose rien à la main.
6. **Annoncer** :
   - l'heure, les deux md5 et l'identifiant publié (`rag3db_libs.py courant`) ;
   - la tête bâtie et le dernier commit moteur ;
   - les symboles ajoutés et retirés ;
   - si l'ABI change, qui doit rebâtir ses binaires.

Le worktree et son `build/` sont gardés entre les rebâtis, sous le plafond de 20 Go, et effacés en
fin de session.

## Ce qui tourne pendant le bâti

GNU ld écrit un **nouvel inode** à chaque édition de liens (vérifié : une `.so` liée deux fois
change d'inode). Un processus qui tient la lib (l'hôte de Lucie, les clients MCP de ses
sessions) garde l'ancienne image et ne tombe pas ; il prend la lib neuve à son redémarrage. Un
processus qui **démarre pendant** le bâti pouvait, lui, charger un fichier à moitié écrit. C'est
pour lui qu'on demandait les « fini ». Depuis le magasin, personne ne charge le dossier de bâti :
le dossier d'un identifiant est écrit à part puis renommé d'un coup.

## Quand l'ABI change

Un rebâti qui embarque un commit touchant `tools/rust_api` ou une signature exportée (B2 :
un paramètre de plus au constructeur de `SystemConfig`) **change l'ABI**. L'annonce le dit :

> ABI changée : rebâtissez vos binaires installés avant de relancer leurs hôtes.

Un binaire lié à l'ancienne lib qui redémarre tombe sur un symbole manquant (luciepc, le
11 octobre au soir). Les hôtes et les clients en cours continuent sur l'ancienne image ; on ne
les relance qu'après avoir rebâti leur binaire. Le refus nommé à la place du symbole manquant
est un ticket moteur (`docs/tickets/2026-10-11-une-lib-d-une-autre-abi-se-charge-et-tombe-sur-un-symbole.md`).
