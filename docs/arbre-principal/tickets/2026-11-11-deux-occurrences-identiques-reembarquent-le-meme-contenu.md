# Deux occurrences identiques réembarquent le même contenu

- **État** : ouvert (11 novembre 2026), lot des versions de contenu 1a.
- **Famille** : produit
- **Gravité** : coût inutile, un contenu identique est embarqué une fois par occurrence.
- **Atteignable en service** : oui (deux fichiers ou branches portent le même scope).
- **Touche rag3weaver** : oui (déclaration du gabarit code, identité de contenu, ingestion/recherche).

- **Pour** : session arbre-principal, porteuse du lot 1a ; coordination recherche/ED pour les contrats communs, orchestration pour les créneaux et le tour de push.
- **Signalé par** : reprise arbre-principal demandée par Lucie via [la prise de contact orchestration](../../orchestration/11-11-2026/04-contacter-orchestration-codex.md) ; [contrats et lots](../11-11-2026/03-contrats-et-lots-des-vues.md).

## Preuve

Le témoin deux_occurrences_partagent_un_embarquement_et_gardent_leurs_positions,
suite e2e_scope_versions, branche cx-arbre-scope-version-2 (c4bce011c), est joué sur
luciepc avec la lib29a7bc52a : deux occurrences a.rs/b.rs, positions distinctes correctes,
mais le compteur reçoit deux fois le même texte de fonction au lieu d'une.
Journal hors target : ~/.cache/rag3weaver-build/cx-scope-version-etat.log ; résultat de
la suite : 2 réussis, 1 échoué. La projection générique corrigée ne partage pas à elle seule
le stockage ; aucun format de base vivante n'a été changé.

## Avancement générique du lot 1a

Checkpoints 7748fa53c et d63157d8c, poussés sur origin/cx-arbre-scope-version-3 : versions dérivées par
contenu, références indépendantes, L3 composé et compteur générique sensible 2→1. Les tests
ciblés prouvent aussi déplacement sans nouvel embarquement, retour à une ancienne version,
recette et modèles 4/6 distincts, rollback des origines/liens/états/FTS. Le schéma Scope et ses
gabarits ne sont pas basculés : son témoin initial reste rouge. Projection publique, masque avant
top-k, passe globale et migration restent ouverts ; ce ticket n'est donc pas fermé.

## Preuves du checkpoint générique

Worktree cx-wt-scope-version aux deux postes, lib 29a7bc52a, target unique
cx-target-contenu-derive sur luciepc. Diagnostic natif, sans passe globale :

- 3 unitaires derive_nodes et 7 config/schéma : cx-contenu-invariants.log.
- 8 e2e_entites_derivees : cx-contenu-derivees-vert-v4.log.
- Compteur sensible 2 au lieu de 1 sous mutation : cx-contenu-compteur-rouge.log ;
  restauration, déplacement/réutilisation et modèles 4/6 : cx-contenu-compteur-modeles.log.
- Retrait de L3 : perte du lien A ; restauration : racines, versions, hashes, file indépendante
  et FTS préservés après échec d'insertion ou de lien. cx-contenu-atomicite-rouge-v2.log,
  cx-contenu-derivees-vert-v2.log puis les suites suivantes.

Ces journaux sont dans ~/.cache/rag3weaver-build/ sur luciepc. Le témoin Scope initial reste
rouge et aucun format de base vivante n'a été changé. PostgreSQL et les autres features ne sont
pas encore rejoués sur cette implémentation.

## Association publique et première projection native

Depuis 5a068a404, une association unique EntityConfig.search_index désigne la dérivée qui sert
la cible publique. Le plan de projection est calculé depuis son from et DERIVED_FROM ; il ne
constitue pas une deuxième déclaration. Le masque passe par le filtre relationnel commun avec
les UUIDs des racines admises. La projection se fait dans finish_signal, avant fusion et pagination.
Les coordonnées ne sont réutilisées que si les champs de contenu existent sur les occurrences et
si leurs octets correspondent ; sinon la recherche refuse un remappage non déclaré.

