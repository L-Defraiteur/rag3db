# Raccord physique d'un index neuf — contrat à relire

Porteur arbre-principal, même [ticket migration](../tickets/2026-11-12-migrer-les-ancres-de-code-vers-le-contenu-partage.md).
Base de lecture master798ab1aa7, après livraison Pub, ED et embarquement.
Qualification ciblée privée P14 **VERTE** sur a43c475f9ce1f7cbd6c1c2a0ee56719b521678f1 :
[preuves et limites](02-preuves-ciblee-raccord-physique.md).20fonctions/19juges,
8enfants réels,44,321s ; aucun GO global/master. Aucune API physique ci-dessous
n'est encore livrée sur master. Aucun index réel/service/modèle touché.
Les étapes de préparation et leurs anciens comptes non joués ci-dessous sont
historiques ; le bilan P14 lié ci-dessus fixe les comptes actuellement qualifiés.
La garde de retrait reste exclusivement chez Recherche.

## Tranche verticale proposée

Créer un index lexical natif explicitement neuf, enregistrer des occurrences
`Snippet`, produire leur dérivée par contenu `SnippetContent`, découper/indexer
celle-ci avec les primitives actuelles et conserver les relations d'occurrences.
Fermer le processus, rouvrir, vérifier les valeurs et rechercher un texte dont
les résultats sont reprojetés vers les occurrences de la bonne cellule.

Premier profil **BM25 seulement, sans modèle configuré ni vecteur/sparse**.
Ni HashEmbedder, ni modèle/dimension inventés pour satisfaire un constructeur.
Le contenu textuel, les configurations et les recettes sont complets/fournis ;
les faits physiques sont réellement relus, jamais déclarés connus par un appelant.
Le premier pas est P0 (index neuf vide) vers P1 (lot initial). Modifier un index
existant, servir les anciennes publications comme vues, migrer du legacy et
transférer des vecteurs connus restent hors de cette tranche.

La fixture concrète est [fixture.json](raccord-physique/fixture.json) : huit
occurrences, deux cellules, six versions de contenu attendues. Nom, docstring
et corps entrent tous dans le rendu ; chemin/positions restent sur l'occurrence.
Le calcul des UUID et des chunks est exclusivement celui de `derive_nodes`,
`Chunker` et `chunk_uuid`, pas la classification pédagogique de cette fixture.

## Trois écarts vérifiés dans le code actuel

1. `Catalog::new` exige un Embedder. `AbsentEmbedder` signifie modèle attendu
   indisponible, pas absence de modèle configuré. Les services d'ingestion
   enregistrent le modèle courant. Embarquement confirme que sa livraison23ceed87b
   ne fournit pas de constructeur texte seul. Il manque une représentation
   explicite de cette absence ; une sentinelle de nom/dimension serait fausse.
2. `MigrationRunner::apply` ne qualifie pas une identité résolue durable ;
   `migration_execute_graph` ne monte que `conn`, alors que DeriveNode demande
   aussi dialect/config/entity_configs/scope. L'ancien journal huit colonnes
   ne devient pas qualifié parce qu'un checksum ou checkpoint est Completed.
3. `Catalog::commit_lot` valide la transaction native, puis applique le tampon
   FTS et pousse ses blobs. Le COMMIT des lignes n'est donc pas, à lui seul,
   une publication effective de l'index lexical. Le reçu doit attendre cette
   durabilité et la validation du résultat, et l'ouverture doit reconnaître
   une candidate interrompue sans la servir comme un état publié.

## Invariants et raccord des identités

- Conserver O/P et H déclaratif acquis. Réutiliser le protocole Registry/Intent
  de `publication`, sans modifier la surface fermée d'OwnedPublicationCatalog.
  Son ancienne API continue à refuser les tables/clés hors contrat.
