# 8792 : dix rouges causaux observés, avant correction du moteur

GO C7-CP-GUARDS-RED-8792-17nov60 publiédd1e21a57a. UNE invocation dans le cwd
scellé, source8792769e55844113ac2a33ad48da94bebd072541 propre ; onze SHA et
Python recontrôlés avant entrée. Aucun ancien GO réutilisé.

```
timeout --signal=TERM --kill-after=2s 58s /usr/bin/python3.14 -B test_publication_guards_grouped.py
```

**10 FAIL aux dix causes attendues, 0 ERROR/skip, rc1**. Unittest0,094s,
enveloppe externe0,26837376900948584s. Le FAIL clock de starting atteint
publication_relay.py:761, la comparaison procstart14000<=queue9300 ; la
cohérence preparedSHA/Gate.pin est franchie. L'ancien CA1C reste distinct :
neuf motifs et un FAIL de fixture, jamais requalifié après son exécution.

| Cas | Motif brut observé |
|---|---|
|Common et watchdog phase|1750 demandé >1708|
|Final local hardEND|TimeoutError attendu non levé|
|Wrapper avant import|Sentinelle exacte moteur non épinglé exécutée|
|Gate avant import|Sentinelle exacte moteur non épinglé exécutée via Gate|
|Pilote legacy avant import|Sentinelle exacte legacy non épinglé exécutée|
|Pilote guard avant import|Sentinelle exacte physical_guard non épinglé exécutée|
|Offset procstart/restant whole|1750 demandé >368|
|Offset procstart/date de file|Comparaison starting761, pas refus read_bootstrap223|
|RPC et grâces de récolte|1,3999999999996362 >0,40000000100000005|
|Stop et échéance originale|Paramètre deadline absent, FAIL nommé|

Aucune suite36/3/8 rejouée. Aucun processus testé ou SSH réel pendant les dix
cas : observations, temps et Popen étaient des fixtures, les entrées/helpers
étaient les vrais fichiers contrôlés par les scénarios. Aucun acteur, natif,
Cargo, target, corpus, Q global ou service activé.

## Preuves deux hôtes AVANT tout correctif

Archive exclusive `~/.cache/codeparsers-notes/publication-guards-red-8792/`,
dix-sept fichiers, mêmes SHA/tailles aux deux hôtes, aucun fichier supplémentaire
au manifeste hors manifeste lui-même. Source/pins/autorisation/argv/rc/temps et
RAW sont conservés. Les seuls SSH/scp ont copié/réhashé ces preuves.

| Preuve | SHA256 |
|---|---|
|manifest.json|f7c622667ff7552402843e35a96fb79337defab907b68eb66c29f781cc86d074|
|command.json|03cc41d67b3553e4ef4e9d38a831621b1851f1b7eec7409e1573912f607abef2|
|result.json|2aad81d4b8b43cd7b571a7c0902b7b2b60d27d5f0932b730c16ad3bcc05bd5b5|
|stderr.raw|1dc7e11b51c4417b7917f20018a426b86cddfb800539388f841b8fba43076df8|

La préparation source corrective groupée conditionnelle est désormais ouverte,
sous le même ticket et sans GO vert. Avant correction : renforcer le témoin de
première cause RPC, selon la nuance TTY. Une erreur de récolte ne doit pas
remplacer le TimeoutExpired original de communicate ; l'absence non constatée
reste UNKNOWN avec erreur secondaire séparée. Le futur correctif, ses pins et
sa revue totale précèdent toute nouvelle exécution. Aucun vert ni qualification
LIVE/global/OS n'est acquis par ces dix rouges.
