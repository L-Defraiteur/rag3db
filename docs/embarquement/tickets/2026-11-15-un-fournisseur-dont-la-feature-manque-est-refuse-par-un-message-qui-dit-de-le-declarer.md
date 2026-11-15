# Un fournisseur dont la feature manque est refusé par un message qui dit de le déclarer

- **État** : ouvert (15 novembre 2026, embarquement ; relevé par tty)
- **Pour** : embarquement
- **Famille** : produit
- **Gravité** : défaut. Le refus envoie sur une fausse piste : il dit de déclarer ce qui est déjà déclaré.
- **Atteignable en service** : oui (un binaire bâti sans `openai-llm`, ou sans une autre feature de fournisseur)
- **Touche rag3weaver** : oui (`src/model_source.rs`, la résolution d'une déclaration)

## Ce que c'est

Sans la feature `openai-llm`, une déclaration `models.llm` en `provider: compatible, protocol: openai` est refusée.
Le message est « models.llm (m-fin) : cette capacité n'a pas encore de démon — déclarer `local` ou `compatible` »
(`src/model_source.rs:272`). Or la déclaration dit déjà `compatible`. Ce qui manque, c'est la feature du binaire,
que le message ne nomme pas. Le 15 novembre 2026, ce refus a fait rougir la lib entière sans `openai-llm` (le témoin
`modele_de_tache` de tty-14) et a coûté une heure de départage entre trois sessions.

## Ce qu'il faudrait

Pour chaque fournisseur qu'une feature porte, un refus qui nomme la feature absente et ce qu'elle apporte :
« models.llm (m-fin) : le fournisseur compatible/openai n'est pas compilé dans ce binaire (feature `openai-llm`) ».
Le message actuel ne resterait que pour une capacité qui n'a réellement aucun fournisseur de ce type.

Témoin, rouge d'abord : sans la feature, une déclaration `compatible`/`openai` est refusée par un message qui nomme
`openai-llm`, et pas par « déclarer `compatible` ».
