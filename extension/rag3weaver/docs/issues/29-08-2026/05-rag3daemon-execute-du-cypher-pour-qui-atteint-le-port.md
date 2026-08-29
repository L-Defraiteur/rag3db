# rag3daemon exécute du Cypher pour quiconque atteint le port

**Ouvert le 29 août 2026**, le jour même où rag3daemon est né
([issue 03 §9](03-un-demon-et-la-fin-du-tout-synchrone.md)).

## Ce que c'est

`POST /cypher` exécute ce qu'on lui donne, sans authentification ni chiffrement.
`--base` accepte n'importe quel chemin. Il n'y a ni utilisateur, ni permission,
ni distinction entre lire et écrire.

## Pourquoi ce n'est pas un défaut aujourd'hui

Sur `127.0.0.1`, **la frontière de confiance est exactement la même que celle du
fichier** : qui peut joindre le port pouvait déjà ouvrir la base. Le démon
n'ouvre rien de neuf. C'est pour ça que ce n'est pas bloquant, et pour ça qu'on
l'a écrit comme ça.

## Pourquoi il faut l'écrire maintenant

Parce que la seule chose qui sépare les deux mondes est **un argument de ligne
de commande**. `--adresse 0.0.0.0:7979` et la frontière disparaît, sans que rien
dans le code ne le signale. C'est le genre de bascule qu'on fait un mardi pour
essayer, et qu'on oublie.

Et c'est le vrai préalable de « plusieurs machines »
([issue 04](04-plusieurs-machines-ce-qui-bloque.md)) — bien avant la
réplication.

## Ce qu'il faudrait, par ordre de coût

1. **Refuser de servir hors boucle locale sans le dire.** Si l'adresse n'est pas
   sur `127.0.0.0/8`, exiger un drapeau explicite (`--exposer`) et un jeton.
   Quelques heures, et ça ferme la bascule silencieuse.
2. **Un jeton partagé**, en en-tête, comparé en temps constant. Un jour.
3. **TLS**, ou un tunnel — `rustls` est déjà dans l'arbre via `ureq`.
4. **Lecture seule par client** : un jeton qui ne donne que `read_only`. Le
   moteur sait déjà ouvrir ainsi (`Rag3dbConnection::read_only`), mais un démon
   ne sert qu'une connexion ; il faudrait deux connexions, ou une inspection de
   la requête — et **inspecter du Cypher pour décider s'il écrit est un piège**,
   pas une solution. Deux connexions et deux ports est plus honnête.

## Ce qui n'est pas décidé

Rien. Le point 1 est le seul qui presse, parce qu'il coûte des heures et qu'il
supprime la bascule silencieuse.
