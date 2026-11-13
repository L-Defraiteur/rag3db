# Writer physique : diagnostic Rust en lecture seule

Ticket commun : [la CI refuse la réouverture](../../orchestration/tickets/2026-11-13-la-ci-refuse-la-reouverture-du-temoin-physique.md).
Source examinée : d17a230b14f60fbac59aa1e4ac10510ba5f3c7e2. Base documentaire : 549ab1e3d2cf05d5ba8e9cb9dc07eda4d10a03bc.
Aucun Cargo, rustc, test, DB, service ou rejeu ; aucun changement produit. Les témoins ci-dessous sont proposés, non écrits et non joués.

## Ouverture et autorité

`physical_index.rs:155` lit le marqueur de racine puis construit le permis opaque. `PhysicalRootPermit::validate_path` exige le chemin exact root/index.rag3db et la même origine dans le marqueur. `with_owned_physical_buffer_pool` valide ce permis, conserve la règle de buffer pool et le mode read-only demandé, puis appelle `open_explicit_config`. Ce dernier appelle directement `rag3db::Database::new` après contrôle de la lib chargée.

L'ouverture ordinaire `with_manifest_buffer_pool` emprunte `open_for_writing`, qui refuse les racines physiques, suit une Weak par chemin absolu et retente pendant REOPEN_WAIT avec polling. `with_config`, également interdit aux racines physiques, utilise lui aussi `open_explicit_config` directement. Le registre Weak ne couvre donc pas toutes les ouvertures configurées ; il n'est ni le permis physique ni une preuve de destruction terminée. Un strong_count nul ne certifie pas la fin du destructeur de la valeur. La protection native reste l'autorité contre deux writers.

Raccorder la voie physique au polling ordinaire ne serait pas une correction de la fermeture. Conserver séparément autorisation du chemin, configuration et contrat de durée de vie ; ne pas contourner le refus de racine physique ni introduire un booléen de bypass.

## Détentions et fenêtre de destruction

`OwnedPhysicalIndex::create_new` relâche le MutexGuard et rend un seul Arc de catalogue. Le test en échec détruit immédiatement cette valeur ; aucun clone de catalogue externe n'est visible dans ce chemin. OwnedPhysicalIndex n'a pas de Drop spécifique. Le Drop de Catalog vide la ReadView, ferme les handles FTS courants et parqués, puis vide le tampon. La méthode explicite close_catalog effectue shutdown puis drop ; elle utilise la même fermeture FTS et ne prouve pas à elle seule la destruction des acteurs.

Cargo.lock D17 fixe lucivy-core et luciole 4.3.3. Lecture des sources déjà présentes sur luciepc, sans modification du registre :

- ShardedHandle::close, sharded_handle.rs:3196–3201 : attend les shutdown des pools.
- ActorPool::shutdown, pool.rs:194–211 : attend les replies de ShutdownMsg.
- ShardActor, sharded_handle.rs:1130 : acquitte avant de retourner ActorStatus::Stop.
- scheduler.rs:1090–1120 : actor_box demeure une variable locale après cet acquittement ; son destructeur intervient ultérieurement en sortie du traitement. Le chemin coopératif a la même séparation.
- ShardActor conserve Arc<LucivyHandle>. Celui-ci garde Index et IndexReader après close du writer ; le répertoire BlobDirectory conserve Arc<store>. BufferedBlobStore/CypherBlobStore conserve, par sa closure, Arc<connexion>, puis la Database native.

Il existe donc une fenêtre permise par ce protocole entre acquittement de shutdown et destruction des ressources détenues par l'acteur. Le marqueur writer natif conservé jusqu'à la fin de destruction Database, relevé indépendamment par coeur-cpp, peut encore refuser la réouverture pendant cette fenêtre. Cela fournit une explication possible du rouge ; cela n'identifie pas le holder exact du run CI. Aucune fuite permanente, ni causalité exacte, n'est affirmée.

## Frontière commune à corriger et plan de témoins

La primitive manquante est une fin de destruction observable par le pool/scheduler. La notification qualifiante doit survenir après destruction de la boîte d'acteur et de ses ressources, pas dans le handler avant Stop. Ne pas assimiler l'événement ActorStopped à cette garantie sans vérifier son ordre par rapport au drop effectif. Le propriétaire Lucivy prépare cette correction ; aucun patch de dépendance ou SDK local n'est proposé ici.

Témoins déterministes à publier avant exécution coordonnée :

1. Un acteur conserve une ressource dont le Drop annonce son entrée puis reste bloqué sur un canal contrôlé. Après traitement de Shutdown, la fermeture ne doit pas annoncer sa réussite pendant ce blocage. La libération explicite du canal permet sa fin, puis le témoin constate la destruction terminée. Aucun délai ou sleep ne sert de preuve d'absence ; prévoir une borne externe pour un éventuel blocage du témoin.
2. Couvrir les chemins multithread et coopératif, les pools reader/shard et le routeur, ainsi que fermeture répétée et fermeture avec erreur de persistance. Une erreur ne doit laisser aucun acteur propriétaire du store.
3. Après qualification de la primitive commune, reprendre le juge natif CI exact avec moteur statique issu de la source corrigée et preuve de lib liée. Conserver son ouverture directe et l'absence de sleep/retry. Tester séparément qu'un writer encore réellement détenu reste refusé.
4. Garder le permis physique : mauvaise origine ou chemin étranger refusé avant Database::new ; voie ordinaire toujours interdite à cette racine. Ne pas confondre cette preuve d'autorité avec la preuve de fermeture.

La recette, les sources exactes et le créneau seront distinctement autorisés par C7. Les verts locaux SDK29 et le préfixe garde Recherche ne remplacent pas le rouge CI statique ; le ticket commun reste ouvert.
