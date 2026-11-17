# Le témoin documentaire Vertex attend un accord sans superviseur

- **État** : ouvert, rouge de qualification du 16 novembre ; diagnostic avant tout correctif ou reprise, arbitrage C7.
- **Famille** : produit
- **Pour** : rag3weaver-archi, coordination C7 et ED pour la porte si nécessaire
- **Gravité** : bloque la qualification et le landing de pas-va-au-bout ; la suite native complète n'a pas été jouée.
- **Atteignable en service** : attente d'accord oui ; défaut produit non établi par ce seul run.
- **Touche rag3weaver** : oui, tests/e2e_equipier.rs et contrat du témoin sous modèle réel ; aucune modification de code demandée sans diagnostic.

## Constat exact

Source713433bd602a6d35266fe5cd528925ba664924c9, produit37771e225c, base685, lib59ac complète. Le témoin `e2e_equipier::l_equipier_ecrit_la_doc_d_un_depot_de_test_par_pas_juges` échoue sur son délai interne900s (tests/e2e_equipier.rs134), après une demande d'accord `rm Cargo.lock`. La boîte porte ensuite un report annonçant README.md créé et un step_done réussi, puis « en attente du superviseur » pour cette approval. Pas d'accord humain ni automatique donné. L'autre témoin Vertex rend le texte demandé et passe. Résultat de la suite1/1/0 ; pas de succès global.

La priorité d'une question/approval est conservée intentionnellement dans le lot de continuation. Le constat montre une attente réelle jusqu'au délai, mais ne prouve ni une régression de ce lot ni un défaut du garde-fou. Une note9 du juge n'est pas une fin de tâche. Ne pas blanchir rm, fabriquer un accord, filtrer le témoin ou rejouer pour obtenir une réponse différente.

## Preuves et suite à trancher

[Rapport04](../16-11-2026/04-arret-vertex-pas-va-au-bout.md), journaux complets PC et portable23 fichiers SHA manifestés. Le résidu de fixture et les traces de charge sont conservés. Autorité terminée, scope rouge/code1/restes[], bus/cgroup absents, aucun descendant attribué vivant ou inconnu au scan postmortem. Hooks/code/F8/preuve/liblarge/NAV NON JOUÉS.

À faire avant nouveau run : examiner le run modèle/journal de fixture encore disponible et le comportement d'approval ; produire un témoin déterministe CallbackLlm si une correction est nécessaire, définir avec C7 le contrat du superviseur du scénario réel sans élargir les permissions. Décider si fixture, produit ou contrat de test est en défaut, puis publier le correctif et sa recette exacte. Aucun correctif n'est implémenté dans ce ticket. Le candidat de continuation et les tickets S1/S2/S3 restent séparés. Critère de fin : cause et correction démontrées rouge/vert, suite Vertex réellement jouée sans skip ni accord fabriqué, qualification restante sous source/lib exactes, nouveau tour C7.

ComplémentRO : le rapport04 borne l'attribution (approval observée, chemin open_question prioritaire, aucune erreur401/429 dans le seul journal conservé ; stopped_by inconnu). Les tours détaillés du TempDir ont disparu à l'unwind ; seule boîte imprimée/tronquée et juge restent. Aucune réparation automatique ni nouveau run. C7 décide la recette de fixture à partir du contrat des accords, jamais d'une attribution réseau ou d'une permission inventée.

## Préparation rouge déterministe, avant correctif

C7 demande un superviseur de fixture qui refuse uniquement rm Cargo.lock par answer/reply_to/approved:false, sans nouvelle request ni accord générique. Branche neuve cx-archi-doc-supervisor-r1 depuis le rapport rouge ; deux fichiers de tests seulement à cette première étape : helper commun documentation_fixture.rs et témoin CallbackLlm. Le helper garde volontairement le comportement précédent « aucun superviseur » pour voir le témoin rouge AVANT de coder le refus. Il ne s'agit pas du correctif prêt à jouer sous Vertex.

