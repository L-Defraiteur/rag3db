# Migration déclarée de l’ancre : plan et protocole 1b

Préparation documentaire du 12 novembre 2026, source relue **bc34624dc** ; socle1a
livré **08086ba36**. Aucun code, test exécuté, contenu de base réelle consulté ou
copié, service, modèle ou gabarit de production modifié pour cette préparation.

Porteur : arbre-principal ; revue orchestration et ED. Le [ticket de préparation](../tickets/2026-11-12-preparer-la-migration-de-l-ancre-sur-copies-privees.md)
reste ouvert jusqu’à relecture. Le [ticket d’implémentation](../tickets/2026-11-12-migrer-les-ancres-de-code-vers-le-contenu-partage.md)
porte les travaux suivants, **à autoriser**, pas une activation acquise.

## Résultat et frontière

Sur une fixture ancienne puis sa copie privée, passer des chunks indexés par
`Scope` à ceux d’une dérivée `ScopeContent`, sans changer les UUID de Scope,
leurs positions ni les arêtes du graphe de code. Réutiliser les valeurs des
vecteurs compatibles et ne jamais appeler un modèle pour compléter un manque.
Une entrée incompatible est refusée avant les effets de migration.

Le périmètre initial est une ancre exclusive, un catalogue, plusieurs cellules
possibles, sans écriture concurrente sur la copie. Aucune couche de tâche,
politique de GC, normalisation/remappage ou mesure M3–M5 n’entre dans1b.
L’ancien index est conservé pour le retour ; sa suppression sera une décision
séparée après validation, sans confondre rétention et migration.

Trois versions indépendantes doivent être nommées dans chaque rapport : format
moteur/lib épinglée ; schéma interne `scope::SCHEMA_VERSION = 8` ; format applicatif
de l’ancre et recette. Le nom proposé **code-scope-content-v1** désigne le dernier,
pas un bump automatique du schéma interne. Un nouveau champ de manifeste ou une
clé méta ne sont pas décrétés ici comme API déjà disponible.

## Primitives relues et limites constatées dans le code

Références relatives à `extension/rag3weaver/` ; les constats sont statiques,
sauf les preuves1a déjà publiées dans les tickets clos.

| Besoin | Existant à composer | Limite à couvrir avant promesse |
|---|---|---|
| Décrire le schéma réel | `src/code.rs::schema_configs`, `src/backend_schema.rs::SchemaFacts`, `EntityConfig` | Le fournisseur code produit encore Scope indexé directement ; éviter une seconde copie de sa définition. |
| Identifier le contenu | `derive_nodes::{render_fields, derived_content_uuid, render_inputs_hash}`, `DerivedIdentity::Content` | L’UUID de version dépend des rendus, rôles, recette et cellule ; ne pas le reconstruire dans un script de migration. |
| Ordonnancer et observer | `src/dataflow/migrations.rs::MigrationRunner`, `_DataflowMigration`, graphes `.mmd`, `ValidateNode`, checkpoints | La recherche des appels ne trouve pas de raccord produit du runner dans `src/bin`/backend ; présence de l’API ≠ commande accessible. |
| Refuser un plan modifié | Checksum rangée par `migration_record`, `runtime.rs` vérifie `graph_hash` à la reprise | `pending` saute un état applied sans comparer la checksum ; l’empreinte de graphe ne remplace pas l’identité du fichier appliqué. |
| Reprendre | `execute_with_checkpoint`, statut/états de nœuds sauvegardés | `migration_execute_graph` n’injecte actuellement que conn ; registre, catalogue, dialecte et opérations typées doivent être reliés. Pas de reprise publique du runner annoncée sans témoin. |
| Annuler | `MigrationRunner::rollback`, contextes d’undo, `CypherNode.capture_query` | `can_undo` dit qu’un contexte existe, pas qu’un schéma/index et ses blobs sont réversibles. Les migrations actuelles ne forment pas une transaction globale. |
| Garder les écritures cohérentes | L3, tampon FTS, fichiers par cellule, publication de dette du1a | DDL/index hors transaction ; une migration longue se compose en lots et point de publication, pas en un unique BEGIN. |
| Relocaliser les vecteurs | `catalog/aside.rs::KnownVectors`, `EntityRecord::vectors`, `embedding_storage::VectorStorage` | KnownVectors est indexé par UUID de chunk et un slug courant ; ce n’est pas un transfert multi-modèle complet. |
| Conserver le creux | Handles `sparse_vector::handle::SparseHandle` consommés par le catalogue | Le chemin aside/EmbedNode indique que le creux n’y est pas relu et que dual peut recalculer. Aucun export sans calcul prouvé à cette étape. |
| Refuser la bascule implicite | `Catalog::register_entity` refuse searchIndex sur table peuplée et changement d’identité dérivée | Ne pas neutraliser ces gardes. La publication explicitement migrée doit vérifier ses propres préconditions. |

