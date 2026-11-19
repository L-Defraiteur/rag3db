# Le lot B : la mémoire tout en déclaratif

Session mémoire (96), 19 novembre 2026. Page de conception, sans code. Elle reprend et détaille la section
« Lot B : tout déclaratif » de la [page du 18](../18-11-2026/02-l-archiviste-en-deux-etapes.md), après les
décisions de Lucie du 19 (relayées par c7).

Lucie : « tout doit être déclaratif en général ; quelqu'un doit pouvoir se dire : c'est juste un DAG comme un
autre, j'aurais pu en faire un moi-même ; et pareil si on veut des boucles auto-amélioratives plus tard, un agent
qui édite son propre workflow de mémoire ».

## 1. Ce qui n'est pas encore déclaratif, sur master

Le gabarit memory est déjà fait de graphes, de scripts Rhai et de schémas. Le Rust ne porte que des nœuds
génériques. Restent quatre choses :

| quoi | où | ce qui gêne |
|---|---|---|
| les motifs de secrets | `src/script/secrets.rs`, une constante Rust | changer un motif demande de rebâtir le moteur |
| l'invite d'extraction | `archive_slice_prompt.rhai` | une consigne en prose dans un script |
| l'invite du Decide | `archive_candidates.rhai` assemble la section, les candidates et le contexte en prose | l'ordre et la forme des morceaux sont enfouis dans un script |
| l'invite du découpage | `archive_segment_prompt.rhai` | comme l'extraction |
| le contexte d'après compaction | `compose_restart.rhai` compose l'identité, la fiche de tâche, le bilan et les L1 dans un budget | les ingrédients et leur priorité sont en dur dans le script |

## 2. La forme cible

### 2.1 `LlmNode`, nœud d'extraction structurée

Pas de type de nœud de plus. `LlmNode` prend trois déclarations :

1. **un port `context`**, qui reçoit une liste de **parts** ;
2. **`instruction`**, la ligne de tâche : un paramètre déclaré du graphe, court par défaut, modifiable sans
   rebâtir (Lucie : « dans tous les cas, même la consigne est paramétrable ») ;
3. **`response_schema`**, qui porte le sens dans les descriptions de ses champs. Une consigne écrite aujourd'hui
   en prose se déplace dans la description du champ qu'elle concerne (Lucie : « éviter qu'il y ait beaucoup de
   prompt à part les descriptions »).

Le nœud rend l'invite ainsi : l'instruction, puis chaque part dans l'ordre de la liste, en section (`## title`,
puis la valeur rendue). Le schéma part dans `response_format`, comme aujourd'hui.

### 2.2 La part

`{name, title, value, format?, max_tokens?}`. `value` est n'importe quelle valeur JSON : une chaîne, un objet,
une liste d'objets. Une chaîne passe telle quelle. Le plafond s'applique après le rendu, et la coupe est dite
dans la section (« … coupé : N jetons sur M »). Une part vide n'est pas rendue.

### 2.3 La présentation d'une valeur

Trois niveaux, le premier qui répond l'emporte :

1. `format` sur la part : il vaut à cette place, dans cette invite ;
2. la présentation par défaut du type, déclarée dans le schéma de l'entité (`fiche.json`, `source.json`,
   `turn.json`…), facultative. C'est soit un nom de forme (`markdown`, `kv` pour « clé : valeur »), soit un
   gabarit de texte sur les champs, comme ceux des fichiers de fiche (`gabarits/*.jinja`). Le type se lit sur la
   valeur : une ligne rendue par une sélection porte son entité (`entity`) ;
3. sinon, du JSON à clés triées, sur une ligne par élément de liste.

### 2.4 Le collecteur

Un nœud générique, `Collect` : des entrées nommées, déclarées dans l'ordre dans le `.mmd`
(`Collect(inputs='identity,report,candidates,slice', empty='drop')`), et une liste en sortie dans cet ordre. Il
rassemble n'importe quelle valeur, pas seulement des parts. L'ordre est donc écrit dans le graphe, et « le
stable d'abord » (pour le cache de préfixe, page du 18, (e)) veut dire : la première entrée.

- Les entrées nommées en nombre variable passent par `NodeFactory::schema_for(config)`, comme les sorties de
  `Route` (vérifié le 19).
- Le moteur ne gagne aucune règle de fusion par type (décision de Lucie : pas de règle cachée dans
  `merge_port_values`).
- **Un ingrédient débrayé rend une valeur vide, il ne se fait pas sauter.** Une branche sautée saute son aval :
  le collecteur sauterait, et l'invite avec. L'option `empty` (`drop` ou `keep`) décide du sort d'une entrée
  vide.

