# La migration de scopes laisse 469 morceaux sans vecteur transféré

- **État** : ouvert (16 novembre2026), cause attribuée au redécoupage479a par rejeuRO exact ae5d3ac4ff ; dette469 non acquittée, décision de suite attendue, aucun correctif produit écrit.
- **Famille** : produit
- **Gravité** : P1 de qualification avant bascule ; dette réelle sur copie, aucune base vivante modifiée.
- **Atteignable en service** : par les commandes officielles migration versions/generations ; témoin sur copie privée complète, pas d'ouverture du vivant.
- **Touche rag3weaver** : oui pour le diagnostic de migration/transfert, correctif éventuel à décider après attribution.
- **Pour** : arbre principal (diagnostic et contre-exemple), orchestration (arbitrage de portée), embarquement/banc si réutilisation ou recalcul à qualifier.
- **Signalé par** : répétition COPY02 le16novembre, rappel explicite de Lucie via C7 pour tracer le correctif séparément du parent.

Parent : `docs/arbre-principal/tickets/2026-11-16-basculer-les-trois-bases-vers-les-generations.md`. Plan et chronologie : `docs/arbre-principal/16-11-2026/04-plan-court-bascule-sans-purge.md`.

## Rouge conservé et périmètre réel

Sourcehôte/CLI927941a67a5805450d49f1e04b39fdd542a6059b ; ELFofficiel0e310a9cfe1c9e12534f3a579b3e781dcef7e9335e37d7e57349b16d053d5288 ; moteur59ac/libSHAb7e9fc75a3d2de04c72680b0309e8bb1df9eaadb5211ebbacf39716c6d0a2101, vector1669d6392ec8067fcf92ae2ccc972fd7cd3086629fb82d1f5d254e90d72d7561, Lucivy4.3.9. Profil pool1GiB/4threads/magasinréel/LD59ac, embedder Refus (aucun calcul).

SurcopieJETABLEsupplémentaire du snapshotfraiscode-principal, `migration versions --base` rc0 en464.043166s :65913occurrences,61223versions,97247morceaux ;96778vecteursrepris et469morceaux_en_dettegranite_278m explicitementdansBilan. `migration generations --base` rc0 en0.430714s, backend0/readrank1. Runtimeexe/lib/vector observés auxdeuxPIDs. Lecontrôleindépendant SQLdebt retrouve469 : copieglobaleROUGE, lesrc0 ne valentpas qualification complète.

ScopePC `~/.cache/rag3weaver-build/bascule-live-20261116/migration-service-corrective-02-scopes/validation-qh1_nf6p`, code1/restes[]/nettoyé. Preuves `~/.cache/rag3weaver-build/bascule-live-20261116/migration-rehearsal-service-corrective-02/code-principal/` : versions.stdout/process/runtime, generations.stdout/process/runtime, models.stdout, debt-granite_278m.stdout, vector-debt.json, verdict.json, failure-source-audit.json. Ce dernier confirme snapshotG ETcanonicalvivant inchangés. Garde horsbus/nonce/OS réelle passée, pas d'attribution au garde de services.

Modèle persisté : granite-278m/768/storage **suffixed**, colonnes embedding__granite_278m et _embed_hash__granite_278m ; models.stdout SHAce61440b3a7ad99c1807fc0044f1697703f37a981266812ce85b381c285ca9f0. Ne pas sonder les colonnes legacy par erreur.

La fenêtrevivante est ANNULÉE ; API/socketPC repris parG(source927/59ac,0backendbaseline), aucuneCLI suroriginal ni restaurationdata. AnciennesgatesLIVEhistoriques, aucunGOcourant, aucunrerunwriter/indexouembedding autoriséparceticket. Copies, étatsaprès etretours72h conservés.

## Diagnostic avant toute correction

SurcopiesAVANTfresh-sets-01/code-principal/copy etAPRÈScorrective02/post-copy, sans writer, HTTP, Cargo ni recalcul : mesurerancienne dette exacte du même modèle ; établirles469IDs nouveaux uniques etTOUTESoccurrencesH. RejouerprédicatEXACTde transferer_les_vecteurs (migration_des_versions.rs) : représentantpluspetitUUID, parentfield/index/hash/title normaliséscommeRust, ancienvecteurnonNULL etmarker=_text_hash ; contrôler aussitexte littéral sansl'exporter.

Matrice469/0doublon/0inconnu : (1) aucuncandidatcompatible (detteavant,nouveauchamp/rang/hash/découpage/titre) ; (2) compatiblechezreprésentantchoisi (omissionpotentielleàattribuer) ; (3) compatibleseulementchezautreoccurrence (choixreprésentant). La sourceutiliseunreprésentantpourborner les lectures : piste, pascausedéjàprouvée. Aucuncontenuvectoriel nisourcecodeenclair dansrapport.

