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
