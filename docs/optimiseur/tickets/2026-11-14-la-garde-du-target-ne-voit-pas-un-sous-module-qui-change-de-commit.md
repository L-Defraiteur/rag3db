# La garde du target ne voit pas un sous-module qui change de commit

- **État** : ouvert (14 novembre 2026, 23 h 15), relevé de codeparsers par c7.
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
l'arbre était le même : seul le commit du sous-module avait changé. Et la garde n'est appelée que par `check_features.sh`,
`run_e2e.sh` et `cargo_garde.py`. Un `cargo_magasin.sh test` seul ne passe pas par elle, et `RAG3WEAVER_TARGET_REPRENDRE=1`
n'y agit donc pas.

## La correction proposée

1. Le marqueur note aussi le commit de chaque sous-module. Quand l'un change, la garde efface d'elle-même les empreintes
   des paquets de ce sous-module (`empreintes_du_paquet`, la fonction de la reprise), puis le dit.
2. `cargo_magasin.sh` appelle la garde.
3. Le rouge d'abord, dans `test_garde_target.sh` : bâtir, ramener le sous-module à un commit plus vieux, rebâtir. Sans
   la correction, l'artefact d'avant est repris ; avec, le paquet est rebâti.

Une règle écrite (« reprendre le target après un `git submodule update` ») aurait suffi en principe. Mais elle s'oublie, et
codeparsers s'y est trompé de bonne foi.