Le témoin réel de l'hôte fait demander rm puis attend la réponse, exige une fin sur la demande initiale après refus, README écrit dans la tâche, Cargo.lock et dépôt principal inchangés, aucun approved:true. Une fixture à échec garde son TempDir/runs/chats/tâches ; le point de rouge attendu préserve la boîte complète AVANT panic. Nouvelle qualification bornée de ce seul rouge nécessite recette/RO/slot distincts ; aucune invocation Vertex900 rejouée. Correctif helper, raccord boucle e2e et gardes approvals inconnues viendront après ce rouge, publiés avant leurs passes. Code teammate de production37771 inchangé.

Pilote rouge concret séparé publié avant admission : [recette05](../16-11-2026/05-recette-rouge-superviseur-documentaire.md). Proposition C7 portée1200 aprèsposte/compile900/exact120, seul target session knownowner repris par garde, ELF antérieurs archivés, aucune Vertex ; résultat attendu demeure ROUGE. Aucun test Rust exécuté à cette publication.

## Réserve Mémoire du pilote0bf, avant correctif

La classification finale EXPECTED_RED n'exige pas scope.code101, ne transforme pas authority_terminated en condition, ne vérifie pas la disparition physique du bus ni une borne fraîche après contrôles/prépublication. Un arrêt après le rouge du driver pourrait être classé attendu à tort. Réserve consignée avant toute correction : témoin hermétique sur le vrai bloc de finalisation, puis contrôle code101/autorité terminée/bus absent/deadline fraîche. Aucun vert global à fabriquer, first-red et journaux conservés ; pilote0bf non admis/non lancé. Sourceea670 inchangée.

Réserve finale pilote traitée : vrai bloc finaliseur AST publiéde844 puis rouge scope.code1 ; correctif207248 publié puis7fonctionsCPUhermétiquesvertes0,035s. Autoritéterminée/busabsent/code101/retourexterne1 et bornes posthash/prépublication stricts. Aucun test Rust encore joué ; sourceea670 et sa baseline sans superviseur intactes, rouge déterministe natif reste à observer avant fixer le refus de fixture.

## Refus réel du pilote f27f491 avant Cargo

GO exact C7 reçu pour APPea670/PILOTEf27f491. Le scan frais après poste refuse PID56584/start117634858, rôle non reconnu, alors que les deux contrôleurs launch/poste sont reconnus. first-red1176350.049473897, expected_red=false ; résultat UNEXPECTED_FAILURE, scope ROUGE/code1/restes[]/nettoyé. Aucun Cargo ni témoin Rust exécuté, aucune reprise du target ; anciens ELF copiés avant scan. P/Q red1 consommés, preuves PC cx-archi-doc-supervisor-red1 conservées. Cause du processus supplémentaire à établir ; aucune exemption ni relance décidée. Le rouge attendu du témoin reste NON JOUÉ.

Diagnostic borné après STOP : PID56584 a disparu au contrôle postérieur ; aucun fichier de supervision conservé ne contient son identité détaillée. cache-after-poste.json conserve seulement PID/start/rôle nul pour found, pas le lien attribuant (argv, cwd, exe, FD), ni ses parents/CG. Le snapshot original est donc insuffisant pour prouver watchdog ou lecteur concurrent. La commande publiée contient bien timeout TERM/kill-after15s/1320s devant poste, mais cela ne prouve pas que ce PID était ce watchdog. Ne pas innocenter rétrospectivement ce refus. Plan soumis C7 avant code : compléter la capture des motifs et identités exactes/ascendance pour tout found, sans exemption nouvelle ; témoins hermétiques sur le vrai scan avant nouveau run. Un éventuel rôle WATCHDOG ne sera permis que sur preuve exacte publiée, jamais exemption des ancêtres/CG ni accès T/OLDW. Reprise éventuelle bornée par le reste de la fenêtre initiale, soumise C7 ; aucun nouveau budget décidé ici.

Sonde RO seule autorisée C7 : recette06, branche cx-archi-doc-observation-r1 ;60s après poste, P/Q neufs et même commande réelle1320/poste. Capture motifs/identités/ascendance et policy avant assertion, aucun nouveau rôle exempté ni target/Cargo/test/signal. Cinq témoins vrai scan sur faux proc publiés avant essais. Toute observation reste OBSERVATION_ONLY et PID56584 non attribué ; nouvelle qualification native distincte après arbitrage.

