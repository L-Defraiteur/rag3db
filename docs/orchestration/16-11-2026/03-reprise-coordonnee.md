# Reprise coordonnée du 16 novembre

Lucie autorise la reprise après la passation et précise : « que chacun explore les diffs récents de son sujet peut-être avant de commencer ».

## Référence et première étape

Référence lue et récupérée : `927941a67a5805450d49f1e04b39fdd542a6059b`.
Les treize pairs ont reçu leur page du 16 novembre et confirmé leur lecture. La première mission commune est un audit en lecture seule des changements récents de leur périmètre, de leurs tickets et de leur éventuel WIP contre master.

Chaque retour donne sa référence, les changements de contrat, le travail déjà livré à ne pas rejouer, le premier ticket encore valide, les chemins à réserver et les validations proposées. Les anciens GO et échéances restent éteints. Le démarrage de code et la répartition des passes suivent ces constats ; aucun tour de push de code n'est ouvert pendant cet audit.

## Ressources relevées au démarrage

- Portable : `poste --qui` libre, zéro lourd sur une place.
- Luciepc : un lourd sur quatre, TSan du dictionnaire (`concurrence_test.tsan`, PID du tenant 2369995). Son résultat et son propriétaire sont à relever par cœur, sans arrêt ni relance par l'orchestration.
- Arbre de Lucie : les modifications préexistantes du sous-module codeparsers et le dossier non suivi `extension/rag3weaver/docs/lucie-tmp/` sont conservés.

Ces relevés sont ponctuels : ils ne réservent pas une place pour une passe future. Les validations utilisent les verrous, les plafonds et les garde-fous courants du dépôt, dont les suites exigées.

## Priorités de reprise

1. Moteur : lire le résultat TSan existant, puis reprendre les défauts par gravité avec cœur et hotfixs sur des fichiers attribués.
2. Algareth : reprendre le lot du pas qui va au bout et les défauts de contexte/délivrable ; rejouer la campagne empirique après les changements concernés.
3. Vues et recherche : distinguer ce qui est livré (écrivain, chargeur, outils sous vue) de la suite par branche/worktree et des bascules qui restent à préparer.
4. Lots indépendants des autres périmètres : choisir à partir des audits, sans refaire les corrections du week-end ni mettre deux propriétaires sur les mêmes chemins.

Les treize audits ont été reçus : chacun cite la référence ci-dessus, les changements de contrat, le WIP et le premier ticket valide. Aucun chantier nouveau n'est créé pour occuper une session.

## Attribution après les audits

Les préparations se font sur des branches `cx-…`, avec les tickets déjà désignés dans les passations. Aucun tour de push de code n'est ouvert. Pendant la fenêtre ci-dessous, aucun nouveau lourd des autres sessions ne part ; les lectures, le code privé et les petits témoins hermétiques bornés peuvent avancer.

| Porteur | Premier travail confié | Frontière |
|---|---|---|
| optimiseur | Audit frais, qualification et bascule de la bibliothèque, hôtes à la source gelée | Seul porteur des poses et des services ; préavis avant coupure, preuves et FINI aux deux postes |
| arbre principal | Préparer puis réaliser la migration des trois bases après le FINI de G | A1 des sondes reporté ; instantanés, retour, contrôles de graphe et vecteurs |
| cœur C++ | Lire le TSan en cours et poursuivre le diagnostic du dictionnaire | Ne pas intégrer le correctif non prouvé ; seul propriétaire de sa chaîne TSan |
| hotfixs | Rapprocher le ticket ADD COLUMN de son témoin déjà livré ; ensuite inspection `rows_outside_graph` | Ne pas recréer le témoin ; aucun élargissement aux fichiers du cœur sans coordination |
| archi | Préparer le landing du lot où le pas va au bout | `teammate.rs`, `teammate_tests.rs` ; nouvelle qualification après la fenêtre |
| everything declarative | Préparer le landing de l'identité stable des ex æquo, puis le jeton Vertex | `search.rs`, `generic_search_nodes.rs`, témoin de `backend.rs` ; pas de nouvelle passe avant FINI G |
| recherche | Rafraîchissement et datation de `fts_counts` depuis le relevé existant | Pas de purge ni nouveau jeton ; landing après ED, fonctions de backend/catalogue à préciser |
| embarquement | Clé du débit par adresse et poste, reprise de `rate-key` | Aucun fournisseur activé pour lire une clé ; `None` ne devient pas « local » |
| banc | Témoin du débit au faux service, entrée 6 pas 2 | Aucun appel aux services partagés ; clé à coordonner avec embarquement |
| mémoire | Reproduire et corriger le `Text file busy` des fixtures | `codex_threads.rs` côté tests ; hypothèse du FD hérité à prouver |
| tty | Alias anglais compatibles des commandes et options | `commandes.rs`, `main.rs`, tests et README ; aucune modification d'hôte ou de thème |
| codeparsers | Terminer le WIP des mots vides Rust et du std homonyme | `code.rs`, `e2e_usages` et chemins du sous-module annoncés ; pointeur et règle d'analyse dans le même lot |
| lucivy | Préparer la validation de l'ulp en parse et retrouver le corpus | Si absent, consigner le blocage avant l'entrée suivante ; aucune fusion de PR #42 ni publication |