Autres bornes : `MigrationRunner` utilise actuellement des versions numériques
et un verrou TTL ; `migration_acquire_lock` fait lecture puis écriture. Ne pas
en déduire une exclusion distribuée ou une reprise sûre après expiration. Le
premier parcours doit imposer l’exclusivité de sa copie et refuser un second
exécutant ; sa preuve précédera toute extension de concurrence.

`Catalog::initialize` fait déjà des opérations de schéma et des réparations
d’index ; `PreparedBackend::open_with_reason` peut joindre des services avant
l’enregistrement final des entités. Le refus de format de la nouvelle cible doit
précéder ces effets et les connexions de modèles. Un échec tardif de
`register_entity` seul ne satisfait pas « ancien format refusé sans mutation ».

## Déclaration cible du code

Partir des faits fournis par `code::schema_configs(default_scope_chunking())`,
puis appliquer une transformation déclarée commune au fournisseur et au plan :

- `Scope` garde tous ses champs, hashsafe `[key]`, returnFields, sourceLines,
  valeurs, métadonnées et relations. Il devient occurrence : `signals: []`,
  `chunked: false`, `searchIndex: ScopeContent`.
- `ScopeContent` dérive de Scope en identité content/format code-scope-content-v1,
  sans gather. Comme la fixture1a `tests/e2e_scope_versions.rs`, rendre sans
  préfixe ni normalisation name, signature, content, docstring, scope_type,
  language ; copier les définitions de champs, les rôles et la recette depuis
  les faits de Scope. Les signaux viennent du profil source admis.
- `ScopeContent_DERIVED_FROM` relie chaque version à toutes ses occurrences de
  **la même cellule**, avec l’état de rendu propre au lien. Les chunks et leurs
  liens appartiennent à ScopeContent. Pas de modification du schéma des parseurs.
- `File`, `Library`, `Package`, `DEFINES`, `CONSUMES`, etc. gardent leurs UUID,
  extrémités, propriétés et occurrences. Le fichier et les coordonnées restent
  des propriétés d’occurrence, jamais de la version partagée.

Les noms/configurations ci-dessus sont une proposition de déclaration, pas un
backend JSON complet ou un gabarit activé. Les colonnes d’entrée de Scope restent
présentes ; aucun gain de taille totale de base n’est promis.

## Admission, plan et protocole de migration proposé

Le plan doit utiliser le journal et les graphes existants, sans deuxième moteur
de migration. Les états proposés ci-dessous doivent être raccordés au journal
existant : l’état « en cours » avant premiers effets est notamment à compléter,
pas à supposer présent dans `MigrationState`.

1. **Préflight explicite.** Lire uniquement la copie privée autorisée : configs
   persistées, tables/colonnes/relations, format et recette, cellules, modèles et
   dimensions, présence/fraîcheur des données indexées. Vérifier la lib et le
   format moteur séparément. Inconnu, état partiel ambigu, index sans source
   complète, modèle incompatible ou recette différente : refus nommé, aucun
   repli sur un réembarquement. Pas d’`index confirm` comme porte de migration.
   Ce préflight futur autorise la lecture de métadonnées de la copie : connexion
   compatible en lecture, sans Catalog.initialize, DDL, réparation ni connexion
   de modèle. Cela ne permet pas à la présente préparation documentaire de lire
   ou copier une base réelle.
