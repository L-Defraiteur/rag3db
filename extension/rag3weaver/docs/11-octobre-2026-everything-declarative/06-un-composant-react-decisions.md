# 06 — Un composant React est un gabarit : les décisions prises

*18 octobre 2026, la nuit. Lucie dormait ; l'orchestration m'a demandé de trancher par la voie
générique et de noter ici chaque décision pour son réveil. Le chemin vient de l'étude de
codeparsers (`../10-octobre-2026-18h20/codeparsers/07-un-composant-react-est-un-gabarit.md`).
Le lot 1, le moteur `tsx` (swc : TSX → JSX automatique → React Refresh), est à codeparsers ; les
lots 2 à 4 sont à moi.*

Chaque décision dit ce qui est choisi, pourquoi, et ce qui reste ouvert. Une décision qui ne
convient pas se reprend : rien n'est publié hors du dépôt.

## Lot 2 — la déclaration, le magasin, la résolution

1. **Un composant se déclare comme un nœud scripté.** `components/<nom>.component.json` à côté
   de son source (`.tsx`, `.jsx`, `.ts`, `.js`), découvert comme `nodes/`, vérifié à chaque
   chargement.
   - Les props sont des **ports d'entrée**, les événements des **ports de sortie**, avec le même
     JSON Schema que les nœuds.
   - **Un port de sortie doit s'appeler `on…`** : un composant ne rend pas de valeur, il émet des
     événements. Refusé sinon, en le nommant.
   - *Pourquoi* : le contrat se vérifie comme entre deux nœuds, et le niveau 2 (le composant
     déclaré par un graphe de clic) n'a rien de nouveau à apprendre.
2. **`libs.json` est indexé par nom de paquet.** Une entrée par paquet, avec sa version exacte,
   ses fichiers et leur blake3, sa table `exports`, son `main` et ses `dependencies`.
   - **Deux versions d'un même paquet ne s'écrivent pas** : une seule instance de React est
     garantie par la forme du fichier, sans règle à part.
   - *Pourquoi* : `react`, `react-dom` et `react/jsx-runtime` doivent partager l'état interne de
     React. Une règle « singleton » pour certains paquets serait une liste à tenir ; la forme
     rend l'erreur inexprimable.
   - *Ouvert* : deux versions d'une même lib qui ne partage pas d'état seraient refusées aussi.
     Aucun besoin connu aujourd'hui.
3. **Les versions sont exactes**, dans `libs.json` comme dans les `libs` d'un composant. Une
   dépendance du `package.json` en `^19.2.0` est rangée en `19.2.0`.
   - *Pourquoi* : un magasin par contenu n'a rien à résoudre ; une plage dirait « n'importe
     laquelle de celles-là », et le hash ne le pourrait pas.
4. **Le magasin est `<backend>/.libs/<blake3>`**, un fichier par contenu.
   - Il est caché, donc hors des déclarations : `list_declarations` saute déjà les dossiers en
     `.`, et un composant ne voit pas mille fichiers de React dans son atelier.
   - Il voyage avec le backend : l'application empaquetée part avec ses libs, hors réseau.
5. **Le magasin se remplit par une commande seulement** : `rag3weaver-backend libs add
   <backend.json> <paquet déballé>`.
   - La commande lit le `package.json`, hache et copie chaque fichier, et écrit l'entrée de
     `libs.json`.
   - Rien ne télécharge : un paquet se déballe d'abord (`npm pack react@19.2.0`, puis `tar`).
     C'est la règle de la vision : « téléchargé sur commande explicite seulement ».
   - *Ouvert* : une option qui va chercher le paquet au registre npm, sur la même commande, si
     Lucie la veut. Je ne l'ai pas faite pour ne pas mettre le réseau dans ce lot.
