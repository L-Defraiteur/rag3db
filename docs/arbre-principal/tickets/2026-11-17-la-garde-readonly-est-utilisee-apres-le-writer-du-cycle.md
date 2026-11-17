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

Deux RO preuves reçues : Recherche favorable bornée et ED4cb7cbd277, sans
innocuité ni qualification logique. [Proposition07](../17-11-2026/07-proposition-contrat-postwriter.md)
prépare le contrat physique distinct, les préconditions writer attestées et
les mutants du vrai raccord. Contrat à relire avant écriture du harnais ;
témoins NON ÉCRITS/NON JOUÉS, aucune garde modifiée. La cause directe du refus
Qv8 est le retrait .wal/.ecrivain ; le changement SHA DB est exclu de CE juge.
Pas de sources-after/postquiet capturés : FIN après rouge non qualifiée.

RO ED contrat3c9 favorable, [rapport38](../../everything-declarative/16-11-2026/38-ro-contrat-postwriter-3c9.md).
stable_readers intact, gardePOST distincte conditionnée aux vraies attestations,
WAL/marque absents après shutdown et autresinvariants stricts ; aucune innocuité
DB ni qualification logique déduite. Pour harnais : baselinepréwriter exclusif/
rehash liéaucycle+mutantsubstitution ; typesACK/rc stricts ; compteur vraiebranche
POST puis lecteurs doublés ; refuspréalable nonqualifiant ; rougecausal capturé
puisFAIL, pasERROR ; mutants fauxfixskip/tousretraits/toutDBréussi refusés.
TémoinsNONÉCRITS/NONJOUÉS par ED, aucun fix/GOCPU/natif/DB/health autorisé.
Prépublication puis RO/GO C7 distincts, Qv8rouge et ses limites préservés.

RO ED subsetinventaires8a29 favorable statique/bornée : sixfonctions vrai
blocdriverAST/save/readonly,ValueError→ASSERT,dossierspreuvesfreshparcas,
exécutableinchangé et NONJOUÉ ici. Pas de qualification d'ensemble : négatifs
peuvent refuser en amont. Le doublon causal ancien conserveWAL/marque ; ajouter
pourfuturvert doublon sur POST autrementadmissible sanscesauxiliaires, motif/
branchevérifiés. Contrôle readonly positif+compteurs+preuvesACK/PID/nonce/
clock/firstred encoreàécrire avantfix, commeannoncé ; suite rapportED38.

RO ED ensemblede809 bornée :12fonctions écrites NONJOUÉES, exécutableinchangé,
neutrepositif+preuvesJSON/deuxframes réelles/profils synthétiques+vraibloc/validateurs
cohérents pour préparer le rougecausal exact. Réserves suffisance à grouper :
preuve du baseline persisté/hashé AVANTwriter noncouverte (bloccommenceafter_writer),
mutantsubstitutionbaseline ; doublonPOST autrementadmissible et motif/compteur
atteint ; self.dirs doit inclure uid du vrai q.directories. AssertFalse/refus
amont ne qualifie pas le mutant visé. Contrôle readonly intact/typesbool-int
à compléter avant qualification globale. Aucun lancement/fix/GO par ED.

RO ED complément48a2 : oracleordre préwriter réel (assignment/save2inventaires/
writer sourceordre, spyvérifieprésence-contenu-inodefsyncAVANTappel) favorable
statiquement ; positif.readers ajouté.13inventaire+1préwriter NONJOUÉES,
runtime6c6b inchangé. Rouge causal à cibler EXACT nominalauxiliaires, pasmodule
failfastpourprétendrecettecausalité. Restentpourvert : baseline substitution/
liencycle,doublonPOSTnonmasqué,motif-compteurs,typesstricts/dirsUID. Aucun test
oufix/GO accordé par ED ; complément rapportED38, C7seuladmissionCPUdistincte.

RO ED code+harnais e9f037be2a favorable statique, [rapport39](../../everything-declarative/16-11-2026/39-ro-postwriter-et-cold-e9f.md) :
POST0e4c distinct/readonlybyteinchangé, deuxsnapshots+contextefsyncpréwriter,
preuvesrejouéesSession/identité/profil/maps/typeACK-rc/nonce/pubSHA puisdeltas
physiquesexactssansqualificationlogique. Réserves ED baseline/bool/UID/
doublonneutre/compteur traitées. COLD utilisevrai stable_readers surPOSTnouveau
avecDBincluse,sources/dirsstrictes.19fonctions(15POST+1PRE+3COLD)NONJOUÉES,
rouges48a2/336c3+3fichiersréhashdeuxhôtes0écart ;pasdevert/paquetQ9validé.
Décision C7 CPU19distincte,puisROsource/paquet avantnatif ;aucuneDB/provider/
writer/recovery/runtime/ancienGO utilisépar ED,ticketOUVERT et65ebrougeconservé.
