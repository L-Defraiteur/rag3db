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

### A — lib v2 rouge de montage runtime, reprise v3 sans rebâtir les verts

Source **d2cb7f0636d099823f63de2cef674b630a46bc6f** reste figée sur luciepc.
F5 **5/5 rc0,133,03s**, vrai checklib sans features **rc0,37,05s** ; compilation
large **rc0,239,01s**, ELF exact rag3weaver-9bb7a242a57fc19a identifié par JSON.
Inventaire2287 tests,65 ignorés avant runtime ;18faits+15aperçu non ignorés.
Runtime **2218 réussis/4 échecs/65 ignorés,46,85s**, scope arrêtée/inactive et
poste0/4. Aucun NAV ni CLI joué. Les4erreurs sont exclusivement NotPresent :
code_tools::cwd_tests::cd_ne_ment_pas_sur_ou_on_est (code_tools.rs2062),
origin::{the_same_file_reached_from_two_depths_has_one_origin_and_one_name,
git_coordinates_name_a_file_by_its_repo_not_by_the_disk} (origin.rs438/491),
work_domain::the_default_domain_is_the_repo_of_the_file_being_worked_on (323).
Ces lignes lisent **std::env::var("CARGO_MANIFEST_DIR").unwrap()** ; le driver
ELF direct avait omis l'environnement runtime que cargo test aurait fixé.
Aucun défaut produit attribué. Les18faits+15aperçu sont réussis dans ce passage,
mais cela ne rend pas la lib entière verte. ELF rouge archivé avec SHA avant
nouvelle compilation ; logs/maps/PGv2 conservés.

Recette v3 publiée séparément, **exécutée hors WT** pour préserver source et ELF
d2 : RAG3WEAVER caches/journaux/CPU/bus identiques, **CARGO_MANIFEST_DIR=crate**
explicite. Pilote/scopev3 propres liés, PG synthétique neuvev3, pas de resetv2.
Réutilise F5/min/compile lib d2 déjà verts ; hash ELF égal archive rouge et
inventaires exacts identiques avant runtime lib entière. Ne rejoue aucun Cargo
F5/min/lib. Budget libruntime≤**300-46,85s**, borne compile/listes restante,
**INITIAL/DEADLINE3600 toujours ceux de20:27:04**. NAV/CLI/attest restent à jouer.
Les deux finitions de revue wrapper sont incluses dans v3 : pidfd ouvert AVANT
relecture starttime puis signal sur ce FD ; cleanupnonzero capturé et VERDICT
réécrit ROUGE. v2 n'a pas été éditée pendant ses jobs ni réécrite après coup.
Verdict final composé exigera aussi rc réel pilote, scope inactive, quiescence
et TMPabsent ; jamais le seul fichier VERDICT. Ticket reste ouvert.

V3 : arrêt **avant PG/Cargo/runtime**, garde quiescence-avant rouge en761ms.
Les deux processus relevés sont le bash client SSH4091595 et son systemd-run
4091616 dont le cwd était resté dans WT après vérification de HEAD : le garde
les a correctement signalés comme occupants, pas comme anciens tests. Scopev3
inactive/poste0/4. Aucun test ni nouvelle PGv3 créé, aucun occupant tiers tué.

V4 publie aussi **demarrer.sh** : après les vérifications source, client SSH
**cd /var/tmp** et pilote **WorkingDirectory=/var/tmp** avant lancement. Le scan
reste strict, sans exemption par nom ou relaxation de son invariant. Recette/
source/ELF/budgetsidentiques à v3, preuve et unitésv4 neuves, PGv4 synthétique
seulement si préflight vert ; INITIAL/DEADLINE v1 toujours conservés. Pas de
rejeu Cargo, aucun résultat v3 inventé. Sourcecompilée d2 inchangée sur luciepc.

### A — lib entière réellement verte v4, lecteur de verdict corrigé v5

V4 source/ELF d2 identiques : **2222 réussis/0 échec/65 ignorés**, runtime46,58s,
phase46,70s. Le vérificateur de noms a ensuite déclaré ROUGE : il comparait le
nom de `sparse_index::tests::sparse_vector_mismatched_lengths - should panic`
au nom inventorié sans ce suffixe. Unique omission du lecteur démontrée par
comparaison complète des2287noms : ancien parser manque1, nouveau parser
`(?: - should panic)?` reconnaît **exactement2222noms réussis**,65ignorés
inventoriés. Preuve CPU sur logarchivé/SHA dans preuves-faits-lies-v5/preuve-parser.json.
Les18faits,15aperçu et lien80 figurent réellement dans les succès. Aucun produit
modifié, aucune lib rejouée pour corriger le lecteur.

V4scopeinactive/poste0/4 ; aucun NAV/CLI. V5 conserve ce ROUGE de vérificateur,
réutilise libv4verte/mapsréelles, F5/min/compilev2 ; source d2 et SHAELF immuables.
**Seuls NAV40, CLI5, attestation/quiescence restent à exécuter**, dans deadline
initiale20:27:04+3600. PGv4 déjà créée identifiée explicitement, sans reset ni
nouvelle PG inutile. Nouveau répertoirepreuves/pilote/scopev5 ; sourceRemote
inchangée. Le rapport final sera composé et ne décrira pas une passe sans incident.

### A — NAV40 verte, bus privés survivants arrêtés et quiescence indépendante

Source/ELF d2 ; NAVcompile43,26s, inventaire40sansignore puis **40/0/0** en137,88s
(phase137,99). Le garde de sortie a relevé **40 dbus-daemon privés** encore
vivants après les wrappers dbus-run-session ; aucun Chrome restant. Le Drop du
navigateur peut tuer le wrapper avant la fin de son bus : pas de cause produit
A attribuée et pas de relance NAV. Le garde a interrompu la passe avant CLI et
arrêté ses seuls descendants attribués par pidfd ouvert/starttime, puis scope.

Contrôle indépendant `cx-passe-faits-lies-v5/nav-quiescence-apres-arret.json`
à20:48:00 Paris : scopeinactive, known_live=[], scan exeTarget/cwdWT/profil=[],
socketnodes=[] ;528recordsdescendants conservés, tous memory.max32 dansscope5,
Chromium natif observé. TMP privé /var/tmp/cx-fa.142trqgk conservé au rouge.
FIN/quiescenceNAV signalées à recherche/C7 pour son unique préflight coordonné,
pas une autorisation master ni une dissimulation du défaut d'hygiène.
Ce fait est transmis au porteur du ticket commun des scopes pour l'adoption
transversale d'un arrêt explicite des bus attribués, hors produit A.

V6 ne rejoue **ni NAV ni lib ni F5/min/compilelib** : réutilisation des logs,
inventaires exacts, ELFhachés, maps et quiescence après arrêtbus ; seules CLI5 et
attestation220 restent. Sourcecompilée d2 inchangée ; recette externe publiée,
unités6 liées, initial20:27:04+3600 toujours inchangé. TMP5 reste conservé jusqu'à
succès composé complet+quiescence, puis retrait du seul root marquépossédé.
PGv4 identifiée, pas de nouvelle base/reset. Verdict composé distinguera chaque
vert réel des incidents de vérification/montage et de nettoyage conservés.

### A — bilan composé final et attestation indépendante verts

Source jouée **d2cb7f0636d099823f63de2cef674b630a46bc6f**, Rust inchangé depuis
le ciblé vert59eb après rebaseED. **F5 5/5 ; vrai minimum checklib rc0 ; lib2222/0/65
(18faits+15aperçu+lien80) ; NAV40/0/0 ; CLI5/0/0,1groupe**. CLI compile34,94s,
runtime0,21s ; artefactJSONe2e-b95258cc43cff951, cinq noms listés avant runtime,
backendCLI réellement observé/mapsnatifs29. Attestation220fichiers/sourceD2 et
quatre ELF archivés+SHA, rehash indépendant vert :

- lib a72ee7f051de6f01c8ad8f3a950ec5a0976a67cd4a77873da92f06b96dd85594 ;
- NAV 44aa11b6a280c9b4a7fc4b73d9f62f473b1b6c0146de371b91a81482efb21fb5 ;
- CLI acc74857d673402e0d405b43cf5b640345a6d3e972f1b047edd29337af0e7440 ;
- backend f23b28644616556aeee1540e13e0cb918b4e996ce8bb6776550cd64dcf880ea3.

**1385,89s depuis INITIAL20:27:04**, fin20:50:10, deadline3600respectée sans reset.
Client systemd rc0/SUCCESS, pilotes/scopes inactifs, HIGH/MAX/OOM/OOMkill0,
scan final indépendantexeTarget/cwdWT/profil=[],TMP5possédé retiré et absence
vérifiée, poste0/4 relu. Preuves PC cache cx-passe-faits-lies-v6 : VERDICT_COMPOSE,
verdict-compose.json, artefacts.json,templates-220.json,maps.json etmapsréutilisées
v4/v5,quiescence.json,tmp-retire.json,controle-final-independant.py. Résumécompact
versionné bilan-compose.json ; métadonnéescopiéeslocalement hors WT. Target8,1Go
et PG synthétiquesv1/v2/v4 conservés pour revue, aucune base réelle ou migration.

Le VERT est **composé**, pas une passe sans incident : F5/min/compilev2,
libv4, NAVv5+arrêtbus/quiescence, CLI/attestv6 ; tous les rouges et leurs causes
restent dans les preuves. Sources/fichiers joués non modifiés pendant jobs,
aucun redo F5/min/compilelib/lib/NAV pour réparer un vérificateur. La primitive
commune Recherche5fab0e1f9 prévoit désormais un seul bus pour toute la portée,
avec résiduaprèsrc0→ROUGE même nettoyé ; elle ne réécrit pas l'histoire d'A.

README/nouveautés/rapports mis à jour ; API publique correspondance pure de
faits **fournis**, comparaison des mêmes contrats versionnés, aucune provenance
physique ou capacité d'écriture. Aperçu toujours non qualifié. Décision composite
Lucie sur publicationdistincte/empreintecontenu publiée777820ef doit être conservée
au rebase documentaire final, sans producteur persistant anticipé. Ticket reste
ouvert ; attente revue C7/mémoire et tour explicite avant master, puis3contrôles,
archives+gardes fraîches et nettoyage propres.

