# Rapport de la session tty, 7 novembre 2026

## Ce qui est fait

1. **L'enquête sur les anciens tty** (page 01).
   - Le TUI ink de RagForge (commit `af29477c1`) et LucieCode, une copie de gemini-cli, ne se reprennent pas.
   - Le design « Daemon Code Summoning » est retrouvé dans l'historique de ragforge (`a25f3ecf9`) ; il n'est plus sur luciepc. Lucie a revu la page HTML (à garder dans le dépôt si elle le veut).
2. **La conception** (page 02), envoyée à c7 et à everything declarative. Ce que Lucie a tranché :
   - la base : ratatui, avec une couche déclarative à nous sur taffy ;
   - les systèmes : Windows, macOS et Linux natifs, donc crossterm ;
   - l'emplacement : le terminal là où est l'hôte, ou à côté de lui par un tunnel ;
   - le design : s'inspirer au plus près de la page HTML, puis un sélecteur de thèmes (fées, forêt, bureau, papillons, Mordor, Dracula au sens vampire).
3. **Le lot tty-1, sur la branche `tty-1-2`.**
   - `rag3weaver-tty --plain` vit dans un crate à part (`extension/rag3weaver/tty`) : 23 crates, sans le moteur.
   - Le flux `/ide/flux` est lu par une connexion TCP à nous, avec un décodeur SSE et une reprise par `Last-Event-ID`.
   - L'adresse et le jeton se lisent d'un fichier, d'une commande, ou d'une adresse de tunnel.
   - `describe()` porte maintenant la `policy` de chaque outil.
   - Le témoin `e2e_tty` est vert ici : 2 tests sur 2. Le flux, coupé après 41 trames, en reprend 261, soit les 302 de la relecture entière, sans perte ni doublon. Avec une reprise privée de son curseur, il rougit bien (0 trame après la reprise).
   - La passe complète tourne à luciepc (`~/.cache/rag3weaver-build/tty-lot1/resume.log`).
4. **Le ticket** de la lib qui ne se bâtit pas sans `rag3db-native` (docs/tickets, 7 nov.).

## Sur master (7 nov. 2026, soir)

| Quoi | Hash | Passe |
|---|---|---|
| **tty-1**, le client `--plain` et `policy` dans `describe` | `cfc5cc8e8` | entière, à luciepc, sur la lib 04063ee06 : e2e_tty 2/2, e2e_lectures 19/19, e2e_code 27/27, e2e_mcp_stdio 4/4, e2e_demon_hote 12/12, check_features 5/5, lib 1955/0, navigateur 13/13 |
| **tty-2**, la vue inline | `1fc1e24bd` | courte (le crate et e2e_tty) : crate 9/9 avec la vue et 4/4 sans, e2e_tty 2/2 |
| **tty-3**, le Daemon Code Summoning, `--depot`/`--tache`, le filtre par worktree | `f66e72d31` | courte, sur la lib 17a094d12 avec `MOTEUR_ANCIEN=1` : crate 21/21 avec la vue et 6/6 sans, e2e_tty 2/2 |
| **le thème commun** `templates/themes/daemon.json`, branché sur le mot de Lucie (8 nov.) | `f2e6e2521` | courte sur dadbf0041 : crate 23/23 et 6/6, e2e_tty 2/2, e2e_catalogue_gabarits 12/12 |
| **la coloration du code** par `POST /highlight` | `3696a71cb` | courte sur dadbf0041 : crate 28/28 et 7/7, e2e_tty 3/3, e2e_catalogue_gabarits 12/12 |
| les lignes de nouveautés | `69aa15eef`, `0a3e5752e`, et celle de tty-3 | docs seules |
| la page d'origine du Daemon Code Summoning et le thème commun | `78f0efeb6`, `422f47d52` | docs seules |

Deux rouges en route, chacun compris avant d'être corrigé :
- **La base trop vieille.** La première passe avait tourné sur une base dépassée par du Rust de master. On a rejoué au lieu de pousser.
- **L'index des tickets.** Après le rebase, la table « produit » annonçait 47 tickets et en comptait 48. On a corrigé en docs seules, puis repris `check_features`, la lib et les témoins navigateur.

Effacé après les lots : les worktrees `rag3db-tty` (ici et à luciepc), les branches locales `tty-*`, et les targets `target-tty`, **6,2 Go ici et 11 Go à luciepc**. Les branches distantes restent. Les journaux de passe restent dans `~/.cache/rag3weaver-build/tty-*` à luciepc.

## Ce qui attend

- **La coloration du code** : faite (`3696a71cb`). Ce qui suit dit comment elle a été décidée (c7, le 7 nov. au soir, sur la piste de la page 07).
  - L'hôte colore par les grammaires tree-sitter de codeparsers et sert des **spans typés** (une route ou un outil `highlight`), les mêmes pour l'IDE et pour le terminal.
  - Codeparsers écrit d'abord une page courte (les requêtes de coloration par langue), puis le code ; highlight.js reste le repli immédiat du chat de l'IDE.
  - Le terminal colore ces spans dans la palette du thème « daemon », par un lot à lui (tty-4 ou tty-5) une fois que les spans existent.

- La suite des lots, dans l'ordre :
  - tty-2 : la vue inline en ratatui avec crossterm ;
  - tty-4 : les états et la boîte ;
  - tty-5 : la mesure sous ssh, tmux et mosh, plus un passage sous Windows et un sous macOS.
