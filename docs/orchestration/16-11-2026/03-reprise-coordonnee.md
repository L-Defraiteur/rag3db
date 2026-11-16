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

## Bijections jugées et nouvel ordre opérationnel

Les six ensembles File/Scope des trois copies froides sont strictement égaux aux offsets du graphe : portable 8 320/81 347, PC principal 6 313/65 913, PC lourd 7 275/82 848. Erreurs, types, doublons et sorties tronquées sont refusés, pas seulement les écarts de cardinalité. Banc a relu indépendamment les 104 fichiers de preuves, leurs hashes, les raw IDs/rows et les douze générations : concordance dans ce périmètre, sans réouvrir une DB ni réempreinter lui-même les bases. Cela reste une preuve sur les copies froides, pas sur l'état vivant.

Plan court `6dce5695e1`, puis adaptation OPS **`360bcd75db`**, publiés avant action : snapshot FRAIS, nouvelle bijection/générations et 18 tables dans les deux sens. Si ces contrôles frais sont verts et la base demeure froide inchangée, **purge non nécessaire, compact reporté**, sans fabriquer un succès de `/maintain`. Les commandes officielles `migration versions` puis `migration generations` peuvent ensuite partir sous nouvelle portée de 3 600 s/base, caps 1 800/180 min solde−30, pool1G/4threads et pins927/59ac. Les retours restent gardés72h, aucun ancien index retiré.

### Adaptation à l'état frais du PC

Avant toute fermeture ciblée, G a refusé son préflight : le `/etat` frais indiquait un nouveau PID de l'hôte (binaire927/59ac conforme), mais zéro backend et zéro alias. Les ouvertures de11h32 étaient donc périmées ; cause du redémarrage non attribuée. Aucun close/open/CLI n'a été effectué pour contourner ce constat.

C7 autorise explicitement la nouvelle fenêtre : nouveau relevé zéro backend/alias, identités conformes et absence de détenteurs des bases, préavis d'au moins60s, puis G arrête **seulement le service et le socket API7970 du PC**. Les deux bases PC restent froides, principal puis lourd, sans réveil intermédiaire ; snapshots et contrôles frais puis CLI conditionnelles. Le portable et les services7878/7879 restent disponibles. G seul réveille après les deux étapes ou coordonne le retour après rouge, avec conservation préalable de l'état refusé. Ce changement d'ordre ne prétend pas que la maintenance est passée et n'attend pas une nouvelle permission routinière de Lucie.

### Lots parallèles : les rouges restent rouges

- Recherche a enfin observé le rouge comportemental du seul témoin positif187 : index physique privé présent mais pas de handle ouvert, une fonction en échec au point voulu, aucun autre juge lancé. L'arrêt préalable était celui d'un lanceur à scope systemd redondante, corrigé en réemployant uniquement poste+confineur commun avec contrôle frais32G/HIGHmax/swap0. Source correctif **`1ff6c23d9f298abbe100db34b801140a270756cd`** publiée, neuf juges écrits, revue mémoire statique favorable, encore non compilée/non jouée. Périmètre : snapshot immuable pour relevé/lecture ; purge, jeton et handles déjà montés inchangés. Pas encore une maintenance d'écriture disponible sur un index fermé.
- Archi : témoin continuation1/0/0, hôte24/0/0 et lectures27/0/0 acquis ; **premier rouge** dans `e2e_equipier`, la documentation par pas jugés n'aboutit pas sous son délai de15min. L'autre cas Vertex texte réussit. Hooks/code/F8/liblarge/NAV non joués après ce rouge ; aucun master. Fixture, juges, événements et quiescence sont conservés avant diagnostic ; pas d'attribution au modèle ou à la correction sans preuves.
- ED : arrêt de préparation avant Cargo, aucun juge : sous XDG_CACHE_HOME privé, le magasin n'était plus trouvé faute de RAG3DB_LIBS_DIR explicite. Ticket avant correction, vrai résolveur vu refuser puis réussir avec quatre variables exportées exactes et hashes ; aucun chargement natif déduit. Correctif pilote **`dae3a62ee6d3ff09500e108afb86a28bcd017a20`**, APPbd76 inchangée, revue archi favorable et admission corrective sur le RESTE de la fenêtre initiale, sans remise à zéro7200. La leçon du magasin explicite est transmise avant les prochains lancements des autres sessions.
- Cœur : lanceur global **`f3190da3d47559822bfae238e9400b9621ed4eeb`** et parseur publiés, dix fixturesCPU, aucune globale native jouée. Portée proposée sans29longs/17probabilistes, intersection1 donc45 fonctions distinctes NON JUGÉES ; la fixture d'ID falsifié reste fidèle et visible dans les raw, l'attestation vient du stdout natif. Revue hotfixs du flux/quota/parseur demandée ; admission moteur attend aussi une capacité qui ne retarde pas la fenêtre froide d'arbre.