2. **Plan stable.** Sortir le descripteur source/cible, empreintes des configs et
   du graphe/fichier, ressources touchées, besoins de rétention et validations.
   Le dry-run public du runner appelle pending et lit son catalogue ; pour le
   plan purement statique, réutiliser parse/build de graphe sans prétendre zéro
   I/O au runner public. Une migration applied au checksum différent est refusée.
3. **Préparer et journaler.** Verrou/identité d’exécution, état en cours et
   empreintes durables avant premiers effets. Préparer les tables/index cibles
   via SchemaFacts et les contrats du catalogue, hors lots de données. Conserver
   les configs et index anciens. Pendant cet état, aucune ouverture normale de
   la copie pour recherche/indexation ; reprise explicite ou annulation seulement.
4. **Transformer par lots déterministes.** Pour chaque cellule, grouper les
   occurrences selon l’identité1a, rendre avec la primitive existante, produire
   versions/chunks/références et transférer les valeurs indexées compatibles.
   Checkpoint et couverture par lot ; répétition d’un lot sans duplication. Les
   écritures/reprises publient honnêtement leur dette. La source reste immuable.
5. **Valider avant publication.** Vérifier couverture et unicité des liens,
   octets, modèles, marqueurs, absence de fuite intercellule et relations dans
   les deux sens. Construire/synchroniser les index cibles depuis les valeurs
   déjà stockées : pas d’appel aux modèles. Toute faute laisse la cible non
   publiée et la source ancienne intacte, avec rapport explicite.
6. **Publier.** Contrôler à nouveau les empreintes source, tous les lots et la
   génération du plan ; publier ensemble la transition de config Scope et le
   format applicatif actif, sans DDL ni calcul à ce point. Réutiliser validation
   et ordre communs ; une capacité de transition de schéma validée par migration
   est à fournir si elle manque. Aucun flag « ignore guards » ou faux comptage
   de table vide. Une mort autour de ce point donne ancien actif OU nouveau
   validé, jamais une configuration cherchant un corpus partiellement migré.
7. **Rouvrir et conclure.** Une nouvelle ouverture normale charge le nouveau
   format sans rattrapage documentaire caché ; exécuter les lectures prévues et
   fermer le journal appliqué. L’ouverture demandant le nouveau format sur
   l’ancienne base non migrée reste refusée. Aucun hôte de Lucie n’est basculé.

La transformation n’autorise pas la publication anticipée de la dérivée dans un
catalogue servi : un drain ordinaire pourrait alors créer ou embarquer les
versions manquantes. L’exclusivité et le refus des ouvertures en cours doivent
être effectifs dès la préparation.

### Transport des chunks et « zéro embarquement »

Un nouveau parent implique de nouveaux UUID de chunks (`chunk_uuid(parent,
field,index)` dans `record_nodes.rs`). Copier KnownVectors sans remapper ses clés
ne suffit donc pas. Le plan doit établir pour chaque destination une liaison
vers un chunk source compatible : cellule, champ/rôle, recette, indice et
coordonnées de découpe, octets réellement présentés au modèle (titre compris),
texte/hash, nom du modèle, dimension et valeurs. Des valeurs manquantes ou deux
valeurs différentes pour une même liaison sont un refus, pas un choix arbitraire.

Réutiliser VectorStorage pour legacy/suffixed, les marqueurs par modèle et les
opérations de stockage/EntityRecord existantes. Un adaptateur de transfert
validé peut remapper source→destination ; il doit porter l’invariant « valeurs
connues uniquement, aucune inférence », servir les entités déclarées et ne pas
connaître Scope. Coût : validation des correspondances, lecture/écriture bornée
et reconstruction des index physiques ; pas un cache global de textes.

