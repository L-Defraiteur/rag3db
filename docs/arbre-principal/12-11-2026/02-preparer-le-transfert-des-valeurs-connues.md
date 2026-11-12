# Transfert générique de valeurs connues — tranche à relire

**État courant, relu sur master41497ba4e :** aperçu intégré sur3b2eab7ec,
passe verte et ressources retirées. La préparation initiale ci-dessous est
historique ; la section « Prochain pas » en fin de cette même page fixe la suite.
Ce chantier avance la migration1b ; les couches2a, leur résolution et leurs
écritures ne sont pas livrées par l'aperçu. Pas de nouvelle vision parallèle.

Préparation sur master **6e5f9458c**, 12 novembre 2026. Porteur : arbre-principal ;
revue commune : orchestration. [Ticket canonique](../tickets/2026-11-12-migrer-les-ancres-de-code-vers-le-contenu-partage.md),
[répartition parallèle](../../tickets/closed/2026-11-12-paralleliser-les-suites-autorisees-sans-chevauchement.md).
Aucun Rust modifié, Cargo lancé, transfert exécuté, target, base ou service créé.
Les signatures ci-dessous sont proposées, pas disponibles dans le produit.

## Résultat borné et dépendances

La première tranche de code proposée construit en mémoire un **aperçu déterministe
et non exécutable** du transfert : occurrences→versions→chunks et correspondances
de valeurs connues par cellule et modèle. Elle vérifie la couverture complète,
sans ouvrir de base, écrire de ligne ou disposer d'un embedder. Elle ne prétend
pas livrer T3–T6 sur stockage ni T7–T12 de publication/reprise.

Le préflight livré et le socle1a restent les frontières : ResolvedMigrationPlan
fixe source/cible, variables et faits ; DerivedIdentity::Content définit la version ;
MigrationRunner est l'unique orchestrateur futur. Pas de second runner, journal,
mode d'ouverture permissif ou chemin d'ingestion qui recalcule un manque.

**Deux prérequis du transfert effectif restent absents** : identité durable de
l'instance de stockage source et génération établie de ses données. Le descripteur
actuel n'a pas d'identifiant d'instance ; l'inspection peut rendre Unversioned.
Ni chemin, hash de configs, empreinte d'inventaire, checkpoint ou UUID des lignes
ne remplace ces deux preuves. Les fixtures peuvent déclarer explicitement une
identité/génération synthétique ; cela ne crée aucun producteur de métadonnées
pour une ancienne base. Tout passage vers une future écriture refusera leur absence
avant effets. Leur établissement sous exclusivité réelle et journal durable
appartient à une tranche distincte, avant tout transfert ou apply.

## Primitives relues et décisions d'API

| Besoin | Primitive existante et décision |
|---|---|
| Rendu et identité de version | derive_nodes::{render_context, render_fields, render_inputs_hash, derived_content_uuid}, EntityConfig ; réutiliser les fonctions, aucune formule parallèle de Scope. |
| Découpe et identité de chunk | Chunker::chunk, ChunkerConfig et uuid::chunk_uuid ; les conventions de record_nodes::ChunkNode portent champs, offsets et liens. Si une extraction commune est nécessaire, proposer son diff avant de modifier record_nodes.rs partagé. |
| Modèles et stockage | EmbeddingModelEntry, StorageKind, VectorStorage::resolve : nom complet+dimension qualifient le modèle ; le slug ne suffit pas (collisions possibles). Résoudre séparément source et cible, legacy ou suffixed. |
| Réutilisation actuelle | catalog/aside::KnownVectors : un slug et map par ancien UUID. Ne pas élargir silencieusement sa sémantique ni injecter dans EmbedNode pour obtenir un prétendu transfert sans calcul. |
| Porteur de valeurs | records::{EntityRecord, RecordVectors, DenseVector} : une seule valeur dense par RecordVectors actuellement. L'aperçu porte une liaison par modèle ; ne pas lui faire croire qu'un EntityRecord contient tous les modèles. Adaptateur multi-modèle d'écriture à fournir ultérieurement. |
| Recherche/occurrences | Socle1a et catalog::resolve_to_source_entities ; conserver les liens canoniques et la projection avant réduction des résultats. Ne pas recopier cette politique dans le transport. |

