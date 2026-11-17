# Passation autonome pour Claude — hotfixs C++, 17 novembre 2026

## STOP prioritaire de Lucie : cette page prime les historiques

Arrêt demandé par Lucie pour quota, relayé par C7. **Aucune reprise automatique.**
Tous les coupons non consommés sont RÉVOQUÉS ; les anciens END sont expirés et
NON RÉACTIVABLES. Aucune recette copiée ci-dessous ou dans les archives ne vaut
GO. Aucun nouveau calcul/test/build/health/provider/DB/SDK/master/tag/nettoyage.
Les seuls travaux de fin autorisés sont sauvegarde/push non-master, passation et
vérifications Git/métadonnées nécessaires. Les consignes antérieures « poursuivre »
sont remplacées par ce STOP. C7 fait la synthèse générale.

Session : cœur C++/hotfixs/tickets. L'arbre principal de Lucie n'a pas été modifié
par nous ; aucun commit ni changement de branche dans cet arbre. Le travail
propre de session vit dans les worktrees cx-hotfixs. Ne pas effacer archives,
worktrees ou targets, ne pas nettoyer les fichiers de Lucie. Pas de force push.
Aucun secret n'est nécessaire à la reprise ; ne lire ni .env ni .vault.

## Livré, en cours, non qualifié

**Livré par ce chantier :** preuves conservées du rouge binder SHOW_INDEXES,
correction compilable de fixture LogicalType et propositions documentaires
membership. AUCUN correctif moteur de rows_outside_graph livré ou intégré.
Les lots historiques de nuit déjà sur master sont décrits dans la passation du
16nov ; ne pas les présenter comme réalisés de nouveau ici.

**Travail actif arrêté :** conception de rows_outside_graph exact au snapshot de
l'appelant, même ticket depuis le08nov. Orientation C7 : troisième REL interne
de membership, séparée des deux graphes ANN ; présence o→o, singleton compris.
ONE_ONE/FWD impose AU PLUS1 par source/direction stockée, pas la présence,
la complétude ou o→o. Les deux opérations proposées assurer_present0→insert,
1→noop et assurer_absent0→noop,1→delete_relID ne sont pas une API implémentée.
Ne jamais ignorer une erreur de multiplicité/conflit/I/O pour simuler un upsert.

**Préconditions non acquises :** idempotence de tous les mutateurs et vue stable
de maintenance, rétention antérieure à toute capture sous oldtx/DROP/checkpoint/
GC, résolution ancienne OID, durée de vie des tables/colonnes/pages, upgrade
ancienWALnonvide→checkpointnouveau durable→rotationWAL→capacité. Aucune
réservation/code/format bump/prototype/fixture nouvelle autorisé. Aucun résultat
de faisabilité globale acquis. Mémoire a donné une RO documentaire favorable
CONDITIONNELLE, pas un GO code.

## Ticket et lectures indispensables, dans cet ordre

Ticket OUVERT :
`docs/coeur-cpp-hotfixs-tickets/tickets/2026-11-08-le-nombre-de-lignes-hors-du-graphe-d-un-index-vectoriel-lisible-sans-ecrire.md`.
Il rassemble les autorisations, preuves et suites ; aucun doublon à créer.

- [Contrat REL conditionnel](03-membership-relational-contract.md), dernière
  correction e4ea441ac9 : remplace « au moins » erroné de269 par AU PLUS1.
- [Faisabilité RO et lacunes](02-membership-feasibility-readonly.md), e4abc7cd05.
- [Comparaison des options et trois groupes Mémoire](01-membership-persistent-options.md),
  588284a313 puis944c1685c7. La préférence initiale NODE est dépassée par
  l'orientation REL ; les réserves de rétention et de format restent entières.
- [Preuves de membership manquant](../16-11-2026/11-rows-membership-storage-findings.md),
  6b83b41e94, et [capacité/coût proposés](../16-11-2026/10-rows-outside-graph-read-capability-proposal.md),
  ebcd3d46df.
- [Rouge binder effectivement observé](../16-11-2026/09-rows-outside-graph-binder-red-observed.md),
  2508291714, et documents07/08 pour l'échec de compilation préalable.
- docs/README.md, AGENTS.md, docs/procedures/push-d-un-lot.md. La présente
  interdiction de reprise prime leurs recettes historiques.

## Contrat public préservé et témoins