Rebase final documentaire sur777820ef79d26af91b93687953bb8c06f790fbb8,
branche neuve **cx-arbre-faits-lies-3** publiée f9b7cd9d6 :13commits,11identiques,
2seuls contextes documentaires de patch adaptés (décision composite ajoutée par
C7 dans ticket/page02). Conflit d'append ticket résolu en conservant intégralement
la décision Lucie ET tout l'historique A ; page02commence par les octetsmaster
sans perte, puis bilanA. Arbre extension/rag3weaver strictementidentique à la tête
avantrebase e6a5006a6 ; delta avec sourcejouéeD2 =README seulement sous extension,
tous autres chemins documentaires complets contrôlés, pas de replayCargo.
Range-diff et rebase-final.json conservés dans preuves finales locales. Revue
mémoire des4ELF/220hashes/native/CLI/quiescence favorable, tour C7 encore attendu.

### A — intégration master et nettoyage final terminés

**Master6d4d1c882b9fc3f59720e6708bd4b3042be7b6a1**, « à toi » C7 explicite,
poste push et dernierfetch417ca1f26 ; troiscontrôles LUS statuspropre/aucunrebase/
originmaster=HEAD. Rebase branche neuve4,14/15commitsidentiques,1contextedocGemini
seul adapté ; décisions777composite et417revueparallèle conservées intégralement.
Rust/assets identiques à sourceD2jouée, deltaREADME seulement sous extension.
Quatre ELF copiés et **rehachés aux DEUX postes**, SHAidentiques aux attestations.
Preuvesbrutes/logs/bilans2postes conservés, pas d'attente Gemini/recherche pour A.

Nettoyage immédiat terminé : WTfaits-lies LOCAL+luciepc retirés, target8,1GoPC
retiré, ancienWTdocqualification-faits LOCAL retiré. Avant chaque retrait :
cp-a des administrationsGit fidèles(sous-module inclus), helperSHAdca13abb7a84cf3c1419f728697837a6b1b9b20e34e34c144c9fbf835252f628,
rapportgardeJSONfrais **VERT lu**. Brancheslocales propres liées au lot vérifiées
égales aux refsremotes puis branch-d ; remotes inchangés, aucun GC/prune.
PGv1/v2/v4 seules : propriété/0sessions, pg_dump+SHA puis DROP/absencevérifiée :

- v1 fdcae0cb32bf3a7ed85f57a98d53a40478c65a3c7f5d473f2340f6cbfdd5eea6 ;
- v2 db112a27c41405d7813ec970c32167075944bbdd989bf1f7322543032aec5458 ;
- v4 f3f299b9c4089149563bd72cecabeae9e86cd51a4758d6242016a9e80c1f93db.

Rapports+archives : cache cx-nettoyage-faits-lies-v1-{local,remote}-6d4d1c882/
(fin.json,garde.json,administration/) et cx-nettoyage-ancien-wt-faits-6d4d1c882/.
Dumps et rapportsremotes recopiés localement dans cx-preuves-finales-faits-lies-v6/
nettoyage-remote ; ELF, logs et rouges historiques conservés. Nouveau WTbilan
MDuniquement pour ce constat, pas de recréation du WTcode ni de target.
Aucune base réelle, modèle/service/GPU nouveau ou migration/apply ; ticket reste
ouvert pour ces suites distinctes. Poste a signalé CI déjàrouge depuis18:33UTC,
pas de rerunCI ni de succès CI attribué au lot. Les attentes de tour/ressources
des paragraphes antérieurs sont historiques, pas des instructions de reprise.

### Seconde lecture Gemini exécutée — résultat incomplet, aucun défaut établi

Un seul appel Vertex global `google/gemini-3.8-flash`, voie OpenAI configurée,
HTTP200 en **41,24s**, borne globale120s incluant ADC, zéro retry. Identifiants
transmis au client ADC uniquement par chemin usuel ; aucun contenu de secret,
base ou conversation réelle envoyé ou conservé. Sources Rust figées D2, six
fichiers complets avec lignes, contrat composite777 et synthèse des preuves.
A était déjà intégré6d4d1c882 ; cette lecture parallèle ne modifie pas sa qualification.

Résultat **incomplet** : `finish_reason=length`, 278tokens visibles et6718tokens
de raisonnement déclarés, 49854tokens d'entrée ; ce n'est pas une revue favorable
complète. Le seul début de constat prétend que les sous-objets dynamiques
seraient omis par `canonical_json`. Relecture directe : facts.rs225–229 traite
chaque `Object`, trie les clés et applique `canonical(v)` à chaque valeur ;
la branche `Array` recurse également. L'affirmation reçue contredit donc
l'implémentation fournie et ne démontre aucun défaut. Texte tronqué conservé
intégralement ; aucune extrapolation sur des constats non rendus, aucun nouvel
appel ni Cargo/GPU/banc. Le ticket reste ouvert pour les suites de migration
déjà identifiées, sans action corrective issue de cette réponse.

Preuves durables : [requête et sources](../12-11-2026/preuves-revue-gemini-A/requete-et-sources.md),
[réponse](../12-11-2026/preuves-revue-gemini-A/reponse.md),
[réponse brute](../12-11-2026/preuves-revue-gemini-A/reponse-brute.md),
[métadonnées](../12-11-2026/preuves-revue-gemini-A/metadonnees.md),
[empreintes sources](../12-11-2026/preuves-revue-gemini-A/sources.md),
[script exact](../12-11-2026/preuves-revue-gemini-A/appel.md).
La requête contient tous les octets sources envoyés et leurs références Git ;
SHA256 requête `cda32c4a9318f4b3787e28519b110761d90c834e46ed214dcdd1ce23a4100864`.
Copie brute locale : cache mémoire `revue-gemini-A-20261112/`.

### Seconde tentative ciblée après diagnostic — réponse complète

GO C7 après diagnostic de la première réponse tronquée : seconde revue différente,
paquet réduit à facts, adaptateur compose, runner18lignes, tests pertinents et
contrat composite777 ; mêmes source D2/voie Vertex/modèle, `max_tokens=16000`.
**11113tokens d'entrée** (sous12k), HTTP200 en **37,69s** total auth+POST,
plafond120s, un seul POST sans retry. `finish_reason=stop`, réponse complète :
**« aucun constat établi »**. Usage déclaré : 4tokens visibles, 6307reasoning.

Il s'agit d'une revue statique complémentaire sans défaut signalé, pas d'une
preuve d'absence universelle de défauts ni d'une qualification du stockage.
Aucune nouvelle action corrective, Cargo/GPU/banc ou barrière à A déjà intégré.
Le premier résultat tronqué reste intact ci-dessus avec ses preuves.
Les identifiants restent confiés au SDK ADC par chemin normal, aucun secret
lu par les outils, affiché, envoyé dans le prompt ou enregistré dans les preuves.

Preuves seconde tentative : [requête exacte et sources](../12-11-2026/preuves-revue-gemini-A/seconde/requete-et-sources.md),
[réponse](../12-11-2026/preuves-revue-gemini-A/seconde/reponse.md),
[brut](../12-11-2026/preuves-revue-gemini-A/seconde/reponse-brute.md),
[métadonnées](../12-11-2026/preuves-revue-gemini-A/seconde/metadonnees.md),
[sélection et empreintes](../12-11-2026/preuves-revue-gemini-A/seconde/sources.md),
[script](../12-11-2026/preuves-revue-gemini-A/seconde/appel.md).
SHA256 requête `f183bdcab67e5b5e5332b1dfbd6a3cf48f7c260638e213c87350f7f8a615c6c0`.
L'en-tête d'un extrait tests annonce1–132 ; seuls1–104 ont été envoyés après
réduction, sélection effective explicitée dans le manifeste sans modifier
a posteriori le prompt conservé. Copie locale : cache mémoire
`revue-gemini-A-20261112/seconde/`. Mission de seconde lecture terminée ;
le ticket migration demeure ouvert pour ses suites propres.

### Suite RO — prise en charge après A, source de départ82e812764

GO C7 pour le raccord déjà prévu page02, même ticket, WT neuf local/remote prévu
cx-wt-observation-faits, branche cx-arbre-observation-faits. Arbre prend
storage_inspection et extraction des décodeurs Catalog ; mémoire revue seule,
pas d'édition concurrente backend.rs. Aucun Cargo/target/créneau encore lancé.

Constat relu : inspect_snapshot_on ouvre une transactionRO puis inspect_on lit
catalogmeta et journal dans cette vue. L'inspection décode les entity_config mais
**ignore relation:<nom>**, alors que Catalog::persist_relation/load_relations
utilisent déjà RelationDef. Une relation malformée reste donc omise, inspection
et admission legacy pouvant réussir. Un témoin de refus sans effet est ajouté
sur ce comportement existant (pas un faux rouge « API absente ») : metadata
relation:Broken=not-json, transactionRO doit finir parROLLBACK sansCOMMIT.

Diff prévu : décodeurs partagés stricts des lignes/namespaceconfigs/relations,
réutilisés par les load_entity_configs/load_relations existants et l'inspection,
sans autre initialisation/écriture. Restitution typée scellée des namespaces
persistés et journal dans la même observation ; table absente=information
indisponible, tableprésentevide=ensemblepersisté observé vide. Ce périmètre ne
prouve pas un schéma effectif complet, ni fournisseur/politiques/recette/modèles.
Provenance vueconnexion distinguée de snapshotRO réussi (promotion seulement
aprèsCOMMIT). Exposer le journal déjà strict par son type existant, sans second
journal ou qualificationApplied inventée. Empreinteentités existante inchangée.

Raccord à A réutilisant DeclaredSchemaFacts::from_complete_input : vérifier
explicitement que les ensembles effectifs fournis concordent avec configs ET
RelationDef observés ; fournisseurs/politiques/signaux/recettes/modèles restent
**fournis**, jamais requalifiés observés. Pas de hash/compose dupliqué, pas de
changement de format metadata, identité durable/génération inconnue, preview
inchangé, aucune migration/apply/bootstrap/publication ou base de Lucie.

Recette à figer/publier avant lourd : confiner_validation.py commun (poste réel,
Max32, deadline600,CPU), targetunique cx-target-contenu-derive32Go/lib29/j8,
compilationnatif+code --no-run JSON, inventaireELF avantruntime du seul témoin
rouge ; erreurattendue nommée et arrêt/diagnostic, sans continuer aux phases
vertes ou rejouer inchangé. Après correctif/fixtures supplémentaires : nouveauSHA
et proposition de ciblé du groupe inspection+décodeurs+A. Pas de modèle/service/
GPU/PG nécessaire au ciblé ; seule DB native synthétique possible pour preuve
lecture seule/octets inchangés. Global ultérieure choisie après diff/revue.

