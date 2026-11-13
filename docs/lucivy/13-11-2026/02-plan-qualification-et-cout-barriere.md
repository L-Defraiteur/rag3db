# Barrière de destruction : suite de qualification et coût CPU (préparation)

Ticket unique : `docs/orchestration/tickets/2026-11-13-la-ci-refuse-la-reouverture-du-temoin-physique.md`.
Porteur Lucivy ; relecture ED ; coordination des fenêtres C7.
**Aucune commande de ce plan complet n’a été exécutée.** Le vert10/10 existant porte
uniquement sur les témoins luciole de fbd3f0a, pas sur les phases proposées ici.

## Sources et invariants à figer avant GO

Avant : tag lucivy4.3.3 `663975adf11fc4a9d6cadf90e70ee68f6c9db075`.
Production après : `fbd3f0a4179917e3674fa52a56ffde027831a354` conservée.
Juges supplémentaires publiés sur `cx-lucivy-core-barriere-temoins`, source
`3b1661446c573ee794b918ec016c0cbff748e4ce` : uniquement tests en plus de FBD,
aucun nouveau delta de production. Le harnais se trouve dans
`outils/cout-envoi/` ; son Cargo.toml.template remplace uniquement LUCIOLE_PATH.
Le pilote `outils/qualifier-barriere-complete.py` fixe la source3b16614,
inventorie la lib et joue la lib entière puis six exacts core ; aucune exécution.
Pas de patch registre, SDK, base réelle, dépendance rag3weaver ou publication.
Les répertoires natifs temporaires des tests lucivy-core sont des fixtures
synthétiques privées et nécessitent un GO explicite de phase, pas une ouverture
de l’index de Lucie. Les réouvertures existantes ne sont pas une migration.

## Phase correction complète, proposition bornée

Une fenêtre candidate900s après vrai `poste lourd`, MemoryMax8G/HIGHmax/j4,
confineur commun et logs horsW, même target Lucivy privé quiescent si C7 l’admet.
Compilation, inventaire puis exécution :

1. `cargo test -j4 -p luciole --lib --no-run`, inventaire `-- --list`, puis
   `cargo test -j4 -p luciole --lib -- --nocapture` : lib entière, nombre réel
   conforme à l’inventaire, aucun test ignoré déclaré vert. Les stress concurrents
   restent inclus ; aucun filtre pour éviter un rouge.
2. `cargo test -j4 -p lucivy-core --lib --no-run`, inventaire de noms exacts.
   Jouer un test exact par appel, `-- --exact --nocapture` :
   - `sharded_handle::tests::a_failed_close_still_stops_its_pools` ;
   - `sharded_handle::tests::test_drop_commits_the_writes_still_queued` ;
   - `sharded_handle::tests::test_close_and_reopen` ;
   - `sharded_handle::tests::test_writes_to_a_shard_without_writer_are_reported_by_commit`.
   Ces noms ont été relus dans la source, pas encore inventoriés par un binaire.
   Chacun doit annoncer1test,0ignoré ; corpus externe non requis par ces noms.
3. Deux juges RAM publiés exigent AU RETOUR de close
   (succès puis erreur) `strong_count(handle.router)==1` et, pour chaque shard,
   `strong_count==1`, sans fenêtre de polling. Le juge amont existant autorise
   deux secondes après close et ne remplace pas ce contrat immédiat.
   Noms : `successful_close_releases_router_and_shards_at_return` et
   `failed_close_releases_router_and_shards_at_return`. Injection déterministe
   sur write_root_file d’un ShardStorage RAM, pas de suppression ni sleep.
4. Juges publiés `scheduler::tests::destruction_completion_pinned` (queue avant
   démarrage pinned) et `pool::tests::shutdown_without_reply_releases_worker_resource`
   (réponse perdue puis libération immédiate et shutdown répété). Les témoins worker/coopératif, Drop réentrant,
   envoi admis/bloqué et unwind publiés restent obligatoires, non dupliqués.

Au premier FAILED/code nonzero/refus : arrêt, brut conservé, pas de retry.
Les paniques attendues par catch_unwind se jugent sur le résultat du test.
Le reçu couvre la fenêtre globale, compilation et sous-commandes incluses ;
aucun reset entre étapes. Inventaire PID/start et quiescence après fin.
Cela ne rejoue pas la CI D17, ni n’identifie son détenteur historique du writer.

