# La garde des règles de l'analyse lit la mention avec sa casse

- **État** : ouvert (16 novembre 2026)
- **Famille** : outillage
- **Gravité** : petite ; un lot qui dit « Sans effet sur l'analyse » en tête de phrase est refusé par check_features,
  et il faut un commit de plus pour le redire en minuscules.
- **Atteignable en service** : non (la passe et la garde du push seulement).
- **Touche rag3weaver** : oui (`extension/rag3weaver/scripts/check_regles_de_l_analyse.py`, `MENTION in messages`).
- **Pour** : everything declarative (la garde vient de sa page 06, § 13), au mot de c7.
- **Signalé par** : arbre principal, lot `arbre-principal/vues-en-anglais`, 16 novembre 2026.

## Ce que c'est

`check_regles_de_l_analyse.py` cherche la mention par `MENTION in messages`, à la casse près. Un message qui écrit
« Sans effet sur l'analyse: … » en début de ligne ne compte pas. Le lot bfdc4a7ae l'a vu : il a fallu un commit vide
de plus (a76dace1a) pour la redire en minuscules.

## Le correctif attendu

Lire la mention sans casse (`MENTION in messages.lower()`, ou une expression régulière insensible à la casse), et
ajouter au témoin `test_check_regles_de_l_analyse.py` un cas « Sans effet sur l'analyse » en tête de phrase, rouge
d'abord.

## Ce qu'il faut pour le fermer

Le cas rouge puis vert dans le témoin de la garde.