Le témoin ADD COLUMN existe déjà sur master : c'est un résultat de l'audit de hotfixs, pas un nouveau vert d'exécution. Les pages du 13 sont historiques. Les champs et mécanismes de générations, vues, socle, archiviste et services livrés du 14 au 16 ne sont pas à réimplémenter.

## Opérations séparées de la reprise de code

Après le résumé demandé sur le travail du week-end, Lucie donne un mot distinct : « il faut pas hésiter à le faire nouvelle bibliothèque et migration des bases ». Ce mot autorise la fenêtre maintenant ; il ne faut pas redemander une heure à l'utilisatrice.

G prend la priorité : bibliothèque `59ac9c86bc`, audit de fraîcheur des artefacts, ABI et preuves, hôtes de la source `927941a67a5805450d49f1e04b39fdd542a6059b` si les binaires préparés sont périmés, qualification sans moteur ancien, puis pose aux deux postes selon le plan du 16. Le code de master reste gelé pendant cette préparation ; la documentation reste sur la branche de coordination. Le TSan reste sous le contrôle de cœur, que G contacte pour la place si nécessaire.

Arbre prépare les migrations et les réalise seulement après le FINI vérifié de G : instantanés et retour, contrôles des deux sens, maintenance, 1b et montée aux générations, repassage puis contrôles visibles, miroirs et vecteurs. Le retour est gardé 72 heures. Aucun retrait d'ancien index ni nettoyage des ressources n'est inclus maintenant.

Les poses et migrations restent sous leurs plans et propriétaires. Ce mot n'autorise ni un contournement du refus de fusion de la PR lucivy #42, ni un tag ou une publication. Une cause bloquante est rapportée avec ses preuves ; les autres préparations continuent.

## Premiers retours de préparation

