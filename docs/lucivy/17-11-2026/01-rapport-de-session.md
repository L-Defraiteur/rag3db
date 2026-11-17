# Lucivy — 17 novembre 2026

État : préparation du rouge FS, aucun Cargo ni corpus contacté.

Suite du [contrat du 16](../16-11-2026/16-qualification-native-et-contrat-fs.md),
arbitrage C7 : capture initiale unique StdFs, ordre effectif, cap5000 et
exclusions versionnés, CapturePlan explicite et snapshot RAM. Pas de recapture
après REFUSED. Rejeu ultérieur par manifeste, mêmes chemins/SHA sans resélection.

## Reprendre

- Code : `/home/lucied/git_workspaces/cx-lucivy-fs-admission`, branche
  `cx-lucivy-fs-admission`, poussé `8b9435363dbd8ec0e3973f2368b641189f0726eb`.
- Documentation : `/home/lucied/git_workspaces/cx-lucivy-plan-shutdown`, branche
  `cx-lucivy-reprise-16nov`.
- [Préparation, limites et recette proposée](02-preparation-rouge-fs.md),
  [pins source/lock](03-fs-red-source-pins.json).
- [Ticket porteur](../tickets/2026-11-16-l-admission-du-corpus-fs-ne-garantit-pas-la-vue-consommee.md)
  ouvert ; revue statique demandée à Recherche puis arbitrage C7. Aucun GO Cargo.

La source 1ee5 reste qualifiée pour neuf juges mémoire seulement ; cette nouvelle
source n'est ni compilée ni qualifiée. Natifs A/B, core complet et ses neuf HOLD,
ULP Linux7.2 et PR42 restent HOLD. Aucun main/tag/publication/fusion.

RO Recherche reçue sur source exacte8b94353 et docs c987957426 : favorable
bornée à la baseline et à la recette proposée ; aucun blocage statique établi.
Compilation non vérifiée. Suite : pilote concret avec delta lock offline
archivé, puis RO et GO C7 distinct avant toute exécution.

Paquet concret préparé, sans exécution : [recette exacte](04-paquet-rouge-fs.md).
Q PC fs-admission-red-17nov-v1, W PC fs-admission-17nov/source8b94353 ;
préflight79adf115e7096644c32cf6be31286d89e03ce54c160fe5da538c933afeac15db.
Exporteur portable autonome armé avant lancement, aucune dépendance humaine
pour ACK ; timeout physique300 incluant acquisition/retour poste, END unique
CommonMIN/final. Neuf copies vérifiées et syntaxe AST uniquement, aucun
Cargo/ELF/proc/corpus. RO couple exact puis GO distinct restent requis.

RO Banc du couple6a16c7220f favorable PRÉPA SEULE : c3cba8417c, rapport
`docs/banc/17-11-2026/03-ro-couple-fs-red.md`. Copies/AST/bornes/barrière
autonome et attribution des juges cohérentes à lecture. Aucun run/GO déduit.

RO Recherche statique du couple6a16c7220f/source8b reçue favorable bornée :
admit166d/pilota5c/export047 et recette/préflight79ad relus. Arm avant launch,
physicalEND arm+300 incluant poste, Commonreste−12/finalpostCommon, ledger
copié/rehashé/ACK avant inventory, six causes/traces/compteurs stricts puis
positif, infraSTOP cohérents à lecture. DeltaLock strict et backups conservés.
Aucun Cargo/test/proc/corpus/contact/SSH par Recherche ; copies PC non rehashées
indépendamment par elle (Banc les a vérifiées). Aucun GO/runtime ni qualification
natif0=0/FSfix/readonce/sink/fullcore. Les deux RO du couple sont reçues.

HOLD C7 452dd0ccfc : absence de watchdog physique portable dans export047111.
Ancien paquet figé/nonconsommé, commande04 HOLD. [Contre-exemple CLI proposé
avant correction](06-hold-borne-physique-portable.md),300s réels/fauxSSH/fsync
bloqué/secours302. Aucune exécution du témoin ni correctif encore effectué.
