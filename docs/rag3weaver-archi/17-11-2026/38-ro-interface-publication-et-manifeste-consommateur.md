# Relecture de l'interface commune et manifeste consommateur proposé

Interface fournisseur EXACT2c8d1df84ae1ea0af6b2da58892bc3a57c7d89a0,
branchecx-codeparsers-publication-contract. Lus intégralement
`publication-contract/01-point-extraction-commun.md` SHAa3c6a1106e7263888441afec85b4ea81c207fe9db3bc1263dec2016ef6a730a1
et `publication_contract.pyi` SHA211e7b90d18d550b4b3e26d32749b0c415b18102914d93802b8b2d4cbf63a644.
Avis Archi favorable au contrat de composition pour préparation, pas au runtime
NONIMPLANTÉ. C7 doit relire le contrat et les limites ensemble avant extraction
ou adaptateur exécutable. D45/P05 et229b/Q sont figés ; aucune de leurs RO ne
s'applique automatiquement au noyau futur.

## Ce que l'interface exprime correctement

Plan ordonné/versionné/hashé AVANT lancement ; les rôles et huit identités ELF
sont attendus, les SHA futurs observés apparaissent seulement après production.
StageHeader lie execution/phase/index/label/prepared/plan/binding et événement
fenêtre. PublicationEvidence distingue SHA des OCTETS ledger, valeurJSON, files
exactes/tailles et ELFmap. Références à ELF déjàACK admises sans fusionner les
occurrences, utile au Rust build/runtime ; globalD6 n'a besoin que CurrentFile.
Provider est lecture/validation métier ; le noyau reste propriétaire des vrais
rôles/autorité/clock/firstred/export/ACK/fsync/link/cancel/reap/quiet. Le wrapper
ne doit pas garder sa copie de run/publish/abort. Tous modules sont pinés, pas
seulement le wrapper ou un fichierready. Pas de tar/fauxinventaire pour entrer
artificiellement dans l'ancienne API2stages.

La séparation operation_remaining/phase_remaining/publication_remaining répond
au besoin : après la fenêtre global90, une future fenêtre D6 peut commencer
sans conserver l'ancienENDpublication ni remonter le GLOBALEND. L'événement
est celui du propriétaire réel, pas l'apparition vue tardivement par le pair.
Transport.ready/startphase puis bootstrap/exec réels avant préflight est une
précondition indépendante d'un ledger déjà copié. Les duréesPC sont ancrées
AVANTchaqueRPC portable ; pas de comparaison monotonic absolue interhôtes.

## Les huit identités et quatre étapes proposées

[Manifeste39](39-manifeste-publications-global-d6-propose.json) est un DOCUMENT
NONEXÉCUTABLE avec run_authorized=false, execution_id/futurepilot inconnus.
Il n'est PAS validate_plan accepté ni source d'une admission. Il fournit les
StageSpecs et contrats statiques dont le futur prepare fera le plan complet,
avec nouveaupin, closuremodules, identités et horloges réelles à relire.

| Label ordonné | Files exactes courantes | Origine ELF / défauts / closurefeatures |
|---|---|---|
|global|ledger,tty,host,large,e2e,nav|TTYbinCargo2features/defaultvue, hostbin13, liblarge13, e2etest13, libNAV14/defaultactif|
|agent-red1|ledger,agent-red1|vrailibtestdanssourceTÂCHEAPP+jugeexactactif, no-default,3features|
|agent-green1|ledger,agent-green1|vrailibtestmêmecorpusREDhorsseulcorpsvalidateur, no-default,3features|
|review1|ledger,review1|sourceagentGREENacquise+SEULappendREVIEWpublié, no-default,3features|

Cinqmanifestesprincipaux visent Cargo.toml APP ; TTYvise tty/Cargo.toml. Leurs
SHA et closures de features locales sont calculés en lecture TOML/graphes,
sansCargo/importdupilote/exécutionELF. SDK59 attendu pour la compilation et les
profils hérités ; TTY n'exige pas de mapping natif, les sept autres ELF oui.
AucunSHA/size futur de fichierELF n'est rempli depuis une fixture.

Les labels fixes D6 ne sont pas les filenames dynamiques `task-N-start` actuels.
Le futur provider doit résoudre UNIQUEMENT l'admission Broker scellée de stage
agent-red/agent-green/review vers son identity et son vrai ledger/ELF. Contrôler
exactement1/1/1invocation, ordre et source_state/judgeidentity déjàqualifiésCPU.
Ni glob,mtime,premierfichierprésent ni chemin fourni librement par le pair.
Le binding_sha est celui du CONTRAT statique source/profil/mutationspermises,
pas le hash futur d'un patch choisi à partir de l'actual. Ce dernier appartient
au ledgerobservé, est validé contre le contrat puis figé aux frontières.

