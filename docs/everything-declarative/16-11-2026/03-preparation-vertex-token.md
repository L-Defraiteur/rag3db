# Préparer le renouvellement des jetons Vertex

Actualisation PIN FINI G : reprise des deux commits préparatoires à
l'identique sur `68590270e21d00285b29800bd9cdecf0cb921461`, branche neuve
`cx-ed-vertex-token-refresh-r2`, WT portable
`/home/lucied/git_workspaces/rag3db-cx-ed-vertex-token-refresh-r2`.
Tête avant cette annotation documentaire :
`b402243bb78d973434842f82e209fb69d58370df`, diff produit r1→r2 vide.
Aucun correctif, compilation ou rouge exécuté. Les deux juges restent
NON JOUÉS ; la branche r1 est conservée. Qualification après landing ED ties
et créneau C7, sous lib59ac épinglée complète/sans MOTEUR_ANCIEN, script cache
SHA2150fcd847b1df7de04908d357a0c78abc05e50ce599e2793ff08ab48108825d
conforme à685 ; pas de commande sur l'ancien pin373 ni `lib_seule.sh`.
Le rouge exact utilisera `run_e2e.sh --lib`/la vraie passe
`SUITE_DU_TEMOIN=lib` avec son nom complet, dans une borne fraîche encore à
coordonner. Les slots actuels restent Recherche/Archi puis ED, pas de Cargo
automatique au seul reçu de FINI G.