Valeur exacte = lignes visibles avec vecteur valide, effectivement absentes du
membership complet de la BONNE instance au snapshot appelant. Pas de
numTotalRows-watermark, pas de degré ni COUNT Graph physique/topK. Capacité
inconnue/nonREADY=>NULL, jamais0 inventé ; corruption/I/O/annulation distinctes.
Une colonne toujoursNULL ne résout pas le besoin. Append UINT64 nullable en9e,
les huit champs précédents restent noms/ordre/types inchangés : table_name,
index_name,index_type,property_names,extension_loaded,index_definition,state,
extension_error. Le vrai consommateur Rust par nom doit être relu au SHA futur ;
aucun produit/catalogue/generic_search_nodes réservé/modifié. Priorité migrations
Arbre/Glib/pose, aucun droit sur les bases de Lucie.

Quatre fixtures existantes sur source71fbe : zéro index complet, READ ONLY sans
verrous retenus, watermark artificiellement0 mais graphe réel complet, trous/
NULL/delete puis reconstruction. Le premier seul a été joué : il s'arrête sur
le binder absent, AVANT les assertions de zéro et de schéma9. Les trois autres
sont compilés NON JOUÉS. Ne pas écrire const0 pour faire passer quatre zéros.

Sept groupes de témoins de note03 sont NON ÉCRITS/NON JOUÉS : singleton,
COPYpositif déterministe, idempotence/rollback/NULL, vieux writer après attente,
offsets finaux/remap, deux oldtx/GC et deux instances, upgrade/crashes/refus
ancienmoteur, budgets multiindex/prepared (regroupés en sept dans la note).
Aucun hook test-only encore réservé. Les plafonds de lecture proposés ne sont
pas des performances mesurées ; budget partagé par appel, allocations/CSR/
versions/E/vecteurs inclus. Aucune erreur/annulation changée en NULL silencieux.

## Trois constats source confirmés indépendamment par Mémoire

Base fraîche fetch origin/master figée99fef92ce6ee9a0007eaf99c7c3a6a3789061c57.
Diff avec71fbe vide sur stockage/transactions/headers associés/vector. À
revérifier sur une nouvelle base après réautorisation ; aucun nouveau fetch
nécessaire à ce STOP.

1. LocalStorage::commit fige nodeTableIDs AVANT callbacks HNSW commitInsert,
   puis relève les REL APRÈS les NODE. Nouvelle NODE M au commit non couverte ;
   précréer vide ne garantit pas l'ordre. L'orientation REL évite ce point mais
   pas unicité/idempotence/remap/WAL/rollback/tous mutateurs. LatestCommittedView
   temporaire est seulement une option de maintenance sous exclusion prouvée,
   jamais dans le diagnostic E/M oldtx.
2. reclaimDroppedIndexes retire les drops sans horizon ; reclaimDroppedTables
   teste DUMMY, reclaimStorage puis erase(unique_ptr). getEntryOfOID filtre
   l'OID de tête AVANT versions. SharedIndex seul insuffisant. Deux témoins
   distincts requis : vieux snapshot sans capture initiale, et scan acquis
   suspendu ; après DROP/reCREATE/checkpoint/GC. Pas de reproUAF native acquise.
3. Recovery dernierCOMMIT rejoue puis tronque/conserve WAL ; initWriter n'ajoute
   FormatRecord que si WAL taille0 ; Database.init ne force pas checkpoint
   d'upgrade. SIMPLE BUMP ne prouve pas la barrière avant checkpoint. Header et
   dryReplay savent refuser versions nouvelles lorsqu'elles sont durables ;
   transition/rotation à prouver, vieux reader ET writer, crashes à chaque borne.

## Preuves natives clôturées, immuables, deux hôtes

Aucune allocation active. Deux passages ROWS consommés une fois, jamais reset.

- RED01 source5099/pilotedb833 : compilation refusée vector<LogicalType>
  initializer_list/copie privée ; aucun cas natif joué. Fin237.437s, scope
  quiescent/PID0,38fichiers657694523B rehash2postes0écart.
  Manifest SHA f0675562ebba890948da012106fe63b308b7c03799eef30453f3e931f077c179.
  Paquet PC `~/.cache/rag3weaver-build/cx-hotfixs-rows-outside-graph-red-v1`,
  portable `~/.cache/rag3db-moteur-notes/rows-outside-graph-red-01`.
- RED02 source71fbe/piloteb307 : build1012/1012rc0, un exact
  VectorIndexDiagnosticsTest.ACompleteIndexReportsAKnownZero,32ms/rc1,
  failure réelle «Binder exception: Variable rows_outside_graph is not in scope».
  BUILD_ID natif59ac réel. Assertions0/schéma9 non atteintes,0skip/error.
  Fin219.838s/quiescent/PID0 ; collecteur0 signifie rouge attendu collecté,
  résumé toujoursROUGE/NOT A DELIVERY PASS.48fichiers755407689B/6artefacts
  rehash2postes0écart. Manifest SHA
  f49462395124e1e605d9303d743c127f665a3adc160e668e471431a60a5c52ee.
  Paquet PC `~/.cache/rag3weaver-build/cx-hotfixs-rows-outside-graph-red-v2`,
  portable `~/.cache/rag3db-moteur-notes/rows-outside-graph-red-02`.
  Git `docs/coeur-cpp-hotfixs-tickets/16-11-2026/preuves/rows-red-02/` conserve
  manifest+tarlogs ; gros ELF/archive/vector dans les paquets2hôtes.