Sonde25107 réellement faite, neuf preuves deux postes réhash0écart (manifeste d723aaa6…438) : nouveau PID203819/start117708245 est timeout1320, attribué uniquement par argv[4] scriptposte Wsource, ascendance/identités/FD conservés. PID56584 reste nonattribué. Rapport06 publie diagnostic et plan WATCHDOG avant fix ; aucune exemption ajoutée, aucun Cargo/target muté, aucune qualification native. Nouvelle étape sous arbitrage C7 distinct.

Préparation WATCHDOG autorisée C7 après sonde : branche cx-archi-doc-watchdog-pilot-r1/rapport07, réel constructeur partagé lanceur/policy et matcher strict+10témoins fauxproc publiés avant essais. T/OLDW/source restent refusés hors arguments contrôleur ; aucune exemption générale. Étape corrective1200 explicitement PROPOSÉE distincte, seulement après expiration ENDinitial1177548.693791571 ; aucun GO natif par cette préparation. APPea670 toujours non compilée et rouge attendu non joué.

Réserve Mémoire sur9f52 AVANT correction : échec snapshot Permission/OSError/ValueError est archivé unknown/attributed=None et ancestor_errors, mais fresh_cache ne les refuse pas. Un lecteur mêmeUID illisible peut donc être accepté avant mutation. Couple9f52 réservé/nonlancé. Plan : capturer les champs encore lisibles et leur attribution ; bloquer UNKNOWN attribué, et refuser conservativement toute attribution impossible ainsi que ascendance incomplète. Snapshot reste publié avant assertion. Témoins vrais scan/matcher/fresh-admission avant nouveau couple RO ; aucune exception de baseline inventée.

RéserveUNKNOWN : contre-exemple vrai bloc admissionAST publié62f146 puis rouge0,009s ; fix9f9e publié puis16fonctionsCPUvertes0,092s. Snapshots partiels/motifs conservés, refus conservateur de toutUNKNOWN/ancestor_errors après archivage, aucune baseline exemptée. Aucun Cargo/PID/signal réel ; nouvelleRO nécessaire, étape corrective1200 proposée sansGO.

Classification UNKNOWN demandée C7 : rapport08, preuve actuelle2b38da2e…93c1 réhashée deux postes. Parmi13PID anciens,10actuellement présents/UID1000/processus de session avec exe/cwd/FD refusés ;3absents. Pas de start historique : aucune identité ancienne reconstituée. Arguments/CG lisibles hors racines ne prouvent pas FD absents. Garde2397 doit refuser et aucune commande1200 lourde n'est proposée dans cet état. À C7 : établir observabilité complète ou preuve indépendante explicite relue ; aucune allowlist ni changement de protection/service, aucun Cargo/signal/targetmuté.

HOLD natif2397 confirmé C7, aucun GO1200 ni exception UNKNOWN. Exploration RO rapport09 : garde commune supporte cache neuf/vide et sous-dossier par worktree ; proposition d'UN sous-cache actif neuf dans même racine session, ancien état gelé/intouché,32GiB cumulés, aucune invalidation/import. Invariant différent explicite à RO avant implémentation ; si Wsource opaque reste bloquant selon contrat, HOLD maintenu. AncienT mesuré6108704768octets, aucun dossier neuf créé, aucun Cargo/service/signal. Budgetcorrectif distinct proposé seulement, aucun nouveau créneau occupé.

Accord C7 PRÉPARATION initial-vide reçu (contrat6b307+ROMémoire), pasGO natif. Pilote neuf cx-archi-doc-empty-pilot-r1/rapport10 : enfantcache exclusif neuf dans mêmeT, ancienétat entièrement réattesté/sansimport,32GiB cumulés, source/gitlinkSHA, chaîne bornée au vrai lanceur enregistré préPopen, UNKNOWNowned/attribué/W/newTbloquant. Lecteur source readonly prouvé n'est pas writer, ancienopaque resteUNKNOWN. Code/témoins publiés avant essais ; étape1200 distincte proposée, aucun enfantcache/PQS/Cargo créé.

