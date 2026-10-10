# rag3weaver pour Node — alpha

**État : `0.0.1-alpha`, Linux x86_64 (glibc ≥ 2.28) seulement.** Le binaire est
publié pour cette plateforme ; Windows et macOS se bâtissent sur nos runners
mais ne sont pas encore livrés. Les modèles d'embarquement ne sont jamais dans
le paquet : ils viennent d'un service que vous déclarez, et sans service le
backend indexe en plein texte seul.

Le backend rag3weaver (base de graphe, recherche hybride mots + sens, outils
d'agent de code) en un paquet npm : le binaire natif `rag3weaver-backend` est
installé par le sous-paquet de votre plateforme (`rag3weaver-linux-x64-gnu`)
et ce paquet le lance et lui parle en lignes JSON. Une seule API : celle du
backend.

```js
const { Backend, prepareManifest, templatesDir, vectorExtensionPath } = require('rag3weaver');
const fs = require('node:fs');

const manifest = prepareManifest(`${templatesDir()}/backends/code/backend.json`, {
  name: 'mon-projet',
  database: '/chemin/base.rag3db',
  vector_extension: vectorExtensionPath(),
  workspace: { root: '/chemin/du/projet', commands: 'off', index: 'code' },
  tools: { run_command: undefined },
});
fs.writeFileSync('/chemin/backend.json', JSON.stringify(manifest));

const backend = await Backend.open('/chemin/backend.json');
const d = await backend.describe();            // outils, capacités, avertissements
await backend.call('index', { confirm: true }); // indexation en fond
const r = await backend.call('search_code', { query: 'où lit-on la configuration ?', options: {} });
console.log(r.presentation);                   // le rendu lisible, section Liens comprise
await backend.shutdown();
```

## Ce qui marche

- Ouvrir un backend de code sur un dossier, lister et lire ses fichiers,
  `grep_files`, `search_code` (balayage des fichiers avant l'index, index
  plein texte + sens après), `usages` et `impact` sur un symbole, `edit_file`,
  `estimate`, `schema`.
- Les embarquements par un service : `RAG3WEAVER_EMBED_SERVICE=hôte:port[,hôte:port…]`
  dans l'environnement, ou `models.embed.address` dans le manifeste
  (modèle `granite-278m`, 768 dimensions, par défaut).
- Sans service : le backend démarre et le dit (`warnings` dans `describe`
  et `index_state`), indexe en plein texte, laisse les vecteurs en dette ;
  la recherche dit « dense signal is not available » et rend ses résultats
  par les mots. Quand un service apparaît, la dette se paie sans réindexer.
- `journal` / `journal_read` (le fil d'un agent), `index_state`, arrêt
  propre, et `mustReopen` sur une erreur qui demande de rouvrir la base.

## Ce qui manque

- Windows et macOS (pas de sous-paquet) ; Linux musl et arm64.
- Les commandes (`run_command`) : le gabarit livré les laisse fermées
  (`commands: 'off'`) ; le bac à sable est Linux seulement (Landlock) —
  ailleurs, le manifeste doit dire `"sandbox": {"mode": "off"}` en le sachant.
- Un modèle d'embarquement local : il faut un service.
- L'API est celle du backend, en lignes JSON, sans couche de confort ; elle
  peut changer avant la 0.1.

Épreuve : `npm test` — un dossier vide, trois fichiers, une recherche, sans
service.
