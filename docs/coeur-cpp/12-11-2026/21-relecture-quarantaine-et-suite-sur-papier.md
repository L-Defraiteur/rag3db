# Quarantaine : relecture du résultat structurel et suite sur papier

12 novembre 2026. Relecture cœur C++, porteur du suivi hotfixs/cœur ;
[même ticket ouvert](../../coeur-cpp-hotfixs-tickets/tickets/2026-11-12-verifier-la-quarantaine-d-une-annulation-partielle-pour-les-transactions-deja-ouvertes.md).
Lecture de e289163bf, de son témoin et de ses preuves ; moteur privé0511dece7.
**Documents uniquement : aucun code moteur/test modifié, build, runtime, patchlib,
publication master ou reprise d'une action arrêtée/rejetée.** Les autorisations
anciennes du témoin ne valent pas pour les cellules envisagées ci-dessous.

## Conclusion relue : juste et suffisamment bornée

Le [résultat hotfixs](../../coeur-cpp-hotfixs-tickets/12-11-2026/04-resultat-structurel-quarantaine.md)
prouve une entrée dans la fabrique interne depuis le COMMIT forcé de TB déjà
ouverte, après annulation partielle de TA, clôture de TA et libération de ses
verrous. BEGIN et CHECKPOINT publics ont refusé la même base auparavant.
La classification internal-path-reached est donc soutenue par le source et steps.tsv.
Aucun checkpoint réel, réouverture, lecture fausse ou corruption n'est démontré.

Les assertions de fixture précèdent les lignes de confirmation : deux lignes
COPY de TA dans le groupe partagé, callback one-shot à offset2, TA inactive,
locks0, TB encore active, puis publics refusés et fabrique1fois. L'identité du
callback est corrélée au contexte de TA et à l'usage exclusif de A sur un seul
fil ; elle n'est pas une instrumentation générique de l'identité d'appel en
concurrence. C'est suffisant pour les préconditions annoncées, pas pour les
étendre à d'autres scénarios.

La fabrique sentinelle est installée avant toute requête, et l'enfant sort par
_exit sans destructeurs Database/Connection. Elle interdit la construction du
Checkpointer, y compris un chemin imprévu pendant préparation/fermeture.
La source utilise un index injecté et setForceCheckpoint explicite : aucune
prémisse de DDL réelle ou d'épuisement mémoire n'en découle. Les échecs de
compilation et de maxDBSize précédents sont distincts du résultat final.

Le code0 du témoin signifie diagnostic-observed : il accepte aussi l'autre
classification named-quarantine-before-factory. **Le code0 seul n'est donc pas
l'oracle de l'arc atteint ni un test de non-régression de quarantaine.** La ligne
internal-path-reached et checkpoint-factory1 sont les preuves déterminantes ici.

## Nuance importante : la fabrique n'est pas la frontière de publication de TB

Source0511, `src/transaction/transaction_manager.cpp:111–179` : le chemin forcé
attend les autres, avance commitTS, appelle Transaction::commit, retire TB et
libère ses verrous, puis entre dans checkpointNoLock. Celui-ci atteint la
fabrique avant toute construction/écriture (`:598–628`).

`src/transaction/transaction.cpp:175–218` transfère les écritures locales, traite
le journal selon le type de durabilité, puis date les versions d'undo. Le forcé
ne journalise pas un COMMIT ordinaire : sa durabilité dépend de son checkpoint.
Ainsi, le chemin observé a déjà traversé la phase de commit de TB **avant** la
sentinelle. Les traces ne lisent pas les valeurs de TB à ce point ; ne pas en
faire une observation directe de visibilité ou de durabilité de B200.

Conséquence de conception : ajouter seulement une garde avant la fabrique peut
bloquer le checkpoint tout en laissant TB avoir franchi sa publication mémoire.
Ce n'est pas une preuve que « rien de TB n'a été validé ». Le refus du commit
**avant publication** et le refus de checkpoint **avant effet** sont deux
frontières distinctes à couvrir.

