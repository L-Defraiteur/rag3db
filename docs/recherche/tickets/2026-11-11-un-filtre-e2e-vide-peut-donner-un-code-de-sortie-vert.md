# Un filtre e2e vide peut donner un code de sortie vert

- **État** : ouvert (11 novembre 2026, session recherche), preuve relue sur le rejeu de l'arrêt du backend.
- **Famille** : produit
- **Gravité** : faux verdict de validation si le lanceur ne vérifie que le code de sortie.
- **Atteignable en service** : non, outillage de validation.
- **Touche rag3weaver** : oui (`run_e2e.sh` et scripts de passe).

## Preuve

À luciepc, `~/.cache/rag3weaver-build/journaux-cx-recherche-arret-11nov/temoins.log` sur `762d93a20` : `run_e2e.sh --features openai-llm,anthropic-llm --test e2e_lectures qu_on_laisse_tomber` retourne zéro. Ses deux tests réussis sont ceux de `e2e_filets` ; la suite demandée joue ensuite **zéro test**. Le filtre visait en réalité deux tests unitaires de `daemon_host`, pas des e2e. Le lanceur ne peut pas compter les filets comme témoins du lot.

La lib entière historique a réellement exécuté les deux témoins : preuve `journaux-recherche-arret-reprise-sans-derogation/lib.log:1625,1639`. Cela ne rend pas valide l'étape ciblée vide. Le verdict final du lot doit lire un rejeu ciblé `cargo test --lib ... qu_on_laisse_tomber` qui nomme les deux tests et vérifie leur compte, en plus des autres étapes.

## Pour fermer

Refuser un filtre qui ne sélectionne aucun test dans les suites demandées, indépendamment des filets du binaire. Ajouter un témoin du lanceur avec filtre absent et code de sortie non nul, puis un filtre réel qui joue son test. Préserver les cas explicitement déclarés comme mesures hors passe ; aucun succès par absence de travail à juger.