Les scans et outils autonomes du banc ne dépendent plus des deps de G ; les sondes d'arbre ont terminé leurs liens et conservent leurs ELF. G a qualifié une archive release de4 806 entrées, source/archives identiques avant/après ; aucune promesse de rebuild relocalisable. Nettoyage propre G autorisé seulement après fin de cette dépendance et garde fraîche, mais suspendu pour donner priorité à la fenêtre des bases. Bibliothèques posées, hôtes, modèles, retours et preuves restent conservés.

## Copie officielle refusée, API PC rendue

COPIE02 a joué les commandes officielles927/59ac : versions rc0 en464,043s,65 913 occurrences,61 223 versions,97 247 morceaux,96 778 vecteurs repris et469 en dette pour granite_278m ; generations rc0 en0,431s. Estampille, schéma dynamique, comptes File/Scope bruts et visibles, liens de contenu et miroirs avec multiplicité0/0 passent. Le contrôle dette0 refuse469 : **validation-qh1_nf6p ROUGE1/restes[]/nettoyé**, aucun vert composé. Inventaires du snapshot G et du canonique strictement inchangés après refus ; aucune CLI jouée sur un original.

Arbre poursuit uniquement le diagnostic RO sur copies : pour chacun des469, ancien candidat admissible absent, présent chez le représentant sélectionné ou seulement chez une autre occurrence, selon champ/rang/empreinte/titre/vecteur/marqueur exacts. Aucune cause attribuée au nombre seul, aucun réembarquement ni seuil élargi pour blanchir le rouge. Ticket fils demandé par Lucie **publié avant tout diff correctif**, af49651ab4 : `docs/arbre-principal/tickets/2026-11-16-la-migration-de-scopes-laisse-469-morceaux-sans-vecteur-transfere.md`, parent et preuves reliés, cause NON ATTRIBUÉE et zéro correction Rust.

Fenêtre LIVE abandonnée : G a rétabli uniquement API7970 PC, FINI13h36/santé13h37, PID511010/start117847552, même ELF0e310/source927/bibliothèque59ac, baseline zéro backend. Vecteur vérifié sur disque, non mappé sans backend, aucune ouverture forcée. Avant reprise,733 identités/SHA canoniques inchangés, aucun writer CLI/provider. Reçu `pc-cold-window-20261116-122721/window-closed.json`, SHA256 d9396ba55a2e538770d75f36f8dd45308c2761e5ee53107298781e0fc61fd814. Aucun retour de données nécessaire ou exécuté. Toutes anciennes gates LIVE sont historiques, variant provider LIVE non activé ; futur essai requiert nouvelle fenêtre et snapshot après diagnostic. Fin de l'interdiction générale7970 relayée aux treize porteurs, sans droit de writer/warmup/index sur les bases de Lucie.

Le garde systemctl dans le bus privé avait un défaut de recette établi par sonde distincte : manager utilisateur absent, pas preuve d'API active. Contrat260d63d93f/provider18ff0e14 corrigent la recette hors bus, sans réinjection du bus réel ; même ticket bascule, source451f4b09e5. Retour_v2 est qualifié pour ce snapshot seulement ; ticket `docs/optimiseur/tickets/2026-11-16-le-retour-d-un-snapshot-oublie-les-voisins-crees-apres.md` toujours ouvert, générateur non corrigé.

## Ordre stable livré, attentes levées et blocages conservés

ED a rejoué sa canonique v3 réelle : F8 8/8, lib2595/0/72 ignorés, NAV45/0/0, code36/lectures27 ; deux fonctions ciblées distinctes et trois invocations avant lib.72 ignorés ne deviennent pas72 verts. Preuves574134fb1e/29 fichiers/manifeste190f5dda…10fb9e, relectures Archi et Mémoire favorables, anciens rouges pilotes conservés. C7 a recalculé6915 entrées hors docs identiques à l'APPbd76,8952 entrées/absences master protégées, quatre entrants G préservés,14range-diff égaux.

