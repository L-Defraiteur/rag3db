# Les services de modèles n'ont aucune authentification

- **État** : ouvert — accepté tel quel aujourd'hui (« plus tard faudra ajouter
  des gardes d'authentification, mais là on s'en fout », Lucie, relayé le
  4 octobre 2026)
- **Gravité** : à faire avant toute exposition hors d'un tunnel
- **Atteignable en service** : oui, mais seulement depuis l'un des deux postes
- **Touche rag3weaver** : oui (le démon `rag3weaver-embeddings` et ses
  clients ; les `llama-server` de décision et de langage aussi)

## Ce que c'est

Les services de modèles de l'autre poste — embarquement, creux, relecteur,
OCR (`rag3weaver-embeddings`), décision et petit modèle de langage
(`llama-server`) — répondent à quiconque les joint. Ils n'ont ni jeton, ni
chiffrement, ni quota.

## Ce qui protège aujourd'hui

- Chaque service écoute sur `127.0.0.1` seulement (`--adresse 127.0.0.1:78xx`,
  `--host 127.0.0.1`).
- Le poste qui s'en sert les joint par un tunnel ssh
  (`ssh -L 127.0.0.1:79xx:127.0.0.1:78xx`). C'est ssh qui authentifie et qui
  chiffre.

## Ses limites

- **Tout processus local de l'un ou l'autre poste peut appeler un service** :
  sur le poste qui sert, n'importe quel programme joint `127.0.0.1:78xx` ; sur
  le poste client, n'importe quel programme joint le bout local du tunnel
  (`127.0.0.1:79xx`). Il peut embarquer, relire, faire de l'OCR ou du texte à
  volonté, et occuper la carte.
- **Pas de chiffrement hors du tunnel** : la protection tient tant que rien
  n'écoute ailleurs que sur la boucle locale.
- Rien n'empêche aujourd'hui de lancer un service sur `0.0.0.0` par erreur :
  il serait alors ouvert à tout le réseau.

## La recette

Sur le poste qui sert : `curl -s http://127.0.0.1:7878/sante` répond sans
rien demander. Même chose depuis le poste client par le tunnel
(`127.0.0.1:7979`).

## Le témoin

Aucun. À écrire avec le correctif : un démon lancé sur une adresse non locale
sans jeton refuse de démarrer ; une requête sans jeton ou avec un mauvais
jeton est refusée.

## Ce qu'il faut pour le fermer

Le jour où un service écoute ailleurs que sur la boucle locale :

- **un jeton par service**, déclaré dans `models.<capacité>` du manifeste
  (`api_key_env` existe déjà pour les fournisseurs tiers : la même clé, lue
  par le client et par le démon) ;
- **TLS** sur le service, ou un proxy qui le porte ;
- **des quotas** (requêtes, caractères, temps de carte) par jeton ;
- **le refus de démarrer** sur une adresse non locale sans jeton configuré.