- Le nouvel adaptateur physique garde **une seule base native** pour les
  données, le journal existant et les métadonnées de publication. Pas une base
  déclarative voisine qu'un second COMMIT ne pourrait pas lier atomiquement.
- Une preuve physique scellée lie O/P, déclarations A, inventaire exact du
  schéma, valeurs applicatives complètes, occurrences/scopes/positions,
  relations avec propriétés/multiplicités et chunks. H des données logiques
  reste distinct des identifiants d'événement et des octets d'une matérialisation.
  Les champs internes volatils ne sont exclus que par contrat explicite ;
  aucune colonne ou donnée étrangère n'est escamotée comme « interne ».
- L'attestation de matérialisation lexicalement prête est séparée : contenu
  réellement indexé/recherchable et blobs persistés, contrôlés après fermeture
  et réouverture. Un hash de schéma ou un nombre de lignes ne la remplace pas.
- `rag3db::get_storage_version()` fournit la version du moteur chargé ; le
  schéma interne et le profil applicatif ont leurs propres contrats. Aucune
  génération d'une ancienne base n'est déduite de ces versions.
- La création absente établit P0 sous exclusivité et prouve l'état neuf vide.
  Le plan résolu P0→P1 fixe source/target, recette/graphes, contenu d'entrée
  exact et publication cible préallouée. La génération événementielle et les
  empreintes observées entrent ensemble dans la qualification, sans rebinder
  l'ancien plan sur l'état courant.

## API à fournir, sans second exécuteur

```rust
// Surface écrite dans le diff en revue, non qualifiée/non livrée.
let mut index = OwnedPhysicalIndex::create_new(root_absent, origin_seed, schema, cells)?;
let before: QualifiedPhysicalState = index.observe_published()?;
let plan: QualifiedIndexPlan = index.prepare_initial_batch(
    &migration_runner, operation_id, complete_record_variables, lexical_probes,
)?;
let receipt: PhysicalPublicationReceipt = index.execute_initial_batch(&migration_runner, &plan)?;
drop(index);
let reopened = OwnedPhysicalIndex::open_owned(root, expected_origin)?;
let after = reopened.observe_published()?;
```

`QualifiedIndexPlan` compose le ResolvedMigrationPlan existant et l'intent de
publication, avec l'origine/état physique source et l'entrée immuables. Il n'est
ni désérialisable sans validation ni un alias de KnownTransferPreview. Le runner
reste seul responsable de la résolution/validation des fichiers Mermaid et de
l'exécution des étapes ; l'adaptateur ne contient pas une autre boucle métier.

L'identité résolue, l'origine et les publications source/cible doivent être
portées durablement par **le journal de migration existant**, avec un contrat
versionné pour les seuls catalogues neufs. Pas de second journal de statuts.
Le vieux journal huit colonnes reste explicitement non qualifié. L'évolution
exacte de sa représentation fait partie du diff à relire avant exécution.

L'absence de modèle doit être structurelle dans les capacités du Catalog et
les services qu'il monte : registre dense vide, aucune acquisition/appel,
aucune disponibilité ou dette vectorielle fictive. Les chemins configurés
actuels conservent leur sémantique. Le coût de cette primitive commune est
à borner après lecture du constructeur et des dépendances, avant édition
partagée ; ni booléen de test permissif ni embedder factice de production.

## Transaction, durabilité et reprise

1. Valider entrée complète/codec, profil BM25, graphes et capacités sans effet.
   Revalider O/P/état observé sous l'écrivain. Tout manque, origine étrangère,
   schéma inconnu, dette dense/sparse ou mutation des données refuse.
2. Noter l'identité d'exécution du plan dans le journal existant avant les
   effets. Réutiliser les checkpoints et le montage commun des services du
   Catalog, puis son chemin d'écriture/derive/chunk/index. Le batch d'entrée
   complet ou son support possédé durable fait partie de la reprise ; un simple
   digest qui demanderait de retrouver un ancien environnement ne suffit pas.
   Après COMMIT, les données et l’entrée durables permettent de reconstruire
   le tampon FTS perdu avec le processus.
