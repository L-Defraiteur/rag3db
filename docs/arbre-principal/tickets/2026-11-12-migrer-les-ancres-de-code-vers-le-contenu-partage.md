# Migrer les ancres de code vers le contenu partagé

- **État** : ouvert, premier patch contrat/admission/plan/préflight autorisé après publication du plan corrigé ; transformation et activation non autorisées.
- **Famille** : produit
- **Gravité** : capacité manquante ; le partage déclaratif1a ne transforme pas les index de code existants.
- **Atteignable en service** : non pour ce chantier préparatoire ; gabarits et index de production inchangés.
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
