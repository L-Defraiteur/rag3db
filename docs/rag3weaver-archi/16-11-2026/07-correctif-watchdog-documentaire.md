# Préparation du rôle WATCHDOG et étape corrective proposée

Le [ticket existant](../tickets/2026-11-16-le-temoin-documentaire-vertex-attend-un-accord-sans-superviseur.md) et le [rapport06](06-sonde-controleurs-documentaires.md) portent le diagnostic/plan publiés avant ce correctif. Le nouveau timeout203819 est prouvé par la sonde25107 ; PID56584 historique reste NON ATTRIBUÉ. Rien ne requalifie le refus natif f27f491. C7 autorise cette préparation, pas un lancement natif.

APP reste ea67054a11d73e06cf44c38d9ce941a754c0190d. Pilote neuf cx-archi-doc-watchdog-pilot-r1, scripts [doc-watchdog](doc-watchdog/qualify_red.py). Le constructeur de commandes appelé par le vrai lanceur fournit aussi la politique. Rôle WATCHDOG exige argv complet timeout/TERM/kill15/1320/poste/admission, exe résolu+SHA, UID/cwd exacts, identité PID/start/PPID/CG égale au snapshot d'ancêtre, parent lanceur exact/mêmeCG ; aucun rôle pour PID réutilisé ni clone extérieur à l'ascendance. Après reconnaissance, seuls motifs argv/CG de contrôleur sont levés ; tous FD/exe/cwd vers T/ancienW/source restent bloquants, excepté l'ancien FD de script exact/hashé du seul poste. Scan conserve motifs+identités+ancêtres/comparaisons avant assertion.

Dix fonctions hermétiques préparées : vrai module chargé par les assignations du lanceur, vrai constructeur et scan sur fauxproc. Cas nominal3rôles, argument1319 refusé, PID réutilisé refusé, clone horschaîne refusé, FDtarget/ancienW et cwdsource du watchdog refusés, FDtargetposte refusé, flagpython refusé, lecteur extérieur capturé. Publication code/témoins avant leur exécution ; aucun PID/Cargo/signal réel dans ces juges.

## Budget : proposition explicite distincte, aucune réactivation

END natif initial1177548.693791571. Le lanceur correctif EXIGE que cet END soit déjà expiré avant toute création P/Q ; il refuse de prendre cette nouvelle étape pendant la fenêtre initiale. Le mode explicite --corrective et configuration CORRECTIVE_STEP_PROPOSED rendent le contrat visible. Proposition à faire admettre séparément : nouveau1200 après vrai poste32G/HIGH0, extérieur1320 avec attente, compile900 puis UN exact120 sous mêmeEND−15. Aucune reprise de l'ancien END ni report du GO f27 ; aucun GO natif implicite fourni par cette préparation. Sans admission C7 distincte, ne pas exécuter.

W PC pilote proposé ~/git_workspaces/rag3db-cx-archi-doc-watchdog-pilot-r1. P/Q neufs cx-archi-doc-supervisor-corrective-r1 et cx-archi-doc-supervisor-controller-corrective-r1. Commande depuis /var/tmp : python3 cheminabsolu/doc-watchdog/qualify_red.py --corrective SHA40luGit. Autorité red1 terminée/ENDexact/rouge nettoyé/bus absent et premierrouge hashé vérifiés après poste ; cache porte toujours713433/ancienW, aucune mutation durant sonde. Garde commune réelle, anciens ELF copiés, lib59ac/profilsréels/étatsfrères/j8borné, compilation/ledger/un exact/finaliseurstrictcode101/bus/TERMINE/deuxENDchecks conservés du candidat f27. Seul rouge attendu manque-superviseur, jamais qualification verte ; sinon STOP diagnostic. Aucun Vertex900 ni service7970 ni accord rm de production.

Publication ead34d097127273255c70c62e96ffaffb29330c4 AVANT essais, puis10fonctions/0échec en0,052s sous timeout60. Log cx-doc-watchdog-cpu-20261116/ead34-tests.log SHA256 f2843d51ce9a47968fc4cc4e7ab3d9064e9300cbd5fc9300fec47145e142defd. AST3/diffcheck ; aucun Cargo, vrai PID, /proc ou signal dans les témoins. Demande RO du couple final et arbitrage de la nouvelle étape corrective ; aucun P/Q créé ni lancement natif.

## Réserve UNKNOWN levée par contre-exemple puis correction

Réserve Mémoire sur9f52 consignée ce8feb avant diff. Tests du vrai bloc admission AST publiés62f146 AVANTessai : UNKNOWN sans attribution accepté à tort, rouge1/0,009s (AssertionError non levée). Correctif9f9e152115449caf8f1cf325822e5b3dffb27916 publié AVANT rejeu : snapshot partiel conserve champs encore lisibles/motifs/erreurs ; fresh_cache refuse TOUSunknown et ancestor_errors après archivage, y compris attribution impossible. Aucune baseline exemptée. Cela peut refuser les13UNKNOWN de la sonde ; aucune autorisation de les blanchir ni de muter le cache malgré eux. Seize fonctions hermétiques passent en0,092s, timeout60, zéroPID/Cargo/signal réel.

Log horsW cx-doc-watchdog-cpu-20261116/red-62f146.log SHA256 641dfb8421ada8eb1234ce64dd265a7612ff9184fa6ded897b5f973da8214d2f.

Log horsW cx-doc-watchdog-cpu-20261116/green-9f9e.log SHA256 79e9086a297da7309184b6f01f36bb085e4704437e45d3bf8743d232c5b90469.

Couple9f52 supersédé/nonlancé. APPea670 reste noncompilée ; demande nouvelle RO sur code9f9e et SHA documentaire final. Étape corrective1200 demeure PROPOSÉE, aucun GO natif, ancienEND expiré sans réactivation.
