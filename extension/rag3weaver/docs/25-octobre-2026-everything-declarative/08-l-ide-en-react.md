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

## Ce qui n'est pas décidé

- Tailwind par le CLI (`@tailwindcss/cli`) ou par le plugin esbuild : le CLI est le plus court,
  et le `--check` compare des octets, donc l'un ou l'autre doit être déterministe ; à vérifier au
  premier bâti sur les deux postes (même node, même lock).
- Le mode sombre : la page actuelle a le sien ; shadcn en a un (`class="dark"`). On suit celui de
  l'OS, comme aujourd'hui, jusqu'à ce que Lucie dise.
- L'ancienne page et la nouvelle se partagent `/ide/` : l'ancienne passe à `/ide/ancienne/` dès la
  vue 1, ou la nouvelle n'y vient qu'à la vue 2 ? Je propose la première : Lucie voit le cadre neuf
  tout de suite, et chaque vue non portée ouvre l'ancienne dans son URL d'avant.
