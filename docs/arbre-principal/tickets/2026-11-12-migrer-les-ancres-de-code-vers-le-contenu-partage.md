# Migrer les ancres de code vers le contenu partagé

- **État** : ouvert, premier patch contrat/admission/plan/préflight autorisé après publication du plan corrigé ; transformation et activation non autorisées.
- **Famille** : produit
- **Gravité** : capacité manquante ; le partage déclaratif1a ne transforme pas les index de code existants.
- **Atteignable en service** : parcours CLI/daemon raccordés sur branche privée, non livrés sur master ; gabarits et index de production inchangés.
- **Touche rag3weaver** : oui, admission de format, migrations déclarées, transfert des valeurs indexées, schéma code et témoins.
- **Pour** : arbre-principal ; orchestration et ED pour revue/autorisation, recherche pour contrat d’index si besoin.
- **Signalé par** : préparation autorisée par orchestration après le lot1a08086ba36 ; ticket de préparation distinct ci-dessous.

## Livrable de préparation et périmètre

[Plan1b sur sourcebc34624dc](../12-11-2026/01-plan-de-migration-de-l-ancre.md),
lié au [ticket de préparation](../../tickets/closed/2026-11-12-preparer-la-migration-de-l-ancre-sur-copies-privees.md).
Ce ticket garde l’implémentation future ouverte lorsque la préparation sera relue ;
il ne rouvre aucun des cinq tickets1a fermés.

## Constats statiques, sans exécution nouvelle

MigrationRunner et les checkpoints existent ; aucun appel produit du runner trouvé
sur la source relue. Checksum stockée mais non comparée par pending/applied ;
services de migration limités à conn ; annulation déclarée ≠ atomicité de tous les
index. Le garde1a refuse déjà le changement de searchIndex sur Scope peuplé.
KnownVectors relie un slug courant et des UUID de chunks ; nouvelle identité de
parent change ces UUID, et la réutilisation dense actuelle ne couvre pas l’export
sparse. Ces écarts sont détaillés avec leurs chemins dans le plan, sans rouge
produit nouvellement revendiqué.

## Travail proposé après autorisation, ordre et critères

1. Relire format applicatif distinct du schéma interne8, point d’entrée explicite
   du runner et préflight old/cible/en-cours ; T1/T2/T12 sans mutation cachée.
2. Adapter le fournisseur de faits code plutôt que recopier Scope ; transport
   générique de chunks/vecteurs connus par cellule, modèle/dimension et recette.
   Profil initial dense+BM25 proposé ; sparse présent refuse tant que son transfert
   sans calcul n’est pas prouvé. Zéro inférence documentaire, jamais de remplissage
   automatique d’un manque.
3. Publication de config/format validés, reprise et annulation observables avec
   journal/checkpoints existants ; gardes ordinaires conservées. Refus d’ouverture
   normale en cours ou ancienne incompatible avant effets et connexion de modèle.
4. Témoins T1–T12 sur fixtures synthétiques et copies privées autorisées ensuite :
   couverture des occurrences, relations dans les deux sens, vecteurs par modèle,
   recherches/positions/provenance et filtres, reprise/annulation/rejeu.
5. Passe réglementaire adaptée, source/binaire/preuves réelles, revue puis tour
   master. Activation d’une base réelle et suppression de l’ancien index exigent
   leur périmètre ultérieur explicite ; pas de GC ni mesures M3–M5 dans ce ticket.

Décision orchestration du12nov : profil initial BM25+dense admis, sparse refusé
avant effets tant que son transfert n’est pas prouvé. Le corpus BM25 change avec
la déduplication ; variation de pertinence acceptée avec preuves de couverture,
filtres, positions, provenance et ordre des égalités, sans équivalence complète
bit pour bit ni scores/rangs arbitraires identiques. Ce profil initial ne livre
pas une migration sparse ; aucun manque n’est comblé par un modèle. ED doit
encore relire la frontière d’API du premier correctif avant toute autorisation.

## Autorisation actuelle

Préparation documentaire : aucun Rust modifié, test/build lourd lancé,
target/service créé, base réelle lue ou copiée, migration/activation exécutée.
L’autorisation bornée suivante prend effet après publication du plan corrigé.

## Trois avis ED intégrés au plan, avant Rust

Admission commune avant RW et embedder_or_absent sur les chemins hôte/CLI,
inspection des métadonnées RO sans initialize/DDL/réparation/modèle ; façade
PreparedBackend proposée plutôt qu’une règle copiée par client.

Qualification T12 : checksum ET plan résolu (variables/configs/recette/formats/
génération), historique inconnu/orphelin/malformé refusé, revalidation sous
exclusivité réelle ; running durable avant effets métier. Le verrou TTL actuel
ne vaut pas preuve d’acquisition exclusive. Bootstrap éventuel du journal à
traiter explicitement, sans effet caché du préflight.

Publication proposée atomique des configs/format/génération/journal appliqué,
sans DDL, après validations ; pas d’ouverture normale running/failed non
réconcilié ni de succès déduit d’un checkpoint Completed. Clôture avant ouverture.

Sparse se décide d’après les données/signaux/marqueurs à transférer ; models.sparse
seul peut n’être qu’un lecteur de surlignage. API/point d’entrée proposés dans le
plan, pas livrés. Autorisation du premier patch encore attendue séparément.

## Autorisation bornée suivante, distincte du livrable documentaire

Orchestration autorise le premier patch sur un WT cx neuf : contrat typé commun
format/plan résolu, inspection/admission sans mutation, raccord appelable du
plan/préflight et qualification stricte journal/checksum sous exclusivité réelle
de copie. T1/T2/T12 sur fixtures synthétiques seulement, rouges avant correction,
compteurs zéro DDL/réparation/raccord modèle lors des refus. Réutiliser les
primitives existantes ; partager API/diff+témoin avant passe lourde.

Une cible dépourvue de transfert/publication validés refuse apply, jamais de
journal applied fictif. Aucun transfert chunks/vecteurs, activation Scope, base
réelle, service nouveau, M3–M5 ou moteur parallèle. Aucun master avant revue.
La suite complète du ticket reste à autoriser séparément ; cette autorisation
limitée ne vaut pas livraison de1b.

## Identité figée et qualification du stockage, précision de clôture documentaire

Le plan résolu garde les inputs source initiaux et les états/générations
source-cible attendus. La qualification compare séparément le stockage courant
à l’état source avant apply et à l’état cible attendu après publication. Applied
identique reste idempotent après cet avancement attendu ; on ne reconstruit pas
un faux « autre plan » depuis les métadonnées cibles. T12 exige aussi le refus
d’un avancement étranger. La revalidation sous exclusivité porte ce couple.
La préparation relue est close ; cette fiche garde le premier patch autorisé et
le reste non livré, sans réintroduire le ticket de préparation ouvert.

## Démarrage du premier patch après livraison documentaire

Master documentaire1cf24f671, préparation close. Nouveau WT local
`cx-wt-migration-preflight`, branche `cx-arbre-migration-preflight-2`, depuis ce
master, sans cherrypick. Aucun WT distant ni target créé à ce stade.

Préfixe test-only T12 : quatre témoins sur l’API actuelle MigrationRunner.pending
et son vrai lecteur d’historique dans Catalog, avec CallbackConnection qui sert
les lignes de journal synthétiques et compte tout effet inattendu ; modèle à
compteur. Cas checksum applied modifiée, version orpheline, statut inconnu et
version mal typée : refus attendu avec zéro effet/modèle, sans initialize. Rouge
pas encore joué ; aucune correction de production ni réussite revendiquée.

Frontière API à matérialiser et partager avant passe : descripteur typé de format
et générations attendues ; ResolvedMigrationPlan immuable construit avec les
variables et faits de schéma réels, son identité distincte du StorageInspection
courant ; qualification source avant opération/applied cible attendu après
publication ; inspection RO et façade d’ouverture commune hôte/CLI avant RW et
modèles. Le point d’entrée plan/préflight sera réellement appelable ; apply sur
la cible1b sans transfert/publication restera indisponible avec refus nommé.
Réutiliser les fichiers/graphes, le lecteur et le runner existants, pas un
second journal. Le premier préfixe ne suffit pas à prouver tout T12 résolu :
variables/configs/générations et revalidation sous exclusivité restent à couvrir.

Affinage du préfixe T12 avant exécution : checksum valide et identique dans les
cas orphelin/statut/version, seule la checksum change dans son cas dédié. Un
témoin positif de l’historique legacy appliqué identique doit déjà passer :
on ne « corrige » pas les quatre refus en refusant tous les historiques. Ce
positif ne prétend pas couvrir la qualification résolue du nouveau format.

## Rouge réel du préfixe journal T12 et correction bornée

Source50febdbea4aa6d61b96956b751304a6b631b34e4 propre sur luciepc,
`cx-preflight-t12-rouge-v1.log`, natif seul, un Cargo sous poste lourd
-j8/Mem32/lib29a7/échéance1800s : **1 réussi, 4 échecs, 0 ignoré**, rc101.
Le positif legacy identique passe ; checksum modifiée est sautée, orphelin,
statut inconnu et version mal typée retournent un pending apparemment valide.
Compteurs d’effets inattendus et de modèles zéro avant chacune des assertions.
Pas de relance aveugle. Target unique recréé1,3Go ; aucune DB créée/lue.

Correctif limité au lecteur existant et au runner : InvalidHistory nommé,
champs/types/statuts du journal validés sans défaut silencieux ni écrasement de
doublon, comparaison version/nom/checksum avec tous les fichiers déclarés avant
status/pending. Aucune génération fictive ni qualification résolue déjà annoncée.

Décision orchestration : legacy Unversioned explicite, profil reconnu distinct
de sa génération ; ouverture ordinaire de l’ancien manifeste conservée. Plan
préliminaire réellement inspectable, readiness de migration refusée si génération
source non établie. Pas d’inventaire de données ajouté dans ce premier patch ;
baseline de génération/fingerprint sous exclusivité à formaliser ici avant le
transfert futur. Fixtures versionnées pour T12 après applied cible/avancement
étranger ; aucune cible1b déclarée prête/applied par cette étape.

