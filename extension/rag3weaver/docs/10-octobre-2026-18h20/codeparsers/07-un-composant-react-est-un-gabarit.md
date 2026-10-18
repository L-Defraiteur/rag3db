# Un composant React est un gabarit : la marche 3 de l'IDE, étudiée

*Étude de codeparsers, 18 octobre 2026 au soir, à la demande de l'orchestration ; docs seules,
rien n'est codé. Sources : la vision générale (« L'atelier comme IDE des gabarits », lignes
434-470 ; « Des applications rag3weaver empaquetées », 556-603 ; « un composant d'interface est
un nœud à ports », 241-249), la page du proto `11-octobre-2026-everything-declarative/05-l-atelier-comme-ide.md`
(§0, marche 3 aux lignes 203-208, question 8.3), le code de master `aafc3ccc2` lu par trois
repérages, et le code de webpack, React et Node (MIT) lu comme référence, sans en copier une ligne.*

La forme que Lucie a en tête : **un vrai frontend React tourne dans le navigateur et se recharge
à chaud, l'état gardé (React Refresh) ; ses composants viennent de nos gabarits.** Deux niveaux,
sur le même chemin :

1. **Le TSX comme donnée** : un gabarit porte son source, l'agent l'édite, l'hôte le compile en
   module, le sert, et le recharge par `/ide/flux`.
2. **Le composant déclaré** : ses props sont des ports d'entrée, ses événements des ports de
   sortie, et son graphe de clic (cases d'état, nœuds purs en expressions TypeScript, nœuds
   d'effet en `/ide/call`) est transpilé par l'hôte en TSX, puis par le niveau 1 en module. Il
   remplace le moteur WASM abandonné le 8 octobre.

**Par où commencer : le niveau 1**, comme le propose l'orchestration. Il donne vite un vrai React
rechargé à chaud, et le niveau 2 n'est qu'un producteur de TSX de plus sur ce chemin. Le niveau 2
sans le niveau 1 n'aurait rien pour s'exécuter.

## 1. Compiler du TSX dans l'hôte, sans Node

**Ce qui existe** (`src/script/typescript.rs:49-85`) : `strip_types` appelle
`swc_ts_fast_strip::operate` en `Mode::StripOnly`. Les types sont remplacés par des blancs, les
positions gardées ; les constructions non effaçables (enum, namespace…) sont refusées avec ligne et
colonne. Le JS obtenu tourne dans QuickJS (`rquickjs` 0.14, `src/script/quickjs.rs`), comme une
fonction `run(input)` : tout `import` est refusé (`quickjs.rs:213-221`), il n'y a ni chargeur de
modules ni résolution. Langages connus : rhai, typescript, javascript ; `.tsx` est inconnu
(`src/script/mod.rs:25-42`).

**Ce qui manque : le JSX.** Mais tout est déjà compilé dans notre arbre de dépendances :

- `swc_ts_fast_strip` 59 sait transformer le JSX (`Mode::Transform`, `parser.tsx = true`,
  `transform.jsx`, runtime automatique `react-jsx` / `react-jsxdev`). C'est derrière sa feature
  `nightly`, une feature vide qui n'exige pas un Rust nightly (lib.rs:71-98, 404-425).
- Il s'appuie sur `swc_ecma_transforms_react` 56, **déjà dans `Cargo.lock`** (tiré par
  `swc_ts_fast_strip` et `swc_ecma_transforms_typescript`). Cette crate porte aussi, en Rust,
  **la transformation de React Refresh** (`src/refresh/`, `RefreshOptions { refresh_reg,
  refresh_sig, emit_full_signatures }`), celle qui injecte `$RefreshReg$` et `$RefreshSig$`.
  `swc_ts_fast_strip` passe `refresh: None` ; il faudra donc appeler la crate directement pour
  l'avoir.

**La voie la plus générique** : un moteur `tsx` à côté de `typescript`, qui fait
parse TSX → effacement des types → JSX automatique → React Refresh → codegen (+ source map), par
`swc_ecma_parser` / `swc_ecma_transforms_typescript` / `swc_ecma_transforms_react` /
`swc_ecma_codegen`. Ce sont les mêmes crates, aux mêmes versions, que ce que nous compilons déjà.
**Aucune crate nouvelle** ; oxc (`oxc_transformer`, qui fait aussi TS + JSX + refresh) serait une
seconde pile d'AST à côté de swc, pour rien. Babel dans QuickJS marcherait mais coûterait cher à
chaque édition.

Deux différences avec le moteur `typescript` d'aujourd'hui :

- **Les positions** : le mode Transform réécrit le code ; les erreurs d'exécution se ramènent au
  source par la source map (option exposée par la crate).