Pour le premier profil candidat : format moteur admis par la lib épinglée,
schéma interne8, ancien Scope BM25+dense, recette identique,
modèles denses complets, aucune dette sparse/dual. Cela couvre le défaut du
gabarit code par défaut sans prétendre couvrir tous les schémas personnalisés.
Une conversion de format moteur est un prérequis distinct ; ce plan ne
l’effectue pas silencieusement. Si sparse/dual est présent, **refus avant effets tant que le transfert sans
calcul n’est pas prouvé**. Le modèle existant ne sera pas invoqué pour masquer
l’absence d’export. Le test de ce refus appartient au premier parcours ; un
profil sparse migrable demande sa preuve avant d’être accepté, sur le même ticket.

BM25 se reconstruit depuis les octets transférés ; ne pas copier des offsets
physiques anciens dans les nouveaux index. HNSW se reconstruit depuis les
vecteurs copiés. Les magasins par cellule ne sont jamais fusionnés.

### Reprise et annulation

Reprendre le même plan et les mêmes empreintes ; les checkpoints identifient les
lots, les fichiers de migration et les validations déjà faites. Un plan modifié
est refusé même si une étape paraît finie. Pas de succès déduit de l’absence de
travail ou d’une checksum simplement stockée.

Avant publication, annuler retire seulement les ressources dont la migration
prouve la propriété et conserve l’ancienne configuration/index. Après
publication, tant qu’aucune nouvelle écriture n’a modifié l’ancre, revenir à la
configuration/index source conservés est le parcours minimal proposé. Après
nouvelles écritures, ce rollback simple est refusé : pas de restauration qui
perde ces écritures. Le retour opérationnel sur copie gelée/sauvegarde reste
explicite ; ne pas annoncer undo général de nouveaux index via CypherNode.

## Témoins à poser avant le code de transformation

Fixtures synthétiques créées par les déclarations anciennes actuelles, aucune
base réelle. Exemple borné : six Scope en Alpha (A/B identiques ; C corps
modifié ; D docstring différente ; E nom différent ; F sans la docstring de A),
deux Scope en Beta aux rendus identiques à A, avec leurs propres clés
d’occurrence. Avec ces champs rendus : cinq classes en
Alpha, une en Beta, huit occurrences et six versions attendues. Deux modèles
denses de noms distincts/dimensions4 et6 ; un profil séparé porte sparse pour le
refus d’admission. Les tailles de chunks se fixent dans la fixture ; comparer
leur couverture sémantique, pas imposer un compte global avant cette définition.

| ID | Témoin et verdict attendu |
|---|---|
| T1 | Cible nouvelle sur format ancien : refus avant modification de config/table et avant connexion de modèle. Ancien manifeste sur sa fixture reste lisible. |
| T2 | Préflight/dry-run : plan déterministe ; format inconnu, recette/dimension manquante et sparse non transportable refusés, aucun effet. |
| T3 | Migration nominale des huit occurrences : six versions selon la fixture, mêmes UUID/positions/données publiques, tous les liens canoniques et aucune référence cross-cellule. |
| T4 | Inventaire exact des arêtes de code, propriétés et multiplicités avant/après ; contrôle des deux sens via les outils du dépôt (`tools/check_rel_directions`, ou extraction équivalente disponible sur le backend). |
| T5 | Tous les vecteurs denses4/6 et leurs marqueurs compatibles copiés ; aucune fusion de modèles. Compteurs à zéro pendant la migration, toute inférence fait échouer le témoin. |
| T6 | Retour d’un modèle absent, vecteur incohérent ou chunk sans couverture : refus sans publication, source conservée. |
| T7 | BM25/dense et filtre de champ/cellule, trois modes de résultat ; UUID, positions, provenance, pagination et admission attendus. Ancien et nouveau graphe déclaré compris. |
| T8 | Réouverture Strict après migration : aucune ingestion documentaire différée ; pas de nouvelle dette masquée par un résultat apparemment complet. |
| T9 | Faute au milieu d’un lot : pas de demi-lot publié, autres cellules intactes, relance idempotente sans duplication ni modèle appelé. |
| T10 | Mort aux frontières journal/préparation, copie de chunks et publication : reprise du même plan ou refus explicite ; jamais ouverture normale d’un état partiel. |
| T11 | Annulation avant publication et retour après publication sans écriture nouvelle ; source exacte retrouvée. Après écriture nouvelle, retour simple refusé. |
| T12 | Déjà appliquée identique : aucun travail et zéro inférence ; fichier/checksum modifié, second exécutant ou état incomplet incohérent : refus nommé. |

