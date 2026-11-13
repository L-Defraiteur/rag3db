# Sous-lot2a : pile pure de générations épinglées

Préparation autorisée C7, source de départba0317ed1. Ticket créé avant diff :
[2a](../tickets/2026-11-13-resoudre-des-couches-epinglees-sans-confondre-les-occurrences.md).
Parent en-tête corrigé : raccord neufBM25 livréb0, suites legacy/pyramide encore
ouvertes. Contrat03 lots2a/2b relu, aucune nouvelle politique de compaction.

## Déjà livré / manque constaté

publication.rs fournit PublishedDeclaration origine/P opaques et H de l’état
déclaré, sans certificat physique. physical_index.rs fournit QualifiedPhysicalState
origine/P/descripteur et snapshot natif neuf validé. L’identité des dérivées
Content existe et conserve H séparé des occurrences ; occurrence projection
(search.rs) et vue ReadView/L6 existent. Aucun de ces mécanismes ne porte
aujourd’hui la pile de parents épinglés ou les tombstones scope/fichier.

## Diff minimal proposé

publication::layers réutilise les P existantes : LayerGeneration référence
(layer,origine,P), construite depuis PublishedDeclaration ou, sous feature
native, QualifiedPhysicalState. Pas de nouveau générateurP, pas de clone DB.
Une référence de déclaration reste une référence : elle ne prouve ni valeurs
physiques, ni présence de couche persistée, ni accès aux index.

LayerRevision contient parent EXACT, occurrences locales et tombstones.
OccurrenceKey=(entité,uuid) et FileKey logique viennent de la projection,
jamais de H ou d’un chemin disque résolu implicitement. Payload T opaque
conserve les champs publics/contentH/positions par emprunt sans recalcul ni
remappage. La primitive n’invente pas le contenu ou les champs manquants.

PinnedLayers valide l’ensemble immuable complet avant résolution : doublons
générations/occurrences, parents absents, cycles, parent d’autre origine,
tombstone occurrence et ajout du même identifiant dans une révision refusés.
Résolution itérative racine→fille : tombstones retirent les ancêtres concernés,
puis deltas locaux prévalent. Tombstone fichier+valeurs locales exprime un
remplacement explicite ; descendant peut réintroduire un fichier sans
ressusciter ses anciennes occurrences. Provenance origine/P/couche pour
chaque occurrence gagnante, mêmeH ne fusionne jamais les occurrences.

Sélection nom seul refuse plusieurs P, pas de notion implicite de « dernière ».
Résolution par clé exacte ne consulte aucune tête mutable ; résultat emprunte
le même ensemble gelé. Une ancre avancée n’altère pas une parente épinglée.

## Témoins écrits avant implémentation, NON joués

Quatre fonctions CPU couvrent fparente/gfille/hancre, mêmeH/deuxoccurrences
avec coordonnées distinctes/provenance ; tombstone scope/fichier et
réintroduction descendante ; override et ancre avancée mêmeétatH/Pdifférent ;
inconnue/ambiguë, doublons/cycles/parentabsent/origine étrangère/identitésvides.
Aucun témoin rouge exécuté revendiqué. rustfmt a uniquement analysé/formatté
le nouveau module et ses tests ; aucun Cargo/rustc/test/DB/service lancé.

## Limites et recette à faire autoriser

Ce diff teste le contrat pur2a, PAS sa persistance, ni l’intégration du
snapshot natif/L6, ni masques avant top-k, filtrage arêtes des deux bouts,
paramètre view des outils ou accessibilité IDE. Cela reste2b/étapes suivantes.
Aucun codec ni migration de format introduit, aucun cache/GC/modèle/service.
Pas de claim que la pile est déjà branchée dans un parcours public.

Après revue exacte : proposer recette CPU bornée sous confiner_validation et
poste réel luciepc, Max32GiB/CPU4rôles/jobs8, target privé session recréé au
chemin cx-target-contenu-derive, inventaire/ELF sourceSHA et premier rouge stop.
Filtre exact publication::layers::tests:: attendu4fonctions/0ignoré ; jeu
sans-default-features pour résolution CPU, puis contrôle de compilation sous
feature native pour adapter typé et qualification native séparée si requise.
Durée et GO exact appartiennent à coordination, ED se qualifie actuellement.
Aucune commande Cargo n’est autorisée par ce document seul.
