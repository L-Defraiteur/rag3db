# Basculer les trois bases de code de Lucie vers les générations

- **État** : ouvert (16 novembre 2026), préparation active ; autorisation Lucie relayée par orchestration, opérations après FINI de G et contrôles, aucune nouvelle permission routinière requise.
- **Famille** : produit
- **Gravité** : bascule des données partagées préparée et répétée, pas encore effectuée ; conserver l'intégrité, la recherche et le retour.
- **Atteignable en service** : oui (code-rag3db-depot au portable, code-principal et code-lourd à luciepc).
- **Touche rag3weaver** : non pour le code produit ; opérations et preuves privées de migration.
- **Pour** : arbre principal (migrations et contrôles), G (lib, hôtes, fermeture/réouverture des services), banc (oracle des copies finales).
- **Signalé par** : reprise autorisée par orchestration le 16 novembre, après l'audit RO du périmètre.

Plan maître : `docs/optimiseur/16-11-2026/01-plan-de-la-bascule.md` ; recette des bases : `docs/arbre-principal/15-11-2026/02-la-fenetre-de-la-bascule.md` ; parité et tolérances : `docs/banc/16-11-2026/01-la-parite-de-la-migration.md`, §12.

Source audit927941a67a5805450d49f1e04b39fdd542a6059b ; lib préparée par G59ac9c86bc2b3529b590a7462ab7051e3ad26b8a et hôtes rebâtis927/Lucivy4.3.9. Attendre ses artefacts réellement vérifiés/FINI, jamais employer ancien3177/373b/794 par défaut. A1 suspendu, aucune modification produit simultanée.

Pour fermer : pour chaque base, identité exacte, snapshot cohérent et RETOUR conservés72h, maintain frais borné, deux sens verts, versions1b lots2000 puis générations0publiée/1lu, repassage terminé (dernière ligne du journal, pas busy), comptes visibles/mirroirs0/0/vecteurs et oracle contemporain jugés, refus ou migrations partielles conservés sans faux vert. Retour et données d'avant gardés, aucun retrait maintenant. Une base non traitée ou oracle incomplet reste explicitement non qualifié.

Rapport et recette d'exécution : `docs/arbre-principal/16-11-2026/03-preparation-bascule-bases.md`.

## Préflight outil RO — rouge vu avant correction

G a compilé read-only-query.cpp04a contre59ac et joué une fixture privée1nœud : BUILD_ID_ERROR Binder exception, CALLBUILD_ID sansRETURN n’est pas une standalone table function. Journal portable ~/.cache/optimiseur-notes/bascule-16nov-927/query-read.log. Aucune basevivante ouverte. Corriger la seule requête d’identité en CALL BUILD_ID() RETURN *, puis recompilation/témoin parG avant usage.

## Première maintenance réelle — arrêt HTTP 422 avant purge

Recette be91b5b3c742209cc59c2ce2b2ac29f9a97b9213, portable code-rag3db-depot : premier POST /backend/maintain avec table File et dry_run=true refusé HTTP422. Preuves privées : ~/.cache/rag3weaver-build/bascule-live-20261116/code-rag3db-depot/maintenance-01/{phase,verdict}.json. Aucun relevé accepté ni appel de purge, aucune migration hors ligne ; autres bases non rejouées. La quiescence du producteur hébergé reste NON ATTESTÉE.

Diagnostic RO : daemon_host.rs:1038 et 1865 au source927 acceptent précisément manifest/table/dry_run ; header X-Rag3weaver-Session conforme. HTTP422 indique une réponse opérationnelle refusée, sans identifier sa cause. Défaut de recette confirmé : host-phase.py:58–59 intercepte HTTPError puis abandonne son corps ; seule la valeur du statut est conservée. La cause hôte est donc NON ATTRIBUÉE, aucun défaut produit établi. G est sollicité pour préserver le journal existant sans rejouer la route. Le corps initial n’est pas reconstituable depuis les deux fichiers client ; ne pas présenter une future réponse comme sa preuve.

