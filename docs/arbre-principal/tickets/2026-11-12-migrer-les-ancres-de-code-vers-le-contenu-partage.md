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
