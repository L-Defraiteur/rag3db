# Témoin structurel préparé : annulation partielle, verrous libérés

Porteur hotfixs, [même ticket ouvert](../tickets/2026-11-12-verifier-la-quarantaine-d-une-annulation-partielle-pour-les-transactions-deja-ouvertes.md).
Autorisation : **code de test et plan uniquement**. Aucune compilation, édition de
liens, exécution ou modification produit réalisée. Aucune ressource historique,
base ou archive moteur touchée. Source non compilée : les compatibilités de lien
restent à vérifier, aucun verdict rouge/vert moteur annoncé.

## Source et périmètre

[Source autonome](../../../test/transaction/quarantine_released_diagnostic.cpp)
(`test/transaction/quarantine_released_diagnostic.cpp` à la racine du dépôt).
Ce programme n’est ajouté à aucune cible CMake ni suite automatique ; il possède
son propre main et n’emploie pas gtest. La tranche couvre seulement cellules 2+4
du plan : deux transactions déjà ouvertes, TA écrit A par COPY, TB écrit B,
faute one-shot dans le rollback de TA, puis COMMIT forcé de TB. Les tables sont
disjointes. Le code vérifie que les deux lignes COPY se trouvent dans le groupe
partagé, que TA tenait des verrous, qu’elle a quitté le gestionnaire après le
refus et que ses verrous sont rendus, tandis que TB reste active.

L’index de fixture reprend l’interface du ThrowingIndex existant. Son callback
vérifie son unique appel, le contexte/identifiant de TA et le premier offset 2.
Le callback n’ayant pas de paramètre transaction, l’identité repose aussi sur
l’exclusivité de A dans ce scénario sur un seul fil ; ce n’est pas une nouvelle
API d’injection globale. Le code appelle réellement `Connection::query(ROLLBACK)`
et donc le gestionnaire : aucun drapeau `halfDoneRollback` ou `rollbackFailed`
n’est affecté par la fixture. Le refus d’injection doit effectivement remonter.

La fabrique sentinelle est installée avant la première requête. Contrairement
au plan général, aucun CHECKPOINT initial n’est demandé : auto-checkpoint et
checkpoint de fermeture sont désactivés. Aucune construction de Checkpointer
ni écriture de checkpoint par le témoin n’est permise, même pendant la préparation.
La création initiale normale des fichiers de la base reste celle du constructeur
Database ; ce n’est pas un mode sans toute écriture disque.

Les contrôles publics BEGIN/CHECKPOINT depuis une troisième connexion sans
transaction doivent être refusés avec le nom exact de quarantaine, sans entrer
dans la fabrique. Ils précèdent le COMMIT de TB, car la sentinelle peut laisser
un état de checkpoint en attente ; aucun appel ensuite ne doit s’y bloquer.

TB reçoit `setForceCheckpoint()` existant ; ce choix teste le branchement
interne, pas la couverture d’une DDL ou d’un COPY forcé réel. Son COMMIT doit
produire l’une de ces classifications :

- `internal-path-reached` : un seul passage dans la fabrique, exception sentinelle
  remontée avant toute construction de Checkpointer ;
- `named-quarantine-before-factory` : refus nommé, zéro passage.

Toute autre issue (prémisse manquante, timeout, faux index invisible, plusieurs
callbacks, erreur non attendue) est inconclusive et sort en échec. Le code 0
signifie **observation structurelle réussie**, pas absence de défaut ni politique
validée. Aucun compte de données après faute, réouverture ou oracle de corruption
n’est inclus. Lecteurs, automatique, verrous gardés et attente concurrente restent
hors de cette tranche.

## Compilation et lien proposés, non exécutés

[Recette argv et empreintes](outils/02-lien-temoin-structurel.json),
`ready_to_compile=false`, `ready_to_run=false`. Source SHA256 :
`8a2be2d79544a45b4ad200c51a1dec30294700ba486ec347e42de3dc1ad3456c`.

Le programme est compilable en principe comme une seule TU, hors gtest, contre
les en-têtes de l’archive privée assortie de 0511, jamais ceux de master mélangés
aux objets anciens. Les argv dérivent de la recette du cœur
`docs/coeur-cpp/12-11-2026/outils/cx-commandes-lien-revue.json`, dont l’empreinte
est gardée dans le JSON ; son autorisation précédente n’est pas héritée.

Paquet lu dans la documentation attestée, pas rehashé à nouveau ici :
`~/.cache/rag3db-moteur-notes/annulation/cx-archive-privee-12nov-02/archive-assortie-0511dece7/`.
Archive SHA256
`3030fe674e33e2ea0b5b01ed7bdf1a919e538c02e5dfc0b68fff7b50e1d06758`,
manifest SHA256
`0049111df769914dbeea66fd0d2d07c6add58a670eb80e797589604d48c80df7`.
Les 17 archives tierces et leurs SHA sont repris dans le JSON, ainsi que les deux
en-têtes générés. Ni bibliothèque distribuée ni archive de base non assortie
ne sert de repli.