RecetteRED02 SHA37e29c250113c12bd4ff0385a1ca06590c41cfa519268c3590bf699223387867,
bundle42f19dc05811589c8f4180c0c1546e85862e76a1c9f31ab769107a1d747ed280.
ELF test c7e96109572fb990946872dc674410666ef25701277e3e7fad3309e4de6392a6.
Ces pins sont des références historiques consommées, PAS des commandes à relancer.
Les dernières vérifications de fin sont des métadonnées/Git seulement, pas un
nouveau rehash exhaustif des deux gros paquets.

## Worktrees, branches, targets : état de fin

Tous les worktrees cx-hotfixs inspectés sont propres avant écriture du présent
rapport ; branches locales==refs origin correspondantes (inventaire ci-dessous).
Le SHA de cette passation sera communiqué par C7 et le message final ; impossible
de l'inclure récursivement dans son propre contenu. Aucun WIP produit non poussé.

Worktree principal de continuation documentaire :
`/home/lucied/git_workspaces/cx-hotfixs-rows-outside-graph-red-pilot-v2`, branche
`cx-hotfixs-rows-outside-graph-red-pilot-v2`. HEAD prépassation
`e4ea441ac91349d20afd6eb816a6ba5cb46e81ae` ==origin. Ne pas utiliser le nom
red-pilot-v2 comme preuve que le code du contrôleur a changé après b307 : les
ajouts suivants sont documentation et preuves.

Sur LuciePC, métadonnées Git revérifiées à la passation : quatre checkouts
privés détachés propres sous `/home/lucied/git_workspaces/` :
source-v1=5099, pilot-v1=db833, source-v2=71fbe, pilot-v2=b307 (préfixe
`cx-hotfixs-rows-outside-graph-`). SHA complets ci-dessous. Répertoires de paquets
v1/v2 et scratch `~/.cache/rag3weaver-build/cx-hotfixs-rows-red-v2` existent.
Builds ROWS ont eu des scratchs privés sous cette racine ; aucun Cargo target
nouveau créé par cette session documentaire. Aucun effacement/recopie autorisé.
État disque/usage actuel non mesuré, ne pas inventer une place libre.

## Revues transverses, périmètres d'autres sessions

Des avis RO ont été transmis à C7, leurs suites restent chez les porteurs :

- Recherche FTS : sources84 puis union636 (5FTS identiques84/4Vertex99/66patchs=),
  ciblés8distincts/11invocations verts sur84 seulement, archive133 manifeste
  ee0e1cde90660c9e210c60e3902f1c4ae3809990fca51fd722b92761fc1d101d.
  Global pilote333 : réserve fauxsuffixe CHECK_FEATURES VERT vs8/8 enregistrée
  dans le MÊME ticket302/301, branche propre `cx-hotfixs-fts-global-ro`.
  Dernier couple relu APP636/PILOTEfc00/Qv4prepared47631f : favorable PRÉPARATION
  SEULE,15pins conformes/46CPUfakes reçus ; PG TOOL SQL RÉEL STRICTHOLD, pasGO6000.
  Contrôle F8 corrigé et transport commun/owner relus, aucun service touché.
  Rapport Recherche39/fcd2d5c3 pour le paquet et ses dépendances, pas cette page.
- ED Vertex : classifier PURE initial197 insuffisant corrigé après3P2 ; dernière
  RO totale86a1313366 favorable PURE bornée, paquet34 manifeste5593b7...60d,
 14CPU verts après3rouges,RAW54 inchangés. InventairePOSTEXEC non indépendant,
 72ignored lib consignés, ancien composéRED/scopeROUGE/ENDexpiré ; aucune
  qualification composite/GOmaster tiré du classifier. Le master99 inclut ensuite
  Vertex selon ED/C7 ; ceci n'autorise aucun rejeu ni service/cloud.
- Cœur : revues historiques des reçus à leur demande, pas ownership de C1.
  **Le chantier C1 a ensuite été bloqué par contrôle plateforme dans SA session.**
  Ne pas transférer son analyse/rejeu/handoff à hotfixs pour contourner ce refus.
  Aucun travail C1 actuel n'est autorisé par ROWS, qui est indépendant.
