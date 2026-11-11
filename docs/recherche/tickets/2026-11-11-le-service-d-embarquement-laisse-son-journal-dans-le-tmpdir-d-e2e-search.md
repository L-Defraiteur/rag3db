# Le service d'embarquement laisse son journal dans le TMPDIR d'e2e_search

- **État** : ouvert, constat isolé pendant la passe partielle 1a, antériorité non comparée.
- **Famille** : produit
- **Gravité** : la suite réussit ses assertions mais sa garde de résidus refuse la passe.
- **Atteignable en service** : non établi ; lancement et nettoyage d'une passe de tests.
- **Touche rag3weaver** : oui, lanceur e2e et service d'embarquement démarré par les tests.
- **Pour** : recherche, qui porte le lanceur ; coordination embarquement si nécessaire.
- **Signalé par** : arbre-principal, passe partielle du lot 1a.

## Preuve

Luciepc, WT cx-wt-scope-version, source exacte archivée après exécution en
**965b835b8b58cb3937df26b99ed6a982e01d2f0d**, branche origin/cx-arbre-source-cellules-v3.
Le journal porte HEAD 10c0e09ae avec modifications non commitées à l'instant de la passe ;
le snapshot 965b835b8 conserve ces mêmes sources sans modification. Empreintes runtime dans `~/.cache/rag3weaver-build/cx-projection-file-regressions-v3.log`.
Jeu complet de huit features, lib29a7bc52a, target privé cx-target-contenu-derive.
Bibliothèque 2101/0/58 ; cinq suites natives jouées. e2e_search fait 40/0, puis :

```
RÉSIDUS e2e_search : 1 entrée(s) dans son TMPDIR (4,0K)
rag3weaver-demons
rag3weaver-embeddings.log (774 octets)
```

La garde refuse la suite (pas dans la liste de résidus admis) ; le lanceur annonce
ensuite l'arrêt SIGTERM du démon d'embarquement né pendant la passe, pid 949380.
Code retour global 101 : aucun vert global revendiqué. Les tests 1a dérivées/cellules/
Scope/projection passent ; l'étape PG chaînée n'a pas démarré à cause de ce retour.
Le témoin PG est lancé séparément après cette fin, sur notre base privée.

## Restant

Identifier le propriétaire du journal et l'ordre arrêt/archivage/nettoyage par suite.
Préserver la preuve avant nettoyage, ne pas ajouter une exemption de résidu pour verdir.
Comparer si nécessaire à master dans les mêmes conditions de service absent ; aucune
régression causale 1a n'est établie. Aucun processus tiers ne doit être arrêté.
Ce ticket trace le défaut ; il ne demande pas d'ouvrir un nouveau lot sans orchestration.

## Provenance figée pour le diagnostic

Journal absolu : `/home/lucied/.cache/rag3weaver-build/cx-projection-file-regressions-v3.log`
SHA256 `065612359f78a6a1320db86649763dcf08e3b5924fe9a2ed676301e52658bac9`.
Script absolu : `/home/lucied/.cache/rag3weaver-build/cx-projection-file-regressions-v3.sh` ;
il appelle `cx-projection-file-regressions.sh` et ensuite `cx-contenu-pg.sh` dans ce même dossier.
Lanceur versionné : `extension/rag3weaver/run_e2e.sh` du snapshot 965b835b8.
Binaire e2e exact `cx-target-contenu-derive/debug/deps/e2e-edaf884b121d1775`, SHA256
`97e438e4dae5a42e6baaea3a2077ebb8fb10125231ea2212a9a4cc13bea8d120`.
Empreinte conservée hors target dans `cx-v3-binaire.sha256`.
Diff binaire original de HEAD10c : `cx-source-exact-v3.patch`, SHA256
`98a616232e8cc85c683c645e6daeb7c7fb55fb5a606f0e75f2b88cef7d96352d`.
Le checkpoint d'avancement 261ff54fc contient des ajustements ultérieurs non joués par v3 ;
il ne doit pas être confondu avec la source exacte 965b835b8.
