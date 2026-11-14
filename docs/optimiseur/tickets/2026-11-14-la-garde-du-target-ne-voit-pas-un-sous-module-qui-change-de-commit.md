# La garde du target ne voit pas un sous-module qui change de commit

- **État** : ouvert (14 novembre 2026, 23 h 15), relevé de codeparsers par c7 ; corrigé sur la branche garde-sous-module (dba057156), dans la pile derrière la chaîne Rust.
- **Famille** : produit
- **Gravité** : faux rouge, ou faux vert ; une passe juge un binaire bâti en partie avec le code d'un autre commit du sous-module.
- **Atteignable en service** : non, c'est l'outillage des passes.
- **Touche rag3weaver** : oui, `extension/rag3weaver/scripts/garde_target.sh` et `scripts/cargo_magasin.sh`.
- **Pour** : optimiseur (G), lot derrière la chaîne Rust 1.98.1.
- **Ouvert le** : 14 novembre 2026

## Ce qui s'est passé

Codeparsers a eu un faux rouge : cargo a repris l'artefact d'un paquet de son sous-module (`extension/rag3weaver/codeparsers`),
bâti pour un autre commit de ce sous-module. Le checkout du commit voulu portait des dates de fichiers plus vieilles que
l'artefact, et cargo, qui juge les paquets locaux par les dates, l'a cru frais.

## Pourquoi la garde ne l'a pas vu

`garde_target.sh` note dans `<target>/.arbre-du-target` l'arbre qui a bâti le target, et refuse un autre arbre. Ici,
l'arbre était le même : seul le commit du sous-module avait changé. La garde est appelée par `check_features.sh`,
`run_e2e.sh` et `cargo_garde.py` (donc aussi par `cargo_magasin.sh`, qui passe par lui). Un `cargo test` lancé
directement ne passe par aucun d'eux, et `RAG3WEAVER_TARGET_REPRENDRE=1` n'y agit donc pas.

## La correction proposée

1. Le marqueur note aussi le commit de chaque sous-module. Quand l'un change, la garde efface d'elle-même les empreintes
   des paquets de ce sous-module (`empreintes_du_paquet`, la fonction de la reprise), puis le dit.
2. ~~`cargo_magasin.sh` appelle la garde.~~ Il l'appelait déjà, par `cargo_garde.py` (relu en écrivant le lot).
3. `garde_target.sh --reprendre <target> <arbre>` (suggestion de codeparsers), à appeler avant un `cargo` direct : le même
   effacement des empreintes des paquets locaux que `RAG3WEAVER_TARGET_REPRENDRE=1` dans les scripts.
4. Le rouge d'abord, dans `test_garde_target.sh` : bâtir, ramener le sous-module à un commit plus vieux, rebâtir. Sans
   la correction, l'artefact d'avant est repris ; avec, le paquet est rebâti. Puis un second worktree sur le même
   target, appelé par `--reprendre`.

**Les deux cas de codeparsers, ce soir**, sur le même target et avec `RAG3WEAVER_TARGET_REPRENDRE=1` :
- un `cargo test` lancé à la main dans un second worktree a repris l'artefact de codeparsers bâti dans le premier. Le
  checkout de 85fe233 avait des dates plus vieilles que le bâti de 6ba8478 ;
- un `cp -a` de restauration gardait les anciennes dates.

**Ce qui reste hors d'atteinte d'une garde** : un `cargo` direct qui n'appelle ni un script ni `--reprendre`, et une
copie qui garde ses dates. Pour eux, la règle de c7 reste la seule défense : avant un bâti hors script, toucher les
fichiers que nomme `git diff --name-only <ancien> <neuf>` du sous-module (et les sources rag3weaver changées), ou passer
par check_features.

Une règle écrite (« reprendre le target après un `git submodule update` ») aurait suffi en principe. Mais elle s'oublie, et
codeparsers s'y est trompé de bonne foi.