Surface proposée dans `src/dataflow/migrations/transfer.rs`, enfant du runner
existant, avec accès `pub(crate)` initial :

```rust
// Esquisse : noms et signatures à arrêter en revue, aucun code livré ici.
fn preview_known_transfer(
    fixed_plan: &ResolvedMigrationPlan,
    inventory: &TransferInventory,
    target: &TransferRecipe,
) -> Result<KnownTransferPreview, Vec<TransferRefusal>>;
```

TransferInventory décrit des faits fournis, sans prétendre les avoir lus d'une
base. TransferRecipe compose les EntityConfig/faits résolus réels ; les fonctions
communes de rendu/UUID/découpe produisent les destinations. KnownTransferPreview
a des champs privés, accesseurs de rapport et correspondances déterministes ;
**aucune méthode execute/apply et aucune conversion en capacité d'écriture**.
Il référence l'identité du plan fixe et les empreintes des inventaires, sans
ajouter d'identité exécutable au journal. Les refus sont typés et localisés
(cellule, entité/chunk, modèle, motif), triés pour un rapport reproductible.

Le coût de cette primitive est la conservation des métadonnées de couverture et
la comparaison des vecteurs candidats. Elle unifie les changements de parent
indexé pour toute entité déclarée ; aucune branche spéciale Scope. La fixture
reste petite et entièrement en mémoire. Streaming, limites de volume des bases
réelles et lecteur RO cohérent sont des prérequis de la tranche de transport,
pas des promesses de cet aperçu.

## Inventaire requis, sans inventer de données

- Origine : identité d'instance et génération explicitement présentes ou absentes,
  descripteurs source/cible, empreinte des configs/faits/recette, identité du plan.
  L'empreinte de l'inventaire couvre ses entrées, pas une génération durable.
- Par cellule : occurrences avec UUID, valeurs rendues, provenance et propriétés
  d'occurrence (fichier, positions), états des liens, masques/visibilité déclarés.
  Les coordonnées d'occurrence restent sur l'occurrence ; les offsets de chunks
  sont relatifs aux champs rendus de leur version. Même contenu dans deux cellules
  ne donne ni liens ni magasin de vecteurs partagés entre elles.
- Relations : inventaire canonique des extrémités, types, propriétés et
  multiplicités, avec sens ; conserver les deux représentations attendues quand
  le moteur les matérialise. Arêtes du code inchangées ; seuls liens déclarés
  version→occurrence et chunk→version sont ajoutés à l'aperçu cible.
- Chunks : parent/champ/rôle/indice, texte et titre exacts, offsets complets/core,
  lignes et content_offset. Vérifier bornes UTF-8, cohérence avec les champs,
  couverture et unicité ; pas seulement l'égalité des comptes ou des hashes.
- Dense : registre complet des modèles, nom/dimension/stockage source/cible,
  colonnes et marqueurs résolus, valeurs brutes par chunk et par modèle,
  disponibilité/dettes. Dimension exacte, valeurs finies ; aucun vecteur absent,
  partiel, périmé ou contradictoire n'est remplacé par un calcul.
- Entrée du modèle : octets effectivement désignés par le chemin d'embarquement
  qualifié, pas une chaîne reconstruite au jugé. EmbedNode utilise text_field
  (`_text` par défaut), pas automatiquement `_title`. Le titre reste à comparer
  comme donnée/recette ; si un chemin a incorporé le titre ou une autre entrée,
  prouver cette recette et cette entrée. Marqueur `_text_hash` seul ne prouve pas
  une autre entrée historique. Source ambiguë : refus InputProvenanceUnavailable.
- Sparse : croiser signaux persistés, colonnes/marqueurs, handles/magasins et
  dette. Tout sparse/dual à transférer ou contradiction est refusé avant effets.
  Une déclaration models.sparse servant seulement au surlignage ne suffit pas à
  classer le profil sparse ; ne pas faire un refus sur ce seul champ.

## Correspondances et invariants

1. Regrouper les occurrences par **identité1a calculée**, dans chaque cellule,
   sur tous les rendus/rôles et la recette ; ne pas dédupliquer au seul corps.
   Garder huit UUID d'occurrence dans la fixture canonique, six versions attendues.
