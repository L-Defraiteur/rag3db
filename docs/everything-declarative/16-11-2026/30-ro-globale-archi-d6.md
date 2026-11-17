# RO totale du pilote global Archi et D6

Lecture statique du 17 novembre, APP
`f1e8a8d502614d10f7cdb40eaea330f08b5d3f37`, PILOTE
`dc868252af9015e6069ef9cb872b41eb80da22c5`, recette33 et tous les modules
`docs/rag3weaver-archi/16-11-2026/global-pass/`. Recoupement avec le vrai
SandboxConfig/Landlock, les CLI host/TTY, garde_target, run_e2e et canonique2150
de l'APP. Aucun module importé/exécuté, aucun témoin CPU, /proc, Cargo, natif,
modèle, DB, service ou transition PC joué. Paquet PC final non fourni : aucun
pin de paquet validé par cet avis.

Avis **réservé**, quatre points groupés avant correction, sous le
[ticket existant Archi](../../rag3weaver-archi/tickets/2026-11-16-le-temoin-documentaire-vertex-attend-un-accord-sans-superviseur.md).

## P1 : la garde du broker est hors capacités de lecture Landlock

`broker.py:67` retourne `C/scripts/garde_target.sh`, avec C dans le WT source
physique, extérieur à la copie D6 et aux domaines de tâche/documentaire.
`cargo_shim.py:21-25` source ce fichier dans le processus déjà confiné, avant
le vrai Cargo. `policy.py:61-71` ajoute seulement Q, socket_root et SDK aux
lectures du template. Celui-ci accorde Rust/Cargo, pas C ni ce script.

Le vrai `BacASable::autour` accorde le domaine en écriture, le système en
lecture, puis les seuls extra_read ; `avec_lecture_git` ne donne pas lecture
aux scripts du WT source. Le chemin fourni reste donc refusé avant Cargo et
ledger. Constat statique, aucun échec Landlock réellement joué ici.

À faire par Archi : matérialiser et épingler la garde réelle dans une capacité
de lecture déjà admise, ou déclarer sa lecture minimale avant HEAD/task ;
recouper aussi ses éventuelles dépendances. Ne pas accorder le WT entier en
écriture ni remplacer la primitive par une simulation. Un témoin hermétique
sur le vrai constructeur doit couvrir le chemin réellement fourni au shim,
et les deux rôles D6/documentaire ; recette native exacte relue ensuite.

## P1 : le rouge D6 n'est pas lié à l'assertion du contrat

`policy.py:93-100` exige le nom exact, 0/1/0 et la sous-chaîne `panicked at`,
mais aucune cause d'assertion. `broker.py`, branche tested, qualifie ensuite
ce retour101 en agent-red. Le texte d'un `panic!("panne indépendante")` au
même nom satisferait ces conditions, sans prouver le refus de « Om bre ».
Le reviewer ultérieur démontre le comportement final, pas la cause du rouge
historique. La recette33 promet pourtant un rouge ASSERT précis.

À faire : preuve figée du test/source rouge et de sa cause attendue, puis
contre-exemples rejetant panic sans rapport, panne d'initialisation et autre
assertion sous le même nom. Conserver le vrai rc101 et l'unique juge exact.
Contre-exemple raisonné par lecture, NON EXÉCUTÉ ici.

## P2 déjà C7 : témoin Cargo-stop non raccordé au pilote actuel

`test_cargo_stop.py:82-93` lit `HERE/driver.py`, absent des 17 modules.
Il cherche encore SDK_PROFILE/RUST_LIB/METADATA_BEFORE_EXPORT qui ne sont pas
les politiques déclarées de ce pilote. Échec FileNotFound prévisible, pas
observation rouge réalisée. Raccorder le juge aux vrais profils/constructeurs
actuels et aux phases SDK ; ne pas ajouter un driver factice pour le satisfaire.

## P2 : hash de provenance du contrôleur périmé

`provenance.json:21` annonce destination controllers.py
`731e219bc889853c8d6694c2a895e4b89f57ccaad5e0ab4241dfdeb829b8ef17`.
Le blob réellement publié dans dc868 est
`f763f5a9cf8800576f910d498c2d905e0ffdc50b2f9dddf3d90f67d9e2cfe916`.
Le dernier commit normalise précisément la fin de ce fichier. Les hashes des
copies à préparer seront calculés depuis les vrais octets, mais cela ne rend
pas exacte cette provenance publiée. Actualiser et contrôler tous les
destination_sha256 contre les blobs finaux avant gel du paquet.

## Frontières relues sans autre blocage établi

Constructeur partagé lanceur/watchdog/poste, vraie entrée mesure exclusive,
quota800%/jobs8, timeout7440+kill15, Common<=7200 et phases uniques MIN END-15
présents. Leur coût réel n'est pas qualifié. T0 figé avant/après transition,
archive742 indépendante, scan frais et UNKNOWN attribués bloquants, reprise
réelle ensuite. Nouvelle tâche dans copie privée, manifeste/Sandbox avant
HEAD et ouverture, refus rm Cargo.lock lié, aucune autorisation générique.

Capture Cargo pidfd/STOP/threads et reprise finally, refus fatal au niveau
Observer, snapshot immuable avant jugement ; nettoyage sans attestation SDK.
Polling non exhaustif explicitement limité. Les profils réels restent à
observer, aucune lecture statique ne supprime la course de chargement runtime.

Cinq ELF CargoJSON/closures, 223 sources Git sur les trois ELF de tests,
inventaires avant juges, publication deux hôtes avant marqueurs et D6,
retours directs du vrai ELF dans Landlock. Canonique2150 byte-épinglée,
inventaires/noms/filtres et résumés par phase, distinction enfants/top-level,
F8/proof/whole-lib/NAV et SDK runtime exigés. Final scope/quiet/hash et contrôle
après fsync présents, premier rouge exclusif ; aucun GO d'exécution déduit.

Suite : Archi traite ces réserves ENSEMBLE dans le ticket avant diff, publie
nouveau SHA, puis RO delta et pins PC exacts. CPU hermétique seulement sur
décision C7 distincte ; aucun transfert de cet avis vers un futur couple.
