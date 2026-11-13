# La boîte ne compile pas sous la feature daemon seule

- **État** : ouvert ; relevé pendant la qualification CPU de résidence.
- **Famille** : produit
- **Gravité** : blocage ; compilation de la bibliothèque avec daemon seul impossible.
- **Atteignable en service** : non ; refus de compilation.
- **Touche rag3weaver** : oui, src/mail.rs et garde de feature daemon_lifecycle dans lib.rs.
- **Pour** : everything declarative, propriétaire boîte/hôte.
- **Signalé par** : embarquement, qualification du ticket d'activation des modèles.
- **Ouvert le** : 12 novembre 2026

## Preuve

Sur f3f8dc80f, luciepc, `cargo test --lib --features daemon
model_source::activation::tests -- --nocapture` échoue avant exécution :
`error[E0433]: cannot find daemon_lifecycle in crate` à `src/mail.rs:120:28`.
`daemon_lifecycle` est gardé par `all(daemon, rag3db-native)` dans lib.rs:247.
Le fichier mail.rs n'est pas modifié par le lot d'embarquement.
Log : `~/.cache/embarquement-notes/activation/cpu-b.log` à luciepc.

## Reste à faire

Qualifier le contrat de features de la boîte : rendre le minimum daemon
compilable ou refuser explicitement cette combinaison si elle n'est pas supportée,
sans importer le moteur natif pour masquer une dépendance de déclaration.
Vérifier les jeux supportés ; ne pas déclarer les témoins d'activation verts
sur cette tentative qui n'a joué aucun test. La suite d'embarquement utilisera
le jeu natif/daemon supporté, avec lib épinglée.

## Fin attendue

Correction et compilation du jeu minimal retenu, puis preuve du comportement
de découverte de la boîte sous ses features réellement nécessaires.

## Deuxième combinaison relevée, même frontière de features

Sur 3520ec019, `rag3db-native,daemon` échoue dans
`daemon_host/chat.rs:835-836` (fichier_nodes exclu sans code) et import Boite
inutilisé sous -Dwarnings dans `daemon_host/livraison_codex.rs:92`.
Log luciepc `~/.cache/embarquement-notes/activation/cpu-c.log`.
Ces fichiers ne sont pas modifiés par la tranche d'embarquement.
La prochaine qualification utilise `rag3db-native,code,daemon`, jeu supporté
par les passes usuelles, avec la lib épinglée et aucun GPU requis.

## Reprise autorisée après la livraison du parcours IDE — 13 novembre

Les preuves initiales viennent de l'ancienne source ; vérifier d'abord les
frontières de features sur le master qui intègre ED43e4f185b et Embar23ceed87b.
ED reprend ce défaut existant après son nettoyage terminé, dans un worktree
neuf. Le périmètre est boîte, lifecycle et usages d'outils sous leurs features
réellement nécessaires ; ni nouveau comportement d'agent ni feature native
ajoutée par défaut pour cacher la dépendance.

Publier le diagnostic actuel, le contrat de features retenu et la recette CPU
bornée avant compilation. Corriger les gardes/imports si les combinaisons sont
bien supportées, puis prouver le jeu minimal sans natif, daemon seul et
daemon+natif sans code, ainsi que le parcours de découverte concerné. Un refus
volontaire doit être explicite et cohérent avec le contrat publié, pas un E0433
accidentel ni une exclusion silencieuse des témoins. Pas de modèle/service/PG
pour ce ciblé ; source figée, poste et plafond mémoire réels, lib épinglée pour
la combinaison native. Premier rouge inattendu arrêté et diagnostiqué.

Coordonner les rares fichiers partagés avec Archi ; son lot grep garde ses
outils/Plans. Aucun ancien worktree nettoyé n'est rouvert, aucune pose du daemon
personnel. Revue et passe adaptée avant tour master ; clôture seulement après
correction et preuves des combinaisons réellement annoncées.

## Diagnostic ED sur master7308544ce et contrat retenu

Worktree neuf `cx-ed-frontieres-boite`, branche homonyme ; aucun Cargo lancé
à ce constat. Cargo conserve `default=[]`, `daemon` ne demande ni `code` ni
`rag3db-native`. La boîte HTTP et la découverte d’une fiche existante sont donc
supportées avec daemon seul ; l’hôte natif reste sous daemon+natif.

Couplages encore présents : mail120 appelle lifecycle exclu sous daemon seul ;
lifecycle utilise le moteur uniquement pour engine_build_id et l’identité du
protocole via daemon_host::SERVICE. L’identité doit être commune client/serveur,
sans importer le serveur. Chat835/836 traite read_file via une constante du
module exclusivement code ; l’import du trait Boite des tests livraison n’est
utilisé que par leurs cas code. Pas de dépendance native ajoutée par défaut.
Archi réserve code_tools/grep et symbol_navigation, pas ces fichiers ; frontière
communiquée avant édition. Aucun plan global partagé édité.

