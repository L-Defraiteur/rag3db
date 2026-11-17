# Pilote Rust : paquet préparé, aucun GO consommé

Même ticket FTS302/301. Cette proposition ne lance ni CPU tests, compilation,
client natif, SQL, Docker ou serveur. Le module PG pur25fakes/de69 reste inchangé.

## Entrées figées horsOUT_T

Sur PC : `~/.cache/optimiseur-notes/pg-rust-tool-qual-20261117-r1/inputs/`.
317fichiers/660534109octets : rustc privé, ses bibliothèques et sysroot,
74artefacts sélectionnés, fingerprints et provenances ED. Copies régulières,
aucun lien/hardlink vers source. SHA sources avant/après puis comparaison
GLOBALE finale aux copies ; aucun octet source modifié.
Manifest SHA bf6b72be485227fadfdda40c133c5b52d4a8a31e7dba1855821c2b34f85d8e25,
READY SHA39e9152c204d5c966307eef4586c7ef278031a82ee58e04c0c237eaeeaccadb4.
Rustc859 identique. Treize pins linker/runtime HÔTE relevés seulement, à revalider
avant chaque phase ; pas une qualification de fermeture du loader natif.
Nativebuild/out.a ED absents : toute erreur de link STOP, zéro fallback/Cargo.

## Phases proposées, chacune exige son GO nouveau

A. CPU hermétique60 : dix faux du VRAI orchestration/NativeBackend avec Popen,
proc, autorité et clock remplacés ; huit vecteurs publiés avant runner plus
frontières timeout/finalfsync. Sources seulement pour le moment, NONJOUÉS.
Aucun proc/processus réel, PG ou calcul de modèle dans ces faux.

B. Native compile+parsing : vrai poste lourd32G puis Common120/OS125+kill2
(proposition non exécutée, coût non mesuré). Un rustc SEUL, cap60 incluant
préflight/stderr/stdout/link/cleanup/publication ; `--sysroot` privé,3externs,
linker hôte piné ; niCargo/build.rs/SDK/C++. Sortie dans un dossier vierge
exclusif lié au GO ; marqueur compile-used avant lancement, aucun retry.

Puis huit CLI/DSN fixtures, OS whole<=5 chacune : CLI sans mode, JSON invalide,
nonce invalide, budget invalide, DB étrangère, hôte étranger, champ SQL arbitraire,
et parsing canonique positif. Toutes les actions sont `--validate-request`
ou CLI vide. Le mode de validation appelle SEULEMENT prepare_config, ni runtime
ni connect/query ; mode `--capture-readonly` interdit par le pilote. DSN fixture
publiée avec faux password inutilisé, jamais un secret canonique dans les pièces.

La borne commence dans NativeBackend AVANT dossier/request/stdin/guards/startup.
Le pilote supervise un groupe OS privé contenant phase Python + natif/linker ;
les captures bornées/fsync sont DANS cette phase. Timeout → kill du groupe
attribué UID/cgroup, reap, préfixes conservés, refus. Une sortie de capture n'est
admise qu'après code contrôleur0, groupe quiet, natif reaped, finalfsync/autorité/
horloge dans la borne. Toute quiescence inconnue est ROUGE, pas une permission
sur un groupe tiers. Les métadonnées result.json sont intermédiaires ; seul
retour global rc0/CommonVERT + qualification finale SANS first-red est acquis.
Si I/O ou cleanup dépasse la borne, conserver first-red et sorties, jamais
réadmettre un JSON VERT par sa seule présence après refus. L'OS réel et ses
chemins d'échec restent à qualifier : les fakes/source ne le prouvent pas.

C. PG réelle : contrat/RO/GO DISTINCTS et pins runtime topologie/OID/réservation
à obtenir avant tout contact. Pas incluse dans A/B. Postcheck = lecture de la
session cliente attribuée sur serveurPG EXISTANT, ne crée/lance aucun backend,
base ou service. Dockerinspect futur FORMAT Id/Image/Ports uniquement, timeout/
reap propres encoreNONQUALIFIÉS. Aucune inspection Env ou dockerexecPG.

## Raccords, autorité et publication