2. Rejouer uniquement le rendu et la découpe déterministes. Construire les nouveaux
   UUID de chunks avec chunk_uuid(nouveau_parent, champ, indice). Une liaison
   source→destination exige mêmes octets d'entrée modèle, recette, champ/rôle,
   offsets/indice, modèle complet et dimension ; pas une copie de map UUID.
3. Plusieurs sources compatibles peuvent couvrir une destination, mais leurs
   valeurs pour un modèle doivent être identiques **bit à bit** (`f32::to_bits`),
   sans tolérance numérique ni « premier trouvé ». Même slug avec noms différents,
   doublon divergent, marqueur périmé ou absence de couverture : refus nommé.
   Le représentant de rapport est choisi par ordre canonique, tous les témoins
   de couverture sont conservés. L'ordre d'entrée n'influence pas le résultat.
4. Garder une liaison distincte par modèle. Les dimensions4 et6 ne sont jamais
   mélangées ; les valeurs et marqueurs ne deviennent disponibles qu'après future
   écriture et validation réelles, pas parce que l'aperçu les énumère.
5. Texte BM25 exact préservé. Les offsets physiques et structures FTS/HNSW ne se
   copient pas ; leur reconstruction future doit utiliser texte/vecteurs connus.
   La déduplication change le corpus BM25 : scores/rangs globaux identiques ne sont
   pas un invariant ; couverture, filtres, provenance, positions et égalités le sont.
6. Aucun état persistant ni index déclaré prêt ne sort de cette tranche. Les
   lectures masquées doivent rester une politique commune ; l'inventaire/aperçu
   conserve les occurrences et masques nécessaires sans prétendre exercer T7.

## Témoins proposés avant correction et budget

Fixture canonique du plan1b : Alpha A/B identiques, C corps différent, D docstring
différente, E nom différent, F sans docstring ; Beta deux rendus identiques à A.
Huit occurrences/six versions, graphes de code avec propriétés et multiplicités,
deux modèles synthétiques4/6 ; vecteurs littéraux, aucun serveur ni modèle.
Un deuxième nom d'entité déclaré autre que Scope vérifie la généricité.

| Cas | Verdict à exiger |
|---|---|
| Nominal, permutations, deux cellules | Mêmes aperçus/liaisons/empreintes, huit occurrences/six versions, aucun lien cross-cellule, octets/positions/provenance préservés. |
| Texte, titre, rôle, recette ou entrée modèle distincte | Classes distinctes ou refus de compatibilité, jamais réutilisation par seul hash de corps. |
| Deux modèles, legacy/suffixed, collision de slug | Résolution source/cible correcte, couverture indépendante ; collision de noms refusée. |
| Vecteur absent/dimension fausse/NaN/infini/marqueur périmé | Refus localisé, aucun succès partiel ni disponibilité inventée. |
| Deux sources d'une destination, valeurs divergentes | Refus bit à bit ; égalité exacte acceptée, ordre d'entrée sans effet. |
| Identité source ou génération absente/étrangère | Refus préalable à toute future opération ; Unversioned reste explicite. |
| Sparse/dual/dette/contradiction ; lecteur de surlignage seul | Refus du transfert sparse ; le lecteur seul n'est pas un faux positif. |
| Masques, relations et couverture manquante | Conservation exacte de l'inventaire valide ; incohérence nommée, pas de remplacement par un compte global. |

Proposition de premier diff : module pur `migrations/transfer.rs`, déclaration
`mod transfer` dans migrations.rs et tests unitaires voisins. Aucun backend.rs,
CLI, gabarit, stockage, runner supplémentaire ou intégration de service. Les
factories d'aperçu ne reçoivent ni DbConnection ni Embedder ; ce fait limite
structurellement les effets, mais ne remplace pas les futurs témoins d'exécution.

Si les conventions de ChunkNode ne sont pas réutilisables sans extraction,
signaler d'abord le petit contrat commun à orchestration ; ne pas réserver ni
modifier record_nodes.rs en parallèle d'ED sans accord sur le propriétaire.
Préparation actuelle : zéro lourd. Après revue de périmètre, proposition d'un
seul ciblé sur luciepc, target unique par session plafonné32Go, j8/Mem32, borne600s
après poste lourd ; jeu minimal à choisir d'après les dépendances effectives.
Premier rouge : diagnostic et arrêt, aucun rejeu inchangé. Passe complète et
budget d'intégration à convenir après le diff et les compteurs réellement lus.

