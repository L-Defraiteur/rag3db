# Rouge final-lock observé avant correctif

[Ticket FS](../tickets/2026-11-16-l-admission-du-corpus-fs-ne-garantit-pas-la-vue-consommee.md),
HOLD C7521411c889. GO exact `C7-LUCIVY-FINALLOCK-RED-3D00-17nov60`, suivi
C7707b0abd0f, consommé une fois sur admission166d/harnaisffdddc inchangés.
Deux cas sans failfast, sous TERM55/KILL60, aucun retry.

Résultat réel : **1 FAIL attendu, 1 PASS, 0 ERROR/skip**, rc1.
Suite0,011s, physique0,146473145s ; start880181.136308118/end880181.282781263.

- Dérive : lock effectif synthétique1.0.0→2.0.0 après le retour Common/juges
  **modélisés**, seed inchangé. Le vrai admit166d.main vérifie source/copies/
  seed et publie encore final-accepted, rc0. Assertion discriminante échouée :
  `real final verdict accepted a changed effective Cargo.lock after the
  modelled seven judges`. Aucun hash final du lock effectif dans les événements.
- Contrôle : lock inchangé, même vrai main, rc0 et final-accepted présent.
  Le harnais atteint donc le vrai dernier consommateur sans rejet systématique.

Les conditions setup ont passé dans les deux cas : initial lock valide, seed
intact, Common modèle revenu, source et seed relus, final-evidence réellement
écrit. La source n'a pas été réécrite ; budget et publications du vrai main
ont tourné sur fichiers possédés. Aucun compilateur/juges Rust exécutés, pas
de vrai Common/Git/processus/proc/SSH/signal/ELF/target/corpus/service dans les
fixtures. Pas de clock virtuelle ni de mesure d'un arrêt OS.

[Bruts, deux observations, argv/rc/temps/SHA et reçu PC](preuves/final-lock-entry-red/SHA256SUMS).
Six fichiers relus par SHA et copiés en444 dans
`~/.cache/rag3weaver-build/cx-lucivy-fs-final-lock-red-17nov-v1` sur PC.
Transfert SSH d'archives après le témoin, pas dans les fixtures. Preuves deux
postes conservées **avant tout correctif final-lock**.

Qv3/nonce/seeddbdb/source0f et trois anciens ELF/T3552 intacts ; aucune dérive
du vrai lock n'est alléguée, aucune restauration/reset. GO CPU clos, journaux
ec619/d13 toujours sans GO, GO300 toujours HOLD. Suite : correctif groupé
du contrôle lock effectif après les juges/au dernier verdict dans END, puis
nouveaux pins et RO totales ; futurs GO CPU/natif exacts distincts requis.
