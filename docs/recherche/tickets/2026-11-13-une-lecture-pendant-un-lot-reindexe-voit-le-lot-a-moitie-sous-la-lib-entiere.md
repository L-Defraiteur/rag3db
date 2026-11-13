# Une lecture pendant un lot réindexé voit le lot à moitié, sous la lib entière

- **État** : ouvert (13 novembre 2026), relevé par archi pendant la passe de son lot (la bulle
  fantôme du chat et « dire vrai ») ; pour recherche (les lectures pendant une réindexation).
- **Famille** : produit
- **Gravité** : passe rouge sans cause dans le lot qui la joue ; et peut-être une lecture qui voit
  un lot réindexé à moitié, si ce n'est pas le seul témoin qui cède sous la charge.
- **Atteignable en service** : à départager.
- **Touche rag3weaver** : oui (`src/backend.rs`, le témoin vers la ligne 7321).

## Ce que c'est

`backend::tests::a_read_during_a_reindexed_batch_sees_the_batch_before_whole` (`a93e98353`,
12 octobre 2026, L3 des lectures) rougit dans la lib entière :

```
panicked at src/backend.rs:7321:13:
assertion `left == right` failed: un lot vu à moitié : {"delivery":{"ok":true,"results":[]},…,
"result":"> Une écriture de l'index est en cours : la réponse porte sur l'état validé, d'avant elle.
# usages: arrivee … ## Définitions (0) (aucune définition indexée sous ce nom) …
  left: false
 right: true
test result: FAILED. 2332 passed; 1 failed; 68 ignored
```

Joué seul, il est **vert deux fois sur deux**, dans le même worktree et le même target.

- **Le code** : master `93cc95a43` (le journal des runs écrit à l'arrêt), plus le lot d'archi, qui
  ne touche que `Chat.tsx` et l'atelier.
- **La lib** : `835d5298f`, sans `MOTEUR_ANCIEN`.
- **Le poste** : lucie-tablette, `POSTE_MEM_MAX=32G poste lourd`, le jeu large.
- **Le journal entier** : `~/.cache/rag3weaver-build/archives/fantome-complete-rouge-*/lib.log`.

## Ce qu'il faut pour le fermer

Le départager : un témoin qui dit trop sous la charge (un seuil, un ordre de fils), ou une
lecture qui voit vraiment un lot à moitié. Puis le correctif ou la borne du témoin, et le témoin
vert dans la lib entière.
