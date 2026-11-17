# Deux nouveaux rouges observés sur047111

[Ticket FS](../tickets/2026-11-16-l-admission-du-corpus-fs-ne-garantit-pas-la-vue-consommee.md).
GO exact `C7-LUCIVY-NEW-RED-180A-17nov60`, suivi C7 70007963cf, consommé une
fois avec le harnais corrigé a99f28/180a5628cf. Source baseline047111 intacte.
Sous TERM55/KILL60, seuls les deux nouveaux cas : aucun rejeu du témoin EC4.

Résultat réel : **2 tests, 2 FAIL attendus, 0 ERROR**, rc1.
Suite0,006s, physique0,102601622s ; start871643.076208981/end871643.178810603.
Les contrôles réels args/préflight/hash/JSON ont été atteints dans les deux cas.

- Durée : corps ARM portant300 au lieu de<=237 pour un parent simulé vieux40s
  et une réserve de transport8s. Assertion exacte `300 not less than or equal
  to 237`, aucune horloge virtuelle.
- Erreur I/O : après lancement simulé et erreur contrôlée du fsync ledger,
  événements finaux ledger-transport-mocked/ledger-fsync-error-injected au
  lieu de terminate-local-child-mocked/wait-local-child-mocked. L'enfant
  **simulé** reste actif, observation locale_mock_child_running=true.

[Bruts, observations, rc/argv/temps/trois SHA, reçu PC](preuves/watchdog-new-probes-red/SHA256SUMS).
Sept fichiers relus par SHA sur PC, en444 dans
`~/.cache/rag3weaver-build/cx-lucivy-watchdog-new-probes-red-17nov-v1`.
Transfert SSH de preuves après le témoin seulement. Aucun SSH/proc/signal/
processus réel dans les fixtures, aucun Cargo/native/T/corpus/service, aucun
OSkill/cleanup distant/300 mesuré. L'ancien paquet/Q/T et source8b sont intacts.

Les2ERROR de la tentative c77827 restent archivés comme erreurs de setup et
ne sont pas remplacés par ces rouges fonctionnels. Le rouge EC4 reste dans
sa portée propre. Aucun test sur la fixe51a9 n'a encore tourné ; nouveaux
pins/revue déjà publiés n'autorisent pas les trois verts. Attendre un **GO
distinct** pour `--case all` sur51a9/a99, sans réutiliser le GO180A.