- Ancienne clôture documentaire ADDCOLUMN sur branche c92d557b : preuves
  historiques seulement, voir dossier16nov ; aucun nouveau runtime/SDK/master.

## Processus, verrous, services et ressources

Aucune exécution ACTIVE lancée par cette session à l'arrêt ; aucun PID/scope
OWNED restant connu. Les deux runs ROWS ont leurs reçus finals quiescents ; tous
les outils récents étaient lectures Git/fichiers et commits-push docs terminés.
Aucun contrôleur à arrêter, aucun poste lourd/mesure détenu par nous. Ceci est
l'état de NOTRE session, pas une observation globale des processus d'autrui ;
aucun procscan/health nouveau n'a été lancé pour la passation.

Aucun service/model/PG/GPU/base réservé ou modifié, aucune lib SDK posée. Les
services, trois bases en migration, PGtest et résidents GPU appartiennent à
Arbre/G/Embar/Banc selon orchestration ; ne pas les arrêter, pinguer ou nettoyer.
Ne pas prendre un ancien reçu pour une réservation encore vivante.

## Trois prochaines étapes — INACTIVES, seulement après nouvelle instruction

1. Reprise documentaire : lire cette page, le même ticket et note03 corrigée ;
   C7 doit réautoriser explicitement la reprise. Les commandes suivantes sont
   des exemples RO INACTIFS, aucun shell à lancer automatiquement :
   `git -C /home/lucied/git_workspaces/cx-hotfixs-rows-outside-graph-red-pilot-v2 status --short`
   et `git show e4ea441ac91349d20afd6eb816a6ba5cb46e81ae:docs/coeur-cpp-hotfixs-tickets/17-11-2026/03-membership-relational-contract.md`.
2. Après arbitrage NOUVEAU de C7 : fixer le protocole idempotence/vued'écriture
   de chaque mutateur et les extensions cœur rétention/OID/upgrade. Faire relire
   source/contrat et réserver les chemins explicitement AVANT code ; STOP si une
   garantie manque. Aucun branchement de mutation dérivé par simple « mêmeTx* ».
3. Après autorisation de code seulement : prépublier témoins nécessaires,
   recette isolée avec nouvelle source/pilote, budget unique et preuves, obtenir
   RO puis NOUVEAU GO natif explicite. Voir rouge avantfix/vert ensuite. Ne pas
   exécuter les anciens scripts b307/db833 ni réactiver leurs coupons/END.

Sans ces nouvelles instructions : STOP, aucune poursuite aux rappels automatiques.

## Inventaire Git complet relevé avant commit de passation

```text
cx-hotfixs-add-column-proof c92d557bd84bf5e9bdacc6cbc753139a33d6748b origin_identique=True
cx-hotfixs-fts-global-ro 7b9d9a8ecb8b01a532c132389843bde6562f0f00 origin_identique=True
cx-hotfixs-passation-week-end-20261113 0eb6ded42eac977f8c66247a49183c3c7e398f68 origin_identique=True
cx-hotfixs-recette-quarantaine 32fc739a8e0ff9e4948dfc83ee9980b46f20b635 origin_identique=True
cx-hotfixs-rows-outside-graph 03fb43f8d651a8ee64b672ce4f733b4a5c19f56c origin_identique=True
cx-hotfixs-rows-outside-graph-df2 5099a367ddf1e48db2b4065d91a61403de994571 origin_identique=True
cx-hotfixs-rows-outside-graph-fixture-fix 71fbe786fcf0b2314e72b3eb4b8484faf590d78c origin_identique=True
cx-hotfixs-rows-outside-graph-red-pilot f996c0edf1e67040e61bbbd28c27cdc702ea06d0 origin_identique=True
cx-hotfixs-rows-outside-graph-red-pilot-v2 e4ea441ac91349d20afd6eb816a6ba5cb46e81ae origin_identique=True
cx-hotfixs-temoin-quarantaine e289163bfc3055421fda47cf0a370cd46d376f17 origin_identique=True
```

Pins source/pilote natifs historiques :

```text
5099a367ddf1e48db2b4065d91a61403de994571
db833d135cbceb3319424f1619f219e64bfd1fc0
71fbe786fcf0b2314e72b3eb4b8484faf590d78c
b307f74cc21587cb982d6ef44cdb95763f992cba
2508291714002df8bf55bdde6209f4a4de9636ac
e4abc7cd05bce9167af355b20a117bb52f5136a3
269377be4b0afcb23ab3945e11b1c908f390394b
588284a313dba223f2a3906dcea3415e89182cf9
944c1685c7b2b01aab5967ec7c1ef2e054499ca8
```
