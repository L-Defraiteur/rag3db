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
