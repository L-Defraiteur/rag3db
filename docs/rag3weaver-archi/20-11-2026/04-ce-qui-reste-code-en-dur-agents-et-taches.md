# Ce qui reste codé en dur : les agents et les tâches — le vendredi 20 novembre 2026

La question de Lucie (21 h 3x) : « à quel point le code, pour des features et non des correctifs,
devient juste l'écriture de gabarits, et à quel point on doit encore hardcoder ? ». Cette page est la
part d'archi, pour la page commune d'ED : le pas de l'équipier, la porte des commandes, l'invite, la
palette, les primitives du socle et le relais du mode pair. Lucie tranche sur la page.

## La ligne, en une phrase

**Reste en Rust ce qui touche le système ou une frontière de sûreté, et ce qui doit tenir quoi que
dise une déclaration. Passe en déclaré ce qui est un texte, une borne, un choix ou une composition.**
Une déclaration peut tout régler, sauf ouvrir une porte que le code garde fermée.

## Domaine par domaine

| Domaine | Déjà déclaré | Encore en Rust | Pourquoi en Rust | Peut passer en déclaré, et ce que ça coûte |
|---|---|---|---|---|
| **Le pas de l'équipier** | Ses bornes (`teammate.*` : appels d'outils et du modèle par pas, contexte, report, attentes, quota, relance d'une réponse coupée) ; le modèle et son raisonnement (`models.llm`) | La boucle (pas, frontière, boîte, plans), le sens de `step_done`, la composition de la demande (« Request / Follow-up »), les constantes `MAX_STEPS` et `MAX_STALLED` | C'est le mécanisme : un état sur disque et des reprises après un arrêt de l'hôte, qu'un gabarit ne saurait tenir de façon sûre | Les deux constantes deviennent des clés de `teammate.*` : une heure. La règle « un pas coupé n'est pas un pas raté » est une politique : une demi-journée (lot 3 du raisonnement borné) |
| **L'invite** | Le nom et la persona de l'agent (`agent.name`, `agent.persona`) | Le message système de l'équipier (`SYSTEM_PROMPT`), celui du chat selon la sorte du backend, les textes de reprise du rangement | Rien ne l'exige : ils ont été écrits là au fil des lots | Tout passe au gabarit, sans perte de garde : un à deux jours, avec le ticket du rangement déclaré. C'est le gain le plus net |
| **Les outils de l'équipier** | Les outils du backend (graphes et outils du gabarit) | `edit_plan`, `step_done`, `report`, `suggest`, `send_to_session` : schémas, descriptions et traitement | Le traitement écrit la boîte et les plans, qui sont un mécanisme | Les schémas et les descriptions passent au gabarit, et le traitement reste un mécanisme nommé : un à deux jours (point 4 du même ticket). Le témoin « une phrase par paramètre » les suit |
| **La porte des commandes** | Le mode (`workspace.commands`), le bac à sable (`sandbox` : environnement, lectures en plus, cible partagée), les accords de l'utilisateur | La sentinelle (l'élévation et l'irréversible toujours refusés, le shell qui demande), les familles de commandes, Landlock, les variables du poste retirées du bac (`RUSTC_WRAPPER`, `SCCACHE_*`) | C'est une frontière de sûreté. Une garde qui dépend d'une déclaration ou d'un modèle pour dire non a une panne de trop (doc de la sentinelle, 30 août) | Les listes blanches de familles peuvent se déclarer, à condition qu'une déclaration ne puisse qu'ajouter des refus, jamais lever les trois interdits : une journée, avec un témoin qui essaie de les lever |
| **La palette par familles** | (à naître) Les familles, la tête, le mode du résumé et de la recherche, le déchargement : déclarés au gabarit dès la conception | Le chargement, `find_tool`, le coût en jetons, le déchargement, `tools/list_changed` | C'est le mécanisme de la conversation avec le modèle | Rien de plus à passer : la conception est déjà déclarative |
| **Les primitives du socle** (`run`, MCP à chaud, fetch, cron, fork, recherche web) | (à naître) Leurs schémas, leurs bornes, le catalogue des serveurs MCP, la politique réseau | Les processus, les sockets, les minuteries, le réseau, le fork du contexte | Ils touchent le système et des frontières : un processus lancé, une adresse locale, un délai | Ce qui se déclare naît déclaré. Le reste est un mécanisme nommé, que le gabarit appelle |
| **Le relais du mode pair** | `teammate.pair`, le nom de l'agent ; (à naître) la table des hôtes pour plusieurs postes | Le registre (`<pid>.json`, au format de Claude Code), la socket, la trame, le relais par pair, le retrait propre | Un format qui n'est pas le nôtre (celui de Claude Code, relevé par essais), et des fichiers d'un autre programme | Rien à passer : seuls les noms, l'activation et la table des hôtes sont des choix, et ils sont déclarés |

## Ce que coûte l'ensemble

Le plus utile, et le moins cher : **les textes**. Ce sont l'invite de l'équipier, celle du chat, les
textes de reprise et les descriptions des outils de l'équipier. Les passer au gabarit prend deux à
trois jours, sans rien relâcher : c'est le ticket du rangement déclaré, points 1 à 4. Ensuite viennent
les constantes du pas et les listes blanches de la porte. Le reste est déjà déclaratif par conception,
ou doit rester un mécanisme.

## Pour les features à venir

Une feature d'agent s'écrit d'abord en gabarit (outil, famille, borne, texte). Elle ne demande du
Rust que si elle touche le système, une frontière de sûreté ou un format d'un autre programme. Dans ce
cas, elle demande un **mécanisme nommé et générique**, que le gabarit appelle et règle. Le socle de
l'équipier (page 02) suit déjà cette règle.
