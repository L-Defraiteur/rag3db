# RO contrat POST-writer3c9

Avis **favorable au contrat documentaire** exact3c9fb1c86e, proposition07,
pour préparer les témoins sous le ticket distinct
[garde post-writer](../../arbre-principal/tickets/2026-11-17-la-garde-readonly-est-utilisee-apres-le-writer-du-cycle.md).
Pas de correction implémentée, aucun témoin écrit/joué par ED ni GO transféré.
L'exécution Qv8/65eb demeure ROUGE et immutable.

La frontière est cohérente : stable_readers garde son rôle readonly ; une
garde POST-writer distincte exige les preuves du writer réellement observé.
Le delta DB consigné n'est pas une intégrité logique démontrée. Seuls WAL et
.ecrivain exacts peuvent disparaître, ils doivent être absents après shutdown,
même si le WAL résiduel serait byteidentique. Tous autres fichiers/dirs stricts,
exception .readers bornée et doublons/types/entrées ambiguës refusés.
Quiescence privée puis POST logiques dans le même END, premier écart bloquant,
finaliseur inchangé : pas de reconstruction du before_writer historique.

## Calage recommandé du vrai harnais

1. Persister le futur baseline complet AVANT writer dans une entrée exclusive,
   le relire/hasher et lier son SHA à la base privée canonique et au dossier
   de preuves du cycle. Un baseline modifié ou substitué doit refuser :
   ajouter ce mutant aux suppressions/altérations déjà prévues. Aucun accès
   DB n'est nécessaire pour juger ce lien sur des inventaires synthétiques.
2. Préconditions positives complètes pour chaque mutant : ACK booléen true,
   rc entier0 (False ne vaut pas0), PID/start/exe/CG/profil/maps conformes,
   nonce/run/publicationSHA liés ; inverser un seul élément à la fois.
   Les valeurs manquantes/mal typées ne deviennent pas des permissions.
3. Sur le VRAI raccord lifecycle.driver, compter explicitement le passage
   jusqu'à la garde POST et jusqu'aux lectures POST (doublées). Un refus
   antérieur ne qualifie pas le mutant tardif ; vérifier aussi que le retrait
   exact des deux auxiliaires permet d'atteindre la suite après correction.
4. Le rouge causal initial est l'admission POST attendue des deux retraits
   exacts, encore refusée par stable_readers courant. Capturer ce refus puis
   faire FAIL par assertion, sans ERROR ni faux résultat natif. Les autres
   mutants prépubliés ne deviennent qualifiés qu'après leur branche atteinte.
5. Essayer les faux correctifs demandés : garde sautée, tous retraits ignorés,
   tous changements DB assimilés à succès logique. Ils doivent échouer même
   si les attestations writer sont positives. Vérifier END/first-red pendant
   collecte ET publication, pas seulement à l'entrée du helper.

La matrice physique ne remplace aucun juge vecteurs/claims/graph/FTS/dette0,
ni les contrôles source/coldreads et quiescence après writer. À un mutant
physique refusé, aucune lecture POST ; à des invariants logiques refusés,
aucun VERT cycle malgré une admission physique positive. Ces chemins doivent
être distincts dans les assertions et les compteurs du harnais.

Aucune nouvelle réserve bloquante établie sur la proposition07. Prépublier
le vrai harnais avant correction ; RO et GO CPU exact distinct à C7 avant
exécution. Source/paquet neuf et deux RO totales avant tout éventuel natif.
Ni RO native de Qv8, ni recovery/health/service/writer/SDK/Cargo/target/retrait
autorisé par cet avis ; toutes les anciennes limites/proofs restent conservées.
