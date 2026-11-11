# Le service d'embarquement laisse son journal dans le TMPDIR d'e2e_search

- **État** : ouvert ; diagnostic borné effectué par recherche sur les sources figées et le journal, correctif borné autorisé par orchestration, avant revue et sans passe lourde, antériorité dynamique non comparée.
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

## Préparation autorisée du correctif

Orchestration autorise un worktree neuf depuis master, même ticket, code limité au lanceur et aux aides de tests : journal de portée passe, archivage rouge du processus ou du garde, arrêt et attente bornée des seuls démons créés par la passe, aucun remplacement de service tiers même périmé. Témoin léger à faux service/deux suites, sans GPU ni service réel. Partager patch et preuve avant toute passe lourde ; aucun push master autorisé pour cette préparation. Reprise dans cx-recherche-journal-embarquement depuis master630f5bdf8, sans importer le parent 1a des commits documentaires.

## Correctif préparé, revue requise avant passe lourde

Source rebasée `4e9cb120b4f564602060b5663cf8d16d5bbc6596`, branche `cx-recherche-journal-embarquement-2` depuis master incluant le lot ED. Diff limité au lanceur, à tests/common, au helper d'arrêt et au témoin léger, plus ce ticket ; aucun parent 1a importé. Aucun changement du runtime produit ni de la liste des résidus admis.

- Le lanceur fournit aux aides de tests un dossier de journaux de portée passe sur disque, hors TMPDIRsuite, via `RAG3WEAVER_E2E_JOURNAUX_DEMONS` et la primitive existante `journal_dans`.
- Rouge de processus ou de garde : les dossiers disque et TMPDIR de la suite sont conservés ensemble ; une archive ratée laisse les parents en place.
- La fin de passe sélectionne seulement ses démons marqués, vérifie propriétaire et naissance, signale par pidfd puis attend leur disparition effective au plus dix secondes. Aucune escalade SIGKILL du lanceur. Refus ou délai dépassé : retour 101 et ressources conservées. Une fois terminés seulement, leurs journaux sont archivés hors des parents avant nettoyage.
- L'aide de test refuse le démon préexistant périmé au lieu de lui demander de quitter ; le service déjà présent n'est jamais remplacé par ce chemin. Un appel hors lanceur garde le chemin de journal de repli antérieur ; ce lot ne promet pas de nettoyer tous les tests lancés à la main.

### Preuve légère

`python3 scripts/test_journaux_demons_e2e.py`, sans Cargo, modèle, GPU ni service réel. Le témoin charge les fonctions effectives du lanceur et compile la vraie fonction bge_m3_par_le_demon avec le vrai module Serveur, un adaptateur minimal DaemonEmbedder (identité/protocole simulés) et des services HTTP privés. Ce n'est pas une validation de l'API d'embarquement réelle ; elle reste à faire dans la passe autorisée ultérieurement.

Même témoin final à six cas : ancien code de `4509efedf` (sources produit inchangées depuis master630f) = un vert / cinq rouges en 2,219 s ; correctif rebasé = six verts en 12,630 s. Cas : partage d'un seul démon entre deux suites et attente effective, rouge du garde réellement détecté et archivé, rouge du processus avec TMPDIR archivé, service existant courant intact, existant périmé refusé intact, refus de nettoyer après dépassement de délai. Les anciennes assertions de recherche 40/0 du cas réel ne sont pas recyclées comme preuve du correctif.

Journaux locaux hors worktree/target : `~/.cache/rag3weaver-build/cx-recherche-journal-embarquement-{rouge-2,vert-rebase,regressions}.log`. Les régressions existantes `scripts/test_run_e2e.sh` passent également ; syntaxe bash et diff vérifiés. Les six cas n'arrêtent que leurs faux processus privés (la simulation du chemin périmé d'avant correction en fait partie). Aucun service 7878/7970 ou GPU du poste n'a été lancé/arrêté pour ce lot.

### Reste à faire

