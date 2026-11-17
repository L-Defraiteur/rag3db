# Nouvelle préparation : signature Path.open du harnais corrigée

[Ticket FS](../tickets/2026-11-16-l-admission-du-corpus-fs-ne-garantit-pas-la-vue-consommee.md),
[erreur conservée](10-erreur-harnais-probes-v2.md)313cba1e95. Aucun retry.

`outils/test-fs-export-boundaries-v3.py` est une nouvelle copie corrigée du
harnais. Le double expose maintenant `mode='r'` comme Path.open : l'argument
nommé est consommé une seule fois avant appel à real_open. Le scénario de test
extérieur est renommé scenario pour ne pas être masqué par ce paramètre. Les
assertions d'ordre, de durée et de nettoyage ne changent pas.

Exporteurs047111 et51a9b1 restent octet pour octet identiques. Source c77827
défectueuse, logs ERROR et anciennes préparations sont conservés. Ni source
fixe ni tests corrigés n'ont été exécutés. Seuls AST et SHA vérifiés.

[Préparation v3 et pins](preuves/watchdog-v3-preparation/preparation.json).
Q local neuf `~/.cache/rag3weaver-build/cx-lucivy-fs-export-boundaries-17nov-v3`.
Il ne consomme aucun ancien Q, target, natif ou corpus. Le paquet PC v2 n'est
toujours pas créé.

Proposition de suite : RO ciblée du delta de harnais, puis **nouveau GO CPU60**
pour `--case new-probes` sur047111 uniquement, attendus deux FAIL sémantiques,
zéro ERROR. Le témoin EC4 n'est pas rejoué. PASS inattendu, ERROR ou timeout :
STOP sans retry. Bruts/observations/rc/temps/sources à conserver deux postes.
Le GO15B2 est épuisé et n'autorise pas cette invocation.

Après ces deux rouges observés, encore un GO distinct pour trois PASS sur51a9.
Aucun OS timer/kill, arrêt distant ou300s ne sera qualifié par ces simulations.
