# Un créneau 2 du pousseur meurt sans laisser de trace

- **État** : ouvert (20 novembre 2026, écrit par G à la demande de c7)
- **Pour** : G (optimiseur)
- **Famille** : produit (l'outillage des passes : le pousseur)
- **Gravité** : basse. Les lots se rejouent au pas suivant, mais une passe est perdue, et la cause reste inconnue.
- **Atteignable en service** : non, le pousseur seulement
- **Touche rag3weaver** : non (`tools/poste/pusher.sh`)

## Ce que c'est

Le 20 novembre vers 22 h 01, le pousseur prend `memoire-b6-secrets` en train A, puis `memoire-extraction-cut` en
créneau 2 (`spec_run`, en fond). A est rejoué vert et poussé à 22 h 16 min 26 s. Le règlement du créneau 2
(`spec_settle`) trouve alors `slot2/result` **vide**, et l'écrit « créneau 2 rouge » avec un motif vide. Ce libellé est
corrigé par le lot `pusher-creneau2-jete` : sans résultat, le créneau est « jeté ».

Du créneau 2 de 22 h 01, il ne reste **rien** :
- aucune ligne « reposé (créneau 2) » ni « créneau 2 jeté » au journal ;
- aucun dossier `pusher-20261120-2201…-memoire-extraction-cut` ;
- le journal de l'unité (`journalctl --user -u rag3weaver-pusher`) est vide sur ce créneau horaire.

`spec_run` est donc mort après avoir vidé `result` (`: > "$SD/result"`), et avant toute écriture de `log` ou de
`printf … > result`. Les chemins qui s'arrêtent tôt, comme le worktree absent, le fetch refusé, une branche qui a
bougé ou une repose qui bute, écrivent tous une ligne et un code.

## Ce qui manque pour trouver la cause

Les fichiers de `pusher/slot2/` (`base`, `result`, `last-repose.log`, `step-lots.tsv`) sont réécrits par le créneau 2
suivant : à 22 h 16 min 34 s, celui de `memoire-template-version` avait effacé ceux de 22 h 01. La sortie d'erreur de
`spec_run` (en fond, `&`) n'est écrite nulle part.

## Le correctif proposé

1. La sortie d'erreur et la sortie de `spec_run` vont dans un fichier par créneau, `pusher/slot2/<STAMP>-<label>.log`,
   gardé jusqu'au ménage.
2. `spec_run` pose un piège `EXIT` qui écrit dans `result` un code « mort » (par exemple `9`) et une ligne au journal
   s'il sort sans avoir écrit de résultat.
3. Témoin rouge d'abord : une passe double qui tue son créneau (`kill` du sous-shell de `spec_run`). Aujourd'hui, rien
   au journal ; attendu : « créneau 2 jeté : mort sans résultat », et le fichier de sortie gardé.