La tranche suivante devra fournir inventaire RO lié au stockage sous exclusivité,
identité/génération durables, journal running réel, adaptateur multi-modèle,
écritures/reconstruction/checkpoints/validation et publication atomique avant
apply. L'aperçu pur ne permet de sauter aucun de ces prérequis.

## Diff pur autorisé et nuance d'identité — 12 novembre

C7 autorise module/déclaration/tests, puis ciblé600s/j8/Mem32 sous lourd après
publication de la source et argv. L'API prend explicitement expected_source_identity ;
absence d'attendu et absence de déclaré sont deux refus distincts. Le plan actuel
ne produit pas cet attendu ; égalité de deux déclarations n'est jamais une preuve
issue du stockage. KnownTransferPreview porte une empreinte de rapport distincte
de la génération et de l'identité du plan, sans exécution ni désérialisation.

Module public de rapport pour éviter une API interne inutilisée en production,
avec types d'entrée explicites et sortie à champs privés ; aucun point CLI ajouté.
Rendus inline seulement : render_fields peut sinon relire un gabarit utilisateur,
ce qui casserait la pureté promise. Chunker/UUID/rendus communs réutilisés ; aucun
changement de record_nodes, catalog, backend ou RecordVectors multi-modèle.

## Prochain pas — lier les faits déclarés au plan sans qualifier le stockage

Préparation demandée par orchestration après livraison de l'aperçu. Source de
lecture41497ba4e ; worktree neuf cx-wt-qualification-faits, aucune modification
Rust, aucun Cargo/GPU, accès base ou nouveau service. Même ticket migration.

### Le défaut précis et ce qui est déjà décidé

ResolvedMigrationPlan reçoit aujourd'hui config_fingerprint et
schema_facts_fingerprint comme des chaînes. Il fixe ces entrées, mais ne produit
pas leurs valeurs depuis les configurations examinées par l'aperçu. Modifier
une config tout en conservant les étiquettes reste donc possible ; le rapport
intégré annonce honnêtement UnqualifiedConfigAndSchemaFacts. Le prochain pas
corrige cette liaison **entre faits fournis et plan**, sans prétendre observer
l'identité ou la génération d'une base.

Restent acquis : contenu distinct d'occurrence ; génération distincte d'un hash
de schéma ; identité logique distincte d'un chemin de copie ; inputs du plan
figés, comparaison source avant opération et cible attendue après publication ;
rejeu applied identique seulement ; exclusivité réelle, running durable avant
effets et publication atomique ; sparse refusé hors transfert prouvé. Ces règles
ne sont pas des questions à reposer à Lucie. Rien ne change dans les masques,
positions, relations, modèles, découpe, rétention ou admission déjà convenus.

### Faits existants à composer, pas à reconstruire à côté

| Existant relu | Usage et limite du prochain pas |
|---|---|
| backend_schema::SchemaFacts::{compose,registration_order} | Faits complets manifeste+fournisseur et validation des dépendances. Réutiliser cette composition/validation ; pas de second fournisseur ni de copie des règles. |
| PreparedBackend::migration_plan | Produit target_entities/target_relations après surcharges index_signals ; prendre ces faits effectifs, pas uniquement le nom du fournisseur ou les champs avant surcharge. |
| StorageInspection::entity_configs / entity_config_fingerprint | Entités persistées seulement ; ni relations, ni données, ni génération. Le périmètre de cette empreinte existante reste inchangé. |
| Catalog::{persist_relation,load_relations} | Les RelationDef existent déjà sous les clés metadata relation:<nom>. Leur format est disponible : ne pas inventer un magasin de relations. Le lecteur d'inspection actuel ne les restitue pas ; ne pas traiter cette absence comme un graphe vide observé. |
| plan::{MigrationPlanInputs,ResolvedMigrationPlan}, MigrationRunner | Un constructeur commun des entrées et un vérificateur utilisant le même producteur ; conserver le runner et l'identité figée, pas un deuxième journal/exécuteur. |
| transfer::encoded, hash::content_hash, EntityConfig/RelationDef | Mécanisme de canonicalisation/hachage déjà présent ; définir une seule projection sémantique de faits pour la construction et la comparaison. Ne pas rebaptiser l'empreinte privée de rapport « contrat commun ». |

