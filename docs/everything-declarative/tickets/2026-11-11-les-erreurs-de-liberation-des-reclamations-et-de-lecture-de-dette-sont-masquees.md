# Les erreurs de libération des réclamations et de lecture de dette sont masquées

- **État** : ouvert (11 novembre 2026), relevé à la relecture du lot réclamations ; porteur everything declarative.
- **Famille** : produit
- **Gravité** : un échec SQL peut être présenté comme une libération réussie ou une fin sans dette.
- **Atteignable en service** : oui, si l'exécution SQL, la lecture du catalogue ou sa prise de verrou échoue.
- **Touche rag3weaver** : oui, `src/catalog.rs` et `src/dataflow/index_nodes.rs`.

## Constat statique

Le lot `cx-ed-reclamations-rendues-6` introduit `Catalog::rendre_mes_reclamations` :
`if let Ok` ignore une erreur SQL de libération. Sa valeur retournée vaut alors zéro pour
la table concernée ; elle ne prouve pas que les réclamations ont été rendues.
Le fil d'index ignore également un verrou empoisonné lors de cette libération.
Dans `run_index`, `lock().ok().and_then(...index_progress().ok()).map(...).unwrap_or(0)`
transforme une erreur de verrou ou de lecture de dette en zéro et peut annoncer une fin
sans dette, alors que celle-ci est inconnue. Le bon résultat sur le chemin nominal ne
valide pas ce chemin d'erreur. Aucun témoin d'injection d'erreur n'a encore été joué.

## Contrat attendu

La libération doit rendre un résultat explicite, distinguant le nombre libéré d'une erreur
avec table et cause. La fin du journal doit exposer l'échec de libération sans effacer la
cause d'arrêt ou d'échec initiale. Une dette illisible est inconnue/en échec, jamais zéro.
Ne pas inférer la réussite d'un UPDATE PostgreSQL depuis un jeu de lignes vide : sa forme
ne rend pas actuellement de compteur par RETURNING.

## Pour fermer

Un lot distinct, rouge d'abord : connexion injectant une erreur sur la requête de libération,
puis sur `index_progress`. Le journal final conserve l'erreur et ne dit pas dette zéro ;
l'échec initial reste lisible si la libération échoue aussi. Vérifier les deux dialectes,
le chemin nominal et la fin arrêtée. La correction ne consiste pas à réessayer silencieusement
ou à changer le témoin nominal.
