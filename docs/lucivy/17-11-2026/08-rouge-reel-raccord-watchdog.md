# Rouge observé sur le vrai raccord CLI

[Ticket FS ouvert](../tickets/2026-11-16-l-admission-du-corpus-fs-ne-garantit-pas-la-vue-consommee.md).
GO exact `C7-LUCIVY-WATCHDOG-RED-EC4-17nov60`, suivi C7bc67913956.
Une seule invocation du témoin prépub ec4f97a83c/be491384 sur source047111
octet pour octet, sous timeout TERM55/KILL60. Aucun retry ni correctif avant.

Résultat réel : **1 test, 1 FAIL, 0 ERROR**, rc1. Suite unittest0,004s,
invocation physique0,110091704s ; start869975.495581373/end869975.605673077.
Les contrôles setup vrais args/préflight/hash/JSON sont passés. Observation :

```text
arm-transport-mocked
post-arm-write-frontier
```

L'assertion échoue précisément sur l'absence de `physical-watchdog-wrapper`
avant ARM puis ouverture du log. Le mode xb est intercepté **avant** ouverture,
aucun log de lancement/ledger produit par le CLI. ARM simulé, Popen interdit,
aucun SSH/réseau/processus enfant, scanproc, signal réel, Cargo, ELF, target,
corpus ou service dans le témoin. Aucun300s mesuré, timerOS/kill/cleanup ou
ENDdeuxpostes qualifié. Le cas315 reste NONJOUÉ et la native FS reste HOLD.

[Bruts et manifest](preuves/watchdog-entry-red/SHA256SUMS) : journal unittest,
observation, argv/GO/rc/deux SHA/temps et reçu d'archivage PC. Copie sur PC
`~/.cache/rag3weaver-build/cx-lucivy-watchdog-entry-red-17nov-v1`, cinq fichiers
relus par SHA, sources et bruts444. Cette copie par SSH est **un transfert de
preuves après le témoin**, pas une connexion effectuée par le CLI testé.
Ancien paquet/Q de qualification, source8b et T3552 non consommés et inchangés.

Le rouge et les sources sont conservés sur les deux postes AVANT correction.
Suite autorisée en préparation : composer timeout physique local avant le
raccord, transmettre des durées restantes conservatrices au PC, prévoir arrêt
des enfants et réserve de fin ; nouveaux pins/RO/GO distinct avant tout vert.
