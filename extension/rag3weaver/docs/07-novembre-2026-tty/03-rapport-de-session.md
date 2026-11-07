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

## Ce qui attend

- La passe de luciepc, puis le push du lot tty-1 sur master, puis le hash à c7.
- everything declarative, à son lot 1 : la boîte relayée sur `/ide/flux` (`MailboxLine`). Le client la prend déjà et cesse alors de relire la boîte.
- À lui demander : le `backend` (manifeste) de la tâche dans `GET /ide/teammate`, pour filtrer le flux par tâche. Aujourd'hui `--plain` montre tout le flux de l'hôte.
- La suite des lots, dans l'ordre :
  - tty-2 : la vue inline en ratatui avec crossterm ;
  - tty-3 : le Daemon Code Summoning, le thème « daemon » d'abord ;
  - tty-4 : les états et la boîte ;
  - tty-5 : la mesure sous ssh, tmux et mosh, plus un passage sous Windows et un sous macOS.