Rouge déterministe natif acquis sous GO C7 INITIAL_EMPTY : APPea670/PILOTE373626, compilefroide3m50 puis UNexact0/1/0 en0,64s au panic nommé manque-superviseur, fixture complète gardée avantpanic. Rapport11+manifest1209fichiers cbd90dd2…a4ba copiés/réhashés deux postes0écart, ELF4271238c…d446 et ledgeravantjuge. FinalEXPECTED_RED/nonVERT extérieur1/scope101/restes[]/nettoyé/TERMINE/busabsent sous244,758s<1200, ancienétatSHAinchangé ; contrôle1043observationsPID/start alive[]owned_unknown[] et scanfound[],14UNKNOWNglobaux conservés. Ticket reste OUVERT : refushelper/raccorde2e noncorrigés, Vertex/qualificationcomplète nonjoués. Aucun run supplémentaire ni nettoyage. C7 autorise ensuite implémentation harness seulement ; nouvelles source/recetteRO avantGOcourtvert, pasnouveauTparessai ni Vertex900retry.

Correctif harness autorisé C7 après rouge11 : branche source neuve cx-archi-doc-supervisor-fix-r1, rapport12. Helper refuse uniquement commandexact rm Cargo.lock viaanswer/reply_to/approved:false ; unknown/question/id/ambiguïté/grant=>STOPpréservé, jamais nouvelle request. Vraie boucle e2e raccordée, lock suivi valide avanttâche/octetstâche+main/README/HEAD/status et juges conservés, derniermailbox gardé avantassertions. Cinq tests policy écrits en plus du natif rouge existant ; NON JOUÉS pour ce correctif. Rustfmtparse/diff seulement, aucunCargo/target. Futurvert garde UNmêmecache ; transition same-owner proposée rapport12 à arbitrageC7 avant mutation/pilote, copiesea670/ancienledger/ELF restent gelés. PasVertex900 niGOreporté.

C7 autorise PREPA transition SAMEOWNER apresROsource1bd : pilote concret cx-archi-doc-green-pilot-r1/rapport13. Git-onlyWTpreuveEA detached/admin propre/gitlink/fulltrackedSHA, scopeROUGEtermine/identitesgone/ledgerELF/cacheidentite/ancienetat stables, puis branchPCNEUVE1bd au memeC avec refEA/remotes intactes ; inventairecache complet prepostGit identique. Vert propose600 distinct/compile360/5policyexacts+1native120, vraie gardeSAMEOWNER sansinvalidation FP/buildrsavantapres,UNmemeT32GiBcumulés. Code/temoins publiesavantessais ; aucunGOvert/Vertex ni transitionPC a cette publication.

Preparation Git-only396d faite sous accordC7 : vraiecopieea670 detached/admin/gitlink independants, fulltrackedSHAegaux ; WPC brancheNEUVE1bd au memeC, refEA/remotes/ELF/cache complet prepostGit inchanges. Aucunegarde/Cargo. Defaut de preparation pilote constate AVANTfix suivant : filtreenv herite fixe CARGO_BUILD_JOBS=8 et omet POSTE_FILS/RAYON ; jMIN8/POSTE_FILS non garanti. Le rouge373626 reste rouge de comportement, pas mesure de budgetCPU/thread attestee ; CPUquota commun non remis en cause. Ajouter temoin du vrai blocenvAST, publier puis observer rouge contre POSTE_FILS2, corriger transport et bornemin ; nouveauSHA/ROavantGOvert. Aucunrunvert lance.

PreparationGit396d acquise1181658.951518812/cinqJSON PC/localrehash0ecart : archiveEA physique indep/gitlink/fulltrackedSHA, PCbrancheNEUVE1bd memeC, cacheprepostGitSHA214ed4d…5757 identique. Defautenv5d61 rougeAST0,003s puisfixa506 publie+38CPUverts ;2temoinsreelguardSAMEOWNER/preflightetranger publiese60 puis40CPUverts0,288s. POSTE_FILStransporte/jobs+Rayonmin8/Rusttests1. ContrôleurPC396d doitFFSHAfinal ; aucunegarde/Cargo/natifvert. NouveaucoupleROavantGO600distinct, APP1bdinchangee.

