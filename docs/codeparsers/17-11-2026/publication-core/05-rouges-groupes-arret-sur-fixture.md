# Paquet ca1 : arrêt sur un motif de fixture inattendu

GO consommé une fois : **C7-CP-GUARDS-RED-CA1C-17nov60**, publié d72aa02288.
Source exacte ca1c9f6e25aa3906719a31946c146758566f0758, arbre propre,
dix SHA et interpréteur recontrôlés juste avant entrée. Aucun correctif après
cette exécution ; le moteur est toujours celui de558, les anciens témoins figés.

Commande UNE fois, sans failfast, dans le cwd scellé par les pins :

```
timeout --signal=TERM --kill-after=2s 58s /usr/bin/python3.14 -B test_publication_guards_grouped.py
```

Résultat réel : **10 FAIL, 0 ERROR/skip, rc1**, dix cas sélectionnés,
0,048s unittest ; enveloppe externe **0,15039676800370216s**.
Ce n'est PAS le résultat attendu dix rouges précis : neuf motifs correspondent
aux contre-exemples, le dixième est un défaut de raccord de fixture.

## Motif inattendu et arrêt

`test_proc_identity_offset_is_not_a_queue_timestamp` entre bien dans le vrai
Gate.starting, mais l'évaluation de sa comparaison appelle d'abord bootstrap.
`read_bootstrap` refuse à publication_relay.py:223 sur
`record['prepared_sha256'] == pin`. Le helper a écrit le vrai SHA du prepared
et l'a retourné localement ; `base.gate.pin` est resté à `'1' * 64`, sa valeur
de fixture antérieure. La comparaison BOOTTIME/MONOTONIC visée n'est pas jugée.
Le registre unittest classe cette AssertionError en FAIL, pas ERROR ; cette
classification ne transforme pas un mauvais motif en rouge du mécanisme.

Les neuf autres motifs sont : Common1750>1708, absence de TimeoutError au
hardEND final, sentinelles exactes wrapper/Gate/pilote-legacy/pilote-guard,
Common1750>368 avec offset, RPC1,4>0,4 et API stop(deadline) non supportée.
Les tracebacks et leurs valeurs restent bruts. Les sentinelles étaient les
seuls RuntimeError convertis explicitement en FAIL ; aucune autre exception
n'a été reclassée ou effacée. Les suites36/3/8 n'ont pas été rejouées.

Conformément au GO : **STOP, zéro reprise, zéro correction de fixture ou de
moteur**. La préparation corrective groupée conditionnée aux dix rouges exacts
n'est pas ouverte par ce résultat. Rapport/ticket seulement ; cadrage C7
nécessaire pour le raccord de fixture, ses pins/revue et un nouveau GO distinct.

## Conservation avant tout changement

Archives exclusives aux deux hôtes, mêmes seize fichiers et mêmes empreintes,
copiés puis vérifiés avant tout diff documentaire :

`~/.cache/codeparsers-notes/publication-guards-red-ca1c/`

| Preuve | SHA256 |
|---|---|
|manifest.json|41c73731458fd15c1003c378257a38f8a8c326adfb64c02a6c79df2a29874b93|
|command.json|9e27bc538d9eaf5df6ff1aa58912c9697e943bcd8eb732aca3187b62afd29b27|
|result.json|c4fcf91304f1cdaeb29f31983ea8c8cadfa3500d54b19c2b1a91729c2673047c|
|stderr.raw|1e0bba579ef28925f05863263195c5395030eb7ed17f1b27f7e5aaa414804bb9|

Source, pins, autorisation, argv/cwd, interpréteur/timeout, rc et horodatages
sont conservés sans réécriture. Le manifeste couvre chaque fichier sauf lui-même,
aucun fichier supplémentaire. Aucun jeton/environnement partagé lu ou copié.
Les seuls SSH/scp ont servi à la conservation et au rehash des preuves ; aucune
phase distante, compilation, ELF natif, acteur, service, target ou corpus joué.

La nuance TTY sur la première cause RPC reste ouverte : la borne seule et
l'acceptation d'un TimeoutError générique ne qualifient pas cette propriété.
Avant tout futur diff correctif/vert, renforcer sa preuve sans remplacer la
première erreur du RPC par celle du nettoyage. Rien de cette suite n'est joué.
