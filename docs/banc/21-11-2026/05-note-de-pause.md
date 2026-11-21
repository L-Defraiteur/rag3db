# Note de pause du banc

21 novembre 2026, 15 h 26, avant le redémarrage de la tablette par Lucie (consigne de c7).

## Où j'en suis

- Les rejeux de la nuit : jugés, page 01 sur master.
- Le mode auto : pages 02 (Gemini contre Qwen3-4B), 03 (le jeu de 120 cas) et 04 (la taille et la cohérence) sur
  master. Ma recommandation : **Qwen3.5-9B en oui/non** comme premier juge local, ses refus à la carte. c7 l'a relayée
  à Lucie.
- Le nom exact d'ED (`exact-name-by-name`, 7b76c260b6) : jugé **vert** aux deux postes. **Il n'est pas encore sur
  master.**

## Ce qui tourne

**Rien** : rien à la tablette, rien à luciepc. Toutes mes passes sont finies, et aucune n'est sous `poste`.

## Mes branches

- `banc-note-de-pause-21-11` : cette note.
- Les branches de pages `banc-page-rejeux-21-11`, `banc-page-mode-auto-21-11` et `banc-page-qwen35-21-11` ont atterri
  par le pousseur. Elles restent sur le dépôt distant, car aucune branche distante ne s'efface.

## Ce que je tiens

- **À la tablette** : le worktree `~/git_workspaces/banc/21-11-2026-nom-exact` (7b76c260b6, 470 Mo, sans target). Il
  est à retirer à l'atterrissage du nom exact.
- **À luciepc** : les worktrees `~/git_workspaces/banc/20-11-2026-revers-a` (e5f1f8052d) et
  `~/git_workspaces/banc/21-11-2026-nom-exact` (7b76c260b6), leurs targets de 4,8 Go chacun sous
  `~/.cache/rag3weaver-build/target-banc/`, et la base gardée `~/.cache/rag3weaver-build/banc-parite/feuille-lot-1636`.
  Tout cela est à retirer à l'atterrissage du nom exact. Ensuite, dire à G que sa copie du dépôt
  `depot-pour-banc-20261120-2335` peut partir.
- **Les fichiers du mode auto** :
  - à la tablette, `~/.cache/rag3db-banc-notes/autorisation/` (les scripts et les sorties), `qwen35/` (le GGUF
    vérifié, 8,9 Go) et `llama-b12603/` (le binaire Vulkan officiel) ;
  - à luciepc, `~/.cache/rag3db-banc-notes/autorisation/` (les sorties de Qwen3-4B).

## Ce qui attend

- **Le oui de Lucie sur trois téléchargements à luciepc** (liste exacte chez c7) : Qwen3-8B Q8_0 (8,7 Go), Gemma 4 12B
  Q6_K (9,8 Go) et Qwen3-14B Q4_K_M (9,0 Go), pour card2. Ils passent par `poste gpu <Gio>`, avec la commande que G
  donnera.
- **Le choix de Lucie pour Inkling 2 Mini** : la conversion `ggml-org` IQ4_NL (17,6 Go) sur llama.cpp officiel, ou le
  binaire du fork d'Unsloth. Le GGUF d'Unsloth est effacé.
- **L'annonce de l'atterrissage de `confiner-poste-classes`** : aucune validation, aucun e2e avant.

## Ma prochaine action

À la reprise : relire les messages de c7. Au oui de Lucie, télécharger les trois modèles à luciepc, sha256 vérifiée
avant tout chargement, puis les passer sous `poste gpu` avec le protocole de la page 04 (`prompts2.py` et
`llama_logits.py`, copiés à luciepc), Qwen3.5-9B rejoué sur le même moteur. Puis une page 06.
