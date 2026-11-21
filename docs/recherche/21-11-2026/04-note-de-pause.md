# Note de pause, 21 novembre 2026, 15 h 3x

Écrite avant le redémarrage de la tablette (sortir /tmp de la RAM), à la demande de c7.

## Où j'en suis

- **Atterri aujourd'hui** :
  - l'accueil vide ;
  - cargo dans l'atelier ;
  - « Ouvrir un dossier » ;
  - le vocabulaire git ;
  - la réinscription MCP ;
  - le confineur à faute injectée ;
  - le juge de la suite à faute ;
  - la page Poste ;
  - la capture des témoins du flux ;
  - « atelier » quitte l'écran ;
  - **l'accueil en arbre des agents** (82f63539f9) ;
  - les pages de conception.
- **2b-C** (une session cherche dans son worktree) : mon témoin de bout en bout est rouge du côté de l'écriture.
  - La couche de la vue ne s'écrit qu'à `open_view`.
  - arbre l'a repris pour son lot (a)/(b), après `sandbox-roots`.
- **Les ratés du MCP** (`~/.local/state/rag3weaver/mcp-misses.tsv`, je le tiens) : trois lignes.
  1. L'hôte bâti sans compteur de jetons, la nuit.
  2. La fraîcheur d'une réponse sous une vue, qui dit la racine du dépôt principal.
  3. La couche d'une vue qui ne suit pas son worktree.

## Ce qui tourne à luciepc

Rien de moi. À la tablette non plus : toutes mes passes sont finies, et je n'y garde plus aucun target.

## Mes branches et leur tête

| Branche | Tête | État |
|---|---|---|
| `ide-opened-text` | 31439f4d2c | prêt, attesté (8/8), **pas déposé** : son bundle conflicte avec master (un lot de l'IDE atterri entre-temps) |
| `view-search-witness` | 4f965148bf | le témoin rouge de 2b-C, repris par arbre ; à ne pas déposer seul |
| `ide-agents-arbre-4` | d096a10de3 | atterri ; worktree à retirer |
| `recherche-pause-21-11` | cette note | poussée sur sa branche, sans dépôt |

## Ce qui attend un dépôt

`ide-opened-text` (l'ouverture ne dit « git » que pour un dépôt git), avec pour parent `arbre-non-git-folder` s'il est encore en file, sinon sans parent.
- **Avant** : rebaser sur master sous `ide-opened-text-2`, rebâtir le bundle, rejouer `the_opening_says_git_only_for_a_git_repository` et ses voisins.
- **Puis** : attester **à luciepc**. Les bâtis et les passes vont à luciepc depuis le 21 nov. (le disque de la tablette).

## Ma prochaine action, à la reprise

1. **Ne relancer ni validation ni e2e** avant que c7 annonce l'atterrissage de `confiner-poste-classes`.
2. Retirer le worktree `21-11-2026-ide-agents-arbre`, qui a atterri.
3. Reposer et déposer `ide-opened-text`, comme ci-dessus.
4. **La tranche 2 de la page d'un agent** : qui il est, ses vues, ses tâches, la conversation des agents de rag3weaver. Ce que Lucie a ajouté depuis va dans la page suivante, déposée une fois :
   - « ce qui attend une intervention » ;
   - le diff d'une PR ;
   - les sous-agents finis, à relancer ;
   - les racines accessibles sous « avancé » ;
   - la case « pour tous mes agents ».
5. **La carte d'accord de l'IDE**, sous la forme d'archi : une seule carte, `approval_state`, une fermeture `answer`.
6. **La tranche 3** : les vues et ce qui n'est pas poussé, sur `GET /views/diff`. Dire alors à arbre qu'il peut retirer l'alias `/vues/diff`.
7. **Le soir** : relire `mcp-misses.tsv`, en tirer des tickets par cause, et envoyer une ligne à c7.
