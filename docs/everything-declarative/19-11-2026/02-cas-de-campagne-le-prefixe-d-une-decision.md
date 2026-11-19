# Cas de campagne : deux décisions sur le même texte ne partagent pas leur préfixe

```json
{"parent": "f063a90e52fd70f61005b66ff93cf979735de33a", "tree": ".", "needs_lib": false, "witness": "dataflow::decide_nodes::tests::two_decisions_on_the_same_text_begin_with_the_same_bytes", "test": "cd extension/rag3weaver && cargo test --lib -- dataflow::decide_nodes::tests::two_decisions_on_the_same_text_begin_with_the_same_bytes", "suites": ["cd extension/rag3weaver && cargo test --lib -- dataflow::decide_nodes::"], "hints": ["extension/rag3weaver/src/dataflow/decide_nodes.rs"], "ours": "1bba2216895c22a59a958058026b5c7e83330e07"}
```

Session everything declarative, 19 novembre 2026, pour la campagne des correctifs passés. C'est un cas court, en Rust,
dans le crate rag3weaver, sans aucune feature. Le modèle y est un faux, qui garde ce qu'on lui envoie : ni la lib
rag3db, ni service.

## Le point de départ

- **Commit parent** : `f063a90e52` (19 novembre 2026, 13 h 30). Le témoin y est, rouge ; le correctif n'y est pas.
- **Arbre** : `.`, pour la même raison que le premier cas (dépendances par chemin hors du crate, sous-module
  `codeparsers`).

## Le signalement, tel qu'il était

> Un nœud `decide` qui classe un texte par un modèle envoie deux tours : un tour système et un tour utilisateur. Les
> fournisseurs (llama-server, Gemini, Anthropic) ne servent leur cache que sur un préfixe identique, octet pour octet.
> Deux décisions sur le même texte long, avec deux jeux d'étiquettes candidates différents, devraient donc commencer
> par les mêmes octets jusqu'au texte compris. Ce n'est pas le cas : les étiquettes sont dans le tour système, avant
> le texte, et le préfixe commun s'arrête à 50 octets. Le texte entier est perdu pour le cache à chaque nouveau jeu de
> candidats. Le témoin `two_decisions_on_the_same_text_begin_with_the_same_bytes` fait deux appels à un faux modèle et
> compare ce qu'il a reçu. Un nœud qui porte son propre gabarit (`template`) ordonne lui-même ses trous et n'est pas
> concerné.

## Le témoin

- **Filtre** : `dataflow::decide_nodes::tests::two_decisions_on_the_same_text_begin_with_the_same_bytes`, dans
  `extension/rag3weaver/src/dataflow/decide_nodes/tests.rs`.
- **Suite** : `cargo test --lib` du crate, sans feature ; le module `dataflow::decide_nodes::` entier comme suite
  exigée. Aucun chemin du correctif n'est dans `tools/poste/required_suites.txt`.
- **Rouge au parent** : le préfixe commun des deux envois s'arrête à 50 octets, avant le texte.
- **Sur master** : identique quand le lot `ed-decider-prefix` aura atterri ; il est en file au pousseur.

## Ce qu'une résolution doit tenir

La décision reste juste : les étiquettes admises sont toujours dites au modèle, et le schéma de réponse les contraint
toujours. Un gabarit garde son ordre. Les autres tests de `decide_nodes` restent verts.

## Notre correctif (caché à Algareth, pour le juge)

`1bba221689`, « fix(rag3weaver): a decision by the model sends the stable first » :
`extension/rag3weaver/src/dataflow/decide_nodes.rs`, +13 −2, et `decide_nodes/tests.rs`, +2 −1. Le tour système ne
garde que l'instruction et la forme, par une nouvelle méthode `stable_instruction`. Le texte puis les étiquettes
admises ferment le tour utilisateur. **Une fuite possible dans l'arbre** : la page
`docs/everything-declarative/18-11-2026/05-…` (la composition des invites pour le cache) est au parent et dit ce
principe. Le juge le sait : une résolution qui la cite l'a lue.
