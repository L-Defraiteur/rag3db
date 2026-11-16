# Garde de mise à froid : management G et validation séparés

Ticketparent : docs/arbre-principal/tickets/2026-11-16-basculer-les-trois-bases-vers-les-generations.md. Après rouge337 AVANTwriter, diagnostic distinct fd2fc73277/service-guard-diagnostic-01 sousvalidation-x1zr7yq4 : rc1, stdoutvide, stderr « The name org.freedesktop.systemd1 was not provided by any .service files ». Busprivérestéinchangé ; cela ne prouvepas unitéactive. G horsvalidation confirmeunitésinactives/MainPID0/7970sanslistener parattestation8f3a131fbaaa066a3ea34a9f2b4254bdc961915981a7de120d96323c1282f2a6. Aucunwriter/ciblecopiée ; snapshot/canonical inchangés audités.

C7 valide séparation : G management horsCommon lit busréel soussonrôle ; filsbornéresteBUSPRIVÉ, aucunDBUS/XDG_RUNTIME réinjecté. Attestation G fraîche aprèsentréeCommon etnoncephase, TTL<=30s ÀPopen, PLUSpreuvesOSlocales/baseferméeSHAstable/CGauthority, jamais JSONseulcommeverrou. NouveaucréneauCORRECTIFdistinct àpublier/proposer (ancien1200ROUGEterminé, aucunresetimplicite), aucunLIVE.

## Requête exacte du fils, par phase versions puis generations

Répertoireprévu PC `~/.cache/rag3weaver-build/bascule-live-20261116/migration-rehearsal-service-corrective-02/code-principal/guard`. FichiersEXCLUSIFS `<phase>-request.json`, `<phase>-response.json`, `<phase>-response.sha256`. ChaqueécritureJSON .open(x), G publie response complètement avantson .sha256 final (64hex+newline). Aucuneanticipation/réponsed'avance/nonceinventé.

Requête schema_version=1 ; stage="service-guard-corrective-02" ; nonce=32hex aléatoires ; phase="versions" ou "generations" ; entered_monotonic=entrée réelle du fils aprèsreceiptCommon ; requested_monotonic=écriture ; runner={pid,start,cgroup} (starttime/procchaine) ; common_receipt={path,sha256,pid,start,cgroup,deadline} (SANSjeton) ; canonical_base, target_base, source_snapshot (cheminsabsolus) ; expected_host={pid:3761669,exe_path:"/home/lucied/.local/bin/rag3weaver-backend",exe_sha256:"0e310a9cfe1c9e12534f3a579b3e781dcef7e9335e37d7e57349b16d053d5288",source_sha:"927941a67a5805450d49f1e04b39fdd542a6059b"} ; reply_path/reply_sha_path absolus sousrépertoireguard.

G vérifie vraiPID/start/cgroupdufils etsuperviseurCommunvivants/receiptactif/nonexpiré, noncephaseissueAPRÈSreceipt ; gardeAPI/socketoff, aucuneopunité/aucunHTTP. ProviderGtemporaire borné, conservévivant jusqu'àconsommationdes2phases/finreceipt pour corrélationobserver, arrêtGàfinbudget ; jamaisdansquiescenceCommon.

## Réponse exacte G, observée sur le même PC

schema_version=1 ; stage,nonce,phase,entered_monotonic,runner égauxrequête ; request_sha256=hashoctetsrequête ; common_receipt_sha256=requête.common_receipt.sha256 ; observed_monotonic=finobservationréelle≥requested/entered ; observer={pid,start,cgroup,uid:1000} ; verdict="VERT" seulement si toutesconditions.

units={"rag3weaver-daemon.service":{LoadState:"loaded",ActiveState:"inactive",SubState:"dead",MainPID:"0"},"rag3weaver-daemon.socket":{LoadState:"loaded",ActiveState:"inactive",SubState:"dead"}} ; previous_host={pid:3761669,absent:true} ; executable={path,sha256,source_sha} égauxexpected_host ; port_7970={rc:0,listeners:[]} ; base_refs={fd:[],maps:[]} pourcanonicalETtarget ; protected_processes=[{pid,start,uid,comm},...] exclusivementlesprotégésnoninspectablesconstatés (pasabsenceFDinférée) ; limits documenteportée/UIDs/proc ; raw_files=[{path,sha256},...] brutsunités/scan/port etpins. FichierG aveccli_authorized absentoufalse, neforgeraucunGOLIVE.

LeconsommateurrevalideSHArequête/réponse/sourceGlocal, unités/PID0/ELF/source, timestampsmonotones/TTL etidentitéobservervivant. Puis OSlocal : ancienPIDabsent ; aucunhôte `rag3weaver-backend daemon` correspondant7970 ; port7970nonLISTEN ; scansbornésFD/maps surbases canonical/target (UID1000, autresUIDexplicitementlistéscommehorsportée), erreursinaccessiblesrefusées saufprotégésPID/start/comm attestésG, UNKNOWNconsignés etpasabsenceglobaleprétendue. Sourceoriginal+snapshot ettargetfp inchangés entreprérequêteetPopen ; autoritéCommunréelle/cgroup<=32G revalidée. CheckTTLfinalAPRÈShashes. CLIexclusivitéOSresteautoritaire, touteerreur/refusSTOPsansnouveauréessaiautomatique.
