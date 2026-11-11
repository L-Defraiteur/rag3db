# Le service d'embarquement laisse son journal dans le TMPDIR d'e2e_search

- **État** : ouvert ; diagnostic borné effectué par recherche sur les sources figées et le journal, correctif non autorisé, antériorité dynamique non comparée.
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
il exécute lui-même la lib puis run_e2e (étapes aussi présentes dans `cx-projection-file-regressions.sh`), puis `cx-contenu-pg.sh` dans ce même dossier. Ce n’est pas un appel au premier script.
Lanceur versionné : `extension/rag3weaver/run_e2e.sh` du snapshot 965b835b8.
Binaire e2e exact `cx-target-contenu-derive/debug/deps/e2e-edaf884b121d1775`, SHA256
`97e438e4dae5a42e6baaea3a2077ebb8fb10125231ea2212a9a4cc13bea8d120`.
Empreinte conservée hors target dans `cx-v3-binaire.sha256`.
Diff binaire original de HEAD10c : `cx-source-exact-v3.patch`, SHA256
`98a616232e8cc85c683c645e6daeb7c7fb55fb5a606f0e75f2b88cef7d96352d`.
Le checkpoint d'avancement 261ff54fc contient des ajustements ultérieurs non joués par v3 ;
il ne doit pas être confondu avec la source exacte 965b835b8.

## Diagnostic borné de recherche : ordre et propriété

Aucune passe ni processus relancé pour ce diagnostic, aucune exemption ni modification de code. Relecture du journal à luciepc : SHA256 conforme à `065612359f78a6a1320db86649763dcf08e3b5924fe9a2ed676301e52658bac9`. Les scripts relus ont les SHA256 `8a815de7edb8b04ec4c69adc9f125c7910f283f1bcb42ff0e60effb9d3eae37f` (v3) et `9b14ab6c4ee459dbe05747fb45fab8dc442a55c1feaea8cc116437d028b3e258` (cx-projection-file-regressions.sh). Le premier appelle lib puis run_e2e sous `set -e`, avant la commande PG ; un retour 101 empêche bien cette dernière.

Dans le journal : ligne 2887, le chemin de remplacement d'un démon périmé est emprunté ; ligne 3089, BGE-M3 s'attache au démon de 127.0.0.1:7878. Les 40 assertions sont vertes ligne 3201 (17,26 s), le résidu de 774 octets est identifié lignes 3203–3206. La suite projection continue dès 3207 ; seulement à la fin de passe, ligne 3249, le trap annonce SIGTERM pour le démon marqué de cette passe, pid 949380. La mention SIGTERM ne prouve pas une attente de fin du processus.

Sources relues au snapshot exact `965b835b8b58cb3937df26b99ed6a982e01d2f0d` :

1. `tests/common/mod.rs::bge_m3_par_le_demon` configure `journal_dans(std::env::temp_dir().join("rag3weaver-demons"))`. Or run_e2e passe à l'enfant `TMPDIR=$E2E_TMP_PARENT/$nom` : le journal devient une occurrence de la suite qui lance le démon.
2. `src/daemon/embeddings.rs::DaemonEmbedder::serveur` pose `Fin::Laisser` ; le démon doit survivre à cette suite pour servir les suivantes. `src/serveur.rs::lancer_avec` crée le dossier et ouvre stdout/stderr en ajout dans ce journal.
3. `run_e2e.sh`, ligne 943 de ce snapshot : contrôle des résidus immédiatement après le retour du processus de suite. Le démon est encore de portée passe ; son journal est donc pris pour un résidu de portée suite. C'est un désaccord entre durée de vie et emplacement, pas une assertion de recherche ratée.
4. L'archivage de la suite regarde le statut brut du processus (`statut != 0`, ligne 951), alors que le garde ne change que le code global. Avec assertions vertes et garde rouge, cette branche d'archivage n'est pas prise ; le nettoyage par défaut du TMPDIR arrive ligne 962 avant l'arrêt final des démons. Cet archivage ne préserve donc pas automatiquement le journal de 774 octets ; le log prouve sa taille et son chemin, pas son contenu.
5. Le trap final sélectionne les démons par `RAG3WEAVER_PASSE_E2E`, envoie SIGTERM puis poursuit le nettoyage, sans attendre explicitement leur disparition. Il ne faut pas transformer cette sélection en arrêt par nom de tous les services.

**Limite de la preuve sur les processus tiers :** le trap final vise ceux marqués de la passe, mais le chemin commun a auparavant demandé le remplacement d'un démon périmé (ligne 2887). Le code appelle alors `quitter()` sur l'attache précédente avant de relancer. Le journal ne donne pas l'identité/propriété de ce premier processus ; on ne peut donc pas déduire de la seule sélection du trap que tout le parcours a laissé les services préexistants intacts. Aucun processus n'a été touché pendant notre diagnostic.

## Antériorité et causalité 1a

Comparaison statique de ce snapshot avec master `a83fa83e22beaa15e7be1306962402605e2382e9` : `tests/common/mod.rs`, `src/serveur.rs` et `src/daemon/embeddings.rs` sont identiques ; le diff de run_e2e ne touche que l'exclusion explicite de suites `RAG3WEAVER_E2E_SAUF`, pas les chemins de journal, garde, archivage et arrêt décrits ici. Le mécanisme est donc aussi présent dans ce master. Aucune comparaison dynamique master/1a à conditions égales n'a été jouée : une régression causale 1a n'est pas établie, et le déclenchement précis reste à départager si nécessaire. Les exports de services hérités au moment de v3 n'ont pas été capturés ; le chemin réellement emprunté est en revanche visible dans le log.

## Proposition soumise à l'orchestration, non implémentée

Conserver les primitives existantes (`Serveur::journal_dans`, attache et marque de passe), en alignant les ressources sur leur propriétaire : un journal de démon partagé entre suites doit vivre dans un dossier disque appartenant explicitement à la passe, hors TMPDIR de chaque suite. Pas d'exemption de fichier ou de suite dans le garde.

Ordre attendu : fin du processus de suite, contrôle de ses résidus, conservation de la preuve sur tout rouge (y compris celui du garde), puis nettoyage de ses seules ressources ; fin de passe, arrêt et attente bornée des seuls démons démarrés par elle, conservation/archivage de leurs journaux, puis nettoyage de leurs ressources. Un service déjà présent doit rester sous son propriétaire ; départager aussi le chemin de remplacement périmé avant de promettre cette garantie.

Avant autorisation d'un lot, proposer un témoin léger du lanceur avec faux service possédé par le test, deux suites et journal réel : partage entre suites, absence de résidu de suite, conservation du journal sur rouge du garde, arrêt attendu à fin de passe et service préexistant laissé intact. Aucun GPU ni réexécution lourde nécessaire pour ce témoin. Le ticket reste ouvert jusqu'au correctif autorisé et sa validation ; transmission et diagnostic seuls ne le ferment pas.
