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
lié au [ticket de préparation](2026-11-12-preparer-la-migration-de-l-ancre-sur-copies-privees.md).
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