Vert du préfixe source**7fbd14434** : `cx-preflight-t12-vert-v1.log` à luciepc,
**5 réussis, 0 échec, 0 ignoré**, rc0, compilation18,48s, natif seul, target1,3Go.
Binaire attesté dans `cx-preflight-t12-vert-v1-binaire.sha256` hors target.
Aucun retry inchangé, aucune passe globale ni modèle/DB. Ces cinq verts couvrent
seulement les quatre refus de journal et le legacy identique : pas encore
l’identité résolue, les générations, l’admission commune ou l’exclusivité réelle.
Le ticket d’implémentation reste ouvert. Recherche CI confirme ne tenir aucun
créneau ; le build C++ privé coordonné ne partage ni target ni lib distribuée.

## Deuxième préfixe : contrat du plan figé, avant compilation

Ajout de `dataflow::migrations::plan` et raccord public
`MigrationRunner::resolve_plan(inputs)`. Cette méthode scanne les mêmes fichiers
et réutilise le parseur de templates et le hash de GraphDefinition existants,
sans catalogue, DDL, réparation ou modèle. Identité déterministe : fichiers,
graphes résolus, variables ordonnées, faits résolus du fournisseur et descripteurs
source/cible (formats moteur/interne/applicatif, recette, configs et génération).
Les entrées et étapes du plan retourné sont immuables par son API.

`StorageGeneration::Unversioned` conserve un plan préliminaire inspectable et
refuse la readiness ; aucun zéro ni fingerprint de données inventé.
`qualify_storage` distingue source avant publication et cible publiée pour
l'identité exacte, sans recalculer la source depuis les métadonnées cibles ;
un avancement étranger refuse. Cette qualification pure ne certifie ni lecture
réelle des métadonnées ni exclusivité et ne donne aucun droit d'appliquer.
Aucune publication/journal applied nouvelle n'est implémentée.

Trois fixtures de contrat ajoutées : qualification source/cible/rejet étranger,
identité sensible aux variables/configs/faits/recette/génération et hash du graphe
résolu sensible aux variables, legacy non versionné inspectable mais non prêt.
**Pas encore compilées ou exécutées** : API/diff partagés avant prochain ciblé.
Ce sont des vérifications d'un nouveau contrat, pas un rouge produit déjà joué.
Admission RO réelle, raccord CLI/daemon avant modèle/RW et exclusivité restent
à construire ; le ticket demeure ouvert. Recherche FTS a terminé sa passe,
C++ prépare sa configuration privée sans Cargo.

Le ciblé source41cc287ef est passé : **31 réussis, 0 échec, 0 ignoré**, rc0,
natif seul, compilation24,47s, fin11:18:54Paris, target1,3Go.
Preuve luciepc `cx-preflight-plan-v1.log`. Les3fixtures de contrat sont vertes ;
les5témoins du journal restent verts. Aucun verdict RO/exclusivité déduit.

## Troisième préfixe : inspection et admission, à relire avant compilation

Extraction sans duplication du lecteur strict du journal en `read_history_on`,
appelé par le catalogue existant et le nouvel inspecteur. Inspection native par
`Rag3dbConnection::read_only`, inventaire des tables et lecture des métadonnées
sans Catalog ni modèle. Configs persistées désérialisées puis empreinte dédiée,
explicitement distincte d'une génération de données. Pas d'inventaire du corpus.
`PreparedBackend::{inspect_storage,admit_storage}` est partagé par daemon et
les5ouvertures CLI avant embedder/RW ; commande réelle
`migration inspect --manifest <backend.json>`. Aucune méthode ne publie le
nouveau descripteur. Un manifeste peut demander `storage` (format applicatif et
recette) : cible non publiée ou publication non validable refusée ; les gabarits
livrés n'activent rien. Le legacy sans génération reste admis ; journal pending,
failed, inconnu ou malformé refuse. Running reste actuellement refusé strictement
comme état non pris en charge, pas interprété comme succès.

Deux fixtures callback couvrent admission legacy/cible et états du journal avec
zéro requête autre que l'inventaire et les lectures prévues. **Non compilées,
non exécutées**. Ce préfixe ne prouve pas encore le parcours natif sur fixture,
l'absence de connexion modèle par un appel produit instrumenté, le plan CLI,
la revalidation sous exclusivité ou la résistance à une course entre inspection
RO et ouverture RW. Ces points restent ouverts avant tout verdict du patch1b.
Aucun nouveau Cargo pendant le créneau C++02.

Suite de la frontière commune (non compilée) :
`PreparedBackend::connect_admitted_storage` effectue admission RO, ouverture du
writer natif exclusif du moteur, puis nouvelle inspection/admission avant de
rendre la connexion au client. Daemon et les5portes CLI passent par cette même
méthode **avant** `embedder_or_absent`. Aucun verrou TTL présenté comme exclusion.
Cette méthode ne fait aucun apply et n'établit pas à elle seule un protocole de
migration durable ni la qualification du plan sous cette garde.
Deux témoins natifs synthétiques ajoutés (non joués) : refus cible sur ancien
format préservant les octets de chaque fichier de fixture ; cible absente refusée
sans créer le répertoire de base. Le cas d'une évolution entre les deux
inspections et le raccord du plan sous garde restent à prouver. Les compteurs
callback du précédent préfixe ne sont pas des compteurs de connexions modèles
sur ce parcours produit : ne pas les confondre.

Revue ED/C7 du modèle pur prise en compte : renommer le contrôle en
`validate_generation_preconditions` (il ne valide pas registre/ports/cycles ou
transport). Témoin complété : sérialiser les inputs source/cible figés, reconstruire
le plan depuis ces inputs et fichiers après publication simulée, retrouver même
identité et PublishedTarget sans rerésoudre la source depuis le stockage cible.
Les fixtures natives déclarent désormais le modèle jouet obligatoire du manifeste
(sans création/connexion à celui-ci). Admission déplacée avant tout mkdir des
callers ; schéma interne inconnu refusé et configs persistées validées par
`EntityConfig::validate`. `open_with_reason(conn, embedder)` refuse aussi toute
cible versionnée avant création du catalogue ; cette API reçoit des capacités
déjà créées et ne promet pas leur admission rétroactive. Le contrat pré-RW et
pré-modèles appartient à la façade produit commune. Revue/témoins restent requis.

Ciblé admission source**6f59d7093** propre natif seul : **4 réussis, 0 échec,
0 ignoré**, rc0 ; compilation18,45s, fin11:25:48Paris, target1,4Go.
Preuve luciepc `cx-preflight-admission-v1.log`. Les deux fixtures natives ont
bien leur déclaration inerte d'embarquement avant ce run ; aucun rouge produit
n'est revendiqué pour cette correction de montage. Le refus cible sur ancien
stockage a préservé exactement les fichiers de fixture ; la cible absente n'a
créé aucun répertoire. Pas encore de preuve d'appel binaire ni d'exclusion.

## Raccord du plan préliminaire, à vérifier

Commande `migration plan --manifest <backend.json> --dir <migrations>
[--var key=value]` : inspection RO réelle et faits de schéma composés par le
fournisseur commun, variables explicites, étapes via scan/parse/hash du runner
existant. `PreliminaryMigrationPlan` se distingue du plan résolu figé d'exécution :
il expose source inspectée, cible déclarée, configs/relations cibles, graphes et
variables, génération Unversioned si absente et raison de refus. Aucun fingerprint
de configs n'est une génération. Le CLI `migration apply` refuse explicitement
avant chargement/ouverture. Aucune activation/transformation/publication ajoutée.
Les effets de `workspace.index_signals` sont reflétés dans les faits exposés ;
leur validation s'appuie sur les types/signaux existants. Ce raccord reste
**non compilé/non joué** au moment de publication pour revue avant ciblé produit.

Nouveau témoin avant correction du chemin bas niveau :
`raw_open_refuses_failed_journal_before_catalog_effects` passe par
`PreparedBackend::open(conn, embedder)`, journal failed, callback autorisant
uniquement inventaire/lecture du journal et comptant toute autre requête ainsi
que les appels d'embarquement. Il exige refus et compteurs zéro. Pas encore joué ;
il vise l'accès direct qui reçoit déjà une connexion et ne peut pas promettre
une admission rétroactive pré-RW, mais doit refuser avant Catalog.initialize.
Le montage de compilation produit07293 reste figé pendant sa fin ; aucun second
Cargo en parallèle.

Premier ciblé brut b76 :1vert, mais **montage insuffisant**, pas preuve de refus
d'admission. Analyse : CallbackEmbedder se nomme `callback` et manifeste disait
`fixture`, donc l'identité de modèle refusait avant le catalogue. Correction du
témoin : nom attendu callback et exigence d'un refus nommé « migration
reconciliation required ». Rejouer cette source avant tout correctif produit ;
le vert b76 ne sera pas réattribué à la frontière d'admission.

Rouge réel bas niveau source**b9d2e273b** : **0 réussi, 1 échec, 0 ignoré**,
rc101, `cx-preflight-raw-rouge-v2.log`, fin11:31:15Paris, compilation18,41s.
Refus observé : « catalog effect before admission » au lieu du refus de
réconciliation ; le callback a reçu une requête non autorisée avant lecture du
journal. Correction : `open_with_reason` inspecte/admet la connexion fournie
avant toute construction de catalogue, résolution de modèle ou initialisation.
L'API ne prétend pas annuler l'ouverture RW/connexion modèle faite par son appelant.

Revue portée de l'empreinte : l'inspecteur expose désormais
`entity_config_fingerprint`, explicitement sans relations ni données ; ce champ
ne sert ni de fingerprint du schéma complet ni de génération. Le plan préliminaire
expose séparément les relations cibles et ne gagne aucune autorisation d'apply.
Pas d'exclusion applicative globale inventée depuis le verrou du moteur.

## Vérification courante et revue du premier patch

Source de code **d6fc63f8bebd9bd6b2c0a6b81bc5f6c84accb283**, branche
`cx-arbre-migration-preflight-2`, worktree local/distant
`/home/lucied/.cache/rag3weaver-build/cx-wt-migration-preflight`.
Pas de Cargo actif après ces contrôles ; poste0/4 relu. Aucun master demandé.

- `cx-preflight-bornes-vert-v1.log` : **5 tests inspection/admission +31 runner,
  zéro échec/ignoré**, puis build binaire natif+daemon+code réussi, rc0 global,
  fin11:33:09Paris. Un seul Cargo à la fois sous poste lourd, j8/Mem32,
  échéance globale1800s, lib29a7 épinglée, target2,9Go.
