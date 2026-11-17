# Enveloppe locale v2 — code et probes proposés, aucun vert exécuté

[Ticket FS](../tickets/2026-11-16-l-admission-du-corpus-fs-ne-garantit-pas-la-vue-consommee.md).
Le [rouge CLI réel](08-rouge-reel-raccord-watchdog.md) est conservé deux postes,
publié00c7e799f9 avant cette préparation. Source047111 et ancien Q restent
figés. Le témoin315 n'est pas joué. Aucun Cargo/native/corpus/index/ELF.

## Composition physique et durée restante

`outils/export-fs-red-ledger-v2.py` est une nouvelle source, sans modifier047111.
Il compose `/usr/bin/timeout` TERM295/KILL300 avant son vrai main, au moyen d'un
execv et d'une réentrée portant un flag interne. Le worker vérifie son parent
vivant : exe timeout, commande exacte, PID stable et début provenant des ticks
de `/proc/<parent>/stat`. Ces métadonnées concernent **son seul parent local**,
pas un scan global ni un accès à un hôte distant. CLOCK_BOOTTIME et les ticks
parent servent au même repère local ; la quantification des ticks arrondit le
début vers le passé. END de travail local = début parent +285, jamais après ARM.

La future recette physique exacte doit poser timeout **depuis le shell avant
Python**, avec le flag enfant, pour couvrir aussi le démarrage de l'interpréteur :

```text
PYTHONDONTWRITEBYTECODE=1 /usr/bin/timeout --signal=TERM --kill-after=5s 295s
  <python absolu épinglé> <exporteur v2 figé> --_physical-watchdog-child
  --preflight <nouveau préflight> --output <ledger neuf> --launch-log <log neuf>
```

Les lignes représentent une seule commande future, pas une commande lancée.
La voie autoexec du script compose le même wrapper avant main ; elle ne prouve
pas une borne sur un démarrage initial d'interpréteur laissé sans surveillance.
Un flag enfant fourni à une entrée directe sans le vrai parent exact est
refusé avant argparse/préflight/transport. Pas de nouvelle primitive de relais.

Juste avant ARM, une durée est calculée : ENDlocal−débutRPC−8. Le PC reçoit
**uniquement cette durée**, pas une échéance locale absolue, et pose son propre
END = nowPC + durée. Le retour complet ARM doit prendre<=8 secondes, sinon
STOP sans lancer la passe distante. Pour un ARM réussi, son instant distant
est situé dans ce round-trip, donc l'allocation distante ne dépasse pas le
END local. Aucune égalité/comparaison d'horloges des deux hôtes.

Les RPC ultérieurs prennent min(8, reste−2). Plus aucun now+300 après ARM.
L'ancien timeout physique PC et CommonMIN restent nécessaires ; le nouveau
paquet v2 devra reprendre la borne PC avec la durée reçue, sans la recréer.
Il n'est pas assemblé/armé maintenant. Réserves de travail285 et de fin, ainsi
que timeout physique local, bornent I/O/ACK/copie finale/attente retour. Les
erreurs ordinaires terminent et attendent l'enfant SSH local dans finally,
avec kill de repli borné. Un blocage I/O est laissé au wrapper physique de
groupe, pas à un test de temps que le blocage empêcherait d'exécuter.

Le système de signaux réel, les I/O noyau non interruptibles, l'arrêt de
descendants/du distant et les300s ne sont **pas qualifiés par lecture ou CPU**.
Il faudra une recette/GO runtime distincts pour les garanties réellement
revendiquées, pas un résultat fictif dérivé des simulations ci-dessous.

## Trois probes courts sur le vrai CLI

`outils/test-fs-export-boundaries-v2.py` ne réécrit aucune source. Il accepte un
SHA exact fourni en argument, copie les octets et exécute __main__ via runpy.
Le watchdog, le parent et le transport sont doublés ; aucune lecture proc réelle
ou exécution distante. Le temps réel n'est pas remplacé.

1. Même assertion d'ordre que le rouge EC4 : args/hash/JSON réels, ARM simulé,
   sentinelle AVANT ouverture log, wrapper avant frontière. Aucun OS timer.
2. Parent simulé déjà vieux de40s : le corps ARM doit porter une durée<=237s
   (285−40−8), pas repartir à300. Seule l'expression de durée du corps distant
   est lue par AST ; le corps n'est jamais exécuté. Ce sont des métadonnées de
   parent simulées, pas une horloge virtuelle ni une mesure de300s.
3. Après lancement d'un enfant simulé, erreur OSError contrôlée au fsync du
   ledger local jetable : aucun ACK ; enfin terminate/wait de ce même enfant
   simulé. Vrais petits writes temporaires, aucun signal réel. L'erreur est
   capturée comme faute injectée ; un setup raté devient ERROR et impose STOP.

Toutes les observations sont persistées avant les assertions. Le SHA de chaque
source est contrôlé et les sources047/fixe restent séparées.

Proposition séquentielle pour conserver des preuves discriminantes : sous
**GO CPU60 distinct**, jouer seulement `--case new-probes` sur047111, attendu
deux FAIL (durée300 au lieu du reste ; absence cleanup après fsync), zéro ERROR.
Le rouge d'ordre déjà vu n'est pas rejoué. Puis nouveau GO distinct pour
`--case all` sur la source fixe, attendu trois PASS ; **aucun de ces appels n'a
encore été joué**. Aucun retry implicite sur ERROR/PASS inattendu/timeout.

Cette qualification CPU ne vaudra que pour les raccords/arguments/décisions.
Elle ne consommera ni ancien Q de qualification ni T3552/source8b ni corpus.
Nouveaux pins, revue ciblée puis GO sont requis avant chaque exécution.

## Pins de préparation

Q privé `/home/lucied/.cache/rag3weaver-build/cx-lucivy-fs-export-boundaries-17nov-v2`, UID1000/dev57/inode96299555.

- Baseline : `0471118fe427d8e5a52685d02c6f30c51ae5ffa2b91937f78e2caa86adf63f6f`.
- Fixe proposée : `51a9b1e2cb6f3048580588ebc9dfa6d837dadfc2a011d2b78d99f4e380b268f8`.
- Probes : `c77827e75d39f1e9191b99fccca07089e20e6d1338df3e808250239d83ef5900`.

Python absolu de préparation : `/usr/bin/python3`.
[Marqueur](preuves/watchdog-v2-preparation/preparation.json). Trois sources
figées, SHA et AST seuls vérifiés ; aucun import/runpy/test. Logs/preuves de
run absents, nouveau paquet PC v2 non créé. Ancien Q toujours intact.