Recette rouge versionnée preuves-observation-ro/rouge.py : SOURCE SHA après
commit en argument ; lancement sur luciepc depuiscrate par
`POSTE_MEM_MAX=32G python3 scripts/confiner_validation.py lancer --delai 600 --preuves <cache/cx-observation-ro-confinement> -- python3 <recette> <SHA>`.
Primitive commune prend poste et fixe deadline ; driver lit reçu/échéance réelle,
réserve5s pourfin. Compile natif+code JSON et archive ELF/hash/source embarquée,
listage exact1test avant runtime, ldd vérifie native29 ; runtimeattendu101/0passé/
1échec/0ignoré surmessage d'omission malformée. **TEMOIN_ROUGE_POSE** est le
résultat métier attendu, un éventuel VERT du superviseur veut seulement dire
exécution/arrêt/nettoyage conformes, pas « test passé ». Pas d'ELF choisi au mtime,
sourcefigée/propre avant/après chaque phase, aucun modèle/service/PG ou DB réelle.

### Suite RO — rouge observé puis correctif et témoins publiés

Source rouge19030f243b29e534c31804ebfe7daecc1fa8afd8, compile132s, ELF réel
rag3weaver-b51f42378bbb8e05 SHA45623a3364aa9e987ce1de1d9b4e130f6187e2b38c6b8b20513de46187be0507,
JSONnatif+code/listageexact1 avantruntime. **0passé/1échec/0ignoré,rc101** :
relation:Broken=not-json omise, inspection rendueOk (doncCOMMIT) au lieu de refus.
Fin133,34s dans fenêtre600 ; preuve cx-observation-ro-rouge-v1/ et confinement
cx-observation-ro-confinement/validation-duzfhijx/ : VERTdu superviseur=arrêt/nettoyage
conformes, pas testvert ; restes=[],tmpbusretiré, placeArbrerendue(EDreste1/4).

Correctif : catalog::metadata partagé pourlignesstrictes et décodageEntityConfig/
RelationDef, consommé par les DEUXloadersCatalog et inspect_on. Pas d'initialisation
ou écriture ajoutée, ni format changé. ObservedCatalogMetadata scellé(aucunbuilder/
Deserialize/champmutablepublic), namespaces Optiontypés et journal existant ;
ConnectionView avanttransaction, ReadOnlySnapshot seulementaprèsCOMMIT réussi.
COMMITdéfaillant tenteROLLBACK et conserveégalementsonerreur éventuelle, sans
rendre d'observation. AppliedMigration devient typepubliclisible/Serialize/Clone,
champs inchangés nonpublics et getters ; aucun statutApplied ou droit inventé.

compare_with_supplied_schema appelle le SEUL producteurA puis vérifie chaque
config/RelationDef **persisté** contre les faits effectifsfournis. Définitions
supplémentaires/fournisseur/politiques/recettes/modèles restentfournis : le rapport
sépare observed_metadata et supplied_schema, sans transformer le sous-ensemble
persisté en schéma completobservé. Pas de compose/hashdupliqué, empreinteentités
historique inchangée, admission/preview/génération/apply non requalifiés.

14nouveaux témoins au total :10callbacks inspection(dontrougeinitial),3décodeurs,
1natifDBsynthétique64MiB encréation puisvraieRO64MiB, snapshotconfigs+relations+
journal et comparaison A sans changementdesoctets des fichiers. Absent≠vide,
transactions/échecsbegincommitrollback, invalide/dupes/journalinconnu, empreinte
entitésstable avecnouvelle relation, politiques/entitéssupplémentairesnonobservées,
mutation deschampsdiagnosticpublics sansaltérer la vue scellée, contenus/propriétés/
extrémités/manquantscontradictoires couverts. Parse6sources rustfmt --emitstdout
skip_children réussi, aucunefichier réécrit ni compilationlocale.

Proposition ciblé correctif séparé du rouge(initialrouge133,34/600 terminé) :
recette preuves-observation-ro/cible.py publiée avec SHA avantjob, mêmeprimitive
commune/délai600/lib29/j8/CPU4/targetunique32 ; aucune globale3600 ouverte.
CompileunefoisJSONnatif+code, archiveELF/source/SHA/ldd puis liste filtresinspection,
décodeursmetadata,migrations avantruntime, aucunignoreaccepté. Exige10nouveaux
callbacksRO+3décodeurs+1natiflectureoctetsinchangés,18faitsA+15aperçu et groupe
migrations existant ; compteglobal tiré de l'ELF, pas d'assert historiqueinventé.
PremierrougeSTOP/diagnostic, aucunretryinchangé, aucunmodèle/service/PG/baseréelle.
La passeglobale se proposera après ce ciblé/diff/revue, horsposte push.

### Suite RO — ciblé83vert, préparation globale conditionnée au master ED

Source91455ec71b9297c4258d258e4b014b409a861736 **83/0/0**,23,54s dans600 ;
inventaire exact83/aucunignore avant runtime,14nouveaux (10callbacks+3décodeurs+
1natif64MiB/octetsinchangés),18faitsA+15aperçu etancienscontrats. ELF réel JSON,
sourceembarquée etlddnative29 attestés/archivés, sourcepropre ; commonconfiner
validation-19kvifpk VERT/restes[]/tempbusnettoyé. Preuves cachePC
cx-observation-ro-cible-v1/ ; revue statique mémoire et C7 favorables.

Rebase documentaire/outillage CI0bec438da sur branche NEUVE
cx-arbre-observation-faits-2 :4/4range-diff identiques, Rustinchangé. Le ciblé ne
se rejoue pas pour workflow/helperCPU. C7 fixe ordre **EDaccordshôte d'abord,
RO ensuite** : seule la base de la future globale attend le masterEDconfirmé ;
pas de Cargo Arbre actif ou fenêtre3600 ouverte pendant cette préparation.

Recetteglobale préparée horslourd dans preuves-observation-ro/{globale.py,
finaliser.py,lancer-globale.sh}, àgeler/publier avecsource aprèsrebaseED surbranche
NEUVE. **GO C7 conditionnel EDintégré** : commonconfiner3600depuisacquisitionposte,
unseulbusprivé sansactivationpourTOUTEleportée, Max32/HIGH0,j8/INCREMENTAL0/LUCIVY8,
CPU4/cachesjournauxbuiltinprivés/CARGO_MANIFEST_DIR exact, targetunique32/lib29.
Client/superviseur cwd/var/tmp(éviterfauxoccupants), navigateur natifsanswrapper
busparChrome/TMPprivédisque /var/tmp/cx-ro.*, aucun changementdeflags oubusLucie.
Préserver DBUS_SESSION_BUS_ADDRESS et reçu/jeton issus duvraisuperviseur commun.

Capsintersectés avec MÊMEdeadline3600 : F51500, vrai minimumchecklib sansfeatures120,
compileliblarge/listes1200, libruntime300, NAVcompile/listes/runtime360,
CLIcompile/listes/runtime180, PG300, attest120. Pas de délai remisàzéro ni replay
inchangé. Huitfeatures commeA ; CargoJSONparcheminsworktreesources, ELF --list et
--ignored --list AVANTruntime, noms de succès (suffixeshouldpanicreconnu), compte
libdérivéELF. Requis10callbacks+3décodeurs+1natifRO+18facts+15aperçu+lien80,0ignore
danscesgroupes. NAV40, CLI5/1groupe ciblee2eexistante, PG36/1suitevia run_e2e,
aucunsaut ; PGinventaireavantCLI/PG, ELFPGdoitêtreexactementceluiidentifiéJSON.

PG **neuve cx_ro_2191_20261112_v1**, absence obligatoire/owner/current_database
vérifiés avantlib puisavant PG ; connexionexplicite5433/DBnomméegardéehorslog,
jamaisbaseimplicite/métadonnéesréelles. Source/template220/4ELFhashés etarchivés,
maps réelles dechaque ELF/native29 ; propreobservateurPID/starttime/cgroupsMax32,
devicedri/kfd FDrelevés sansdéduireinférence, quiescenceNAVetfinalescanexe/cwd/profil.
Finaliseur aprèsreelrcsuperviseur : exigeverdictcommun/restes[]/busnettoyé,
rehash4ELF,scanindépendant puisseulTMPpossédéretiré, VERDICT_COMPOSE rougepar défaut
siéchec. Aucun fichierjoué éditépendantjobs. Premierrouge STOP/diagnostic, pasapply,
générationpersistante, GPU/modèle/service/DBLucie. READMEpréparé, nouveautés/bilan
sepublierontsurpreuvesfinales ; ticketresteouvert.

### Suite RO — GO actualisé pour valider l'union empilée, sans attendre master ED

C7 lève l'attente de master pour la VALIDATION : base EDexacte
**143a21706abed7c38b62d036a4a8d4c9af170790**, publiée sur
origin/cx-ed-accord-publication-3 ; tip **2a31b271689df34aab77cc6170e623229cb47be7**
contient143 etson delta src/tests/Cargo/templates estVIDE. AncienED-2b229
explicitementrefusé commebase(codeobsolète) ; jamaisutilisé.

Branche NEUVE **cx-arbre-observation-faits-3**, six SEULScommitsRO rebasés depuis
baseCI0bec surED2a31 ; **6/6range-diff identiques**, têteempilée e32e482c6 publiée.
Preuve **union-ed-ro.json** : tous les blobs src/tests/Cargo/codeparsers/templates
égalent EXACTEMENT ED +les6cheminsRustRO,220templates ; scriptscritiques
commonconfiner/lib/cargo identiques à ceux revus. Aucun fichierED édité parArbre.
Reprise/README/recettes àjour ; sourceglobal SHAfinal aprèsceMD etfreeze avantjob.

GO3600 inchangé, mêmescaps/procédure/scripts décritsci-dessus. Commonconfiner
prendposteAVANTéchéance, CPU4/Max32/j8/lib29/target32/dirsprivés/CARGO_MANIFEST_DIR,
PGneuveexplicitcurrent_database, busuniqueentièreportée, inventaires réelsELF,
4ELF/source220/maps etquiescence+rcsuperviseur requis. EDpeutqualifierhôte enparallèle,
4lourds max. **IntégrationB reste APRÈS EDmaster ETsonverdict13** ; validation de
l'union n'est pas un masterED inventé. Si EDchangeRust aprèsbase ou diff produits/
tests/Cargo/assets final nonvide : STOP/requalification. Si arbresED+ROidentiques,
conservercettepasse sansrejeu pourIDs decommits/MD seuls. Aucun apply ou baseLucie.

### Suite RO — globale de l'union verte et rebase sur master ED équivalent