Recette initiale CPU AVANT correction : source publiée, luciepc seulement,
poste réel/Max32G/HIGH0/j8/target privé neuf. Portée900s inclut cleanup ;
`cargo check --lib --no-default-features` <=300s doit être vert, puis même
commande avec `--features daemon` <=300s reproduit le E0433 connu et STOPPE.
Aucun troisième jeu après ce rouge ; logs et quiescence conservés. Aucune
variable native pour ces deux commandes, aucun modèle/service/PG/navigateur,
aucune lecture de configuration personnelle ; bus privé du lanceur commun seul.

Après diagnostic du rouge : source corrective et recette publiées avant ciblé.
Frontières attendues : minimum réellement sans features, daemon seul, puis
rag3db-native+daemon SANS code. Lib29 épinglée uniquement pour la dernière.
Tests de découverte sur fiches synthétiques (valide/absente/corrompue) sans
modifier ENV global, sans HTTP écoutant, sans appeler ensure/answers ni lancer
un hôte. Compiler l’API publique depuis_daemon_json sous daemon seul et préserver
les règles du jeton et l’identité du service. Gardes/imports de code ajustés
sans élargir les outils accessibles. Arrêt au premier rouge inattendu ; revue
et passe adaptée avant tour C7, pas de clôture sur un simple check natif/code.

### Diagnostic initial interrompu avant rustc

Le premier lancement sur 19cd9383d échoue au chargement du manifeste path
`codeparsers/Cargo.toml`, sous-module non initialisé dans le W PC. Le journal
`frontieres-boite-20261113/initial/minimum.log` est conservé ; aucun résultat
sur les features n'en est déduit. Correction de préparation : initialiser le
seul sous-module codeparsers à son gitlink, sans activer sa feature.
Reprise diagnostique séparée `diagnostic-gitlink`, même cible privée, primitive
900 s : minimum `cargo check --lib --no-default-features -j8` (timeout 300 s),
puis daemon seul avec `--features daemon` (300 s). Premier rouge arrête ; le
rouge E0433 attendu doit être constaté avant correction de production.

### Défaut reproduit puis source corrective préparée

533639368, diagnostic-gitlink : minimum vert 48,10 s ; daemon seul reproduit
exactement E0433 mail120/lifecycle exclu. Portée arrêtée rouge, aucun troisième
jeu exécuté. Source corrective : lifecycle sous daemon, introspection native
seule conditionnelle, identité HOST_SERVICE de protocole réexportée par hôte.
Gardes code sur read_file/import trait ; Cargo.toml/default inchangés.
Lecture de fiche factorisée sans changement de politique : trois témoins
valide/absente/corrompue ou sans jeton, aucune requête ni démarrage implicite.
Témoin moteur absent sans native ; jeton masqué dans Debug conservé.

Recette corrective publiée [02-recette-frontieres-boite.sh](../13-11-2026/02-recette-frontieres-boite.sh) :
primitive1800s incluant nettoyage, j8/Max32/HIGH0/target privé existant ; chaque
jeu check --tests <=180s, compilation ELF libtests <=420s, chaque filtre <=60s.
Ordre minimum puis daemon puis rag3db-native,daemon sans code. Minimum joue
le parcours chat générique existant (1) ; daemon mail9+lifecycle8 ; natif
mail9+lifecycle7. Zéro ignoré attendu, inventaire et copie ELF/SHA par jeu
AVANT toute compilation suivante ; arrêt au premier défaut même de comptage.
Arbres de dépendances minimum/daemon refusent rag3db et codeparsers actifs.
Lib29 immuable exportée uniquement pour le troisième jeu. Pas de service,
modèle, base personnelle, navigateur, PG ni HTTP. check_features conserve les
cinq jeux antérieurs et en ajoute trois (minimum/daemon/natif-daemon), avec
no-default-features explicite. Diff à revoir avant globale/master.

### Premier ciblé ad021 : minimum libtests/E2E révèle trois gardes manquantes

Arrêt au premier jeu/check-tests, avant toute exécution de test : deux helpers
libtests (`Instantane::vieillir`, `Backend::call_tool`) ne servent qu'aux tests
native, et le seul cas E2E qui inspecte marque_d_ecrivain oublie native.
Les usages sont tous relus : gardes exactes native ajoutées aux deux helpers
et à ce cas, aucun allow(warnings), aucune suppression de module/suite, aucune
exclusion du témoin sous son jeu natif supporté. Logs/supervision rouges dans
cible-ad021 conservés. Recette corrective suivante identique et republiée avec
ces changements causaux, dossier neuf cible-gardes-minimum ; pas de replay
inchangé. Les jeux daemon et natif n'avaient pas été lancés. Le ticket ancien
la-lib-native-daemon-ne-compile-pas-hors-matrice décrit le même bord code que
le présent ticket canonique, pas un deuxième lot produit.