- **Où ça tourne** : le module produit ne tourne pas dans QuickJS mais **dans le navigateur**.
  QuickJS reste le moteur des nœuds scriptés ; le composant n'y passe que pour être compilé. (Un
  rendu côté hôte, plus tard, pourrait l'y faire tourner avec un faux DOM ; hors marche 3.)

## 2. Le composant comme gabarit

**Ce qui existe** :

- Le manifeste (`BackendManifest`, `src/backend.rs:34-118`, `deny_unknown_fields`) n'a pas de champ
  pour un composant, un asset ou une lib.
- Les nœuds scriptés se découvrent dans `nodes/` (`backend.rs:900-920`) : `*.node.json` avec
  `name`, `language`, `script`, `inputs`, `outputs`, `config` ; chaque port est
  `{schema: <JSON Schema>, required}`, vérifié à la frontière (`src/dataflow/scripted_node.rs:10-13,
  156-175, 375-390`). C'est **déjà la forme d'un composant à ports**.
- La famille `Component` existe dans les gabarits (`src/template.rs:48-93`), mais `place` la refuse
  (« un composant s'écrit dans un fichier », `src/dataflow/template_nodes.rs:88-97`).
- Les vues sont du minijinja côté serveur (`views/`, `src/routes.rs:89-90`), sans JS client.
- Aucun magasin par contenu pour du JS ; seul le CDN jsdelivr sert mermaid dans l'atelier
  (`src/atelier/page.html:57-58`).

**La forme proposée** (la plus générique : le composant suit le nœud scripté) :

- **Un dossier `components/`**, découvert comme `nodes/`, et `*.component.json` à côté de son
  `*.tsx` :

  ```json
  {
    "name": "FicheProduit",
    "source": "fiche_produit.tsx",
    "inputs":  { "produit": { "schema": { "$ref": "entities/Product" }, "required": true } },
    "outputs": { "enregistrer": { "schema": { "$ref": "entities/Product" } } },
    "libs":    { "react": "react@19", "react-dom": "react-dom@19" }
  }
  ```

  Les **props sont les ports d'entrée**, les **événements (`on*`) les ports de sortie**, avec le
  même JSON Schema que les nœuds. Le contrat se vérifie comme entre deux nœuds (vision 241-249) : à
  la compilation, le type des props du TSX contre les schémas d'entrée (le type peut se générer
  depuis le schéma, dans un `.d.ts` donné au composant, ce qui évite de lire du TypeScript côté
  hôte) ; à l'exécution, en mode dev, les props reçues contre le schéma.
- **Les libs déclarées, rangées par contenu.** Un fichier `libs.json` au niveau du backend : nom →
  version → fichiers et leur blake3 (nous avons déjà `src/hash.rs`), avec une table `exports` à la
  Node (sous-chemins, conditions `browser`, `development` / `production`, `import`, `default`). Le
  téléchargement se fait **sur commande explicite seulement**, jamais en silence (vision 556-603) ;
  ensuite tout se sert du magasin, hors réseau, et part avec la base dans l'application empaquetée.
  Cela tranche la question 8.3 de la page du proto : pas de CDN pour ce qui est déclaré.
- **La résolution est une table, jamais le disque.** Spécifier relatif → un autre composant du
  backend ; spécifier nu → `parsePackageName` (`react/jsx-runtime` = `react` + `./jsx-runtime`) →
  la lib déclarée par l'importeur → son `exports` → un hash. Non déclaré : refus nommé. C'est le
  modèle des *package maps* de Node (`lib/internal/modules/package_map.js:66-196`), avec un hash à
  la place de l'URL.
- **Une seule instance de React** : `react`, `react-dom` et `react/jsx-runtime` doivent résoudre à
  la même lib (ReactDOM lit l'état interne de React, `packages/react/src/ReactClient.js:110`). Le
  résolveur l'impose pour tout le backend.

## 3. Le rechargement à chaud par `/ide/flux`

**Ce qui existe** : `/ide/flux` n'existe pas encore (lot A2 du proto : SSE relais du bus, `id:`
croissant, reprise par `Last-Event-ID`, sujets `files`, fin de réindexation, `dataflow`,
`run.<id>` ; page 05 lignes 95-101). L'hôte démon n'a pas de route `/ide` ; il dit seulement
`MANIFEST_CHANGED` (`src/daemon_host.rs:314-320`). Mais l'**atelier** recharge déjà tout :
surveillance des déclarations toutes les 500 ms (`src/atelier.rs:147-203`), `check_reload` puis
`apply_reload_with_reactions` (`backend.rs:2512`, `:2453`), événements `reloaded` / `refused`. Un
fichier de composant y entre dès que `list_declarations` le liste, et une erreur de compilation
devient naturellement un `refused` avec sa ligne.

