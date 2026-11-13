# La démo Node.js embarque encore par 7981, un service arrêté

- **État** : ouvert (13 novembre 2026, relevé par archi, à la demande de c7) ; pour embarquement
- **Famille** : produit
- **Gravité** : gêne — la démo attend un service qui ne répond plus ; ses embarquements partent
  sur les deux autres, après les essais vers 7981
- **Atteignable en service** : oui, à qui lance la démo sans poser `RAG3WEAVER_EMBED_SERVICE`
- **Touche rag3weaver** : oui (`extension/rag3weaver/bindings/nodejs/demo/demo.sh`)

## Ce que c'est

`bindings/nodejs/demo/demo.sh` donne pour défaut `RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7979,127.0.0.1:7980,127.0.0.1:7981`
(lignes 11 et 19). Le port local 7981 mène, par le tunnel ssh, à luciepc:7880 (granite-107m),
arrêté le 12 novembre au mot de Lucie : la connexion n'aboutit pas. Le défaut est à ramener à
`127.0.0.1:7979,127.0.0.1:7980`, commentaire de la ligne 11 compris.

Hors du dépôt, l'unité `rag3weaver-tunnels.service` ouvre encore 7981 : c'est la configuration de
Lucie, c7 le lui dit ; ce ticket ne la touche pas. Les annexes de docs du 3 octobre qui citent 7981
sont historiques et restent.