Correction nécessaire avant toute nouvelle recette : archiver statut et corps de refus, sans jeton ni headers d’authentification, avant de lever l’erreur ; témoin hermétique HTTP422 pour vérifier conservation et arrêt. Ce paragraphe est publié avant modification du lanceur. Premier rouge conservé, pas de retry automatique ni de fermeture/snapshot comme si maintenance terminée.

### Conservation corrigée, cause initiale toujours inconnue

Journal existant G conservé host-journal-after-maintenance-01.log SHA2563d58adecef03aaaad5afe07263732813aad7085d7a0e9e2eabd1b1ecb1dab997 ; aucune entrée dans la fenêtre, journalctl utilisateur également vide. Aucun corps initial retrouvé. /etat observé par G : ouvert, sans held_closed/refused ; ce n'est pas une preuve de quiescence.

Correction privée : capture du corps HTTPError en hexadécimal, plafonnée à1MiB avec indicateur de troncature et longueur lue, répertoire chmod0700 ; route/statut archivés avant exception, aucun header ni auth copié. Le témoin hermétique de conservation est vu rouge avant ajout de la primitive, puis neuf témoins passent ; contrôle AST du véritable handler. Aucun HTTP réel, Cargo, base ou service dans ces témoins. Ancien verdict422 intact, aucune cause hôte déduite.

Piste RO distincte signalée par C7 : fts_releve_a_blanc utilise les handles déjà ouverts, dont l'ouverture est paresseuse ; backend ouvert ne prouve pas handle FTS ouvert. Ce n'est pas l'attribution du refus initial. Toute sonde dry_run File seule doit avoir recette et arbitrage distincts ; aucun retry maintenance autorisé implicitement par ce correctif.

### Sonde distincte live-ids — résultat réel

Source9a8ea38ce3, autorisation C7 distincte : /backend/fts-live-ids SEUL/File, session fraîche, client45s sous confineur60s/poste32GiB. Verdict ROUGE validation-502hg_hl ; preuves ~/.cache/rag3weaver-build/bascule-live-20261116/code-rag3db-depot/diagnostic-01. Corps422 intégral capturé, non tronqué : « pas d'index plein texte ouvert pour « File » (ouverts : []) », ok=false/transient=false. Cela établit l'absence de handle ouvert AU MOMENT DE CETTE SONDE, pas l'absence d'index sur disque, ni le corps du premier refus. Aucun maintain, purge, compact, ensure/warmup, autre base ou fermeture lancé. Aucune qualification de quiescence du producteur hôte. Maintenance reste arrêtée ; suite à arbitrer avec G/C7 sur recette nouvelle.

### Suite autorisée C7 : inventaire sur copie privée froide

Arbre prépare outil de lecture fichiers (configuration racine/shards/génération, inventaire SHA et noms par index/cellule sans supposer default), fixtures absente/corrompue avec preuve d'absence de mutation, et sonde native readOnly59ac déjà qualifiée. Entrée : snapshot G prélib à froid, ensemble base+extensions+fts/checkpoints copié par reflink vers destination privée neuve ; snapshot G et vivant jamais ouverts en écrivain. Avant/après : SHA des fichiers, pas seulement tailles. Lecture native sur COPIE uniquement, mémoire1GiB et poste commun32GiB borné. Inventaire physique seul ne qualifie pas décodage Lucivy ; ouverture snapshot immutable4.3.9 distincte à qualifier, aucun fallbackcreate. Recherche porte primitive réutilisable existing-only dans ticket distinct avant code ; aucun produit modifié par cet inventaire. Hôte reste ouvert, maintenance STOP. Publication recette/pins/fixtures avant lecture réelle.

### Préflight natif froid — rouge latéral avant toute copie