**Ce qui manque** :

- **Un événement par module** sur `/ide/flux`, sujet `components` :
  `{template, hash, importeurs}`. C'est le manifeste de mise à jour de webpack
  (`HotModuleReplacementPlugin.js:877-899`, champs `c`, `r`, `m`), mais poussé au lieu d'être
  sondé. Le client ignore un hash qu'il a déjà ; un événement qu'il ne comprend pas lui fait
  recharger toute la page, comme `webpack/hot/dev-server.js:16-59`.
- **Un petit registre de modules dans la page** (une trentaine de lignes, l'idée de
  `__webpack_require__` : fabriques, cache, `require(id)`), et non l'ESM natif : un module ESM ne
  se remplace pas en place, et des URL versionnées laisseraient traîner d'anciennes instances. Le
  module compilé est une fabrique enregistrée sous **l'id du gabarit** (stable, il sert de nom de
  famille à React Refresh) ; le hash n'est que la version.
- **React Refresh tel quel** : son runtime (`react-refresh`, une lib déclarée comme les autres)
  décide seul de garder l'état (même genre, même signature de hooks :
  `ReactFreshRuntime.js:121-176`) ou de remonter le composant. L'ordre de démarrage compte :
  runtime de refresh, puis react, puis react-dom (`injectIntoGlobalHook`, `:437-450`), puis les
  composants. Après chaque événement : la nouvelle fabrique s'exécute, puis
  `performReactRefresh()`.
- **La remontée** : un module qui n'exporte que des composants s'accepte lui-même ; sinon la mise à
  jour remonte à ses importeurs (`JavascriptHotModuleReplacement.runtime.js:125-195`), et au
  sommet, la page se recharge. Le manifeste peut le dire d'avance (« ce gabarit n'exporte que des
  composants »).

## 4. Le composant indexé comme du code

**Ce que codeparsers sait aujourd'hui** :

- `.tsx` et `.jsx` sont admis (`code.rs:130-195`) et envoyés au parseur, les imports (par
  expression régulière, `base_scope_extraction_parser.rs:2660-2790`) donnent `USES_LIBRARY` vers
  `react` et résolvent les imports locaux (`.jsx` → `.tsx`, `index.tsx` :
  `import_resolution/type_script_import_resolver.rs:327-345`).
- Les composants fléchés deviennent des `Function`, les interfaces et types de props des scopes à
  part ; `React.memo(...)` / `forwardRef(...)` deviennent des `Variable`.
- **Un défaut** : les `.tsx` sont parsés par la grammaire **TypeScript**, pas TSX
  (`base_scope_extraction_parser.rs:247-251`, aucun `LANGUAGE_TSX` hors de deux exemples de
  mesure). Le JSX y tombe en nœuds `ERROR`. Le code qui ferait de `<Button/>` un usage de `Button`
  existe (`:2412-2445`) mais ne se déclenche jamais. Ticket :
  `docs/tickets/2026-10-18-codeparsers-parse-le-tsx-avec-la-grammaire-typescript.md`.

**Ce qu'il faut pour « usages d'un composant, ses props, son voisinage »** : la grammaire TSX pour
`.tsx` et `.jsx` (le correctif du ticket, qui réveille le code JSX existant, en filtrant les
balises en minuscules et en gardant `Foo.Bar`) ; un lien du composant vers le type de ses props
(premier paramètre, ou `React.FC<Props>`) ; `memo` / `forwardRef` rangés en `Function` ; des
fixtures `.tsx`. Le niveau 2 donne en plus, gratuitement, le voisinage déclaré : un composant
déclaré est un nœud du graphe du backend, ses ports y sont typés, sans rien à deviner.

## Ce qu'on reprend, ce qu'on laisse

