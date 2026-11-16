# Un pas d'équipier s'épuise en lecture : le plafond de 12 appels

- **État** : ouvert, le 16 novembre 2026 ; à faire par Codex (passation d'archi).
- **Famille** : produit
- **Pour** : archi (passation à Codex)
- **Gravité** : bloquant pour une tâche d'écriture : la lecture épuise le pas, rien n'est écrit (D6).
- **Atteignable en service** : oui, toute tâche qui lit avant d'écrire.
- **Touche rag3weaver** : oui
- **Signalé par** : c7, campagne D6 du 16 novembre 2026 (`docs/orchestration/16-11-2026/01-essais-empiriques-algareth.md`, S1)

## Ce que c'est

`STEP_BUDGET = 12` (`extension/rag3weaver/src/daemon_host/teammate.rs:38`). `Step::gate` compte
chaque appel d'outil de travail et refuse au-delà avec « budget du pas atteint… déclare d'abord
step_done ». La lecture d'une tâche réelle (D6 : la fonction, ses tests, ses appelants) épuise ce
budget ; le pas finit sans avoir écrit, et le suivant repart d'un fil neuf (voir le ticket
`2026-11-16-le-pas-suivant-ne-voit-pas-les-retours-d-outils.md`). En 20 min : 7 pas, 82 appels,
0 édition.

## Ce qu'il faut faire

Remplacer le plafond d'appels par un plafond de **jetons** du pas, réglable, assez large pour une
édition et deux `cargo test` : `teammate.step_budget_tokens` (clé neuve de `TeammatePolicy`,
`src/backend.rs`, en anglais, défaut à mesurer, de l'ordre de 60 000 jetons d'entrée cumulés).
Garder un garde-fou d'appels très haut (`teammate.max_tool_calls_per_step`, défaut 60) contre une
boucle sans fin. Le refus dit lequel des deux est atteint. Ne pas toucher à `MAX_STALLED` ni à la
garde B (`request_without_a_line`).

## Comment le prouver

- Rouge d'abord, sur le faux modèle (`src/daemon_host/teammate_tests.rs`, `CallbackLlm`) : un pas qui
  fait 15 lectures puis un `edit_file` puis `step_done` finit en un pas, et le fichier est écrit.
  Aujourd'hui, le 13e appel est refusé.
- `FILTRE="daemon_host::teammate daemon_host::chat" ETAPE=module` sous `POSTE_MEM_MAX=32G ./poste
  lourd bash <script de passe>` (modèle : `~/.cache/rag3weaver-build/wt-bout-passe.sh`), puis la
  passe complète, puis `e2e_equipier` sous Vertex (variables dans la page de passation).

## Taille et blocage

4 h. Aucun blocage. Se pose après l'atterrissage de `pas-va-au-bout`.

