# Recette proposée : union99, Sandbox déclaré, D6 et canonique complète

PRÉPARATION SANS RUN, autorisée C7 après suffisance du bac ciblé.
[Union31](31-union-pas-va-au-bout-master-99.md), source produit742+master99,
[ ticket harness existant](../tickets/2026-11-16-le-temoin-documentaire-vertex-attend-un-accord-sans-superviseur.md)
et [ticket nom/D6](../tickets/2026-11-16-le-nom-agent-accepte-un-blanc-interieur.md).
Cette recette remplace les propositions16–18 pour cette union ; le module
full-pass/policy.py historique274 n'est pas un pilote ni un reçu réutilisable.
Aucun nouvel accord7200, aucun service/compte/modèle appelé pour la préparer.

## Cache, baselineT0 et entrée physique

Une seule racine de cible de session existante, plafond32Gio CUMULÉS avec tous
fichiers privés F. Garder le souscache natif T déjà utilisé ; pas de purge,
import/clone d'artefacts ni target par essai. Les propriétaires des enfants de
tâches sont définis d'avance sous SESSION/task-targets, absent au T0, et chaque
sous-cible est celle calculée réellement par commande::cible_du_depot.

Avant toute transition source/cache : archive physique742 indépendante et
détachée, administration/gitlink/fulltrackedSHA ; anciens EA/1bd/049/223 et
ELF/ledger positifs742 préservés. Le PC C canonique doit rester le même chemin ;
une branche PC NEUVE porte l'union, pas de checkout inféré par --git-dir.
L'opération Git compare l'inventaire intégral de T avant/après : identité,
propriétaire, modes/liens/tailles/SHA inchangés, aucune garde ni Cargo durant Git.

BaselineT0 de confiance : source742 et P01c3/scopes terminés/72preuves/2ELF
réhashés, identité T actuelle et inventaire complet pris AVANTmutation ; état
racine historique comparé en excluant seulement le souscache natif autorisé et
le futur task-targets explicitement absent. T0 est publié/copied/hashé hors W.
Après le vrai poste et juste avant première garde : recontrôle complet T0,
source/SDK/outils/archives et scan frais exe/cwd/FD/argv/PIDstart/CG. Trois rôles
launcher/watchdog/poste stricts partagent le vrai constructeur ; aucun ancêtre
exempté en bloc, UNKNOWN attribué bloque. Garde commune réelle source742→union
au même C, REPRENDRE=1 explicitement admis ; aucun marqueur Python forgé,
aucune dissimulation du build.rs externe/FP ni réinvalidation maquillée.
Chaque enfant de tâche a ensuite sa propre admission exacte avant Cargo.

Watchdog physique AVANT tout worker/log/fsync d'allocation/file/préflight,
CLI pure avant spawn et parent ultime rc seul (correctifs01c3). START du worker
est une observation APRÈS armement, pas une heure OS reconstruite. Un Common6439
vivant en mode mesure EXCLUSIF proposé/32Gio/HIGHmax/POSTE_CPU_QUOTA=800%,
POSTE_FILS effectif>=8 exigé avant mutation, jobs+Rayon8=min(8,POSTE_FILS), Rusttests1,
bus et états privés. Tout final/hash/scan/publication reste sous ce watchdog ;
contrôles postopen ET postflush/fsync avec reclassement cohérent fichier/exit.

## Contrat Sandbox réel AVANT création de tâche

Deux chemins de fixture explicitement distincts :

1. Tests natifs/canoniques : helper cfgtest742 lit une déclaration privée typée
   SandboxConfig et un RAG3WEAVER_TARGETS_DIR explicite. Il copie le Sandbox
   AVANT write/add_files/commit HEAD des deux vrais setups. Le fichier Q privé,
   SDK59/Rust/Cargo et autres capacités de lecture requises sont déclarés,
   Landlock/réseau fermé/extra_write vide/commands=approval maintenus.
2. D6 en service privé : le helper cfgtest n'existe PAS dans le binaire host.
   Préparer les templates/config backend de la copie privée, copier la
   déclaration Sandbox dans le manifeste SOURCE AVANT code_tree::add_files,
   commiter/vérifier son contenu dans HEAD AVANT l'ouverture de tâche.
   Dans cette source, l'opération réelle est POST /ide/taches vers
   DaemonHost::ouvrir_une_tache_nommee (le « TaskService add » de la recette),
   pas une API TaskService ajoutée pour le pilote. Vérifier ensuite manifeste
   de tâche chargé/parentHEAD/Sandbox effectif avant la demande modèle.