Premier témoin natif index_derive_rend_les_occurrences_publiques_avant_pagination : 1 réussi,
0 échec, cx-projection-publique-premier.log sur luciepc. Il couvre dense/plein texte × trois modes,
deux occurrences distinctes pour une seule version et une page offset=1. Pas encore de preuve
sparse ni de saturation par candidats invisibles sur ce diff ; pas de passe globale. Scope et ses
gabarits restent inchangés. Le texte natif qui ne garantit pas ce masque est refusé explicitement.

## Renforcement en cours après 49caf64c6

- Projection neutralisée : 1 version au lieu de 2 occurrences, rouge produit dans
  cx-projection-publique-rouge.log. Restauration : les 9 cas dense/texte/creux × 3 modes et
  pagination passent leurs assertions, mais le lanceur refuse un résidu de cache de la fixture
  (cx-projection-publique-vert-v2.log, code 101). Ce lancement n'est pas un vert global.
  Ordre de destruction corrigé : Catalog et ses handles avant le TempDir. Rejeu ensuite accepté
  par le lanceur, 13/0, cx-projection-masques-vert.log, sans résidu.
- Le masque suit maintenant la relation DERIVED_FROM canonique pour calculer les UUIDs de versions,
  puis emploie les filtres d'index existants. Une recherche de relation par seuls noms d'entités
  serait ambiguë si d'autres liens existent ; un témoin ajoute précisément un AUTRE_LIEN trompeur.
- Rouge sans masque avant top-k : les trois signaux perdent le résultat visible (0 au lieu de 1),
  cx-projection-masque-rouge.log. Restauration : 13/0, incluant les 12 orphelins toujours indexés et mieux classés, budget 2, filtres A/B partageant
  une version, pour dense/creux/texte, ordre de pagination stable et refus d'une transformation d'octets sans remappage déclaré.

## Reprise de l'ingestion directe par lot

Rouge observé : ingest_entities écrit sa source par un graphe direct avant le drain des dérivées.
Un échec de lien laissait alors A.body=après, version=avant, B inchangée :
cx-projection-lot-reprise-rouge.log sur luciepc. Envelopper uniquement le second drain était insuffisant.
Le mécanisme L3 est maintenant composé par ecrire_un_lot_atomique pour les deux chemins,
drain et ingestion directe, avec mise à part des ressources indépendantes.

Vert ciblé : un_echec_du_rendu_en_lot_preserve_sources_et_references_apres_reouverture, 1/0,
cx-projection-lot-reprise-vert.log. Base privée sur fichier : échec injecté de lien, source/référence/hash
antérieurs conservés ; réouverture sans réenregistrer le schéma, mêmes valeurs ; nouvelle tentative
réussie qui change seulement A. La régression native large est en cours, pas encore un verdict global.

## Scope privé et vérification large partielle

Runtime a1af9b507 + fixture privée Scope, lib29a7, 8 features : 2 099 unitaires réussis,
0 échec, 58 ignorés ; 3 e2e_scope_versions et 14 e2e_entites_derivees réussis, code de sortie 0.
Journal cx-contenu-complet-prive.log (luciepc), SHA256
f570448be0bc00a6872273ae602f382cf0a05e17ab6797645f0511acd2db8263 ; fixture SHA256
d05eaa6443ba46b3036ccda64d8099049b61ea116e5c9ed9936b2b0ffc451d13.
Le compteur Scope partagé est vert dans cette déclaration privée : une représentation indexée,
occurrences/positions/arêtes distinctes, déplacement sans embarquement, docs et recherches Scope
préservées. Le schéma code par défaut n'est pas basculé ; l'accès depuis le vrai manifeste attend
la combinaison avec ED. Les colonnes source restent une entrée du rendu ; aucune réduction physique
globale ni mesure de stockage n'est prouvée par ces comptes. Cette passe n'est pas la passe master complète.

