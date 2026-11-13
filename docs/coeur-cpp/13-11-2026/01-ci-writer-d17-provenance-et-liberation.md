# Refus writer D17 : provenance et cycle de vie C++, lecture seule

Rattaché au ticket [CI et réouverture physique](../../orchestration/tickets/2026-11-13-la-ci-refuse-la-reouverture-du-temoin-physique.md). Source examinée par git show : d17a230b14f60fbac59aa1e4ac10510ba5f3c7e2, jamais le seul arbre courant. Aucun code modifié, build/SDK/DB/service ou rerun CI exécuté.

## Bibliothèque du job

Log du [job131826283645/run43407787602](https://github.com/L-Defraiteur/rag3db/actions/runs/43407787602/job/131826283645) relu via connecteur GitHub. [Extraits de provenance et premier refus](preuves/ci-writer-d17-extraits.log), conservés avec les dates du log.

Le checkout fetch puis checkout master est exactement D17. check_features reçoit RAG3DB_LIB=statique ; son script43–45 retire RAG3DB_SHARED/RAG3DB_LIBRARY_DIR/RAG3DB_INCLUDE_DIR. Cargo déclare la dépendance path ../../tools/rust_api. Le lien suivi tools/rust_api/rag3db-src vaut ../.. : la source CMake est celle du dépôt checkouté, pas un SDK29 du magasin. build.rs choisit static en absence de RAG3DB_SHARED, build_bundled_cmake puis cible rag3db ; la liaison Rust récente utilise static:+whole-archive=rag3db. Les env supprimés par un script ne persistent pas magiquement aux steps suivants ; le rapport du step vector confirme indépendamment la liaison statique du test effectif.

Rapport vector du log10:39:27 : contextecommitD17, moteurpath+file tools/rust_api#rag3db@0.11.1, CMake build sous target/debug/build/rag3db/838549c3de9e092b/out/build ; cacheSHA890e980d00442d8e522582fcb3ccdd7785a9011d0c5f4b0138f6571ecad0ad23 ; archiveSHA36ece6a62ab092dc575a57879acdef0dc25cd7742725825784532cd7a998bc9b ; exécutabletestSHAfbd960f8ffbaf2ea810dd292f22deea7450d3238030615e35dfe048f0d60a1d9. Le sélecteur bati_vector_ci.py vérifie CMAKE_HOME_DIRECTORY, linked_paths/build/src et linked_libs statique, puis hache archive et ELF. Ce sont les identités annoncées/contrôlées dans ce job, pas des objets récupérés et rehachés ici. Aucun binaire CI ni BuildID moteur complet n’a été téléchargé. Ne pas substituer SDK29 ni réclamer une équivalence binaire des deux environnements.

## Registre et destruction

D17 src/main/database.cpp126–155 : set<string> process-global sous mutex, clé weakly_canonical(absolute(path)). Seule l’ouverture écriture hors mémoire insère la clé, avant ouverture du writerGate et avant recovery (178–188). L’échec de insert produit précisément le message ALREADY_OPEN_FOR_WRITING observé. Ce message n’est donc pas le refus du verrou fcntl d’un autre processus, ni une simple existence du fichier.

La marque est shared_ptr<void>(nullptr, deleter) dont le deleter efface la clé. Un pointeur stocké nul ne supprime pas le contrôle RAII : le deleter existe et s’exécute à destruction du dernier propriétaire. openForWritingMark n’est affecté qu’à cet endroit et est premier membre de Database (database.h218) ; il est donc détruit le dernier, après writerGate et les autres fichiers. Le corps ~Database fait éventuellement checkpoint puis marque le lifecycle fermé ; il n’efface pas prématurément le writer. Un constructeur qui lève après acquisition déroule aussi les membres déjà construits, dont cette marque. Pas de reset manuel ou de contournement à introduire.

Pont tools/rust_api : Database Rust porte UniquePtr<ffi::Database>, new_database renvoie make_unique<Database>. Connection C++ porte un Database* non propriétaire et partage uniquement DatabaseLifeCycleManager (bool isDatabaseClosed), pas un shared_ptr<Database>. Les résultats de requête partagent ce même lifecycle. Une Connection C++ seule ne prolonge donc pas la propriété de Database. Côté wrapper Rag3dbConnection, conn est déclaré avant db:Arc<Database>, garantissant la destruction de la connexion avant l’Arc ; son Drop ne retire aucune marque C++.

Le propriétaire Rust réellement en vie et les acteurs/index susceptibles de le retenir restent du ressort du diagnostic ED. Si le dernier Arc a commencé sa destruction sur un autre fil, une tentative concurrente peut encore rencontrer la marque jusqu’à fin de ~Database. Le texte du refus seul ne permet pas de choisir entre détenteur encore vivant, destruction concurrente, fuite ou autre réouverture dans le processus.

## Chemin exact du témoin et limites de la conclusion

physical_index::connection(root,false) → with_owned_physical_buffer_pool → open_explicit_config → Database::new direct. Ce chemin n’utilise pas open_for_writing/OPEN_FOR_WRITING Rust ni sa boucle REOPEN_WAIT. L’absence d’attente explique pourquoi un chevauchement de durée de vie pourrait être immédiatement visible ; elle ne prouve pas qu’un tel chevauchement est la cause. Ajouter un sleep ou rediriger aveuglément vers la voie normale ne serait pas un diagnostic.

La fixture D17 droppe create_new puis ouvre la connexion à la ligne414 lors d’une variante change>0. Aucun contenu de /tmp/.tmpPROaIV n’a été lu ni déclaré retenu ici. Le refus prouve uniquement une clé writer déjà présente à cet instant. La garde ne prouve pas que les bytes du fichier ont changé et n’autorise pas à désactiver la protection.

## Suite bornée proposée si la lecture ED ne tranche pas

Avant tout runtime, publier les détenteurs Rust identifiés, source exacte et instrumentation minimale proposée : acquisition/fin de destruction de l’instance et chemin canonique, sans données de Lucie. Préférer un témoin ciblé unique de cette fixture sous la provenance statique D17/correctif explicite, avec limite globale publiée et poste lourd coordonné32G/j8, en conservant stdout, source/ELF/archive/BuildID, sortie et quiescence. Cette proposition n’est ni un GO ni un ordre de compilation ; aucun créneau/budget/runtime n’a été pris. Une comparaison SDK29 ne devient probante qu’avec son BuildID/SHA et sa source établis séparément. Aucun changement C++ ni nouvelle charge native justifiés à ce stade ; ticket reste ouvert.

## Comparaison SDK29 demandée par C7

SDK identifié par le contrat publié des vues et en-tête relu sur luciepc : rag3db-build-29a7bc52aaf11e6204f9beafaffb154105417e13-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF. En-tête SHA7b115ba1f436f841eafaa8c4d1fb0d8f1642449f5deb10797c0424ee0eca48ac ; librag3db.so SHA b014febe51417ca535218d1c0b18308e7d301157abf45027a5cbba70dfc54929, calculés en lecture seule dans ~/.cache/rag3db-libs/<id>/. Aucun chargement de cette bibliothèque.

Git show29 et D17 : bloc registre/mutex/markOpenForWriting/deleter **textuellement identique**, SHA comparés dans [preuve](preuves/sdk29-garde-comparee.json). openForWritingMark reste premier membre aux deux sources ; acquisition avant recovery et corps destructeur identiques. Le diff des deux fichiers database.cpp/database.h porte uniquement tri d’includes et repli de commentaire. La garde writer n’est donc pas absente du SDK29 : cette différence de présence ne peut expliquer le contraste local/CI. Cela ne prouve pas une équivalence binaire ni l’égalité des ordonnancements/acteurs ou l’absence d’un autre écart moteur. Détentions et séquencement de destruction restent à départager avec ED/Lucivy.
