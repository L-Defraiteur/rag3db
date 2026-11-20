# Le poids des lignes minifiées

Session codeparsers, 20 novembre 2026, à la demande de c7. Page courte, sans code. Mesure sur une copie de
code-rag3db-depot faite par G à 15 h 38 (`mesures/poids-app-js-20261120-1538`, lecture seule par Cypher).

## Ce qui pèse aujourd'hui

`extension/rag3weaver/templates/ide/app/app.js` est un bundle de 509 Ko sur 86 lignes, dont une ligne de 133 294
caractères. Le chiffre de 57,6 % du plein texte date du 14 novembre, quand chaque fonction de cette ligne en recevait
le texte entier (le morceau de 133 Ko répété 484 fois de la mesure de lucivy). Sur la copie d'aujourd'hui :

| | app.js | toute la base | part |
|---|---|---|---|
| contenu des `Scope` | 5,56 Mo | 101 Mo | 5,5 % |
| contenu des `ScopeContent` | 5,56 Mo | 92 Mo | 6,0 % |
| morceaux des `Scope` | 5,56 Mo | 99 Mo | 5,6 % |

app.js reste le fichier le plus lourd de la base (le second, `json.hpp`, pèse 1,6 Mo). Sur ses 5,56 Mo, **4,5 Mo
viennent de 37 scopes `variable` qui portent chacun la ligne entière** (133 294 caractères), le reste de 123 `method`.
Ce sont les seuls `variable` de plus de 5 Ko de toute la base.

## Qui les porte : des scopes orphelins

- **app.js n'a plus de ligne `File`**, sous aucune version : il est exclu de l'index (`index_exclude` de Lucie, posé
  le 14 novembre), et ses voisins du même dossier ont bien les leurs.
- **Ses 160 `Scope` sont restés** : tous ouverts depuis la version 0 (`_depuis = 0`, `_jusqu_a` nul), étalés sur
  quatorze instantanés, et sans aucune arête `DEFINED_IN`. Ils viennent d'analyses antérieures au bornage aux octets
  (57bfc142bf, 14 novembre), d'où la ligne entière dans leurs variables.
- **L'analyse d'aujourd'hui ne les produirait plus ainsi.** Relu par codeparsers, app.js donne 1 511 scopes, et chaque
  `variable` porte l'empan exact de son déclarateur (`$0` : octets 4 à 20, `pd` : 658 à 701). Le défaut de la ligne
  entière pour les variables est déjà corrigé ; ce qui pèse est un reste.

La synchronisation ne connaît les fichiers que par leurs `File` : elle ne retirera jamais ces scopes. C'est le défaut
qu'everything declarative prouve en ce moment (le relevé lui est transmis).

## Ce que chacun résout

- **everything declarative** :
  - **le retrait des scopes orphelins**, sans `File`, résout tout le poids d'aujourd'hui (les 5,56 Mo) ;
  - **l'exclusion par défaut des bundles**, règle déclarée dans le manifeste (la décision de Lucie, 17 h 00), empêche
    qu'un bundle neuf revienne ;
  - **les morceaux bornés** (son lot 14) plafonnent la taille d'un morceau, quel que soit le scope.
- **Lucie** : l'`index_exclude` de ses manifestes est déjà posé pour app.js ; l'exclusion par défaut le généralise.
- **codeparsers**, ce qui reste après l'exclusion, non prioritaire (décision de Lucie) : savoir analyser les scopes
  d'une ligne minifiée qu'on n'exclurait pas. Sur une ligne unique, ce qui se borne encore aux lignes, ce sont les
  trous entre déclarations (les scopes de fichier) et le texte brut : ils n'ont pas d'empan de nœud. La forme voulue :
  un pseudo-retour à la ligne forcé mécaniquement, à 300 caractères au plus. Ticket
  `2026-11-20-une-ligne-minifiee-se-decoupe-par-colonnes.md`.

Les deux remèdes se composent : l'exclusion retire le bundle, le découpage par colonnes ne sert qu'à ce qu'on garde.
