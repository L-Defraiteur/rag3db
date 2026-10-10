# rag3weaver pour Node

Le backend rag3weaver (base de graphe, recherche hybride, outils d'agent de
code) en un paquet npm : le binaire natif `rag3weaver-backend` est installé par
le sous-paquet de votre plateforme (`rag3weaver-linux-x64-gnu`, …) et ce paquet
le lance et lui parle en lignes JSON. Une seule API : celle du backend.

```js
const { Backend, prepareManifest, templatesDir, vectorExtensionPath } = require('rag3weaver');
const manifest = prepareManifest(`${templatesDir()}/backends/code/backend.json`, {
  database: '/chemin/base.rag3db', vector_extension: vectorExtensionPath(),
  workspace: { root: '/chemin/du/projet', commands: 'off' },
});
// écrire le manifeste, puis :
const backend = await Backend.open('/chemin/backend.json');
console.log(await backend.describe());
console.log(await backend.call('grep_files', { pattern: 'TODO' }));
await backend.shutdown();
```

Les embarquements : par un service déclaré (`RAG3WEAVER_EMBED_SERVICE=hôte:port`,
ou `embeddings.address` dans le manifeste) ; sans service, un backend en mots
seuls (pas de `workspace.index`) démarre quand même. Les poids ne sont jamais
dans le paquet. Le bac à sable des commandes est Linux seulement (Landlock) :
ailleurs, le manifeste doit dire `"sandbox": {"mode": "off"}` en le sachant.

Épreuve : `npm test` — un dossier vide, trois fichiers, une recherche.