3. Après COMMIT logique et poussée FTS, relire le résultat et la matérialisation.
   Tant que cette fermeture n'est pas acquise, l'état est une candidate en
   exécution, pas P1 publié ; les notifications de disponibilité restent retenues (la fenêtre actuelle
   `avec_retenue`, qui publie aussi sur erreur, ne suffit pas). Une ouverture normale refuse sa lecture comme
   index prêt. Les valeurs déjà écrites ne sont pas prétendues annulées.
4. Publier atomiquement le reçu, la tête déclarative/physique et l'issue
   qualifiée du journal seulement après cette fermeture. Le reçu porte les
   faits vérifiés, pas uniquement un statut Applied. Une erreur de COMMIT rend
   le handle inutilisable jusqu'à réouverture/résolution.
5. Une reprise reconnaît le même plan/entrée durable et termine/valide ses
   ressources ; elle n'en invente pas un nouveau. Après publication/réponse
   perdue, le même identifiant rend le même reçu sans refaire l'effet. Toute
   tentative différente sur la même opération refuse.

Le détail du point de publication et du journal sera vérifié dans le diff,
pas prouvé par ce document. Le CLI `migration apply` reste indisponible tant
que sa qualification générale n'est pas implémentée ; ce profil neuf contrôlé
ne fournit aucune passerelle permissive pour les autres index.

## Témoins concrets avant qualification

| Cas | Fait à vérifier |
|---|---|
| Création, exécution, fermeture, nouvelle ouverture | Huit UUID d'occurrence, six versions, liens/chunks exacts et texte réellement retrouvé ; aucun modèle construit/appelé. |
| Alpha/Beta, nom/doc/corps différents, positions | Déduplication seulement sur les rendus de la même cellule, positions/provenances intactes, aucun lien croisé. |
| Reçu et entrée figés | Résolution par op après réponse perdue ; entrée/recette/plan changé sur même op refuse ; ancien plan identique octet pour octet. |
| Origine/P/schema/données étrangers, legacy | Refus avant les nouveaux effets ; tous les octets de fixture inchangés par le refus RO. |
| Mort enfant avant COMMIT logique | Données applicatives P0 vides ; intent/journal durables possibles, aucune candidate servie. |
| Mort après COMMIT logique avant FTS durable | Exécution reconnue/reprise du même plan, ouverture normale refusée, pas de faux rollback. |
| Mort après FTS durable avant publication | Candidate non servie ; reprise vérifie puis publie une seule fois. |
| Mort après publication avant accusé | P1, toutes valeurs et index persistés ; reçu retrouvé sans doublon. |
| Échec FTS, COMMIT/ROLLBACK ambigu | Aucun reçu prêt ; erreur complète conservée, handle inutilisable/reprise explicite. |
| Graphes/services/journal non qualifiés | Aucune auto-réparation legacy ni modèle par défaut ; refus explicite. |

## Frontière du diff et recette prévue

- Nouveau `migrations/physical_index` : adaptateur possédé, faits/plan/reçu scellés
  et lecture exhaustive du profil ; réutilisation de `facts`, `publication` et
  `storage_inspection`, sans élargir implicitement leurs garanties.
- `migrations.rs` : entrée qualifiée du runner et identité dans son journal
  existant ; aucun second exécuteur/checkpoint-store.
- `catalog.rs` : montage de services commun ingestion/migration, absence réelle
  de modèle et barrière de publication/reprise lexicales ; jamais appeler
  `initialize` sur un legacy découvert pour l'adopter.
- Contrats modèle/schéma/dialecte seulement si la primitive d'absence ou le
  journal l'exigent. ED relit ces frontières ; codeparsers, IDE, activation/
  résidence et garde de retrait ne sont pas portés par cette tranche.

