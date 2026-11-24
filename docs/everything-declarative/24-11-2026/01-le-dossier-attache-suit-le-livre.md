# Un dossier attaché suit le livré : le manifeste en surcouche

24 novembre 2026, session everything declarative. c7 : deux lots de la nuit disent en nouveauté qu'« un dossier déjà
attaché garde son manifeste ». Conséquence : les dossiers déjà ouverts de Lucie, rag3db le premier, ne reçoivent pas
ce qui est livré par gabarit, tant qu'elle n'édite pas leur manifeste à la main. Plus le comportement passe en
gabarits, plus l'écart grandit : c'est le revers du tout déclaratif.

## Ce qui suit déjà le livré, et ce qui ne le suit pas

Relevé en lecture seule sur le manifeste de son dossier rag3db (`code-rag3db/templates/backends/code/rag3db.json`),
avec `rag3weaver-backend templates drift` et une comparaison clé à clé avec le manifeste livré.

- **Ce qui suit** : chaque fichier désigné par `builtin:<nom>` (18 dans ce manifeste : les graphes des outils, les
  gabarits de rendu) se lit chez le livré du moment. Les invites de l'équipier suivent aussi : le fichier de
  l'utilisatrice sous `templates/teammate/prompts/` remplace le livré fichier par fichier, et ce qu'elle n'a pas écrit
  vient du livré.
- **Ce qui ne suit pas : le manifeste lui-même.** C'est une copie entière, faite à l'attache. **30 clés du manifeste
  livré n'y sont pas** : tout le bloc `teammate` (donc `carry_over_node`, posé cette nuit), l'outil `navigate_symbols`
  entier, les politiques de fraîcheur des outils (`freshness`), `workspace.sandbox`, `embeddings.catch_up_on_open`…

## Les deux voies

**A. Le manifeste en surcouche.** Le manifeste d'un dossier commence par `"extends": "builtin:backends/code/backend.json"`
et ne garde que ce que l'utilisatrice a changé. À l'ouverture, l'hôte fusionne clé par clé : un objet se fusionne
récursivement, une valeur ou un tableau de la surcouche remplace celle du livré, et `null` retire une clé. Le manifeste
lu est la fusion, validée comme aujourd'hui.

**B. Une commande « mettre à jour les gabarits de ce dossier »**, qui montre le diff entre la copie et le livré, puis
écrit les clés acceptées.

## La recommandation : A

- C'est le modèle qui marche déjà pour les invites (une surcharge par fichier) et pour les graphes (`builtin:`), étendu
  au manifeste, clé par clé.
- Le dossier reçoit chaque amélioration livrée sans geste, et ce que l'utilisatrice a choisi reste à elle.
- B demande un geste à chaque mise à jour, et le geste oublié est exactement l'écart d'aujourd'hui.

**Ce que A coûte** :
1. **la fusion** : une fonction pure, clé par clé, avec son témoin (objets fusionnés, valeur et tableau remplacés,
   `null` qui retire). Petit ;
2. **la migration** des dossiers existants : une commande qui lit la copie, la compare au livré du moment, et écrit la
   **surcouche minimale** (les seules clés qui diffèrent), en gardant l'ancien fichier en `.avant-extends`. Elle montre
   le diff avant d'écrire, et n'écrit que sur le mot de l'utilisatrice. Moyen ;
3. **l'attache** : un dossier neuf reçoit d'emblée une surcouche vide (`{"extends": …}`) au lieu d'une copie. Petit ;
4. **une décision par clé changée par Lucie**, au moment de la migration : une clé qu'elle avait changée exprès reste
   dans la surcouche, une clé seulement ancienne part au livré. La commande les sépare : ce qui diffère d'une version
   livrée passée est ancien, le reste est à elle.

**Ce qu'il faut garder** : une clé de sûreté (les dossiers protégés, le plafond, la porte) ne se retire pas par `null`
dans une surcouche. La fusion le refuse en le nommant.

## En attendant, pour le dossier rag3db de Lucie

Sur son mot seulement, puisque c'est sa configuration :
- **le passage des faits d'un pas à l'autre** : ajouter `"carry_over_node": "PreviousStep"` dans un bloc `teammate` de
  `rag3db.json`. Ce geste deviendra inutile quand le nœud sera le défaut de l'hôte (lot à venir, après celui d'archi) ;
- **les descriptions neuves des outils d'index** (`usages`, `outline`) : rien à faire, ces outils sont lus par `builtin:`
  dans le manifeste de l'atelier, qui ne diffère en rien du livré (`templates drift` : 0 copie différente) ;
- **le reste de l'écart** (les 30 clés) attend la migration en surcouche, plutôt qu'une recopie à la main qui
  vieillirait de même.