TransactionContext::commit (`:66–92`) oublie une transaction déjà retirée par le
manager lorsque le checkpoint suivant lève ; il n'en rejoue pas un rollback.
Une future correction doit préserver cet invariant de durée de vie et distinguer
refus avant commit, commit ordinaire durable suivi d'un checkpoint raté, et commit
forcé dont la durabilité n'a pas abouti. Pas de message générique « rien validé »
si le point de refus est tardif.

## Contrat proposé à arbitrer avant un diff produit

Le mécanisme existant est un état de santé sous le mutex public
(rollbackFailed/checkpointFailed et son refus nommé). Réutiliser cette frontière
plutôt qu'ajouter des exceptions métier par table, un drapeau atomique parallèle
ou un chemin spécial pour la fixture.

| Entrée après annulation partielle reconnue | Politique proposée, non implémentée |
|---|---|
| BEGIN/public checkpoint | Refus nommé déjà présent, conservé. |
| Commit d'écrivain déjà actif, ordinaire/forcé/en file | Refus avant commitTS, préactions, transfert local, WAL et publication des versions. |
| Checkpoint interne, automatique, différé ou fermeture | Refus avant toute entrée effective et après toute attente ayant relâché le mutex, même si l'appelant a oublié une garde. |
| Nouvelle instruction d'une transaction déjà ouverte | Refus au prochain point d'entrée ; aucune affirmation que les opérateurs déjà en vol sont instantanément arrêtés. |
| Fermeture de lecteur/rollback de transaction | Doit permettre de rendre contexte et verrous sans nouveau checkpoint, sans rejouer TA ni manipuler un objet déjà détruit. Contrat exact des erreurs à arrêter. |
| Réouverture dans un processus neuf | Contrat de récupération distinct, pas encore validé par ce témoin. |

L'identité du préfixe d'effets n'est pas un axiome nécessaire. Ce qui est requis :
isolation des écritures déjà validées, invisibilité des écritures non validées,
sûreté des lecteurs/opérateurs déjà en vol, aucune persistance d'un état partiel
par un nouveau checkpoint, et récupération sûre. L'inobservabilité de tous les
préfixes n'est pas acquise par le seul refus des nouveaux BEGIN.

## Carte de couverture à examiner, sans patch dans ce lot

1. Commit sous mutex : contrôle avant tout effet, **puis recontrôle après les
   attentes** de stopNewTransactionsAndWaitForOthersNoLock/commitInCheckpointQueueNoLock,
   avant timestamp et Transaction::commit. Une garde d'entrée seule rate un état
   posé pendant une attente qui relâche le mutex.
2. checkpointNoLock : contrôle commun avant modification du cycle checkpoint,
   et après attente avant T/F, fabrique et writeCheckpoint ; inclure les appels
   directs depuis commit et writeRequestedCheckpointIfLastNoLock.
3. Publication de la quarantaine : réveiller les attentes concernées pour qu'elles
   refusent nommément. Modifier seulement un bool et appeler notify_all ne suffit
   pas si les prédicats attendent encore seulement checkpointPending/commitQueuedNow.
   Lire aussi l'annulation des demandes automatiques et la sortie des files.
4. Départs sans checkpoint : la fermeture d'un lecteur appelle actuellement
   writeRequestedCheckpointIfLastNoLock ; ne pas confondre départ sûr et permission
   d'écrire un checkpoint. Ne pas garder indéfiniment les verrous d'un écrivain dont
   le commit a été refusé ; choisir le protocole de clôture et le prouver.
5. Instructions de transactions existantes et opérateurs en vol : les guards
   ClientContext/TransactionContext actuels vérifient surtout l'échec local.
   Une politique globale doit respecter le mutex qui protège l'état, les versions
   et la durée de vie des blocs ; un bool lu sans synchronisation ne la remplace pas.