- G confirme les deux bibliothèques au magasin : `librag3db` md5 `697c7a0cdf7a5dc7860a5c13b7e1135f`, `libvector` md5 `925b6b1331e78a3b1bd112497611bd95`. L'hôte de 927 est reconstruit, SHA256 `0e310a9cfe1c9e12534f3a579b3e781dcef7e9335e37d7e57349b16d053d5288` ; lucivy 4.3.9 au lock, essais 7991 verts aux deux postes. Le verdict des trois suites compile encore : **aucun vert global, aucune coupure à ce relevé**. Plan de G publié sur `cx-optimiseur-bascule-16`, tête `806715b2d6` puis complément `dc32e7073a` ; contrôle des backends par `/etat`, pas le vieux script qui ne jugeait pas un 404.
- Arbre a publié la préparation `04a3970be09a06fc5ff1fabd8f8b18cc71d3ee9e` sur `cx-arbre-bascule-bases-20261116-r1`, dont un nouvel outil privé explicitement en lecture seule ; G le compile contre 59ac. Aucun accès d'arbre aux bases vivantes avant FINI G. Banc a publié sa recette d'oracle `df02f39a06c9949d285acd08747824f30cfa0297`, avec un ticket propre, et attend copies finales, manifestes et empreintes du working tree/non suivis. Ses limites de score distinguent BM25 brut, fusion et dense.
- Archi : candidat `e8b147277fcd4cc91c293d5ada08aec42d482ce7`, deux patches conservés au rebase ; aucune nouvelle passe. ED : candidat des ex æquo `f1cfa89542247d5d575f9953c80ce6d83f61dce1`, quatre patches conservés ; préparation du témoin Vertex `d148fe0e8701f1653f6261d9ce601a3e4cbc5dd5`, non joué et sans correctif.
- Mémoire : WIP `61dc67eb18b7c60e81894c4670a4903ee8b02aee`, rouge/vert CPU du mécanisme Linux et de la commande extraite, **pas encore du helper Rust compilé**. Le lot ne revendique pas l'attribution unique du rouge historique. Tty et banc ont publié les témoins d'alias et de débit, non joués en Rust ; banc tête `675cf02000b4c0540a3bd89ebc7eb2384bd74059`.
- Embarquement : contrat de clé accepté (écritures automatiques `address@host`, lecture ancienne en secours ; API historiques explicites conservées), WIP `5fddd5481d0787654f243aad4a07a94f159822db`. Relecture statique mémoire favorable ; code et neuf témoins non compilés/non joués. La provenance lue après une mesure n'est pas une attestation atomique d'un index multi-fournisseur.
- Hotfixs : rapprochement du témoin ADD COLUMN publié `c92d557bd8` ; le rouge du juge global historique reste nommé, seul le contrat du ticket est rapproché des deux témoins réussis. Pour `rows_outside_graph`, la conception doit compter les lignes à vecteur valide de l'instantané absentes du graphe utilisé ; un watermark ou un nombre de candidats n'est pas ce compte exact. Inconnu vaut NULL, jamais zéro. Pas encore de modification de production.
- Lucivy : corpus Linux 7.2 non retrouvé, aucune substitution silencieuse du corpus 2.6.0 ; l'ulp reste bloquée sur la couverture complète. L'entrée `indexed:false` avance par une API de diagnostic pure, témoin publié mais non joué ; aucune fusion ni publication.

### Exception de capacité pour C1

Pour ne pas bloquer le diagnostic du moteur pendant la compilation de G, une seule validation C++ privée est admise après publication du lanceur exact : nouvelle borne de 1 800 s, un poste lourd, 32 Gio, `-j8` au plus, target privé borné, aucune bibliothèque du magasin ni service partagé. Le témoin direct est publié sous `cx-coeur-c1-dictionary-diagnostic`, compléments `2f88b3df36591b3b8a4275018c7cddec6828a8d1` ; il ne porte aucun correctif de production et reste non joué à ce relevé.

Le relevé de capacité doit rester frais : TSan, G et C1 occupent au plus trois des quatre places ; G garde la priorité. Cette exception n'ouvre ni les passes Rust, ni les modèles/GPU, ni les bases réelles. Un premier rouge se conserve et se diagnostique avant la suite. La bibliothèque autonome de ce test C++ n'est pas la bibliothèque posée.

## Préavis et coupure des hôtes

G déclare les trois suites de déploiement vertes à la source gelée 927, sous 59ac sans moteur ancien : code 36/36, lectures 27/27 et démon 24/24, zéro ignoré. Les essais isolés et contrôles des manifestes aux deux postes sont également verts. Cela valide la préparation, pas encore la pose ni les migrations.

Le préavis d'au moins une minute a été relayé aux douze autres porteurs : aucun appel au 7970 ni nouvelle passe pendant la fenêtre ; 7878 et 7879 restent inchangés. G a engagé la coupure à 11 h 27 CET : sockets et hôtes des deux postes inactifs, MainPID zéro ; arrêt propre du portable en environ vingt secondes. Les instantanés et le retour 72 heures sont en préparation, avant les deux sens sur copies et la pose. Un écart de nom de poste (`LuciePC` contre `luciepc`) a refusé la capture d'identité avant copie ; G conserve ce rouge et paramètre explicitement le rôle, sans attribution à une corruption de base.

L'outil RO d'arbre, corrigé après le `RETURN` manquant, a été rejugé par G sur une fixture : identité moteur 59ac, compte exact, écriture refusée et octets inchangés. Il ne remplace pas les contrôles de migration. Les trois copies de sources du banc restent des candidates tant que l'admission du scanner (.ignore, liens, taille) n'est pas recoupée ; leurs déclarations privées ont `database=null` et ne sont pas exécutables en l'état. Aucun accès d'arbre aux bases vivantes avant le FINI explicite de G.

## FINI de G à 11 h 32 CET

