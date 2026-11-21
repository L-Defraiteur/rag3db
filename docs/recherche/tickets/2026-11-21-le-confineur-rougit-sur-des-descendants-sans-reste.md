# Le confineur rougit sur des descendants laissés, avec une liste de restes vide

- **État** : ouvert (21 novembre 2026, session recherche, signalé par c7 depuis les passes de la nuit à luciepc).
- **Gravité** : faux rouge : une passe toute verte est jugée rouge, et le lot est rejoué pour rien.
- **Atteignable en service** : oui (le pousseur, chaque passe confinée).
- **Touche rag3weaver** : oui (`extension/rag3weaver/scripts/confiner_validation.py`, le verdict des descendants).
- **Famille** : produit
- **Pour** : recherche (suivi ; toute réattribution passe par l'orchestration).
- **Signalé par** : c7.

## Le pas exact

Deux fois dans la nuit du 20 au 21 novembre, à luciepc, sur deux suites et deux branches de la mémoire :
`e2e_lectures` (28 passés, 0 rouge, `validation-2ayzs6tu`) et `e2e_archiviste` (48 passés, 0 rouge,
`validation-db07_i31`). Toutes les épreuves sont vertes ; le verdict dit « descendants laissés après la commande »
avec `restes: []` et `nettoye: true`. `enfants.json` ne nomme aucun processus. Le rejeu seul passe.

## Lecture

Un descendant est compté vivant à un relevé, puis meurt avant le relevé qui nomme les restes : le verdict garde le
compte du premier et la liste vide du second. La grâce de fermeture (1 s, landed le 20 nov.) ne couvre pas ce cas, car
le mort ne paraît plus nulle part.

## Ce qui est faux

Un verdict qui rougit pour des descendants doit en nommer au moins un ; `restes: []` avec ce rouge est une
contradiction du confineur, pas un défaut de la suite.

## Pour le fermer

- Témoin rouge d'abord : un descendant qui meurt entre le relevé du compte et celui des restes ne fait pas rougir le
  verdict ; un descendant toujours vivant après la grâce le fait toujours (le témoin du 20 nov. reste vert).
- Le verdict des descendants se juge sur une seule liste, relue après la grâce : vide, vert.

## Le contournement

Rejouer la suite seule ; regarder `restes` et `enfants.json` avant de conclure à une fuite.
