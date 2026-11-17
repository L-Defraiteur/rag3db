# Trois raccords CPU verts — portée simulée seulement

[Ticket FS](../tickets/2026-11-16-l-admission-du-corpus-fs-ne-garantit-pas-la-vue-consommee.md).
GO exact `C7-LUCIVY-BOUNDARIES-GREEN-180A-17nov60`, suivi908bc8f457.
Une seule invocation sous TERM55/KILL60, harnais a99f28 et fixe51a9b1, case all.

Résultat réel : **3 PASS, 0 FAIL, 0 ERROR, 0 skip**, rc0.
Suite0,013s, physique0,102948729s ; start872073.385756712/end872073.488705441.
Les vrais args/préflight/hash/JSON ont été atteints dans les trois cas.

- Ordre : physical-watchdog-wrapper, ARM simulé, frontière d'écriture.
- Durée : parent simulé vieux40s, durée transmise236,991636704<=237, sans
  réinitialisation à300 ni horloge virtuelle.
- Erreur I/O : OSError fsync contrôlée après lancement simulé, puis
  terminate-local-child-mocked/wait-local-child-mocked, enfant simulé inactif.

[Bruts, trois observations, argv/GO/rc/temps/SHA et reçu PC](preuves/watchdog-boundaries-green/SHA256SUMS).
Huit fichiers relus par SHA et copiés en444 sur PC dans
`~/.cache/rag3weaver-build/cx-lucivy-watchdog-boundaries-green-17nov-v1`.
La connexion SSH de copie de preuves est après les tests, pas dans les fixtures.

Ces verts valident les raccords/arguments/décisions du vrai CLI avec parent,
timeout, transport et enfant **simulés**. Aucun proc/SSH/processus/signal réel
par les fixtures, aucun timerOS/kill/cleanup distant ou300s mesuré. Aucun
Cargo/native/T/corpus/service. Ils ne qualifient ni l'admission FS ni les six
rouges Rust encore à jouer. Les rouges EC4/180A et les2ERROR313c sont conservés.

Suite autorisée en préparation si ces verts passent : matérialiser **un nouveau
paquet PC v2**, sa recette physique complète et ses pins, puis RO avant tout
futur GO distinct. Ancien Q/047/source8b/T restent figés/nonconsommés. Aucun
GO Runtime/Rust/natif ne découle de cette passe CPU.