Staging futur dans un espace neuf propre au témoin
`~/.cache/rag3db-hotfixs-notes/cx-quarantaine-0511-structurel`, jamais dans
l’ancien pilote ou dans un build moteur. Y copier le seul source de test,
les deux générés en conservant leur espace d’inclusion, et le seul header de
fixture `test/transaction/flaky_checkpointer.h` sous `test-headers/transaction/`.
Ce header est identique entre le worktree neuf et 0511 (comparaison des octets
faite en lecture seule). Tous les SHA doivent être relus avant un futur build.
Aucun staging distant créé dans cette tranche.

Compiler uniquement cette TU avec les flags de l’archive et l’include du header
de fixture ; lier son objet avec l’archive attestée et son interface statique,
comme détaillé dans le JSON. Aucun nouveau moteur, aucune TU moteur, gtest,
objet de tests ancien, export-dynamic, lib partagée courante ou installation.
Pour une future compilation : proposer un créneau `poste lourd`, au plus un
compilateur, 120 secondes de compilation et 60 secondes de lien, groupe de
processus supervisé intégralement ; toute différence de source/flags/archives
ou tout symbole manquant revient en revue, sans recompilation moteur improvisée.

## Bornes et artefacts du futur lancement

Le source contient les limites suivantes, pas encore vérifiées par un lancement :

- un seul enfant, une seule cellule, aucun thread de fixture ou sommeil ;
- pool moteur 64 Mio, deux threads maximum configurés, requêtes bornées à 1000 ms ;
- SIGALRM rétabli par défaut et débloqué, alarme enfant 12 secondes avant Database ;
- core dumps interdits ; toute fin par signal est inconclusive ;
- aucun destructeur Database/Connection : allocations volontairement non détruites,
  sortie enfant par `_exit` sur succès et exceptions, donc pas de rollback ou
  checkpoint de fermeture implicite.

Le parent attend uniquement son enfant et conserve son statut brut. L’alarme
borne l’enfant tant qu’il peut recevoir le signal ; elle ne garantit pas une
réponse instantanée d’un noyau bloqué en E/S non interruptible. Le pool de 64 Mio
n’est pas un plafond RSS global. Un futur lanceur distinct devra ajouter un
plafond mémoire de processus/cgroup de 1 Gio et une borne extérieure 20 secondes
pour tout le groupe, sans lancer un second essai si le premier refuse. Il n’est
ni écrit ni exécuté ici. L’argument du programme est un répertoire privé absolu
existant, sans apostrophe ni antislash (littéral Cypher du CSV). `mkdtemp` crée un
sous-dossier neuf et exclusif ; aucun dossier existant n’est nettoyé ou réutilisé.

Artefacts conservés sans suppression : base et fichiers annexes, CSV, `steps.tsv`
avec phases/erreurs/identités/entrée fabrique, `parent.tsv` avec PID/statut brut et
classification. Le futur lanceur devra ajouter source/binaire/lib SHA, argv,
flags et configuration, statut du timeout et limites réellement appliquées.
Le parent n’ouvre jamais la base, ne la répare pas et ne la supprime pas.

## Points de revue avant build puis avant run

Vérifier le placement de la faute dans le vrai handler de 0511, visibilité de
l’index sans stockage, garanties one-shot et sortie sans destructeur ; valider
les argv et l’interface statique pour ce source qui appelle plus d’API que le
pilote de retrait. Une compilation réussie, si autorisée plus tard, ne donne
aucun accord de lancement. Ticket ouvert ; aucune conclusion de corruption,
d’isolation ou de récupération à tirer de ce seul témoin.

## Correction de qualification après compile-01

La tentative initiale a échoué sur `common::` ambigu et l’alias `visible_func`
non qualifié. Source corrigé exclusivement en `rag3db::common::` et
`storage::visible_func` ; aucune logique de fixture changée. SHA courant :
`d2db4d049f92dc5d197eb34e4fea758259bf4af7419537e1042b8a3c88a38530`. Les argv de la seconde
tentative ne changent que les chemins du staging neuf suffixé `-02`, le SHA
source et l’explication de statut. Première tentative conservée intégralement.
La reprise compilation/lien pour cette correction précise est autorisée sous
les mêmes bornes, après relecture du diff ; aucun run du témoin autorisé.

[Contrôleur de compilation](outils/03-compiler-temoin.py) conservé avec ses
contrôles de SHA et limites ; [lanceur runtime](outils/04-lancer-temoin.py) préparé
mais jamais invoqué. Celui-ci exige un accord JSON distinct lié aux SHA du
source, du binaire, de son rapport de build et du lanceur ; le modèle d’accord
ci-joint garde `approved_to_run=false`. Aucun binaire ne peut être lancé en
utilisant ce modèle inchangé. Cgroup runtime20s/1Gio/swap0, groupe complet,
timeout interne15s et sortie limitée1Mio ; ni checkpoint réel ni réouverture
ajoutés au témoin. Entrée runtime prévue : python3 <staging>/runtime.py
<staging> <accord-absolu.json>, puis argv systemd déclarés par le lanceur.