Recette4c2b0ea7f7 réellement jouée sous portée600/poste32GiB, validation-axdpr7th. Fixture absente : refus nonzero sans création. Fixture corrompue : refus nonzero « file is not a valid Rag3db database file », MAIS apparition de corrupt.rag3db.readers vide (0octet). Le test d'inventaire ensemble a donc arrêté AVANT lecture/hash du snapshot source, reflink ou ouverture de sa copie (copy absente). Preuves cold-inspection-01/{absent,corrupt}.stderr et verdict.json, seules fixtures privées touchées. Aucun fichier snapshot/base vivante ouvert.

La qualification G antérieure vérifiait le hash du fichier DB, pas l'absence de fichiers latéraux ; ne pas la transformer en non-mutation de l'ensemble. Ce constat n'attribue pas le422 initial. Native readOnly59ac ne satisfait pas ici l'exigence sans effet latéral. Pas de relaxation ni retry : C7 arbitre latéral.readers sur COPIE seulement ; inventaire fichiers pur et outil Lucivy immutable restent des voies distinctes sans moteur natif. Maintenance reste STOP, ancien rouge conservé.

### Inventaire fichiers portable accompli, portée strictement physique

Source07779472c7, portée validation-4haq4mdx VERT (client seul, aucun producteur hôte inclus). Résultat cold-files-01/verdict.json FILESYSTEM_INVENTORY_ONLY : 3007fichiers ensemble snapshot/copie, SHA source/copie avant/après identiques ; FTS3004fichiers. Quatre index racine Lucivy_File/Scope/Library/Symbol, chacun4shards ; générations disque respectives4526/5502/142/3573. Configuration et inventaires intégralement conservés, aucun moteur natif ou Lucivy ouvert. Aucun nom d'index de cellule supplémentaire observé dans CET snapshot ; cela ne remplace pas le contrôle des cellules en base.

Le décodage Lucivy4.3.9 et l'accord générationDB restent NON QUALIFIÉS. L'inventaire natif antérieur reste ROUGE pour latéral.readers sur fixture corrompue. Snapshot G original conservé, copie privée cold-files-01/copy disponible pour les lectures suivantes après recette appropriée.

Empreintes résultat : verdict.json SHA256 b84a159517cc6d5c9e2ae0d5b8b152b81f6820ab47350708b66f189f8ea7e38d, fts-inventory.json SHA256 1ca53be9831e0eee2b76c0f13d9fd7f7d1bd8767c99950ddde42a5b090110d0a, source-before.json SHA256 71377e1da485835454b9c7d8317c2069d5f948563dad4d268569f37c6d3583a2, source-after.json SHA256 71377e1da485835454b9c7d8317c2069d5f948563dad4d268569f37c6d3583a2, copy-before.json SHA256 71377e1da485835454b9c7d8317c2069d5f948563dad4d268569f37c6d3583a2, copy-after.json SHA256 71377e1da485835454b9c7d8317c2069d5f948563dad4d268569f37c6d3583a2.

### Inventaire natif sur copie privée02 accompli

Sourcea3116aa7d2, portée validation-t1977o3g VERT sous600/poste32GiB, query43cff et BUILD_ID59ac effectifs. Fixtures absente/corrompue refusées ; seule création autorisée corrupt.rag3db.readers régulièreUID1000/vide consignée. Snapshot source strictement inchangé, copie3007fichiers inchangée (readers déjà présent sur cette copie). Show_tables33tables ; File19colonnes et8320lignes, Scope34colonnes et81347lignes ; les deux regroupements donnent uniquement cellule default/default sur ce snapshot. Aucun code_scope_form retourné, Scope_Chunk présent et pas de ScopeContent observé ; pas de migration jouée.

Comparaison explicite générationDB/dossier : File4526=4526, Scope5502=5502, Library142=142, Symbol3573=3573, quatre accords. fts_counts_at_index_end et maintenance03:30 sont des métadonnées historiques, pas des comptes Lucivy actuels ni une nouvelle maintenance. fts_promesse vide observée. Copie privée cold-inspection-02/copy disponible ; décodage Lucivy4.3.9 toujours NON QUALIFIÉ, hôte vivant non interrogé par cette étape. Rouge01 conservé, aucune purge ni service touché.

