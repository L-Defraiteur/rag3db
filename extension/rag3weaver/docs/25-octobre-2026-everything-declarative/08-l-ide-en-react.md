# L'IDE en React — la chaîne de bâti, le cadre, puis les vues une à une

Demandé par Lucie le 25 octobre 2026 au soir, après son essai de l'IDE : « il nous faut du React
maintenant, là c'est très moche, il nous faut shadcn et React et lucide ». Relayé et découpé par c7.
Priorité après l'atterrissage d'équipier-1 et du lot IDE groupé. Identifiants en anglais.

## Ce qui ne change pas

- **Les points d'entrée HTTP de l'hôte** (`src/ide.rs`) : `/ide/etat`, `/ide/call`, `/ide/flux`
  (SSE), `/ide/chat`, `/ide/taches`, `/ide/bilans`, `/ide/teammate`… La page nouvelle les appelle
  comme l'ancienne.
- **Jamais de CDN, jamais de compilation au vol.** Le bundle est un fichier du dépôt, embarqué
  dans le binaire avec `templates/` (`build.rs`, `src/builtin_templates.rs`) et servi par l'hôte,
  comme `page.js` aujourd'hui.
- **L'ancienne page reste servie** jusqu'à ce que chaque vue soit remplacée ; les témoins
  navigateur existants suivent vue par vue, mêmes repères DOM ou repères changés dans le même
  commit.

## La chaîne de bâti

- **Les sources** : `extension/rag3weaver/ui/ide/` — `package.json` et `package-lock.json`
  (versions épinglées, `npm ci`), `src/` en TSX, `components/ui/` les composants shadcn/ui copiés
  dans le dépôt comme la bibliothèque le prévoit, `tailwind.css` et sa configuration.
- **Les paquets** : react 19.2.0 et react-dom 19.2.0 (ceux que `scripts/paquets_react.sh` épingle
  déjà pour l'aperçu à chaud), lucide-react, les dépendances des composants shadcn copiés
  (radix-ui, class-variance-authority, clsx, tailwind-merge), esbuild et tailwindcss comme outils
  de bâti. Licences lues dans les paquets installés avant le premier commit : shadcn/ui MIT,
  lucide ISC, react MIT, esbuild MIT, tailwindcss MIT ; la liste et les licences sont dans
  `ui/ide/LICENSES.md`.
- **Le script** : `scripts/ide_bundle.sh` fait `npm ci` puis bâtit en une passe esbuild (TSX →
  un seul `app.js`, ES2022, minifié, sans source map servie) et une passe tailwindcss (`app.css`),
  et écrit `templates/ide/app/app.js`, `templates/ide/app/app.css` et
  `templates/ide/app/bundle.json` : l'empreinte blake3 des deux fichiers, les versions des paquets,
  l'instant. Les trois sont **commités** : le bâti Rust n'a pas besoin de node.
- **Servi** : `GET /ide/app.js` et `GET /ide/app.css` rendent le contenu embarqué
  (`builtin_templates::get("ide/app/app.js")`), avec l'empreinte en `ETag`.
- **Vérifié, deux fois** :
  - un témoin de la lib (toujours jouable) : l'empreinte des octets embarqués égale celle de
    `bundle.json`, et `bundle.json` nomme les versions épinglées de `package.json` ;
  - une étape de passe (`scripts/ide_bundle.sh --check`, avec node) : rebâtit dans un dossier
    temporaire et compare octet pour octet au bundle commité ; un écart est un rouge qui dit
    « le bundle servi n'est pas celui du code : relancer scripts/ide_bundle.sh ». Elle entre dans
    `check_features.sh`-comme-étape, pas dans `cargo test --lib`.

## Le cadre (vue 1)

La page nouvelle, `templates/ide/app/index.html` servie à `/ide/` (l'ancienne reste à
`/ide/ancienne/` tant qu'une vue n'est pas portée) : l'en-tête, la navigation entre backends et
vues (le routeur lit l'URL comme aujourd'hui : `/ide/code/<id>/<vue>/…`), la liste des backends et
des tâches, le flux SSE ouvert une fois et partagé par les vues (un contexte React), l'état de
l'hôte relu à chaque ouverture du flux. shadcn : `Button`, `Card`, `Badge`, `Tabs`, `Sheet` (la
colonne repliable), `Separator`, `Tooltip`, `ScrollArea` ; lucide pour les icônes.