## Fenêtres, ressources et arrêt : raccord encore à écrire

Le futur Runner.published calcule sa fenêtre AVANT attente/publication,
MIN(nowPC+90,CommonEND−réservearrêt), publie un événement lié au vrai PID/start/
script/config/autorité/label/index/ledgerSHA complet. Le Gate commun revalide
le propriétaire et cet événement, fige les durées restantes, puis exporte.
Il ne déclenche pas90s à la réception d'un messagehumain ni ne rallongeGLOBAL.
La boucle existe une fois dans le noyau extrait. Le provider ne peut pas écrire
le marqueur natif ou accorder un ACK depuis une simple déclarationactive.

Les valeurs Rust2098/300/5 ne sont pas importées comme budget du global.
Common7200MAX/TERM7440+KILL15 restent la proposition globale relue ; la limite
locale/remote de queue et réservearrêt doit être arrêtée par C7 ensemble,
DANS cette enveloppe. Limiterlesappels et annuler doit rester possible après
expiration opérationnelle avec réserve déjà retenue, pas cap()op expiré.
Une observation finale vide ne peut jamais provenir d'une boucle non entrée.

GrantWatcherEmbar96f3 demeure le producteur des ressources, séparé de la preuve
de copieELF. Admission réelle doit posséder les DEUX acteurs : publication et
grants, vivants aprèsexec/préflight, poll+OS+HB contrôlés, préflightPGrefusant
avantARM sioutilnonqualifié. Un ACKcopie ne vaut jamais lease Vertex/7878/7879/PG.
Grants couvrent CommonEND original et arrêt ; validated_pg(admission=False)
réutilise admitted_at du grantinitial, pas un nouveauTTL/postcheckSQL supprimé.
Aucune de ces politiques/captures réelles n'est produite par ce manifeste.

## Séquence de preuve avant future admission

Après accord C7 sur interface+limites : extraction EFFECTIVE d'un même noyau,
migration des36 assertionsD45 pour appeler ce noyau, nouveaux témoins prépubliés
4pub5+1+1+1, ordre/extra/missing/SHAchange/references/window90sansreset,
ARMavantpréflight/firstred/expiration/UNKNOWN/cleanup. Publication avant CPU,
GOs distincts et premierrefusconservé. Puis adaptateurglobal lecture seule et
contrats owner/resource, nouveaucode/pin/paquet/RO TOTALE noyau+raccords.
Ce document n'implante rien et ne qualifie pas la sourceCargoprod ou le natif.

État actuel : relaisD4536CPUfakesverts mais NONQUALIFIÉtransportLinux/D6 ;
outilPGG toujoursNONQUALIFIÉ. Aucun actorARM/GET/SQL/health/provider/poste/Cargo/
rustc/SDKexport/service/targetmutation ou nouvelleactiviténative dans cetteprépa.
Ticketporteur documentaire et ticketspropriétaires du rapport36 restent ouverts.

### WindowAPI00e208 relue, contrat fermé sans bridge livré

Source exacte00e2084673e02b37a1d7ac3314015c80ca9c96e7 relue en lectureGit,
aucun import/exécution du noyau. PreparedContext.window_events mappe chaque
label à WindowEventSpec(pathPpréapprouvé,owner_role,event_kind), Rust mappingvide.
PhaseLifecycle.record_window_start/observe_window relient ledgerSHAoriginal,
ordre/ACKprécédent, owner réel et ENDop ; WindowEvidence retourne eventSHA,
owner vérifié et duréePC. StageHeaderSHA attaché par Gate APRÈS contrôlekernel,
pas par une affirmation du provider. Cela répond au manque d'entrée signalé.
Avis Archi favorable au CONTRAT PRÉPA, pas à un comportement runtime absent.

Codeparsers publie extractionétape1constante c9b84168e8, D45 déplacé au noyau
unique+wrappersRust :36assertions encoreNONREJOUÉES à son annonce. Cette étape
n'implémente NIquatrepublications NIWindowEventruntime. Le Reader domaine6e12
reste indépendant/NONJOUÉ aprèsfix ; aucun bridgePublicationSource n'est créé
pour contourner ces absences. Son format prototype windowJSON n'est pas une
normalisationruntime OwnerIdentity : notamment start/script_sha n'impose pas
au noyau les alias de son interface OwnerIdentity(start_ticks/entry_sha256/
component_hashes). Le futur bridge utilisera le schéma réellement publié et
validé par le noyau, avec nouveaupin/tests/RO, sans second juge de liveness.
229b/Q restent figés, outilPG nonqualifié et GO7455 toujours absents.