Réserve Mémoire EXACT1b9e AVANTfix : contrôle END avant save(result.json), puis write_text peut ouvrir tard et publier GREEN_TARGETED_ONLY aprèsEND. Couple1b9e réservé/nonlancé. Corriger comparaison APRÈSopen immédiatementavantdump, convertir UNEXPECTED_FAILURE/rc1 et employer valeur retournée pourprint/exit ; témoin du vrai prepare/final+save avec open retardé99→101/END100 publié avant essai. SourceAPP1bd/cache/proofs inchangés, aucune admission native.

Réservepostopen : vrai prepare+save40591 publié puisrouge0,004s ; fixe63c88 publiéavantrejeu puis41CPUverts0,299s. Save compareAPRÈSopen/avantdump, écritUNEXPECTED_FAILURE siéchéanceexpirée et retourneverdictpourprint/exit1.1b9esupersédé/nonlancé,APP1bd/transition/cachesintacts. NouvelleROcouple avantGO600distinct ; toujours0PQSvert/Cargo/Vertex.

Vert CIBLE réel sous GO APP1bd/Pb12 :5policy exacts1/0/0 chacun +natifexact1/0/0 en2,26s,6distincts/6invocations+inventaireNONjuge, compile42,17s. Rapport14+36preuves PC/localrehash0ecart/manifeste fa7f6193…c584/ELF734f6547…bffa et ledgeravantjuges. FinalGREEN_TARGETED_ONLY/nonqualificationglobale,scope0/restes[]/TERMINE/busabsent/postopenEND sous66,667s<600 ; scan78PIDstartobservationsalive[]unknownowned[]/found[],12UNKNOWNglobaux conserves. ArchiveEA/ancienELFrouge/ancienetatT proteges, vraieguardSAMEOWNERcacheprepostegal, POSTE_FILS/CargoRayon5. Ticket OUVERT : vraie bouclee2e/Gemini etqualificationcomplete NONJOUEES, aucune Vertex900relance ni nettoyage/master.

C7 autorise PREPARATION qualificationCOMPLETE apresROae8f. UnionNEUVE surmasterbef/produit377+tests1bd, rapports15/audit/rangeproduit5=,4fichierslot==1bd/6913horsdocsprotegesmodeOIDmaster0ecart ; seulconflitdocumentticket absent preserve. Recette16 PROPOSE7200neufs avec vraieVertex/juge+D6privee reelle puiscanoniqueF8/lib/NAV ; source/services/targets/sandboxSDKtempscouts explicités, pasd'autoapproval niCargosS0. TicketnomblancD6distinctcreeavantartefactprive, S1/S2/S3inchangés. Aucunlourd/runtime/modele/service par cette préparation ; piloteconcret/RO/GOencore requis.

Autorisation C7 PREPA source TEST explicite du16novembre, après constat consigné branchefull-pilot-r1/92116e997a : les vrais setups e2e_equipier::depot_de_test et chat_qui_code::depot_inscrit ne raccordent aucune configuration Bac privée avant add_files. Préparer deux témoins appelant réellement ces setups avec une déclaration privée extra_readQ/env via enfant Rust isolé ; aucun set_var global ni Cargo de fixture, aucun montage de bac, modèle ou DB. Le JSON exact doit déjà figurer dans le manifeste ET le HEAD du backend créé par add_files avant toute tâche. Les témoins seront publiés NONJOUÉS ; absence actuelle du raccord attendue rouge, pas compilation cassée. Aucun correctif avant observation rouge sous nouveau GOcourt source/pilote exact. Ensuite seulement raccord TEST via primitive sandbox existante, nouvelle APPunion/RO avant7200. Unitlibtoy conserve Cargo réel/TMP privé, aucun shim partagé ni installation registry. Présente branche neuve depuis APP274 ; produit nonmodifié à cette consignation préalable.