Paquet source : tool_qualification.py + tool_phase.py, Rust/SQL, vecteurs/faux,
Common6439 copié bitidentique et packet.json scellé. Aucun appel à l'import.
Le pilote `--run` exige SHA paquet externe, GO fichier+SHA, purpose exact,
référence/nonce, deadline et sortie exclusive, puis VRAI Common/poste dans
l'ascendance (32G). Il ne crée aucune allocation, ne lit aucun .env/.vault,
ne transmet pas le jeton Common au natif, ne journalise pas d'environnement.
Un JSON GO de préparation a run_authorized=false et n'est pas une permission.

Le Common DOIT être celui copié dans le paquet, afin que le chemin signé du
superviseur et sa SHA correspondent. Proposition de lancement B, NONEXÉCUTÉE :
`POSTE_MEM_MAX=32G poste lourd timeout --kill-after=2 125 python3 <packet>/common-confiner.py lancer --delai 120 --preuves <preuves-neuves> -- python3 <packet>/tool_qualification.py --run --packet-sha <SHA> --go <GO-neuf> --go-sha <SHA-GO>`.
Aucune nouvelle fenêtre/timebox de service et aucun nouveau budget par retry.
Chaque borne est min(cap,END original restant), jamais un renouvellement.

Un VERT éventuel B ne qualifiera que build/link et refus/parsing SANSCONTACT,
avec ELF/source/inputs/bruts/PID et phases réellement joués. Il ne rendra PAS
transport_qualified=true. Nouveau transport Rust incompatiblev1 ; pas de
renommage private-host-psql, pas d'adaptation Q/owner figés à la volée.
Armement global Arbre/Embar est distinct, aucun ARM/READY/grant global ici.

## Paquet physique pour RO totale — 17 novembre

Source code publiée : `808be0595ffd033e5ffd69d934eedfceee22d174`.
Paquet PC privé :
`/home/lucied/.cache/optimiseur-notes/pg-rust-tool-qual-20261117-r1/packet-r1`.
`packet.json` SHA256 :
`b9b0e8ea52e83cf39da7969354f7d99c9e76584ce590fb9752a10838b382389c`.
Ses douze fichiers sources sont épinglés individuellement ; le modèle GO est
inactif, délai zéro, et le futur GO exact doit rester extérieur au paquet.
La documentation embarquée est la version du commit source ci-dessus.

Entrées privées voisines `inputs/` : 317 fichiers, 660534109 octets,
compiler/sysroot et 74 artefacts dépendants. Manifest SHA256
`bf6b72be485227fadfdda40c133c5b52d4a8a31e7dba1855821c2b34f85d8e25` ;
READY final SHA256
`39e9152c204d5c966307eef4586c7ef278031a82ee58e04c0c237eaeeaccadb4`.
Provenance OUT_T ED manifeste
`de797438448c79ae54a1bc313003b0fdead4e37b70b180298ce21690b320c945`.
Les treize pins linker/runtime hôte seront revérifiés avant toute phase ;
ils ne constituent pas une qualification exhaustive du chargeur.

Contrôles effectués : syntaxe AST et JSON uniquement, fichiers réguliers privés
sans liens vers les sources. Dix témoins CPU préparés, huit cas CLI/DSN préparés,
**aucun joué**. Aucun compiler, client natif, SQL, Docker, health ou service lancé.
Les budgets restent proposés : CPU distinct 60 s, compilation unique 60 s,
huit workers 5 s chacun incluant stdin/démarrage/publication, Common 120 s,
OS 125 s puis kill 2 s. Le ledger ELF est publié avant les cas CLI.
Le fonctionnement réel du watchdog, de la compilation et du nettoyage reste
NON QUALIFIÉ jusqu'aux validations autorisées distinctement.

RO totale demandée sur ce code et ce paquet avant tout nouveau GO.
Le transport Rust reste incompatible avec le contrat PGv1 figé ; aucune
qualification d'accès PG ni activation globale ne découle de ce paquet.

### Réserve RO parent et correctif préparé