### 2.5 Un ingrédient, c'est une branche du graphe

Chaque ingrédient (`archivist.context.*`, `restart.context.*`) est un petit sous-graphe qui rend une part : sa
source (une sélection, un rappel, le rapport d'avant), sa borne, son titre. Le débrayer, c'est mettre son
paramètre `enabled` à faux : la branche rend alors une part vide. Un réglage du banc change des paramètres du
graphe, rien d'autre.

### 2.6 Les motifs de secrets

`mask_secrets` garde son parcours générique en Rust : trouver un préfixe en début de mot, mesurer le corps,
reconnaître un bloc PEM ou un `Bearer`. **La liste des motifs** devient un paramètre de `put_turn` (une valeur
JSON, avec la liste d'aujourd'hui pour défaut) : `[{name, prefix, min_body, charset?}]`. La fonction d'hôte
prend `mask_secrets(text, patterns)`. Le remplacement dit le genre (« [masked secret: github token] »), comme
la page du 18 le voulait.

## 3. Ce qui disparaît

- `archive_slice_prompt.rhai`, `archive_segment_prompt.rhai`, et la partie prose d'`archive_candidates.rhai` :
  il ne reste qu'un script qui rend les candidates en parts (ou plus de script du tout, si une sélection les
  rend déjà dans la bonne forme) ;
- dans `compose_restart.rhai`, l'assemblage en prose. La priorité dans le budget reste une règle, mais elle se
  déclare (l'ordre des entrées du collecteur, et un `max_tokens` par part).

## 4. La graine : un graphe qu'un agent peut éditer

Rien dans la forme ne doit empêcher qu'un agent, un jour, lise, modifie, fasse relire, revienne en arrière et
juge un graphe de la mémoire. Ce que cela demande de la forme, dès ce lot :

- chaque outil est un graphe `.mmd` court, ses paramètres déclarés avec un défaut et une phrase ;
- aucune logique propre à la mémoire en Rust : tout réglage est un paramètre, toute consigne est un
  paramètre ou une description de schéma ;
- chaque outil a son témoin, qui sert de juge ;
- les scripts Rhai restent des fonctions pures, sans état, lisibles seules.

Les outils de l'agent (lire, écrire, revoir, revenir) ne sont pas dans ce lot.

## 5. Les témoins

- un graphe à deux parts et un schéma rend l'invite attendue, octet pour octet ;
- deux branches qui finissent dans le désordre donnent la même liste, donc la même invite ;
- une part au-dessus de son plafond est coupée, et la coupe est dite ;
- une part objet et une part liste se rendent en JSON stable, à clés triées, octet pour octet ;
- la même fiche se rend pareil dans deux invites sans `format` de part, et autrement quand la part en déclare
  un ;
- un ingrédient débrayé ne fait pas sauter l'invite ; avec `empty='drop'`, sa section n'y est pas ;
- **l'invite d'extraction réduite** : le témoin d'extraction rejoué sur les mêmes tranches avec l'invite d'une
  ligne rend les mêmes sujets (faux modèle : la même requête attendue ; vrai modèle : le banc compare les
  sorties). Une consigne qui manque devient une description de champ ;
- `put_turn` masque un motif ajouté par le paramètre, sans rebâtir.

## 6. Les lots

| lot | contenu | Rust |
|---|---|---|
| B1 | `Collect`, le port `context` et `instruction` de `LlmNode`, la part et sa présentation (niveaux 1 et 3) | oui, générique |
| B2 | la présentation par défaut d'un type (niveau 2), déclarée dans le schéma | oui, générique |
| B3 | l'extraction et le découpage en parts, leurs scripts de prose retirés ; le témoin de l'invite réduite | non |
| B4 | le Decide en parts (`archive_candidates`) | non |
| B5 | `compose_restart` en ingrédients déclarés (`restart.context.*`, profils `l1_only`, `summary_only`, `l1_and_summary`) | non |
| B6 | les motifs de `mask_secrets` en paramètre | une signature de fonction d'hôte |

B1 d'abord, puis B3 qui le prouve sur l'extraction. B4 à B6 dans l'ordre que voudra c7.

## 7. Ce qui reste à trancher

- le nom du nœud (`Collect`) et de son option (`empty`) ;
- le format de gabarit pour la présentation d'un type : jinja, comme les fichiers de fiche (déjà dans le
  moteur), plutôt qu'une forme de plus ;
- si `archive_candidates` garde un script (pour les alias courts `candidate_N`) ou si les alias deviennent une
  présentation déclarée.
