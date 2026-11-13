# Reprise durable de la garde des targets

Ticket existant : [provenance](../tickets/2026-11-11-un-target-repris-embarque-les-gabarits-d-un-autre-worktree.md).
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