Source figée **f19e3cbb034d7bfac09a5314c2abbd422d76461e**, début21:59:29,
fin22:10:53, **685,11s dans3600** après acquisition du poste, aucun incident ni
réessai. F5 **5/5**149,65s ; vrai minimum checklib37,04s ; compile lib227,68s puis
**lib2243/0/65**46,69s ; NAVcompile36,70s/**40/0/0**137,75s ; CLIcompile35,23s/
**5/0/0**, PG **36/0/0,1suite et2filets verts** en8,29s runner inclus. Tous les
inventaires réels ELF ont précédé leurs runtimes ; les14nouveaux RO/décodage,
18factsA+15aperçu+lien80 sont passés sans ignore. Même ELF e2e pour CLI et PG,
confirmé dans la sortie du runner ; PG nom/rôle revérifiés avant sa suite.

Quatre ELF/source/native29/maps/220gabarits chacun attestés, copiés et rehachés
par le finaliseur indépendant. Superviseur commun validation-lmo8tclm VERT,
code0/restes[]/busphysiquementabsent ; finaliseur VERDICT_COMPOSE VERT,
quiescence indépendanteexeTarget/cwdWT/profil=[],TMPNAV marquépossédé retiré.
HIGH/MAX/OOM/OOMkill0, aucun Cargo Arbre actif, aucun service/modèle/GPU nouveau
ou DB réelle. PG synthétique cx_ro_2191_20261112_v1 et target conservés pour revue.
Preuves PC cx-passe-observation-ro-v1/ ; copieslocales cx-preuves-observation-ro-final/,
résumé versionné preuves-observation-ro/bilan-global.json. Les rouges19030 et
ciblé91455 83verts sont conservés séparément avec leurs ELF2postes.

ED a depuis intégré son code et ses témoins13verts sur
**0bf06b21a5fda28520f6ae0b2b2ad07acbeac331**. Comparaison exhaustive src/tests/
Cargo/codeparsers/templates : EDmaster=ED143 ; rebase B sur branche NEUVE
**cx-arbre-observation-faits-4**, **8/8 commits identiques**, tête7145fa36f avantceMD.
Arbres produits/tests/220assets/Cargo STRICTEMENT identiques à la source jouée
f19 ; aucun faux masterED, aucun replay pourIDs ouMD. Range-diff et équivalence
conservés dans preuves locales finales. README/nouveautés/page02/reprises mis
àjour ; revue indépendante mémoire des4ELF/220hashes/native/CLI/PG/quiescence
favorable, pas une autorisation master de sa part.

Attente revue finale et tour C7 explicite avantpushB ; ticket demeure ouvert,
provenances observée/fournie distinctes, aperçu toujours non qualifié au stockage,
génération/bootstrap/apply/publication toujours non livrés. Après tour/push et
3contrôles : retrait réglementaire des seules ressources B, remotes et preuves
conservés. Aucun autre ticket ou lot clos par ce raccord.

### Suite RO — intégrée sur master, ressources propres retirées

**Master8e24c0055bd81a58e6204d346f0582fc214af8f1**, « à toi » C7 puis poste
push/dernierfetch a0674bde4 documentaire, branche neuve5, **9/9range-diff identiques**.
Contrôles LUS : statuspropre, aucunrebase, origin/master=HEAD. Source qualifiée
f19 conservée, unionED143+RO exactement préservée sur src/tests/Cargo/codeparsers/
220assets. Pas de replay pourMD/IDs. CI vue parposte push verte le12nov ; aucun
rerunCI. Ticket migration reste OUVERT : aucune origine/génération durable,
transfert réel, bootstrap/apply/publication ou qualification physique de l'aperçu.

Nettoyage immédiat terminé : WTcode LOCAL+luciepc retirés, targetPC retiré,
brancheslocales propres comparées aux remotes puis branch-d ; remotes conservés,
aucun GC/prune. Chaque administrationGit(sous-module/symlinks inclus) copiée
fidèlement, helperSHAdca13abb7a84cf3c1419f728697837a6b1b9b20e34e34c144c9fbf835252f628
vérifié et gardeJSONfraîche VERT LUE avant retrait. Quatre ELF et preuvesbrutes
copiés/rehashés aux deux postes avantcleanup, aucun fichier d'autre session édité.
PGseule cx_ro_2191_20261112_v1 : propriété/0sessions contrôlées, pg_dump puisSHA
**10334f9cd35bab37c18c6da1ed67061985e36bf1f066dbcca0ca6854c82f8957**, DROP/absence
vérifiée ; dump et rapports distants copiés/rehashés sur portable.

Précontrôle distantv1 a refusé3PIDstransitoires [476177,476178,476181] avant PG/
admin/garde/retrait. Tous physiquement absents au contrôle suivant ; aucun arrêt
forcé ni exception aux occupants. V2 conserve l'arrêt initial, attend au plus10s
une quiescence réellement vide avec échantillons puis applique le même invariant
et la garde fraîche. Ce n'est pas un contournement de garde ni un rouge produit.
Rapports : cache cx-nettoyage-observation-ro-v1-local-8e24c0055/ et
cx-nettoyage-observation-ro-v2-remote-8e24c0055/ (fin.json,garde.json,administration,
quiescence-avant.json), arrêtv1 distant dans arret-precheck.json. Copieslocales
cx-preuves-observation-ro-final/nettoyage-remote/. Preuves, ELF, rouges et remotes
gardés ; zéro Cargo/place/service/client/modèle Arbre actif, aucune DB réelle.

Un dernier worktree documentaire neuf ne sert qu'à publier ce bilan, sans Cargo,
puis sera retiré par archive/garde fraîche. Les instructions d'attente masterED,
de tour B et de conservation des ressources des paragraphes antérieurs sont
historiques ; reprendre une suite sur WT neuf et contrat relu, pas sur un vieux job.

## Suite nocturne autorisée après A et RO — 12 novembre

Lucie demande de découper les suites utiles cette nuit. Arbre reprend le protocole
de publication/génération composite choisi, sur le même ticket ;
[répartition et frontières](../../orchestration/12-11-2026/02-lots-independants-pour-la-nuit.md).
Composer plan/journal et observation existants, puis publier contrat/diff/témoins
avant lourd. Identité logique neuve explicitement établie seulement sur un
catalogue neuf possédé ; pas de bootstrap automatique de legacy. Publication
P1→P2 et retour A→B→A, même opération rejouée, identité/génération étrangères
et interruption à éprouver sur fixtures. Un état incomplet refuse ; aucun
transfert ou format appliqué ne découle de la seule mécanique de journal.

Mémoire relit sans édition ; source/targets distincts de l'agent IDE, de la
navigation et des modèles. Le ticket reste ouvert pour la pyramide effective.

### Nuit — contrat de la première publication persistée, avant code

WT neuf cx-wt-publication-composite/branche cx-arbre-publication-composite depuis
a783013f1. Lecture effectuée des plans figés/PlanJournalState, journal historique,
transactions DbConnection, KV Catalog, décodeurs RO et producteur/canonique A.
Contrat concret publié dans
[03-publication-composite-sur-catalogue-neuf.md](../12-11-2026/03-publication-composite-sur-catalogue-neuf.md).

Périmètre proposé : catalogue natif neuf dédié aux instantanés déclaratifs complets
A, origine explicite préallouée, valeur/tête/publications/reçus atomiques sous vraie
transaction. Empreinte versionnée de la déclaration complète, pas des données d'un
index ; P distinct de H. Intent figé attenduO/P/H et cible préallouée, rejeu exact
sans effet, reçu historique distinct de tête courante, nouvel intent après
requalification sans muter l'ancien. ReopenOwned refuse legacy/contrat incomplet
et contrôle RO avant écrivain puis sous exclusivité. Ni storage_descriptor ni
PlanJournalState::Applied ni format/transfert appliqué. Cette frontière évite un
journal seul annoncé comme migration ; le contenu déclaré est réellement persisté.

Réutilisation A/canonique/KV/transactions/générateur d'identifiants d'événements ;
pas de Cargo/lib.rs/modèles. Extraction étroite de l'écriture KV de Catalog prévue,
petit export migrations.rs et Deserialize sur entrées A seulement, pas sur faits
scellés. Aucun fichier de l'agent IDE/navigation/modèles édité. Mémoire revue seule.
Catalogue dédié fermé (aucune donnée externe/parents) pour établir la complétude
du contenu couvert ; coût O(historique) explicité, pas d'annonce scalable.
Aucun Cargo, target, PG, modèle, GPU ou service lancé. Diff et recette/témoins à
publier avant lourd ; reste à implémenter/revoir/qualifier, ticket toujours ouvert.

## Suite publication composite — diff préparé

Contrat d0f251633 relu favorablement par orchestration et mémoire. Protocole
fermé et adaptateur natif privés dans `migrations/publication`, écriture KV
partagée avec Catalog. Onze nouveaux témoins prévus, dont deux interruptions
natives en enfant (avant COMMIT et réponse perdue après COMMIT), complétés par
les témoins A/RO existants. Source et recette `preuves-publication-composite/`
publiées avant ciblé luciepc unique600s sous superviseur commun/lib29/CPU/j8.
Pas encore compilé ni exécuté ; aucune capacité annoncée livrée à ce stade.

### Revue mémoire : colonne étrangère (avant ciblé)

Sur f6d7feeec, le registre ferme les tables/clés mais pas les colonnes de
`_catalog_meta`. Une colonne `foreign STRING` avec une donnée conserve la même
clé/valeur du registre et peut être admise. Critère du contrat fermé déjà fixé,
pas nouvelle décision produit. Le témoin natif dédié est publié avant exécution
pour poser ce rouge ; correction par introspection native `table_info` ensuite.
L'attente poste v1 a été annulée avant créneau/Cargo par TERM de notre PID1085759 ;
son finaliseur ROUGE rc143 est conservé hors worktree (annulation de revue).

Premier ciblé rouge v2 arrêté avant runtime : E0277 dans le diagnostic du
témoin enfant (`String + &Cow<str>`), source4b592eca8. Preuves luciepc
`cx-publication-composite-rouge-v2/compile.log`, confinement
`cx-publication-composite-confinement-rouge-v2/validation-tmk95sii/verdict.json`
ROUGE/code1/restes[]/nettoye=true. Correction de formatage du diagnostic ;
recette rouge v3 distincte publiée avant reprise, gate schéma pas encore corrigé.

Rouge fonctionnel établi v3/sourceb59be0fe8 : 0/1/0, message exact
« foreign column admitted by closed declaration catalog ». Binaire inventorié
avant runtime, lib29 liée, `cx-publication-composite-rouge-v3/` conserve logs/ELF.
Supervision `validation-jsakyq9f` : exécution du témoin rouge attendue achevée,
aucun résultat applicatif vert déduit. Correction : `table_info` natif dans la
même transaction, colonnes/types/défauts/clé primaire exactement fermés.
Ciblé complet v4/source et recette publiés avant nouvelle exécution600s.

Ciblé complet v4 VERT sur386861a5d : **95/0/0**,12 témoins publication
(incluant aide enfant),83 régressions A/RO. Finaliseur composé VERT,
`validation-alg4bj_0` code0/restes[]/nettoye=true, bus absent, scan propre[],
ELF rehashé. Revue mémoire du correctif favorable. Rebase sur24f3603cf,
nouveau nom `cx-arbre-publication-composite-r1` ; diff `extension/rag3weaver`
entre source qualifiée et pointe rebasée vide, même code testé.
Recette globale publiée avant exécution :5400s communs, F5/min/lib/NAV40/CLI5,
PG36 sur nouvelle base synthétique `cx_pub_2191_20261113_v1` et quatre suites
`e2e_tx_*` exigées pour le helperKV partagé. Aucun modèle/service réel requis.
Pas encore de passe globale ni d'intégration ; le ticket reste ouvert pour
la migration physique et la pyramide.

### Représentabilité des entrées typées — arrêt volontaire globale v1

Inspection de ModelSource/Addresses : le tuple public `Addresses(vec![" a, b "])`
se sérialise en une chaîne, mais sa désérialisation commune parse/trim/sépare les
adresses. A valide ces déclarations sans tester ce roundtrip. Le registre strict
peut donc écrire une valeur qu'il refusera à la prochaine lecture. Nouveau témoin
à poser sans modèle réel ; porteur arbre, même ticket/contrat.
Critère : les instantanés acceptés doivent être représentables sans changement
par le codec v1 ; une entrée typée non stable est refusée avant tout effet, pas
normalisée en silence sous une identité déjà calculée.
La globale v1/sourcea754d66af a été interrompue volontairement pendant F5,
avant qualification (superviseur proprePID1164753/starttime vérifié, TERM).
Preuves `cx-passe-publication-composite-v1`, supervision `validation-62gukwsz`,
verdict global ROUGE rc1 ; aucune passe verte annoncée. La PG synthétique
cx_pub_2191_20261113_v1 reste possédée, à conserver/dumper avant retrait.

Rouge codec v5/source943bb4a38 lu :0/1/0 message exact
« non-lossless declaration admitted by publication snapshot ». Entrée normale
issue du parseur acceptée et registre complet roundtrip exact avant assertion
rouge. Preuves/ELF `cx-publication-composite-rouge-v5/`, supervision
`validation-x094ljo8` code0/restes[]/nettoye=true (exécution du rouge attendu).
Correctif au constructeur avant Hpublication/effets : validation A, roundtrip
canonique complet ET égalité typée des mapsModelSource (Eq inclut AddressesVec),
puis calcul H. Pas de normalisation silencieuse. Matrice comma avec/sans
espaces et éléments fusionnés/vides, plus contrôle natif root absent/octets
inchangés avant tout catalogue ; entrée normale et valeur riche persistée
qualifiées par le nouveau ciblé v6 prévu14nouveaux+83régressions.

Ciblé v6 VERT sur5f710dd21 : **97/0/0** (14publication +83régressions),
compilation23,27s, phasecomplète23,89s. Composé VERT, supervision
`validation-zxa2aw0p` code0/restes[]/busabsent/scanpropre[]/ELFrehash.
Mémoire revue5f710 favorable, omissioncodec levée danscode etruntime.
Nouvelle globale v2/source etrecette publiées avant exécution5400s ; nouvelle
PGsynthétique cx_pub_2191_20261113_v2, aucun reset de v1. Les preuves des deux
rouges fonctionnels et des interruptions volontaires restent séparées.

Globalev2/source904af7c20 arrêtée premierrouge : F5 VERT5/5 en74,86s,
check minimal ROUGE (Registry/impl jamaisutilisés quand nativeabsent,
-Dwarnings/dead_code). Pas de runtimeglobal. `validation-e9fl03fc`
ROUGE/code1/restes[]/nettoye=true, proofs cx-passe-publication-composite-v2.
Correction ciblée : cfg(any(rag3db-native,test)) pour registre/impl et ses
imports de collections ; aucun allow dead_code ni relâchement -Dwarnings.
Recetteglobalev3 publiée avant reprise5400s, PGv3 neuve sansreset des2PG
antérieures. Mêmecodefonctionnel97qualifié ; globalecouvre minimum etruntime.

Globalev3/source626d050d3 : toutes phases runtime VERT (F5/min/lib2257,
NAV40/CLI5/PG36 et4e2e_tx), puis ROUGEattestation `maps runtime manquantes,
backend`. BackendCLI trèscourt (CLI5en0,12s) échappe à l'échantillonnage25ms ;
aucun défaut fonctionnel observé, mais aucun verdict composévert déduit.
`validation-fhuit77b` ROUGE/code1/restes[]/nettoye=true.
Correction recette : capture déterministe /proc du VRAI backend arrêté au
main par GDB après chargement dynamique, puis --help termine normalement.
Maps/PID/starttime/exe/cgroup/native/SHA conservés ; pas de service/modèle
ni de nouveau binaire. Probe30s publiée avant usage sur ELF626déjàconservé ;
puis globalev4 unique5400s avec PGv4neuve et même capture exigée en plus des
observationsruntime. Aucun seuil d'hygiène ou preuve source n'est supprimé.

ProbeGDBv7 : maps bien capturées au main, mais ROUGE car le backend n'offre
pas de `--help` racine (interprété en chemin de manifeste, rc1). Erreur de
recette, pas défaut produit ; `validation-24tj01jo` ROUGE/code1/restes[]/nettoye.
Recette corrigée après lecture CLI : `migration inspect --manifest` synthétique
vers un stockage explicitement absent, résultat null/code0 et absence de
fichier vérifiée, manifeste inchangé. Aucun modèle construit ni catalogue
créé. Probev8distincte30s publiée avant reprise, puis globalev4 inchangée dans
ses exigences. GDB--nx sans réglage utilisateur ni téléchargement debuginfod.

C7 autorise explicitement la composition runtimev3+attestationautonomev8,
sans rejouer les suites vertes, sous identité stricte des4ELF/source/gabarits
et budgetINITIAL5400 inchangé. v4 déjàlancée a été interrompue immédiatement
surPIDpropre1507246 pendantlibcompile, avantNAV/PG/tx ; supervision
`validation-a755jpea` ROUGEinterruption/restes[]/nettoye. PGv4 conservée.
Probev8 VERT `validation-kj3l5tap`, backend626SHAe372…capturé au main,
migrationinspect null/code0/manifesteinchangé/stockageabsent. Sources produit
626→03d strictementidentiques (diffextensionvide), seuls docs/recettes changés.
Composition publiée avant lecture : conserveROUGEoriginaldistinct, vérifie
4copiesELF626/source+220gabarits+maps et quiescence, retireseulTMPNAVmarqué,
produit un verdictcomposé explicite sous anciennedélaisansreset. Aucune suite
ni nouveau backend ne sera exécuté par cette composition.

Composition C7 runtimev3+attestationv8 source626d050d3dcd9316bb8a1fc923bf71fcd0e36baf VERT en1440.33s/budgetINITIAL5400, F5 5/5, minimal et lib 2257/0/65, navigateur 40/0/0, cli 5/0/0, postgres 36/0/0, e2e_tx_ligne_a_ligne 3/0/0, e2e_tx_merge_puis_copy 1/0/0, e2e_tx_par_paquet_arret 18/0/0, e2e_tx_vecteurs_apres_rollback 3/0/0. Quatre ELF/maps/native29/220gabarits copiés et rehashés deux postes, supervisionv3ROUGEcartographie/v8VERT, deuxrestes[]/busabsents/TMPNAVretiré. Prêt pour tour C7 ; ticket reste ouvert pour pyramide/application physique.

## Requalification de l'union sur master symboles — en cours (13 novembre)

Master est désormais `8f604328a` (CI C++ et symboles intégrés). Lot reposé
sur cette pointe, branche neuve `cx-arbre-publication-composite-r2`, après
résolution documentaire conservant les deux entrées de nouveautés. Les chemins
Rust de publication et helper KV sont identiques à626 ; le vrai delta entrant
est la navigation des symboles et trois graphes (223gabarits au total).
L'ancienne composition626 reste une preuve de son ancienne source, pas de cette
union. Nouvelle recette/source publiées avant globale v5 bornée5400s, native29/
CPU/j8/32G/targetunique. Attend explicitement14témoins publication et9symboles
nonignorés dans la lib ; F5/min/lib/NAV40/CLI5/PG36/4tx. PGv5neuve, zéroreset.
Aucun push master avant requalification et tour C7 frais. Les sections suivantes
conservent la qualification historique626 et ses preuves séparées.

Union31db55abb/master8f604328a VERT globalev5 en628.58s/5400 : F5/min et lib 2266/0/65, navigateur 40/0/0, cli 5/0/0, postgres 36/0/0, e2e_tx_ligne_a_ligne 3/0/0, e2e_tx_merge_puis_copy 1/0/0, e2e_tx_par_paquet_arret 18/0/0, e2e_tx_vecteurs_apres_rollback 3/0/0,14publication+9symboles requis ;4ELF/SHA/maps29/223gabarits rehashés2postes, superviseurcode0/restes[]/busabsent/scans[]/TMPNAVretiré. Prêt tourC7frais, pas encore intégré. Ancienne composition626 reste distincte et historique.

Publication déclarative intégrée masterbc1a81500 (GO C7/ifVERT/3contrôles lus),
sourceunion31 globalev5 VERT628,58s : lib2266/65,NAV40,CLI5,PG36,tx25,14pub+
9symboles/223gabarits/4ELF2postes. WTcode/target retirés,5PGown exportées/SHA/
DROP/absence et gardes administratives fraîches VERT. Bilan/preuves dans les
rapports et caches indiqués. Le ticket reste OUVERT pour le transfert physique,
l'adoption de stockage et la pyramide ; le registre ne certifie pas ces suites.

## Suite après livraison — 13 novembre, reprise par arbre-principal

La dépendance publication est levée : bc1a81500 est intégré et ses ressources
privées sont rendues. Le parcours IDE est également intégré par43e4f185b.
La suite cohérente autorisée commence par le raccord au stockage réel, sur un
nouveau worktree depuis master et des fixtures synthétiques neuves possédées.

Publier d'abord le prochain contrat concret et la frontière du diff : comment
le plan figé, la publication déclarative et l'état physique qualifié se relient,
quels faits manquent encore avant de pouvoir ouvrir une vraie vue en couches.
Choisir une tranche verticale minimale qui crée puis rouvre un index réel neuf,
et permet d'en vérifier contenu, occurrences/relations et publication effective,
en réutilisant runner, transactions, fournisseurs et gardes existants.
La préparation doit préciser les témoins d'interruption/reprise, la distinction
entre déclaration et application, et les refus avant tout effet non qualifié.
Éviter un second registre qui ferait seulement avancer des métadonnées.

Arbre reste porteur ; Mémoire relit le contrat et ED les frontières communes
si elles changent. API, diff et recette ciblée se relisent avant un nouveau
lourd ; cette préparation n'autorise pas à annoncer un format physique appliqué.
Aucune base existante de Lucie n'est lue, copiée, migrée ou activée. Pas de GC,
de changement des services ou de raccord implicite d'un modèle. Les décisions
déjà prises, notamment publication distincte de l'empreinte, restent acquises.
Le ticket reste ouvert jusqu'au résultat physique demandé et à ses preuves.

Préparation physique sur master798ab1aa7 : contrat/API/fixture publiés dans
`docs/arbre-principal/13-11-2026/01-raccord-physique-index-neuf.md`. Trois
écarts relus : Catalog impose un modèle, runner n'a pas d'identité résolue
durable/montage des services d'ingestion, commit_lot précède la durabilitéFTS.
Proposition de profil BM25sansmodèle sur index neuf, journal unique existant
qualifié, candidate non servie avant fermeture physique/publication ; entrée
durable et même plan pour reprise. Fixture8occurrences/6versions/2cellules,
recette600s à relire avant lourd. Aucun Rust/Cargo/target/index/service créé.
Recherche seul éditeur de la garde d823 ; diagnostic RO transmis séparément.


Précurseur physique7d62a8113 publié, non qualifié : absence dense structurelle,
montage commun ingestion/migration, notifications après persistance FTS et
COMMIT ambigu nommé/poisonné. Deux témoins écrits, aucun Cargo joué. Revue
Mémoire favorable comme précurseur seulement. Embarquement relève avant toute
suite un cas sparse : None dense coupe actuellement ingestion/rattrapage alors
que set_sparse_embedder et les services peuvent annoncer sparse présent. Pour
cette première capacité strictement lexicale, refuser explicitement ces montages
et les entités/requêtes sparse ; ne pas annoncer une capacité non exécutée.
ED demande une lecture stricte du registre dans l'entrée qualifiée : ni erreur
absorbée ni unwrap_or_default comme preuve de registre vide.


## Revue du précurseur7d62 — 13 novembre, avant qualification

Le diff natif n'est pas livré ni compilé. Deux frontières du même contrat
restent à matérialiser avant avis favorable global ; aucun test réel nouveau
n'est revendiqué par ces constats statiques.

- `commit_lot` appelle `conn.poison` sur erreur de COMMIT ou de FTS après
  COMMIT, puis rend MustReopen. Le défaut de `DbConnection::poison` ne fait
  rien et `must_reopen` rend None ; PostgreSQL ne les redéfinit pas. Une
  connexion callback ou PostgreSQL peut donc recevoir MustReopen alors que
  `Catalog::must_reopen` reste None et que le handle demeure utilisable. Le
  nouvel invariant d'inutilisabilité ne doit pas dépendre silencieusement du
  backend. Poser un témoin causal avec connexion dont COMMIT échoue et poison
  est le défaut ; qualifier aussi le chemin FTS post-COMMIT. Choisir une
  invalidation commune ou borner explicitement le contrat, sans faux succès
  ni suppression de l'erreur d'origine/ROLLBACK. Pas de serveur PG requis
  pour ce contre-exemple CPU.
- L'absence dense structurelle peut coexister actuellement avec
  `set_sparse_embedder` et une entité SPARSE. Le registre annonce has_sparse
  et la requête sparse appelle le fournisseur, mais `peut_embarquer` et
  `preparer_l_embarquement` s'arrêtent sur l'absence dense : ingestion et
  rattrapage sparse n'ont pas cette capacité annoncée. Revue Embar : rendre
  cette composition cohérente ou refuser explicitement le profil non pris
  en charge. Ne pas créer de dense factice. Le profil initial reste BM25,
  sans élargissement implicite au transfert sparse.

Arbre conserve seul le code ; Mémoire et Embar restent en lecture seule.
Les gardes, leurs 29 témoins et le lot grep restent séparés. Préparation de
ces témoins puis source/recette publiées avant ciblé ; premier rouge inattendu
arrêté et diagnostiqué, aucune nouvelle autorisation humaine nécessaire.


Diff physique proposé après les réserves : programme Mermaid/records figés,
IngestEntitiesNode déléguant au Catalog, nouveau profil possédé BM25 sans modèle,
cellules créées avant P0 et transactions par cellule sous contrôle du runner.
Journal existant étendu sur le neuf seulement avec qualified_input et
qualified_result (préfixe accompli + relevé de valeurs), candidate fermée aux
ouvertures normales, fermeture/reprise FTS depuis valeurs exactes, reçu/tête/
journal publié atomiquement. Huit témoins natifs et cinq CPU préparés ; recette
600s/source/API/diff dans le rapport13-11-2026/01 et raccord-physique. Aucun
Cargo, index réel de fixture ni target nouveau encore lancé ; revue avant lourd.


Lecture statique après publication be9699e73 : BufferedBlobStore::Drop pousse
encore les écritures retenues, et delete touche directement son backend. Un
Catalog invalidé après issue de COMMIT inconnue ne doit pas retenter ces effets
via sa destruction, même si DbConnection::poison est sans effet. Même porteur,
même ticket : ajouter une invalidation de persistance partagée par le buffer,
qui refuse save/delete/flush et le flush de Drop sans jeter les octets retenus
avant destruction. Témoin CPU avec backend compté ; aucune exécution revendiquée.


Réserve complémentaire Mémoire consignée avant action : set_scope pouvait encore
écrire après erreur de COMMIT avec poison backend sans effet. La correction en
revue porte la révocation sur la famille de connexions du Catalog (writer,
read_pool/read_conn/sync et clones), avec délégation des transactions/settings
au backend et rollback réservé au nettoyage. Barrière de persistance partagée
du buffer avant fermeture/Drop, y compris derniers clones. Témoins CPU renforcés
COMMIT+ROLLBACK perdus, set_scope/Arc exporté refusés sans requêtes, première
poussée échouée qui réussirait à la suivante mais aucun retry après invalidation.
Recette adaptée à7 CPU+8 natifs, toujours aucun Cargo/test natif exécuté.


## Revue du diff physique be9699 — fenêtres avant qualification

Code encore non compilé/non exécuté. Les réserves suivantes appartiennent
au même contrat, sans nouvelle base ni nouveau lot de migration :

- Création avant P0 : physical_index.rs712 initialise le Catalog ordinaire,
  enregistre les entités, étend le journal et prépare cellules/FTS avant
  d'écrire STATE_KEY dans la transaction759. Le refus des ouvertures
  ordinaires642 ne reconnaît que cette clé. Un processus mort ou une erreur
  avant sa première écriture laisse donc une base initialisée sans marque
  d'appartenance physique : l'ouverture Catalog ordinaire peut la prendre
  pour un legacy. Poser un arrêt enfant entre initialisation et P0, puis
  exiger refus de toutes portes ordinaires sans nouveaux effets. Établir
  l'appartenance et l'état incomplet avant les effets de préparation, ou
  garder la matérialisation inaccessible jusqu'à sa création qualifiée.
  Aucune adoption implicite d'un reste ne devient permise.
- Invalidation déjà demandée : Mémoire relève que set_scope5101 peut encore
  atteindre les UPSERT ensure_scope_nodes sans check_reopen avec un backend
  poisonNOOP. Étendre le témoin COMMIT ambigu au changement de cellule et
  exiger MustReopen avec compteur d'écritures inchangé. Les autres portes
  publiques restent à relire, pas seulement execute_raw/initialize/shutdown.
- Drop déjà signalé par Arbre : fermer les handles FTS peut remplir le tampon,
  et BufferedBlobStore::drop retente flush après un échec. L'interdiction
  d'effets après invalidation doit atteindre le store et ses clones avant
  fermeture ; un simple if autour de flush dans Catalog::drop est insuffisant.
  Témoin : save_many échoue puis réussirait, invalider et lâcher Catalog plus
  dernier clone ; aucune tentative supplémentaire ni notification prête.

Arbre reste seul éditeur. Relectures Mémoire/Recherche/Embar limitées à leurs
frontières, sans test ni base créés par les relecteurs. Ces constats statiques
ne sont pas un rouge runtime joué ; source/recette/témoins avant ciblé600.


Revue indépendante Recherche be969 : lecture favorable bornée du journal,
programme et préfixe. Deux limites de qualification, avant nouvelle édition :
les injections COMMIT visent toujours la première cellule ; ajouter morts avant
et après COMMIT de la deuxième avec préfixe acquis. Huit fonctions natives
comprennent sept scénarios juges et un helper enfant, exercé par les sous-processus
du parent, pas huit scénarios indépendants. Recompter explicitement fonctions,
scénarios juges et exécutions enfant. Aucun runtime exécuté par Recherche.


Complément de revue Recherche, même source be9699 : les injections par nom
de frontière surviennent à chaque cellule et meurent toujours dans la première.
Avant de qualifier la reprise multicellule, ajouter un cas qui meurt dans la
deuxième après acquisition du premier préfixe, puis prouver absence de rejeu
des effets de la première, reprise exacte et fermeture lexicale des deux.
L'identité du point d'arrêt doit être explicite dans le fils de fixture.
Le total de huit fonctions natifs comprend physical_crash_child, auxiliaire
réellement exercé par les enfants du parent ; il ne constitue pas un huitième
scénario indépendant au lancement sans son environnement. Distinguer le compte
Rust brut, les scénarios juges et les sous-processus dans l'inventaire/verdict.
Ces réserves de couverture sont statiques ; aucun échec runtime observé ni
second exécuteur trouvé par Recherche.


Correctifs des réserves bootstrap/couverture en revue : claim de root immuable
et synchronisé AVANT création native ; trois constructeurs natifs ordinaires
et Catalog refusent cette appartenance, même incomplète, avant effets. Capacité
interne scellée root/origine/chemin exact, bootstrap limité au native vraiment
vide ; restes avant P0 quarantinés, aucune adoption automatique. Deux morts
enfant supplémentaires couvrent claim/init avant P0. Deux morts explicites :2
couvrent avant/après COMMIT de cellule2, avec préfixe natif acquis constaté et
reprise depuis le même support durable, fichiers sources retirés. Comptes
corrigés :9 fonctions natives (8 scénarios juges+helper),9 CPU,18 fonctions
brutes/17 scénarios juges/8 sous-processus de mort. Registre du nouveau nœud et
ports communs inclus. Syntaxe seule, aucun Cargo/runtime encore lancé.


Lecture transversale avant figer la source de qualification : usages du champ
embedder encore directs dans catalog/progress.rs après le passage en Option.
Adaptation du débit/origine/sonde à la capacité réelle : None rend débit inconnu,
aucune sonde et refus d'une mesure sans modèle ; Some et Some(Absent) gardent
leur sémantique antérieure. Le témoin lexical existant vérifie ces portes.
Aucune compilation/runtime encore jouée,9c12 reste un état de revue non qualifié.


Lecture statique finale avant lancement : l'ouverture publiée compare les valeurs
à la tête mais ne relie pas encore le relevé qualified_result appliqué à cette
tête ; une ligne de journal supprimée pourrait aussi passer pour l'absence de
journal du P0. Même périmètre, correctif avant qualification : absence de journal
permise seulement à la publication de création, et résultat appliqué complet
(préfixe final, valeurs/schema/checkpoints) obligé à correspondre à la tête.
Témoin de corruption après publication (résultat modifié ou ligne retirée),
refus RO sans nouvelle écriture ; aucun Cargo/runtime exécuté à ce constat.


Précision C7 sur la genèse, avant lancement : absence de journal admise seulement
sur le vrai SourceRevision::Genesis d'un registre validé à une publication unique,
pas sur le seul texte d'opération. Le prédicat de chaîne est explicite ; le juge
codec existant construit aussi une tête non-genesis nommée create-owned-index et
la refuse comme genèse. Source appliquée également vérifiée comme vrai Genesis.
Comptes inchangés19 fonctions/18 juges/8 enfants, aucun runtime lancé.


Premier ciblé8df arrêté ROUGE avant compilation : XDG_CACHE_HOME privé a déplacé
le magasin implicite de libs, exporteur refuse2 en0,154s avant execCargo. Target
absent/SDK29 réel intact/0native/0service. Superviseur validation-om027vtw code1,
restes[], bus disparu, autorité terminée, scan contexte own[],234 preuves SHA
identiques aux deux postes ; aucune relance inchangée. Correction de recette :
RAG3DB_LIBS_DIR explicitement épinglé au magasin réel, caches privés préservés,
nouveaux chemins preuves v2 et599s sans réutiliser/prolonger le reçu v1. Rust et
modèle inchangés ; nouveau SHA/argv à coordonner avant toute nouvelle exécution.


Ciblé correctifv2 source2e71e1755 arrêté au premier rouge en110,4066s : deux
E0609 dans catalog/progress.rs319/357, current_embedding_entry devenu Option mais
bilans IndexProgress.model restés String. Aucun ELF/tests natifs/DB/service/modèle
joué. Superviseur validation-rcz_mqen ROUGE/code1/restes[], bus absent, autorité
terminée, scan contexte own[] ; target unique conservé1 349 776 841octets (<32Go).
Correctif produit à relire avant nouveau lancement : model Option<String> dans
les bilans, Some garde son nom réel, None reste absence structurelle/JSONnull,
ligne lexicale sans modèle inventé ; appelant e2e configuré rendu explicitement
Some. Renforcer témoin lexical et exact fixture, préserver le rouge et les preuves.


Ciblé v3 source10e6c75270a93c4db7a5d42c69c9dbd5d658f46e : compilation
réussie23,595s puis ROUGE24,918s au contrôle provenance, avant inventaire/test.
Cause prouvée RO sur le MÊME ELF : bytes in mmap vautFalse alors que find
retrouve10e6c7527 à9822122. SHA original/copie conservée :
8387de3dd95ef2c3d290166d6d2049c0a4cf9f4cb29ace565da362b9b8a9426d.
Deuxième défaut de recette découvert sans exécution : search_base.mmd est sous
templates/tools/, pas directement templates/. Les octets exacts sont présents
dans cet ELF : tools/search_base.mmd à9788041, search_expansion.mmd à9722386.
Superviseur validation-e94smksf ROUGE/code1/restes[], autorité terminée, bus
absent, scan contexte propre vide ; target conservé1607645974octets. Preuves et
ELF copiés local. Aucune recompilation ni aucun test relancé. Suite proposée à C7 :
attestation autonome et seules phases jamais jouées sur ELF original10e6,
sansCargo, sous échéance originale887138.455654083 si coordination l'autorise ;
sinon nouveau budget correctif explicite, sans effacer ni requalifier le rougev3.


Attestation autonome recette4cc12159a : VERT RO, source/ELF/gabarits/SDK29
identifiés, zéroCargo/test. Complément autorisé par C7 sous échéance originale :
inventaire puis4CPU verts, premier autre rouge au5e CPU en0,846s. Aucun build
rejoué, aucun juge natif/enfant lancé. Superviseur validation-9ls9hhzp ROUGE/code1,
restes[], nettoyé. Le juge committed_rows_with_failed_fts échoue sur file vide.
Diagnostic RO : flush_blob_store émet explicitement CatalogEvent::Error avec
context blob_flush:lot avant de rendre IndexPersistence ; commit_lot jette les
annonces retenues et révoque les accès. Le juge doit exiger cette erreur précise,
puis file vide (donc aucune annonce de disponibilité), plutôt que supprimer le
signal d'erreur légitime. Correctif limité au juge, pas au chemin de production ;
à publier et relire avant nouveau budget correctif explicite, aucun rejeu inchangé.


Ciblé v4 bb894 : compilation23,437s, inventaire et9CPU VERTS. Groupe natif
ROUGE24,671s total :9juges échouent dans declarations() avant ouverture, même
refus du contrat search_index (champ dérivé content absent de Snippet). Helper
sans environnement passe mais ne juge rien ;2enfants lancés échouent101 avant
injection, aucun des8enfants attendus validé. Pas de VERT natif. Diagnostic RO :
validate_search_index_context exige que les champs cherchables dérivés existent
sur l'occurrence ; fixture déclarait body sur source et content sur dérivée.
Correction fixture seule : nommer body le champ rendu dérivé également, adapter
les attentes exactes/chunks/UUID/colonnes à ce nom, sans toucher au validateur ni
aux valeurs rendues name+docstring+body. Recette à scinder en juges natifs exacts
pour arrêter au premier échec à l'intérieur du groupe (libtest continue sinon).
Pas de rejeu avant publication/revue/budget correctif distinct.

Précision RO C7/Recherche avant publication : simple renommage body insuffisant
pour prouver les coordonnées du rendu composite. Correction retenue : déclarer
ET peupler Snippet.content avec le texte exact name\n docstring\n body (sans espaces
ajoutés), garder body/file/start/end distincts, conserver dérivée content et
assertion explicite égalité champ source/rendu, lecture native exacte du champ
source supplémentaire. Offsets chunks dans content rendu uniquement, aucune
projection vers body brut ou lignes fichier annoncée. Validateur inchangé.


P5 source39f798 : compilation/inventaire/9CPU verts puis premier juge natif
exact frozen_program_codec refusé par binder : PRIMARY KEY(_uuid) ne correspond
à aucune propriété déclarée. Aucun autre juge natif/enfant lancé, arrêt failfast.
Diagnostic RO précis : Catalog::migration_initialize appelle dialect.create_table
pour _DataflowMigration et _DataflowMigrationLock avec colonnes sans _uuid ; les
deux dialectes ont pourtant PRIMARY KEY(_uuid), et les écritures existantes
migration_record/acquire_lock ainsi que journal_row fournissent déjà _uuid.
Correctif partagé minimal : déclarer _uuid Text dans les deux listes de colonnes
existantes ; aucun nouveau journal, ni identité, ni reprise alternative. Le juge
natif existant couvre réellement ces créations ; avis RO avant nouveau budget.


P6 source4c35c3136 : compilation/inventaire/9CPU verts, premier juge exact
refusé Corrupt(table without explicit properties), durée24,735s ; autres juges
/enfants non lancés. Le journal dépasse le défaut binder précédent. Diagnostic
RO : PhysicalSnapshot refuse toute table sans propriétés, alors que les relations
CHUNK_OF produites par schema.rs374 sont légitimement sans propriétés. Leurs
extrémités ET la multiplicité sont des valeurs physiques à capturer. Correctif
adaptateur : autoriser zéro propriété seulement pour REL, capturer src._uuid et
dst._uuid même sans champs supplémentaires, conserver définition+connexions et
doublons. NODE sans propriétés reste refusé avec nom explicite. Aucun saut de
table/relations ni affaiblissement des valeurs, pas de rejeu avant RO/budget.

Demande C7 après RO favorable6bd : renforcer le juge natif existant codec par
contre-exemple physique 0/1/2arêtes sans propriétés aux mêmes extrémités dans
sa base synthétique, après ses assertions de plan. Définition stable et deux
identités noeuds fixes ; cardinalités0/1/2, deux lignes identiques conservées,
empreintes logiques distinctes. Aucun juge supplémentaire ni mock seul ; aucune
publication de la base ainsi altérée. Nom tableP6 toujours non attribué.


P7 source857a2 : neuf CPU et premier juge natif codec VERT, y compris
RELATION_WITNESS physique0/1/2. Deuxième juge exact applied_journal échoue à
l'ingestion Alpha : tampon natif plein, limite explicite64MiB du profil. Durée
25,170s, aucun autre juge/enfant lancé. Pas de panne SDK ou OOM machine déduite.
Cause : tampon copié du profil publication métadonnées, sans politique commune
du catalogue complet. Correctif proposé avant revue : porte owned opaque réutilise
config_with_buffer_pool(buffer_pool_choice(None)) et conserve BufferPoolChoice ;
recette épingle RAG3DB_BUFFER_POOL_SIZE=536870912 pour toutes ouvertures/enfants,
sous Max32GiB, comme témoin natif existant. Pas de nouvelle règle mémoire produit,
ni auto-retry après OOM : ouverture/reprise explicite inchangée et source neuve.


P8 source40d36eab4 : compilation9CPU et6juges natifs VERTS, dont publication
/reopen exact8occ6versions8refs2cellules0modèle, corruption journal appliqué,
bootstrap2enfants aux frontières prévues. Septième juge normal_catalog_open
ROUGE comparaison octets (32,461s total), derniers juges et6enfants de reprise
non joués. Diagnostic RO : le juge ouvre lui-même une connexion native owned RW
avec permis AVANT Catalog::initialize ordinaire ; cette ouverture autorisée et
sa fermeture/checkpoint ne sont pas un test de refus avant toute ouverture native.
Corriger le juge : exiger refus des3constructeurs ordinaires avec racine entière
inchangée, puis fournir à Catalog normal une connexion owned RO pour éprouver
la seconde porte initialize sans effets du cycle RW. Production/gardes inchangées.
Assertions octets conservées avec diagnostics noms/empreintes, sans déverser les
37Mo de tableaux d'octets dans le log. Aucun VERT complet ni rejeu avantRO/GO.


P9 sourcec19f :9CPU+7juges natifs VERTS, huitième juge reprise stoppé. Premier
enfant before-logical-commit atteint code91, puis ouverture RO de contrôle échoue
dans Database::new : vector ANY non résolu. Reprise NON qualifiée ; deux enfants
bootstrap seuls déjà qualifiés. Durée32,123s, pas d'autre juge/enfant lancé.
Diagnostic RO : initialize_journal émet directement ALTER ... STRING DEFAULT NULL,
contournant Rag3dbDialect::alter_add_column_default qui omet NULL explicite pour
éviter son type ANY (historique du3oct, COPY). Réutiliser cette primitive avec
ColumnType::Text et défaut NULL, sans changement de sens du journal. Lien causal
au rejeu reste hypothèse à qualifier, pas un défaut SDK démontré ; C++ sollicité
RO uniquement, aucun rebuild ni SDK nouveau autorisé. Base du témoin supprimée
par TempDir à unwind, limite de preuve explicitée ; logs/ELF conservés.

Pour ne plus perdre la base synthétique au prochain premier rouge, les TempDir
des seuls juges physiques deviennent gardés : nettoyage habituel au succès,
keep explicite lors d'une panique avec chemin PHYSICAL_FAILED_FIXTURE imprimé.
Le dossier privé sous TMPDIR du reçu garde alors DB/WAL/plan pour diagnostic RO
et copie des preuves. Aucune base réelle concernée, aucun changement production.


P10 e100 :9CPU+7juges VERTS ; reprise before-logical-commit VERT exacte8/6/8,
puis after-logical-commit échoue récupération : maintenance appelle porte publique
sur SnippetContent interne. Reprendre EXACTEMENT open_fts_handles_for :
forme.resolve_storage_target(entity,true), chaque stockage physique, source
comprise ; publics seulement omettrait un index entretenu. C7/Recherche convergent.
Défaut distinct observé :20blobs retenus dans un lecteur puis tentative Drop RO.
initialiser_en_lecture crée un BufferedBlobStore hors blob_buffer ; ouverture FTS
peut créer si absent et close commite le handle writable. Correctif autorisé C7 :
CypherBlobStore structurel RO (aucun DDL prépare/save/delete/batch avant query),
sans buffer, BlobBacked RO assemblé depuis fichiers lus via format snapshot
PUBLIC EXISTANT puis ShardedHandle::open_snapshot réellement RO (pas writer
intermédiaire). Refus absence/corruption sans CREATE. Coût eager et copie RAM
transitoire du paquet explicites ; profil déjà eager, Max32 inchangé. Ajouter
juge CPU store aucune écriture + assertion native handlesRO/root immuable et
index absent refusé. Compte proposé20fonctions/19juges/8enfants, pas GO runtime.
Fixture P10 conservée tmp/.tmp7Pmai8 (codecs/DB/WAL/plan), aucune mutation aprèsSTOP.


P11 ec2f : compilation10CPU etjuge codec natif VERT, deuxième juge arrêté sur
shard_0 sans fichiers dans helper snapshot. Fixture conservée tmp/.tmpAqrL9d.
Cause RO confirmée dans lucivy4.3.3 BlobShardStorage/BlobDirectory : fichiers
racine utilisent index brut, fichiers de shard utilisent BLOB_PREFIX+index/shard_N.
Helper avait repris index/shard_N sans préfixe. Lecture binaire RO de la fixture
retrouve effectivement Lucivy_Lucivy_SnippetContent__synthetic__Alpha/shard_0/meta
et _config, etc. Réutiliser la constante PUBLIQUE BlobDirectory::BLOB_PREFIX
(module blob_directory), sans hardcode/déplacement/mutation ni namespace nouveau.
Pas de lecture SQL/native hors recette ; root/shard manquant doit toujours refuser.
Échec suivant aucun modèle dense est secondaire à FTS indisponible, pas admission
d'un modèle ; aucun calcul/modèle exécuté. Pas de rejeu avant delta/RO/GO distinct.


P12 source1477e506e : compilation10CPU et4juges natifs VERTS, namespace corrigé
dépasse le refus précédent, deux enfants bootstrap qualifiés, aucune alerte de
blobs lecteur dans ces logs. Cinquième juge normal-reopen échoue assertion ajoutée
fts_handles() nonvide : recherche dans_la_cellule restaure la cellule courante
par défaut ; les handles réels Alpha/Beta sont garés. Corriger ce juge seul pour
exiger nonvide fts_handles_in de CHAQUE cellule sondée avant is_read_only/commit
refusé, pas accepter une boucle vide. Prod inchangée, assertions fichiers entiers
/index absent conservées ; fixture échec P12/tmp/.tmpy3EsNL préservée. Pasrejeu
avant publication/revue/GO distinct.


P13 source9c7361 :10CPU+8juges natifs VERTS ; six reprises après mort avec
préfixe0/1/2 et exact8/6/8, plus2bootstrap =>8enfants réels qualifiés. Lecteur
handlesRO/commitrefus/indexabsent/rootimmuable VERT. Neuvième juge négatif
source altérée échoue ROreopen ANY après ALTER foreign_value STRING DEFAULT NULL
écrit par le TEST lui-même. Même précaution DDL commune déjà connue, pas nouvelle
régression produit établie. Durée44,010s ; helperisolé nonjoué. Fixture conservée
P13/tmp/.tmpIo9yxf. Remplacer les DEUX injections foreign_value expliciteNULL
des tests négatifs par primitive dialect.alter_add_column_default(ColumnText,
"NULL") ; garder colonneétrangère/refus/instantanés/journalabsent inchangés.
Production inchangée ; nouvelle qualification distincte aprèsRO/GO.


P14 sourceEXACTa43c475f9ce1f7cbd6c1c2a0ee56719b521678f1 VERTE :44,321345s,
20fonctions/19juges/helper1,10CPU+9jugesnatifs,8enfants réels qualifiés,0ignoré.
Superviseur validation-gllfe1kq VERT/code0/restes[], autorité terminée, bus absent,
scan propre vide ;1653preuves+ELF SHAégaux2postes. Bilan et limites :
[qualification ciblée](../13-11-2026/02-preuves-ciblee-raccord-physique.md).
Tous rouges précédents restent distincts ; aucun GO global/master, tickets ouverts.


Préparation union autorisée C7 après P14 : branche neuve
cx-arbre-raccord-physique-union-r1, WTcache cx-wt-raccord-physique-union, base
origin/masterf8cbd6a48 (ED/Archi/garde inclus). Rebase privé publié6b6f12d33 :
22patches code range-diff tous= ; aucun conflit Rust, seul ticket parent en conflit
repris canonique privé qui contient les ajouts master (aucun retrait). Fichiers
master non chevauchants conservés exactement ; tous autres fichiers du lot
identiques à la source privée. Audit et range-diff sous raccord-physique.
Recette globale composée6000s à publier/relire AVANT GO lourd : F8+vrai minimal,
lib large/NAV40/CLI5/lectures27 distincts+hôte13/PG36+TXpertinents. BGE7879
existant autorisé par C7 pour lectures avec créneau réel futur,27distincts/28
invocations (26jugés sous ancien1 +CREATEexact mêmeELF/profil sansceflag).
PG synthétique neuf seulement, OID et dump à conserver avant retrait ultérieur.
Aucun runtime global autorisé à cette préparation. Contrôle CPU tickets a trouvé
un champ Atteignable absent sur ticketRO ; en-tête corrigé avant future F8.

Revue RO recette ed13 favorable Mémoire/Recherche, sansGO : deux métadonnées
à aligner avant gel (phase_caps lib900/nav600 ; elapsed depuis début réel
confineur deadline−6000, pas entrée pilote). Corrigées ; identité PG avant
dump renforcée OID/propriétaire/commentaire. Aucun Cargo/global lancé.

RO ED/Recherche/C7 source5a3338 : défaut PILOTE concret avant runtime, seconde
EXPORT sur profil lectures capturé préfixe encore LD_LIBRARY_PATH ; égalité
stricte du complément échouerait avant test. Correction : complément env-u/bash
DIRECT sur profil capturé déjà exporté, garder comparaisons exactes et maps.
Aucun budget global consommé. Revue C7 demande aussi observations explicites
PermissionError des processus protégés : enregistrer unknown/champ/pid/identité
et preuve cgroup ; cgroup illisible reste ROUGE, pas d'échappement toléré.
Quiescence vérifie les PID possédés même si exe/cwd protégés, jamais faux zéro.

Le relevé d'ascendance lit les stat de tous PID avant attribution par parent+
starttime, sans filtre UID qui ferait disparaître un descendant SUID. Aucune
lecture env/FD ni signal à un PID tiers : observations/actions restent bornées
aux descendants attribués ; scan final inclut aussi un PID attribué passé SUID.


Globale4e STOPpremierrouge lib code_tools::tests::a_closed_policy_refuses_and_an_open_one_reads_outside_the_source. F8/min/4compiles/ledger verts conservés ; runtime
lib non conclu, NAV/CLI/lectures/PGTX non lancés. Cause précondition CWD fausse
confirmée métadonnées/source par ED/C7 : /var/tmp sans Cargo.toml. Ticket
[correctif pilote](2026-11-13-le-pilote-de-lib-sort-de-la-racine-attendue-par-les-temoins.md).
Préparation séparée du runtime complémentaire autorisée, source4e etELF figés,
NOUVEAUreçu borné échéanceinitiale896572.526453239, aucune prolongation/GO runtime
implicite. Aucun changement de politique/test produit pour masquer le rouge.


## Reprise corrective distincte — décision C7 après expiration

L’échéance initiale 896572.526453239 est expirée : aucune reprise ni prolongation
de ce reçu. Le rouge global initial reste conservé. Source application EXACTE
4e5ae8a324f50313b74f70772eb2e92ee41fad6f et quatre ELF du ledger inchangés.
Pilote préparé hors WT source, étape corrective DISTINCTE de 3600 s après poste
réel, runtime complet avec cwd crate, sans Cargo/rebuild/F8/min déjà verts.
PG initiale cx_phys_2191_20261113_u1 conservée, identité/OID et dump requis avant
tout retrait ; nouvelle PG privée cx_phys_2191_20261113_u1c, absence préalable
et création unique contrôlées. Aucun DROP/reset. Revue et GO exact requis avant
lancement. Aucun résultat runtime global vert ; P14 ciblé seul demeure qualifié.
Les champs protégés UNKNOWN ne prouvent pas une absence d’accès GPU.
Voir [recette corrective](../13-11-2026/04-recette-corrective-runtime-physique.md).
