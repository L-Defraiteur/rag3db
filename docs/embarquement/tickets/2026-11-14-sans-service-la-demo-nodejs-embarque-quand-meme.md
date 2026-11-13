# Sans service, la démo Node.js embarque quand même, sans dire par quoi

- **État** : ouvert (14 novembre 2026, relevé par hotfixs en jouant la démo, à la demande de c7) ;
  pour embarquement, à départager
- **Famille** : produit
- **Gravité** : réponse fausse en apparence. La démo annonce « plein texte seul, l'avertissement
  visible », et rend des vecteurs prêts à 100 % sans dire quel embarqueur les a calculés.
- **Atteignable en service** : oui, à qui lance `bindings/nodejs/demo/demo.sh`, et sans doute à qui
  lance le binaire sans `RAG3WEAVER_EMBED_SERVICE`
- **Touche rag3weaver** : oui (`extension/rag3weaver/bindings/nodejs/demo/`, et l'embarquement du
  binaire si un repli local répond)

## Ce que c'est

La passe « Sans service d'embarquement (plein texte seul) » de `demo.js` retire bien
`RAG3WEAVER_EMBED_SERVICE` (`delete process.env.RAG3WEAVER_EMBED_SERVICE`, ligne 59). Pourtant, son
index rend `File`, `Library` et `Scope` avec « vecteurs ready (100%) », comme la passe avec service, et
aucun avertissement ne s'affiche. Démo jouée le 14 novembre 2026 au portable, avec `rag3weaver@next`
0.0.1-alpha.1. Journal : `~/.cache/rag3db-locks/demo-nodejs/demo.log` au portable.

## Les deux lectures possibles (c7)

1. L'embarqueur local du binaire, le repli par défaut, prend la main sans le dire.
2. L'en-tête de `demo.sh` promet un « plein texte seul » qui n'existe plus.

## Ce qu'il faut pour le fermer

- Départager : quel embarqueur a répondu dans la passe sans service, à lire dans `/sante` de l'hôte
  ou dans le journal de l'index.
- Faire dire à la démo, à chaque passe, quel embarqueur a embarqué.
- Si le plein texte seul n'existe plus, réécrire l'en-tête de `demo.sh` ; sinon, rétablir
  l'avertissement.