- `cx-preflight-cli-v2/result.json` : **8 appels réels** sur le binaire de cette
  source : création legacy synthétique, inspect, plan avec variable résolue,
  quatre refus cible (positionnel, MCP standalone, serve, files verify), apply
  refusé avant chargement d'un manifeste absent. Fichiers de fixture inchangés
  entre inspection/plan/refus. Pas de modèle/service démarré ; ce constat ne
  remplace pas un compteur d'appels réseau sur chaque chemin.
- Binaire SHA256 : `e8a31d6e92fa431511db8301246f47dd5ecfe6d9dfac5ed2899b44ce64c8a0ab`.
  Log ciblé SHA256 : `b613b134be26ae4fb6e49e41187f3066a7af267ee8933261207b9835fd1fd57a`.
  Résultat CLI SHA256 : `472cf5f2cf225e5810f5cd58aae7725e396b7795293277ea9b8037d6d12d613a`.
- Rouge brut b9d2 puis vert d6fc : journal failed refusé avant effets de catalogue
  et appels d'embarquement (deux compteurs zéro sur le vert). Le faux montage
  b76 est conservé distinctement, sans attribution au produit.
- `git diff --check` propre ; vérificateur tickets :373 en-têtes valides.

Limites à faire relire avant gel/passe : aucun transfert, publication atomique,
état running écrit ou génération legacy établie. Le plan CLI est préliminaire,
non exécutable ; le modèle figé source/cible est qualifié dans les fixtures pures,
pas contre un journal résolu publié réellement. La connexion commune conserve
l'exclusion writer native pendant la seconde inspection, mais ce n'est pas
une garde SQL globale et la qualification d'un plan de migration sous cette
exclusion n'est pas encore raccordée. L'API bas niveau refuse avant initialisation
mais ne peut pas annuler des capacités déjà créées par son appelant. Les tests
ne couvrent pas un second processus concurrent pendant la qualification.
Le ticket reste ouvert ; aucune passe complète ou livraison master autorisée
par ces ciblés. Les preuves ne valent que pour les sources citées.

## Dernière édition de contrat, après les preuves d6, non compilée

`PlanJournalState` remplace le paramètre Option d'identité applied de la
qualification pure : Unapplied, Running, Failed, Applied et RolledBack sont
explicites. Running/Failed/RolledBack refusent la qualification normale et exigent
réconciliation ; ils ne peuvent plus être effacés en `None` dans ce contrat.
Le témoin existant source/cible vérifie ces refus sans nouvelle matrice de suites.
Ce type ne migre, n'écrit et ne qualifie pas le journal legacy : son raccord à un
journal résolu durable reste requis. L'inventaire refuse aussi un nom de table
vide. **Aucun Cargo lancé pour cette édition**, poste réservé à la mesure C++.
Les36verts, le binaire et les8appels CLI restent attribués uniquement à d6.
Preuves d6 préservées hors cible sous
`/home/lucied/.cache/rag3weaver-build/cx-preuves-preflight-d6fc63f8b/` :55fichiers,
manifest `sha256.json`, binaire SHA identique à celui publié ci-dessus.

Dernière consolidation statique de la frontière modèles, non compilée :
`PreparedBackend::open_on_disk` compose la connexion admise, la création de
l'embarqueur, puis l'ouverture bas niveau. Daemon/MCP/serve/files partagent ce
parcours ; positionnel/réouverture gardent l'accès à la connexion pour leur
crochet existant, toujours après connect_admitted_storage. La fabrique interne
d'embarqueur peut être fournie au témoin natif existant : son compteur exige
zéro construction avant le refus cible, en plus des octets de fixture préservés.
Il s'agit du même chemin que le produit, pas d'un compteur de modèle reconstitué
dans le test. `serve` résout aussi son modèle de chat après ouverture admise.
Ce changement unifie l'ordre d'acquisition et rend sa frontière testable ; il
n'ajoute aucun fournisseur ni moteur de modèles. Source d6 et ses preuves sont
conservées ; aucun Cargo pendant la mesure C++.

## Raccord plan/historique demandé en revue, témoins avant correction

C7 demande le même snapshot pour diagnostic/plan et la qualification commune
fichiers/versions/noms/checksums. Un applied legacy sans identité résolue durable
ne peut pas devenir « identique » sur sa seule checksum. L'exclusivité d'exécution,
running durable et publication restent des prérequis de la future tranche
exécutable, **hors garantie du diagnostic courant** ; aucun verrou décoratif ajouté.

`tests/e2e_migration_preflight.rs` pose cinq témoins du vrai binaire, avant le
raccord : fichier appliqué modifié, orphelin, failed, état inconnu et applied legacy
identique mais sans identité résolue. Chaque témoin construit sa propre base
native synthétique (DDL/DML de montage uniquement), invoque le CLI plan et exige
refus nommé + octets de tous les fichiers inchangés. Modèle déclaré inerte, aucun
service lancé ; la fabrique de modèle de l'ouverture produit est comptée par le
témoin natif unitaire existant. Pas encore compilés/joués pendant le créneau C++.
L'état inconnu devrait déjà être refusé ; les quatre autres visent le raccord
manquant, sans revendiquer de rouge avant exécution.

Les nouveaux témoins sont montés dans le **binaire e2e unique** par le générateur
`tests/e2e/generer.py` (autotests=false), sans second binaire de suite ni modification
manuelle du fichier généré. Le ciblé sera `--test e2e e2e_migration_preflight::`,
avec features natif/daemon/code et un seul thread de test pour les petites
fixtures. Rien n'a été lancé pendant la mesure C++.

Relecture pendant compilation ae6 : le DDL de montage doit déclarer `_uuid` ;
`SchemaDialect::create_table` désigne cette clé mais ne l'ajoute pas lui-même.
Correction du montage avant le prochain ciblé. La source ae6 en cours reste
figée ; son résultat ne pourra pas constituer un rouge produit si le montage
échoue avant appel CLI. Aucun correctif du raccord plan/historique n'est appliqué.

Le ciblé ae6 s'est arrêté **à la compilation**, rc101, fin11:47:26Paris,
target3,7Go : le regroupement d'ouverture passait `Arc<PreparedBackend>` à la
fonction de chat qui attend une référence. Emprunt corrigé ; aucun témoin CLI
n'a été joué dans ce montage, aucune preuve rouge produit revendiquée. La clé
_uuid du DDL de fixture est aussi corrigée dans la source suivante. Le raccord
plan/historique reste inchangé avant le rouge à obtenir.

## Rouge CLI réel puis raccord commun

Source**2684f2415** propre, `cx-preflight-cli-histoire-rouge-v2.log` :
**1 réussi, 4 échecs, 0 ignoré**, rc101, fin11:49:12Paris, target3,9Go.
L'état inconnu est refusé ; les quatre autres cas renvoient0 avec un plan
préliminaire malgré l'historique incompatible. Toutes les comparaisons d'octets
précèdent ces échecs et passent. AE6 reste un montage non compilé, distinct.
Job terminé avant la consigne de priorité mesure reçue ; aucun Cargo relancé.

Correction publiée après ce rouge, pas encore compilée :

- L'inspection produit ouvre une transaction **read-only** pour garder tables,
  métadonnées/configs et journal dans le même snapshot ; commit/rollback explicite.
  Ce snapshot ne revendique aucune exclusivité d'exécution.
- Le journal strict est conservé dans `StorageInspection` pour le raccord interne,
  sans relecture de catalogue ni second parseur. Le CLI plan donne ce même journal
  à `MigrationRunner::resolve_inspected_steps`.
- `validate_declared_history` est extrait du runner existant : mêmes vérifications
  version/fichier/nom/checksum pour status/pending et le plan public. Les fichiers
  scannés une fois sont ceux dont le contenu sert ensuite à résoudre les graphes.
- Après cette vérification commune, un journal legacy applied identique est
  **non qualifiable** : le format existant à huit colonnes ne porte pas d'identité
  résolue durable. Aucune checksum n'est promue en identité et aucune clé méta
  parallèle n'est inventée. Les autres états exigent réconciliation. Le diagnostic
  historique legacy de pending garde son comportement compatible ; il ne qualifie
  pas un plan résolu.

L'identité résolue durable, son lecteur versionné, running avant effets et sa
publication atomique restent des prérequis de la future tranche exécutable.
Le modèle pur avec états typés reste distinct de ce format legacy. Unversioned
reste inspectable/non prêt quand aucun historique ne rend le diagnostic invalide.
Aucun transfert, modèle, service ou verrou décoratif ajouté ; pas de passe
complète. Nouveau ciblé seulement après libération de la mesure C++.

Relecture sans build pendant mesure : le plan et l'ouverture partagent aussi
`validate_internal_schema`, avec refus du schéma99 dans le témoin callback
existant (pas de suite ni matrice supplémentaire). La conversion des signaux
workspace en faits de plan/enregistrement réutilise maintenant le contrat Serde
existant de SearchSignals, au lieu de deux match manuels. La politique du manifeste
reste celle du chargement existant ; aucun signal/profil/gabarit ajouté.
Ces dernières éditions restent **non compilées** ; le prochain ciblé portera
leur SHA figé, pas celui des preuves d6.

## Vert du raccord public, source figée85ef

Source**85ef4921128b445e74766c1e628809b206957485**, propre :
`cx-preflight-raccord-vert-v1.log` termine rc0 le12nov à11:57:50Paris.
**5 inspection/admission +31 runner +5 CLI =41 réussis, zéro échec/ignoré**,
features rag3db-native/daemon/code, un Cargo à la fois, même target4,2Go,
j8/Mem32/lib29a7, délai global1800s. Les cinq témoins CLI atteignent maintenant
les refus nommés attendus ; chaque fixture conserve tous ses octets. La fabrique
d'embarqueur reste à zéro construction lors du refus natif ; le bas niveau garde
ses compteurs d'effets et d'embarquement zéro. Les états typés et la reconstruction
source/cible figée sont vérifiés par les tests du runner existants complétés.

`cx-preflight-cli-v3/result.json` ajoute les **8 appels normaux réels sur85ef**,
rc0 du contrôle : création legacy, inspect/plan préliminaire avec variable,
quatre refus cible et apply indisponible. Octets inchangés entre diagnostic et
refus. Exécutés sous poste lourd/timeout300, sans Cargo, après les ciblés et avant
le nouveau créneau de mesure C++. Aucun modèle/service/transfert démarré.

