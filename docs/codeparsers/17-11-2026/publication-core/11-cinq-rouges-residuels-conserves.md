# Cinq rouges résiduels exacts, avant correction unifiée

GO C7-CP-RESIDUAL-RED-6F47-17nov60/d10a34f119 ; source exacte
6f471fe97f71b2348dc6ce72fca27439bc68063c propre et dix-huit SHA/Python frais.
UNE commande scellée, sans failfast ni reprise :

```
timeout --signal=TERM --kill-after=2s 58s /usr/bin/python3.14 -B test_publication_residual_counterexamples.py
```

**5 FAIL précis, 0 ERROR/skip, rc1** ; unittest0,036s,
enveloppe externe0,23567038401961327s. Trois erreurs de journal remplacent le
RuntimeError initial ; close remplace l'OSError initial de reap ; le loader
retourne HOST_FROM_UNVERIFIED_SECOND_READ dans B alors que seuls les bytes A
correspondent au pin. Les quatre premières assertions d'identité sont atteintes
avant le champ de secours futur : aucun champ absent n'a fabriqué un ERROR.

Les58cas et anciens jeux8792/f3d n'ont pas été rejoués. Faux IO/processus/temps
et FS privé seulement ; aucun SSH, signal, enfant natif, actor, globalQ, target,
corpus ou service pendant les cas. Les seuls SSH/scp ont archivé ces preuves.

`~/.cache/codeparsers-notes/publication-residual-red-6f47/` : vingt-quatre
payloads et manifeste, vingt-cinq fichiers TOTAL, mêmes tailles/SHA aux deux
hôtes et aucun extra, réhashés AVANT premier diff correctif.

| Fichier | SHA256 |
|---|---|
|manifest.json|68d76af23699734d51df594d9cd71343dfbdb9d6a47f598c141d2e5fc3d1e42a|
|command.json|7177c8447291bdaf30345a67f436d0bdd52240c31493ef1c5d2807c448befd46|
|result.json|778c1434d1c421511536bf0188c97310dc60ca4a40ea30a769fe66566786b2a7|
|stderr.raw|85d8a5ef054af69cf2a7f91955bc6a662295f5db10e2ad0b9b9cf5fb57223357|

Source, autorisation, pins, argv/cwd, outils, rc/temps et RAW sont conservés.
Les preuves CA1C/8792/F3D0 restent distinctes et intactes. Le fix groupé source
est autorisé après ces rouges, pas une validation verte : même snapshot
prepared pour tous les lecteurs, même objet premièrecause, mémoire/secondaires
UNKNOWN même quand un constat ne peut être écrit, cleanup encore tenté. Rien
ne transformera ce secours mémoire en archive ou admission CP/LIVE réussie.
Nouvel ensemble source/pins, revue totale puis GO distinct avant63cas proposés.