Le paquet b9b0e8ea est désormais historique : la RO R a relevé que le parent
pouvait bloquer en hash/fsync hors interruption malgré son contrôle de temps.
Le correctif source prépare un superviseur Linux fork/pipe borné ; toutes les
opérations de phase sont dans le worker, ses sous-processus restent dans son
groupe. Le parent ne lit pas les fichiers de phase et réserve .4 s pour
kill/reap ; manque de résultat complet, descendant restant ou retour tardif
refusent la phase. Les deux anciens témoins NativeBackend testent maintenant
explicitement le corps interne, pas la supervision externe.
Deux témoins supplémentaires de délégation/refus sont préparés (12 au total),
sans processus réels ; ils ne prouvent pas encore le comportement OS du fork,
du pipe et du nettoyage. Aucun rouge runtime ni vert n'est revendiqué.
Cette nouvelle source doit recevoir sa RO avant création d'un paquet r2 ;
aucun nouveau GO n'est demandé sur l'ancien paquet.

Seconde réserve R consignée avant correction : le nettoyage du finally
précède maintenant le dernier check temporel, puis le retour ne fait plus de
scan ni de lecture de fichier. Le cgroup attendu est capturé au setup global
par le pilote, injecté au backend ; sa revalidation est dans le worker supervisé.
Cela prépare le refus d'un nettoyage tardif ; la latence réelle du scan proc
et le kill/reap restent à qualifier, sans garantie OS déduite d'une lecture.

## Paquet r2 — après levée des deux réserves statiques

RO R favorable bornée au code exact
`32df1b7fdc2fc4d10d8e100c2dcd3a741eef5153` : nettoyage avant dernier check,
cgroup injecté au setup et revérifié worker. Cela ne qualifie pas les opérations
OS réelles ; une latence proc peut dépasser le cap, mais ne doit plus rendre VERT.
Le Common extérieur demeure le filet physique.

Nouveau paquet PC :
`/home/lucied/.cache/optimiseur-notes/pg-rust-tool-qual-20261117-r1/packet-r2`.
Descripteur packet.json SHA256 :
`b0795ccbb6f2d3aad0ad7b20bc19e571cd01c6071762c48a751bd45e58142780`.
Code embarqué : commit exact ci-dessus ; runner SHA256
`61cc3231d2488ca2b6d2cda92720f2d78c4a3316be2f0d324d24ecdaf03f3af7` ;
témoins SHA256
`1baa764d3c7668ca55b1067b8145366f57b9cec8776434b27a3391a4e0578629`.
Douze fichiers épinglés ; entrées manifest/READY identiques au premier paquet.
Douze témoins et huit parser cases NON JOUÉS, aucune compilation ni SQL.
Le paquet r1 reste conservé historique, aucun remplacement. RO des pins r2
requise avant nouveau GO CPU distinct et qualification compile/parse distincte.

## CPU distinct acquis — C7-G-PG-CPU-32DF-17nov60

GO C7 rapport17 publié c7a2bc988d ; invocation UNIQUE autorisée depuis paquet-r2 :
`timeout --signal=TERM --kill-after=2s 58s python3 -B -m unittest -v -f test_tool_qualification`.
Après rehash des douze fichiers : 12 PASS, 0 FAIL, 0 ERROR, 0 SKIP, rc0.
Unittest 0.014 s ; commande 0.089162598 s. Monotoniques PC
1257220.533717243 → 1257220.622879841. Sources avant/après strictement égales.
Aucune relance. Preuves privées aux DEUX postes, chemin identique :
`/home/lucied/.cache/optimiseur-notes/pg-rust-tool-qual-20261117-r1/cpu-32df-17nov60-01/`.
Receipt SHA256 `6d594431f54494e36d931c074bd25deec5116fcb6a0ae333ae944a9fa3196906` ;
stderr SHA256 `578818fc1669273ee90012f4bc4dd2d21dde06c97c8f5de61739697707775894`.
stdout vide. Copie des quatre fichiers bruts/receipt vérifiée par SHA.
Le paquet immutable garde son statut de préparation initiale ; le présent reçu
porte la validation ultérieure distincte. Les 317 entrées/OUT_T ne sont pas modifiés.

Ces verts sont exclusivement ceux des faux : supervise_phase est remplacé
entièrement dans deux témoins. Aucun fork/pipe/scan/kill/reap réel ni borne5s
physique n'est qualifié. Aucun compiler, rustc, SQL, Docker, SDK, service7970,
slot ou client natif lancé. Les huit cas CLI natifs restent NON JOUÉS.
Transport et lease PG restent NON QUALIFIÉS ; qualification native nécessite
un GO distinct, aucun transfert de ce GO CPU.