Le compteur est remis à zéro **après construction de la source synthétique**.
« Zéro » pendant la migration concerne tous les appels de modèles. Pendant les
recherches de validation, les encodages de requête sont tracés séparément des
réembarquements de documents : utiliser des requêtes connues, distinctes des
textes de corpus, et refuser tout appel documentaire ou inconnu. Ne pas annoncer
zéro appel total pour une recherche dense qui calcule son vecteur de requête.

**Équivalence à relire.** La déduplication change le corpus BM25 et potentiellement
ses statistiques : ne pas promettre scores et rangs arbitraires identiques.
Les témoins imposent couverture des occurrences, filtres/admission, coordonnées,
provenance et ordre déterministe des égalités, avec résultats attendus sur les
requêtes de la fixture. L’orchestration accepte pour ce profil la variation de pertinence BM25 due au
corpus dédupliqué (revue du12nov), avec ces garanties à prouver. Ce n’est pas une
équivalence complète bit pour bit. Un autre besoin de classement exact exigerait
son contrat distinct ; aucun résultat M3–M5 n’est ici invoqué.

## Premier correctif reviewable et budget proposés

Premier patch **après autorisation nouvelle** : admission du format applicatif
et plan de migration explicite via le mécanisme existant, avec reconnaissance
old/cible/en-cours et rapport de préflight. Témoins rouges T1/T2/T12 adaptés au
point d’entrée réel, refus avant effets et checksum ; aucune copie de vecteur,
aucune activation du gabarit code à cette première étape. Noms des champs/clé
applicatifs et raccord public du runner sont à faire relire par ED avant ce code.

Ensuite, sur le même ticket : transfert générique des chunks connus et T3–T6,
publication/reprise/annulation et T8–T11, puis lectures T7 et fermeture du parcours.
Chaque morceau reste reviewable avec ses rouges, sans prétendre livrer une
migration partielle utilisable. Les fichiers/nœuds et leur autorisation seront
nommés dans le ticket avant exécution ; pas de migration en SQL brut ignorant
entity_config, pas de second moteur de checkpoint.

Budget de validation proposé, non consommé : douze scénarios nommés ci-dessus,
deux modèles jouets, deux cellules, trois modes de résultat et quatre points de
faute/mort déterministes, petits corpus sans GPU. Un WT isolé et un target unique
sur luciepc, plafond32Go/-j8, sous poste lourd ; lib épinglée explicitement selon
la future fenêtre. Aucun target créé pour ce document.

Pour une future passe : features5, lib entière, navigateur séparé si backend/
gabarits changent, suites dérivées/Scope/code/code_sync/projection/search/PG,
transactions et lectures selon les fichiers touchés, plus la suite de migration
à créer. Une suite par appel, compte attendu lu, zéro SAUTÉ, PG privée. Proposition
de borne globale60min avec échéances par étape et arrêt de l’arbre propre ;
pas une estimation mesurée. Les copies de fixtures et sauvegardes restent hors
sources/target, propriété et nettoyage soumis aux gardes du dépôt.

## Décisions à relire sur ce plan

Revue orchestration du12nov : séparation des trois formats, publication après
validation, profil initial BM25+dense et refus sparse avant effets acceptés.
Variation de pertinence BM25 admise avec les garanties nommées, sans égalité
arbitraire des scores/rangs. Cela ne livre pas un profil sparse migrable.

ED relit encore la frontière d’API runner/admission/checksum et le premier
correctif. Le ticket de préparation reste ouvert et aucun code n’est encore
autorisé. L’implémentation future reste suivie par son ticket distinct ; cette
revue ne permet pas l’accès à une base réelle ni son activation.
