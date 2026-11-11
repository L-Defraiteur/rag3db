# Le service d'embarquement laisse son journal dans le TMPDIR d'e2e_search

- **État** : ouvert, constat isolé pendant la passe partielle 1a, antériorité non comparée.
- **Famille** : produit
- **Gravité** : la suite réussit ses assertions mais sa garde de résidus refuse la passe.
- **Atteignable en service** : non établi ; lancement et nettoyage d'une passe de tests.
- **Touche rag3weaver** : oui, lanceur e2e et service d'embarquement démarré par les tests.
- **Pour** : recherche, qui porte le lanceur ; coordination embarquement si nécessaire.
- **Signalé par** : arbre-principal, passe partielle du lot 1a.

## Preuve

Luciepc, WT cx-wt-scope-version, source HEAD 06b3ee4cb + candidat file/scan/FTS,
empreintes runtime dans `~/.cache/rag3weaver-build/cx-projection-file-regressions-v3.log`.
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
