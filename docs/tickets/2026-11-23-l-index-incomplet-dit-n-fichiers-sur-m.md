# L'index incomplet dit « N fichiers sur M », et l'outil `index` offre la reprise

- **État** : ouvert (23 novembre 2026), rag3weaver archi.
- **Famille** : produit
- **Gravité** : une lecture sur un index partiel dit seulement « index incomplet », avec la source et l'heure de départ ; on ne sait pas combien de fichiers manquent.
- **Atteignable en service** : oui.
- **Touche rag3weaver** : oui (`backend.rs`, `say_index_incomplete` ; l'outil `index`).
- **Pour** : archi.
- **Signalé par** : c7, au dépôt des lots `sync-session` et `index-incomplete`.

## Ce que c'est

La ligne « > index incomplet » doit dire aussi combien de fichiers sont indexés sur combien en a le dépôt. L'outil `index`, face à une session vivante d'un autre processus, doit offrir la reprise (`takeover`) au lieu d'un refus sans issue. Une session orpheline est déjà reprise d'office.