## L'ordre des vues, chaque vue un push

1. **La chaîne de bâti et le cadre** (ci-dessus). Témoin : le bundle vérifié ; le cadre se rend dans
   un vrai navigateur (accueil, liste des backends, liste des tâches).
2. **Le chat** de la tâche et de l'atelier : un seul composant, la forme de `ui/chat` (colonne des
   conversations, en-tête nom / modèle / état, fil en flux, saisie en bas, Entrée envoie, reprise
   après rechargement), plus les outils dépliés sous la réponse (`details` par appel, pendant le
   flux), le lien vers la relecture quand un tour a écrit, le lien « bilans ». Suite écrite ici,
   pas codée : les tours typés entre agents (reçu de `agent:<clé>`, envoyé à, interne : la
   mailbox de l'équipier et les notifications entre sessions, la clé stable et le nom en alias),
   chaque fiche atteinte par un tour en lien, et une barre latérale repliée qui s'agrandit au
   clic — en haut le lien de la fiche de session, dessous les sujets touchés dans l'ordre, chacun
   ouvrant la fiche à côté du fil ; tout vient de la fiche de session et du journal des runs.
3. **La vue d'un fichier** et sa marge de symboles (le plan d'outline, le repli sur les scopes de
   `read_file`), l'édition CodeMirror gardée telle quelle (ses modules viennent du magasin
   embarqué, `/ide/editeur/modules.js`).
4. **La recherche**, avec ce que la session recherche prépare : surlignages, signaux, provenance,
   la case « reclasser ».
5. **La relecture d'une tâche** : le diff par fichier, fusionner, rejeter, l'équipier (boîte,
   plans).
6. **Les bilans** et la fiche par outil.
7. **Le graphe** et l'aperçu d'un composant.

Quand la dernière vue est portée, `page.js` et `page.html` partent, et `/ide/ancienne/` avec eux.

## Ce qu'on garde de `page.js` (1 335 lignes)

- La logique, pas le DOM : le routeur d'URL, le client du flux (`EventSource`, `Last-Event-ID`,
  les sujets), `appeler(id, outil, args)`, la relecture de l'état à l'ouverture du flux, le suivi
  d'un run (`RunStarted` → `RunFinished`), le diff de la relecture, le rendu d'un Plan en json
  (outline), les liens de symbole et de fichier. Chacun devient un module TS sous `ui/ide/src/`.
- Les repères DOM que les témoins navigateur lisent (`#vue`, `#plan .scope`, `#plan-repli`,
  `#dag-etat`, `.proposition`, `#fusionner`, `#rejeter`, `#chat-message`…) : gardés tels quels
  dans les composants, ou changés avec le témoin dans le même commit.

## Tranché par c7, et prouvé au premier bâti

- **Tailwind par la CLI épinglée** (`@tailwindcss/cli` 4.3.3), et le bâti est déterministe :
  deux `scripts/ide_bundle.sh --check` successifs après un bâti rendent VERT sur ce poste
  (node 22.23, `npm ci` sur le lock) ; esbuild n'écrit ni date ni chemin absolu (`legalComments:
  none`, pas de source map servie). La preuve se rejoue à chaque passe par l'étape `--check`.
- **Le mode sombre dès le cadre**, par les variables CSS de shadcn (`:root` et `.dark`) : la
  préférence du système par défaut, un bouton dans l'en-tête (`#theme`, lucide Sun / Moon), le
  choix gardé dans `localStorage` (`ide-theme`) et posé avant le premier rendu par un script de
  `index.html`. Témoin navigateur : la classe `dark` et la couleur de fond basculent au clic, le
  choix tient au rechargement.