Source témoins049153945e publiée/NONJOUÉE, setups inchangés/correctif absent. Rappel C7 de reprendre le SEUL pilote déjà ouvert traité : rapport20/doc-bac-red matérialisent1800 NEUFS/timeout1920/Git-only1bd→049 avec archivephysique/1T SAMEOWNER/2builds sansrun/inventaires/2ELFledger répliqué AVANT2exacts rouges typés cap45. Six contrats hermétiques écrits NONJOUÉS avant publication, AST seulement. Common scopeROUGE101/expectedstatusNONVERT etfinalpostopen borné, aucune mutation target ni Cargo/native/DB/service/sourcePC en préparation. Paquet horsW et RO exacts avant GOdistinct ; aucun nouveau lot ni réparation avant les rouges.

Clarification C7 du budget avantGO :1920 WHOLE inclut common1800 et final≤120, pas1935. Défaut constaté AVANTdiff : constructeur81d066 donne timeout1920+kill15, donc enveloppe dure1935, et résultat final emploie ENDcommon au lieu de déclarer sa propre réserve bornée. Adapter le témoin constructeur DÉJÀ publié pour exiger timeout1918+kill2=1920 et budget phase/final/global explicites ; publier avant la première exécution des sixCPU autorisée (timeout60/failfast, aucun PID/DB/natif). Rouge ⇒ STOP/diagnostic conservé, pas de rejeu aveugle. Ensuite seulement correction sur même branche, nouveau pin/paquet ; préparation Git/QPC déjà autorisée, aucune autorité native.

Rouge CPU08523 acquis AVANTfix :4PASS/1FAIL constructeur/6eNONJOUÉ,RAWmanifest e4bbdc8c horsW/diagnostic a08b publié. Dernier arbitrage C7 après messagescroisés CONFIRME1920dur, pas alternative1935 ; microfix1918+kill2 et MINcommon/soft/finalsolde sur mêmebranche, renouvellement explicite unique sixCPU purs après publication. Ancienrouge préservé, aucun lancement natif. PrépaGit/QPC/copieshash reste déjà autorisée et sera poursuivie sans accord redondant, APP049/setup inchangés.

Arbitrage FINAL C7 suivant reçu après publication015ce566 :1935HARD/TERM1920+KILL15, sans autre aller-retour. Retour du constructeur original, constantes/docs1935 et MINENDcommon/soft/finalsolde cohérents ;015 et contrat1920dur supersédés avant tout run, ancienrouge08523 conservé comme contrat remplacé, jamais peintvertproduit. Unique sixCPU renouvelé à jouer après publication finale ; prépaGit/QPC autorisée, pasnativeGO. APP049/setups inchangés, même branche/lot.

GOexactDDE6 closROUGEINATTENDU : libcompile0 puis Cargo refuse targete2e_equipier inexistant, scopeROUGE1nettoyé/restes[]/TERMINE/bus+cggone/58ownedgone,0juge/ledger/marker. Rapport21/39SHA c1002d92 etELFlib15d copiésréhash2postes, firstredimmutable. C7 confirme autotests=false/targete2e/mainmod e2e_equipier ; autre défaut de montage : enfant --exact nonqualifié aurait0juge. AVANTfix : quatre tests purs réels Cargo.toml+main+sourceenfant+NAMES+ASTconstructeur+artifact/cfgfeatures prépubliés NONJOUÉS. Prépa correction de montage autorisée seulement, setupsNONcorrigés/retryINTERDIT ; nouveaupin/RO/GOdistinct etMINresteoriginalsoft1197212.902/hard1197227.902. RéemploiELFlib conditionnel àpreuve source/profil/env, jamaisqualifiéparsimplecompile0.

Reprise GOexact223/72338 acquise après montagecorrigé etRO : rapport22/2111SHA9d82deuxpostes0écart/fixturesCOMPLÈTES+2ELF, ledgerpub1197104AVANTmarker/juges. DEUXrougesprécis réellementvus (2juges4invocs, chaqueparent+enfant0/1/0), firstredexpectedimmutable, scopeROUGE101/0restes/TERMINE/bus+cggone, final1830,931sdepuisSTARTORIGINAL<1935sansreset ; post130ownedgone/11UNKNOWNglobauxconservés. Avanttoutdiff suivant, archive etprésentticketpubliés. C7 autorise CORRECTIFTESTS commun AVANTadd_files/commitHEAD viaSandboxConfig existant/extrareadQ/targetsprivés, sansprodAPI/registry/race/setenvglobal ; publier UNAPP+protocole2verts/recette puisROavantnouveauGOdistinct/fenêtreNEUVE. Source223/049/1bd etanciensrouges restentfigés, aucunancienEND/GOréactivé. D6/7200/Vertexréel/F8/libentière/NAVencoreNONJOUÉS.

