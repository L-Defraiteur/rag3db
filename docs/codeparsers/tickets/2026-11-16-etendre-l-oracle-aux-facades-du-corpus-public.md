# Étendre l'oracle aux façades du corpus public

- **État** : ouvert — parsing des façades mesuré, couverture sémantique non jugée par l'oracle des huit langages de code
- **Famille** : produit
- **Gravité** : absence de diagnostic AST public et compteurs seuls insuffisants pour qualifier la couverture
- **Atteignable en service** : via les API spécialisées ; raccordement à l'index principal à prouver séparément
- **Touche rag3weaver** : potentiellement, dépendance codeparsers et ses consommateurs
- **Pour** : codeparsers et oracle indépendant Banc, coordination C7
- **Signalé par** : référence FULL02b e5e2d9cf5a, CSS/SCSS/Vue/Markdown et dialectes Sass/MDX distingués

La référence conserve les compteurs des sorties spécialisées et leur temps,
mais pas leur inventaire sémantique compact ; ASTstatus=not_exposed ne veut
pas dire valide. Le routeur de l'index principal ne prouve pas leur raccordement.
HTML est un refus déclaré, Svelte un chemin incomplet dans son ticket existant :
ne pas les compter comme façades qualifiées dans cette suite.

Poser des attentes sourcées avant résultats pour règles/variables/imports/styles,
blocs/props/composants Vue et sections/liens/code Markdown. Distinguer syntaxe
SCSS et Sass indenté, Markdown et MDX. Exploiter d'abord les API existantes ; si
un export compact ciblé manque, borner et publier cet ajout sans modifier le
binaire historique. Mesurer uniquement les sources nécessaires sous une nouvelle
allocation, pas rejouer32382fichiers pour compléter quelques cas.

Fin : faits observés/erronés/manquants/non exposés ou non conservés clairement
séparés, limites de captures/relations et tickets minimaux. Aucun nombre de règles
ne devient un taux de couverture par simple renommage du compteur.
Parent initial : `2026-11-16-qualifier-la-couverture-sur-des-corpus-publics-par-langage.md`.

## Relu le 18 novembre (session codeparsers)

Chantier de qualification, pas un défaut. Utile pour savoir ce que valent CSS, SCSS, Vue et Markdown, qu'aucun consommateur de rag3weaver ne lit aujourd'hui par l'index principal : priorité basse.

Le parent `2026-11-16-qualifier-la-couverture-sur-des-corpus-publics-par-langage.md` est clos et reste, avec le rapport de la campagne, sur la branche `cx-codeparsers-language-corpus` (non portée : son pointeur 1b35cfb, un outil du banc, ne va pas sur master).