Le rouge2684 (1vert/4échecs) et ses binaires sont préservés séparément sous
`cx-preuves-cli-rouge-2684f2415/` (manifest sha256.json) ; l'ancien montage AE6
non compilé y reste explicitement distinct. Les preuves d6 antérieures restent
sous leur propre identité. Aucun master/fullpass ni1b complet annoncé.

Reste avant passe adaptée : revue de ce préfixe et compatibilité de compilation
minimale sans natif (méthode privée de qualification et champ historique lus par
le seul raccord natif). Aucun job supplémentaire pendant les quatre points
mécanismes C++ suivants. La tranche d'exécution durable est toujours hors garantie :
identité résolue persistée, génération legacy établie sous exclusivité, running,
transfert/publication/réconciliation. Le diagnostic ne crée aucun de ces droits.

## Comparaison minimale et correction cfg bornée

Check autorisé `--lib --no-default-features`, même WT/target, sans nouvel outil
ou cible : source85ef **rouge rc101**, fin12:08:30Paris, exactement deux nouveaux
dead_code (`resolve_inspected_steps`, champ history),
`cx-preflight-minimal-v1.log`. Baseline**1cf24f671** propre avec la même commande :
**verte rc0**, fin12:10:09Paris, compilation5,33s,
`cx-preflight-minimal-baseline-v1.log`. Cette régression minimale est donc celle
du préfixe, pas une panne de baseline. Le [ticket tty historique](../../tty/tickets/2026-11-07-la-lib-rag3weaver-ne-se-batit-pas-sans-rag3db-native.md)
reste ouvert : sa recette daemon et ses tests sans natif n'ont pas été vérifiés,
et aucune matrice ou décision de default features n'est ajoutée ici.

Correction : seuls la méthode privée de raccord natif, le champ privé de journal
retenu pour elle et son initialisation sont sous cfg(rag3db-native). Les types du
plan pur et de déclaration storage restent communs. Pas d'allow(dead_code),
d'installation de cible, changement de feature par défaut ou correction étrangère.
SHA neuf à vérifier ; les41verts/8CLI et binaires85ef restent immuables sous
`cx-preuves-preflight-85ef49211/` (28 fichiers, manifest sha256.json).
Binaire produit85ef SHA256 :
`0b59e5f6ef0c5094c9df2716d8a72ad680ef48584e4bc90304c57a370f1643e9`.

Le check minimal corrigé **8ff52782bf881964ef7ed2de304b8f5a6a1f20f4** est
**vert rc0**, compilation5,38s, fin12:11:57Paris,
`cx-preflight-minimal-vert-v1.log`. Défaut attribué puis corrigé, sans élargir le
ticket tty ni prétendre avoir joué sa recette daemon. ED/C7 ont relu85ef sans
bloqueur et le delta cfg8ff est relu sans bloqueur ; revue supplémentaire ED non
requise pour ces seuls cfg. Aucun autre Rust modifié après ce contrôle.

## Proposition de passe finale du préfixe diagnostique — non lancée

Rebaser la branche privée sur masterb28b55db6 avec nom neuf avant la passe :
les six commits reçus depuis1cf ne changent que le témoin FTS cfg(test) et deux
MD ; conserver range-diff et preuves antérieures sous leurs SHA. Pas de tour
master implicite. Sous poste lourd à luciepc : un target32Go (actuellement4,5Go),
un Cargo à la fois, j8/Mem32, lib29a7 épinglée, borne **globale3600s** couvrant
la construction et tous les tests. Arrêt de tout le groupe propre à l'échéance,
aucun retry aveugle ; journaux/compteurs/SHA conservés hors cible.

1. Cinq jeux réglementaires `scripts/check_features.sh`, plus contrôle ciblé
   `--lib --no-default-features` sur la source finale.
2. Lib entière sous les huit features réglementaires, sans service d'embarquement
   dans son environnement. Fixture PostgreSQL neuve privée par l'infrastructure
   existante ; aucun serveur ni modèle nouveau, aucune donnée de base réelle.
3. Étape navigateur exigée, huit features, paquets React existants du poste.
4. Douze suites du changement, une suite par processus via le runner existant,
   sous les huit features : `e2e_migration_preflight`, `e2e_demon_hote`,
   `e2e_mcp_cycle_de_vie`, `e2e_mcp_stdio`, `e2e_fichiers_entite`,
   `e2e_fichier_compose`, `e2e_lectures`, `e2e_postgres`, `e2e_tx_par_paquet_arret`,
   `e2e_tx_merge_puis_copy`, `e2e_tx_ligne_a_ligne`,
   `e2e_tx_vecteurs_apres_rollback`. Lectures est requise par l'hôte, transactions
   par le catalogue ; PostgreSQL est requis car le lecteur de journal commun
   change également sur son dialecte. Pas de suite code/IR/gabarit ou banc ajouté. Les services
   d'embarquement préexistants ne sont consultés explicitement que pour lectures
   et MCP stdio (qui exige la connexion, sans calculer de vecteur). Santé/profil
   à vérifier avant usage : pas de nouveau service, remplacement ou lancement
   de modèle pour faire passer un manque. Cette non-régression est distincte des
   témoins de migration qui restent sans modèle ni effets.
5. Lire N/N, les tests réellement joués et résidus par suite, attester les assets
   du dépôt dans les binaires réels, SHA des binaires/source/journaux, revue finale.
   Nettoyage de la fixture PG privée et des ressources propres ensuite selon
   la procédure ; les preuves restent hors target.

Cette proposition attend la décision C7 sur la passe adaptée ; rien n'est
annoncé vert à l'avance. Le ticket migration reste ouvert : publication, transfert,
génération établie, running durable, reprise/annulation et activation ne sont pas
livrés par ce préfixe diagnostique.


Autorisation C7 après revue ED : passe adaptée ci-dessus **autorisée avec
l'ajout e2e_postgres**, soit12suites. Figer ce MD puis exécuter sur le SHA
actualisé ; premier rouge arrête pour diagnostic, sans retry automatique ni
nouveau service/modèle. Global3600s après acquisition poste lourd, j8/Mem32,
target32Go/lib29 ; fixture PG neuve sur serveur existant, sparse lectures compris.
Pas d'activation/apply ni de tour master. Le ticket migration reste ouvert.

## Passe adaptée v1 : premier rouge et correction d'ordre

Source**9fb30e53a34e0f88aadb3d6a67136074f2d25dc1**, portée bornée
`cx-preflight-final-1b-v1-9fb30e53a.scope`, global3590+stop5<3600, j8/Mem32.
Features **5/5 vertes**, minimal vert ; bibliothèque **2149 réussis,3 échecs,
64 ignorés**, rc101. Arrêt immédiat du lanceur à la fin de cette étape, pas de
navigateur ni e2e lancés, aucun retry automatique. Portée inactive, poste0/4 relu.
Preuves `cx-passe-preflight-1b-v1/{lib.log,etat.json,source.json,VERDICT}` ; base
privée `cx_preflight_2191_20261112_v1` gardée pour le diagnostic, serveur préexistant.

Les trois échecs existants de `backend::opening_tests` sont de vrais rouges
métier : `opening_manifest_validates_the_whole_plan_before_any_database_call`,
`opening_named_schema_refuses_conflicting_relations_before_any_database_call` et
`opening_public_content_manifest_refuses_invalid_associations_before_database`.
Le nouveau BEGIN read-only précédait les validations pures des faits/associations.
Deux témoins comptent3appels au lieu de0 ; le troisième voit le mauvais refus
« begin metadata snapshot » au lieu de l'association invalide nommée.

Correction bornée : extraire `opening_schema` depuis la composition/registration
order existantes et l'appeler avant le snapshot dans l'API bas niveau, ainsi
qu'avant toute inspection native dans la façade. Le plan préparé est revalidé,
même si un appelant a modifié ses configs publiques après load. Aucun catalogue,
modèle ou accès base n'est requis pour ce contrôle ; le refus du journal reste
après cette validation et avant les effets. Pas de flag de contournement ni
nouveau validateur de schéma. Ciblé des opening_tests et des gardes du préfixe
à jouer avant toute nouvelle passe finale, sur SHA neuf.

La revue C7 confirme la correction et autorise le ciblé opening_tests +
storage_inspection + migrations sur SHA neuf avant toute reprise finale.
Le témoin existant d'association publique invalide est complété pour la façade :
config préparée puis modifiée après load, refus Missing nommé, fabrique modèle
non appelée et fichier/répertoire de base toujours absent. Le chemin bas niveau
conserve son compteur de zéro appel DB ; aucune nouvelle suite/matrice ajoutée.

Le ciblé44c s'arrête à la compilation (rc101,12:55:28Paris,
`cx-preflight-ordre-vert-v1.log`) : l'extraction avait perdu la valeur
`provided_schema`, encore utilisée pour éviter de réenregistrer les entités du
fournisseur et pour la libérer après enregistrement. Aucun test joué, aucun
nouveau rouge métier revendiqué. Le résultat de la primitive conserve maintenant
ensemble le fournisseur original, les faits composés et l'ordre validé ; raw les
réutilise tous, sans recalculer le fournisseur ni introduire un second algorithme.
Même ciblé à rejouer sur cette correction explicite, pas de passe finale relancée.

Vert ciblé après correction explicite, source**7a1aed927** propre :
`cx-preflight-ordre-vert-v2.log`, rc0, fin12:58:38Paris, huit features,
j8/Mem32, global1800s, target8,7Go. **18 opening_tests +5 inspection/admission
+31 runner =54 réussis, zéro échec/ignoré**. Les trois rouges métier9fb sont
verts, les contrôles d'ouverture valides aussi. Le témoin de façade modifiée
après load confirme Missing nommé, fabrique modèle0 et base toujours absente ;
le journal failed continue de refuser avant effets. Le montage44c non compilé
reste distinct. Pas de nouvelle passe complète lancée avant revue de ces verts.
La portée v1 est inactive et le poste rendu ; la PG privée v1 et les preuves
source9fb sont conservées sans les attribuer au nouveau code.

## Passe v2 : arrêt sur résidu de la suite hôte