- **`/ide/` dès la vue 1** : l'hôte sert la page nouvelle à `/ide` et `/ide/`, l'ancienne pour
  toute autre URL (`/ide/code/<id>/<vue>/…`) tant que la vue n'est pas portée, et toujours sous
  `/ide/ancienne/…` ; l'ancienne page lit son URL sous ce préfixe. Les liens de l'accueil neuf
  restent `/ide/code/<id>/<vue>/`, donc les témoins de l'ancienne page passent par le nouvel accueil
  sans changer (`#vue h1` « Backends », un lien `/ide/code/<id>/arbre/` par backend).

## Ce qui est fait (vue 1)

- `ui/ide/` : `package.json` et `package-lock.json` épinglés, `build.mjs`, `tsconfig.json`,
  `components.json`, les composants shadcn copiés (`button`, `badge`, `card`, `tabs`, `sheet`,
  `separator`, `tooltip`, `scroll-area`), `lib/utils.ts`, `lib/host.ts` (l'état, l'appel, le
  flux), `lib/theme.ts`, `src/main.tsx` (le cadre), `src/views/Home.tsx` (l'accueil : les backends
  en cartes, leurs vues, leurs tâches), `src/app.css` (le thème), `LICENSES.md`.
- `templates/ide/app/` : `index.html`, `app.js` (235 Kio minifié : React, les primitives radix de
  huit composants, lucide à la demande), `app.css` (29 Kio), `bundle.json`.
- L'hôte : `/ide/app.js`, `/ide/app.css`, `/ide/` neuf, `/ide/ancienne/…`.
- Témoins : `ide::tests::le_bundle_de_l_ide_est_celui_que_son_manifeste_decrit` (sha256 des
  octets embarqués = `bundle.json`, versions = `package.json`, `index.html` charge le bundle) ;
  `navigateur::le_cadre_de_l_ide_se_rend_dans_les_deux_modes`.

## Ce qui est fait (vue 2 : le chat)

- `src/views/Chat.tsx`, un seul composant pour le chat de la tâche et celui de l'atelier, sur la
  forme de `ui/chat` : la colonne des conversations (`#sessions` : ce backend et ses tâches, ou la
  tâche et son parent), l'en-tête (`#name`, `#model` « modèle : … » ou « chat éteint : … »,
  `#status`), le fil (`#messages`, `article.message.user|assistant|error` avec `.role` et `.body`,
  `.en-cours` pendant le tour), la saisie (`#composer`, `#message`, `#send`, `#stop` ; Entrée
  envoie, Maj + Entrée saute une ligne), la reprise après rechargement (un run en cours relu à
  l'ouverture et à chaque réouverture du flux, `window.__ide.relireLeChat`).
- Ce qui s'ajoute à `ui/chat` : les outils dépliés pendant le flux (`details.outil` par appel,
  `data-outil`, `data-appel`, `data-ok` ; `summary` le nom, `pre` les arguments puis le résultat),
  les propositions du tour (`.proposition-du-tour a[data-proposition]`), le lien vers la relecture
  quand un tour a écrit (`.tache-du-tour a[data-tache]`), les liens « relecture » et « bilans »
  dans l'en-tête, la consommation (`#chat-conso`).
- Le flux : un bus dans la page (`onFlux` / `dispatchFlux`) ; `RunStarted` nommé `chat:<id>`
  donne le run (le bouton Arrêter), `Token` remplit la réponse, `ToolCallStarted` /
  `ToolCallFinished` les blocs d'outil, `PropositionChanged` les liens ; la réponse du tour pose le
  texte entier, les fichiers écrits et la tâche.