## Phase B concrète après CPU — préparation02, sans GO

Le premier exemple de lancement par common-confiner.py est invalide et ne doit
pas être utilisé : signature_superviseur exige le basename confiner_validation.py
ET argv[2]=lancer. Une option -B devant ce script décalerait cette signature.
Le module chargé par le runner et le superviseur exécuté peuvent avoir deux
chemins différents : la primitive vérifie leur SHA identique, et la signature
vérifie le chemin réellement exécuté contre le reçu. Aucun changement Common.

Recette privée versionnée aux deux postes :
`/home/lucied/.cache/optimiseur-notes/pg-rust-tool-qual-20261117-r1/phase-b-preparation-02/recipe.json`.
SHA256 `25715b699a72eea34d6adc8e8a32b479d9d19f7d93fc618d31d33a2a65a7b2d4`.
La préparation01 est conservée historique ; elle n'a pas été exécutée.
La02 contient argv POSTE absolu, argv rustc unique intégral, huit cas CLI,
GO externe inactif, hashes poste/Python/timeout et copie régulière du Common6439
nommée confiner_validation.py. Le paquet-r2/code32df reste figé.

Audit RO PC : 317/317 fichiers des entrées rehashés, taille et chemin résolu
conformes ; 13/13 pins hôte rehashés ; 12/12 sources r2 rehashées conformes.
Monotoniques audit1257943.367455839→1257943.710505389.
Le manifeste bf6b72be et READY39e9152c sont inchangés. Poste cache SHA256
`79e3a8bbf457e3bc9104aa1558e7d03072216c70b68e08ae606ace6aa4584c7b` ;
Python /usr/bin/python3→python3.14 SHA256
`a38210a53e07ac5eb912ee35c2d92ddb7eaa349301b60c188a5f496c7c87449d`.
La fermeture complète du chargeur n'est pas qualifiée par ces treize fichiers.

Commande proposée, NON EXÉCUTÉE (P désigne le parent privé pg-rust-tool-qual-20261117-r1) :

```bash
POSTE_MEM_MAX=32G PYTHONDONTWRITEBYTECODE=1 \
/home/lucied/.cache/rag3weaver-build/poste lourd \
/usr/bin/timeout --signal=TERM --kill-after=2s 125s \
/usr/bin/python3 "$P/phase-b-preparation-02/confiner_validation.py" \
lancer --delai 120 --preuves "$P/phase-b-preparation-02/common-proofs" -- \
/usr/bin/python3 -B "$P/packet-r2/tool_qualification.py" --run \
--packet-sha b0795ccbb6f2d3aad0ad7b20bc19e571cd01c6071762c48a751bd45e58142780 \
--go EXTERNAL_FRESH_AUTHORIZED_GO_PATH --go-sha EXTERNAL_FRESH_AUTHORIZED_GO_SHA256
```

La sortie exclusive proposée `P/runs/phase-b-32df-01` et les preuves Common
sont absentes ; aucun poste/slot n'a été pris. go.inactive.json reste run=false,
deadline0, nonce zéro ; SHA256
`a4c91289eab68db4721357f59e66bca067a9e380b41ffef5842d69cd9875d81d`.
Il ne peut être transformé sur place en autorisation. Le futur GO distinct
lie packetSHA, inputSHA, référence, nonce frais, deadline absolue et cette sortie ;
aucun renouvellement après attente de poste. Le runner refuse si son délai
excède END Common ou est épuisé. Rehash et absence sortie sont à refaire au GO.
RO totale R source+paquet+recette après CPU demandée avant qualification native.
Les douze faux ne sont pas rejoués. Aucun compiler/SQL/Docker/service déclenché.

## B01 rouge avant compilation — aucun retry