Avantcorrectif : contre-exemple minimalreproductible vuROUGE, ticketaffinéàcauseétablie, revue/recettebornée. Aprèscorrectif : juge cibléVERTpuis qualification surcopie au coupleexact ; pasabaissementdugarde0 nimodificationdesrougeshistoriques. Si detteattendueparcontrat, documenterarbitrageexplicite etpreuve derepassage avantdeconclure ; nepasinventerunfixproduit. Toutefuturebascule réclame nouvellesprérequis/snapshot/autorisationC7.

PréparationROcomplète : diagnose-vector-debt.py+vector-debt-matrix.py,6CPUfixturesvertes ; sourceplan04recette600/32G etmodèlepersistésuffixedSHAce61440b. MatricevisiteTOUTESoccurrencesH etnormaliseexactementcommeRustchamp/rang/hash/titre ; textelittéralauditéenplusparégalitéSQL, jamaisexporté. Aucuncorrectifproduit, aucunerequêtesurcolonneslegacy erronées exécutée. Classificationnonencorejouéeàcettepublication.

## Premier résultat RO : choix du représentant écarté sur la cohorte

Sourcematrice2c217b656e, scopevalidation-rv44n4sv VERTDIAGNOSTICSEUL5.003739s/600, jamaismigrationverte. AVANT :102002ancienschunks, dettevectorielle0 ; APRÈS :ces102002ancienschunks encorelà/dette0,469nouveauxchunksendette. Matrice469/0doublon/0inconnu :469 NO_COMPATIBLE_CANDIDATE ;0chezreprésentant,0chezautreoccurrence. Chaquechunkdelacohorten'aqu'UNEoccurrenceH : lechoixdu représentant n'expliquepas ces469.

Les469ontunancienchampcompatible ;320ontunrangancienmaisaucunhashcompatible,149n'ontpasderangancien. Aucunanciencandidatpassechamp/rang/hash/titre, doncaucunvecteurcompatible exact ignoré. L'audittextelittéralparmilescandidatsvalides estvide (0mismatchvacuous, PASpreuve que textesanciens/nouveauxégaux). SHAensemblescopiesavant/aprèsstricts, aucunwriter/HTTP/recalc. Cause profonde du changement de texte/découpage restantàattribuer ; aucunfixproduit écrit. ConfigurationspersistéesScopeavant/ScopeContentaprès etdettesdécoupageparentàcomparer sansplaintext.

PreuvesPC vector-debt-diagnostic-01/proof-manifest.json : 55 5447d16abcb0bf27b117522fca27354ae30dcaf6086cc1552055f3002236a28c.

## Suite RO : contenu parent, géométrie et métadonnées Banc

Piste à éprouver, pas attribution : le correctif479a46549e9d3eb04459bbfd05445e3750fbdfd6 du16novembre05:28 coupe les longues lignes et borne le contexte. Le ticket fermé `docs/tickets/closed/2026-11-16-le-decoupeur-laisse-passer-un-morceau-de-53-000-caracteres.md` précise que les anciens morceaux persistent jusqu'à réécriture ; suivi des anciens géants chez `docs/embarquement/tickets/2026-11-16-les-anciens-morceaux-geants-sont-a-reecrire.md`. Ne pas écrire un second correctif de découpage sans établir la causalité de cette cohorte.

`diagnose-debt-geometry.py` : diagnostic copies uniquement sous common32G/délai600 neuf, aucun writer/HTTP/Cargo/embedding. Compare l'égalité du contenu Scope/ScopeContent en SQL (compteurs seulement), sort UUID/rangs/bornes/hashes des469 et de leurs anciens morceaux ; aucun corps de texte ni vecteur exporté. Ajoute les métadonnées File.path/Scope.file_path pour `.env.example` sur les3copies et docker-compose.test.yml sur code-lourd, sans lire les fichiers ni leurs contenus stockés. Leurs flags `_absent_since` ne valent pas preuve de visibilité estampillée. Inventaires stricts avant/après, premier refus arrête les requêtes, nouvelle destination par poste.

Correction du résumé de profils : les configurations brutes emploient `sourceLines/isContent/isTitle`, pas les seuls noms snake_case utilisés dans le premier résumé. Témoin pur camelCase passé ; nouvelle synthèse générée depuis les sorties brutes immuables avec leur SHA, anciennes observations conservées. Aucun rerun natif de configuration nécessaire. Configurations de découpage numériquement identiques ; champs content/docstring de contenu et name de titre des deux côtés. SourceLines existe sur Scope ancien, pas ScopeContent ; ce constat ne prouve pas à lui seul un changement du texte découpé.

Les deux fichiers non suivis return-proof-gate.py/test-return-proof-gate.py restent WIP de l'ancienne préparation LIVE, conservés sans exécution ni activation ; le garde froid courant et le lanceur historique ne constituent pas un nouveau couple autorisé.

