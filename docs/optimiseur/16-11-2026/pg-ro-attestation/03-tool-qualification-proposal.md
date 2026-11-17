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
