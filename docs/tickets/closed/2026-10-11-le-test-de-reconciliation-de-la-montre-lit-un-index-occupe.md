# Le test de réconciliation de la montre lit l'état d'un index occupé, et échoue une fois sur cinq

- **État** : ouvert (11 octobre 2026, relevé par l'optimiseur en rejouant son lot sur master)
- **Gravité** : test instable (un rouge sans défaut du produit)
- **Atteignable en service** : non pour le test ; en service, `index_state` d'un index occupé ne
  rend pas les montres, ce que le test suppose
- **Touche rag3weaver** : oui (`src/backend.rs`, test
  `backend::tests::the_watch_reconciles_what_moved_before_it_started`, de 9d80aa873 et 94354595c)

## Ce que c'est

Le test attend que les usages montrent ce que la montre a repris, puis lit
`backend.index_states()["watches"]["code"]["scans"]`. Si une réindexation tient encore le
catalogue à cet instant, `index_states` prend la branche `try_lock` occupée et rend
`{"busy": true}`, sans `watches`. Le test lit alors `null` et échoue à la ligne qui exige
`scans > 0`.

## La recette

```bash
cargo test --features rag3db-native,code,daemon,openai-llm --lib -- the_watch_reconciles_what_moved_before_it_started
```

Joué cinq fois de suite sur le poste, avec le moteur au bâti 6df47a543 : quatre verts, un rouge.
La panne :

```
panicked at src/backend.rs:…: null
```

## Le témoin

Le test lui-même, rejoué en boucle.

## La cause

La branche occupée d'`index_states` (`let Ok(cat) = self.catalog.try_lock() else { return
{"busy": true} }`) rend avant le bloc des montres. Or les statistiques des montres
(`self.watches`) ne demandent pas le catalogue.

## Ce qu'il faut pour le fermer

Deux formes possibles, au choix de l'auteur de la montre :

- poser `watches` aussi dans le reçu occupé, puisqu'il ne dépend pas du catalogue (c'est un
  vrai défaut du reçu : la montre est la plus utile justement quand l'index travaille) ;
- ou faire attendre au test un état non occupé avant de lire.

**11 octobre, 23 h 17 (arbre principal)** : la première forme est faite dans `cce19d031` (en file
de push) : `index_states` lit les montres sur le backend avant de prendre le catalogue, et le
reçu « busy » les porte, comme il porte l'embarquement en cours. Le test n'est pas touché. Reste,
pour fermer, le témoin déterministe (une réaction qui tient le catalogue, puis la lecture de
l'état). À savoir : depuis le même lot, chaque lecture d'outil lit aussi une clé méta (le compte
des morceaux sans texte, pour l'en-tête) sous le verrou du catalogue, ce qui allonge un peu sa
tenue.

**12 octobre, nuit (everything declarative) : fermé.** Le correctif de l'arbre principal est sur
master (`e5c53d2e2`). Le témoin déterministe `a_busy_index_state_still_tells_the_watches`
(`backend.rs`) tient le catalogue 2,5 s, au-delà de l'attente d'une lecture, et exige `busy` **et**
`watches.code` : rouge sur la branche d'avant (`{"busy":true}` seul), vert après. Le reçu occupé
porte aussi `resume` (les fichiers repris à la fin d'une indexation entière), lu hors du catalogue
comme les montres ; et `index_states` attend un verrou court (`lock_for_read`, au plus 1 s) au lieu
de rendre `busy` au premier essai.