## Résultat RO contenu/géométrie : piste479a renforcée, rejeu exact restant

Source d5de903d30 ; scopes PCvalidation-n72f1k0k etportablevalidation-xm33y4w6 VERTdiagnosticseul32G/600,3.518530s et2.516747s, inventaires de toutes les copies strictement identiques avant/après. Pas de writer/HTTP/Cargo/embedding/service.

Comparaison SQL sans export de contenu : les65913liens ScopeContent→Scope ont un champ content identique ; les469morceaux en dette concernent51parents et les469comparaisons sont égales. Les configurations de découpage sont identiques. Les1008anciens morceaux de ces51parents comprennent42cœurs>1500octets et120plagescomplètes>4500 ; maximum53295octets. Les469nouveaux :0cœur>1500,0plage>4500,maximum4255. Sur51parents,18ont au moins un anciencœur>1500 et50un contexte ancien d'un côté>1500 ; l'union couvre les51, aucun parent sans ce dépassement. Les1008 incluent tous les anciens champs du parent : ce comptage n'est pas une bijection de morceaux ni une validation de leurs hashes.

Ces observations correspondent aux frontières précisément corrigées par479a (coupe des longues lignes et bornage du contexte). Elles écartent une différence du contenu parent et renforcent fortement la piste du redécoupage ; **aucun rejeu des deux algorithmes sur les51parents, aucune attribution complète des469hashes à479a, aucun fixproduit ni detteacquittée** à ce stade. Prochaine preuve concrète : rejouer le vrai module chunker avant/après479a, paramètres persistés, comparer seulement les tuples rang/bornes/hash aux morceaux stockés ; pas une réimplémentation Python. Préparer une recette standalone bornée, sans Cargo ni écritureDB, avant toute éventuelle compilation de l'outil.

Profils corrigés : profiles-corrected-from-immutable-raw.json rattache chaque synthèse au SHA de la sortie brute de profile-01 ; anciens résultats intacts.

Métadonnées pour Banc : File.path ET Scope.file_path ENDS WITH '/extension/llm/test/.env.example' rendent0ligne dans chacune des3copies avant ; même résultat pour '/extension/lucivy/ld-lucivy/docker-compose.test.yml' sur code-lourd. Sorties limitées UUID/path/repo_path/langue/taille ou offsets/flag absent. Résultat borné au prédicat de chemin absolu/suffixe, pas preuve de visibilité par génération ni lecture de fichier .env. Banc informé ; oracle global non qualifié.

Preuves sous ~/.cache/rag3weaver-build/bascule-live-20261116/ :
- PC vector-debt-geometry-pc-01/proof-manifest.json :42fichiers, SHAdd5157b51fac6d4ed1877585655039ac30ca41b4f46f3841a8192f37088c2568 ; copie locale vector-debt-geometry-pc-local-01,42tailles/SHA recalculés sans écart.
- Portable vector-debt-geometry-portable-01/proof-manifest.json :12fichiers, SHAb431988159eb5a306b9e90b874059ca3b3a9365ca0474400dcfbf50c097ba55a.
- queries.json, observations.json, geometry-audit.json(PC), sorties brutes, processus, inventaires avant/après et verdicts conservés. Rouge COPY02 inchangé, aucun GO de reprise vivante déduit.

## Recette de rejeu proposée, pas encore exécutée

`replay-chunker.rs` est un adaptateur CPU stdin→hashes/offsets. Ses seuls types locaux sont ChunkStrategy/ChunkingConfig ; l'algorithme n'est pas recopié/réimplémenté. Préparer horsW/T deux fichiers **octets git show** : chunker-before.rs depuis49644f064f9f9ece4c9c4bd8714eb86838afa4a0 (parent du fix) et chunker-after.rs depuis479a46549e9d3eb04459bbfd05445e3750fbdfd6 ; vérifier after identique au blobchunker de927. SHA de chacun dans ledger avant exécution.

Compilation proposée **rustc standalone seulement, pas Cargo**, outil1.98.1 SHA859254978c0a0402c32f949f6de0d99aee73be8d15f45aac00ae1448aac51e74 ; dépendances archivéesG en lecture seule `~/.cache/optimiseur-notes/bascule-16nov-927/release-archive/deps`, externes uniques libtext_splitter-a566ac746e53062b.rlib et libblake3-427059a5348cca07.rlib, hashes avant/après. `--edition=2021 -C codegen-units=1` vers destination privée neuve, aucun targetG/W/Glib modifié. Source+dependencies+ELFhash avant tests. Fenêtre proposée common32G600 après poste réel, compilation bornée90 et appels min(reste-15,60), aucun reset ; compilation non encore jouée, résultat inconnu.

