# La migration de scopes laisse 469 morceaux sans vecteur transféré

- **État** : ouvert (16 novembre2026), cause NON ATTRIBUÉE ; diagnosticRO des copies en cours, aucun correctif produit écrit.
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
