# Le témoin documentaire Vertex attend un accord sans superviseur

- **État** : ouvert, rouge de qualification du 16 novembre ; diagnostic avant tout correctif ou reprise, arbitrage C7.
- **Famille** : qualification
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
