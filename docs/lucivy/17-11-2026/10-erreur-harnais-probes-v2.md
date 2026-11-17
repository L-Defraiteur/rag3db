# Refus de passe : deux ERROR de harnais

GO exact `C7-LUCIVY-NEW-RED-15B2-17nov60` consommé une fois, sous
TERM55/KILL60. Source baseline047111, probes c77827, fixe51a9 **non exécutée**.
[Ticket porteur FS](../tickets/2026-11-16-l-admission-du-corpus-fs-ne-garantit-pas-la-vue-consommee.md).

Résultat réel : **2 tests, 2 ERROR, aucun FAIL sémantique**, rc1.
Physique0,088187572s, start870951.529067523/end870951.617255095.
La passe s'arrête sans retry ni qualification : aucun des deux contre-exemples
annoncés n'a été observé. Les contrôles de setup n'ont pas été atteints.

Cause : le double de `Path.open(path, mode_arg='r', **kwargs)` reçoit le
`mode='r'` nommé de pathlib.read_text. Il appelle ensuite real_open avec mode_arg
positionnel **et** mode encore dans kwargs, causant `TypeError: Path.open()
got multiple values for argument 'mode'`. Ce défaut appartient au harnais.

[Bruts/argv/rc/temps/trois sources et reçu PC](preuves/watchdog-new-probes-error/SHA256SUMS).
Aucune observation.json n'a été créée, ce qui est conservé comme absence et
ne doit pas être comblé à la main. PC archive privé
`~/.cache/rag3weaver-build/cx-lucivy-watchdog-new-probes-error-17nov-v1`, cinq
copies444 vérifiées par SHA. Transfert SSH de preuves seulement après témoin,
pas une action des fixtures. Aucun vrai SSH/proc/signal/processus enfant,
Cargo/native/T/corpus par le témoin. Ancien paquet intact.

C7 informée immédiatement. Suite : corriger uniquement le contrat de signature
du double dans une nouvelle version/pin, publier puis RO et nouveau GO exact.
Ni retry implicite ni réutilisation du GO15B2. Le rouge EC4 reste valide dans
sa portée ; les deux nouveaux rouges et les trois verts ne sont pas acquis.