Prochains témoins et garanties à traiter ici : refuser un changement d'association sur des données
sans migration déclarée ; faire refléter l'index dérivé par l'état de la cible publique et son mode
automatique ; préserver les cellules lors des lectures et de la reprise du rendu ; valider l'association
avant écritures en réutilisant les contrats partagés après rebase ED. Aucun nouveau lot ni activation.

## Travail restant et responsabilités

La session arbre-principal garde ce même ticket pour la suite du lot, sans duplication :

1. Déclarer une seule association cible publique → index dérivé ; composer from/DERIVED_FROM,
   filter_indirection, Forme et réadmission dans le même L6. Recherche a relu ces primitives :
   ResolveParent après pagination ne suffit pas. Redéployer les occurrences avant pagination.
2. Appliquer le masque d'occurrences avant le top-k dense, creux et plein texte ; refuser un
   signal incapable de le garantir. Prouver absence de fuite, couverture et positions/arêtes.
3. Raccorder le schéma/gabarit Scope en test privé et rendre vert son compteur initial.
4. Passer les vérifications du lot sur son hash final, puis demander un tour à l'orchestration.
5. Préparer la migration déclarée 1b et son refus d'un ancien format, sur copies privées seulement,
   avec couverture, recherches, arêtes dans les deux sens et zéro embarquement.
6. Pour M3–M5, arbre-principal garde la recette et les résultats attendus ; coordination avec
   l'optimiseur pour copies et fenêtre exclusive, autorisée par l'orchestration. Aucune mesure
   lancée, aucun chiffre disponible. L'attente de créneau ne ferme pas ce travail.

Recevoir ce ticket ou relayer une demande n'autorise ni activation en service ni push master.
Les transferts éventuels déplacent ce fichier et ne ferment pas l'action.

## Ses filets

- generic_search_nodes.rs, BM25SearchNode : le texte natif qui ne garantit pas le masque
  d'occurrences refuse cette projection ; à lever uniquement avec sa preuve avant top-k.

- src/read_view.rs, Forme::resolve_search_target : refuse la recherche directe des versions
  internes tant que projection publique et masque ne sont pas déclarés. À réévaluer avec leur
  implémentation ; le refus ne constitue pas une livraison de recherche sur les occurrences.
- Forme::search_target_names ne propose pas ces versions internes comme cibles publiques.
- Catalog::verifier_ecriture_de_version et verifier_ecriture_de_reference préservent l'immuabilité
  et les rattachements gérés. Ces garanties doivent rester après résolution ; ce ne sont pas
  des replis temporaires à retirer avec le ticket.

## Pour le fermer

Mettre en œuvre le partage de contenu/chunks/vecteurs déclaré du lot1a, avec empreinte sur
l'entrée effectivement chunkée et version de découpage, modèle/dimension distingués. Conserver
les occurrences, positions, relations et visibilité. Prouver un seul embarquement pour le même
contenu et des contenus distincts pour docstrings/titres pertinents ; migration explicite1b sur
copies avant toute base partagée. Contrat : docs/arbre-principal/11-11-2026/03-contrats-et-lots-des-vues.md.

## Gardes d'association, avancement et dette par cellule — suite de 10c0e09ae

Porteur arbre-principal, même lot 1a, sans activation ni migration de base vivante.
Deux rouges natifs observés dans `cx-projection-gardes-rouge.log` sur luciepc :
retrait d'association accepté sur une racine peuplée ; avancement public sans vecteurs
alors que l'index physique en possède. Corrections ciblées vertes :
`cx-projection-gardes-vert.log`, 3 tests réussis (plus 2 gardes du lanceur), code retour 0.
Le changement None/Some/autre index est refusé avant migration/persistance dès que la table
est peuplée ; l'état, ses écritures et ses pannes suivent la résolution publique existante.

