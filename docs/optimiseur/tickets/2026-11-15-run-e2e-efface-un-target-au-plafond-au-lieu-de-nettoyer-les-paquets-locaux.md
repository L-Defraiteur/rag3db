# run_e2e.sh refuse un target au plafond au lieu de nettoyer ses paquets locaux

- **État** : ouvert (15 novembre 2026), relevé par archi, mis en ticket sur le mot de c7
- **Famille** : produit
- **Gravité** : coût — une passe s'arrête avant de jouer, et l'effacement du target entier coûte
  un rebâti complet (de 5 à 10 min sous poste lourd) à chaque fois
- **Atteignable en service** : non, c'est l'outillage des passes
- **Touche rag3weaver** : oui (`extension/rag3weaver/run_e2e.sh`, sa garde du plafond
  `RAG3WEAVER_TARGET_MAX_GO`)
- **Pour** : G (optimiseur)

## Ce qu'on voit

`run_e2e.sh` mesure le target et refuse au-delà du plafond (32 Go), avec pour seul remède
« rm -rf <target> ». Avec un seul target par session gardé entre les lots, la lib entière, le
navigateur et les e2e, chacun sous ses features, l'y font passer en quelques lots. Le 14 novembre au
soir, trois effacements chez archi — 19 h 20 (36 Go), 21 h 48 (33 Go), 0 h 17 (35 Go) — et un chez
embarquement, chacun suivi d'un rebâti complet.

## Ce qui est attendu

Au plafond, `run_e2e.sh` fait lui-même le ménage des seuls paquets locaux (les crates de rag3weaver,
`cargo clean -p` pour chacun, dans chaque profil et chaque triplet comme `garde_target.sh` les
connaît), le dit au journal avec la taille avant et après, recompte, et ne refuse que si le target
reste au-dessus. Les dépendances tierces, qui ne changent pas d'un lot à l'autre, restent bâties.