### Tranche A proposée : constructeur et vérificateur purs des faits

Entrée : ensembles **complets explicitement fournis** source et cible, chacun
avec toutes les EntityConfig et RelationDef, faits effectifs du fournisseur,
recette déclarée et définitions effectivement résolues nécessaires à cette
recette, registres des modèles/dimensions/stockages ; variables et étapes déjà
résolues du runner. Les données d'occurrences/vecteurs restent l'inventaire de
l'aperçu, pas une génération déduite de ces faits.

Un ensemble d'entités sans les relations requises, un fournisseur nommé sans ses
faits ou une recette référencée sans ses octets résolus est **incomplet**.
L'appelant doit distinguer « ensemble déclaré complet et réellement vide » de
« information absente ». Aucun remplissage par les defaults du manifeste courant
pour réparer une observation source manquante. Aucun accès fichier ou résolution
de template externe depuis la fonction pure ; les octets résolus sont fournis,
ou le contrôle refuse UnresolvedRecipe.

Surface indicative, noms et emplacement sont des choix d'implémentation :

```rust
// Proposition pour un prochain diff, pas API déjà disponible.
let facts = DeclaredMigrationFacts::from_complete_inputs(source, target)?;
let inputs = facts.bind_inputs(formats, generations, variables)?;
let plan = existing_runner.resolve_plan(inputs)?;
let check = facts.compare_with_fixed_plan(&plan)?;
```

Le constructeur et la comparaison partagent **le même producteur**. Ce qui entre
respectivement dans l'empreinte des configs d'entités, celle des faits complets
(entités/relations/fournisseur/recettes effectives) et l'identité globale est
nommé et versionné explicitement. L'empreinte entités seule ne devient pas
silencieusement une empreinte de graphe. Types/définitions normalisés par les
contrats existants, défauts effectifs inclus, clés de maps ordonnées ; l'ordre
des listes sémantiques est préservé. Valeurs non finies ou non représentables
refusées avant sérialisation, sans conversion en null.

Un ancien champ opaque dont le producteur/version/périmètre n'est pas connu ne
se requalifie pas par supposition : refus FactsContractUnknown ou état non
qualifié explicite. Le nouveau chemin construit ses empreintes depuis les faits,
pas depuis une étiquette passée à la main. Il ne réécrit aucun descripteur
stocké ni l'identité d'un plan déjà figé ; la voie préliminaire actuelle et
l'ouverture legacy restent disponibles avec leurs limites présentes.

Le résultat du contrôle veut dire **faits déclarés concordants avec les entrées
du plan**, jamais « faits lus d'une base », « plan appliqué » ou « transfert
exécutable ». Pas de conversion vers PlanJournalState::Applied ni vers une
capacité d'écriture. Première tranche : introduire et éprouver ce producteur et
sa comparaison ; ne pas ajouter prématurément un état de stockage qualifié à
KnownTransferPreview. Son statut intégré reste inchangé tant que les prérequis
d'observation/liaison ne sont pas réalisés et revus ensemble.

### Source et génération : preuves séparées, absence visible

La liaison de configs est possible même si les générations source/cible sont
Unversioned. Elle ne doit pas appeler validate_generation_preconditions pour
forcer une génération inventée : ce contrôle d'exécution reste distinct et
continue de refuser. De même, la comparaison d'une identité déclarée exige
l'attendu explicite déjà demandé par l'API ; l'égalité de deux déclarations
n'établit pas leur provenance. Absence d'attendu, origine inconnue, génération
absente ou étrangère gardent des motifs séparés.

Fixtures du prochain diff : attendu et origine synthétiques fournis explicitement,
ainsi que cas absents/étrangers ; jamais d'UUID tiré du chemin, d'un hash de configs
ou du rapport. Le contrôle ne crée aucun identifiant de stockage. Une empreinte
d'inventaire ne devient ni génération durable ni preuve que des écritures ont
cessé. Un plan figé reste le même lorsque sa cible attendue est examinée après
une publication simulée ; un avancement étranger est refusé.