G confirme la pose aux deux postes : source 927, hôte SHA256 `0e310a9cfe1c9e12534f3a579b3e781dcef7e9335e37d7e57349b16d053d5288`, lock `090cc4e1330dc694127c0f08330d25acc656699763079400b502ba6607093fa7`, lucivy 4.3.9. `courant`, la bibliothèque et vector sont sur `rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF`. Les maps des processus confirment lib et vector ; les cinq ouvertures du portable et deux du PC sont acceptées et confirmées par `/etat`.

Les contrôles sur copies couvrent dix-huit tables par base, unions 1 148 759 et 9 054 au portable, 1 205 199 et 931 405 au PC, zéro perte. Les instantanés et `RETOUR` sont sous `~/.local/share/rag3weaver/instantanes/avant-59ac9c86b-20261116-927`, gardés 72 heures aux deux postes ; preuves sous `~/.cache/optimiseur-notes/bascule-16nov-927/`. Ce FINI lève la dépendance d'arbre pour les trois migrations, pas leurs propres contrôles ni l'oracle. G reste propriétaire des services.

Les passes générales restent retenues jusqu'au lot prioritaire qui épingle `passe_epinglee.sh` sur 59ac, puis à sa copie et son témoin aux deux postes. L'oracle peut préparer son outil autonome RO depuis une rlib 927 attestée, avec un seul rustc privé borné à 120 s après poste ; aucun Catalog, ingestion ou base vivante. Le listing doit conserver le contexte d'admission git original : une copie sans `.git` ne reproduit pas seule ses exclusions.

Les relectures croisées sont favorables dans leurs limites : ED pour le candidat archi (S1/S2/S3 restent distincts), archi pour les ex æquo ED (pas de preuve de chaque chemin par le seul juge), mémoire pour la préparation des alias tty (témoins non joués, implémentation encore à faire). Aucun avis ne remplace une nouvelle passe sous la bibliothèque posée.

### Rouge de préparation C1

Le bâti autonome C1 a fini, mais son lanceur a refusé avant le test : il attendait le commit de test b82 dans le BUILD_ID, alors que le générateur emploie le dernier commit moteur 59ac et exclut les tests. Zéro juge joué, aucun correctif du dictionnaire qualifié. Le scratch a disparu avant copie des artefacts, limite conservée avec les journaux. Cœur publie le diagnostic puis corrige la recette (source et moteur distincts, copies hors scratch avant contrôle) ; une étape corrective distincte bornée à 1 800 s est admise aux mêmes frontières CPU privées, après publication de l'identité exacte. Aucun ancien reçu ni rouge n'est converti en vert.

## Entrées après la pose

- Arbre a publié la recette exacte `be91b5b3c742209cc59c2ce2b2ac29f9a97b9213` et lancé la maintenance du portable, client privé sous poste 32 Gio/1 800 s, API de l'hôte, relevé frais puis même jeton pour purge au plus 1 % et compact. Cela n'atteste pas le cgroup de l'hôte producteur, qui reste à G. Aucun résultat de maintenance ni migration n'est encore déclaré ici. Fermeture, instantané à froid et migration suivent seulement cette phase verte.
- L'outil RO portable a refusé sur le CPU du PC dans une fixture, pas sur une base vivante. G a recompilé et rejugé la même source nativement PC : `read-only-query-v2-luciepc`, SHA256 `48ae2328a29676fa69154cb9c67d9125d4d04cf92b029bd2b2a09981dea95616`, identité 59ac, compte exact, écriture refusée, octets inchangés. La recette d'arbre épingle chaque exécutable au bon poste ; les hôtes posés ne sont pas concernés.
- G qualifie son candidat de pin `68590270e21d00285b29800bd9cdecf0cb921461` par le script candidat, `SUITES=-`, dossier neuf `pin-59ac-20261116-927`. Témoin ciblé et deux filets verts ; huit jeux de features verts et preuve de recompilation acquise. La lib entière et le navigateur restent nécessaires avant le tour de push et la pose des copies du cache. Aucun tour de code n'est encore ouvert.

### Oracle : admission exacte, pas encore corpus comparable