Contrôleur à matérialiser avant lancement : (1) témoin synthétique ligneASCII53294octets, anciens/nouveaux différents comme479a ; (2) dans la copie migrée seulement, les51UUIDparents exacts de geometry-01, lire s.content en mémoire via helperRO59ac, une seule colonne/une seule ligne par requête ; parser en retirant UNIQUEMENT framing BEGIN_QUERY/ROW et suffixe final END_QUERY1, jamais splitlines/trim du contenu ; contrat FlatTuple::toString et Value::STRING relu, caractères/newlines conservés ; (3) stdin au replay, aucun fichier/log du contenu et aucune exception contenant le corps ; (4) pour chacun des51parents, comparer multisets ordonnés de tuples index/start/end/corestart/coreend/BLAKE3 aux anciens et nouveaux chunks `_parent_field=content`, avec cardinalités strictes ; (5) rendre les couvertures exactes des469UUID/hashes, premiers écarts ROUGE sans poursuite. Si tous les anciens tuples ne correspondent pas à before, ne pas attribuer intégralement à479a. Inventaires sources stricts avant/après, aucun service/HTTP/embedding/index/écritureDB.

Ce programme n'a pas été compilé/exécuté ; la proposition rustc doit être arbitrée séparément de l'autorisation RO metadata déjà consommée. La fenêtre LIVE reste annulée.

Contrôleur matérialisé : diagnose-debt-replay.py vérifie les42preuvesgeometry, paramètresconfig bruts persistés (1500/0/30/5/lines), sourcespréparées, rustc/helper/lib ; compilation standalone bornée, ledgerELF avant toute exécution, témoin synthétique, deux lectures de tuplescontent pour51parents puis51lectures de corps strictement en mémoire, comparaison Counter (multiplicités conservées), couverture469, inventairecopie+dépendances avant/après et deadline après hashes. Premier refus arrête la suite ; aucun fichier de corps, stdout de la lecturecorps jamais archivé. DeuxjugesCPU de framing passés (six corps dont UTF8/newlines/texte ressemblant au protocole ; quatre refus0/2lignes/troncature). Ces juges ne compilent ni n'exécutent Rust, ne touchent aucune base et ne qualifient pas le rejeu natif. Adaptateur+contrôleur encore NONCOMPILES/NONJOUÉS.

Revue ED statique655 favorable bornée, réserve de provenance préflight traitée : le contrôleur impose maintenant directement les SHA des deux vrais blobs Git, indépendamment du manifeste externe : before b2b4a0a916b81a25bab50f18b28bd9355e6094f7ee42a456f8dcabf33bb6e9dd ; after 1b35738216ab11ec7d0ba88f47b8724ef88eb48cfc858292790b79e39d114622, recoupés indépendamment par ED avec49644/479a et after==927. Le booléen du manifeste ne suffit donc plus. Un prebuild-ledger.json consigne sources, SHA du paquet, rustc et deux rlibs AVANT build ; ledgerELF reste avant runtime. Ancien paquet01 conservé, nouveau paquet02 horsW à matérialiser et réempreinter avant tout GO distinct. Deuxjugesframing toujours verts ; aucun rustc/DB/runtime exécuté pour cette correction. Avis ED reste statique ; matching éventuel ne paie pas la dette et n'autorise pas LIVE.


## Rejeu RO exact terminé : la cohorte469 est expliquée par479a

GOdistinctC7 pour ae5d3ac4ff/paquet02SHA46cf932225bd14f7cbf600d35fc34800cf4c26d7feddab1955964dcd9c9fe48d. Exécution PC seulement, aucun second hôte : common neuf600/32G, scopevalidation-qwbuqe__, autoritéPID1577711/start118239677/END1182996.897060039, mémoireeffective34359738368 ; autoritéterminée, superviseurVERTcode0/restes[]/nettoyé. Compiler rustc1.98.1 standalone rc0, aucun Cargo ni target deG touché. ELF ac861f7f6d5da390a8b171429656f121497dfc79ed2e60d5e5bb3451819a37f0, ledgeravantCPU etprebuild-ledger avantcompilation.

Verdict READ_ONLY_CHUNKER_REPLAY_MATCHED en7.283473716s : témoin synthétique53294octets =1ancienmorceau→36nouveaux ; les51parents réels donnent1008tuples anciens et1157neufs. Pour CHAQUE parent, Counter deTOUS les tuplesindex/start/end/corestart/coreend/BLAKE3 est exactement égal aux chunks persistés avant/après respectifs, multiplicité comprise. Couverture des469UUIDdedette complète, aucun inconnu. Lesmultisets bruts stockés ont688tuples communs,320anciensseuls,469nouveauxseuls :1157−1008=149, cohérent avec les149rangs neufs de la matrice initiale.

