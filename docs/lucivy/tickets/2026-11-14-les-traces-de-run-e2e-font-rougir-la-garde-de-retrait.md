# Les traces de run_e2e.sh font rougir la garde de retrait

- **État** : ouvert (14 novembre 2026). Rencontré deux fois en retirant les worktrees de la montée de
  lucivy 4.3.7, puis une troisième au retrait du worktree de 2a′-1 (everything declarative), avec
  `node_modules` en second artefact ; contourné à la main, voir plus bas.
- **Gravité** : gêne de procédure. Aucune donnée perdue, mais chaque retrait d'un worktree qui a
  porté une passe s'arrête.
- **Atteignable en service** : non (outillage de passe et de ménage).
- **Touche rag3weaver** : oui (`extension/rag3weaver/run_e2e.sh`, `scripts/garde_worktree.py`).
- **Famille** : produit
- **Pour** : hotfixs (il tient la garde de retrait)

## Ce qui se passe

`run_e2e.sh` écrit deux traces **dans le worktree** : `extension/rag3weaver/target/charge-last.tsv`
(« Charge tracée dans … (tail -f pour suivre) ») et `extension/rag3weaver/target/e2e-last.log`. Le
dossier est ignoré par git (`git status --ignored` le montre en `!!`). La passe elle-même bâtit dans le
target de session, `CARGO_TARGET_DIR`, hors du worktree : ce dossier ne sert qu'à ces deux traces.

Au retrait du worktree, `garde_worktree.py` refuse, comme la procédure le veut pour toute donnée
locale ou ignorée non préservée : « fichiers ou index non préservés dans les sources :
<worktree> », verdict ROUGE.

Rencontré le 14 novembre 2026 à luciepc, sur `rag3db-lucivy-437-2` et `rag3db-lucivy-437-4`, les deux
worktrees où `run_e2e.sh` avait tourné ; les quatre autres, sans passe e2e, sont sortis VERT du
premier coup. 52 Ko par worktree.

## Le contournement suivi (préserver, retirer, rejouer)

1. Copier le dossier dans l'archive du retrait : `cp -a <worktree>/extension/rag3weaver/target
   <archives>/<nom>-ignores/`, puis le comparer à la copie avec `diff -rq`.
2. Retirer le dossier du worktree. `git status --porcelain --ignored` ne montre plus de `!!`.
3. Rejouer `garde_worktree.py` avec un rapport neuf : VERT, puis `git worktree remove --force`.

Archives de ce cas : `~/.cache/rag3weaver-build/archives-worktrees/20261114-161730-lucivy/` à luciepc
(les copies `*-ignores`), et `20261114-161801-lucivy/` pour les rapports verts.

## Troisième occurrence, et un second artefact : `node_modules` (everything declarative)

Le 14 novembre 2026 à luciepc, au retrait de `rag3db-ed-navigateur` (le worktree des passes de
2a′-1), la garde a rendu ROUGE avec deux refus :
- les deux traces de `run_e2e.sh` ci-dessus (52 Ko) ;
- **`extension/rag3weaver/ui/ide/node_modules`** (142 Mo), posé par `npm ci` pour le témoin du
  navigateur (`temoins_navigateur.sh`). Ses liens `node_modules/.bin/*` (`esbuild`, `tsc`,
  `tailwindcss`…) sortent en « lien source non suivi ou ambigu ». Il se refait par `npm ci` depuis
  `package-lock.json`, qui est suivi.

Le contournement suivi, au mot de c7 :
1. les deux traces sont déplacées dans le dossier des journaux de la session
   (`~/.cache/rag3weaver-build/ed-navigateur/target-du-worktree/`) ;
2. `node_modules` est effacé ;
3. la garde est rejouée avec un rapport neuf ;
4. `worktree remove --force` sur VERT seulement.

**Un piège de plus dans le contournement** : la copie de l'administration (`cp -a
.git/worktrees/<nom>`) doit se faire **juste avant** la garde. Une commande git dans le worktree
entre les deux, ici un `git status --ignored` pour relever les ignorés, récrit l'index. La garde rend
alors « archive diffère de l'administration », alors même que `diff -r` ne voit aucune différence.
Recopiée juste avant, la garde sort VERT. Les rapports sont dans `ed-navigateur/garde-worktree-rapport-{,2,3}.json`.

Ce qu'on pourrait faire, en plus : les dépendances npm du témoin du navigateur pourraient vivre hors
du worktree, comme le target cargo, par exemple dans un cache de session par lien.

## Ce qu'on pourrait faire

- Faire écrire ces deux traces par `run_e2e.sh` dans le dossier des journaux de la passe, ou sous
  `CARGO_TARGET_DIR`, hors du worktree.
- Ou bien prévoir, dans le script de push ou de ménage, l'étape « préserver les ignorés » avant la
  garde, avec son archive. La garde, elle, garde son refus.

Rien n'est décidé. C'est à hotfixs de choisir, puisqu'il tient la garde.