Revue mémoire+C7 du contrat/API/fixture puis diff et argv **avant tout lourd**.
Ciblé natif sur luciepc : commonconfiner, lib29 immuable, CPU/j8/Max32,
target de session unique32Go, fenêtre initiale600s après poste pris. Aucun
modèle/service/base réelle ; noms de fixtures absents et possédés. Premier
rouge arrêté/diagnostiqué, sans retry inchangé. Passe adaptée et tour master
seulement après la preuve physique. Ce périmètre n'autorise aucun GC, parent
layer, activation de service ou accès à une base de Lucie.


## Précision du diff : cellules et transactions

La lecture de `dans_la_cellule` confirme un refus explicite de changer/emprunter
une cellule sous transaction ; `TamponPleinTexte` est adressé par entité, pas par
cellule. La tranche conserve cette frontière. La création reçoit donc les cellules
finies du nouveau profil, crée leurs nœuds et leurs index FTS vides avant de sceller
P0. Le programme figé contient une cellule par étape ; le runner valide une
transaction native par étape et conserve dans le journal existant les valeurs
observées et la longueur du préfixe validé, dans cette même transaction. Ce
résultat n'est pas un second journal de statuts : la ligne reste pending jusqu'à
publication globale. Un arrêt avant le premier COMMIT laisse les données P0 ;
après un COMMIT intermédiaire, la candidate peut contenir un préfixe non servi.
La reprise vérifie ses valeurs, rebâtit le FTS perdu et poursuit les seules étapes
restantes du même programme. Reçu/tête/journal appliqué restent un seul COMMIT
final après fermeture, réouverture et requêtes des deux cellules.


## Diff proposé et recette figée avant premier Cargo

- `Catalog` : Option du modèle, profil lexical strict (dense/sparse refusés),
  assemblage commun des services avec lecture stricte pour les migrations ;
  invalidation locale même avec un backend dont poison est sans effet ; annonce
  après FTS/blobs et fermeture incomplète propagée. Le montage physique impose
  `BlobBacked { lazy: false }`, indépendamment de l'environnement.
- `IngestEntitiesNode` appelle le verbe complet existant, depuis des records JSON
  canoniques conservés dans la définition du graphe. Aucun nouveau derive/chunk/
  exécuteur. Le runner fige sources+variables et conserve la boucle d'étapes.
- `_DataflowMigration` neuf garde les colonnes historiques et ajoute
  `qualified_input` (protocole1, source complète, intent, programme, entrées,
  sondes et identité qualifiée) et `qualified_result` (préfixe accompli et
  valeurs observées dans la transaction des données). Le vieux journal n'est
  ni adopté ni requalifié. La ligne reste pending jusqu'au COMMIT final unique.
- `physical_index` capture toutes les tables/propriétés, les valeurs logiques,
  les multiplicités et les métadonnées ; blobs FTS et lignes de checkpoints
  restent des témoins séparés. Le relevé sert de garde contre les modifications
  avant la reprise. Les ouvertures normales Catalog/CLI/hôte refusent ce profil
  fermé ; seul l'adaptateur possédé passe par sa capacité interne, après lecture
  RO de l'origine, du contrat et de la candidate/publiée.
- Neuf fonctions natives (huit scénarios juges et leur helper enfant), avec morts
  avant/après COMMIT des deux cellules,
  après FTS et après publication, plus neuf scénarios de capacités/erreurs CPU.
  Exactitude8/6/8/2, UUID/rendus/chunks par les producteurs communs ; réponses
  lexicales nom/doc/corps, cellules, codecs figés et refus étrangers sont vérifiés.
  Les fichiers Mermaid de fixture sont retirés après mort : reprise depuis le
  support durable du journal et non depuis l'ancien environnement.

