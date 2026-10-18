# La CI de rag3weaver était rouge depuis le 3 septembre sans que personne le voie

- **État** : ouvert (18 octobre 2026, G) — remise en état sur la branche `ci-rag3weaver` ; à fermer sur un run vert de bout en bout sur master, lu
- **Gravité** : filet absent. Rien ne cassait dans le produit, mais aucun contrôle automatique ne gardait le crate pendant six semaines
- **Atteignable en service** : non, c'est l'outillage
- **Touche rag3weaver** : oui (`.github/workflows/rag3weaver-workflow.yml`, `scripts/check_features.sh`)

## Ce qu'on a vu

`.github/workflows/rag3weaver-workflow.yml` compte 1 108 runs : 991 rouges, 115 annulés, 2 verts.

- Les deux derniers verts datent du 23 août 2026 (run 32664932135, 962ce9dc2).
- **Premier rouge : le 3 septembre 2026 à 13 h 40 UTC** (run 33762456387, d9e3c2631). Depuis, tous les jobs rougissaient à la même étape.

L'étape fautive clonait lucivy « à la révision du sous-module », dans `../lucivy` depuis `extension/rag3weaver`. Or ce chemin est `extension/lucivy`, le dossier du sous-module lui-même :
`fatal: destination path '../lucivy' already exists and is not an empty directory`.

Derrière cette étape, d'autres rouges plus anciens attendaient, cachés par elle (run 39132590401, le 18 octobre) :

- six jobs : `no matching package named codeparsers`. Le workflow n'initialisait pas le sous-module codeparsers, une dépendance par chemin que cargo exige même sans la feature `code` ;
- deux jobs demandaient des features retirées du crate : `candle-wasm` et `wasm-emscripten` ;
- le job « fmt + clippy » passait pour vert par `continue-on-error`, alors que fmt et clippy rougissaient sur le passif.

## Pourquoi personne ne l'a vu

Personne ne lisait ce workflow. Chaque session contrôle sur son poste (`check_features.sh`, les e2e), et les seuls runs lus étaient ceux du paquet npm. Un vert masqué par `continue-on-error` aurait de toute façon trompé la lecture.

## La remise en état (branche `ci-rag3weaver`)

- Le sous-module lucivy est retiré (284cc00c0) et lucivy vient de crates.io : plus de clone.
- codeparsers est initialisé par https, comme dans `paquet-npm.yml`.
- La matrice est remplacée par `scripts/check_features.sh`, dont les jeux font foi, avec le moteur bâti en statique par la crate rag3db (`RAG3DB_LIB=statique`, option ajoutée au script). En CI, la sortie de chaque jeu est gardée (`CHECK_FEATURES_LOGS`) et un rouge s'affiche.
- Les tests unitaires se jouent sous le jeu du paquet (`rag3db-native,code,daemon,openai-llm`), dans le même job. Sans feature, cinq tests qui supposent `code` rougissent, car ils ne sont pas gardés par cfg : `backend::tests::{index_signals_se_valide_dans_les_deux_sens, la_politique_refuse_en_disant_quoi_faire, locks_via_exige_le_schema_de_code}`, `dataflow::graph_tool::tests::chaque_gabarit_d_outil_livre_se_construit`, `gabarits::tests::chaque_gabarit_livre_se_charge` (run 39133418842). Les garder, ou leur donner un sens sans `code`, reste à faire.
- fmt + clippy est retiré, avec un commentaire : il revient sans `continue-on-error`, une fois le crate formaté et clippy propre.

## Le filet qui manquait

Qu'un rouge de ce workflow sur master se voie :
- une ligne dans le rapport de l'orchestration ;
- ou une notification au premier rouge, après un vert.

À décider par l'orchestration.

## Pour le fermer

Un run vert de bout en bout sur master, lu (job `check_features (cinq jeux) et tests unitaires`). Puis le filet décidé.
