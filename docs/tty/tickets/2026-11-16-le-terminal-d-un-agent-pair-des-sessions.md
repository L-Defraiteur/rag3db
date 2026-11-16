# Ce que le terminal montre d'un agent pair des sessions Claude

- **État** : ouvert. À concevoir après le mode pair d'archi (session tty, 16 nov. 2026).
- **Famille** : produit
- **Gravité** : manque. Lucie veut voir, dans le terminal d'Algareth, qu'il est une session parmi les autres, et avec qui il parle.
- **Atteignable en service** : oui, dès que le mode pair d'archi existe.
- **Touche rag3weaver** : oui (le crate `extension/rag3weaver/tty`).

## Ce que c'est

La vision (`docs/orchestration/visions/16-11-2026-algareth-pair-des-sessions-claude.md`) fait d'Algareth une session parmi les sessions Claude : une socket sous son nom dans `/run/user/1000/cc-socks/`, visible par `ListAgents`. Archi conçoit ce mode (« le mode pair », par l'hôte ou un `rag3weaver-backend pair`). Ce ticket dit ce que le terminal doit en montrer ; les messages eux-mêmes sont l'autre ticket du 16 nov. (`2026-11-16-les-messages-entre-sessions-dans-le-terminal.md`).

## Ce qu'il faut faire, une fois la route d'archi connue

- **L'en-tête** (à droite de la ligne d'état, `Vue::entete`) dit que l'agent est inscrit comme pair, et sous quel nom de socket (par exemple « pair : algareth »), ou rien s'il ne l'est pas.
- **`/peers`** (nom anglais) liste les sessions que l'agent voit, par la route que l'hôte donnera (à demander à archi, sur le modèle de `GET /ide/teammate/models`) : leur nom, leur état, la dernière fois qu'il leur a parlé.
- **Une perte du bus se dit** sur la ligne d'état, en phrase, jamais en mot nu : « le bus des sessions ne répond plus : les messages attendent ».

## La preuve

Des tests de la vue et de `commandes.rs`, rouges d'abord, sur un faux hôte qui sert la route ; une séance du harnais.

## Taille et blocage

4 à 6 heures. Bloqué par le mode pair d'archi : ne rien commencer avant sa page de conception, puis caler les noms de route et de champs sur elle.
