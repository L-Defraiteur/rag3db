# La recherche web compte ses jetons au registre, pas le prix de sa requête Google Search

- **État** : ouvert (22 novembre 2026)
- **Famille** : produit (le registre des dépenses)
- **Gravité** : une dépense sous-comptée. Chaque recherche de l'équipier (`web_search`, Gemini ancré par Google Search sur Vertex) se facture aussi à la requête ancrée, au-delà d'une franchise ; le registre n'en compte que les jetons.
- **Atteignable en service** : oui, dès qu'un équipier cherche sur le web.
- **Touche rag3weaver** : oui (`usage_ledger`, `templates/prices.json`, `daemon_host/web_search.rs`).
- **Pour** : everything declarative.
- **Signalé par** : everything declarative, en faisant passer `web_search` par le registre (lot `llm-logprobs`, 22 novembre 2026).

## Ce qui se passe

`web_search` réserve sa borne avant l'appel, puis solde avec `usageMetadata` : le prompt, le cache, les candidats et la
pensée. Google facture en plus chaque requête ancrée par Google Search, à un prix par millier de requêtes, au-delà d'une
franchise quotidienne. Le registre ne connaît que des jetons : ce prix-là n'y entre pas.

## Ce qu'il faut

- un prix par appel dans un tarif (`request`, en USD par appel), compté par `Tokens::cost` pour chaque appel du slug ;
- un slug à part pour l'appel ancré (`<modèle>+search`), avec son tarif relevé sur la page de Google, daté et sourcé, et
  la franchise dite dans sa note (comptée prudemment : sans franchise) ;
- `web_search` réserve et solde sous ce slug.

## Le témoin

Une recherche web compte, au registre, ses jetons et une requête ancrée, au prix du tarif.
