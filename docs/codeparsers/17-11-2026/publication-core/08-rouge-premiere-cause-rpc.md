# Cause RPC authentique : rouge observé avant correctif groupé

GO C7-CP-RPC-FIRST-RED-F3D0-17nov60/003950c01b, source exacte
f3d049e4f9f3dd13aaf1fc7c05a2f6205e314caf propre. Treize SHA et Python frais
avant UNE commande scellée, aucun ancien paquet rejoué :

```
timeout --signal=TERM --kill-after=2s 58s /usr/bin/python3.14 -B test_publication_rpc_first_cause.py
```

**1 FAIL précis, 0 ERROR/skip, rc1** ; unittest0,010s,
enveloppe externe0,17597999307326972s. assertIs à la ligne44 constate
`TimeoutExpired('fixture-reap', 0.5) is not TimeoutExpired('fixture-ssh', 3)`.
La vraie méthode call reçoit un Popen fictif ; sa récolte en finally remplace
l'objet d'erreur original de communicate. Le motif attendu est donc observé,
pas un TimeoutError générique ni une erreur d'import. Aucun SSH/enfant réel,
suite10/36/3/8, acteur, natif, Q global, target ou corpus lancé.

Archive exclusive deux hôtes :
`~/.cache/codeparsers-notes/publication-rpc-first-red-f3d0/`.
Dix-neuf payloads réhashés, plus leur manifeste = vingt fichiers TOTAL,
aucun extra. Source/pins/autorisation/argv/rc/temps/RAW sont conservés avant
le premier correctif. Les seuls SSH/scp ont copié/réhashé ces preuves.

| Fichier | SHA256 |
|---|---|
|manifest.json|8c2fa5bc8e3b174ee98a827f665c68a3b181208624db29af4b4a82916be7f956|
|result.json|71cb48ee54647567b221b29a720ab74778650e02c2237af15e3c5c9478622352|
|command.json|c7a4b3b0e4ac1dca56648e71edcc63c2349f83471d14660d79433769c403f1db|
|stderr.raw|cb14723fbbbe2e6ceac8247bbd2d48043f73a95eb021671e31e6e173c91a8583|

Le rouge8792 reste celui de dix causes précises, CA1C celui de neuf causes et
un FAIL de fixture ; aucun résultat passé n'est réécrit. Leurs comptes étaient
respectivement17 et16 PAYLOADS, soit18 et17 fichiers avec manifestes.

Préparer maintenant le correctif groupé source sous le même ticket. L'erreur
originale doit rester le même objet, le nettoyage garder son erreur secondaire
et UNKNOWN si récolte non attestée, sans grâces ajoutées hors cap/END. Aucun
vert n'est exécuté ou acquis ; nouvelles sources/pins/RO totale puis GO distinct.