SHA256 génération-comparison.json a02d66b1060e3cc2c4092409779bfb6094294e938a1a24865a6393fa76c429fd, native-inventory.stdout 930901a88a3376fd62ec6768cfc689bf230b4ab966149563346807994b68d691.

### Décodage immutable PC accompli sur les deux copies froides

Sourcea21654fa9a, validation-grnev9rk VERT,8.784876523s<600, une compilation rustc1.98.1 rc0 et aucunCargo. ELF248222d2f76e43465730d0f2c1da5f0ce7101196c3440c609ac0094257fd36f1, preuves immutable-pc-01 ; manifeste65fichiers SHA170026a346c35c2f91a326c1d6fa796039ad0184422a9278af52763ec50cfa12. Fixtures absente/corrompue refusées code1/SNAPSHOT_REFUSED et ensembles inchangés. Parbase711fichiers sourcesG strictementinchangés/copies conformes. Huit ouvertures open_snapshot/is_read_only/commitREFUSED, chacun4shards. Principal documentsFile6313/Scope65913/Library398/Symbol73373 ; lourd7275/82848/464/84928. Les comptes File/Scope graphe égalent ces cardinalités, PAS une bijection d'ids démontrée ni absence d'orphelins. Générations DB/FS8sur8 égales : principal13/8/18/11, lourd13/8/17/11 dans ordreFile/Library/Scope/Symbol. Une cellule default/default par table File/Scope sur ces snapshots ; code_scope_form absent. Aucun vivant ni maintenance qualifiés.

### Décodage immutable portable accompli après fixtures ISA locales

Sourcea86ef5af20, validation-qohrfmyw VERT,11.534303557s<600. ELF248222d2 copiéPC puis rehashé ; fixtures absente/corrompue exécutées réellement surportable, code1/SNAPSHOT_REFUSED/ensembles inchangés, avant la copie réelle. Quatre open_snapshot/is_read_only et commitREFUSED, chacun4shards. DocumentsFTS File8320/Scope81347/Library454/Symbol90276 ; comptes File/Scope égaux au graphe précédemment lu sur le même inventaire froid, sans bijection d'ids démontrée. Inventaire sourcePRIVÉEcold02 qualifié et nouvel ensemblecopie3007fichiers strictement inchangés. Pasde moteurDB, .readers, hôte, maintenance ni basevivante dans cette étape. Coût eager Scope858474463octets empaquetés, pas live-only/minimal.

Les trois copies froides sont donc décodées avec Lucivy4.3.9, générationsDB/FS12sur12accord ; cela ne qualifie ni leur étatlive actuel, ni absence d'orphelins, pertinence/oracle, dettevectorielle ou maintenance. Le premier422 sanscorps reste NONATTRIBUÉ ; sonde ultérieure handles[] indépendante ; aucun retrymaintenance, purge, warmup ou ouvertureécrivain. Recherche porte primitiveexisting-only distincte, avant reprise de la bascule.

Preuves portable immutable-portable-01/proof-manifest.json : 28fichiers, SHA256 4c5d9e51b33c88660e77c79238d6f203b3bff4d333fc6e35843d1656e913d1b3.

### Suite C7 : comparaison des ensembles vivants, avant toute décision purge inutile

Nouvelle action autorisée sur copies froides seulement : lire CHAQUE _node_id vivant des snapshots immutables File/Scope (alive_bitset ; colonneu64 obligatoire ; exactementunevaleurpardocument ; doublonsrefusés), puis les OFFSET(id(n)) graphe par query59acreadOnly, comparer les deux différences. Erreurs/type/doublons/comptage/parserincomplet refusent, pas de dédup silencieuse. Sourceoutil/recette et témoinssetdifference à publier avantrun,600s/poste32G parposte, rustc court sansCargo surPC si nécessaire et fixturesISA portabledistinctes. Absence d'orphelins ET manquants +générationsaccord surcopiesfroides ne vaut paslive : permet seulement proposer fermetureG pour snapshotFRAISdiagnostic et même comparaison avantmigration, purge nonnécessaire/compactreporté explicites. Toute différence STOPvoieNOP, anciens422rouges restent conservés. Aucun maintien ni warmup implicitement autorisé.

