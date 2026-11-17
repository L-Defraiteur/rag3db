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

## Sous-ensemble inventaires8a29c5c293

RO statique favorable à ce périmètre de préparation : six fonctions écrites,
vrai bloc AST lifecycle.driver after_writer→avant quiet, vrais save et
stable_readers, inventaires/répertoires seuls doublés. ValueError capturé
puis assertion ; dossier d'evidence neuf par gate/sous-cas depuis8a29, pas
de FileExistsError pris pour un refus de garde. Exécutable inchangé, aucun
témoin joué par ED.

Les deux cas causaux possibles sur l'ancien raccord sont l'admission nominale
POST attendue mais refusée readonly et le doublon silencieusement collapsé.
Les négatifs peuvent refuser en amont : leurs verts éventuels ne qualifient
pas encore le bon mutant. Point concret pour la suite : le cas doublon
actuel garde WAL/.ecrivain pour exposer l'ancien collapse ; après correction,
leur présence pourra elle-même refuser. Conserver ce rouge historique et
ajouter un cas doublon sur baseline POST autrement admissible (deux auxiliaires
retirés) avec branche/motif attendu vérifié. Ce dernier seul établira que
la duplication est vraiment jugée, sans se satisfaire du refus WAL.

Le contrôle readonly positif intact, les compteurs de passage et les mutants
ACK/PID/start/nonce/run/publication/profil/maps/clock/firstred restent à écrire
AVANT le fix comme annoncé. Ce sous-ensemble n'est pas une suffisance du
harnais global, encore moins une qualification native. À C7 seul le choix
des témoins CPU exacts après prépublication/RO ; aucun lancement ici.

## Ensemblede809c288a avant fix : avis borné

Les12 fonctions restent NON JOUÉES, exécutable6c6b inchangé. Le harnais
fabrique JSON writer/publication/ACK/deux frames/profil, appelle public_call
réel pour le fixture, puis le VRAI blocdriver/save/readonly. Dossier fresh
par sous-cas et capture ValueError→ASSERT conservés. L'inventaire neutre
positif est utile : les mutations ACK/rc/PID/start/nonce/run/exeSHA/CG/
pubSHA/maps/diff et END/firstred ne se cachent plus derrière les deux retraits
physiques. Le nom causal demandé cible bien l'admission POST readonly refusée.

Avis favorable à la préparation de CE rouge causal exact ; **réserve sur la
suffisance de l'ensemble** avant qualification verte, à grouper avant fix :

- L'extraction commence à after_writer. Elle ne prouve pas encore que le
  futur inventaire complet est persisté/relu/hashé AVANT l'appel writer ;
  ajouter un témoin du vrai raccord préwriter et le mutant baseline substitué,
  faute de quoi une persistance après l'action échapperait à ces12 fonctions.
- Le doublon conserve les deux auxiliaires ; au futur vert leur présence
  pourrait suffire au refus. Ajouter le doublon POST autrement admissible,
  vérifier motif/compteur du validateur visé. Plus généralement assertFalse
  seul ne prouve pas que la branche attendue a été atteinte.
- q.directories réel produit path/mode/uid ; self.dirs actuel n'a pas uid.
  Aligner cette donnée synthétique sur le format réel avant un validateur
  strict, pour ne pas produire un refus d'entrée qui masque tous les mutants.

Conserver le rouge ancien et ses limites. Compléter aussi les types exacts
(rc=False ne vaut pas entier0), le contrôle readonly intact et les traces de
passage attendues ; pas de vert d'ensemble à partir de refus en amont. Le
choix CPU est exclusivement C7, les12 n'ont pas été exécutées par ED ; aucun
fix, DB/health/native/service/SDK/target ni ancien GO reporté.

## Complément48a2cfcdc1 : quatorze fonctions prépubliées

RO statique favorable au témoin préwriter ajouté : extrait les vraies
instructions assignmentbefore_writer/save deuxinventaires/appelwriter dans
l'ordre source ; le writer est un spy, toutes les autres actions du driver
sont explicitement exclues. À l'instant de cet appel, contrôle présence,
contenu relu et inode observé dans le vrai fsync des fichiers temporaires.
La réserve sur l'ordre baseline→writer a maintenant un oracle écrit, sans
résultat CPU déduit. L'ancien code sans sauvegarde doit faire FAIL sur ces
observations, pas invoquer une DB pour en apporter la preuve.

Le positif .readers absent→vide a aussi été ajouté. Total13fonctions
inventaire +1préwriter, toutes NONJOUÉES ici, exécutable6c6b inchangé. Pour
le seul rouge causal sollicité, cibler EXACTEMENT
test_postwriter_inventory.PostwriterInventory.test_exact_writer_auxiliary_removal_is_not_a_readonly_mutation ;
un module entier failfast pourrait s'arrêter sur un autre défaut attendu.

Les points restant avant suffisance verte restent ceux ci-dessus : mutant
baseline substitué/lien au cycle, doublon POST autrement admissible et motif/
compteur, types exacts et UID des répertoires synthétiques. Le nouvel oracle
d'ordre ne qualifie pas à lui seul les autres14branches. Favorable pour la
préparation et ce ciblage CPU si C7 l'admet ; aucun GO donné par ED, aucun
test/fix/runtime/native/DB/health/service effectué.
