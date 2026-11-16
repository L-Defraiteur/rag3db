# Attestations de mise à froid par phase — préparation LIVE

Lié au plan Arbre360bcd75db et au contrat260d63d93f. Aucun GO LIVE déduit de cette page.

## Copier sans emprunter le bus propriétaire

Provider copie public18ff0e14e907ff9166261d422fbb0ffff9eb3a95,
SHA84168019beaa26a588fb475ed8dcd95ca2c94395e4752d5343c304f5bee281bd.
À la fenêtre corrective02 il a observé hors Common les deux requêtes réelles,
écrit les stdout unités séparément, répondu puis publié le SHA. Phases versions
f05aabdda454d0fc9d8ca85b184bb6ef3960294a7eef5670560254d8d65c576d et générations
f953b9df3de5681e0e87e541d8d220c6184220644f7fc92e03ed4a1f7c12f7d2.
ObserverPID180268/start117698091 maintenu jusqu’à Common terminé, puis absent,
processusGrc0. Preuves PC `~/.cache/optimiseur-notes/bascule-16nov-927/provider-corrective02-20261116-131125*`.

Cela n’est pas un vert de migration : la portée Common finit ROUGE/code1/restes[]
et nettoye=true. Arbre rapporte versionsrc0 en464s et générationsrc0 en0,43s,
mais le contrôle final refuse469chunks en dettegranite_278m (96778repris+469=97247).
Copie uniquement ; canonical et snapshot froids inchangés selon les audits d’Arbre.
Aucun LIVE, réembarquement, réveil API ou retrait de preuve déclenché par G.

## Variante LIVE, séparée et non activée

Source publique `fd9bcd292a303cc5354f86296c5a96be17d8148f`,
`bascule/provide_live_cold_service_proof.py` SHA
49f2d736e296e0b644356260fb410bba881168163e540b2b0bd7c302cb1eda09.
Installation PC `~/.cache/optimiseur-notes/bascule-16nov-927/live-provider-fd9bcd292a/` :
source, tests, noyau du provider copie8416 et primitive Common9276439 épinglés.
8filets CPU verts aux deux postes, aucun moteur ou service réel dans ces filets.
Le provider copie8416 reste bitidentique et refuse toujours une requête LIVE.

La nouvelle entrée réutilise ses primitives d’observation/autorité ; elle ne
modifie pas ses variables ou ses politiques. Stages stricts :
`live-code-principal-01` puis `live-code-lourd-01`. Requête complète du contrat260,
avec `base` correspondant et `target_kind="live"`. `canonical_base` et `target_base`
doivent être exactement `/home/lucied/.local/share/rag3weaver/<base>.rag3db` ;
`source_snapshot` doit être celui du reçu frais G épinglé par SHA à l’activation.
Le guard est dérivé, pas redirigeable :
`~/.cache/rag3weaver-build/bascule-live-20261116/migration-offline-01/<base>/guard`.

Réponse du contrat260, plus `base`, `target_kind` et `cold_receipt_sha256` ;
`cli_authorized=false` reste explicite. Common réel, ascendance/poste/plafond32,
PID/start/cgroup, nonce, stdout unités et OS sont identiques aux contrôles copie.
Le fournisseur ne lance pas de CLI, n’ouvre pas de base et ne touche pas aux unités.
Il reste vivant après ses deux réponses jusqu’à fin du reçu Common ; borne
observateur3900s maximum (budget Common LIVE3600 + amorçage), pas un reset du budget enfant.

## Commande, uniquement quand les conditions C7 sont réunies

Après qualification de la copie, du retour V2 et des gates propres à la base,
G fournit les arguments exacts. Exemple paramétrique, **non exécuté** :

```bash
B=~/.cache/optimiseur-notes/bascule-16nov-927/live-provider-fd9bcd292a
python3 "$B/provide_live_cold_service_proof.py" \
  --base code-principal \
  --core-script "$B/provide_cold_service_proof.py" \
  --common-script "$B/confiner_validation_927.py" \
  --cold-receipt <reçu-frais-G-exact> --cold-receipt-sha <SHA-exact> \
  --evidence <nouveau-dossier-privé> --seconds 3900
```

Aucune activation LIVE à cette rédaction ; dette469 reste à diagnostiquer et
arbitrer. API/socket PC toujours froides ; portable/embarquement/tunnels préservés.