**Attribution bornée désormais établie pour cette cohorte** : les vrais algorithmes49644/479a rejoués sur les mêmes contenus rendent exactement les anciens/nouveaux morceaux. Les469clés incompatibles viennent du redécoupage du correctif479a ; aucun vecteur compatible exact ignoré ni défaut de choix du représentant n'a été trouvé. Il ne faut pas recopier un vecteur calculé sur un texte différent pour obtenir artificiellement0dette ; cela ne signifie pas que tous les469textes sont nouveaux au sens du contenu. Aucun nouveau correctif produit requis pour démontrer ce diagnostic.

Inventaire intégral de la copie post-copy strictement identique avant/après ; les deux rlibs archivéesG également inchangées. Les51corps n'ont existé qu'en mémoire, aucun fichier/log de texte ; seules longueurs/hashes/bornes sortent du programme. Dette469 toujours présente, aucun embedding/recalcul/HTTP/service/nativewriter niCLI migration rejoué. Le rouge COPY02 reste rouge et LIVE reste annulé. Le résultat ne qualifie ni migration complète ni acquittement des vecteurs.

PreuvesPC vector-debt-replay-01/proof-manifest.json :121fichiers SHA6d4667936fc876e4a415d5cd12698486f0d231b7f8ebd1cca57250b475868cc6 ; copie locale vector-debt-replay-pc-local-01,121tailles/SHA recalculés conformes. Source/configuration/ELF/dépendances, sortieshashes51parents, checks51, coverage469, sorties tuples old/new etinventaires conservés. Scope indépendant conservé sous vector-debt-replay-scopes/validation-qwbuqe__.

Suite : revue indépendante de cespreuves puis arbitrageC7/Embar/Banc sur le moyen d'acquitter et qualifier les469vecteurs dus sur copie, AVANT toute nouvellefenêtre vivante. Ce ticket resteOUVERT avec son parent ; il porte maintenant la dette après redécoupage et sa qualification restante. Aucun GO de recalcul ni de bascule n'est déduit du diagnostic.


## Relectures indépendantes et limite de la clé de transfert

ED etBanc concordent :121tailles/SHA,51Counter,1008/1157,688/320/469 ; UUID469 exactementmultisetnew-only,0inconnu ; modules Git etledgers conformes, inventairesJSONDB/deps identiques. ED a aussi relu scopepublicVERT0/restes[]/nettoyé/busabsent. Aucun runtime/DB/rustc joué par ces reviewers. RevueBanc SHA31585107444934bd0844eda4fc2327faf3316afb5b6e15c5403f09135c0d2454.

Nuance précise Banc : **74nouveauxseuls ont un ancien candidat de même(parent,hash), mais à un INDEX différent ;0a le même(parent,index,hash)**. Le prédicat produit exige(rep,parent_field,index,hash,title) : absence de clé exacte confirmée, pas absence absolue de contenu déjà embarqué. Une réutilisation sûre par contenu peut donc être étudiée séparément ; elle demanderait encore validation du modèle, champ, texte/hash/titre et invariants du transfert, sans relâcher aveuglément le rang. À ce stade ni74réutilisations acquises ni469recalculs nécessaires démontrés. Les469dettes restent présentes ; C7 arbitre la suite surcopie, aucune autorisation d'éditionproduit/recalcul/bascule déduite.

## Préparation suivante demandée parC7 : candidats74 et deux options

Source017254dc8d, [plan07](../16-11-2026/07-suite-dette-469-options-sur-copie.md). Analyse pure desTSV/proofs121+42+55 entièrement réempreintés, aucunebaseouverte :74candidats de mêmeparent/champcontent/hash, UNancien chacun, tousmêmesbornescomplètes+cœur etrangdifférent ;0ambiguïté dans ce périmètre.395sanscandidat mêmeparent/hash, pas de rechercheglobale. Modèlegranite278m/dimdéclarée768/storage suffixed/ancienvecteurnonNULL+markerhash0dette prouvés globalement ; titres/texte littéralpersisté/dimensionréelle/composantes/conflitsvectoriels nonprésentsdansTSV. Textes reconstruits sur mêmesplages identiques, mais pas de comparaisonindépendante des `_text` stockés. **74potentiels/0réutilisationnouvellementqualifiée**.

Preuve locale vector-debt-moved-candidates-01/proof-manifest.json3fichiers SHA7efd0727a81c5c2b0d1410d9de2a83079463094564992f422b59d3f09e88d21f. OptionApréférée pour préparation : voie existante Catalogretard/EmbedNodeCompute et porte déclarativeSearchSourceNode exigenamedense, CLIstandaloneJSON, textesDBpersistés, pas catch_up_on_open niIndexNoderescan. Graphe+manifesteCOPY/pilote exact restent àmatérialiser/relire AVANTGO ; nepasannoncervoie déjàqualifiée. OptionB : fallbackindépendantrangseulementaprèscontratidentitéentrée/modèle etvraiROUGE, bornemémoire/3lecturesreprésentant préservées, aucunfixparintuition.