GO distinct C7-G-PG-NATIVE-B-32DF-17nov120/a3da7a954b consommé une fois ;
recette02 exécutée, rc1 en0.520146039s. Common validation-_je8hi1g ROUGE,
restes[]/nettoye=true. Contrôleur refuse l'autorité avant native Popen ;
rustc NON INVOQUÉ, aucun ELF/ledger/cas CLI natif. Sources r2 inchangées.
La lecture pointe HOMEprivé sans POSTE_LOCK_DIR transmis au contrôleur,
Common.poste_tenu utilisant HOME pour retrouver les fiches du vrai poste.
Diagnostic consigné dans ticketFTS302 avant toute correction ; pas de retry,
pas d'invention de sous-cause attestée par le refus générique.
Bruts/GO/identités du request/receipt/firstred/verdict conservés aux deux postes ;
receiptSHA2565d3044612e97d8e6cbf1bd607b8f6fd968c9bed6bd425fefeca96fdb3e05390e.
Copie Common publique sans jeton incluse. OS/transport toujours NONQUALIFIÉS,
aucun SQL/PG/Docker/service/phaseC lancé, aucun lease produit.

## Correctif Common B01 — préparation après refus, aucun rejeu

Témoins publiés AVANTfix7e4f02afb7 : six nouveaux cas test_poste_authority.py,
constructeur controller_environment et Common6439.poste_tenu réels, proc/fdinfo
et fiches uniquement fixtures privées. Ils couvrent HOMEprivé+racinelock,
racine arbitraire, fiche sans FLOCK ou FD correspondant, changement PID/start,
absence d'autorité côté natif. NON JOUÉS ; aucun rouge CPU revendiqué.

Le correctif prépare une attestation depuis Common.autorite et poste_tenu :
racine POSTE canonique attendue EXTERNE au paquet (`poste_lock_root`), ancêtre
PID/start/UID/cgroup, fiche régulière UID/inode/SHA, FD9 même dev/inode que
poste.lock et fdinfo kernel FLOCK ADVISORY READ (lourd) ou WRITE (mesure),
identité et métadonnées stables après lecture. Atime est exclu de la stabilité,
puisque la lecture elle-même peut le modifier. Le contrôleur reçoit cette seule
racine dans POSTE_LOCK_DIR, garde HOME privé et refait Common+attestation ;
sa preuve doit égaler celle du parent avant toute invocation native.
Le natif reçoit une whitelist sans reçu/jeton/lockdir Common. Aucune fiche créée
ou verrou acquis par ce mécanisme : lecture seule des preuves du vrai poste.

Les nouveaux champs et sources exigent un paquet r3 distinct, une nouvelle
recette/sortie/GO, RO totale et GO CPU/natif distincts. r2/recette02/B01 restent
immuables. Le correctif n'est ni exécuté ni qualifié ; pas de retryB01/ancien120.

### Paquet correctif r3 et recette03 — préparés, NON EXÉCUTÉS

Code exact de4a90268d78e2af1d2236fb08e0623123b4d89e ; témoins préfix7e4f02afb7.
PC : `P/packet-r3/packet.json` SHA256
`7c0f3f46fa0dd9e20895496fecd57ad71846a631a98e9bd35665e1434bf75302`.
Treize fichiers sources pinés ; poste_lock_root attendu externe fixé au chemin
canonique `/home/lucied/.cache/rag3weaver-build`. Ce champ n'atteste pas à lui
seul une autorité : Common+PID/start/fiche/FD9/flock sont exigés à l'exécution.

Recette aux deux postes `P/phase-b-preparation-03/recipe.json` SHA256
`950786b463c212c5b919a0fff09f55569ab9211b0840e7461d91821345d4cd01`.
P reste `/home/lucied/.cache/optimiseur-notes/pg-rust-tool-qual-20261117-r1`.
GO inactif, nouvelle sortie `P/runs/phase-b-corrective-01` absente,
preuves Common03 absentes ; aucun remplacement d'un ancien paquet/GO.
Les compteurs d'audit317/13 antérieurs ne sont pas présentés comme audit r3 :
la recette exige leur requalification avant futur GO. Aucun source importé,
CPU, proc/fdinfo réel, poste, rustc, SQL ou service lancé ici.

Dix-huit témoins préparés (douze existants adaptés plus six nouveaux), tous
NON JOUÉS sur cette source. Demande RO totale code/r3/recette03 avant tout
GO CPU puis natif distinct ; les verts32df ne se reportent pas sur de4a.