CorrectifTESTS préparé après preuvepub480604 etrattachementce4dd8 AVANTcode : rapport23, helperpartagé cfgtest connect_declared_sandbox<realSandboxConfig>, absenceFILE=>normalinchangé ; fichier/targetrootcanoniques, Landlock/réseaufermé/extra_writevide/approval/tokensrepo_target+tasktmp/offline/jobsMINposte validés, injectionVERBATIM avantwrite/add_files/commitHEAD. Deuxvraissetups appelants mêmehelper, aucuneAPIprod. Parent déclaretargetprivéVIDE parCommand.env/j1sanssetenvglobal ; enfantassertmanifest+HEAD puistracepositivefiltrée, parentrendstdoutenfantpourprouver4invocsvertes. Protocole2verts1/0/0chaqueparentenfant+recetteNEUVE1800MAX/global1935proposés,NATIFNONJOUÉ/ancienGOexpiré. NouveauAPPsource/RO puispilote/GOdistinct nécessaires, ticketOUVERT.

### Reprise de préparation globale après suffisance ciblée C7

C7 accepte la suffisance cibléea4a2/07fc (deux ROfavorables), puis autorise
PRÉPAseule unionneuve master99fef92+source742 et recetteglobale/D6. Branche/WT
cx-archi-pas-va-au-bout-union-r2 ; rapport31/audit6913protégés0écart/5produits
modeOID==742/11patcheségaux ; nouveaux entrants Mem/B0/Vertex conservés.
Rapport32 propose SandboxConfig réel/Qextra_read/targetsprivés AVANT création
Task(ouvrir_une_tache_nommee)/D6 distinct/2150+6439+SDK59/baselineT0trusted+
freshpostroles/guardold→new. Proposition7200Common/7455physique à arbitrer,
aucun GO ni pilote concret global ni Cargo/Vertex/service/PCtransition encore.
Sources742/P01c3/T/72preuves restent figés ; ticketOUVERT et suite à C7.


Point ressource de la recette32 explicité AVANT pilote : canonique2150 force
jobs8 ; proposer vrai poste mode mesureEXCLUSIF avec quota800%/POSTE_FILS>=8,
refus avantmutation si place plus petite, aucunshimPATHcanonique/interception
ou faux profiljobs5. Coût de réservation exclusive et7200/7455 à arbitrer C7,
aucune entrée/poste/activité réelle effectuée.

### Matérialisation du pilote global, après RO totale recette32

C7 arbitre PREPA7200Common/7440TERM+15KILL, mesureexclusif/quota800%/FILS>=8,
MémoireROtotalef1e8 favorable. Nouvelle branche/WT
cx-archi-pas-va-au-bout-global-pilot-r2/cx-wt-archi-pas-va-au-bout-global-pilot-r2,
APPf1e8 figée. Avant code : corriger une imprécision du rapport32 : rag3daemon
sert une base, pas DaemonHost ; le vrai hôte D6 est rag3weaver-backend daemon
--address127.0.0.1:0, construit de l'APP, avec son répertoire daemon privé.
Le pilote doit vérifier cet argv dans les vrais Cargo/main/CLI et ne pas
inventer une route TaskService. TaskAPI réelle ouvrir_une_tache_nommee, TTY réel
crate tty/featurevue. SourceRust/APIprod intouchées. Piloteseul sous docs,
publication/témoinshermétiques/ROcoupleexacte AVANT tout Cargo/Vertex/service/
PCtransition/T0mutation. APP742/P01c3/72preuves gardés. Aucun GO7455.

### Réserve statique C7 sur le témoin CPU du pilote dc868, avant correction