Le contrôle de75 s'arrête sur le helper E2E `Client::autonome` devenu
inutilisé sans native : son unique appel est le témoin écrivain gardé ci-dessus.
Même garde ajoutée au helper, omission de lecture corrigée ; rouge conservé
cible-gardes-minimum, aucun test joué. Source/recette republiées avant reprise
causale dans cible-garde-autonome, mêmes argv/bornes et aucun rejeu inchangé.

29d56f6ef : minimum check --tests vert2,22s, ELF libtests compilé76s puis
parcours chat exact1/0/0 vert (inventaire1641). Arrêt au jeu daemon/check-tests :
agent_de_temoin et DELAI_DES_TEMOINS ne sont appelés que par daemon_host (native).
Gardes test+native ajoutées, sans toucher aux clients HTTP de production.
Relecture préventive du dernier jeu : BoiteDirecte et son impl sont réservés
au seul test code déjà gardé ; même garde ajoutée à ces helpers.

Reprise publiée : recette accepte troisième argument `daemon` pour reprendre
les deux jeux non exécutés, dossier cible-garde-client ; les preuves minimum29d
restent acquises. Équivalence du minimum : les seuls fichiers Rust changés
après29d sont daemon/mod.rs et daemon_host/livraison_codex.rs, modules exclus
sans daemon. Aucun rejouage inchangé du minimum, aucun saut de témoin daemon.
Borne1800s/Max32/j8/lib29/ELF conservés et arrêt premier rouge maintenus.

004279d4a : daemon check-tests10,68s et compilation29,06s verts ; mail9/0/0,
lifecycle8/0/0 verts, ELF/SHA/inventaire1722 archivés. Dépendances actives
sans rag3db/codeparsers confirmées. Native+daemon check-tests s'arrête sur
Hosted::rattrapage, OutilsDuChat::en_attente/deux builders, Navigateur::touche,
utilisés exclusivement sous code. Gardes code précises champ/initialisation/
méthodes, usage relu, aucun contournement de warnings. Pas de test natif exécuté.

Reprise publiée : recette troisième argument natif-daemon, dossier neuf
cible-garde-code, même portée1800 ; seul jeu restant. Deux parseurs existants
livraison Codex ajoutés (sans code :2/0/0 attendus, ni hôte ni processus Codex).
Preuves minimum29d et daemon004 conservées : tous les nouveaux fichiers Rust
sont dans daemon_host, module exclu de ces deux jeux. Les gardes ne changent
pas le comportement sous le jeu large qui possède code.

### Ciblé final 204ca4c39 vert, globale encore requise

Native+daemon sans code : check-tests vert, compilation ELF69s, mail9/0/0,
lifecycle7/0/0 et parseurs livraison2/0/0. Inventaire1879 ; primitive
validation-neivavmm VERT/code0/nettoyétrue/restes[]. Bilan composé36/0/0 avec
minimum29d et client004 selon équivalences ci-dessus. Aucun test sauté présenté
comme vert. Quiescence privée[] ; target7,1Go, preuves532Mo, trois ELF/SHA.
`bilan-cible.json` dans cache privé frontieres-boite-20261113 aux deux postes.
Aucun modèle/service/PG/navigateur lancé pour ces ciblés. Le ticket reste ouvert
jusqu'à revue, globale adaptée exigée par C7 et intégration.

### Revue et recette globale composée — avant entrée

C7 relit204 puis4cc favorablement et exige une globale6000 sur l'union grep,
pas les seuls ciblés/F8. Rebase branche neuve cx-ed-frontieres-boite-2 sur
f0feee30a (inclut d3f5a264d grep) ; seul conflit documentaire append du ticket
résolu en conservant C7 et ED, diff des fichiers ED src/scripts vide après rebase.

Recette publiée : 04-globale-frontieres-boite.py, 05-create-exact-meme-profil.sh,
06-lancer-globale.py dans ../13-11-2026. Adaptation relue des quatre scripts
Archi6786/transmission-ed, jamais lancés tels quels. Namespace ED neuf, target
unique existant, TMP court ~/.cache/cx-ed-fb, aucune ressource Archi recréée.
Lanceur06 exige le vrai poste avant de calculer6000s ; reçu commun/Max32/HIGH0
vérifiés ; budget initial durable réutilisé pour tout éventuel complément,
pas de reset implicite. Premier rouge arrête, logs gardés.

