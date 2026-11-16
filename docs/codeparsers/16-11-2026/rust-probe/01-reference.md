# Référence contrôlée pour la sonde Rust

Production inchangée depuis d395cb1f03f84c6abdfa023a0e3c7f87a7c0dab6,
codeparsers bd133850f89d6b12a3ca2b2f6ce238785ed37e3a. Seule la sonde de test
et ce document sont ajoutés/modifiés. Aucun Cargo, test ou ancienne commande
sonde.sh exécuté dans cette préparation.

Le même fichier tests/sonde_marque_nom.rs instrumenté est posé sur référence et
candidat : comptages découverts/admis/analysés/ingérés, fichiers stockés, scopes
orphelins, BUILD_ID réellement lu par le moteur, données de six champs JSONL et
TSV historique. Le profil strict RUST_PROBE_PROFILE reste optionnel pour les
anciens appelants ; le nouveau pilote l'exigera et refusera des métadonnées absentes.
Les assertions portent sur la complétude de l'entrée, pas sur un nombre de liens
présumé correct. Les références et marques seront comparées après les deux passes.

Ticket existant porté par la branche candidat :
2026-11-16-un-chemin-de-la-bibliotheque-standard-se-relie-a-un-homonyme.md,
plus le ticket des mots vides Rust. Le candidat fournit le pilote commun et
le manifeste physique1840sources. SDK59/configuration identiques requis :
les journaux et la lib373 de l'ancienne sonde ne sont pas réutilisés comme passe.

Proposition de calcul : référence puis candidat, séquentiels sur un unique target
PC, chacun sous allocation distincte à faire relire, aucun calcul autorisé par
ce document. Le candidat est la production118cc71eea avec parser34c74e4 ; ses
unités/usages verts ne prouvent pas encore les14sites/18couples historiques.