### Frontière de fichiers et suite qui n'entre pas dans A

Premier diff proposé : migrations/facts.rs (contrat/producteur purs et tests),
plan.rs et point de construction dans MigrationRunner. Petit adaptateur
backend_schema.rs à convenir avec sa propriétaire mémoire et orchestration,
s'il est nécessaire pour exporter ses faits validés sans dupliquer compose.
La session choisit la forme du code et des types ; l'orchestration coordonne
la propriété de ce fichier partagé avant modification. Aucun fichier partagé
n'est encore réservé ou modifié par cette préparation.

Ne pas élargir A à backend.rs, au CLI, à Catalog::initialize, à un changement
d'ouverture legacy, à la lecture d'une base ou à une nouvelle clé metadata.
Le raccord ultérieur de l'inspection RO pourra restituer relation:<nom> dans le
même snapshot que les configs/journal, avec décodage commun et refus des lignes
ambiguës ; ses tests se feront sur données synthétiques. Ce raccord et le bootstrap
persistant ne sont pas supposés livrés par le producteur pur.

### Témoins et critère de fin de A

| Cas à écrire | Preuve attendue |
|---|---|
| Faits complets identiques, maps permutées | Même empreinte et même identité du plan, validation des dépendances existante réutilisée. |
| Config/rôle/découpe/signal/modèle ou recette modifiés, anciennes étiquettes conservées | Refus nommé de liaison ; pas un simple rapport différent annoncé cohérent au plan. |
| Relation inversée, propriété changée, relation manquante | Détection par l'empreinte complète ; empreinte entités seule incapable de qualifier ce cas. |
| Fournisseur code réel et fournisseur d'une autre entité | Faits effectifs réellement fournis, pas de raccourci Scope ou fournisseur=None de fixture. |
| Source limitée aux entités, recette non résolue, version d'empreinte inconnue | Incomplet/non qualifié ou refus explicite, jamais graphe vide/defaults réputés observés. |
| Source/génération absentes mais configs complètes | Liaison déclarée vérifiable ; readiness d'exécution toujours refusée, aucune création implicite. |
| Identité attendue absente/étrangère, génération étrangère | Motifs distincts ; aucune identité ou génération déduite des empreintes. |
| Cible attendue après publication simulée, puis avancement étranger | Plan original immuable, même identité pour le passage attendu ; refus de l'avancement étranger. |
| Valeurs non finies, listes dont l'ordre est sémantique | Refus avant perte de valeur ; ordre conservé et changement détecté. |

Fin de tranche : constructeur réellement consommé par le chemin de résolution
concerné du runner, comparaison utilisant exactement le même producteur, tests
synthétiques ci-dessus et diff relu. Pas seulement une utilité hash sans appelant.
L'aperçu intégré n'est pas rebaptisé « migration livrée ». Après revue de code,
proposition d'un ciblé600s/j8/Mem32 sur luciepc, target de session32Go/lib épinglée,
poste lourd et mesure prioritaire ; aucune réservation ni commande de test faite
pour la présente préparation. La passe d'intégration se choisit après le diff.

### Décisions autonomes et génération composite retenue par Lucie

Sont autonomes, car déjà couverts par les contrats : découpage A puis raccord RO,
noms/types, canonicalisation et version du contrat de faits en mémoire, extraction
d'une primitive commune, placement des tests et refus d'entrées incomplètes. Pas
de nouvel outil visible ni de changement de politique de stockage dans A.
La répartition d'un fichier partagé et la recette de passe sont du ressort de
l'orchestration. **A n'attend pas une décision de conception nouvelle.**

1. **Question tranchée le 12 novembre : génération de publication et empreinte
   d'état ont des rôles distincts.** Après l'explication A→B→A puis la proposition
   composite, Lucie valide la dernière recommandation : « oki pour ta
   recommandation alors ». La génération désigne une identité durable de
   publication ; l'empreinte de l'état logique permet le partage et la
   réutilisation du travail après requalification. Ce n'est pas le choix d'une
   génération définie uniquement par le contenu.