Ticket : [un modèle Vertex déclaré garde son jeton d'une heure](../../banc/tickets/2026-11-16-un-modele-vertex-declare-garde-son-jeton-d-une-heure.md).
Préparation après audit de master927941a67, branche `cx-ed-vertex-token-refresh-r1`.
Aucun Cargo, faux serveur lancé, service, cloud ou lecture d'identifiants. La
fenêtre G/Arbre reste prioritaire. Le lot d'ordre stable doit atterrir avant.

## Contrat et frontière

`connect_llm` crée TokenSource, appelle token une seule fois et construit
OpenAiLlm::vertex avec un Bearer fixe. TokenSource conserve déjà son cache,
son renouvellement et invalidate ; le client doit conserver cette capacité,
plutôt que recopier une fois son résultat. Une interface de source de jeton
peut unifier la source réelle et la source factice sans nouveau client HTTP.
La clé d'API fixe reste fixe ; aucun code d'activation ou embedder modifié.

Avant chaque appel authentifié Vertex, obtenir le jeton par la source. Si le
serveur refuse401 AVANT le flux : invalider puis retenter UNE fois avec le
nouveau jeton ; deuxième401 en erreur nommée. Ce renouvellement n'est pas le
retry générique des429/5xx ; il ne doit pas devenir une boucle. Après200 et
ouverture du SSE, ne jamais rejouer, même si le flux casse. Ne pas publier
de jeton dans erreurs, Debug ou journaux.

## Rouge à préparer avant correctif

Faux HTTP loopback privé et borné, source factice au jeton1 puis jeton2 après
invalidation. Premier appel jeton1 accepté ; second appel jeton1 refusé401 ;
retentative jeton2 attendue et acceptée. Attendre trois requêtes, une seule
invalidation et aucun fragment dupliqué. Le code actuel rend une erreur au
second appel. Aucun sleep d'une heure nécessaire.

Le premier témoin de transport peut reproduire la connexion actuelle en
prenant une fois le jeton factice pour le VRAI constructeur vertex existant.
Cette reproduction n'est pas encore une preuve du raccord de déclaration
`connect_llm`. Avant correctif, jouer ce rouge sous le créneau autorisé ; le
correctif raccordera ensuite le même scénario à la source retenue par le
client de production, sans modifier serveur, assertions ou scénario pour le
faire passer. Un témoin complémentaire devra couvrir le raccord déclaré.

Compléter les juges : deuxième401 persistant et borne exacte ; renouvellement
entre appels sans401 ; clé API non invalidée/non réessayée sur401 ; SSE déjà
ouvert non rejoué ; erreur d'obtention du jeton nommée sans valeur secrète.

## Recette future

Après FINI G et script épinglé neuf seulement : sources et sous-module figés,
lib59ac annoncée et résolue en id exact, ancien mode retiré selon annonce.
Rouge exact fauxHTTP sur le code avant fix, sous vrai poste/32G/j8/timeout
convenu ; journal conservé et commit du témoin publié. Puis correctif et même
juge vert, autres témoins de frontière ; passe courante proposée
`SUITE_DU_TEMOIN=e2e_demon_hote SUITES=-`, avec F8/lib entière/NAV de l'outil.
Pas de F300 ni accès cloud pour prouver ce correctif HTTP ; la mesure F300
revient au banc après qualification, avec sa propre décision.

Tous ces juges et recettes restent NON JOUÉS à la préparation. Aucun À TOI
ou délai historique repris, aucune modification de master ni pose d'hôte.

## Témoin de transport écrit, non joué

`src/openai_llm/vertex_refresh_tests.rs`, raccordé sous `cfg(all(test,
feature = "daemon"))` (le faux HTTP utilise la dépendance tiny_http déjà
présente). Deux fonctions :

- `vertex_rejected_token_is_refreshed_once_before_opening_a_stream` : premier
  appel accepté puis401 ; attend réussite, trois requêtes et une invalidation.
  Rouge attendu sur le client figé actuel, mais NON OBSERVÉ à cette étape.
- `fixture_token_source_changes_its_cached_token_only_after_invalidation` :
  contrôle de la source factice, qui ne remplace pas le juge de transport.

Le helper `current_declared_vertex` reproduit explicitement la frontière
actuelle : il prend un jeton factice puis appelle le VRAI constructeur vertex.
Il ne prétend ni retenir déjà cette source dans le produit ni passer par
`connect_llm`. Ce raccord et son témoin complémentaire restent à écrire après
observation du rouge. HTTP sur loopback aléatoire, client borné3s, serveur
borné8s et joint au Drop, pas de sleep d'expiration, pas de service/cloud ou
identifiant réel. Les refus passent par la racine d'état temporaire déjà
imposée par `builtin_templates::state_root` sous cfg(test).

Aucun comportement de production modifié, aucun verdict de compilation ou
d'exécution ; seul `git diff --check` a été contrôlé. Le correctif de
renouvellement reste NON ÉCRIT avant le rouge exécuté.

## Reprise après livraison TIES : audit avant nouveaux témoins

InstructionC7 de reprise aprèsmasterd395 etnettoyageTIESlivrés. WIPc642b40db2acccaab89ea5b186b97d87f29b998c propre, deux fonctions NONJOUÉES. Nouvellebranche/WT cx-ed-vertex-token-refresh-r3/rag3db-cx-ed-vertex-token-refresh-r3, basefetch **df2a5789888d16247f0dad42dcdaa3c4ceea2094**, rebase3patches= sansconflit, têtepréparatoire156986b340bcdfcdb56792a30bfd2cfc156bf281. Aucun delta des trois sources model_source.rs/openai_llm.rs/gcp_auth.rs entrebase685 etdf2 ; rechercheGit tousbranches : pas de correctif authVertex concurrent identifié. Pas de travail Claude dupliqué établi ; tout nouveau delta fera arrêter pour relecture.

Embarquement confirme sa zone disjointe : model_source/activation.rs uniquement DenseTenu::called_address etsonjuge, plusratekeyembedder/catalog/progress/daemonembeddings ; aucuneéditionconnect_llm/openai_llm/gcp_auth de sonlot07731af482ac0ed65ae3775e49900215d40ee2bf. ED réserve précisément connect_llm brancheVertex, Auth/constructeurVertex/envoiHTTP401 etcapacitéTokenSource si nécessaire, pas le module entier ni un service. Aucun fichier de sonW/T/pilote touché.

AVANTnouveaudiff : compléter le scénario de test par un serveur toujours401, un juge du refuspersistant (UNE invalidation/deuxrequêtes/erreurnommée) etdeuxgarde-fous positifs (BearerAPIfixe401sansrepriseauth ; SSEdéjàouvert puis401dansfluxsansrejeu ni fragmentsdupliqués). Les deux juges existants etces3nouveaux restent NONJOUÉS. Aucun correctif de production avantpremierrougeréel ; sourcefakepréserve sa frontière explicite de snapshot, raccordproductionconnect_llm/source retenue àfaireaprèsrouge. AucunvraiTokenSource::from_env ni cloud/clé/credential ne sera appelé par lesfakes.

Recette rouge bornée àmatérialiser séparément : poste32G/HIGHmax/quotaPARPLACE/j8, TMPcourtprivédisk etétatsfrères, cachemagasin59réel/pin2150 vérifiés, targetsessionneuf (ancienTIESsupprimé). CompilationCPUdaemon+openai-llm sansnative, inventaire5fonctions/features/copieELF/ledgerpubavanttest ;positifsbornés puis premierrouge transportexact attendu STOP, sansretry ni patchautomatique. PasdeGOexécutiondéduit, aucunCargo/rustc/test/serveurfake/target créés àcettepréparation. C7 arbitre budget/recetteexacte avant lancement. F300resteaubanc horspérimètre.

## Complément des cinq témoins publié, toujours sans exécution

Code publié **2b3bdfd1ccbb8649009c859f506f0debb04ae930** aprèsplan e82a3c9258. Cinq fonctions #[test] : sourcefacticedéjàprésente, refresh401,401persistant(uneinvalidation/deuxrequêtes/erreurVertexnommée), BearerAPIkeyfixe401sansrefresh etHTTP200/SSEpartialpuiserreur401sansrejeu. Serveurloopbackaléatoire, client3s/serveur8s/dropjoint, aucunTokenSource::from_env/cloud/credential. Le snapshotfake→vraiconstructeurVertex reste la frontière AVANTfix ; aucun correctifproductionni raccorddéclaratifinventé.

Rustfmt --emitstdout a PARSÉ le fichier, gitdiffcheckVERT. Les5fonctions sont ÉCRITES/NONJOUÉES, aucunCargo/rustc/ELF/target/fauxserveur démarré. Une recette rouge isolée est préparée surbranchepiloteNEUVE ; le nom du targetsession ancienTIESpeut être repris car son retrait etsonabsence sont prouvés(rapportnettoyage90a92), jamais comme cacheporté. Qualif rougeCPU seulement ne remplace ni natif/F8/liblarge/NAV ni futurjugeconnect_llm. Budgetproposé1200neufs aprèsposte(compile900,contrôles3x30,refreshred30,pausepublication<=120) ; GOexactC7reste nécessaire.
