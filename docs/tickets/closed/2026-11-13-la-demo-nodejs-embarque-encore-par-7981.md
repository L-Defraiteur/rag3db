# La démo Node.js embarque encore par 7981, un service arrêté

- **État** : corrigé le 14 novembre 2026 (session hotfixs) : le défaut de `demo.sh` est ramené à
  `127.0.0.1:7979,127.0.0.1:7980`, commentaire de la ligne 11 compris
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

## Le correctif (14 novembre 2026, hotfixs)

`extension/rag3weaver/bindings/nodejs/demo/demo.sh` : ligne 19, le défaut devient
`127.0.0.1:7979,127.0.0.1:7980`. Ligne 11, le commentaire dit le même défaut et que 7981
(granite-107m) est arrêté depuis le 12 novembre. Plus rien ne cite 7981 sous `bindings/nodejs/demo/`.

**La démo jouée une fois** au portable, sous `poste lourd`, sans `RAG3WEAVER_EMBED_SERVICE` posé, avec
`TMPDIR` sous `~/.cache` (journal : `~/.cache/rag3db-locks/demo-nodejs/demo.log`) :
- `npm install rag3weaver@next` a installé 0.0.1-alpha.1 ;
- la passe avec service annonce `(127.0.0.1:7979,127.0.0.1:7980)`, indexe (File, Library et Scope :
  texte, vecteurs et relations prêts), puis répond à `search_code`, `usages` et `impact` ;
- la passe sans service tourne aussi ;
- code de sortie 0.

**Une observation hors du ticket** : dans la passe « sans service d'embarquement », `demo.js` retire bien
`RAG3WEAVER_EMBED_SERVICE`, et pourtant les vecteurs y sont prêts à 100 %. L'en-tête de `demo.sh` promet
pourtant « plein texte seul, l'avertissement visible ». Les vecteurs viennent donc d'ailleurs,
peut-être d'une configuration du poste ou d'un modèle local. Je ne l'ai pas départagé ; c'est dit à c7.
