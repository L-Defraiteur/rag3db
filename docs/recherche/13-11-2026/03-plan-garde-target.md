# Reprise durable de la garde des targets

Ticket existant : [provenance](../../tickets/closed/2026-11-11-un-target-repris-embarque-les-gabarits-d-un-autre-worktree.md).
Complément C7 ff0d6661b repris par le seul blob du ticket ; base940ad65bf.
Lot neuf cx-recherche-garde-target, aucune ressource union employée.

Avant correction, sept fonctions CPU écrites dans scripts/test_garde_target_cpu.py.
Elles exécutent la garde réelle sur des répertoires temporaires et remplacent
Cargo par une fixture qui retourne uniquement des métadonnées JSON déclarées.
Aucun compilateur, résolution réseau, base, SDK ou service sollicité.
Les refus sur target non vide sans marqueur (même REPRENDRE=1), marqueur
symlink et entrée cargo_magasin non gardée sont des rouges attendus.
Tout autre rouge arrête le lot et exige diagnostic, sans relance automatique.

Correction prévue : ne jamais inventer le propriétaire d'un target non vide ;
refuser le marqueur non régulier/symlink. Conserver la reprise explicite d'une
origine connue via la primitive existante et les dépendances externes.
Raccorder cargo_magasin (donc temoins_navigateur) à la garde avec le workspace
et le target réels de cargo metadata --no-deps ; respecter --manifest-path,
--target-dir et le séparateur -- sans confondre arguments Cargo et arguments test.
Épingler ensuite CARGO_TARGET_DIR sur la destination réellement gardée.

Recette publiée AVANT exécution : sous timeout60, python3 -B
extension/rag3weaver/scripts/test_garde_target_cpu.py -v ; journaux hors W sous
~/.cache/rag3weaver-build/journaux-cx-recherche-garde-target-13nov/.
D'abord source des témoins contre garde inchangée, échecs attendus nommés,
puis correctif publié et même suite. Aucune passe Rust/NAV autorisée ici.
Contrôles CPU bash -n/py_compile sans cache dans W et check_tickets ensuite.

Frontière : les entrées communes sont gardées ; cargo brut externe peut
contourner les scripts. Ce lot ne prétend pas interdire tout Cargo du système,
ni prouver un ELF réellement reconstruit sans qualification Rust ultérieure.
L'ancien témoin test_garde_target.sh compile une fixture Rust et n'est PAS
inclus dans cette recette CPU. Les vieux tests ne sont pas supprimés.
Ticket ouvert jusqu'à revues et qualification suffisante, aucun master/retrait.

## Correctif isolé publié après rouges attendus

Préparation29087a73d : sept fonctions jouées en0,138s ; deux passent,
les cinq autres exposent les refus manquants (quatre assertions rouges avec
sous-cas et deux fichiers marqueurs absents). Journal rouge-29087.log conservé.
Aucun rouge hors défauts prévus. Suite étendue à neuf fonctions : marqueur
vide/relatif non qualifié, et --config explicitement refusé par l'adaptateur.

cargo_garde.py utilise les métadonnées --no-deps pour workspace/target,
respecte les options manifest/target avant --, puis épingle CARGO_TARGET_DIR
et appelle la garde shell réelle. --config est refusé avant toute commande :
aucune prétention de qualifier les surcharges arbitraires Cargo. Le vieux
témoin Rust attribue désormais son target AVANT build, pas après artefacts.
Les invocations hors wrapper restent possibles et doivent appeler la garde
explicitement ; pas de partage concurrent autorisé par un marqueur.

Même recette timeout60/neuf fonctions CPU après publication de ce correctif,
puis syntaxe et tickets. Aucun témoin Rust exécuté dans ce lot préparatoire.

## Entrées suivies supplémentaires

La revue des options révèle qu'une garde initiale sur CARGO_TARGET_DIR seul
ne suffit pas si les arguments Cargo nomment une autre destination. Les appels
Cargo compilants de check_features et run_e2e passent donc aussi par
cargo_garde.py, sans réexport SDK (celui du script est conservé). NAV transite
par cargo_magasin. Les anciennes gardes initiales restent des refus précoces.
Les pilotes privés figés qui extraient run_e2e ne sont ni modifiés ni qualifiés
par ce diff ; leur évolution devra être revue séparément. L'interface confined
accepte toujours une commande, aucun changement de supervision.

Résultat CPU sur302f9b827 : neuf fonctions passées,0 échec en0,216s ;
vert-302f9.log. Nouveaux raccords shell publiés avant vérifications syntaxiques ;
aucune compilation de produit ni suite navigateur/e2e prétendue.

La fixture Cargo du témoin existant test_journaux_demons_e2e.py sait maintenant
répondre à metadata puis rendre l'artefact simulé comme auparavant. Ce témoin
compile lui-même un adaptateur Rust : NON JOUÉ dans le périmètre autorisé.
Adaptation lue et analysée AST uniquement, qualification future requise.

Contrôles sur599109a86 avant cette adaptation : bash -n cinq scripts VERT,
AST deux fichiers VERT,406 en-têtes tickets VERT, diffcheck VERT, WT propre.
Aucun Cargo réel ni rustc invoqué ; les métadonnées des neuf tests sont des
fixtures explicites. Ressources gardées : un worktree de source sans target,
journaux rouge/vert horsW ; aucun poste lourd pris. Revue indépendante demandée
avant qualification Rust et intégration ; pas de clôture du ticket ici.

Contrat des métadonnées : --offline/--locked/--frozen sont transmis aussi à
metadata, pour ne pas lever les restrictions de résolution demandées par
l'appelant avant la commande. Assertion ajoutée dans le juge existant des
options, toujours neuf fonctions. Publication avant nouvelle exécution.

## Validation finale de préparation sur aa28b6f68

Neuf fonctions CPU passent,0 échec en0,233s, journal vert-aa28.log.
Bash -n cinq scripts et AST trois fichiers Python passent ;406 en-têtes
valides (124 ouverts/282 fermés), diffcheck propre. Aucun compiler réel.
Le faux Cargo est exécuté depuis PATH privé de chaque fixture, sa commande
metadata explicitement simulée ; aucun cache produit créé ni muté.
Qualification plus large NON JOUÉE : F8, lib Rust, NAV, e2e et vieux témoin
Rust. Demande de revue de ce couple code/préparation avant recette dédiée,
avec focus paramètres Cargo, refus target inconnu et reprise connue.

## Restriction de reprise corrigée avant qualification

Archi a établi une omission dans la seconde résolution metadata de la garde.
Dixième juge publié d28fcf7ea, puis rouge attendu : --offline absent de
metadata --all-features (0,090s), journal rouge-restrictions.log. Correction :
arguments optionnels de la garde limités à offline/locked/frozen, validés avant
mutation puis transmis à sa résolution ; l'adaptateur transmet la même liste
aux DEUX résolutions. Les anciens appels à deux paramètres restent compatibles.
Même recette CPU dix fonctions après publication ; aucune compilation réelle.
Ancien candidat7cb remplacé pour revue par le correctif, aucun GO repris.
