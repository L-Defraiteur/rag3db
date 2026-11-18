# S2 : le pas suivant voit ce que le pas d'avant a fait — le 18 novembre 2026

Branche `pas-suivant-voit`, worktree `~/.cache/rag3weaver-build/wt-delegate`. Ticket :
`docs/rag3weaver-archi/tickets/2026-11-16-le-pas-suivant-ne-voit-pas-les-retours-d-outils.md`.

## Ce que dit le code

- **Chaque pas repart d'un fil neuf** : c'est voulu depuis le 8 novembre, pour borner les jetons.
  Le message de frontière (`frontier_message`) ne porte que la demande, la boîte et les `step_done`
  d'avant. Les retours d'outils du pas N sont perdus au pas N+1.
- **L'empreinte de lecture** (`empreintes.rs`) vaut pour le tour courant et le précédent. Chaque
  pas est un tour de la session de la tâche. Une lecture du pas N couvre donc déjà un
  `replace_lines` du pas N+1 : le ticket le demandait, le témoin le vérifiera sans changer cette
  règle.

## Conception, en quatre lignes

1. **Le pas note son travail** dans `Step::note_tool`, sans rien relire :
   - les lectures, avec leur chemin et leur plage (`offset`, `length`) ;
   - les écritures (`write`, `replace_text`, `replace_lines`, `insert_lines`, `edit_file`), avec
     leur chemin ;
   - la dernière commande (`run_command`), avec la fin de sa sortie (1 500 caractères au plus).

   Le dernier texte du modèle vient de la réponse du tour.
2. **La frontière du pas N+1 porte un bloc `Previous step`**, en anglais, borné par
   `teammate.carry_over_chars` (clé neuve de `TeammatePolicy`, défaut 8 000). Le bloc dit les
   fichiers lus, puis les fichiers écrits, puis la commande, puis le texte. Au-delà du budget, il
   coupe, et il dit qu'il a coupé.
3. **Ce qui ne bouge pas** :
   - le fil neuf par pas ;
   - le témoin des jetons du 8 novembre (le pas 2 reste petit) ;
   - l'empreinte de lecture ;
   - la garde B, `MAX_STALLED`, les accords.
4. **La preuve** : un rouge d'abord sur le faux modèle. Au pas 1, `read_file src/lib.rs`, puis un
   texte sans `step_done`. Au pas 2, la frontière contient `Previous step` et `src/lib.rs` avec sa
   plage, et un `replace_lines` sur ces lignes passe sans relecture. Puis le module, la passe
   complète et e2e_equipier sous Vertex.