Ordre : tickets/CPU état, F8 check-tests sous borne2100, minimum0<=120,
compilation des trois ELF sous borne partagée1500, archive et inventaire complet
publiés avant runtime ; lib entière large<=900, NAV40<=600, coordination7879,
lectures raw27<=120 (ancien1 :26jugés+CREATE nonjugé attendu unique) puis même
ELF/profil exactCREATE1<=120 sans seul flagancien, hôte13<=120 et filets2 par
wrapper. CREATE restaure le marqueur d'état possédé puis appelle les vraies
terminer_etat_suite_e2e/residus_de_la_suite, aucun garde réécrit/contourné.
Santé BGE réelle avant/après, rendu immédiat après lectures ; pas PG/Vertex/
juge/équipier, aucun nouveau modèle/service/GPU. Les modèles locaux restent
CPU sans acquisition forcée. 223gabarits et source réellement embarqués par
ELF, PID/start/maps native29 et cgroup observés ; plafond target32Go gardé.

check_features s'arrête désormais au premier gate ou jeu rouge et nomme
sans-feature même dans son verdict rouge ; aucun jeu suivant ne s'exécute.
Contrôles shell/AST/diff propres avant publication ; ciblés Rust inchangés
non rejoués. Passage global autorisé C7 après cette publication ; aucun tour
master à ce stade. Inventaire global doit fixer ses propres comptes, sans
copier les2278/66 d'Archi.

### Globale093 interrompue : erreur de recette d'identité ELF, pas de défaut produit

F8/8, minimum, lib2281/0/66 (38,38s), NAV40/0/0 (131,81s) verts.
Raw lectures27/0/0 nominal (9,47s), filets2 verts :26 réellement jugés et
CREATE nonjugé attendu unique. Arrêt ensuite sur FileNotFound du précompilé
E2E-f34, supprimé par le ménage normal du wrapper. Aucun CREATE exact/hôte13
lancé. Santé réelle avant/après verte, 7879 rendu après arrêt, scope rouge
nettoyée/restes[] ; originaux et logs conservés.

Cause de mon adaptation : --no-default-features ajouté aux précompilations
NAV/E2E, alors que leurs wrappers activent le tag Cargo default même vide[].
Fingerprints E2E : seul ajout default, profils/config identiques ; aucun
cfg(default)/CARGO_FEATURE_DEFAULT dans src/build.rs. La recette04 est corrigée
pour préparer exactement les argv des wrappers (F8 reste no-default explicite).
Aucun Cargo/rebuild/rejeu de vert n'est autorisé par cette correction de recette.

Ledger : large réellement exécuté a25481dcf6fc… inchangé ; précompilés NAV00c3…
et E2Efb0b… restent des archives historiques, PAS les binaires des runtimes.
NAV réellement exécuté7497ef8b980cb374c9ed4922b8a98793c899899ebf0308a67cd6b0b7063e03bb,
E2E réellement exécuté6ead5c3ae216816386950f14a44a4c5bde7e95e3799074ea85e819c604a94b7a
archivés après arrêt, source093/223gabarits/maps native29/PID/start/Max32 relus.
La limite chronologique est explicite : leurs inventaires effectifs n'étaient
pas publiés avant les runtimes initiaux. C7 accepte le principe de composition
sous comparaison exhaustive des inventaires et tags avant complément.

Pilote07 et lanceur08 publiés : source PC demeure093, code Rust inchangé ;
recette complément copiée du commit documentaire publié dans le cache privé,
sans checkout du W de compilation. Aucun Cargo ni recompilation, borne locale
min(600, reste échéance globale888487.840738597 après vrai poste), aucun reset.
Inventaires des trois ELF réellement exécutés relus et comparés aux listes
initiales avant CREATE ; tout autre écart/tag/hash/nonjugé refuse. Même E2E6ead
et profil whitelist capturé raw, unique flagancien retiré, vraies primitives
état/TMP inchangées. Puis hôte13+filets2 via09 sans Cargo, racines privées
neuves et vraies primitives créer/terminer état/résidus, pas de copie de garde.
Coordination/santés7879 avant/après seul CREATE, rendu immédiat ; zéroPG/Vertex.
GO final de recette bornée C7 requis avant entrée, préparation autorisée.

Le pilote07 exige aussi avant tout complément les fingerprints effectifs des
paires précompilé/réel : aucun tag supplémentaire sauf default pour NAV/E2E,
aucune différence de profil/config ; default=[] relu dans Cargo.toml et aucune
branche cfgdefault dans src/build.rs. Ledger des tags écrit avant inventaires.
Absence de nonjugé dans les logs lib/NAV exigée. Préparation renforcée, toujours
sans Cargo et sans consommation de la fenêtre CREATE.

Pour hôte13 direct sans wrapper/Cargo, pilote07 reprend explicitement les
réglages natifs capturés du wrapper (ROOT/SHARED/LIBRARY_DIR/INCLUDE_DIR/LD),
exige lib29 pour les deux répertoires et SHARED=1. Les racines hôte/état restent
neuves ; aucun environnement personnel ni ancien TMP n'est réutilisé pour hôte.