## Mesure du coût du mécanisme, proposition CPU comparative

Le delta ajoute mutex/compteur/garde et cloneArc à chaque send/try_send. Ce coût
concerne l’ingestion normale ; le vert fonctionnel ne mesure aucun débit.
**Fenêtre MESURE distincte candidate600s**, après RO/GO source et protocole.
Même CPU, affinité relue et fixée aux mêmes quatre CPU disponibles, pas GPU,
consommateurs concurrents inventoriés et fenêtre coordonnée. Ne pas inventer un
isolement si la machine n’est pas stable : consigner la charge ou refuser mesure.

Harnais autonome CPU à publier avant GO, identique octet pour octet entre A/B :
dépendance `luciole` par chemin vers deux worktrees figés A/B, sans patch de
.cargo/registry ni modification du code de production. Deux manifests ne diffèrent
que du chemin. Le premier build résout le lock du harnais ; copier ce lock au
second avant build --locked, vérifier égalité octet pour octet et empreintes
avant chronos. Toolchain, flume, profil
release/debug0/features et flags. Un seul target privé Lucivy pour les deux builds
séquentiels, exécutables figés/copiés horsW avec SHA ; compilation hors chronos.
Aucun cache froid/chaud de compilation confondu avec le débit runtime.

Matrice : worker scheduler2 et pinned ; canal non borné et capacité1024 ; un et
quatre producteurs. Chaque producteur envoie100000 messages scalaires via
ActorRef::send, acteur cumulant localement le nombre. Après jonction producteurs,
Fence rend le compte exact ; Shutdown suit. Pour le tag ancien, l’acquittement
n’est pas une barrière : chronométrer jusqu’à Fence pour comparer le même travail,
conserver un témoin de Drop et attendre sa réception pour quiescence hors chrono.
Le harnais n’utilise pas la nouvelle API wait_stopped absente du tag4.3.3.

Échauffement10000messages non mesuré ; ordre A/B/B/A, trois blocs maximum par
case, même N et seed (payload fixe), budget global600s maximum et premier échec
arrête. Produire brut JSONL : source,toolchain,exeSHA,profil,affinité,cas,N,compte,
durée monotone,débit,charge et statut. Vérifier nombre envoyé==Fence et absence
de restes ; un mauvais compte n’est pas une mesure lente mais une erreur.
Rapporter médiane et dispersion par case, ratios B/A et coûts par message.
Le ratio mesure le **delta global**, incluant HandlerGuard et l’ensemble du
changement ; il n’isole pas le mutex. Aucun seuil de régression inventé : C7/Lucie décideront sur ces chiffres s’ils
justifient de revoir le mécanisme. Ce microbanc mesure envoi/dispatch CPU, pas
une ingestion complète ni une performance du stockage.

## Fin attendue

Preuves bornées de la lib entière et du contrat close/router, puis mesure CPU
interprétable. Une fusion ordinaire dans main lucivy relève du GO de coordination après revue
et qualification dans ce correctif autorisé. Tag, release et publication de
version restent des décisions explicites distinctes de Lucie ; aucun GO ici. Ticket partagé reste ouvert.

### Limite de mesure pinned relevée en lecture seule

Dans les deux sources, ActorRef::send émet un diagnostic sur stderr lorsqu’il
n’a pas de notifier, ce qui est le cas pinned. Ne pas ajouter silencieusement
une suppression dans le code d’un seul côté. Fixer stderr vers le même puits
pour A/B et le relever dans la recette ; même vers /dev/null, formatage/syscalls
restent dans le chrono et peuvent masquer le coût d’admission. Les cas worker
servent alors de comparaison sans ce diagnostic. Cette mesure doit déclarer
cette limite et ne pas s’attribuer un coût « mutex seul ».

### Contre-exemples du pilote MESURE

Chaque cas écrit raw stdout fsync, identité source/exeSHA/PID/start/argv/rc
avant toute validation JSON/compte ; firstred est contextuel, ouvert x et fsync.
Le harnais A/B identique installe horschrono un hook de panique JSON sur stdout,
uniquement en cas de panique. stderr reste vers /dev/null pour les diagnostics
pinned volumineux, avec la limite de coût déjà notée. Aucun replay pour retrouver
une raison perdue ; aucun diagnostic normal ajouté dans la boucle chronométrée.
Pilote et hook préparés seulement, aucun build ni MESURE exécutés.