Plan07 décrit une nouvelleétapeCOPYDEBT_PENDING→DEBT_ZERO_OBSERVED→QUALIFIÉE (statutsdepreuve seulement), propositionfenêtre1800aprèsposte/32G àarbitrer, préflightmodèle/service/copie/code/géométrie puisshutdown+SQLindépendantdebt0/dims+FTS/graph/gen/provenance. Aucune destinationcréée, aucunwriter/embedding/service/migrationLIVE lancé ou autoriséparcettepréparation. Les anciensGO restent caducs ; ce ticket etparentrestentouverts.

## PréparationA retenue parC7 : déclaration standard, data-only

C7 choisit A, pas d'éditionRust/fallbackB. SourceNode existant exige[data,dense], signals[] etmodeindexed, résultat source.meta : consigne de rattrapage sans recherche de candidats ni embarquement de requête, sans lectureFS. source.meta ne sera PAS le juge de disponibilité ; SQLstrictpost/HNSW+graph/FTS/gen/visibilité/debt0 restent requis. Au refus de capacité/résolveur, STOP sansmodifierSourceNode ni flagsinternes.

Manifeste concret `debt-copy-a/backend.json` etgraphe `payer-retard-dense.mmd` préparés. La porte normale needs_embeddings() se fonde sur les entités déclarées ou le fournisseurnommé workspace.index ; une déclarationmodel seule avec entities{}etaucunworkspace joint unAbsentEmbedder. On compose donc le fournisseurcodeSTANDARD, workspace.index=code, rootréelread_only/commandsoff, aucunIndexNode/catch_up_on_open/reaction/watch/outilFS/daemon7970. Il charge/adapte les configurations persistées à la formegénérations, sans rescan de fichiers. C7aexplicitement confirmécechoix de déclaration. Ne pas redéclarer les configs via JSONSchema en normalisant Text→String ou Integer→Int64 : cela introduirait une mutation évitable.

Préflight `--describe` proposé sur ce manifeste etCLI927/ELF0e310 vérifié : commande existante qui charge/valide seulement PreparedBackend, retourne AVANT connect_admitted_storage etAVANT embedder_or_absent (rag3weaver-backend.rs970–978). Aucune base ouverte/créée, aucun providerjoint ; destinationCOPYdoit rester absente avant/après. Ce contrôle de types/raccords n'est pas un GOwriter ni un pilote1800qualifié. Pilotecomplet encore en préparation ; aucun appel7878 réalisé parArbre. Embar sollicité pour identité/coordination fraîche avant le futurGO, sansréservation ni redémarrage.

## PiloteA matérialisé pour RO, pas d'exécution acquittante

Voir [recette08](../16-11-2026/08-pilote-copie-a-dette-469.md), source `debt-copy-a/`, brancheNEUVE cx-arbre-dette-copie-a-20261116-r1 propre/produit927identique. Manifeste68328/--describeportable rc0 validé sansDB/provider, graphSource.metaEXISTANTdata-only. Contrôleurconcret1800outer1798+kill2 aprèsvraiposte/common1730/32G/HIGHmax/swap0/4threads/SDK59 ; nouveaulecteurC++typedRO+selftest AVANTDB, snapshotscompletspublics horsdelta469, FTS5ensembles/gens, visFileScope/bijectionH, cohortetextes/titresBLAKE/hashavantcontact, puisready+ACKEmbarfrais/PIDstart/exe/maxchars/usage0 etuniqueCLIexisting0e. AFTERshutdownréel : SQLdebt0/dim768/finiteTOUSScopeContent_Chunk/marker/HNSWREADY, invsourceCOPY02/SDK/outils/quiet stricts ; pasdeclaim0inventé (claimsopérationnellescohorte consignées, writerunique/copienouvelle).

10jugesCPUsynthétiques0.001s +AST/diffcheckVERT ; C++noncompilé, aucuneDBCOPYcréée, aucunprovider/writer/embedding ni service lancé. ReçuPROVIDER_GO.absent ; nouvellesP/Sabsentes. Qpréparationmetadataàmatérialiser aprèspublicationavantRO ; aucunGO déduit decesjuges. Nouveau `COPY_DEBT_ZERO_CHECKED` éventuel resterait enattented'unerevuepreuves ; minimumcalculs/réutilisationsUNKNOWNsi nonobservés, pas74acquis ni395suffisants. C7demande ROED/Recherche du coupleavantGO1800distinct ; LIVEresteANNULÉ.

## Réserve de revue COPYA avant entrée : conserver le premier refus

