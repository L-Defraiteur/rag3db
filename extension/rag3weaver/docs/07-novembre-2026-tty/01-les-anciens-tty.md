# Les anciens tty de RagForge

7 novembre 2026, session « tty ». Recherche en lecture seule dans `~/LR_CodeRag` sur luciepc et dans l'historique de `LuciformResearch/ragforge` sur GitHub.

## Ce qu'il y a

| Où | Quoi | Date | Verdict |
|---|---|---|---|
| luciepc `~/LR_CodeRag/ragforge/packages/cli` | `@luciformresearch/ragforge-cli` 0.3.3 : déclare `ink` 5, `ink-spinner`, `ink-text-input` et `react` 18, mais il ne reste aucun `.tsx` dans `src/` (commandes daemon, mcp-server, agent seulement) | sources de déc. 2025 à janv. 2026 | rien à reprendre |
| GitHub ragforge, commit `af29477c1` (« agent terminal debugging », 9 déc. 2025) | le TUI ink : `src/commands/tui.ts`, `src/tui/App.tsx`, `components/layout/{Header,Footer,MainContent}.tsx`, `components/shared/InputPrompt.tsx`, `hooks/useAgent.ts` ; retiré de `main` par la suite | 8-10 déc. 2025 | historique seulement : Node/ink, on ne le reprend pas |
| luciepc `~/LR_CodeRag/ragforge/LucieCode` | une copie de gemini-cli (Node, ink) renommée : `.luciecode/`, `.lucieignore`, plus `Table.tsx`, `LogoutConfirmationDialog.tsx` et `terminalCapabilityManager.ts` | sans dépôt git | rien à reprendre ; les `wittyPhrases` sont identiques à celles de gemini-cli |
| GitHub ragforge, commit `a25f3ecf9` (10 déc. 2025), `docs/project/2025-10-dec-18h10/` | **le design démoniaque** : `Agent_terminal_beautification.md` (la page HTML « Daemon Code Summoning »), `AGENT_TERMINAL_BEAUTIFICATION.md` (782 lignes : correspondance entre outils et animations, adaptation à ink), `beautification-roadmaps/ROADMAP_ANIMATIONS.md`, `ROADMAP_INGESTION_ANIMATION.md` | 10 déc. 2025 | **c'est ce qu'on garde, le design seulement** |

La page HTML n'existe plus nulle part sur luciepc. Elle n'est plus que dans l'historique git de ragforge, au commit `a25f3ecf9`.

## La page « Daemon Code Summoning »

Fond `#050505`, texte rouge `#ff3333`, halo `text-shadow: 0 0 5px #ff0000, 0 0 10px #8b0000`, police Courier. Trois animations d'attente sur une seule ligne :

1. **Le Cercle**, toutes les 150 ms : les runes `᚛ ᚨ ᛒ ᛟ ᚜ ⸸ ‡` tournent autour d'un pentagramme `⛧` fixe.
   `⟪ ᚛ᚨᛒ ⛧ ᚜⸸‡ ⟫ INVOCATION DU CODE...`
2. **Transmutation**, toutes les 200 ms : un symbole alchimique change au centre (`⍟ 🜂 ☿ ☉ ♄ 🜄 ∮`).
   `⁅ ⸸ ⁆—[ ☿ ]—⁅ ⸸ ⁆ ANALYSE EN COURS...`
3. **Le Glitch**, toutes les 100 ms : des symboles mathématiques tirés au hasard (`∇ ∫ ∃ ∀ ∴ ∵ ⊕ ⊗ ⌇`) et un libellé tiré lui aussi au hasard.
   `[∇ ⛧ ∫] ...COMPILING CURSE...` · `❬∃∀ ⛧ ∴❭ ...SUMMONING COMPILER...` · ` ⊕ ⸢⛧⸣ ⊗  ...PARSING SOULS...`

Le document de 782 lignes associait l'intensité de l'animation au genre d'opération :

| Opération | Animation |
|---|---|
| lecture (`read_file`, `list_directory`) | Cercle, calme |
| recherche (`grep_files`, `brain_search`) | Transmutation, modérée |
| écriture ou exécution (`write_file`, `edit_file`, `run_command`, `delete_path`) | Glitch, intense |

Le rouge passait par `chalk.redBright` ; le halo n'existe pas dans un terminal.

## Ce qu'on en retient

- Lucie, 7 nov. : on ne reprend pas le code. On part d'un projet Rust open source, et on ne garde de l'ancien que **le design de l'effet de chargement**.
- L'effet tient sur une seule ligne. Il se redessine sur place avec `\r` et un effacement de fin de ligne, sans plein écran, ce qui convient bien à ssh : environ 30 octets toutes les 100 à 200 ms.
- À vérifier avant de le reprendre :
  - **La largeur des glyphes.** `🜂` et `🜄` (plan astral, U+1F702/4) et `⛧` sont mesurés différemment selon les terminaux et les polices, et risquent de faire sauter la ligne. Mesurer sous tmux, mosh, kitty, alacritty et la console Linux. Le cas échéant, retirer ces glyphes du jeu ou prévoir une version ASCII.
  - **Le halo.** Il faut l'imiter : rouge vif sur un fond `#050505` en truecolor, en tombant sur la couleur 196 de la palette 256 puis sur le rouge de base quand le terminal n'a pas mieux.
  - **Le hasard du Glitch.** Il doit pouvoir être fixé par une graine, pour que le témoin soit reproductible.
- La suite, avec le choix de la bibliothèque et la forme du flux, est l'objet de la page 02.
