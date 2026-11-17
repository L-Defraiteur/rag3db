# La garde readonly est utilisée après le writer du cycle

État : OUVERT. Porteur : arbre-principal. Consigné avant correctif.

Lié au [ticket cycle17](../../orchestration/tickets/2026-11-17-le-cycle-demarre-avant-l-armement-effectif-du-producteur-de-grant.md)
et au [rapport06 des preuves immuables65eb](../17-11-2026/06-cycle-q8-rouge-apres-shutdown.md).
Parent dette469 et programme trois bases restent ouverts ; aucun GO LIVE.

## Fait observé et limites

GO C7-LIFECYCLE-HB6C6-Q8-17nov900 clos ROUGE sur source6c6b/Qv8a5d861.
ARM/admission/grant autonomes puis réponse publique camelCase et shutdownACK
mêmePID913237/start126604879/rc0 présents. Premier rouge1266055.091886152 :
audit_result.stable_readers appelé par lifecycle.driver après le writer.

La garde readonly exige la conservation des fichiers existants, avec seulement
l'ajout éventuel d'un .readers vide. Inventaires persistés2252→2250 : retraits
de code-principal.rag3db.ecrivain (254octets) et .wal (1589305octets), changement
du SHA DB à taille/mode constants ; aucun autre changement ni ajout.
La comparaison post-writer exclut la DB principale mais refuse les retraits.
L'inventaire before_writer est en RAM et n'a pas été persisté séparément :
cette limite est conservée, aucun nouveau témoin historique reconstruit.

Suites POST vecteurs bits/graph/FTS/finished/driver-result NON JOUÉES après le
premier rouge. Les écarts ne sont pas déclarés innocents ; aucune intégrité
logique, durabilité, réussite du cycle ou qualification LIVE déduite.
103 bruts conservés sur deux hôtes avant diagnostic, manifeste497baaf...2d5d.

## Travail admis et critères à définir

C7 demande un contrat post-writer distinct du readonly, puis des témoins du
vrai raccord et des mutants. Cette préparation attend les deux RO preuves.
Ne pas autoriser génériquement les disparitions de fichiers ni modifier
stable_readers pour faire passer le cycle. Définir les préconditions liées au
writer réellement attesté, les deltas physiques explicitement attendus et
les invariants logiques à juger séparément avant toute proposition de fix.
Un futur inventaire avant-writer doit être persisté avant l'action concernée.

Aucune nouvelle DB/health/writer/recovery/service/retour natif autorisée.
Une proposition de qualification RO éventuelle nécessite source/paquet neufs,
revues et GO exact distinct ; les anciens rouges et leur paquet restent figés.
