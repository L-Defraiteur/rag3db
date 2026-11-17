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

Premier harnais inventaires prépublié : test_postwriter_inventory, six
fonctions au vrai bloc AST lifecycle.driver entre capture after_writer et
première quiescence post-shutdown. Inventaires/dirs synthétiques seulement,
refus ValueError capturé puis jugé explicitement. Cas nominal deux retraits,
mutants FTS/retraits/octets/modes/WAL/ajout/répertoire et doublon de chemin.
NON JOUÉ ; les négatifs peuvent être refusés par l'ancienne mauvaise garde
avant la branche visée, donc aucun succès de ces seuls cas ne la qualifie.
Préconditions preuves writer/horloge et autres mutants de proposition07
restent À ÉCRIRE avant tout correctif. Aucun exécutable modifié.

Complément prépublié avant tout correctif : douze fonctions désormais,
preuves synthétiques complètes et contrôle positif d'inventaire neutre.
Les discordances ACK/rc/PID/start/nonce/run/exeSHA/cgroup/publication/profil/
maps et END/premier rouge sont injectées sur inventaire neutre, sans retrait
amont masquant. DB absente/mode modifié, marque résiduelle et .readers bornés
ajoutés. Tous NON JOUÉS, aucune garde modifiée. Les champs/directories sont
des fixtures, pas attestations OS. RO harnais complet avant GO CPU distinct.
Le rouge causal readonly doit cibler la seule fonction nominale exacte,
pas le module failfast qui peut rencontrer le doublon en premier.

Complément RO Recherche de809 avant fix : positif .readers initialement
absent/vide ajouté et juge test_prewriter_persistence au vrai ordre AST
assignment before_writer/save inventaires/appel writer (spy, autres actions
exclues explicitement). Le spy exige fichiers/dirs exacts et fsync terminé
avant son appel, sans aucun proc ni writer réel. Quatorze fonctions écrites
au total dans ces deux modules, toutes NON JOUÉES. Une preuve manquante devra
être convertie en refus ValueError explicite dans le futur contrat, sinon
ERROR et STOP, jamais un vert. Aucun exécutable modifié.

## Rouge causal puis correctif local prépublié

GO C7-POSTWRITER-CAUSAL-RED-48A2-17nov60 consommé sur48a2 propre : nominal
exact seul, 1 FAIL readonlycopychanged capturé→assert, 0 ERROR/skip,0.008s,
rc1/0.144241s. Treize autres NON JOUÉS. Bruts/reçu/manifeste hors WT
postwriter-causal-red-48a2-20261117-01 sur deux hôtes, trois size/SHA conformes ;
manifeste046c89cf87dbc61fd319f01ba024f47b48dc198a3f503cc29e53c6be2e5c1d64.

Correctif local lifecycle.postwriter_inventory prépublié après ce rouge :
deux inventaires fsync avant writer et contexte liant base/pilot/prepared/
nonce/END/cgroup/profil et SHA inventaires. Garde post-writer exige ce lien,
les preuves mêmePID/publication/shutdownACK/rc0/profil/maps et valide les deux
frames brutes par Session existante. Contrat physique distinct : DB conservée
et mode stable, WAL/marque absents, autres fichiers stricts sauf .readers0
initialement absent, répertoires/UID stricts, doublons/types invalides refusés.
END/premier rouge avant/après collecte et admission. Stable_readers readonly
inchangé. Admission physique ne qualifie jamais l'intégrité logique.

Réserves ED686ba30827 traitées avant demande de vert : baseline/context
substitués, bool/int ACK/rc, vrai UID fixture, doublon sur inventaire NEUTRE
sans WAL/marque, contrôle readonly inchangé. Le harnais impose un appel du
vrai postwriter_inventory : sauter la garde ne suffit pas à rendre les juges
verts. Seize fonctions désormais (15 postwriter +1 prewriter), NON JOUÉES
après correctif. P/Q/S+manifest v9, Qv8/65eb/ancien budget clos conservés.
Nouvelles RO et GO CPU distinct nécessaires avant toute exécution ; aucun
GO auditRO/900/natif/DB/provider/writer/recovery acquis par ce correctif.

## P2 raccord readonly après lectures froides — avant correction

RO Recherche0e4c favorable bloc post-writer, mais le raccord final compare
encore after_reads!=after_writer strictement. Un .readers initialement absent
et vide serait refusé, contrairement au contrat readonly inchangé prévu au
nouvel étage. Déjà présent dans code ancien, pas un résultat natif nouveau.
Trois témoins du vrai bloc AST after_reads→avant sources-after prépubliés,
mutants .readers non vide et DB bytes modifiés avec positif .readers vide.
NON JOUÉS ; aucun correctif P2 ni transfert GO depuis les16autres.
GO CPU causal exact distinct à demander avant test, puis revue/correctif/vert.

GO C7-POSTWRITER-COLD-RED-336C-17nov60 consommé sur336c propre : nominal COLD
seul, 1 FAIL `sources or cold read-only fixture mutated` capturé→assert,
0 ERROR/skip,0.007s,rc1/0.110809s ; dix-huit autres NON JOUÉS.
Archive postwriter-cold-red-336c-20261117-01 sur deux hôtes, trois size/SHA
conformes, manifeste0ece970a239988394f7ebe05952b0a8b066c957e8ce13ab88271026f17f4488f.
Puis correctif raccord prépublié : sources/répertoires restent stricts et
stable_readers EXISTANT juge after_writer→after_reads, sans exclure DB.
Pas de modification du validateur readonly lui-même ; Qv8/65eb restent figés.
Ensemble19 NON JOUÉ après ce correctif : nouvelle RO/GO CPU vert distinct,
aucune autorisation native/DB/provider/ancien budget déduite.