Recette initiale publiée avec le diff (historique, remplacée par P14) : `raccord-physique/lancer-cible.sh SHA40`,
qui appelle le superviseur commun depuis `/var/tmp`, délai initial600s, poste
lourd/MemoryMax32G/Highmax, bus privé unique et preuves neuves `...-v1`. Le client
`cible.py` compile une seule lib de tests native+code sans features par défaut,
`-j8`, target unique32Go, lib29 du magasin ; il inventorie l'ELF, le copie/rehashe,
atteste source et deux gabarits de recherche requis, puis joue9 CPU+9 fonctions natives (8 juges et helper)
avec0ignoré. Tous les lancements ELF exportent la même lib et le manifest_dir
exact ; état/journaux/gabarits/TMP propres sont hors WT. Maps natives/cgroups,
argv, sorties et témoins signés par les calculs sont enregistrés. Le résultat
client VERT reste subordonné au reçu final du superviseur VERT/code0/restes[],
bus physiquement absent et quiescence indépendante ; aucun master permis par
cette seule préparation. Aucun Cargo de cette recette n'a encore été joué.


Réserve Drop fermée dans le diff de revue : le buffer possède une invalidation
partagée, sérialisée avec save/delete/flush ; ses clones et sa destruction ne
retentent pas la persistance après l'issue inconnue. Les octets restent lisibles
jusqu'à destruction, la reprise native conserve déjà son support durable.
Le Catalog utilise aussi une famille de connexions révocable : writer, lecteurs,
read_pool, connexion sync et Arc exportés partagent l'invalidation, y compris si
le backend n'a pas poison. Les méthodes transactionnelles et settings restent
explicitement déléguées au backend ; rollback seul demeure permis pour nettoyer.
Cela ferme le contre-exemple set_scope trouvé en revue sans une liste fragile de
gardes par API. Deux témoins CPU couvrent première poussée échouée qui réussirait
à la suivante/clones/Drop sans retry, et délégation pool/révocation des lecteurs.
Les comptes détaillés finaux figurent ci-dessous ; aucun scénario exécuté à ce stade.


## Fermeture du bootstrap et couverture du préfixe acquis

L'appartenance est maintenant revendiquée par un fichier canonique immuable
`.owned-physical-index-v1` sous le root neuf (fichier et répertoire synchronisés)
AVANT toute création native. Sa présence, même partielle/illisible, refuse les
constructeurs ordinaires `new`, `read_only` et `with_config` AVANT Database::new
ou toute reprise WAL. Le Catalog refuse aussi cette appartenance avant son
initialisation. L'accès natif interne nécessite une capacité scellée de root,
origine et chemin exact ; l'accès bootstrap n'accepte qu'une vraie base vide.
Un arrêt après claim ou après initialize avant P0 reste quarantiné : aucune
adoption implicite et aucune reprise automatique de ce bootstrap sans plan.
Un témoin enfant vérifie ces deux fenêtres et les trois portes ordinaires,
avec tous les octets de root inchangés par leurs refus.

Les morts COMMIT portent désormais aussi `:2` : avant et après le COMMIT de la
deuxième cellule, avec relevé natif du préfixe et de ses6 ou8 occurrences avant
reprise. Les sources Mermaid sont retirées, et le même plan durable se poursuit
sans rejouer le préfixe acquis. Les comptes remplacent les anciens raccourcis :
9 fonctions natives =8 scénarios juges +1 helper ; ce dernier est exercé par
8 sous-processus réels (6 fenêtres du lot,2 du bootstrap), et ne compte pas
comme un scénario indépendant lorsqu'il retourne hors environnement enfant.
Avec9 scénarios CPU (dont les2 contrats du registre de nœuds existants), cela
fait18 fonctions Rust brutes,17 scénarios juges et8 exécutions enfant. Aucun
Cargo ou de ces scénarios n'est encore exécuté ; la recette600 consigne ces
comptes séparément dans le verdict et attend revue du delta bootstrap.