Source des attentes : manager362–425,427–480,486–509. failAfterStatement220–242
peut poser rollbackFailed en gardant TA active/verrouillée ; rollback245–280
la retire et notifie via clearTransactionNoLock. Ces deux cas ne doivent ni
partager un oracle « TA partie » faux, ni se résoudre par un sommeil arbitraire.
Le callback one-shot actuel reste une prémisse utile, pas une preuve de ces
interleavings concurrents.

## Plus petite prochaine preuve structurelle à préparer si autorisée

D'abord fixer les invariants précédents, puis relire une matrice sans écriture de
checkpoint. Une seule cellule sur verrous libérés ne couvre pas les autres routes.

| Cellule proposée | Oracle nécessaire |
|---|---|
| Contrôle sain sans faute TA, TB forcée | La fabrique peut être atteinte ; évite de déclarer vert un moteur qui désactive tous les checkpoints. |
| Même faute TA et TB préouverte forcée | Refus nommé **avant publication**, fabrique0, clôture/locks conformes ; la classification internal-path-reached ne serait plus un succès de non-régression. |
| TB ordinaire, auto désactivé | Refus avant publication même sans checkpoint ; pas seulement fabrique0, qui serait trivial. |
| Chemin automatique/différé, dernier départ lecteur | Ni publication post-quarantaine ni entrée interne ; le lecteur doit pouvoir se fermer sans déclencher un checkpoint. |
| Quarantaine posée pendant une attente déjà engagée | Barrière observée avant faute, réveil/refus nommé après faute, pas de timeout utilisé comme oracle ; aucune publication ni fabrique après attente. |
| TA échouée gardée active puis fermée | Pas de faux constat de progression sous verrous ; pas de replay d'undo, pas de checkpoint à la fermeture. |

Les oracles « avant publication » doivent être concrets avant code : point source
attesté plus état de transaction/version ou sentinelle disponible avant cette
frontière, sans supposer qu'un échec tardif annule les effets. Réutiliser les
primitives de fixture existantes si elles permettent cet oracle ; sinon expliquer
le manque avant d'ajouter une interface de test. Ne pas remplacer le refus nommé
par « fabrique pas appelée » seulement : cela pourrait cacher blocage, timeout ou
une autre erreur de prémisse.

Pour les lecteurs déjà en vol : preuve séparée sur lignes validées et lignes TA
invisibles, avec synchronisation des étapes et versions retenues. Cette lecture
n'est ni dans runtime02 ni acquise par la matrice de checkpoint. Aucune assertion
de lecture erronée actuelle.

Bornes futures à soumettre avec source/recette, pas une autorisation : conserver
le principe privé, mémoire/groupe/temps bornés, arrêt one-shot et preuves de fin,
sentinelle avant Checkpointer, aucune donnée utilisateur. Ne pas réutiliser les
accords, binaires ou quatre SHA d'un témoin modifié.

## Durabilité et correction produit : différées explicitement

Un essai de checkpoint réel demanderait un périmètre distinct, une base durable
initiale attestée, une faute atteignable et une fixture dont l'index a un contrat
persistant valide. L'index DIAGNOSTIC de runtime02 ne prouve pas cette capacité.
Le supprimer après la faute pour rendre le checkpoint possible modifierait l'état
étudié : ce n'est pas une preuve de récupération équivalente.

Définir alors les écritures durables attendues de TA et TB, les marqueurs de WAL,
les fichiers concernés, le résultat après redémarrage et les modalités d'arrêt
sans destructor implicite. Rien de ceci n'est exécuté, ni proposé comme reprise
immédiate d'une action refusée. Un garde de chemin prouvé ne suffirait pas, seul,
à fermer le ticket sans le contrat de protection/récupération convenu.

## Livrable et décision attendue

Relecture favorable à la conclusion/limites de e289163bf. Complément demandé :
expliciter l'ordre publication de TB → fabrique et arrêter le contrat de clôture/
réveil. Ensuite choisir la plus petite cellule discriminante avant tout nouveau
source de test ou patch moteur. Ticket ouvert hotfixs/cœur, sources/base/logs
privés conservés ; aucune publication master, aucun nouveau travail natif engagé.