| État logique | Identité de publication | Empreinte d'état |
|---|---|---|
| A initial | P1 | H-A |
| B après nouvelle publication | P2 | H-B |
| Retour au même A | P3 | H-A |

Une publication nouvelle a une identité distincte, même si son état retrouve une
empreinte antérieure. Le rejeu de la **même** opération déjà applied conserve son
identité ; il ne crée ni nouvelle publication ni droit de répéter ses effets.
Le format de l'identifiant et son producteur durable restent à implémenter avec
le journal : aucun compteur global ou horodatage n'est choisi par cet exemple.

L'empreinte ne se réduit pas aux octets de texte : elle doit couvrir les faits
effectifs dont dépend l'opération, notamment occurrences, positions, relations,
visibilité, configurations, recettes, modèles/dimensions et références aux
couches parentes pertinentes. La portée et la complétude des dépendances doivent
être établies ; une empreinte partielle ou de provenance inconnue ne justifie
aucune réutilisation. On vise les vues/couches concernées, sans invalider toutes
les branches pour une publication étrangère au plan.

Si P1 est devenu P3 mais que les dépendances requises sont requalifiées
identiques, les calculs et étapes préparées peuvent être réutilisés. On résout
un **nouveau plan figé pour P3**, avec son identité propre ; le plan et l'identité
de P1 restent immuables. L'empreinte ne contourne ni la vérification de génération
à l'admission/application ni le journal, les annulations, les effets déjà appliqués
ou la publication atomique. Le partage des contenus/vecteurs demeure indépendant
des occurrences et des publications.

La décision retire cette question de conception des attentes ; elle n'ajoute
aucun code durable dans A et ne qualifie pas une empreinte d'inventaire existante
comme génération ou état complet. Toute opération d'établissement initial ou d'avancement durable,
transfert, apply, activation, publication, copie/lecture d'une base de Lucie ou
politique de rétention reste hors de la tranche préparée.

## Historique — tranche A avant passe adaptée

GO Lucie relayé par C7 ; source privée59eb3d9df, producteur/comparateur implémentés
et consommés par MigrationRunner::resolve_plan_from_facts. Déclarations complètes
explicites (None distinct de vide), compose et validation finale communs via petit
adaptateur backend_schema ; backend.rs non modifié. Contrats versionnés séparés
entités/faits complets, mécanisme canonique et validation des nombres finis
partagés avec l'aperçu. Faits/modèles/recettes fournis, aucun builder ou observation
stockage ; report scellé SuppliedDeclarations, pas un jeton d'exécution.

Ciblé natif+code :64/0/0,18nouveaux+15aperçu+31existants, inventaire avant exécution,
fenêtre initiale600 respectée sans reset (fin332,06s). Source206 non compilée après
syntaxe repérée ; d95363/1/0 dû à mon mauvais attendu DEFINES→Scope alors que le
fournisseur donne Scope→Symbol ; test corrigé vers égalité de toutes les relations
réelles, pas de modification du producteur. Preuves et SHA dans le ticket.
Revue seule mémoire effectuée. Passe adaptée et intégration encore à faire après
le master ED prioritaire ; aucun changement de la génération durable ou de l'aperçu
non qualifié n'est déclaré livré. La question1 a depuis été tranchée par Lucie ;
la décision composite de la section précédente fait foi.


## État actuel — A intégrée et nettoyée

Source compilée d2cb7f063 ; intégration master6d4d1c882 après rebase documentaire
417ca1f26, code produit strictement identique à la source jouée.
F5 5/5, vrai checklib sans features, lib2222/0/65 (18faits+15aperçu+lien80),
NAV40/0/0, CLI5/0/0 et quatre ELF/source/native29/220gabarits attestés. Verdict
composé en1385,89s dans fenêtre initiale3600, incidents de montage/parser et
bus privés conservés dans le ticket ; quiescence finale et TMP retiré prouvés.
Aucun nouveau Cargo pour le rebase documentaire. Tour C7/3contrôles verts lus ;
WTs/target et seules PG privées retirés après dumps, archives et gardes fraîches.
Preuves et ELF rehachés aux deux postes conservés. La décision composite ci-dessus
est acquise, producteur persistant non implémenté. Aperçu toujours non qualifié
au stockage ; ticket migration reste ouvert.
