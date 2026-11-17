# GO1 et GO2 : cinq contrôles CPU verts, archives avant GO3

[Ticket FS](../tickets/2026-11-16-l-admission-du-corpus-fs-ne-garantit-pas-la-vue-consommee.md).
Admissions distinctes publiées par C709726a3640, couple77a/source0f inchangés.

- GO1 `C7-LUCIVY-FINALLOCK-GREEN-V4-17nov60` : une invocation33ae/d508,
  **2 PASS, 0 FAIL/ERROR/skip, rc0**. Physique0,078369030s, suite0,004s ;
  start885727.154898568/end885727.233267598. Dérive refusée au vrai dernier
  main avec cause exacte, seed intact ; contrôle inchangé accepté. Six fichiers
  sources/bruts/observations/rc/temps relus et archivés deux postes avant GO2.
- GO2 conditionnel distinct `C7-LUCIVY-JOURNAL-GREEN-V4-17nov60` : une
  invocationdc0/7c58, **3 PASS, 0 FAIL/ERROR/skip, rc0**. Physique0,100854246s,
  suite0,016s ; start885880.136229865/end885880.237084111. START avant ARM,
  END après cleanup, échéance conservée, durée236,992<=237, enfant simulé
  réapé après erreur fsync. Sept fichiers archivés deux postes avant GO3.

[GO1 bruts et reçu PC](preuves/final-lock-entry-green/SHA256SUMS),
[GO2 bruts et reçu PC](preuves/journal-v4-green/SHA256SUMS).
PC caches privés final-lock-green-proof/journal-v4-green-proof-17nov-v1,
fichiers444. Copies contrôlées ; archives transférées par SSH après les tests,
aucun SSH/proc/processus/signal/Cargo/ELF/target/corpus dans leurs fixtures.

Ces contrôles valident les vrais raccords CLI avec Common/Git/parents/RPC
modélisés. Les reçus imbriqués des observations CPU ont un parent **modèle**,
ils ne doivent pas être confondus avec les reçus physiques réels de GO3.
Aucun OS kill/timeout expiré/cleanup distant ou300s ne découle de ces cinq PASS.
Les anciens51a9/a99 restent historiques et ne sont pas transférés à7c58.

Les conditions GO1+GO2 et conservation deux postes étant réellement remplies,
GO3 distinct a été armé une fois : [six rouges Rust FS](24-six-rouges-rust-fs-observes.md).