Source**3377cdd84890770702fe9201c26cf457b54f24ce** : features5+minimal verts,
lib**2152/0/64**, navigateur**39/0/0**, migration_preflight**5/0/0**. Hôte :
**13/0/0** puis garde de TMPDIR rouge,24Ko de livres de manifestes privés.
Arrêt rc1 du lanceur à13:10:52Paris, aucune autre suite jouée, portée finie/poste0/4.
Le [ticket de résidu dédié](../../tickets/closed/2026-11-12-le-dossier-d-etat-d-e2e-demon-hote-reste-dans-son-tmpdir.md)
porte l'attribution baseline et le nettoyage possédé ; pas d'exception ajoutée ni
reprise automatique. Preuves `cx-passe-preflight-1b-v2/` et dossier e2e-gardes
nommé dans ce ticket. Les deux PG privées restent gardées, aucune base réelle.
README/nouveautés restent en copie hors worktree jusqu'à une passe complète verte.

## Passe finale diagnostique v4 — verte, avant tour master

Rebase sur masteraa741a4c3 (MD seules depuisb28), branche neuve
`cx-arbre-migration-preflight-4`,37/37 commits identiques au range-diff.
Source figée propre **e4239bddae8dedc1db7543a32ffe802822c0e1d7**.
Le raccord du témoin état dans check_features est réellement joué dans cette passe.

La tentative **v3 est un refus de montage**, avant tout Cargo et toute création PG :
clé tuple de run_env introduite par remplacement trop large lors de l'ajout de
GARDER à la liste des variables à retirer. Arrêt immédiat38ms, scope inactive/poste0/4.
`cx-passe-preflight-1b-v3/etat.json` conserve ce refus. Driver v4 neuf : correction
clé tuple→str, contrôle AST des clés et construction de l'environnement avec vrai
subprocess /usr/bin/true verts, aucun service ni dossier de passe dans ce contrôle.
C7 a relu ce delta puis autorisé explicitement v4 : aucun retry automatique.

**v4 rc0 en612,81s**, borne globale3600s après lourd, portée3590+5, j8/Mem32,
target unique11Go (<32Go), lib29 inchangée. Cinq jeux de features et minimum verts ;
lib**2152/0/64**, navigateur**39/0/0**. Douze suites, chacune1/1 demandée,
**125/0/0** au total (hors deux filets rejoués en tête de chaque processus) :

| Suite | réussis | échecs | ignorés |
|---|---:|---:|---:|
| migration_preflight | 5 | 0 | 0 |
| demon_hote | 13 | 0 | 0 |
| mcp_cycle_de_vie | 11 | 0 | 0 |
| mcp_stdio | 4 | 0 | 0 |
| fichiers_entite | 3 | 0 | 0 |
| fichier_compose | 4 | 0 | 0 |
| lectures | 24 | 0 | 0 |
| postgres | 36 | 0 | 0 |
| tx_par_paquet_arret | 18 | 0 | 0 |
| tx_merge_puis_copy | 1 | 0 | 0 |
| tx_ligne_a_ligne | 3 | 0 | 0 |
| tx_vecteurs_apres_rollback | 3 | 0 | 0 |

Seul résidu accepté : deux CSV24Ko de tx_par_paquet_arret, ticket recherche11nov
déjà déclaré ; aucune exception ajoutée. Les autres suites ne signalent aucun
résidu. Ce n'est donc pas une affirmation de zéro résidu pour toute la passe.
Santé du service7879 validée au départ, avant MCPstdio et lectures ; pas de modèle
ni serveur créé. Base privée `cx_preflight_2191_20261112_v4` conservée pour revue,
v1/v2 également ; aucune v3 créée. Portée finale inactive, poste libre0/4 relus.

Preuves luciepc `/home/lucied/.cache/rag3weaver-build/cx-passe-preflight-1b-v4/` :
source.json, source-head, verdict-detaille.json, logs par phase, binaires conservés
et binaires.sha256.json. ELF identifiés depuis les lignes réelles de Cargo/lanceur
(les douze chemins e2e concordent), sans heuristique de date :

| Binaire | SHA256 |
|---|---|
| lib | 0673d180af2ccfac257ee0e936f4dcd8336edc8f6db862fa875ec20ee5d5c5fd |
| navigateur | 9c000319ddf3a5200c36d98738493734bd08ca4515f98f9c2cb30dce548203a5 |
| e2e | ae96a1c56a83bd740ccee8b7881cd9e46bba722f36bfb0910c3926d3037f293b |
| backend | ccae63f68a55c925190e0457b87c30d10e064c62f8327cc454f764515128771d |

Attestation ELF du navigateur :220assets attendus présents, table issue de son
.d, app.jsfff528ffdcbfb08c022c1249d0ec5ecc82a4caf4a010eaad4447ca7c3081bc00,
bundle6d317a42f06655af8d59d934bd6bb4ac4aba7be82794365e1c6a6e027f3fba43.
Driver SHAaa3ac4de03cf16b1de7bc30ff357c3da4ac497e08edd440e02e718cea1a1bb95 ;
launcher6c19305532bf52d64a1eb27407a6ea08896991e032cb4d4cafd8448f5ab130c9.

README et nouveautés décrivent le préfixe diagnostic seulement. Ticket migration
**toujours ouvert** : ni producteur d'identité durable ni génération publiée,
transfert, activation, exclusion d'exécution ou reprise de migration livrés ici.
Ticket hygiène : clôture préparée sur branche, intégration master encore à faire.
Attente lecture C7 du verdict et tour explicite « à toi » ; aucun push master.

## Intégration du préfixe et nettoyage

Tour C7 reçu après revue indépendante des logs et rehash des quatre binaires.
Master **2b06682a2e28e7ef92641de5dc80c91cf986d799**, rebase MD seules sur4c4a5b691,
branche neuve5, code identique à sourcee4239. Trois contrôles lus : status propre,
aucun rebase, origin/master=HEAD. Ticket hygiène désormais intégré fermé.

Script terminé rc0 : worktrees local/remote retirés après copies cp-a fidèles
administrations (sous-module inclus remote), helperdca13 et rapports frais VERT lus ;
target11Go retiré. PG privéesv1/v2/v4 exportées+SHA puis drop, absence vérifiée ;
v3 absente. Cache `cx-nettoyage-preflight-1b-{local,remote}-2b06682a2/` contient
copies, garde.json, fin.json et dumps sur remote. Preuves/ELF/logs et branches
privées distantes conservés. Aucun déploiement ou rerun CI. **Migration ouverte** :
ce lot ne transforme aucune ancre existante ; repartir d'un worktree neuf pour la suite.

## Reprise parallèle autorisée — préparation du transfert générique

Orchestration, après accord Lucie sur la parallélisation : préparer la prochaine
tranche BM25+dense sur fixtures synthétiques ; revue du périmètre avant Rust.
Master6e5f9458c, nouveau worktree local `cx-wt-migration-transfert`, branche
`cx-arbre-migration-transfert`. Aucun distant, target, Cargo, transfert ou base créé.
[Proposition API/inventaire/invariants/témoins/budget](../12-11-2026/02-preparer-le-transfert-des-valeurs-connues.md).

Premier diff proposé : aperçu pur non exécutable dans migrations/transfer.rs et
mod/tests voisins, sans conn/embedder ni modification backend/CLI/stockage.
Compose les identités/rendus/découpe1a et préflight ; KnownVectors et RecordVectors
actuels ne prouvent pas un transfert multi-modèle. Identité d'instance source et
génération durable absentes restent des refus, pas des hashes inventés. Sparse
se décide sur faits à transférer, pas sur models.sparse seul ; refus avant effets.

Possession actuelle : ce ticket et le nouveau document. Fichiers Rust proposés
mais **pas encore modifiés/réservés** : migrations/transfer.rs, migrations.rs et
leurs tests ; extraction record_nodes éventuelle à soumettre avant modification.
ED garde sa correction d'accord ; orchestration tient la revue commune. Budget
proposé ciblé600s/j8/Mem32/target32 après revue, aucun lourd réservé ou lancé.
Ticket reste ouvert ; aucun transfert/apply/publication/migration réelle autorisé.

### Premier diff pur autorisé, avant ciblé

Autorisation C7 après revue21d09 : migrations/transfer.rs, déclaration et tests
uniquement ; aperçu sansDB/modèle/écriture. Nuance fixée : identité étrangère
comparée à un attendu explicite, absent du plan actuel ; déclaration d'inventaire
ne devient pas preuve de stockage. Quatorze tests métier écrits, pas un rouge
fondé sur l'absence d'API. Aucun test exécuté à cette étape.

Prévision du ciblé : scripts/cargo_magasin.sh test --no-default-features --lib
-j8 dataflow::migrations::transfer::tests:: -- --test-threads=8. Target de session
cx-target-contenu-derive (neuf après nettoyage du lot précédent), lib29, j8/Mem32,
borne600s après lourd ; aucun modèle, GPU, service ou base. Source/argv publiés
avant lancement ; premier échec diagnostiqué, aucun retry inchangé. Diff partagé
avant passe complète ou master. Fichiers communs à ED inchangés.

### Revue du lien entre aperçu et faits du plan

C7 pointe que la simple égalité des descripteurs déclarés ne qualifie pas les
configurations fournies contre config_fingerprint/schema_facts_fingerprint.
La primitive commune de liaison aux faits complets n'existe pas dans le plan
actuel. Correction locale pendant l'attente du ciblé0cc (source distante inchangée) :
report toujours PlanQualification::UnqualifiedConfigAndSchemaFacts, identité du
plan seulement référencée, aucun succès déclaré cohérent au plan. Quinzième témoin :
configs/recette changés à descripteurs constants restent explicitement non qualifiés,
avec empreinte de rapport différente ; pas de nouveau hash annoncé contrat commun.
Une qualification effective exigera les faits complets et un producteur commun
revu séparément. Aucun apply ou jeton d'écriture ne découle de cet aperçu.

### Ciblé minimal v1 annulé avant Cargo, cfg isolé intégré

C7 signale le défaut minimal déjà connu du ticket tty7nov : un témoin natif de
record_nodes importe Rag3dbConnection sans cfg. Sur instruction explicite, seule
l'attente poste de cette session (pid2034961, UID/cmdline vérifiés) a reçu TERM,
alors que la mesure C++ tenait toujours le poste. Scope inactive, dossier de
preuve et target absents avant/après : **source0cc jamais jouée**, budget non
démarré, pas de rouge produit. Preuve cache luciepc
cx-transfert-apercu-v1-annule-avant-cargo.json ; commande d'attente finie143.

