# Le test du manifeste lié demande code sans garde de feature

- **État** : ouvert (11 novembre 2026).
- **Famille** : produit
- **Gravité** : vérification bloquée dans la configuration rag3db-native sans code ; aucun défaut en service établi.
- **Atteignable en service** : non, témoin unitaire ; le refus du manifeste sans feature est attendu.
- **Touche rag3weaver** : oui, src/builtin_templates.rs, test a_linked_manifest_loads_and_a_modified_copy_is_reported.
- **Pour** : everything-declarative, avec orchestration si le porteur doit changer.
- **Signalé par** : arbre-principal, [reprise du lot 1a](../../arbre-principal/11-11-2026/01-rapport-de-session.md).

## Constat et preuve

La passe `scripts/cargo_magasin.sh test -j8 --lib --features rag3db-native` sur le lot arbre-principal
(49caf64c6 avec le diff de projection et de transaction en cours) donne 1 565 réussis, 1 échoué,
15 ignorés. Le seul échec est builtin_templates::tests::a_linked_manifest_loads_and_a_modified_copy_is_reported :
`workspace` exige un binaire avec la feature code, alors que le test n'a qu'un attribut test.
Il prépare explicitement templates/backends/code/backend.json, puis unwrap PreparedBackend::load.
Le refus produit est correct ; le montage de cette configuration de test ne l'est pas.

Journal luciepc hors target : ~/.cache/rag3weaver-build/cx-projection-natif-regressions.log.
Source : src/builtin_templates.rs autour de 521–529 dans extension/rag3weaver.
Ce fichier n'est pas modifié par le lot 1a ; aucune régression de recherche n'est établie par cet échec.

## Travail attendu et fin

Confirmer le contrat de la matrice de features et vérifier le témoin sur sa base, puis le borner
à code ou employer un manifeste portable qui conserve son intention. Prouver la configuration
sans code et celle avec code, sans affaiblir le refus d'un workspace par un binaire qui ne le sait pas.
Ne pas marquer la passe native précédente verte. Le transfert ou le simple signalement ne ferme pas ce ticket.