| idée (référence) | où | on reprend ? | pourquoi |
|---|---|---|---|
| Registre de modules : fabriques, cache, `require(id)` | webpack `JavascriptModulesPlugin.js:1715-1762, 2133-2240` | oui, réécrit (~30 lignes) | les fabriques se remplacent à chaud ; l'ESM natif ne le permet pas |
| Point d'interception de l'exécution d'un module | webpack `:2159-2167`, `HotModuleReplacement.runtime.js:128-136` | oui | un seul endroit pour poser `$RefreshReg$` par module et suivre les importeurs |
| Id logique distinct du hash de contenu | webpack `NormalModule.js:868`, `DeterministicModuleIdsPlugin.js:55-67` | oui | id = nom du gabarit (stable, nom de famille de Refresh) ; hash = version et clé de cache |
| Hash d'entrées corrigé après coup (`RealContentHash`) | webpack `RealContentHashPlugin.js:276-324` | non | un module par gabarit, pas de renvoi entre fichiers : on hache les octets finaux (blake3) |
| Graphe des modules, arêtes entrantes et sortantes | webpack `ModuleGraph.js:289-316` | oui, en Rust | il faut les importeurs pour la remontée ; codeparsers les a déjà par l'index |
| Chunks, JSONP, sondage du manifeste et 404 | webpack `JsonpChunkLoadingRuntimeModule.js` | non | un gabarit = une unité ; l'événement SSE porte le manifeste |
| accept / dispose / remontée vers les parents, rechargement au sommet | webpack `HotModuleReplacement.runtime.js:249-273`, `JavascriptHotModuleReplacement.runtime.js:125-195` | oui, interne (le code généré s'accepte) | la règle de remontée est la bonne ; aucun auteur ne l'écrit à la main |
| `decline` | webpack `:262-267` | non | sans usage pour un gabarit |
| Les appels injectés `$RefreshReg$` / `$RefreshSig$` | React `ReactFreshBabelPlugin.js` | oui, par **swc en Rust** (`swc_ecma_transforms_react::refresh`) | même contrat, sans Babel ni Node, déjà compilé chez nous |
| La décision de garder l'état (famille, signature de hooks) | React `ReactFreshRuntime.js:121-176, 209-320` | oui, le runtime tel quel, en lib déclarée | c'est exactement la règle voulue ; la réécrire serait la trahir |
| JSX automatique `{jsx, jsxs, Fragment, jsxDEV}` | React `packages/react/src/jsx/ReactJSX.js:17-29` | oui | `react/jsx-dev-runtime` dans l'IDE, `react/jsx-runtime` empaqueté |
| `process.env.NODE_ENV` | entrées npm de React | adapté | les builds dev et prod rangées comme deux libs, ou un `process` minimal posé avant |
| `parsePackageName`, table `exports` et conditions | Node `package_map.js:205-226`, `esm/resolve.js:449-638` | oui, un sous-ensemble | `browser`, `development`/`production`, `import`, `default` suffisent |
| Package maps : dépendances par importeur, sans disque | Node `package_map.js:66-196`, `doc/api/packages.md:950-1067` | oui, **le modèle principal** | c'est notre « libs déclarées, rangées par contenu », l'URL devenue hash |
| `#imports`, `node:`, `main`/`index`, liens symboliques | Node `resolve.js:165, 195, 663, 729-733` | non | ni Node ni arborescence de `node_modules` |
| Import maps du navigateur | Node `doc/api/packages.md:438-441` | plus tard, pour l'empaqueté | bien pour la prod ; inutilisables pour le rechargement à chaud |

## Les lots, dans l'ordre

**Niveau 1 (le TSX comme donnée)** :

1. **Le moteur `tsx`** dans `src/script/` : swc parse → types → JSX automatique → Refresh →
   codegen + source map, sans crate nouvelle ; témoin : un composant compilé contient `jsx(` et
   `$RefreshReg$(…, "<gabarit> FicheProduit")`, une erreur de syntaxe sort avec sa ligne.
2. **`components/` et `libs.json`** : découverte, déclaration à ports (le schéma des nœuds), le
   magasin par contenu, la commande explicite qui remplit le magasin, la résolution par table ;
   témoin : un import non déclaré est refusé nommément, `react` résout au même hash partout.
3. **Servir** : une route qui sert un module par hash (cache immuable) et la page d'aperçu
   (registre, runtime de refresh, react, react-dom, puis les composants).
4. **Le sujet `components` de `/ide/flux`** (sur le lot A2 du proto) et le rechargement dans la
   page ; témoin du navigateur : l'état d'un compteur survit à l'édition du libellé, un hook
   ajouté remonte le composant.

**Niveau 2 (le composant déclaré)** : un graphe de clic (`*.mmd` comme les graphes du backend :
cases d'état, nœuds purs en expressions TS, nœuds d'effet en `/ide/call`) transpilé par l'hôte en
un `.tsx` (un `useReducer` pour les cases, des fonctions pour les nœuds purs, des `fetch` vers
`/ide/call` pour les effets), qui entre au niveau 1 par la même porte. Le contrat des ports se
vérifie alors deux fois, à la transpilation et à la compilation.

**Pour codeparsers, en parallèle et petit** : le correctif de la grammaire TSX, qui fait de tout
composant du produit un code indexé comme les autres.