Orchestration : relire patch et preuve légère, puis décider du rebase/périmètre et de l'autorisation de passe lourde. Recherche : appliquer les remarques, puis seulement jouer la validation autorisée. Pas de push master ni de tour demandé à ce stade. Le ticket reste ouvert. Les contrôles de propriété, quiescence et conservation sont les invariants cibles du correctif, pas des exemptions de résidus à retirer pour verdir.

## Revue : la conservation doit survivre au lancement suivant

Orchestration bloque le lourd sur 663d85781 : un démon qui ignore TERM survit au PID de passe ; les parents conservés restent dans les motifs que la passe suivante balaie seulement d'après ce PID disparu. Même défaut pour les parents TMP et disque. Nouveau témoin léger à deux lancements : premier arrêt refusé/101, puis deuxième lancement laissant démon et ressources intacts. Même ticket, correction de la preuve de quiescence du balayage ; aucune exemption ni service réel.

### Correction du balayage après revue

Le nouveau témoin à deux lancements est rouge sur `930a8fc92` : premier retour 101 après refus d'arrêt, second lancement efface le journal vivant (0/1, 10,584 s). Preuve locale conservée `~/.cache/rag3weaver-build/cx-recherche-journal-deux-lancements-rouge.log`.

Révision `18ea54b78` : arrêt et balayage partagent désormais l'inventaire des démons d'embarquement. Le balayage des parents TMP **et** disque réutilise la marque héritée `RAG3WEAVER_PASSE_E2E` : le préfixe PID de passe est déjà dans cette marque. Un PID de lanceur disparu n'autorise plus l'effacement si l'un de ces démons porte encore cette provenance ; une identité/inventaire non vérifiable conserve également les données. Un PID recyclé peut conserver trop longtemps, jamais permettre un effacement précoce. Aucun fichier d'exemption ni nouvelle métadonnée de bail ne sont ajoutés. La même lecture d'identité/naissance sert aux deux chemins ; un simple changement d'état R/S ne fait plus perdre le propriétaire.

Après quiescence des démons d'embarquement de la passe, les parents anciens sont déplacés vers la primitive existante `e2e-gardes`, avec leurs contenus, plutôt qu'effacés sans preuve. Cette reprise ne les déplace pas sous un écrivain vivant ; les chemins restent intacts jusqu'à la fin de leur propriétaire. Le nettoyage reste limité aux ressources des tests, aucun retrait de worktree n'est effectué par ces fonctions.

Le témoin final renforce le cas de revue : le premier faux démon ignore TERM ; le deuxième lancement ne rajoute **aucun** signal, ne lui demande pas de quitter, préserve son identité vivante et les octets des deux parents. Après une fin déclenchée par le propriétaire du faux service, un troisième lancement archive séparément parents TMP/disque et conserve leurs preuves. Les autres six cas restent verts. Résultat sur `18ea54b78` : 7/7 en 23,399 s ; 19 contrôles existants du lanceur verts, bash et diff vérifiés. Journaux `cx-recherche-journal-quiescence-vert-2.log` et `cx-recherche-journal-quiescence-regressions-2.log` hors worktree/target.

Essai intermédiaire `813b525ac` conservé dans `cx-recherche-journal-quiescence-vert.log` malgré son nom : 6 verts/1 rouge, car un inventaire de tout /proc rencontrait un processus sans rapport non lisible et conservait les parents même après la fin du faux service. La révision finale borne la vérification au même inventaire d'embarquement que l'arrêt, sans ignorer un candidat illisible. Ce rouge n'est pas promu en vert. Toujours aucun modèle/GPU ni service réel, aucune passe lourde ni push master ; nouvelle revue nécessaire avant la suite.

## Plan fermé proposé à l'orchestration, lancement non autorisé à ce stade

Source figée `43b57eb78` (code relu `18ea54b78`, delta documents seuls). Plan JSON local hors worktree : `~/.cache/rag3weaver-build/cx-recherche-journal-plan-43b57eb78.json`, empreintes des sept fichiers source/manifestes critiques incluses. Pas de rebase, mutation source ou extension de suites pendant cette passe ; si master impose du code, arrêter et soumettre un nouveau plan.