Le premier lien du petit outil a refusé (E0514) : rustc 1.98.0 choisi hors checkout ne correspondait pas à la rlib 927, bâtie avec 1.98.1. Rouge conservé, aucune base ouverte. G a fourni le compilateur absolu déjà installé, SHA256 `859254978c0a0402c32f949f6de0d99aee73be8d15f45aac00ae1448aac51e74` ; la recette corrective publiée a lié la rlib attestée sans Cargo ni nouveau rebâti de produit.

Les trois listings du scanner 927 sur les racines originales sont maintenant jugés ; l'exécutable a aussi passé une fixture ISA avant le portable. Sources, copies, HEAD et statuts sont restés identiques avant/après. L'exécutable de listing ne lie pas la bibliothèque DB et n'ouvre ni Catalog ni base. Des différences restent à qualifier : fichiers de plus de 1 Mio absents des captures, métadonnées `.git` volontairement absentes, `.env.example` non lu selon la règle des secrets, un compose ignoré au PC lourd et quatre archives de preuves au portable. L'admission finale du manifeste et ses filtres de contenu ne sont pas déduits du seul listing. Les captures restent NONMATCHED ; aucun fichier secret n'est lu pour forcer l'oracle.

### C1 : premier rouge physique

La recette corrigée `4e3773250d639f2d7b396e7d4ff8611912f282ba` a joué un test sur source b82, BUILD_ID moteur 59ac réellement émis : une fonction, un échec, zéro skip. Un refus d'allocation de 48 octets des offsets laisse les trois offsets, mais la dernière chaîne devient `gammadelta` au lieu de `gamma` et le compteur d'octets passe de 14 à 19. Le défaut de préservation après cette exception est prouvé ; la causalité du SIGSEGV initial ne l'est pas. Les recherches de hash après refus n'ont pas été jouées.

Preuves publiées `cd7731186b2bb7044df8441fe7a3e67bf3863f99`, 29 fichiers, artefacts sauvegardés avant assertion puis rehashés aux deux postes ; unité finie et place rendue. Cœur prépare un correctif séparé qui réserve les capacités avant publication, sans reprendre le correctif 256 ni revendiquer toutes les exceptions possibles. Le parent C1, l'allocation de l'index de hash et la garde `sanityCheck` tautologique restent à suivre dans des tickets adaptés ; aucune nouvelle bibliothèque posée ni intégration n'est déduite.

## Pin intégré et reprise des qualifications

G a intégré `68590270e21d00285b29800bd9cdecf0cb921461` après le tour exact, la garde et les trois contrôles lus aux deux postes. Vraie passe du script candidat : témoin un juge et deux filets, F8 8/8, preuve de recompilation, lib 2 594 réussites/0 échec/72 ignorés, puis navigateur 45/0/0. Parmi les 72 ignorés de la lib, ces 45 navigateurs sont rejugés séparément ; les 27 autres restent NON JUGÉS et leurs raisons sont inventoriées. Aucun vert ne leur est attribué.

Les copies du cache des deux postes correspondent à la source, SHA256 `2150fcd847b1df7de04908d357a0c78abc05e50ce599e2793ff08ab48108825d`, mode 755, témoins contre copies verts et anciennes copies conservées. Le lot documentaire de clôture `f83c6c4dd6a40200883c6325326f093ecdba3e23` ferme le ticket du pin, pas celui de l'ancienne route status404. `lib_seule.sh` est archivé par renommage réversible après contrôle d'absence d'acteurs ; `run_e2e.sh --lib` est l'entrée actuelle.

Annonce aux treize : reposer les WIP sur des branches neuves base685, sans moteur ancien pour ces sources Rust et ce moteur59ac, puis qualifier sous garde/budget frais. Priorité à deux places Rust, recherche (déblocage FTS) et archi (Algareth), sans flood des services ; ED puis mémoire, embarquement et codeparsers suivent selon libération réelle des places. Le portable reste utilisable pour les inspections d'arbre. TSan continue, sans attribution au correctif ciblé.

Archi a conservé un refus de SHA mal recopié avant toute admission, puis reçu le maintien du GO sur `713433bd602a6d35266fe5cd528925ba664924c9` (deux MD seuls, script/produit inchangés), avec lecture de HEAD par Git. Sa vraie passe 59ac a commencé ; aucun résultat n'est encore déclaré ici. Recherche a publié le témoin positif d'ouverture d'un index fermé avant implémentation, admission rouge seul 1 200 s. Une réserve statique corrige le négatif avant la suite : une table physique source reste maintenable malgré sa redirection de recherche publique ; il faut refuser la redirection, pas la table physique légitime.

