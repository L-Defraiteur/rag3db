# C9 : trente-six verts constants puis trois contre-exemples rouges

Source exacte c9b84168e83caaf2aa083a53ad14b93da6c4330e propre/publiée, P06prepared
36d0127104ebd8f290b42305b9ee85b557b6a7dedc34cea1b43706f89ee580fd et profil81969
inchangés pendant les deux allocations. D45/P05/Archi229bQ figés.

## Régression constante

GO C7-PUBLICATION-CORE-C9-CPU36-17nov60, UNE invocation depuis rust-probe-relay :
`timeout --signal=TERM --kill-after=2s 58s python3 -B test_relay.py -f -v`.
36PASS/0FAIL/0ERROR/0skip,rc0 ;0,217897s externe/0,084s unittest. Les imports et
patchs visent le vrai module commun ; aucun faux noyau dans les fixtures.

Bruts+manifest copiés et SHA comparés sur les deux hôtes AVANT la seconde prise :
~/.cache/codeparsers-notes/publication-core-c9-cpu36/.

| Preuve | SHA256 |
|---|---|
| manifest.json | 6fc717365b4fcf1d01476fd1e3a6ec458e4420a1d06decd9a5408609e3ee560d |
| command.json | a38ae671206fdeef51cba00d196203b6dc2d88e530b081210d83db93c6c47ce4 |
| stderr | 3c25c4da3408aab76341712a8b36b51f4c8ca95888d8a942647d5b5a81d207a1 |
| result.json | b7b680718c11f97b81218ca3669cf96d7d86fe57ee783c36170cb0215dbdba7e |

## Contre-exemples avant capacité

Conditions36verts+preuves2hôtes et source/pin inchangés vérifiées avant GO
conditionnel distinct C7-PUBLICATION-CORE-C9-RED3-17nov60. UNE invocation depuis
publication-core :`timeout --signal=TERM --kill-after=2s 58s python3 -B test_publication_counterexamples.py`.
3FAIL attendus/0ERROR/0skip,rc1 ;0,122832s externe/0,004s unittest :

- Charge build supplémentaire non refusée par le Relay direct avant ACK.
- RéférenceELF runtime sur simple copie locale sans ACK antérieur acceptée.
- Plan explicite4labels non supporté par le vrai constructeur : TypeError précis
  converti en FAIL nommé EXPLICIT_ORDERED_PLAN_UNSUPPORTED_BY_REAL_BASELINE.

Les deux premiers cas ciblent la nouvelle frontière commune avec un fournisseur
incorrect ; le Gate Rust actuel applique déjà son ordre et son jeu de fichiers.
Le troisième expose une capacité absente, pas une erreur d'import/harness. La
construction des futurs types sera portée sans relâcher labels/ordre/5+1+1+1.

Bruts+manifest2hôtes AVANT tout correctif de capacité :
~/.cache/codeparsers-notes/publication-core-c9-red3/.

| Preuve | SHA256 |
|---|---|
| manifest.json | ce154c521ded16e3fe1e9a8999a0486769f89b84dd2da35b427b76396e63573d |
| command.json | c01e410e9b33016fa22d6a270a91f9166e647ad58263244970e41d45f55beea3 |
| stderr | f2b9aa82310c39ad59d59bc6ea53318367298e7216c25f4244a8c785c4c23a4e |
| result.json | df9b60321fc7a303a95dcd05d2a119c9b4ef4843ebddfa1df9d93e9db0e0447a |

stdout est vide dans les deux lots. Source/commandes/cwd/GO/started enregistrés
avant chacun, rc/elapsed ensuite. Aucun SSH,/proc,signal/processréel par les
fixtures,Cargo,natif,target ou corpus ; scp conservatoire seulement après lots.
Ces preuves sont publiées sur branche de capacité distincte AVANT modification
du code commun. C9/P06 restent une baseline figée. Aucun GO natif ni nouvelle
qualification réelle de l'OS/transport ; capacité4 et WindowEvent non livrés à
cette consignation. Leur code/tests devront être publiés puis relus, avec
nouveaux budgets CPU distincts avant exécution.
