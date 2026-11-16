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