RAG3WEAVER_TARGETS_DIR pointe explicitement SESSION/task-targets NEUF ;
CARGO_TARGET_DIR reste `{repo_target}`, TMPDIR `{task_dir}/tmp` et offline=true.
La valeur réelle taches-<nomASCII> et son verrou doivent être sous SESSION,
aucun second cache implicite ailleurs. Précréer la tâche par API avant modèle,
initialiser/gitlink exact, fixer cwd/nom/sous-cible puis geler l'admission.

Adaptateur nommé cargo UNIQUEMENT dans Q/bin privé, PATH déclaré dans la SEULE
configuration des fixtures/tâches concernées, Q explicitement extra_read AVANT
création. Aucun fichier sous ~/.cargo/registry modifié, aucune interception
globale ni PATHshim du canonique entier. Le vrai Cargo/rustc s'exécute DANS
Landlock sous le reçu commun ; un contrôleur extérieur admet/observe, jamais
substitue une réussite, un faux échec ou une exécution hors Sandbox.

Le manifeste privé peut déclarer workspace.scripts avec les argv exacts du test
D6, sans désactiver approval pour les autres commandes. Le shim n'admet que la
commande précise et les metadata strictement nécessaires déclarées d'avance ;
commande/cwd/features/profil/target/source inconnu=STOP. Témoin natif réel
commande::tests::cargo_plays_its_tests_inside_the_sandbox INCHANGÉ, Cargo réel
sans shim, mêmes observations de profil déclaré avant lecture. Pas de politique
choisie selon l'environnement observé. FeaturesCPU et environnementSDK hérité
sont deux faits distincts ; pas d'env_clear mensonger pour un binaire nonnat.

## D6 réellement empirique, isolé du témoin Vertex documentaire

Host rag3daemon ET vrai binaire rag3weaver-tty reconstruits de l'union, CargoJSON
et artefacts uniques : host features explicites code/daemon/rag3db-native/openai-llm,
TTY --manifest-path tty/Cargo.toml et feature vue explicite. BinaireTTY est celui
de tty/, pas rag3weaver-chat. Même cache de session, propriété source racine
et profils cwd/CARGO_MANIFEST_DIR du souscrate déclarés ; aucune nouvelle cible
racine pour contourner la garde. SDK59 épinglé/2SO+headerSHA/export réel ; profils
compilation/runtime et LD rustlib attendus fixés avant observation.

Host privé127.0.0.1:port0/daemon.json privé, jamais7970/927 ni service utilisateur.
Copie Git complète de l'union et backend/juges mémoire privés ; initialiser le
sousmodule exact avant gel. Vrai terminal --plain --depot --tache --demande
--journal --daemon-json privé. UNE request ; l'attendu est écrit avant.

L'agent écrit backend::tests::agent_name_rejects_an_inner_space, joue le vrai
test exact ROUGE sur « Om bre », corrige le validateur pour refuser ce blanc
intérieur avec erreur agent.name, préserve trim/vide/retourligne, rejoue le même
test VERT, présente gitdiff puis Ready. Aucune édition du code gelé de l'union,
aucun commit/push/fusion de l'artefact de tâche. Un rouge de compilation, zéro
test, échec étranger ou absence d'édition n'est pas le rouge D6 attendu et STOP.
Les deux vrais Cargo sont séquentiels après le bâti APP, leurs empreintes
source/patch/SHA/argv/cwd/SDK/provenance/portée sont reliées au même descendant.

Capturer outils/arguments/retours/tours/événements/runIDs/boîte/jugementsréels,
fin autonome sans demande « continue » ; l'affichageTTY seul ne suffit pas (T1).
Pas d'approbation générique : seul refus lié exact rm Cargo.lock s'il apparaît,
unknown approvals/questions=STOP avec diagnostic complet. Après capture du diff
agent, un reviewer indépendant peut injecter son propre test Rust exact et le
jouer ; il reste explicitement juge du contrôleur, jamais travail de l'agent.
D6 réussi ne ferme ni S1/S2/S3 ni le ticket de politique de nommage.

## Canonique inchangée, modèles et services coordonnés

