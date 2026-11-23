# Le cumul de jetons d'une conversation ne suit pas d'un pas à l'autre

- **État** : ouvert, le 23 novembre 2026
- **Famille** : produit
- **Gravité** : basse : la vue écrirait un cumul faux (celui du dernier pas seulement)
- **Atteignable en service** : oui
- **Touche rag3weaver** : oui (`src/chat.rs` `ChatSession.tokens`, `src/daemon_host/chat.rs`, l'enregistrement de la session d'un pas)
- **Pour** : archi
- **Signalé par** : archi, en lisant l'essai `~/essai-3` de Lucie, le 23 novembre 2026

## Ce qui se passe

`LlmCall.conversation_tokens` doit cumuler depuis le début de la conversation. Dans l'essai, deux pas successifs
disent `input` 10 179 puis 10 307 (le second n'ajoute pas le premier), et la conversation enregistrée
(`chats/9d2394394ab4.json`) garde `tokens` à 0. La session sur laquelle l'agent d'un pas d'équipier compte n'est pas
celle que l'hôte enregistre ensuite : le cumul repart de 0 à chaque pas.

## À faire

Faire porter le cumul par la session que l'hôte enregistre pour un pas, et le recharger au pas suivant. Témoin : deux
pas successifs d'un équipier, le second `LlmCall` dit la somme des deux, et le fichier de la conversation la garde.