Rouge supplémentaire réel `cx-projection-cellules-rouge.log` : après nouvelle recette en
bêta, deux origines alpha sont reliées à une version estampillée bêta (2 liens au lieu de 0).
Correction en vérification : dette persistée globale mais sélection des racines par cellule
avant LIMIT, sans mise en file globale lors du changement de recette. L'indice de dette ne
s'efface plus après le traitement d'une seule cellule dans un catalogue multicellule.
Reste : verdict de ce correctif ; lecture explicite alpha avec catalogue bêta et file bêta ;
garde du repli scan pour les projections ; passe après composition ED et rebase.
La file actuelle appartient encore au catalogue courant : une bascule temporaire pour la
cohérence d'une lecture étrangère doit préserver cette file, jamais drainer bêta en alpha.

Verdict ciblé cellules : `cx-projection-cellules-vert.log`, 4 tests réussis, 0 échec,
code retour 0 (plus 2 gardes du lanceur). Après bêta, les deux racines alpha gardent leur
cellule ; leur dette est ensuite retrouvée et rendue en alpha. Il ne s'agit pas de la passe
globale ni encore du témoin lecture étrangère avec écritures bêta en file.

La lecture stricte alpha avec file bêta possède désormais son ticket commun distinct :
[la file d'écriture ne suit pas la cellule de lecture](2026-11-11-la-file-d-ecriture-ne-suit-pas-la-cellule-de-lecture.md).
Le témoin v2 prouve un drain bêta lors de la lecture alpha ; l'antériorité hors 1a reste
à comparer. Les invariants et le coût de la file partitionnée sont consignés dans ce ticket.

## Fin des filets de cellules et préparation SQL

La passe partielle `cx-projection-file-regressions-v2.log` est verte pour la bibliothèque
**2101/0/58 ignorés**, e2e_scope **13/0**, Scope privé **3/0**, recherche **40/0** et projection
**2/0**. La suite dérivée fait **19/1** : le nouveau test de rollback avait laissé armée
sa faute de lien pour le drain bêta ; après désarmement explicite, ce test passe seul
(`cx-projection-rollback-cellule-rouge-v3.log`, nom historique, **1/0**, rc0).
Le warning d'index source absent reste soumis à une assertion dédiée avant verdict.
La première relance bibliothèque avait échoué sur la garde du commentaire L4 (2100/1/58),
corrigée dans v2 ; ce n'était pas une régression fonctionnelle cachée.

Le témoin scan produit est rouge v2 : auto choisit scan pour une projection vide avec
FileSource. Son correctif et les unités retour d'erreur/bascule en transaction passent
dans les 2101 tests de bibliothèque. La sélection public→index rejoint la fermeture de
lecture, sans devenir une dépendance inverse de création du schéma.

**Ressource SQL privée créée par cette session** : base `cx_contenu_2191_20261111`
dans le conteneur `rag3weaver-pg` de luciepc. Script a refusé de réutiliser un nom existant
avant createdb ; propriété consignée dans `cx-contenu-pg-creation.log` hors target.
Aucun schéma de la base partagée rag3weaver_test n'est touché. Nouveau test SQL préparé
sur versions/occurrences, 12 lectures dense/BM25×modes×cellules, dette de recette et
rollback du lot. Il n'est pas encore joué ; aucune preuve PostgreSQL de 1a revendiquée.
La base privée sera supprimée à la fin du lot après contrôle de son nom/propriété.

L'assertion dédiée FTS est rouge réel : `cx-projection-rollback-cellule-rouge-v4.log`,
0/1, après désarmement de la faute. Le drain réussit mais signale l'index propre
Occurrence absent. Correctif candidat : `open_fts_handles_for` résout le stockage physique
(`Forme::resolve_storage_target`, visibilité crate), tandis que la recherche garde sa
résolution publique et l'interdiction des versions internes. Il réutilise donc la primitive
existante plutôt que de reconstruire un second résolveur ou d'ajouter un chemin Scope.
La passe partielle suivante est `cx-projection-file-regressions-v3.log`, en cours :
bibliothèque, cinq suites natives, puis le nouveau témoin dans la base SQL privée.

Passe v3 : bibliothèque 2101/0/58 ; dérivées **20/0**, cellules **13/0**, Scope privé
**3/0**, recherche **40/0**, projection **2/0**. Néanmoins rc101, garde de résidu du
journal du démon né pendant e2e_search : [ticket recherche dédié](../../recherche/tickets/2026-11-11-le-service-d-embarquement-laisse-son-journal-dans-le-tmpdir-d-e2e-search.md).
L'étape SQL chaînée n'a donc pas démarré ; elle est lancée séparément dans
`cx-contenu-pg-premier.log`. Aucun faux vert de passe ni exemption de résidu.

SQL premier rouge réel : `cx-contenu-pg-premier.log`, 0/1, sur les références de ContenuPg,
`error serializing parameter 0`. La liste d'UUID est sérialisée en tableau text[] par la
connexion commune ; nos deux requêtes la demandaient comme texte JSON. Correctif candidat
borné au dialecte : unnest($uuids::text[]) pour lire les références, ANY($uuids::text[])
pour marquer leur dette. Aucun changement de sérialiseur ni nouveau format de paramètres.

Le checkpoint suivant sauve aussi des témoins **pas encore joués** : réouverture multicellule
sans réinscription, et Scope privé avec signaux propres NONE (l'index associé garde les
signaux d'origine, pas de corpus plein texte propre redondant). Ils utilisent les primitives
existantes, sans changement du gabarit de production. Les preuves v3 ne couvrent pas encore
ces deux ajustements ni le correctif SQL. Ne pas les étiqueter VERT avant leurs journaux.

Provenance v3 figée : source exacte 965b835b8b58cb3937df26b99ed6a982e01d2f0d,
origin/cx-arbre-source-cellules-v3 (archive, pas nouveau lot). Les journaux portent HEAD10c
avec modifications ; ce snapshot les matérialise sans changement. Le checkpoint261ff54fc
ajoute les témoins de réouverture/ScopeNONE et le fix SQL, non joués par v3.
Binaire e2e SHA256 97e438e4dae5a42e6baaea3a2077ebb8fb10125231ea2212a9a4cc13bea8d120 ;
provenance complète et hash du journal v3 dans le ticket recherche de résidu lié ci-dessus.

### Chemin SQL public atteint et contrat de paramètres

Le rouge PG premier n'est pas une adaptation de fixture : le test appelle
`Catalog::ingest_entities("OccurrencePg", records)`, chemin public normal. Le lot atomique
compose le graphe de dérivation ; DeriveNode lit les références canoniques avec
`dialect.derived_references` et `QueryParam("uuids", CypherValue::List(String...))`.
La connexion PostgreSQL commune sérialise volontairement une liste scalaire comme
`Vec<String>`/text[], tandis qu'une liste de maps devient du JSON de lignes.
Le dialecte doit donc demander text[] : unnest joint chaque UUID à la racine attendue.
Les mêmes références servent au masque de `Catalog::rechercher`, et le changement de
recette passe la même liste à `mark_derived_references_pending` : ANY compare ce tableau
aux UUID d'origine. Ce correctif couvre l'ingestion et la lecture publiques sans changer
leur format de données ni leur sérialisation. Le témoin vérifie ensuite de vraies lectures
publiques sur chaque cellule : on n'escamote pas une erreur restante du masque de recherche.
Source candidate PG v2 : **1a728dcd6**, WT distant propre au lancement ; verdict à lire dans
`cx-contenu-pg-v2.log`. V3 natif reste exclusivement attribué à **965b835b8**.

PG v2 (source1a728) ne teste pas encore SQL : compilation refusée par l'extraction du
helper de réouverture dans e2e_entites_derivees (mauvais bloc coupé, variables du compteur
hors portée). Erreur de fixture corrigée, six erreurs de compilation distinctes du défaut
SQL produit. Aucun résultat PostgreSQL obtenu par ce lancement, 0 suite jouée.

Garde complémentaire du même invariant de coordonnées, en préparation : les champs de
contenu sont comparés octet pour octet avant projection ; le titre également indexé en
BM25 doit porter la même garantie. Témoin `projection_refuse_aussi_un_titre_indexe_sans_remappage` :
corps inchangé, titre rendu préfixé. Le test exige un refus nommé plutôt que des spans
attribués au titre brut. Aucun changement de format ni mapping implicite ; rouge pas encore joué.

Source **288bc80ae**, journaux `cx-contenu-pg-v3.log` et `cx-cellules-reouverture.log` :
compilation réparée ; **21 dérivées + 3 Scope privé verts**, rc0 pour le natif. Cela couvre
la réouverture multicellule sans réinscription et Scope::NONE. Le PG franchit désormais
la lecture des références (sérialisation réparée), puis échoue réellement à l'application
post-commit du tampon FTS : requête UNWIND construite dans `appliquer_le_tampon`.
Le point public atteint reste ingest_entities, pas une requête brute de la fixture.
Candidat correctif : réutiliser `dialect.select_node_ids(entity)` déjà existant dans les
deux dialectes ; même liste UUID, même retour UUID+offset, sans nouveau sérialiseur ni
branche SQL ad hoc. Le défaut n'est pas escamoté par désactivation du plein texte : le
témoin conserve Lucivy et contrôle ensuite les lectures publiques puis le rollback.

Titre : rouge réel `cx-projection-titre-rouge.log`, source288bc80ae + seul témoin ajouté.
Une requête BM25 « prefixe » rend l'occurrence au titre « titre » alors que le titre indexé
est « prefixe titre », sans remappage déclaré. La garde inclut désormais le titre dans la
liste existante des octets à préserver ; aucun nouveau format de cible ni règle spéciale Scope.
Le correctif est candidat, à rejouer avec les lectures positives existantes.

Le lancement prévu v4 sur ba879 n'a pas démarré : checkout distant refusé à cause du
seul témoin de titre encore non commité (diff archivé avant restauration). Aucun test SQL
ni natif de cette série n'est revendiqué. Avant relance, le consommateur de select_node_ids
est raccordé à son contrat réel : ID natif table:offset, row_id entier en SQL ; le parseur
InternalNodeId existant fournit l'offset natif. Une valeur physique illisible devient une
erreur nommée, pas un index plein texte silencieusement vide. Aucun nouveau format d'ID.

Série réelle v4 source **b6f78bf878f1ee4f610c5db70b5a6de6a98e4f2d** : natif22/0 +
Scope privé3/0, rc0 (`cx-cellules-reouverture-v4.log`). Titre/remappage, offsets FTS natifs,
réouverture sans réinscription et ScopeNONE couverts. PG v4 bloque après l'ingestion pendant
la lecture publique ; trois sessions privées, zéro bloqueur SQL, une idle-in-transaction.
`cx-pg-v4-piles.log` conserve le refus Yama de gdb ; aucun contournement. Seul le test PID1166384
(chemin et filtre relus) a été arrêté par SIGTERM ; zéro session privée ensuite. La série PG
reste en échec, pas de réussite déduite des nouvelles primitives SQL. Reste diagnostic public
borné, puis preuve combinée après master ED c693 et branche neuve. Aucun Search lourd relancé.

Correction du diagnostic PGv4 : le contrôle SQL après arrêt révèle seulement alpha validée
(2 racines, 1 version), aucune bêta. Le blocage est dans la deuxième ingestion, pas encore
la lecture publique. Le ticket commun [appels PostgreSQL/session épinglée](2026-11-11-les-appels-postgres-concurrents-peuvent-bloquer-la-session-epinglee.md)
porte le témoin direct borné et l'éventuel correctif ; cause et antériorité restent à prouver.