### Maintenance arrêtée sur 422 et diagnostic séparé

Le premier dry-run File du portable a reçu HTTP422 : aucune purge, compaction ni migration n'a suivi. Le client jetait le corps de l'erreur ; le premier corps est irrécupérable et le journal existant ne le contient pas. Diagnostic consigné avant correction, puis archivage statut/corps ajouté et jugé hermétiquement. G n'a pas fermé le backend pour une prétendue maintenance verte.

Une sonde DISTINCTE, `/backend/fts-live-ids` seule, a ensuite établi à son propre instant : « pas d'index plein texte ouvert pour File (ouverts : []) ». Cela ne prouve ni l'absence sur disque ni le corps du premier 422. La map courante et les poignées garées par cellule restent à distinguer. Le warmup par `open_fts_files`/`ensure_fts_handle` est exclu : chez l'écrivain actuel il peut jeter un dossier de mauvaise génération, marquer un rebâti puis créer après un échec d'ouverture.

Arbitrage : lecture sur copie privée qualifiée d'abord ; recherche prépare une primitive réutilisable « existant seulement », sans création, rebâti ou retrait implicite, avec refus strict absent/corrompu/génération incohérente/cellule étrangère. La maintenance et les trois migrations restent STOP tant que la voie n'est pas qualifiée. Les tickets d'arbre et le fils recherche portent le blocage, les preuves et la suite ; aucune nouvelle permission de Lucie n'est nécessaire pour la préparation et les diagnostics autorisés.

### Copies froides et fichier de coordination

Le premier préflight natif a refusé avant copie : la fixture corrompue est refusée par le moteur readOnly, mais un fichier vide `<base>.readers` apparaît. Rouge conservé ; le hash de la DB seule ne prouvait pas la neutralité des voisins. Le code `ReadersLock` explique ce fichier de coordination, distinct des données.

C7 a autorisé explicitement ce seul chemin latéral sur COPIE PRIVÉE/fixture, régulier, UID1000, vide et sans symlink ; tous les autres fichiers DB/WAL/FTS restent stricts et le snapshot SOURCE n'a aucune exception. Cinq témoins hermétiques jugent le validateur avant l'étape native corrective. Aucun passage en écriture ni création d'index n'est autorisé par cette exception.

Inventaire Python puis lecture native readOnly de la copie : 3 007 fichiers d'ensemble, dont 3 004 FTS, hashes source/copie inchangés avant/après. DB et FS concordent pour les générations File4526, Scope5502, Library142 et Symbol3573, quatre shards chacun. Cellule default/default seulement, comptes bruts File8 320 et Scope81 347, 33 tables, Scope_Chunk présent et code_scope_form absent. Les counts historiques ne valent pas comptes FTS actuels. Le lecteur immuable Lucivy4.3.9 est préparé séparément via export snapshot/open_snapshot, sans factory writer ; sa qualification reste à faire.

### Correctif ciblé du dictionnaire

Source033 a passé cinq natifs (offsets1/finalize2/split2, aucun saut), BUILD_ID033 réellement émis. Preuves `bf6102a2939ae6277afc9098cb5bf8dca4851d4d`, 37 fichiers et artefacts rehashés aux deux postes ; place rendue. Hotfixs confirme en RO la garantie restreinte aux refus BufferManager des réservations. L'exception de `unordered_set::insert`, les alias empruntés et le crash C1 restent hors preuve.

Une livraison propre base685, `15ab2f4815511400ac806314a3e964f2d7e723e0`, n'emporte que trois fichiers natifs identiques à033 et des MD ; nouveau moteur888 à qualifier, sans réétiqueter033. Les preuves volumineuses restent sur leurs branches. C7 accepte de préparer une passe moteur bornée sans les longs du comparateur courant, explicitement NON JUGÉS avec les probabilistes, pas une concurrence entièrement verte. Recette, artefacts fidèles et lanceur exact restent à publier avant admission ; aucune intégration ni nouvelle bibliothèque n'est encore autorisée.

Le suivi de la reprise est dans [le ticket](../tickets/2026-11-16-reprendre-les-perimetres-apres-lecture-des-diffs-recents.md).
