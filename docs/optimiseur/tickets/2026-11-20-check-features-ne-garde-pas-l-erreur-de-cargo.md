# check_features ne garde pas l'erreur de cargo d'une étape rouge

- **État** : ouvert (20 novembre 2026), relevé au rouge de 2.12 au pousseur
- **Famille** : produit (les outils de validation)
- **Gravité** : un rouge de check_features ne dit pas pourquoi ; l'auteur doit rejouer cargo chez lui pour le savoir,
  et le pousseur ne peut rien lui transmettre de plus que le nom de l'étape
- **Atteignable en service** : non (outil de validation)
- **Touche rag3weaver** : oui

## Ce qui s'est passé

Le 20 novembre à 2 h 21, `ed-container-not-candidate` (2.12) est rendu rouge par le pousseur : « ROUGE check_features ».
Le journal de la passe, `check_features.log`, dit seulement :

```
ROUGE sans-feature   (cargo check --tests --no-default-features --features )
CHECK_FEATURES ROUGE sans-feature
```

L'erreur de compilation de cargo n'y est pas : `check_features.sh` jette la sortie de `cargo check` de chaque jeu de
features et n'en garde que le code de retour. ED a dû rejouer la commande chez elle pour trouver la cause.

## Pour le correctif

Pour une étape rouge, garder dans le journal les lignes d'erreur de cargo (`error[…]`, `error:` et leur contexte
`-->`), avec un plafond, une quarantaine de lignes. Un témoin rouge d'abord : un jeu de features dont un test ne compile
pas, et le journal de check_features qui doit en citer l'erreur et le fichier.
