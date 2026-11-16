# UNKNOWN : classification partielle et admission toujours refusée

Demande C7 après RO Mémoire du couple2397/APPea670. Lecture seule des13entrées de snapshot SONDE25107, puis relevé actuel séparé ; aucune sonde/poste/Cargo/garde/signal/mutation target supplémentaire. [Ticket existant](../tickets/2026-11-16-le-temoin-documentaire-vertex-attend-un-accord-sans-superviseur.md). Les snapshots historiques ne contiennent pas de start des UNKNOWN : les correspondances de PID actuelles ne prouvent JAMAIS l'identité historique.

| PID historiques | Lecture actuelle | Champs refusés |
|---|---|---|
|1247|argv systemd --user, UID réel/effectif/sauvé/fs1000, CG user@1000/init.scope|exe/cwd et tous liens FD consultés|
|1256,4085469|argv sd-pam, UID1000, CG user@1000/init.scope ou session-60.scope|exe/cwd/liste FD|
|2578,4086221|argv kwin_wayland, UID1000, CG plasma-kwin ou session-60.scope|exe/cwd/liste FD|
|2754,4086421|argv polkit-kde-authentication-agent-1, UID1000, CG plasma-polkit ou session-60.scope|exe/cwd/liste FD|
|2532216,2532317,2532319|argv kscreenlocker_greet/worker, UID1000, CG plasma-kwin|exe/cwd/liste FD|
|146509,180232,203361|PID absents au nouveau relevé|aucune identité actuelle, historique non attribuable|

Pour les10PID présents, stat/PPID/start/statusUID/argv/cgroup sont conservés dans le relevé actuel. Aucun de leurs arguments ni CG lisibles ne correspond à W/T/OLDW/control ou au CG du run historique. Cela ne prouve pas l'absence de FD/cwd/exe protégés ; le nom d'un programme et son appartenance à une session ne constituent pas une exemption. Les trois disparitions n'innocentent pas le snapshot historique. Source opaque de permission non attribuée au produit et aucune modification ptrace/dumpable/Yama/permissions proposée en douce.

Preuve actuelle PC cx-doc-unknown-classification-20261116/classification.json, copie portable même nom suffixe-local ; SHA256 identique deux postes **2b38da2e37e79528a0ebdf170401ec0687da39c638051fc9a53c79f9a18293c1**. Snapshot25107 original inchangé. Le relevé conserve chaque erreur/champ et les valeurs lisibles, aucune lecture environnement, clé ou corps de base.

## Quelle porte peut réellement verdir ?

Avec le garde2397 inchangé et l'observabilité actuelle : aucune admission native démontrable. Tous les UNKNOWN et ancestor_errors bloquent, donc même ces processus de session empêchent la reprise. Un nouveau budget1200 n'apporte pas la preuve manquante ; je ne propose pas une commande lourde condamnée à ce refus.

La porte peut verdir lorsque la capture complète devient réellement possible (changement externe d'observabilité autorisé), ou lorsqu'une preuve indépendante explicite de non-détention est fournie puis relue sous un nouveau contrat. Un observateur habilité pourrait par exemple établir les liens exe/cwd/FD de ces identités à l'instant d'admission et attester sa propre portée ; aucun tel observateur ni droit n'est disponible/autorisé dans ce lot. Une simple allowlist PID/nom/CG/baseline serait un assouplissement sans preuve et n'est pas proposée. Ne pas arrêter les processus de session ni modifier leurs protections.

Couple2397 reste publié et RO favorable au refus conservateur, mais non lancé. Ancien END natif1177548.693791571 expiré définitivement ; étape corrective1200 reste seulement proposée, sans GO. Décision d'observabilité/contrat à C7 avant toute nouvelle recette ; pas de mot Lucie sollicité ici, aucun Vertex900. Le rouge attendu du témoin Rust reste NON JOUÉ.
