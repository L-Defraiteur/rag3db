# Un témoin de liens empêche la lib sans natif de compiler

- **État** : ouvert, constaté pendant le ciblé M8 ; aucun correctif de ce fichier commun engagé dans ce lot.
- **Famille** : produit
- **Gravité** : blocage de la compilation des témoins sans feature native.
- **Atteignable en service** : non, compilation du binaire de tests.
- **Touche rag3weaver** : oui, tests de src/dataflow/record_nodes.rs et qualification des features.
- **Pour** : mémoire pour le suivi du signalement, correction à coordonner avec everything declarative.
- **Signalé par** : ciblé du [ticket M8](2026-11-07-une-memoire-ne-sait-pas-que-ses-gabarits-sont-en-retard.md), base 6e5f9458c, source jouée 59b3f0ff7.

## Preuve

À luciepc, sous poste lourd, target privé cx-target-memoire-m8-plan et -j8 :
`cargo test --no-default-features --lib template_plan::tests::a_rejected_candidate_does_not_extract_a_builtin_cache -- --exact --nocapture`.
Journal externe `~/.cache/rag3weaver-build/cx-memoire-m8-rouge.log` : E0432, import `crate::Rag3dbConnection` absent à src/dataflow/record_nodes.rs:4570 ; ce type est conditionné à rag3db-native dans lib.rs. Le test `l_annulation_d_un_lot_de_liens_ne_retire_que_ce_qu_il_a_cree` n’a pas cette condition. Compilation arrêtée, code101, aucun témoin M8 joué.

## Reste et fin

Qualifier précisément ce témoin natif ou fournir une autre voie réellement testée, sans retirer ni affaiblir ses assertions. Vérifier la compilation de la lib de tests sans natif et conserver l’exécution effective du témoin lorsque rag3db-native est actif. Le signalement et le passage du ciblé M8 au jeu natif ne ferment pas ce défaut. La correction d’un fichier hors périmètre doit être coordonnée avant modification.