6. **La résolution est une table, jamais le disque** (`Components::resolve`) :
   - un spécifier relatif d'un composant mène à un autre composant ;
   - un relatif d'un fichier de lib mène à un fichier de la même lib (`.js` et `/index.js`
     essayés) ;
   - un spécifier nu mène à la lib que l'importeur **déclare** (les `libs` du composant, les
     `dependencies` d'une lib), puis par ses `exports`, avec les conditions `browser`,
     `development` ou `production`, `import` et `default`.

   **Les conditions d'`exports` se prennent dans notre ordre** (`browser`, le mode, `import`,
   puis `default` en dernier), et non dans l'ordre de l'objet comme le fait Node. Notre lecture
   du JSON range les clés : « default » passait avant « development », et le mode
   développement prenait le fichier de production. Le témoin l'a vu rouge d'abord. L'écart avec
   Node ne compte que pour un paquet qui mettrait, par exemple, `import` avant `browser`.

   Ce qui n'est pas déclaré est refusé en nommant l'importeur et le paquet. Un composant
   n'importe que ce qu'il déclare, même si une autre partie du backend l'a.
7. **Les vérifications au chargement** : la version des libs d'un composant contre `libs.json`,
   chaque dépendance déclarée à sa version, chaque fichier présent au magasin, chaque schéma de
   port valide. Le backend refuse de se charger en les nommant toutes, comme pour `nodes/`. La
   vérification des octets eux-mêmes (le hash recalculé) se fera au service (lot 3), là où on
   les lit.

## Lot 3 — servir les modules, la page d'aperçu

8. **Un module par fichier, un id stable, un hash pour version.**
   - `comp:<Nom>` pour un composant : le nom de famille de React Refresh, qui ne change pas d'une
     édition à l'autre.
   - `lib:<paquet>@<version>/<fichier>` pour un fichier de lib.
   - Le hash est celui du module compilé (moteur `tsx`) ou des octets du magasin.
9. **Tout passe en CommonJS, dans une fabrique.** Le composant est compilé par le moteur `tsx`
   en CommonJS (codeparsers l'a ajouté à ma demande, 748133ef3) ; les paquets npm de React le
   sont déjà. Chaque module est servi enveloppé :
   `__rag3weaver.define(id, hash, deps, function (require, module, exports, process) { … })`.
   - *Pourquoi* : un module ES ne se remplace pas en place. Le rechargement à chaud (lot 4)
     redéfinit une fabrique et la sort du cache, comme le fait webpack, sans copier son code.
   - `process.env.NODE_ENV` est posé par la fabrique (`development` dans l'IDE).
10. **La table des `require` de chaque module est calculée par l'hôte**, jamais par la page :
    `deps` va du spécifier tel qu'écrit à l'id visé, résolu par la table du lot 2. Les
    `require` d'un fichier de lib se lisent par leurs littéraux. Un `require` que rien ne
    résout est dit à la console, et le registre le refuse en le nommant s'il s'exécute (un
    `require` dans une branche morte ne bloque rien).
11. **Le magasin est vérifié au service** : un fichier de lib dont les octets ne redonnent plus
    leur blake3 est refusé (« le magasin a été touché »).
12. **Le démarrage de la page se résout par le backend** (`react`, `react-dom/client`, pris dans
    `libs.json`), pas par le composant : un composant n'a pas à déclarer `react-dom` pour être
    vu. Un backend sans ces deux libs ne peut pas montrer d'aperçu, et la route le dit.
13. **Les routes** :
    - `/ide/registre.js` : le registre, une constante Rust que le témoin exécute aussi ;
    - `/ide/apercu/<backend>/<Composant>` : la page ;
    - `…/modules.js` : les modules du composant, puis son entrée et son démarrage.

    Les props viennent de l'URL (`?props={"titre":"…"}`). C'est provisoire : le lot 4 et le
    niveau 2 les tireront des ports d'entrée.
14. **Le témoin exécute le vrai chemin sans navigateur** : le composant compilé par `tsx`, le
    graphe résolu, le registre, dans QuickJS, avec un faux React rangé au magasin. Le composant
    se rend en son élément, son enfant aussi, et `react-dom` voit le même React. Le vrai React
    dans un vrai navigateur reste à voir par Lucie : `rag3weaver-backend libs add` sur `react`,
    `react-dom` et `scheduler` déballés, puis l'aperçu.

## Lot 4 — le rechargement à chaud

15. **La montre des composants réutilise le `Scanner` de la montre de fichiers**, sans la porte
    de l'index (`Scanner::without_gate`) : une déclaration `.component.json` est une donnée que
    l'index écarte. Elle regarde `components/` et `libs.json`, jamais le magasin ni la base, et
    hérite des dates douteuses (le « racy git » corrigé le 18 octobre).
    - *Pourquoi pas un sondeur à part* : il aurait refait la course que la montre vient de
      perdre.
16. **Elle est montée à la demande** par la page d'aperçu (`Backend::watch_components`) et vit
    avec le backend : un backend que personne ne regarde ne balaie rien.
17. **Ses changements partent sur le bus du backend**, en `ComponentsChanged {paths,
    declarations, at_ms}`, sujet `components`, que le flux de l'IDE relaie comme les autres
    (`SUJETS`). `declarations` vaut vrai quand autre chose qu'un script a bougé.
18. **Le graphe relit les déclarations sur le disque** (`PreparedBackend::components_on_disk`) au
    lieu de recharger le backend : l'hôte ne recharge un backend nulle part (le rechargement
    arrête et remonte les réactions), et l'aperçu montre le disque. Les composants que voit le
    reste du backend restent ceux de son ouverture.
19. **Le registre ne redéfinit qu'un module dont le hash change**, et note les modules changés.
    `mettre_a_jour(frontieres)` remonte de chacun par ses importeurs jusqu'à une **frontière** :
    un module qui n'exporte que des composants (`exports_only_components`, le même critère que
    Vite et webpack). Chaque frontière est réexécutée, puis la page appelle
    `performReactRefresh`. Un chemin qui arrive en haut sans frontière rend `recharger`, et la
    page se recharge entière.
    - *Pourquoi dans le registre* : la logique se joue dans QuickJS sans navigateur
      (`an_edit_is_taken_back_at_its_boundary`).