Correctif mémoire **80faa48385ad4600122c545b876255a7c53050eb** relu : un seul
cfg(feature=rag3db-native) sur le témoin natif, rien d'autre. Commit privé publié
sur origin/cx-memoire-cfg-temoin-natif, intégré séparément sur branche neuve
cx-arbre-migration-transfert-2. Le ticket tty n'est pas fermé : son périmètre
build/daemon et gardes complètes n'est pas validé par cet unique cfg.

Deltafb0 nonqualification et218 refus des configs non finies également inclus ;
quinze témoins métier. C7 autorise le nouveau ciblé600 après ce raccord, même
argv --no-default-features --lib -j8 et filtre transfer::tests, j8/Mem32, target
unique32Go et lib29, mesure prioritaire. Source propre/hash/argv publiés avant
lancement ; aucun transfert, service, base ou globale/master autorisé.

### Ciblé v2 : compilation arrêtée sur deux helpers de témoins natifs

Source privée **390b70185d3c466b3d1c5379c2d14b5f36ee71b6**, cfg mémoire80faa
intégré séparément. Cargo test --no-default-features --lib, filtre aperçu transfert,
j8/Mem32/lib29 : **rc101 en112,69s, aucun test joué**. L'import natif manquant
n'est plus signalé ; deux helpers cfg(test) deviennent du dead_code fatal :
read_pool.rs:196 Instantane::vieillir et backend.rs:4005 Backend::call_tool.
Leurs appels sont dans les témoins natifs ; aucun de ces deux fichiers n'est
modifié par ce lot. Attribution statique à cette surface de montage préexistante,
pas de rouge métier du transfert, pas de preuve de correction globale du ticket.

Preuves luciepc : cx-transfert-apercu-cible-v2/{cargo.log,source.json,resultat.json,VERDICT}.
Source propre inchangée, portée inactive et place arbre rendue. Pas de retry
inchangé ni de allow(dead_code) ajouté. Coordination demandée à C7 : qualification
sélective des helpers par leurs propriétaires, ou ciblé natif du même aperçu
sans prétendre prouver le minimal ; aucun backend partagé modifié par arbre.

### Ciblé natif du même aperçu, autorisation séparée après diagnostic

C7 autorise une tentative sur source390 propre inchangée, même filtre15témoins,
ajout --features rag3db-native, nouvelles preuves v3,600 après lourd/j8/Mem32/
lib29/targetprivé32. Aucun allow ni correctif étranger : les deux helpers restent
la dette du ticket tty ouvert. Propriété backend.rs : mémoire ; ED possède les
commandes/run_nodes, pas ce fichier. Ce ciblé ne prouvera pas le minimal ni le
témoin natif liens filtré hors de l'aperçu. Pas de globale/master avant revue.

### Ciblé natif v3 vert — aperçu pur uniquement

Source propre **390b70185d3c466b3d1c5379c2d14b5f36ee71b6** figée, même code
que v2 ; ajout explicite --features rag3db-native, autorisé après diagnostic.
**15 réussis,0 échec,0 ignoré**,1609 filtrés ; rc0 en90,91s compilation comprise,
tests0,02s. Budget600 après lourd, j8/Mem32/lib29, target unique2Go. Portée inactive
et poste libre0/4 relus. Aucun modèle, base, transfert ou état persistant créé
par les quinze témoins ; uniquement des faits synthétiques et vecteurs littéraux.

Preuves luciepc `cx-transfert-apercu-cible-v3/` : source.json, cargo.log,
resultat.json, VERDICT, binaire.json et binaire-tests. ELF réellement nommé par
Cargo : rag3weaver-bcb55d57389f25b4 ; SHA256
**24e675e5c2998d7b913fe4027ee378f29e93299e78dd6ba88803b2f27dc5cfbd**.
V1 jamais jouée, v2 minimal compilation rc101 et v3 natif vert restent distincts.
Le ticket tty est ouvert ; le témoin natif liens n'a pas été joué par ce filtre.

Témoins : huit occurrences/six versions, autre entité que Scope, cellules séparées,
conservation positions/provenance/masques et relations/multiplicités, deux modèles4/6,
legacy/suffixed, permutations, collisions, trous et doublons de couverture,
marqueurs périmés, entrée modèle inconnue, NaN/infini/dimension et conflits bit à bit
(y compris signe de zéro), sparse/dette/contradiction vs lecteur seul, recettes et
gabarits non inline refusés. Mutation configs/recette à descripteurs constants :
rapport différent et toujours non qualifié, jamais cohérence au plan inventée.

Le module a une API de **rapport** publique, sans désérialisation de sortie ni
execute/apply/connexion/écrivain : referenced_plan_identity n'est qu'une référence,
PlanQualification::UnqualifiedConfigAndSchemaFacts est toujours sérialisé.
Binding des faits complets, identité/génération durables, lecteur RO sous
exclusivité, adaptateur multi-modèle, écritures/reconstruction/checkpoints et
publication/reprise restent des prérequis de la suite. Ticket migration ouvert.
Diff et preuves transmis à C7 avant toute passe complète ou push master ; aucune
globale, publication ou migration réelle autorisée à ce stade.

### Passe complète de l'aperçu autorisée, préparation du montage

Après revue du ciblé natif15/0/0, C7 autorise features5 + minimum en **cargo check
--lib --no-default-features** (pas les tests minimaux connus rouges), lib entière
sous huit features, navigateur attendu39 et migration_preflight attendu5.
Le témoin natif liens qualifié par80faa doit apparaître réellement dans la lib.
Global3600 après acquisition lourd, portée3590+5, j8/Mem32/target32/lib29 ; premier
rouge stop/diagnostic, aucune répétition inchangée. Aucun master sans revue/àtoi.

Rebase final sur master8cc7e023b : seul document de répartition déplacé/clos depuis
base6e5 ;10/10 commits identiques au range-diff, branche neuve
cx-arbre-migration-transfert-3. Source définitive à publier après cette MD ; pas
de nouveau Rust après le ciblé natif. Modèles/services/GPU supplémentaires interdits.

Montage proposé : Docker PostgreSQL existant rag3weaver-pg, nouvelle base synthétique
cx_apercu_2191_20261112_v1 (absence vérifiée, aucune base réinitialisée). Connexion
fixture fournie aux seuls processus de passe, sans affichage des identifiants.
Aucune variable de service d'embarquement ; React déjà présent au cache I-paquets-react.
TMPDIR navigateur créé par mkdtemp sous **/var/tmp/cx-pv.XXXXXX**, court/privé sur
disque (findmnt : btrfs), limité au processus navigateur ; conserver si rouge,
retirer seulement la racine possédée après succès. Éviter la limite des sockets
UNIX de Chrome, sans modifier tests, délais ou options de sécurité du navigateur.

Driver/launcher neufs cx-passe-apercu-transfert-v1, source propre/hash/argv/env
et contrôle AST/environnement à publier avant lancement. Artifacts lib/nav/e2e
pris depuis les lignes réellement exécutées ; attestation des220assets et SHA,
modèles4/6 de l'aperçu littéraux. La passe n'ajoute aucun transport/apply ni
qualification des configs/faits : aperçu toujours explicitement non qualifié.
Tickets migration et tty restent ouverts.

### Passe complète autorisée de l'aperçu — verte, avant tour

Source propre figée **20e4e2ffce839910fc3586b305ab0791e49c90ab**, branche3,
code identique au ciblé390 après rebase MD seules. **rc0 en687,38s**, budget3600
après lourd, portée3590+5, j8/Mem32/target32/lib29. Features**5/5** + minimum
**cargo check --lib --no-default-features** verts ; bibliothèque**2167/0/64**,
navigateur**39/0/0**, migration_preflight**5/0/0**,**1/1 suite**, sans résidu.
Les quinze témoins de l'aperçu et le témoin natif liens80faa sont explicitement
exigés et lus réussis dans lib.log. Les tests sans natif restent distincts et rouges
sur les deux helpers de la dette tty ; la passe ne les rebaptise pas verts.

Preuves luciepc `/home/lucied/.cache/rag3weaver-build/cx-passe-apercu-transfert-v1/` :
logs, source.json/source-head, verdict-detaille.json, VERDICT, binaires conservés
et binaires.sha256.json. SHA depuis chemins réellement exécutés :

| Binaire | SHA256 |
|---|---|
| lib | eed8b033b76f196f1363ccf02042cfd734124f3ddc7d9a2fe39b7d70e306d8f9 |
| navigateur | 0e75e80608e124cfb6f33a4066ca2634ab28f2b4c64c43dfc48298f511eb765e |
| e2e | 3d6bc6923b16c9d3143f3059705722c37870711d553aa71e3833053438c7cd2b |
| backend | c5517dc467e261baa0fe3a6cb4d445ec269f604d228d5e559bdd5488b9c1cc0f |

Attestation ELF du navigateur :220assets présents,0manquant, table issue du .d,
app.jsfff528ffdcbfb08c022c1249d0ec5ecc82a4caf4a010eaad4447ca7c3081bc00 et
bundle6d317a42f06655af8d59d934bd6bb4ac4aba7be82794365e1c6a6e027f3fba43.
Driver SHAd2194665910777dd205d9d10c897187c5c8643ede84b4f64c480a7b4fc944f17,
launcher e8bfb4ddeef3d70ef998b94199975488bee78f6756ef076cdebeb3687e0ed8b3.

Portée inactive/place arbre rendue, target8,4Go ; un lourd tiers tient encore
1/4 place, donc aucune affirmation de poste global libre. TMPDIR navigateur
/var/tmp/cx-pv.r3jqx4s9, sur btrfs, retiré uniquement après portée éteinte et
succès ; preuve nettoyage-tmp-navigateur.json. PG privée fraîche
cx_apercu_2191_20261112_v1 conservée pour revue puis dump/hash/drop autorisés après
intégration. Aucun modèle/service/GPU nouveau ni base réelle consultée.

README/nouveautés décrivent l'API de rapport, toujours
UnqualifiedConfigAndSchemaFacts, aucune capacité de transfert/apply. Tickets
migration et tty ouverts. Attente revue C7 puis tour explicite ; aucun push
master, déploiement, publication de stockage ou migration réelle à cette étape.

### Intégration et fin des ressources de l'aperçu

