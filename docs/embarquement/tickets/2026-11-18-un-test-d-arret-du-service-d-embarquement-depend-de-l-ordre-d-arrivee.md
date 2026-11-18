# Un test d'arrêt du service d'embarquement dépend de l'ordre d'arrivée

- **État** : ouvert (18 novembre 2026) — observé une fois sous charge, vert seul trois fois sur trois
- **Famille** : produit
- **Gravité** : faux rouge d'une passe complète (la lib entière), sans défaut du service
- **Atteignable en service** : non, c'est le test (`daemon::embeddings::tests`)
- **Touche rag3weaver** : oui, `src/daemon/embeddings.rs`, la passe de la lib
- **Pour** : session embarquement
- **Signalé par** : session codeparsers, passe du lot des mots vides de Python et de C (journaux `passe-stopwords-pyc`, lib 3d706f709, 18 novembre vers 17 h 40)

## Constat

`l_arret_attend_d_avoir_repondu_aux_requetes_en_vol` envoie « boum-lent » à 0 ms (le faux embarqueur
`CarteRefusee` tient la passe 600 ms puis panique) et « après » à 200 ms, et suppose que « boum-lent » tient
déjà la passe quand « après » arrive. Sous la charge d'une passe complète, le fil de « boum-lent » a démarré
après celui d'« après » : « après » a été servi et a réussi, et le test a rougi sur
`expect_err("après la panique : refusée")` (« refusée: () », `embeddings.rs:1726`). La panique « failed to
reserve 4102146304 bytes » du journal est celle que le faux embarqueur simule, pas la carte.

Rejoué seul, sous `poste lourd`, sur la même lib et le même jeu de features : vert trois fois sur trois.

## Ce qu'il faudrait

Envoyer « après » quand le service a **commencé** la passe de « boum-lent » (un signal du faux embarqueur à
l'entrée d'`embed`, une barrière), plutôt qu'après un délai fixe. Le commentaire du test dit déjà qu'une
première forme comptait sur l'ordre du verrou ; c'est le même piège par l'horloge.
