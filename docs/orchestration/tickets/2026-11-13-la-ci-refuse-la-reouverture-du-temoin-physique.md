# La CI refuse la réouverture du témoin physique

- **État** : ouvert ; échec CI exact observé, diagnostic de cycle de vie à attribuer sans rejeu.
- **Famille** : produit.
- **Gravité** : CI de master rouge ; portée production et cause à établir.
- **Atteignable en service** : à départager ; échec actuel dans une fixture native privée.
- **Touche rag3weaver** : oui, dataflow/migrations/physical_index/tests.rs:414 et cycle de vie de la connexion native ; moteur statique CI à examiner.
- **Pour** : orchestration, ED pour le diagnostic Rust et coeur-cpp pour la provenance et le registre des writers ; coordination C7.
- **Signalé par** : contrôle périodique C7 du 13 novembre 2026.

## Preuve exacte, sans attribution prématurée

CommitCI d17a230b14f60fbac59aa1e4ac10510ba5f3c7e2 ;
[run43407787602](https://github.com/L-Defraiteur/rag3db/actions/runs/43407787602),
job131826283645, étape11 cargo test --lib
rag3db-native,code,daemon,openai-llm. F8 et compilation des tests ont réussi.
Résultat :2257 passés,1échec,66ignorés,2exclusions existantes du workflow.

Échec native_legacy_foreign_schema_and_values_refuse_owned_open_without_writes,
src/dataflow/migrations/physical_index/tests.rs:414:49 : connection(root,false)
refusée parce que owned/index.rag3db est déjà ouvert en écriture dans ce
processus. La ligne précédente crée puis droppe OwnedPhysicalIndex ; il faut
identifier quel writer ou quelle référence reste vivant, sans inventer la cause.
La fixture /tmp/.tmpPROaIV est signalée PHYSICAL_FAILED_FIXTURE par le log.
Aucune copie ni base de Lucie n'a été ouverte pour ce contrôle.

Les quatre nouveaux juges de layers passent dans ce même log. Aucune
attribution du rouge au résolveur pur2a n'est établie. Son ticket livré n'est
pas rouvert par simple coïncidence temporelle.

La CI utilise RAG3DB_LIB=statique ; les qualifications locales utilisent
SDK29 épinglé. Ne pas assimiler ces environnements ni réétiqueter l'un avec
les preuves de l'autre. Source native exacte, garde d'écriture et lib réellement
liée en CI restent à relever depuis workflow/logs/sources.

## Prochaine action et limites

ED : lire fixture, création/Drop/ouverture et détentions Rust réelles ; proposer
un diagnostic et, si établi, un correctif isolé fondé sur le contrat existant.
Coeur-cpp : identifier la garde du writer dans les sources de la CI et sa
libération ; comparer aux conditions du SDK local sans rebâtir/patcher la lib.

Lecture seule initialement ; aucun Cargo/rustc/C++/DB/service ni rerun GitHub.
Tout runtime futur exige un plan publié/source exacte et un créneau coordonné.
Ne pas supprimer le juge, ajouter un sleep arbitraire, affaiblir la protection
contre deux writers ou forcer une ouverture concurrente.

Fin : cause attribuée avec preuve, correction et qualification pertinentes,
CI future verte sur source corrigée. Aucun ancien run rouge repeint ; ticket
reste ouvert tant que cette conclusion n'est pas acquise.


## Diagnostic de sources coordonné ED / coeur-cpp

Le log CI lie la crate tools/rust_api (rag3db-src vers la racine) au moteur
statique issu du checkoutD17, et non au SDK29. Coeur-cpp relève archive native
SHA36ece6a62ab092dc575a57879acdef0dc25cd7742725825784532cd7a998bc9b et
ELFfbd960f8ffbaf2ea810dd292f22deea7450d3238030615e35dfe048f0d60a1d9.
Sa lecture montre le marqueur C++ d'écriture conservé jusqu'à la fin de la
destruction Database. Les connexions C++ n'en sont pas les propriétaires.

ED relève une course possible dans les sources Lucivy4.3.3 réellement liées :
ShardedHandle::close attend ActorPool::shutdown, dont l'attente porte sur la
réponse du message Shutdown. Le handler l'acquitte avant son retour Stop ;
la destruction de la boîte d'acteur intervient ensuite dans le scheduler.
Cet acteur peut retenir LucivyHandle/Index/BlobDirectory/BufferedBlobStore/
CypherBlobStore puis la connexion/DB. Cette chaîne est cohérente avec un Drop
natif retardé, mais ne prouve pas quel acteur ou Arc retenait la base du run CI.
Aucune fuite ni attribution causale exacte n'est affirmée à ce stade.

Porteurs : ED poursuit le cycle de vie Rust et la primitive physique qualifiée ;
coeur-cpp conserve sa lecture de provenance et de la garde native ; Lucivy est
sollicitée pour relire la sémantique shutdown/destruction et proposer un témoin
déterministe puis une correction si ce contrat est effectivement violé.
Rester sur le même ticket, sans dupliquer le signalement.

Préparation Lucivy en branche/worktree isolés autorisée : lecture de la
version4.3.3, plan et témoins/diff publiés avant qualification CPU coordonnée.
Aucune modification de .cargo/registry, aucun patch local des dépendances de
rag3weaver pour maquiller le juge, aucune reconstruction du SDK épinglé, aucun
index/base réel, tag, publication crates/npm/PyPI ou release sans décision
explicite distincte. Une barrière de destruction doit être fondée sur les
primitives du scheduler/pool ; pas de sleep, retry arbitraire ou garde affaiblie.

Le préfixe du garde Recherche (cache/F8/compilations/ledger) est indépendant
et autorisé sur SDK29 ; il s'arrête avant tout runtime. Le nouveau rouge CI
reste ouvert et n'est pas remplacé par les verts locaux.

## Lecture coeur C++ : provenance et comparaison SDK29

[Rapport borné et preuves](../../coeur-cpp/13-11-2026/01-ci-writer-d17-provenance-et-liberation.md) : CI archive/ELF statiques D17 attestés par le job ; garde C++ writer RAII effacée en dernier à destruction Database. Connection C++ ne possède pas Database. Chemin physique direct open_explicit_config sans boucle REOPEN_WAIT normale. SDK29 identifié29a7bc52a/en-tête/SHA bibliothèque relus : bloc registre/mutex/marque/deleter textuellement identique à D17, même ordre de destruction ; absence de garde29 exclue comme différence explicative. Aucun détenteur ni défaut moteur attribué par ce seul refus ; ED/Lucivy suivent les acteurs. Aucun build/SDK patch/DB/service/CI rerun, aucun code modifié. Ticket ouvert.
## Relecture Lucivy 4.3.3 et plan CPU publié (13 novembre)

Porteur Lucivy pour le contrat destruction ; ED et coeur-cpp conservent leurs
périmètres ci-dessus. Source exacte relue : tag `v4.3.3`, commit
`663975adf11fc4a9d6cadf90e70ee68f6c9db075` du dépôt lucivy.

Constats de source, sans runtime :

- `luciole/src/pool.rs`, `Pool::shutdown` attend uniquement les `Reply<()>`
  des `ShutdownMsg`. Le protocole documenté exige `ack()` avant `Stop`.
- `luciole/src/scheduler.rs`, `run_loop_handle_actor` et
  `run_one_step_actor` retirent le slot et émettent `ActorStopped` alors que
  `actor_box` est encore une variable locale. Le registre vide et l’événement
  d’arrêt ne constituent donc pas une preuve de destruction achevée.
- Le témoin amont de fermeture 4.3.3 précédemment joué autorise jusqu’à deux
  secondes après `close()` pour observer la libération des shards : il prouve
  une libération éventuelle, pas une barrière au retour de `close()`.

Préparation publiée, sans qualification : dépôt lucivy, branche
`cx-lucivy-shutdown-destruction`, worktree
`/home/lucied/git_workspaces/cx-lucivy-shutdown-destruction`, tête `273b7c6`.
Diff limité au test `pool::tests::shutdown_waits_for_worker_destruction`,
aucun correctif de production. `git diff --check` joué et propre ; aucun
Cargo/rustc ni test exécuté pour cette préparation.

Témoin proposé : un worker CPU reçoit Shutdown, acquitte puis retourne Stop.
Son destructeur signale son entrée et retient un Arc témoin derrière un canal.
Un thread externe appelle shutdown. Après l’entrée dans Drop, l’observateur
attend au plus une seconde un retour prématuré, libère toujours le destructeur,
puis joint le thread avant d’asserter. Les délais bornent les canaux du test ;
aucun sleep, DB, GPU, index, SDK ou corpus. Un retour dans l’intervalle où Drop
est explicitement retenu est un contre-exemple concret au contrat « destruction
achevée ». L’absence de retour prématuré sur une exécution ne suffit pas, seule,
à démontrer l’absence universelle de course ; la garantie doit venir du code.
La voie coopérative reste à couvrir par un témoin distinct si correction.

Commande CPU candidate, uniquement après créneau coordonné C7 :
`cargo test -j8 -p luciole --lib pool::tests::shutdown_waits_for_worker_destruction -- --exact --nocapture`,
sous `poste lourd`, avec target unique Lucivy, timeout externe et journal séparé.
Pas de relance CI, build SDK ni patch de registre. Aucun rouge observé revendiqué
avant cette exécution.

Correction proposée à concevoir après rouge : primitive de fin de vie portée
par le scheduler et partagée avec ActorRef, complétée seulement après destruction
de l’acteur et de sa mailbox, sans tenir le mutex des acteurs pendant Drop.
Pool::shutdown doit attendre cette primitive via les attentes coopératives
existantes, et non inférer la fin de vie d’un ack, d’un registre absent ou d’un
événement ActorStopped. Vérifier aussi les arrêts sans réponse, appels répétés,
chemins worker et coopératif, panique et interdiction d’attendre sa propre
fin de vie. Le routeur de ShardedHandle doit employer la même garantie.
Aucune implémentation de cette primitive n’est livrée à ce stade.

Reste : coordonner puis jouer le témoin CPU publié, relever le résultat exact,
concevoir/qualifier la barrière si nécessaire, et relier séparément cette course
au détenteur réel du writer CI. Le ticket reste ouvert ; l’échec D17 n’est
ni rejoué ni attribué définitivement à Lucivy par cette lecture.

## Tentative CPU Lucivy : entrée refusée, aucun témoin joué

GO C7 reçu pour source exacte `273b7c6ccaff5b3d49507ca62aebb5bfe87095fa`,
fenêtre unique 600 s après poste lourd, MemoryMax 8 Gio, MemoryHigh infinity,
quatre jobs, target neuf privé. Consigne figée avant entrée dans
`luciepc:~/.cache/rag3weaver-build/cx-lucivy-shutdown-13nov/consigne.txt`.
Worktree `cx-lucivy-shutdown-cpu` propre, aucun patch registre/SDK/DB.

Le lancement de confinement a échoué avant Cargo, code 1 :
`Failed to start transient scope unit: Unit run-p1207294-i89214559.scope was already loaded or has a fragment file.`
Commande : poste lourd puis scope utilisateur explicite MemoryMax=8G,
MemoryHigh=infinity, RuntimeMaxSec=600s puis helper commun
confiner_validation.py --delai 600 et cargo test ciblé -j4. Le poste utilise
lui-même systemd-run ; une collision de nom de scope est observée à l’entrée,
sans attribuer davantage la cause avant examen.

[Brut conservé](../../lucivy/13-11-2026/preuves/shutdown-entree-refusee.log).
Aucun compte de tests, aucun rouge TDD et aucune compilation observés.
Conformément au GO « compilation/autre rouge STOP diagnostic, aucun retry »,
aucune seconde tentative lancée. Porteur Lucivy ; reste : coordonner avec C7
la correction de l’entrée (nom de scope explicite unique) et une nouvelle
autorisation de qualification. La correction de production n’est pas entamée
avant le rouge CPU attendu. Le ticket CI reste ouvert et sa cause non attribuée.