Master **3b2eab7ec76b2f1a8b56de0dedd99c80184de0e1**, tour explicite C7 après
revue indépendante. Rebase branche4 sur8839,13/13 commits identiques au range-diff,
aucun Rust différent de source20e4. Trois contrôles lus : status propre, aucun
rebase, origin/master=HEAD. L'aperçu non qualifié est intégré ; migration et tty
restent ouverts, aucun transfert/apply/publication ou déploiement exécuté.

Le premier nettoyage a refusé son préflight avant toute opération PG/admin/retrait :
son assertion « tous .md » était plus stricte que le périmètre docsCPP revu par C7.
168 chemins après source jouée,143 non-MD tous sous docs/coeur-cpp (preuves JSON,
scripts, archives), aucun code extension modifié. Contrôle Python exact enregistré
dans cx-preuves-finales-apercu-transfert-v1/delta-apres-passe.json, puis nouveau
montage/outputv2 ; aucune garde_worktree n'avait encore été invoquée ou refusée.
Piège du script de push également identifié : git diff | grep -v | grep -q sous
pipefail peut masquer un grand delta par SIGPIPE. Ne pas employer ce pipeline
pour une barrière ; collecter les chemins et juger la liste complète. La revue
indépendante et la vérification Python confirment ici l'identité du Rust livré.

Cleanupv2 achevé : PG privée unique cx_apercu_2191_20261112_v1 exportée, SHA256
364d0a9dfc90e887375a3c1342c34af3a69ac897248a69eb34c0de44d1add918, zéro session,
drop et absence vérifiés. Worktrees local/remote et target8,4Go retirés après
archives cp-a fidèles administrations (sous-module inclus remote), helperdca13
et rapports frais VERT lus avant chaque retrait. Cache de chaque poste :
cx-nettoyage-apercu-transfert-v2-{local,remote}-3b2eab7ec contient copies,
garde.json, fin.json et dump sur remote. Preuves, ELF, logs et branches privées
distantes conservés. Aucun Cargo/créneau arbre actif ; reprendre la suite dans
un worktree neuf. Les autres lots et l'arbre de Lucie n'ont pas été nettoyés.

Correction du script privé de push effectuée, sans rejeu du push ni de CI :
/tmp/cx-push-apercu-transfert.sh collecte désormais chaque delta dans un fichier
(échec Git fatal), puis /tmp/cx-verifier-delta-push-apercu.py lit toute la liste.
Témoin synthétique exécuté pour les deux barrières :70000 chemins docsCPP admis,
70000docs +1Rust en fin de liste refusés, tous les chemins effectivement lus.
Contrôle réel20e4→3b2 :168 chemins lus, aucun refus, aucun delta produit.
Preuve locale cx-preuves-finales-apercu-transfert-v1/preuve-garde-delta-complet.json.
Aucun nouveau protocole partagé ou correctif source ajouté pour cet incident privé.

## Reprise de préparation après l'aperçu intégré — qualification des faits

Demande orchestration : déterminer le plus petit prochain pas sans apply,
stockage ou bases de Lucie, et séparer implémentation autonome/arbitrage produit.
Source de lecture41497ba4e, nouveau WT local cx-wt-qualification-faits, branche
cx-arbre-qualification-faits. [Page existante mise à jour, section Prochain pas](../12-11-2026/02-preparer-le-transfert-des-valeurs-connues.md).

Tranche A proposée : producteur commun de faits déclarés complets et vérification
des empreintes du plan figé, consommés par sa construction dans le runner existant.
Compositions/dépendances SchemaFacts réutilisées, entités/relations/recettes
résolues/modèles distingués ; source partielle ou producteur inconnu refusés.
Clés relation:<nom> déjà persistées par Catalog identifiées statiquement ; ne pas
réinventer de magasin ni considérer l'inspection entités seule comme graphe observé.
Aucun fichier Rust modifié/réservé ; coordination future backend_schema avec mémoire.

Liaison de faits possible en restant Unversioned : aucun identifiant/génération
fabriqué, aucune qualification de stockage ou évolution de l'aperçu actuel supposée.
Pas de nouvel arbitrage Lucie pour A. Une vraie question numérotée dans la même
page porte la sémantique future de génération durable (révision de publication
ou état adressé par contenu), à lire avant son producteur persistant ; elle ne
bloque pas A. Aucune nouvelle vision, aucun Cargo/GPU/target/service ou base.
Ticket reste ouvert, prochaine revue du plan puis diff borné, pas attente technique
artificielle sur des garanties déjà fixées.

## Décision de Lucie — génération composite, 12 novembre

Lucie tranche après la proposition composite A→B→A : génération comme identité
durable de publication, empreinte d'état logique distincte pour partager et
réutiliser le travail après requalification. P1/P2/P3 restent distincts ; H-A
peut être retrouvé après H-B. Le rejeu de la même publication already applied
conserve son identité et ne répète pas ses effets.

Le travail de préparation peut être réutilisé si les dépendances pertinentes
complètes sont requalifiées identiques, mais un nouveau plan est figé pour la
publication courante. Ancien plan, identité et journal ne sont pas réécrits ;
empreinte partielle ou inconnue, annulation et contrôle d'admission restent des
refus ou vérifications distincts. Partage des contenus/vecteurs et identités
d'occurrences restent séparés ; aucune invalidation globale des autres branches
n'est imposée par le contrat.

La [page de conception existante](../12-11-2026/02-preparer-le-transfert-des-valeurs-connues.md)
porte la décision et ses limites. Cette question n'est plus en attente. Le ticket
reste ouvert pour l'implémentation et l'adoption effectives ; le choix ne transforme
pas A, son aperçu ou ses empreintes déclarées en stockage durable déjà livré.

## Seconde lecture des contrats par Gemini — 12 novembre

À la précision de Lucie sur l'usage normal de Gemini, mémoire prend une revue
complémentaire bornée du producteur/comparateur de faits A et de sa frontière
avec la décision composite. Fournir uniquement les sources identifiées,
contrats et synthèses des preuves ; aucune base ou conversation réelle.
Question : omissions concrètes dans la complétude/canonicalisation des faits,
leur liaison au plan, leur provenance et l'immuabilité des générations/plans.
Demander des références de code et distinguer hypothèses et défauts établis.

Cette seconde lecture avance en parallèle du lot déjà qualifié par ciblé,
relectures et passe composée ; elle ne crée pas une attente administrative
avant son intégration. Conserver requête, source, résultat/erreur et borne dans
les preuves du même ticket. Tout défaut concret relance un travail explicite,
sans transformer l'avis du modèle en résultat de test ou en pouvoir d'appliquer
une migration persistante. Le ticket global migration reste ouvert.

## Tranche A — Rust commencé après GO Lucie relayé par C7

Source87799f681, nouveau WT cx-wt-faits-lies/branche cx-arbre-faits-lies.
Implémentation : facts::{DeclaredSchemaInput,ProviderFacts,RecipeFactsInput,
DeclaredSchemaFacts,DeclaredMigrationFacts}. None signifie incomplet, Some(empty)
un ensemble déclaré complet et vide ; faits fournis, jamais observation stockage.
Entrées effectives EntityConfig (champs JSON Schema déjà fusionnés), relations
BackendRelation/RelationDef, fournisseur complet/absent/inconnu, surcharges de
signaux explicites, recettes nommées aux octets fournis, ModelSource et registre
EmbeddingModelEntry existants. Aucun I/O ou appel modèle dans le producteur.

Adaptateur unique backend_schema.rs : compose_effective réutilise SchemaFacts::compose,
applique les surcharges puis valide toutes les configs et registration_order sur
l'ensemble FINAL. Mémoire en revue seule ; backend.rs inchangé. Le runner expose
resolve_plan_from_facts, produit ses inputs puis compare avec le même producteur
avant de rendre plan+DeclaredFactsMatch scellé, provenance SuppliedDeclarations.
Générations Unversioned admises pour cette liaison, pas pour l'exécution ; aucun
identifiant de stockage créé, bootstrap/apply/journal/CLI inchangés.

Contrats d'empreintes en mémoire séparés et versionnés : declared-entity-configs:v1
pour les configs effectives seules ; declared-migration-facts:v1 pour l'ensemble
source/cible (déclarations, fournisseurs, relations/politiques, recettes/modèles).
Les anciennes étiquettes/producteurs inconnus refusent la comparaison ; aucune
réécriture de metadata ni renommage de StorageInspection.entity_config_fingerprint.
Le mécanisme de JSON canonique est partagé avec l'aperçu, pas sa sémantique.
L'aperçu garde son statut UnqualifiedConfigAndSchemaFacts et aucune capacité write.

18 témoins écrits, pas encore exécutés : runner réel/parsing MMD synthétique,
variables/immutabilité, permutations de maps vs ordre des listes, champs fusionnés,
dépendances finales, fournisseur code réel (feature code), configs/relations/
recettes/modèles altérés, incomplets, empreintes opaques, génération absente et
cible attendue/étrangère, non-finis. Revue provisoire mémoire a signalé fusion et
field_weights : validation des f64 complétée avant JSON dans manifest/provider/
effectif, puis partagée avec l'aperçu ; cas NaN/Inf ajoutés aux témoins concernés.
Aucun faux rouge fondé sur l'absence d'API n'est revendiqué.

Recette ciblée proposée à publier avec SHA avant lourd : cargo_magasin.sh test
--no-default-features --features rag3db-native,code --lib -j8
dataflow::migrations:: -- --test-threads=8. Ce filtre couvre les18nouveaux et15
aperçu, plus les contrats/runner existants ; compte final à lire réellement.
Budget600 après poste lourd, j8/Max32G/target32Go/lib29. Aucun lourd réservé/lancé,
aucun GPU/modèle/base réelle/service ; publier diff+témoins à mémoire/C7 avant essai.

### Préflight syntaxique avant le premier ciblé A

Mémoire a repéré sur206e276 une accolade de fermeture omise dans validate_finite,
confirmée par lecture de la source figée. Aucun Cargo/lourd n'avait démarré sur206.
Correction minimale publiée séparément ; parse rustfmt édition2021 --emit stdout,
skip_children, sur tous les Rust modifiés et les deux nouveaux fichiers réussi,
sans réécriture de fichiers. Ce contrôle de syntaxe n'est pas un test métier ni
une compilation. Nouveau SHA obligatoire avant ciblé, même recette/budget autorisés.

