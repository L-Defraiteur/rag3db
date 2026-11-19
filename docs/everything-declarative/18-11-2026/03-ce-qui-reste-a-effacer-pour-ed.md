# Ce qui reste à effacer pour ED : la liste pour Lucie

18 et 19 novembre 2026, session ED. Les worktrees d'ED sont déjà retirés par `git worktree remove`, après contrôle
(propre, tête sur une branche distante, aucun processus). Ce qui suit, ce sont des dossiers : le `rm -rf` m'est
refusé, par la garde de Claude Code puis par le classifieur. **Rien ne s'efface avant le oui de Lucie.** Le oui
du 18 au soir couvre déjà les deux premiers groupes. Les chemins sont littéraux, et tous sont sur la tablette.

## 1. Les preuves et le ménage du harnais de Codex pour ED (environ 19 Go, oui de Lucie le 18 au soir)

Le produit (ex æquo, jeton Vertex) est sur master, et ses manifestes sont dans les branches distantes, qui restent.

- `~/.cache/rag3weaver-build/cx-ed-*` : 11 Go pour `cx-ed-vertex-cleanup-contract-v1`, 3,7 Go pour
  `cx-ed-cleanup-6309-exec-local-v1`, 927 Mo pour `cx-ed-ties-cleanup-archives-20261116.tar`, et une soixantaine de
  petits fichiers `.py`, `.json` et `.tar`.
- `~/.cache/rag3weaver-build/cx-preuves-ed-*` : 1,4 Go pour `cx-preuves-ed-vertex-token-union-red-v1-local`, quatre
  dossiers `cx-preuves-ed-vertex-token-{red,green}-v1-{local,before-tests}` d'environ 188 Mo chacun, et
  `cx-preuves-ed-read-scope-r3-rouge-local` et `-r4-local`, environ 235 Mo chacun.
- `~/.cache/rag3weaver-build/ed-navigateur` (448 Mo) : les journaux de la mesure navigateur du 13 novembre.

## 2. Les dossiers des tâches mortes de l'atelier (oui de Lucie le 18 au soir, par c7)

Leurs arbres `code` sont retirés. Il reste leurs bases et leurs `arbre`. Les `arbre` sont des worktrees du dépôt
d'essais de c7 (`~/.config/rag3weaver/code-essais-c7`), qui n'a pas de dépôt distant : c'est à c7 de les retirer, par
`git -C ~/.config/rag3weaver/code-essais-c7 worktree remove --force <tâche>/arbre`, avant le `rm`.

- `~/.local/share/rag3weaver/taches/e4d8635ecfe7/journaux-annulation-tmp`
- `~/.local/share/rag3weaver/taches/e4d8635ecfe7/journaux-annulation-tmp-2`
- `~/.local/share/rag3weaver/taches/e4d8635ecfe7/journaux-annulation-tmp-3`
- `~/.local/share/rag3weaver/taches/e4d8635ecfe7/racine-disparue`
- `~/.local/share/rag3weaver/taches/e4d8635ecfe7/readme-nouveautes-novembre`
- `~/.local/share/rag3weaver/taches/e4d8635ecfe7/tests-bases-dans-tmp`
- `~/.local/share/rag3weaver/taches/e4d8635ecfe7/undo-json-fichier-efface`

L'hôte garde au registre de `/etat` les backends de ces tâches, fermés. Un appel qui en viserait un le rouvrirait ;
dossiers effacés, ils y restent avec un manifeste absent, jusqu'au prochain redémarrage de l'hôte.

## 3. La base, la config et les targets morts d'ED (à demander à Lucie)

- `~/.local/share/rag3weaver/code-wt-react5.checkpoints`, `code-wt-react5.rag3db`,
  `code-wt-react5.rag3db.extensions`, `code-wt-react5.rag3db.fts` et `code-wt-react5.rag3db.readers` (2,3 Go) :
  la base de l'index du worktree wt-react5, retiré le 18 (le lot juge-genres-2 est posé).
- `~/.config/rag3weaver/code-wt-react5` : son arbre inscrit. **L'inscription d'un arbre, c'est ce dossier** : sans
  argument, `check-manifests` lit `~/.config/rag3weaver/*/templates/backends/code/*.json`
  (`code_tree::Roots::registered_manifests`). Aucune route de l'hôte ne l'inscrit ni ne le désinscrit. Tant qu'il
  est là, `check-manifests` le refuse, sa racine étant absente. L'effacer le désinscrit, et rien d'autre ne l'ouvre.
- `~/.cache/rag3weaver-build/flux` (7,7 Go), `vrai` (6,3 Go), `pese` (3,4 Go) et `pese-c` (5,2 Go) : les targets du
  13 novembre de l'atelier (ide-par-le-flux, flux-partage-entre-onglets, atelier-dire-vrai) et d'une pesée. Leurs
  lots sont posés et leurs worktrees retirés.
- `~/.cache/rag3weaver-build/vrai-c` (15 Go) : le target de wt-vrai, retiré.
- `~/.cache/rag3weaver-build/I-luciepc` (13 Go, **sur la tablette seulement**) : une copie des preuves de Codex du 11
  au 13. **L'original, à luciepc, est à garder** : il tient le dernier vert prouvé du gros paquet défait.

## Ce qui reste vivant (ne pas toucher)

- Mes worktrees en cours : `wt-docs-ed`, `wt-define-ref`, `wt-accord` et `wt-access` ;
  `wt-release-errors` et `wt-llama-local` partiront après leur pose.
- Les targets `target-ed-navigateur` et `target-ed-bissect` (tablette).
- À luciepc : `target-ed-bissect`, `-b`, `-c`, `target-ed-26`, `target-ed-def` et `target-ed-tty`, ainsi que les
  worktrees `wt-bissect-tx*`, `wt-pile-ed-pile-1`, `wt-define-ref*`, `wt-method-kind*`, `wt-first-sync*`,
  `wt-llama-local`, `wt-access` et `wt-release-errors`. Ils sont à retirer après la pose de leurs lots ; je le ferai
  par `git worktree remove`, et je dirai ce qui reste.