### Ensembles exacts sur copies PC — zéro différence

Source9bcab4965a, validation-0_vhpecn VERT,7.795124363s<600, une compilationrustc rc0. ELFIDs d0dd13e6d403e614038b251f413637d5f6d02a98ba1c10509388d0be1f5e15c7. Pourchaque table, IDsFTSvivants relevés avantquery OFFSET(id(n)), types/dups/complétude stricts. PrincipalFile6313/Scope65913 etlourdFile7275/Scope82848 : les DEUX différences sont vides dans les quatrecomparaisons, pas simple égalité de compte. Générations8sur8égales, inventaires sourcesprivées/copies identiquesavantaprès. Preuves id-sets-pc-01/*-set-comparison.json et sortiescomplètes. Portées et lecturesdepsGfinies ; aucunprocessusutilisanttargetGpararbre, Gprévenu pour ses archives. Copies froides seulement : nouvel étatlive/purge nonencorejugés.

### Ensembles portable — zéro différence ; nouvel ordre proposé

Sourcecc12cf35da, validation-_67ggcda VERT,11.937765126s<600. NouveauELFIds d0dd13e6 préflightISA réelportable puis ensembles : File8320 etScope81347, orphans[]/missing[] pourchaque, gen4sur4nonnullesaccord, sourcesprivées/copieinchangées. Ensemble troisbases : SIXcomparaisons de sets strictes sansécart, douzeaccordsgénération surlescopiesfroides, aucune déductiond'étatlive. Manifestespreuves : portable34fichiers SHA1cc2d5b1812a494d48b26cf2a63823819aee64c5deed72d6faea44cb94168ce1 ; PC70fichiers SHA903c3fcbbba0da52b3a8dcbdac40c8d189906d4af319bc42b4b4d08068bdb0c2. Ancien422 et rouge.readersinitiaux restent rouges.

Plan court révisé : docs/arbre-principal/16-11-2026/04-plan-court-bascule-sans-purge.md. C7 autorise OPSconditionnelles aprèspublication, ordrecode-principalPUIScode-lourd ; G ne ferme que pour snapshotFRAISdiagnostic+retour, PAS sousprétexte de maintenanceverte. Requalification fraîche indispensable avant dispense depurge/CLI ; sinonSTOP etaucuneCLI.

### Refus avant close principal — état hébergé antérieur périmé

G a arrêté la demande6dce AVANT POST/backend/close : aliases[] ; /etatPC12:20:35 donne PID3761669 (ancien3136522), build0e310/source927/lib59ac conformes mais backends[]. Aucune fermeture/ouverture/snapshot frais/CLI exécutée. Cause du changement/inactivité NON PROUVÉE. Preuves G ~/.cache/optimiseur-notes/bascule-16nov-927/diagnostic-code-principal-20261116-122008/{pins.json,held-close.log,unexpected-state.json}. Code-lourd ne doit plus être annoncé ouvert ; l'état11:32 est historique.

La fermeture d'un hosted_existing inconnu ne peut pas poser le hold voulu ; aucun writer ne sera ouvert pour contourner ce refus. La preuve initiale copiesfroides reste bornée, aucunepreuvefresh ni dispensepurge accordée. Adaptation opérationnelle à arbitrer par C7/G : sous nouveau constatbackends[] etabsenceFDde toutesbasesconcernées, mise horsactivation contrôlée du SEULhôteAPI7970/service/socket parG peut remplacer le holdparbase, sans toucher Embar7878/7879/autresservices ; identité/unités/retouràattester avantexécution. Aucune autorisation d'arrêtglobal déduite de ce paragraphe, aucunserviceactionnépararbre. Autre mécanisme holdexisting-only absent reste produitdistinct àqualifier siC7lechoisit.

### Freshprincipal : gates RO vertes, commandes officielles à répéter sur copie

SnapshotG diagnostic-code-principal-20261116-122933, reçuSHAe21599096e1a6b93ea120fbddaae5dfcf7c9508a060a5e4a9e1aa3cc917d02f1,733entrées. Freshsets validation-5bcj5e_4 VERT3.3817s File6313/Scope65913 sansorphelin/manquant/gen4égales ;verdict6b16aa39c05e1fa9944aaf4b855279cd76ee3a1f957df8ea34c00e6c465c5810. Fresh2sens validation-a44_ui1c VERT11.0943s18tables/union931405/lost0 ;verdict5e13925af1020bebf2255b8fe1bc2c48232c79990771c26b85742f88d445efa2 ; même sourceinventoryb3c152800d5cd6a641a92e5bf2fd27e2dbac6333aac1ce96d95a447d88c2b1e9. API/socketPCtoujoursarrêtés. AucuneCLIvivante.

C7 demandeavantlive preuveOFFICIELLE927/59ac1b+gen surJETABLEsupplémentaire1200 : outil/testsgates/profil/commandes publiésavantentrée, anciencouple794/373 exclu. RET_v2(WALnéaprès) attendu séparément deG avantvraiewriter ; ancienGOLDimmutabilitéconservée. Voirplan04pourrecette, autreterme maintenance422 resteROUGE etcompactREPORTÉ.

### Préflight officiel copie337 — rouge AVANT tout écrivain

Portée validation-0fx58rsr ROUGE : gardeunits_inactive() refuse rag3weaver-daemon.service dèsentrée. AucunPopenwriter, aucunecommandeversions/generations etpasdedossiertarget créé. failure-source-audit.json atteste snapshot_unchanged=true etcanonical_live_unchanged=true ; seulespreuves/profilprivés créés. Le drapeau conservateur target_may_be_partially_migrated=true du verdictinitial n'est PAS une mutationobservée : targetabsent etaucunwriterl'établissent, nepasréécrire ce verdict.

Piste de recette : systemctl--user hérite du bus/XDG_RUNTIME privés du confineur et ne peut pas interroger le manageruser réel ; initialstdout/stderrdiscardés parunits_inactive, donc la cause brute initiale n'est pas reconstruite. Ce refus ne prouve pas APIréveillée, aucun défautproduit attribué. G est sollicité pour attestationROunités horsvalidation ; aucun busLucie n'est restauré danslaportée. Diagnosticdistinct+archivage/testsgarde avantfix/reprise, pasretryCLI ni autrebase àcausecommuneinconnue. RET_v2 attestationG fournie séparément SHA24eced05414acb1e505b49ff018519820cc48cbbb6597824bf7af47befda452b, toujoursàintégrer avantLIVE ; GOLD e215 immuable.

### Cause de garde confirmée par observation distincte, correctif proposé

Diagnosticfd2/service-guard-diagnostic-01 sousvalidation-x1zr7yq4 : systemctlshow exact rc1/stdoutvide/stderr « The name org.freedesktop.systemd1 was not provided by any .service files ». Busprivéset, XDGruntimeabsent ; aucunwriter. Ce NOUVEAUcorps n'est pas l'initialperdu, mais établit l'incompatibilité du gardeavecCommon. G horsCommonunitésinactive/dead/MainPID0/port7970sanslistener, preuve8f3a131f. C7 validemanagementGhorsCommon +filsprivé ; contrat05 etconsommateur/bonsmauvaisstdouttests17CPU avantnouvelétapecorrectiveproposée1200 (ancienreceiptterminéETexpiré). AucunepermissionderéinjecterbusLucie, ni deLIVEwriter ; sourcesnapshot/canonicalauditéesinchangées, retour_v2distinct reçu etqualificationcopyCLIencoremanquante.