- L'hôte sert la page nouvelle pour `/ide/code/<id>/chat` (`VUES_PORTEES`).
- Les deux témoins navigateur du chat (`chat::tests::le_chat_se_mene_et_s_arrete_dans_la_page`,
  `chat_qui_code::le_chat_d_une_tache_mene_a_sa_relecture_dans_la_page`) lisent les repères de
  `ui/chat` : `#model`, `#status`, `#message`, `#send`, `#stop`, `article.message.assistant`,
  `details.outil[data-outil]`, `.proposition-du-tour`, `.tache-du-tour` — changés dans le même
  commit que la vue.

## Ce qui est fait (vue 3 : le fichier et sa marge)

- `src/views/File.tsx` : le fil d'Ariane (`#fil`), le texte numéroté (`tr#L<n>`, la ligne visée
  par l'ancre centrée et marquée `.vise`), chaque identifiant en lien vers son symbole (`a.id`), la
  suite d'un long fichier (`?depuis=`), l'état (lignes, « modifié depuis l'index » en `.alerte`,
  « non indexé »).
- La marge `aside#plan` « Dans ce fichier » : le plan d'outline (`.scope` avec `--profondeur`,
  `data-ligne`, le nom vers `#L<n>`, la flèche vers le symbole, `#plan-borne`, `#plan-provenance`)
  ; sans `outline` au manifeste, les scopes de `read_file` et `#plan-repli` qui dit pourquoi — le
  même repli que le lot IDE (400813249), porté.
- **L'édition reste sur l'ancienne page** jusqu'à ce qu'elle soit portée (vue 3b, CodeMirror par
  le magasin embarqué) : le bouton « Modifier » (`a[data-modifier]`) mène à
  `/ide/ancienne/code/<id>/fichier/<chemin>`, et les deux témoins qui éditent (`les vues de
  l'IDE`, `le dépôt d'à côté`) y vont tout droit pour cette partie.
- `VUES_PORTEES` : `chat`, `fichier`.

## Ce qui est fait (vue 4 : la recherche)

- `src/views/Search.tsx`, sur le JSON de `search_code` convenu avec la session recherche (voir
  plus bas) : le formulaire (`#chercher`), le titre « q », la ligne d'état (`#recherche-etat` : la
  ligne « index : » de `warnings`, la fraîcheur, les comptes mots / sens / creux / relus, « partiel »),
  la case `#reclasser` (`&reclasser=1` → `options.rerank = { candidates: 20 }`), les résultats
  (`.resultat` : le fichier vers sa ligne, le symbole, `.meta` type · signal, `.score`, `.signaux`
  « label n°rang (score) », l'extrait borné à 600 caractères), les avertissements en pied
  (`.avertissements`), et l'enveloppe `{status, result}` d'une lecture dépliée (`unwrap`).
- **Les surlignages** : `chunk.highlights: [{start, end, source}]` (dans le morceau : il désigne ce
  qu'il surligne et survit à la fusion ; la forme au niveau du résultat passe aussi), en unités UTF-16 dans
  `chunk.text`, rendus par `mark.surligne[data-source]` (trois styles : lucivy, sparse,
  query_words) avec une légende quand il y en a. Le champ n'existe pas encore : la vue l'attend.
- Le JSON : `result[]` (uuid, score, entity, `data` en snake_case, `chunk`, `signal`,
  `signals[{label, score, rank}]` — `bm25_exact` et `rerank` sont des labels de plus),
  `metadata["render.meta"]` (signals, consistency, partial, les comptes, `warnings` omis si vide),
  `freshness` en snake_case ; pas de `total`, le nombre rendu est `result.length`.
- `VUES_PORTEES` : `chat`, `fichier`, `recherche`. Les deux témoins de la recherche (« la
  recherche », « la recherche à reclasser » dans les vues de l'IDE) passent par la page neuve avec
  leurs repères inchangés.