Dernier filet de publication en revue : l'ouverture et observe_published ne
permettent l'absence de journal qu'à la publication de création. Une cible
appliquée exige le journal, le préfixe final et son relevé de valeurs/schema/
checkpoints correspondant à la tête ; son intent et sa source doivent être ceux
du registre de publication. Un scénario natif de corruption (ligne supprimée
ou résultat modifié après publication) complète le refus RO sans nouvelle
écriture. Comptes finaux :10 fonctions natives =9 juges+helper ;9 CPU ;19 brutes,
18 scénarios juges,8 enfants. Ces comptes restent préparés/non joués.


## Premier lancement arrêté avant Cargo — rouge v1 conservé

Source8df28b0d3, GO C7 ciblé600, client arrêté en0,154s : l'isolation de
XDG_CACHE_HOME a aussi déplacé le magasin implicite de libs. L'exporteur cherche
lib29 sous le cache privé et refuse2 avant exec Cargo. SDK réel intact, target
absent, aucune compilation, aucun index natif ni modèle/service exécuté.
Superviseur `cx-raccord-physique-confinement-v1/validation-om027vtw` ROUGE/code1,
restes[], nettoyé, autorité terminée ; bus `/tmp/r3v-m963pido` physiquement absent,
scan contexte own[] et poste rendu. Les234 fichiers de preuves client/superviseur
sont copiés et SHA identiques sur les deux postes, diagnostic/quiescence dans
`cx-preuves-raccord-physique-cible-v1-local/`. Aucun rouge n'est effacé ni réétiqueté.

Correction de recette seulement : `RAG3DB_LIBS_DIR` épingle explicitement le vrai
magasin `/home/lucied/.cache/rag3db-libs`, en conservant XDG/cache de travail privés,
la même lib29 immuable et tous les témoins. Preuves neuves v2 ; fenêtre corrigée
plafonnée599s (coût de v1 conservé), sans prolonger ni réutiliser son reçu expiré.
Nouveau SHA/argv soumis à coordination avant nouvelle exécution ; aucun rejeu
inchangé, aucun changement Rust ou reconstruction/publication de SDK.


## Compilation v2 arrêtée, bilan sans modèle corrigé

Étape corrective distincte source2e71e1755 arrêtée au premier rouge en110,4066s :
la compilation signale deux E0609, progress.rs319/357 prenaient encore name sur
Option<EmbeddingModelEntry>. Aucun test ni ELF rag3weaver produit, aucun index
natif/service/modèle exécuté. Superviseur validation-rcz_mqen ROUGE/code1/restes[],
bus `/tmp/r3v-4yycm8du` absent, autorité terminée, scan contexte own[]. Le target
unique de dépendances est conservé1 349 776 841octets, sans effacement/rejeu.

Correctif produit en revue : IndexProgress.model est Option<String> ; un modèle
configuré garde son nom et sa représentation JSON string, un profil lexical rend
None/JSONnull et la ligne sans vecteurs non déclarés. Les deux producteurs de
bilan utilisent map, le helper des tests conserve Some réel, et l'e2e du modèle
attendu indisponible exige explicitement ce Some dans son affichage. Le témoin
lexical existant et les assertions de la fixture native vérifient absence de
modèle/dette fictive dans les bilans. Preuves/recette v3 distinctes, même plafond
599s et même target/SDK29 ; delta/source à relire avant toute nouvelle exécution.


## État ciblé final après corrections

Voir le [bilan P14](02-preuves-ciblee-raccord-physique.md) :20fonctions brutes,
19juges,8enfants réels, qualification ciblée VERTE sur sourcea43c475f9.
La recette publiée lance désormais P14 distinct599s,10CPU et10fonctions natives
exact par exact avec arrêt au premier rouge ; tampon explicite512MiB/Max32GiB.
Les corrections et anciens rouges sont conservés dans le ticket parent et les
preuves de chaque portée. Aucune qualification globale ou livraison master ici.