Relecture indépendante de l'orchestration : copie des seuls fichiers versionnés de `18ea54b78`, sept tests verts en **23,400 s**, journal reviewer distinct `~/.cache/orchestration-notes/journal-review/tests-18ea54b78.log`. Cette preuve est distincte des 23,399 s auteur.

Ordre exact et bornes (secondes), sur luciepc sous poste lourd, un seul target97k neuf pour recherche, jobs8, plafond32 Go ; lib native complète 29a7bc52a épinglée sans MOTEUR_ANCIEN :

1. Les sept faux-service (120 s), les 19 régressions légères existantes du lanceur (60 s), garde tickets et syntaxe bash.
2. check_features.sh (2400 s au total), ses cinq jeux exacts : `rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres` ; le même avec `openai-llm,anthropic-llm` ; `rag3db-native,code,daemon,openai-llm` ; `rag3db-native,code` ; `rag3db-native`.
3. Preuve cargo check --tests sous jeu large (600 s), sans modification d'octets ; la seule mise à jour mtime de src/lib.rs sert à rendre la compilation visible si elle est autrement fraîche.
4. Lib entière sous `rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres,openai-llm,anthropic-llm` (900 s), comptes non nuls et ignorés lus dans ce binaire, pas de nombre repris d'un ancien master.
5. Navigateur séparé, même jeu + `navigateur` (600 s), **39** témoins attendus et zéro ignoré ; paquets React préparés déjà présents, EXIGER_NAVIGATEUR=1. Chrome est lancé par son helper existant avec --disable-gpu.
6. UN appel réel `run_e2e.sh --test e2e_search --test e2e_search_queue`, avec RAG3WEAVER_E2E_FEATURES fixé au jeu large (1200 s). Attendus : e2e_search40, search_queue5, zéro ignoré/saut ; filets implicites2 comptés à part. Pas d'autre e2e, juge, banc, compatibilité ou mesure ajouté.
7. Attestation source/lock (octets identiques), noms/comptes réellement joués, chemins et contenus des gabarits dans les binaires lib/e2e/navigateur (120 s), journaux et SHA hors target. Tout rouge/expiration arrête la séquence ; pas de retry automatique. Borne globale6600 s avec arrêt de l'arbre propre à cette passe avant de rendre le lourd.

Services : RAG3WEAVER_SERVICE_EMBED=`127.0.0.1:7878,127.0.0.1:7879,127.0.0.1:7880`, sans alias ambigu hérité. Santé relue sans inférence : granite-278m768 sur7878, BGE-M3 dense+creux1024/OCR sur7879, granite-107m384 sur7880, tous non factices. RAG3WEAVER_SANS_DEMON=1 bloque le lancement de repli. Aucun service existant remplacé ni arrêté, aucun cinquième service de modèles.

Pour les petits modèles explicitement locaux des tests, les quatre variables Burn (générale, EMBEDDER, RERANKER, OCR) sont fixées à `gpu:0`, soit card2/PCI04:00.0 sur ce poste selon le mapping documenté et le sysfs relu ; card0/PCI07:00.0 reste hors sélection. Ce sont des chargements éphémères du binaire de tests, pas un nouveau service résident. Régime confort conservé.

**Portées de preuve séparées :** les deux suites réelles prouvent recherche via BGE existant et les autres modèles de leurs parcours ; elles ne prouvent pas le démarrage d'un nouveau démon possédé par la passe. La propriété, le partage entre suites, l'attente, le refus du périmé et la survie entre lancements sont prouvés par les sept cas à faux services privés. La validation réelle n'est pas présentée comme la reproduction du lancement absent qui avait créé le résidu initial.

Plan soumis avant lancement ; ticket ouvert, aucun tour de push ni autorisation de passe lourde déduits de ce document.
