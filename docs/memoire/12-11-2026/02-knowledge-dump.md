# Mémoire — knowledge dump du 12 novembre 2026

## Livré et limites

Première tranche M8 livrée sur master 8b3fab422c9b481b34b9343ec4dc439846a7cc29 : API Rust de plan sur trois paquets de contenus, une base unique, empreintes BLAKE3 vérifiées et provenance seulement déclarée. Fusion JSON à trois voies (absent/null distincts, conflits structurés), tableaux/fichiers atomiques. `lecteur::Contenus` ferme le monde aux sources fournies, y compris builtin/nested ; chargeur déclaratif commun raccordé, contrôles physiques absents refusés. Les voies disque ordinaires gardent leurs contrôles. Refus conservateurs des changements de schéma, format, storage, vector_extension et seed et autres ressources protégées. Un candidat valide ne certifie pas les données ou une future ouverture.

Aucun CLI de préparation, adoption persistée, apply, politique vivant/épinglé tranchée, bascule réelle, nouveau service/GPU/modèle ni activation M9. Le [ticket M8](../tickets/2026-11-07-une-memoire-ne-sait-pas-que-ses-gabarits-sont-en-retard.md) reste ouvert ; réutiliser ce ticket. Les 50 documents sur copie et le classeur local gardent leurs autorisations distinctes, la mémoire réelle reste intacte.

## Validation et preuves

Source de rejeu809e260d4cd41aa472c02d4609e08df7b6e926b8 sur ED adcf, source finale 8b3fab422 après modifications documentaires seules. Features 5/5+minimumchecklib, lib 2201/0/65, navigateur 40/0/0,8 suites e2e, 100/0/0. Lib29 complète épinglée, -j8/Max32/targetunique32, budget de 3 600 s après poste lourd, PGprivées/Reactexistant/TMPcourt disque. Chaque vrai PID e2e attesté sousscope Max32/control-group ; BUILD_MEMORY_HIGH=0 supprime la scope sœur de run_e2e, sans relever Max32. Le marqueur de verrou suit une vraie acquisition poste lourd.

Preuves principales luciepc ~/.cache/rag3weaver-build/cx-memoire-m8-globale-4 :quatre ELF, 220 assets, compteurs, cgroups et quiescence, rawROUGE etTESTSVERT conservés, VERDICT_COMPOSE motivé admis par orchestration. Une copie entre FS 60 et 55 ne peut préserver les sockets Chromium ; renommage atomique sur même FS a préservé 1 950 entrées navigateur et 29 entrées globales (modes/inodes/liens/SHA0écart). Archive-root.json nomme les archives /var/tmp/cx-m8-preuves4.* ; copiepartielle/originaux conservés. Le scanChrome/sockets explicite est postérieur au renommage, aucun récit rétroactif. La règle TMPvide reste stricte par suite e2e ; un TMPglobal incluant les fixtures lib s’inventorie/archive après quiescence. Voir [le détail](02-plan-m8-api-et-preuves.md).

## Ressources et reprise

Les quatre worktrees mémoire/cfg aux deux postes, les deux targets PC et les quatre PG privées sont retirés. Gardes exactes 65aa/dca13 VERTES et archives administratives fidèles avant chaqueWT ;quatre dumps PG/SHA conservés. Premierrefus de garde : unique target/e2e-last.log ignoré, préservé avant nouvelle garde. Branches distantes conservées. Les snapshots locaux historiques sansWT ne sont pas une ressource active.

Nettoyage : portable ~/.cache/rag3weaver-build/preuves-m8-nettoyage-final-20261112 ; PC dossier suffixe-reprise-01, premierrefus/PGdumps dans dossier sanssuffixe. Push et3contrôles portable ~/.cache/memoire-notes/m8-integration-20261112. Copies compactes de la passe portable ~/.cache/memoire-notes/m8-preuves-globale-4. Le dernier worktree documentaire du bilan est retiré après livraison, sous garde identique.

Aucune passe ni code à reprendre sur cette tranche. La suite M8 doit définir adoption persistée, maintenance/instantané/reprise multiressources et compatibilité réelle avant application. Le cfg 80 du témoin natif est déjà sur master avec Arbre ; le ticket tty global reste ouvert pour ses deux helpers minimaux préexistants. Les tickets communs de portée e2e et de chemins de journaux restent chez leurs porteurs.