ED/Recherche favorables statiques sur7b264a72e1, paquetPC37a2/10fichiersconformes/sourcepropre/P-S-GOabsents. RéserveP2 commune, repriseexplicitementparC7 **avanttoutediffcorrective** : command() lève sur rc!=0 AVANT archivage des stdout/stderrcompile ; TimeoutExpired perd aussi ses captures, goldenbrut/stderr non conservés. Un premier échec natif donnerait seulement « preflight tool refused » sans rc/argv/logs. Aucun CPP/DB/provider/writer n'a été lancé ; ancien couple nonGO/paquetv1 àconserver.

Correction à faire dans le même ticket : trois phases préflight sanscorpus (compile/selftesttyped/copie) doivent archiver argv/cap/rc ou timeout+sorties complètes/partielles **AVANT**require/raise, puisSTOP/first-redimmuable. Tester le VRAIhelpercommand extraitAST avec subprocessfictif, rougepour logsabsentsavantfix, pas de compilateur niDB. NouveaupaquetQv2/pin aprèspublication et ROdelta ; aucun budget ouGO ancien reporté, aucun élargissement de la voieA.

RéserveP2logs traitée : rouge41ba du VRAIcommand AST (compile.stdout absent), puis command archiveAVANTrefus les3phasespréflight sanscorpus avecargv/cap/rcoutimeout/stdout/stderr+goldenbrut, callerne réécritplus. TroisCPUASTfakeverts0.028s+10gatesCPUinchangées0.001s, aucunnatif/CPP/DB/provider. Qv2neuf àpréparer aprèspublication ; Qv1/7bconservéssansGO. Manifeste/CPP/SQL/Source.meta/budget1730dansouter1800 inchangés ; mêmesconditionsROetnouveauGO1800copieseule, LIVEannulé.

## COPYA9a2 exécuté : rouge du juge de protocole, dette non qualifiée