Le montage compilera une fois --no-run/JSON natif+code, identifiera l'ELF nommé
par Cargo, puis listera son inventaire avant runtime. Exiger18facts et15aperçu,
compter le groupe migrations complet et exiger tous les tests sélectionnés passés
sans ignore ; aucun compte global supposé depuis l'absence d'API. Premier rouge
stop/diagnostic. Les revues continuent en lecture seule sans toucher source figée.

### Premier ciblé A : rouge du témoin fournisseur, diagnostic arrêté

Source **d953f27813567810d797b2f3225d19f03ee457c3**, compilation --no-run réussie,
ELF identifié par JSON Cargo avec natif+code ; inventaire AVANT runtime :64 tests,
dont18facts et15aperçu. Exécution **63 réussis,1 échec,0 ignoré**, rc101,
126,94s compilation comprise. Unique échec actual_code_provider : le témoin
attendait à tort DEFINES.to=Scope ; la définition réelle est
code.rs RELATIONS (DEFINES,SCOPE,SYMBOL), rendue par schema_configs, donc Symbol.
Les17autres nouveaux,15aperçu et31existants ont réussi. Ce rouge n'est pas attribué
à un défaut du producteur. Source propre, scope inactive et place arbre rendue.

Preuves cache luciepc cx-faits-lies-cible-v1/{compile.log,inventaire.json,tests.log,
source.json,resultat.json,binaire.json,binaire-tests}. SHA ELF réel
6e5b075c45cd4efe37b464bab209bc630c18af3703847f1a8fc5eb89a8684820.
Aucune relance inchangée. Correction limitée au témoin : comparer l'ensemble
exact des RelationDef du vrai fournisseur avant/après composition, extrémités
ET propriétés, plutôt qu'une extrémité présumée erronée. Nouveau SHA publié et
parse de syntaxe effectué avant proposition de ciblé v2 au même budget.

### Ciblé A v2 vert dans la fenêtre globale INITIALE

Source propre **59eb3d9df59e14c1f8aa5c02cf00b250bd7f82e9** ; seul Rust changé
depuis d953 : témoin comparant la map complète des RelationDef réelles. Revue mémoire
du delta conforme, producteur/adaptateur/runner inchangés. **64/0/0**, dont18faits,
15aperçu et31contrats/runner existants ; inventaire exact64 identique à v1 vérifié
avant runtime, natif+code attestés dans l'artefact JSON Cargo.

C7 a exigé de conserver la fenêtre600 INITIALE : début conservateur
20:00:17.214+01, échéance20:10:17.214+01. V2 acquise avec291,07srestants,
RuntimeMax281s, timeout au reste ; driver limite end-15s. V2 **23,13s**, fin à
**332,06s depuis le début initial**, sans remise à zéro. HIGH/MAX/OOM/OOMkill0,
source propre, scope inactive et poste libre0/4 relus ; target unique1,6Go.

Preuves luciepc cx-faits-lies-cible-v2/{source.json,compile.log,inventaire.log,
inventaire.json,tests.log,resultat.json,VERDICT,binaire.json,binaire-tests}.
ELF rag3weaver-88737abc986584d2 réellement produit/nommé Cargo : SHA256
**0bde14c21adae5ae67efe4a99a45e2535c8a3e92fd42ba46cc7f2cd8ede9c84c**.
206 sans Cargo, d953 compilation/inventaire verts puis63/1/0 erreur de témoin,
et59eb64/0/0 restent distincts. Aucun rouge de produit inventé, aucun retry inchangé.

Producteur/comparateur purs des faits fournis ; runner consommateur lit ses
fichiers MMD existants, pas de promesse IOzéro pour le runner entier. Faits scellés
SuppliedDeclarations, Unversioned non fabriqué, aperçu toujours non qualifié au
stockage. Aucun modèle/service/base réelle/apply/bootstrap. Ticket migration ouvert.

Prochaine étape préparée hors push : revue du bilan et passe adaptée sur master
incluant d'abord le lot ED prioritaire (rebase avant source de passe). Proposition
features5+minimum checklib, lib entière8features, navigateur selon inventaire réel
du nouveau master, CLI migration5/1suite ;18faits+15aperçu réellement exigés enlib,
contrats source/génération conservés. Global3600/j8/Max32/target32/lib29 sous poste,
PG privée neuve si requise par lib, TMP navigateur court privé disque ; aucun
service/modèle/GPU nouveau nécessaire. Aucun lourd ou master de cette étape lancé.

### A — passe globale figée après intégration ED, autorisée C7

Rebase sur **6dd116e8f70fd328acf3b45a89401e06f71365cb**, branche neuve
cx-arbre-faits-lies-2, range-diff4/4 identique, code A inchangé depuis le ciblé
vert59eb. Master860f51706 ajoute seulement le bilan/nettoyage ED documentaire.
Recette reviewable versionnée dans
`docs/arbre-principal/12-11-2026/preuves-faits-lies-v1/{passe.py,lancer.sh}` ;
SHA source passé en argument après commit/push. Ces scripts sont la preuve de
montage du lot, sans nouvelle primitive produit. Syntaxes Python/bash contrôlées.

Lancement luciepc via scripts/poste lourd, POSTE_MEM_MAX=32G, scope propre
cx-passe-faits-lies-v1.scope, un seul target cx-target-contenu-derive32Go,
lib29a7 épinglée, j8/INCREMENTAL0/LUCIVY8. **Échéance unique3600s** prise juste
après acquisition, jamais réinitialisée ; driver fin-15s, runtime scope3590s.
Bornes intersectées avec cette échéance : features5≤1500, véritable
`cargo check --lib --no-default-features -j8`≤120 (aucun test minimal revendiqué),
compilation large+listes≤1200, runtime lib≤300, NAV compilation/listes/runtime≤360,
CLI compilation/listes/runtime≤180, attestation≤120. Premier rouge stop/diagnostic,
pas de rejeu inchangé ni de tour master acquis par ce GO.

Huit features explicites : rag3db-native,burn-embedder,burn-ocr,code,daemon,
postgres,openai-llm,anthropic-llm. Compilation test --no-run --message-format=json,
chemins ELF issus des artefacts Cargo, puis --list ET --ignored --list avant
runtime direct via export lib du magasin. Lib entière : compte dérivé de l'ELF,
18faits+15aperçu et témoin lien80 requis réussis, ignorés historiques inventoriés.
NAV : mêmes marques source que temoins_navigateur.sh, feature navigateur,
**40** attendus sans ignore. CLI : cible existante `--test e2e`, puis filtre
`e2e_migration_preflight::` **5 tests/1 groupe**, sans ignore/saut ; autotests=false,
aucune cible autonome inventée ou ajoutée. Backend CLI identifié par JSON Cargo.

CPU explicite commun+EMBEDDER+RERANKER+OCR, BUILD_MEMORY_HIGH0 ; aucun service
embed/sparse/rerank/OCR hérité. Énumération matérielle/RADV peut ouvrir dri/kfd,
FD observés sans en déduire une inférence ; aucun modèle/service/GPU nouveau.
PG synthétique neuve **cx_faits_2191_20261112_v1** sur conteneur existant5433,
absence obligatoire avant création, rôle et current_database vérifiés avant
lib ; connexion explicite jamais base implicite5433. Pas de .env/vault ni DB réelle.

NAV sous TMPDIR court mkdtemp propre **/var/tmp/cx-fa.*** sur btrfs, wrapper natif
Chromium avec bus D-Bus privé **sans aucun répertoire d'activation**, aucun ajout
aux flags navigateur/système. Observateur PID/starttime sur descendants réels,
cgroups et memory.max32 de chacun, cartes/maps réelles de librag3db29, ouvertures
devices ; arrêt de nos seuls PID par pidfd si rouge. ELF exacts archivés avec SHA,
source embarquée et **220 fichiers templates** vérifiés en octets pour lib/NAV/CLI/
backend ; maps de chacun exigées. Quiescence propre puis scope inactive avant
retrait du seul TMP possédé si vert ; rouge conserve TMP/proofs/PG. Preuves hors
target sous cache luciepc **cx-passe-faits-lies-v1/**, PG conservée pour revue.

Coordination FTS : l'ancien message5a83eee91/5dddf215c reçu pendant ce montage
est **historique** selon C7 ; déjà intégréb28b55db6, ticket fermé et ressources
retirées. Recherche en veille, aucune nouvelle revue/créneau/reprise FTS à attendre.
Le lot A reste prioritaire ; son producteur de déclarations ne qualifie toujours
pas un stockage et ne livre aucun apply/bootstrap/publication. Ticket reste ouvert.

### A — arrêt préventif v1 et correction de montage v2

v1 source79f7c6d13 lancée20:27:04 Paris, arrêtée pendant F5 à19,93s par revue
mémoire : RAG3WEAVER_JOURNAUX_DIR et RAG3WEAVER_BUILTIN_TEMPLATES_DIR privés
manquants ; scope non liée à un pilote. Scope inactive, verdictROUGE conservé,
poste0/4 relu. Aucun runtime lib/NAV/CLI ; ce n'est pas un défaut produit ni un
vert F5. PG synthétiquev1 conservée, pas de reset ni suppression en cachette.

Recette v2 distincte versionnée dans preuves-faits-lies-v2 : les deux caches sont
explicitement privés (journaux sous commandes, pas dans TMPDIR). Pilote service
cx-passe-faits-lies-v2.service Max32/KillModecontrol-group ; acquiert poste réel
puis scopev2 BindsTo/After **cette unité exacte**, Max32/HIGHmax, propriétés lues
avant Cargo. À l'arrêt du pilote, sa scope liée doit donc s'arrêter aussi.
**Début monotonic et échéance lus de v1/source.json**, jamais recréés ; durée des
unités et tous les budgets réduits au reste, F5 retirant aussi19,93s déjà dépensés.
Nouvelle PG synthétiquev2 avec absence/current_database et source vérifiés,
v1 conservée. Toute autre phase inchangée : lib18+15, NAV40/bus privé, CLI5,
attestations/maps/220/quiescence. Premier rouge arrêt, aucun rejeu inchangé.

Complément de revue v2 : stop_owned poursuit les pidfd même si observe redéclenche
un invariant, erreur d'observation consignée séparément ; corrigé déjà dans v1
publiée puis explicité dans v2. Scan final indépendant exeTarget/cwdWT/profilTMP
contre les descendants manqués ; jalon nav-quiescence.json avant CLI avec socket
bus absent, pour rendre le navigateur à recherche dès ce jalon réellement vert.