Le témoin importé `test_real_cpu_profile_requires_six_absent_fields_and_rustup_ld`
cherche encore `driver.py` et ses anciennes constantes, absents du pilote global.
Défaut certain en lecture seule, aucun essai exécuté : aucun rouge dynamique revendiqué.
Raccorder le témoin au vrai constructeur `Observer.cargo_profile` actuel par AST/appel,
sans recréer un profil attendu inventé dans la fixture. Regrouper les réserves de RO
avant nouveau pin et GO CPU distinct. Aucun GO natif ni activité réelle.

### Réserves groupées Banc sur dc868, avant corrections

RO totale Banc publiée 49edc50197, rapport banc17nov/02. HOLD natif conservé :
R1 le shim lit C/scripts/garde_target.sh hors extra_read Landlock ; R2 témoin
CPU hérité absent déjà consigné ci-dessus ; R3 `judged` accepte tout panic du
test nommé sans preuve de cause assertion Om bre/agent.name ; R4 empreinte
controllers de provenance périmée après normalisation EOF. Corriger et publier
les témoins discriminants avant essais, puis revoir ensemble le couple exact.
Aucun test, import du pilote, processus, signal ou accès cloud/DB lancé.

### RO totale Mémoire dc868 : groupe complet avant corrections

HOLD confirmé, six points reçus ensemble avant édition du code :
- Montage D6 provider vertex invalide : APP attend Provider Local/Service/Compatible,
  Vertex étant un protocole de Compatible. Tester le vrai schéma de l'APP.
- Témoin CPU hérité : traité par témoin du vrai constructeur global, NONJOUÉ ;
  six absences anciennes ne sont pas une précondition du global SDK hérité.
- Rouge D6 causal : nom+panic+0/1/0 insuffisants. Exiger assertion métier préattendue
  Om bre/agent.name, vraie source et contre-exemple panic d'une autre assertion.
- Provenance broker : git diff --binary ignore index et non-suivis. Capturer HEAD,
  index et état source complet pertinent ; comparer avant/après build, puis attribution
  agent avant ajout du reviewer. Refuser toute mutation non attribuée.
- Runtime : environnement/maps relus après identity sans bracket final. Archiver un
  frame cohérent identité/start/exeSHA/CG avant/après, avant assertions ; aucune
  attestation reconstruite en cleanup ni déduite de l'inventaire.
- Première cause : d6 finally arrêt/wait et qualify finally service.close peuvent
  masquer l'erreur initiale. Latcher au refus, préserver cause immuable ; cleanup
  séparé borné MINEND avec host/TTY/broker/reap explicites, sans profil SDK.

Split no-run/ELF accepté en principe sous vérification des vrais raccords ; aucun
pin PC/Q/T0 ni ACK services/Vertex acquis. Budget/CLI/watchdog/finalfsync relus
sans réserve supplémentaire dans ce périmètre. Aucun import/CPU/native/SDK/proc/
signal/cloud/DB/HTTP exécuté par cette RO. Attendre groupe ED avant pin correctif.

### RO delta Mémoire 4461 : juge rouge→vert, réserve avant correction

HOLD sur réserve résiduelle unique : `red_source` est appelé seulement pour
agent-red ; agent-green peut garder le nom et remplacer le corps par assert true.
Le reviewer final n'atteste pas l'identité du juge rouge→vert. Exiger D6_RED_TEST
inchangé à admission/built/tested pour les deux étapes agent, validateur seul
modifiable ; témoin au vrai Broker.handle refusant l'admission verte altérée,
publié avant correctif et NONJOUÉ sans GO CPU. APPf1e8 inchangée.

### RO Banc 60f9 : activation du juge, avant correction

HOLD résiduel eac60810b4 : substring/hashconstant ne prouvent pas le juge actif.
Ajouter cfg(any()) avant bloc intact et un homonyme vide peut contourner le vert.
Comparer au rouge acquis tout le backend hors CORPS de la seule fonction validateur,
et tout autre source/index/mode ; aucune modification du juge/attributs autorisée.
Écrire avant fix un témoin réel Broker.handle cfg/homonyme refusé avant session.
Pas de test exécuté ni de GO reporté ; APPf1e8 et anciens rouges préservés.