20. **React Refresh est une lib du backend comme les autres** (`react-refresh/runtime` dans
    `libs.json`), pris au démarrage en développement s'il est rangé. La page l'injecte avant de
    charger `react-dom`, qui lit le crochet à son chargement. Sans lui, un changement recharge
    la page.
21. **La page relie l'id de son URL au manifeste du flux** par `/ide/etat` : le flux porte le
    manifeste, comme pour les autres sujets.
22. **Le témoin est dans un vrai navigateur** (celui de G, `daemon_host/navigateur.rs`). C'est
    le vrai React 19.2 rangé par `libs add`, un compteur cliqué trois fois et marqué dans le
    DOM, puis sa source réécrite sur le disque. Il exige : le libellé neuf, le compte 3, le
    même nœud, une mise à jour qui reprend `comp:Compteur`, un clic de plus qui compte 4, et
    zéro erreur à la console. Les paquets viennent de `scripts/paquets_react.sh`
    (`RAG3WEAVER_PAQUETS_REACT`), aussi en CI.
23. **Un pair se résout vers la copie que le backend range.** Le vrai `react-dom` 19.2.0 déclare
    `react` en `peerDependencies`, pas en `dependencies` : le premier passage du témoin dans
    Chromium l'a montré (« react-dom requiert react, que la table ne résout pas »). `libs add`
    garde l'intervalle tel qu'écrit (`peers`), et l'import d'un pair prend le paquet de
    `libs.json` quelle que soit sa version : c'est le sens d'un pair dans npm, une seule
    instance fournie par l'hôte. Un pair absent se dit à l'import, en le nommant.
24. **Un rechargement entier attend une seconde**, et s'annule si une mise à jour suivante reprend
    le module à une frontière. Un éditeur qui écrit en place vide le fichier avant de l'écrire :
    la montre peut tenir le fichier vide pour établi, et un module vide n'est pas une frontière.
    La passe de push l'a montré une fois sous la lib entière (« Total : 0 », l'état perdu) ; le
    témoin le rejoue de façon déterministe (le fichier vidé, 600 ms, puis le texte). Sans
    l'attente il rougit, la page rechargée sur le fichier vide (« le module n'exporte ni
    Compteur ni default ») ; avec elle, cinq passes sur cinq sont vertes. La raison d'un
    rechargement entier passe à la page suivante par `sessionStorage` et se lit dans
    `window.__apercu.recharge`.
