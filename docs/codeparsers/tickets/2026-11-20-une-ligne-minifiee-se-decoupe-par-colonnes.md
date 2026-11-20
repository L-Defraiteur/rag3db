# Une ligne minifiée se découpe par colonnes

- **État** : ouvert (20 novembre 2026)
- **Famille** : produit
- **Gravité** : basse depuis l'exclusion par défaut des bundles (décision de Lucie, 20 novembre, 17 h 00) : ne sert
  qu'à une ligne minifiée qu'on garde dans l'index
- **Atteignable en service** : oui, pour un fichier minifié ou généré que le manifeste n'exclut pas
- **Touche rag3weaver** : oui, par codeparsers (les bornes des scopes de fichier et du texte brut)
- **Pour** : session codeparsers
- **Signalé par** : session codeparsers, mesure du poids des lignes minifiées
  (`docs/codeparsers/20-11-2026/01-le-poids-des-lignes-minifiees.md`)

## Constat

Depuis 57bfc142bf, un scope né d'un nœud porte l'empan exact de ce nœud : sur une ligne minifiée, une fonction ou une
variable n'a que son texte (app.js relu : 1 511 scopes, chacun borné). Mais ce qui n'a pas de nœud se borne encore
aux lignes : les trous entre déclarations (scopes de fichier) et le texte brut. Sur une ligne unique de 133 Ko, un
trou reçoit la ligne entière, et ses références sont relevées sur toute la ligne.

## Ce qu'il faudrait (la forme de Lucie)

Savoir analyser les scopes d'une ligne minifiée en forçant mécaniquement un **pseudo-retour à la ligne à 300
caractères au plus** : codeparsers découpe la ligne en colonnes de 300 caractères au plus (à une frontière sûre,
après `;`, `,`, `}` ou un espace, jamais dans une chaîne), et les trous se bornent à ces pseudo-lignes au lieu de la
ligne entière. Les numéros de ligne réels ne changent pas ; les octets restent ceux du fichier.

Témoin rouge d'abord : une ligne minifiée de plusieurs fonctions et de code de premier niveau entre elles ; chaque
trou a moins de 300 caractères de contenu, et ses références sont celles de sa colonne seulement.