GO distinct C7-COPYA-9a-16nov consommé sur PC, voir [rapport09](../16-11-2026/09-copie-a-premier-rouge-du-juge-meta.md).
CPP/selftest avantDB/copie/inventaires initiaux/FTS5/payload469 passent ; ACK
Embar frais cap12000>=4166 puis writer privé exact0e310 exécuté. Réponse
réelle ok=true/partial=false/signals[]/**pendingCount=0** ; verify_meta
attend **pending_count**, donc refuse à1188885.336819958 avant shutdown.
Défaut du juge établi AVANT tout diff correctif ; aucun défaut SourceNode
ni dette persistante attribué à cette seule réponse.

Scope ROUGE/code1 pour descendants laissés après commande, ensuite
nettoyés/restes[]/autorité terminée ; owned vivants0 avec start contrôlé et
bus absent après arrêt. Aucun SQL post-writer ni invariant final qualifié ;
la copie peut avoir reçu des écritures, son état de dette reste inconnu.
Aucun rejeu, aucun service/LIVE/7970 ni base originale ciblés. P/Q/S et
copie conservés. Preuves376 fichiers manifesteSHA
373dccfa2a9550621fed8c89e0922ea3db56a9ff0666f7a0ed1c7528d398e687.

Prochaine préparation à arbitrer séparément : vrai verify_meta testé rouge
sur contrat camelCase puis correction stricte, nouveau couple/revue et GO
distinct. Ne pas transformer cleanup en preuve de shutdown ni ce rouge en
acquittement des469. Ticket et parent restent ouverts.

Microfix préparé après témoin rouge dd6a953473 sur les392octets réels :
contrat public SearchMeta camelCase repris strictement, pendingCount int0
uniquement, tout alias pending_count refusé (y compris paire concordante),
partial=false explicite. 16fonctionsCPU hermétiques vertes0.032s, aucun
runtime natif ; ancien rouge inchangé. Rapport09 propose un audit RO900 de
la copie figée sans nouveau Source/embedding/writer, première ouverture RO
refusée=>STOP sans recovery ; code et GO distinct restent à obtenir. Rien
n'est lancé par cette correction ni par la proposition.

Sur PREPA explicite C7, audit_result.py est maintenant matérialisé pour RO
du futur couple : 900outer898+kill2/common830 après poste32G, P/Q/S neufs,
copie complète du résultat potentiellement écrit avant toute ouverture,
lecteur typé existant ELFadd01058 épinglé sans compiler, SDK/IDS et références
du rouge373dcc épinglés. SQLgraph/FTS5/vis/gens/bijection puis TOUSvecteurs/
marker/debt sont les mêmes primitives indépendantes ; premier refusSTOP,
aucun writer/Source/provider/recovery/LOAD. SOURCECOPY02 et résultat figé
jamais ouverts en DB, inventaires stricts avant/après/final. Exception
SDK.readers0/mode0644 sur nouvelle copie seulement, WAL nouveau refusé.
Trois fonctionsCPU de gardes hermétiques vertes0.004s, aucun natif/DB lancé.
PréparationPC/RO/GOexact restent distincts ; succès éventuel seulement
FROZEN_RESULT_RO_OBSERVED, pas shutdown propre ni durabilité ni LIVE.

Réserve RO Recherche bloquante sur9a144, consignée AVANT correction :
copy_quiet collecte exe/cwd/tousFD puis attribue seulement après collecte.
Une PermissionError tardive peut perdre un exe/FD déjà observé sous copie
et laisser attributed=False ; le scan accepte alors un UNKNOWN pourtant
attribuable. Aucun audit natif lancé. Préparer témoin du VRAI helper sur
/proc fictif, rouge avant fix, puis attribuer chaque lecture immédiatement
et conserver found/attributed lors des OSError. Nouveau paquet/delta RO et
GO distinct requis ; ancien paquet préparé reste conservé sans GO.

Réserve commune ED/Recherche traitée après rouge e5392b1945 sur vraihelper
avec /proc fictif : attribution à CHAQUE lecture exe/cwd/FD immédiatement,
found/attributed conservés si OSError ultérieur, UNKNOWNattribué fatal ;
UNKNOWNglobal non attribué toujours consigné. TroisCPU hermétiquesverts
0.001s, aucun scan réel/DB/provider. Nouveau Qv2 à préparer ; Qv1 conservé
sansGO, P/S encore absents. Autres branches et bornes inchangées, pas de
nouveau runtime autorisé par cette correction.

## AuditRO EDD exécuté : rouge de représentation du juge

GO distinct C7-DEBT-AUDIT-EDD-16nov consommé, piloteedd5f4a988, scope
validation-1wdottjl, END1190939.194661205. Copie complète fidèle puis148
lecturesRO ; refus1190497.036858159 sur graph!=référence AVANTFTS/finished.
ScopeROUGE1/restes[]/nettoyé/autoritéterminée, ownedvivants0/busabsent.
Aucun autre SQL, aucune reprise ni correction native après rouge.

Défaut du juge établi AVANT diff : les deux fichiers graph JSON relus sont
ÉGAUX sur toutes sections (38tables/65913liens). audit.snapshot retourne
tables et paires en tuples runtime ; json.loads de la référence retourne
des listes. Leur comparaison directe refuse donc sans delta de données.
Ce rouge n'établit aucune mutation produit ; FTSaprès/dims/marker/debt0
restent NONJOUÉS, aucune qualification d'écritures/durabilité/shutdown.
Préparer un témoinCPU du VRAI audit.snapshot et de la VRAIE condition du
driver, rouge avant fix, puis aligner les représentations persistées sans
relâcher les valeurs/comptages/multiplicités. Nouveau paquet/RO/GOdistinct
nécessaires ; résultat et rouge précédents restent conservés.

Preuvesaudit334fichiers PC/local0écart manifeste
e22f56a13b0eeef837d68f3a988c1dc2d98335ee49f3cae81a4373e85940e5eb,
sources/copiefichiersmodesdirsinchangés aprèsrouge/aucunWALnouveau.
Témoin98f303c896 VRAIsnapshot+conditionAST rougeavantfix ; normalisationJSON
des deux côtés puis comparaisoncanonique conservant types/valeurs/multiplicité,
neuf mutations refusées. Continue_result.py préparé séparément : RAWgraph
classifié sans requête snapshot, FTS/finished seulement sur même copieaudit
figée, bornes aprèsposte floor(ENDoriginal−now)−3/refus<=60, pasreset900.
Nouvelles5fonctionsCPU vertes0.024s, aucuneDB/provider. Paquet/source/RO/GO
distinct requis ; expirationinitiale interdit toute extension implicite.

ExpirationCC18 confirmée1190971.838355937/END1190939.194661205, aucune
autorité/P/S/allocation de continuation. DeuxRO ED/Recherche favorables
normalisationRAW seulement, canonicalSHA indépendant934621126891d9d8b35f2f875ed5bf31fc94072a11948169ce49a79ee6f6c07a.
C7autorise PREPAÉTAPEDISTINCTE FIN-RO600, PASexécution. Voir [recette10](../16-11-2026/10-preparation-fin-ro-dette.md) : nouveauWT/branche,
nouvellefreezeFIN entière avantDB, neufsources/pins dont huitCC18historisés,
outer598+kill2/common530 aprèsposte32G ; grapheRAW classifié sans snapshotSQL,
FTS5/finished seuls manquants. AncienEND conservé commehistorique, pasreset.
DeuxCPU nouveaux +2normalisation verts0.025s, aucun natif/DB/provider.
NouveauSHA/paquet/2RO/GOdistinct requis ; les deuxrouges et limites restent.
