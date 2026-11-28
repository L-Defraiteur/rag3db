# Un protocole d'agent et ses adaptateurs (ACP)

- **État** : ouvert (28 novembre 2026, orchestration).
- **Gravité** : fonction neuve — l'IDE, le site et les graphes ne savent parler qu'à notre équipier.
- **Atteignable en service** : non (rien n'existe encore).
- **Touche rag3weaver** : oui (l'hôte, l'IDE, les nœuds agents).
- **Famille** : produit
- **Pour** : à attribuer par l'orchestration (l'hôte et l'IDE : rag3weaver archi et everything declarative).
- **Signalé par** : Lucie, 28 novembre 2026 : « de toute façon c'est à faire génériquement, pas uniquement pour Claude ;
  Claude Code doit implémenter un adaptateur ».

## Le besoin

Afficher nos propres interfaces (le chat de l'IDE, celui de notre petit site) pour tout agent de code, et faire de
chaque agent un nœud de nos graphes, que ce soit notre équipier, Claude Code, Codex, Gemini CLI, opencode ou un autre.
Chaque agent a son adaptateur ; l'interface et les graphes ne parlent qu'au protocole.

## Ne pas l'inventer : l'Agent Client Protocol

L'[ACP](https://zed.dev/acp) (Zed, depuis août 2025, avec Google pour Gemini CLI) est un protocole ouvert entre un
éditeur et un agent de code : messages et morceaux en direct, appels d'outils et demandes d'autorisation, diffs, plans,
commandes. Relevé du 28 novembre (sources de seconde main, à vérifier sur le dépôt) : Claude Code par l'adaptateur de
Zed, Codex CLI, Gemini CLI, Augment, Qwen Code et d'autres (25 à 50 agents selon les sources) ; un registre d'agents
depuis janvier 2026 ; un premier brouillon de la version 2 le 20 juillet 2026 ; licence Apache-2.0 annoncée ; une crate
Rust `agent-client-protocol` ([docs.rs](https://docs.rs/crate/agent-client-protocol/0.3.0/source/README.md)).

## Ce qui est demandé

1. **Côté client** : l'hôte parle ACP aux agents ; l'IDE et le site affichent ce flux avec nos vues (messages, outils,
   diffs, plans, demandes d'accord en cartes oui / non). Tout agent compatible se branche sans code neuf.
2. **Côté agent** : notre équipier s'expose aussi en ACP, utilisable dans Zed et les éditeurs compatibles.
3. **Les capacités déclarées** par agent (commandes `/`, mode plan, choix du modèle, veto avant écriture, reprise de
   session) : ce qu'un agent ne sait pas faire reste dans l'interface, désactivé, avec une phrase qui dit pourquoi.
4. **Un nœud agent générique** dans les graphes : il parle ACP, quel que soit l'agent derrière ; les tours qu'il produit
   entrent dans la mémoire, le journal et le récapitulatif du matin comme ceux de l'équipier.
5. **D'abord une étude** : l'état réel d'ACP (version, crate Rust, licence au dépôt), l'adaptateur de Claude Code, et ce
   que le flux donne vraiment (veto avant écriture, usage et coût par appel, compaction).

## Les règles d'Anthropic pour Claude Code dans notre produit

Lues le 28 novembre sur la [page du SDK de Claude Agent](https://code.claude.com/docs/en/agent-sdk/overview) :
- **Clé d'API obligatoire** : « sauf accord préalable, Anthropic ne permet pas aux développeurs tiers de proposer la
  connexion claude.ai ni ses limites d'usage dans leurs produits, y compris les agents bâtis sur le SDK » ; un produit
  passe donc par une clé d'API (au jeton, ou Bedrock, Vertex), jamais par l'abonnement de l'utilisateur.
- **Le nom** : interdit d'appeler l'agent « Claude Code » ou « Claude Code Agent », ni d'en imiter l'apparence ;
  permis : « Claude Agent », « Claude » dans un menu déjà intitulé « Agents », « <notre nom> Powered by Claude ».
- **Les conditions** : les Commercial Terms d'Anthropic, y compris pour des produits offerts à nos propres clients.

Une équipe qui lance elle-même Claude Code sur ses postes, avec ses places, et y branche notre MCP et nos hooks reste un
autre cas : c'est son usage de Claude Code, pas notre produit qui l'embarque ; à confirmer dans ses propres conditions.

## Le témoin

Par un vrai hôte, rouge d'abord : un faux agent ACP (un petit binaire de test) branché à l'hôte ; un message envoyé par
le chat de l'IDE arrive à l'agent, sa réponse en morceaux, un appel d'outil et sa demande d'accord reviennent dans le
fil ; un refus de l'accord bloque l'écriture ; le tour entre dans le journal de la tâche.