D6 puis canonique2150 BYTEidentique à tools/poste/passe_epinglee.sh, Common6439,
SDK59. Source/ELF/closure/features et223gabaritsGit attester avant runtime ;
ledger/copies réhashées2hôtes AVANTmarker, aucun ELF préfixe source seulement
présenté comme profilSDK. Observations cohérentes owned/PIDstart/phase, selected
fields seulement ; aucun environnement brut ou jeton publié. Tous les Cargo,
metadata/nested comprise, passent par les garde/admissions communes déclarées.
Le script2150 force jobs8. Proposition concrète sans interception : prendre le
mode poste mesure EXCLUSIF avec quota explicite800%, vérifier POSTE_FILS>=8, puis
8=min(8,POSTE_FILS). Si la place reçue est inférieure, STOP AVANT toute mutation,
pas de réécriture du script ni de shim cargo sur le PATH canonique. C7 doit
arbitrer le coût de cette réservation exclusive et le créneau ; cette recette
ne lui substitue pas un quota ou un accord. Le mode exclusif couvre aussi les
entrées imbriquées --build sans upgrade de mode interdit.

SUITE_DU_TEMOIN=lib, filtre teammate_tests pour témoins du lot ; suites par
défaut e2e_lectures/e2e_code conservées, plus e2e_demon_hote,
e2e_hooks_claude_code,e2e_equipier (le vrai scénario Vertex documentaire900).
required_suites.sh recalculé à chaque gel master→union : aucune exigée à ce
premier audit, toute exigence ultérieure ajoutée. Ne pas rejouer séparément le
scénario documentaire déjà joué dans la suite canonique de CETTE union.
F8huitjeux réels, Checking/Compiling preuve source exacte, liblarge ENTIÈRE sans
filtre, NAV réel/0ignored ; les ignorés normaux lib sont consignés, jamais passés.
Résumé officiel produit par2150 uniquement, verdict composé D6+canonique séparé.

Avant premier service/modèle réel : coordination C7/compte Vertex/capacité,
coûts/token/temps par demande+juge conservés. Identifiants seulement par chemin
.vault/vertex-sa.json, projetlr-hub-472010/locationglobal/EXIGER_GEMINI1 ; fichier
jamais lu/copié/affiché par le pilote. Aucune injection de fausseauth à la place
du vrai Vertex de D6/équipier. Services7878/7879 et fixturePG : ACK frais avant
usage (au début D6 si index en a besoin), santé/identités avant/après, aucun
start/restart ni appropriation. Canonique définit ses variables de service :
le futur GO doit explicitement couvrir leur usage, pas un GO implicite du script.
Base/index/tâches sont privés, jamais bases de Lucie ni requêtes sur7970.

## Proposition chiffrée, à arbitrer avant pilote/GO

Observations disponibles : deux builds ciblés74236.88+31.52s, vert ciblé177.936s ;
ancienne canonique ED native rc0 avant son composé rouge : retour1193428.800995905
après initial1192303.460980840, soit1125.340s INCLUANT ses préphases, pas une durée
canonique isolée ni un vert composé transférable. D6 ancien a échoué en20min
sans écriture ; latence/nombre de pas et tokens du nouveau D6 restent inconnus.
C'est une mesure de coût historique, pas une promesse sur cette nouvelle union.

Proposer Common NEUF7200MAX, physique TERM7440 + KILL15 =7455s au total depuis
watchdog (file/final inclus), STARTobservé après armement. Après poste delai=min
7200/restephysiqueconservateur, refus insuffisant ; aucune remise à zéro aux
transitionsAPP/tâche/canonique ni aux coordonnées services/pauses.
Caps : préflight120, host600,TTY240, ledgerpublication90,D6inclreview1800,
canonique4200, final120 =7170s ; tous intersectent le même END−15.
La marge extérieure240s est pour file/final, pas une prolongation de Common.
Le capcanonique couvre suites supplémentaires dont Vertex900, F8/lib/NAV après
une ancienne borne de coût1125s sur moins de suites ; aucun coût cloud garanti.
Premier rouge STOP/pasretry ; D6TDDrouge précis seul événement attendu distinct.

Arbitrages C7 proposés :7200Common/7455physique, mode mesure exclusif/quota800%
et POSTE_FILS>=8, créneau+services+compte Vertex. Aucun de ces choix n'est un GO.

Pilote exécutable, témoins hermétiques publiés avant essais, Q/pin/paquet/RO
TOTALcodeCLI+metadata puis GO exact C7/créneau obligatoires AVANT Cargo/modèle/
services. Sourceunion+présente recette seules ne sont ni ce pilote ni un GO.
Les ressources et preuves précédentes restent en conservation ; pas de cleanup.
