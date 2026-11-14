# L'épreuve « sans service » du paquet Node.js hérite des variables du poste

- **État** : ouvert (14 novembre 2026, embarquement, relevé en fermant
  `2026-11-14-sans-service-la-demo-nodejs-embarque-quand-meme`)
- **Pour** : embarquement
- **Famille** : produit
- **Gravité** : témoin qui dépend du poste. Sur un poste qui pose un service, l'épreuve rougit sans
  défaut du paquet, ou embarque en silence. C'est plausible ; ce n'est pas joué.
- **Atteignable en service** : non. Seuls `npm test` du binding et ceux qui le rejouent sont touchés.
- **Touche rag3weaver** : oui (`extension/rag3weaver/bindings/nodejs/test/essai.js`)

## Ce que c'est

`test/essai.js` éprouve le backend de code **sans service d'embarquement**. Il exige l'avertissement
« pas de service d'embarquement » dans `describe` et dans `index_state`, puis le repli dense
(« not available »). Mais il ne retire aucune variable de service de son environnement. Le binaire
lit `RAG3WEAVER_SERVICE_EMBED`, son alias `RAG3WEAVER_EMBED_SERVICE`, et `RAG3WEAVER_SERVICE_<CAPACITÉ>`
pour le creux, le relecteur et l'OCR (`Capability::variables`, `src/model_source.rs`).

Au portable, fish, zsh et `environment.d` posent tous les trois
`RAG3WEAVER_SERVICE_EMBED=127.0.0.1:7979,…`. Un `npm test` lancé là verrait le service. C'est la même
cause que le ticket de la démo, fermé le 14 novembre.

## Ce qu'il faut pour le fermer

- Jouer `npm test` au portable, variable posée, et lire la couleur. Elle n'est pas annoncée sans
  avoir été jouée.
- Retirer de l'environnement du binaire **toutes** les variables `RAG3WEAVER_SERVICE_*` et l'alias,
  comme la démo depuis le 14 novembre (`estVariableDeService` de `demo/demo.js`), rouge d'abord.
