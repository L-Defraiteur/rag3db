# Le juge ne rend que dix-huit bilans sur vingt-six en dix minutes

- **État** : ouvert (11 novembre 2026), relevé pendant la passe du reste explicite ; porteur everything declarative.
- **Famille** : produit
- **Gravité** : pas de verdict vert global : les bilans de tous les pas ne sont pas disponibles dans la borne de la suite.
- **Atteignable en service** : oui, un équipier peut produire des pas plus vite que leur jugement ; le débit et la cause restent à départager.
- **Touche rag3weaver** : oui, boucle équipier, jugement des pas, `tests/e2e_equipier.rs`.

## Observation et preuves

Sur la source `51cf79843`, tête de passe `446a80a38`, jeu large et Gemini réel
`google/gemini-3.8-flash`, l'équipier livre le README et son report après **26 pas en 491 s**,
fin « fait sans plan ». Les vérifications du livrable et du dépôt principal passent.
La suite attend ensuite les bilans de chaque pas, selon sa borne existante de 600 s.
À l'échéance, la fiche `bilan : doc` contient **18 apports pour 26 pas** ; la suite échoue
à `e2e_equipier.rs:162`, 0 réussi / 1 échec / 0 ignoré, 1093,93 s au total.

Preuve luciepc : `~/.cache/rag3weaver-build/I-luciepc/reste-explicite-20261111/e2e_equipier.log`.
Les suites précédentes sont distinctes : features 5/5, lib 2102 réussis / zéro échec /
61 ignorés existants, navigateur 36/36, lectures 24/24, juge 5/5 sur Gemini. La passe globale
reste ROUGE. Aucun changement de prompt, d'assertion, de borne ou de modèle ; aucun retry
identique lancé pour masquer ce résultat.

La durée et la quantité de pas pour un README court sont constatées, pas expliquées.
Cette exécution ne démontre ni une régression causée par la correction remaining ni une
panne d'infrastructure ; comparer les données des pas et la file du juge avant de conclure.
La correction du reste explicite a ses preuves séparées sur faux modèle : rouge initial,
continuation et reprise avec curseur déjà lu, garde de la petite demande.

## Pour fermer

Départager le travail répété de l'équipier, le débit et l'état de la file des bilans ; conserver
les traces des sources, appels, durées et attentes. Corriger la cause démontrée dans un lot
séparé, sans rendre le juge autorisateur de la tâche. Le scénario doit livrer ses bilans
attendus dans le contrat de durée retenu et justifié ; ne pas transformer 18/26 en vert ni
augmenter la borne sans décision étayée. Aucun correctif dans le lot relais.