Tour unique exact donné puis **d395cb1f03f84c6abdfa023a0e3c7f87a7c0dab6 intégré**, vrai postePush/fetchf83/garde/verdict canonique réel, trois contrôles lus aux deux postes : propres, aucun rebase, HEAD=origin/master=d395. Tour fermé. Aucune pose de nouvel hôte : opérationnel927/59ac reste figé. Nettoyage des seules ressources ED autorisé avec archives fidèles deux postes, garde6f3 courante/rapports frais verts/refus conservés ; réalisation à confirmer.

ED signale ensuite un défaut réel de la garde pile95a : les en-têtes range-diff sont indentés, case colonne0 en juge zéro puis accepte. Les audits indépendants du produit qualifié restent valides ; aucune validation du parseur n'est revendiquée. Ticket publié AVANT transmission241c53a5dd : `docs/everything-declarative/tickets/2026-11-16-la-garde-pile-ne-juge-aucun-entete-d-un-range-diff-court.md`. G est attribué au rouge CPU du vrai parseur, correction/rejets stricts et revue avant copie cache. Prochains tours pile retenus d'ici là ; validations privées continuent, garde de nettoyage6f3 distincte inchangée.

Relevé capacité : portable0/1, PC0/4 et57GiB disponibles sur93 ; TSan fini (six codes66/111 avertissements, pas un vert, anomalie catalogue ticketée). La fenêtre froide ne retient plus tous les travaux privés.

- Cœur : Hotfixs0de favorable, trois réserves superviseur levées dans son périmètre ; C7 vérifie quatre hashes scripts et trois natifs888 identiques à033. GO10800 neuf source/moteur888/livraison15ab/pilote0de. **Entrée réelle** : unité cx-coeur-dictionary-global-01 active PID673654, parentposte cpu.max500000/100000, j5,32G/swap0, départ1179063.5567. Tx/strings fail_fast, known_red premier verdict de phase complet, e2e positifs fail_fast puis DROP exact sous même budget.45 exclus/2 helpers/1 mesure disabled NON JUGÉS ; C1 crash ouvert.
- TTY : GO rouge seul4cc0b98 portable600 Common32G/j4/Tinitial absent prouvé. **Rouge réellement observé** : deux fonctions0/2/0, /help inconnue et /st sans complétion, compilation2,83s ; validation-g4dqpr6q ROUGE1/restes[]/nettoyé. Sources/rouge archivés, correctif et autre rouge CLI en préparation sur union neuve, aucun vert déduit.
- Mémoire : source734087a4/pilote1e3a publiés surd395, **HOLD avant run** : ED et C7 trouvent la réintroduction de P.rglob(log) avec cache embarqué sousP, même mécanisme que le rouge historique TIES. Réserve ticket existant3d2d6c avant correction ; liste explicite des logs producteurs et témoins du vrai chemin requis, sans Cargo admis.
- Embarquement : couple5e88932d01/profil qualifypyb04a9d2f et recette5400 publiés surd395, pas encore de GO. Neuf ciblés privés puis pause sous même deadline avant la canonique complète, créneau BGE/PG à coordonner au point réel. Appels modèles/PG canoniques permis sans modification ni redémarrage ; pas d'interdiction artificielle des services nécessaires.
- Recherche : cible046/d218 verte,10 distincts/11 invocations,30 fichiers47a8fde4c0, aucun reste. Lecture froide seulement, écriture de maintenance inchangée. Uniond395 puis canonique code_sync/lectures/code/TX/F8/lib/NAV proposées6000, non admises.
- Archi : classificationcbcac46 des13 UNKNOWN, dix UID1000 présents avec exe/cwd/FD opaques et trois disparus ; aucune absence de détenteur prouvée. HOLD natif2397, pas1200 condamné ni exception par nom. Proposition RO6b307b27 de sous-cache initial neuf dans la même racine session, ancien cache gelé et plafond cumulé32GiB, à arbitrer avant implémentation ; aucun Cargo ni ancien délai réactivé.
- Codeparsers/lucivy : préparation des rouges/pilotes déjà autorisés reprise ; corpusLinux7.2 absent et refus PR42 respectés. Banc attend uniquement métadonnées de copie identifiée pour borner l'oracle ; absence dans les logs ne vaut pas compte0, NO_MATCHED conservé.

Le suivi de la reprise est dans [le ticket](../tickets/2026-11-16-reprendre-les-perimetres-apres-lecture-des-diffs-recents.md).
