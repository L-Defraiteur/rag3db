# Vérifier la quarantaine après annulation partielle pour les transactions déjà ouvertes

- **État** : fermé (19 novembre 2026, branches `quarantine-holds-open-transactions-2` puis
  `quarantine-last-reader-checkpoint`) ; ouvert le 12 novembre 2026
- **Famille** : moteur
- **Gravité** : risque de persistance ou de lecture d’un état partiellement annulé, à établir ; aucune corruption observée par cette revue
- **Atteignable en service** : à confirmer — une annulation doit lever après le début des effets partagés, tandis qu’une autre transaction reste ouverte
- **Touche rag3weaver** : potentiellement, par les connexions concurrentes au moteur
- **Pour** : cœur c++/hotfixs/tickets, en coordination avec cœur C++
- **Signalé par** : session hotfixs, revue documentaire du [plan de regroupement](../../coeur-cpp/12-11-2026/10-proposition-regroupement-undo.md), à la demande de l’orchestration
- **Ouvert le** : 12 novembre 2026

## Périmètre et lien

Ce diagnostic est distinct de l’optimisation du retrait des clés suivie dans
[le ticket du pool plein](../../coeur-cpp/tickets/2026-11-09-sous-vingt-ecrivains-un-pool-plein-laisse-une-annulation-inachevee.md).
Il examine si la quarantaine après annulation partielle protège également les
transactions déjà ouvertes et tous les chemins de point de reprise.
L’autorisation actuelle porte uniquement sur la revue et sa documentation :
aucun correctif, build, test, mesure ou accès à une base.

## Preuves de lecture et limite du constat

Sources lues dans `/home/lucied/git_workspaces/cx-coeur-annulation`, à
`0511dece74f94db6937fb03e6ec45f0c58828c5a`. Les positions ci-dessous se rapportent
à ce commit, pas nécessairement à master courant.

- `src/storage/undo_buffer.cpp:87-105,219-227` : liste inverse construite par
  tampon ; `started` posé avant le callback d’annulation.
- `src/transaction/transaction.cpp:225-239` : une exception de rollback copie
  ce drapeau dans `halfDoneRollback`, puis remonte l’exception.
- `src/transaction/transaction_manager.cpp:219-239,245-280` : les chemins
  `failAfterStatement` et `rollback` posent `rollbackFailed` après une annulation
  partielle et demandent la réouverture. Le premier garde la transaction échouée
  et ses verrous ; le second retire la transaction et libère ses verrous.
- Dans ce même fichier, `beginTransaction:53-63` et `checkpoint:293-300`
  appellent `throwIfCheckpointFailedNoLock:303-318`, qui refuse `rollbackFailed`.
- `commit:110-200` n’appelle pas cette garde à l’entrée. Ses chemins peuvent
  rejoindre directement `checkpointNoLock`, ou passer par
  `writeRequestedCheckpointIfLastNoLock:518-527`.
- `checkpointNoLock:598` et suite n’appelle pas cette garde à son entrée.
  Les appels de garde repérés dans les chemins de file de checkpoint
  (`commitInCheckpointQueueNoLock:445,470`) ne constituent pas une garde
  générale de tous les chemins précédents.
- `src/main/client_context.cpp:456-464` et
  `src/transaction/transaction_context.cpp:52-65` valident une transaction
  manuelle existante sans consulter `rollbackFailed`. La condamnation locale
  `manualTransactionFailed` concerne la connexion échouée, pas toutes les autres.
- `src/storage/table/chunked_node_group.cpp:467-476` consulte les versions pour
  la visibilité d’une ligne, sans consulter ce drapeau global.

Ce sont des arcs de contrôle observés dans le code. Ils ne prouvent ni qu’une
transaction concurrente atteint effectivement un checkpoint après cet échec,
ni qu’une lecture est fausse, ni qu’une donnée incorrecte devient persistante.
Aucune recette Cypher exécutée, aucun témoin natif, aucune corruption reproduite.

## Préconditions et garanties à établir

1. Identifier une exception réellement atteignable après effets partagés et
   vérifier le moment où `halfDoneRollback`, puis `rollbackFailed`, deviennent
   visibles sous `mtxForSerializingPublicFunctionCalls`.
2. Distinguer les deux chemins d’échec : verrous conservés par le bloc manuel
   échoué ou relâchés par la fermeture. Vérifier quels lecteurs et écrivains
   déjà actifs peuvent continuer sur les tables concernées ou disjointes.
3. Vérifier les conditions `shouldForceCheckpoint`, demande automatique,
   transactions actives et attentes de checkpoint : un arc sans garde explicite
   peut rester inaccessible sous ces préconditions. Démontrer cette exclusion
   si elle constitue la protection.
4. Suivre le journal des transactions concurrentes, leur validation, les
   versions et les lecteurs qui tiennent déjà des blocs. Vérifier aussi la
   fenêtre avant la pose du drapeau ; une garde ultérieure ne couvre pas à elle
   seule l’observation concurrente des effets déjà appliqués.
5. Examiner les chemins directs et différés vers le checkpointer, y compris
   sortie du dernier lecteur, validation forcée et file de validation, ainsi
   que le cas base en mémoire et les appels internes.
6. Établir les garanties de réouverture/rejeu avant d’accepter des préfixes
   d’annulation différents. Le même préfixe n’est pas un axiome ; isolation,
   sûreté mémoire, absence de persistance invalide et récupération le sont.

## Travail attendu et critère de fin

Produire une conclusion sourcée sur l’accessibilité de chacun de ces chemins,
avec ses préconditions et verrous. Si le risque est exclu, documenter pourquoi
et fermer avec cette preuve. S’il est confirmé, compléter ce même ticket avec
la recette et le contrat à préserver, puis obtenir l’autorisation applicable
avant toute implémentation ou exécution ; la création du ticket ne la donne pas.
Une éventuelle correction devra montrer la protection des transactions déjà
ouvertes, des lecteurs et des checkpoints internes ainsi que la récupération,
avec des validations autorisées et réellement jouées avant fermeture.

Transmettre ou attendre une décision ne ferme pas ce ticket.

## Lecture complémentaire des préconditions, 12 novembre

Toujours au commit `0511dece74f94db6937fb03e6ec45f0c58828c5a`, sans exécution.
Le contrôle de flux permet de distinguer les cas suivants :

- **Transaction échouée encore ouverte** : `failAfterStatement` conserve son
  entrée active et ses verrous (`transaction_manager.cpp:219-242`).
  `stopNewTransactionsAndWaitForOthersNoLock:362-425` attend le départ des
  autres écrivains ; un checkpoint peut donc être bloqué, puis expirer.
  Cette exclusion conditionnelle ne constitue pas une garde de quarantaine.
- **Transaction échouée fermée** : `rollback:245-280` la retire même après
  échec ; `clearTransactionNoLock:577-590` libère tous ses verrous et réveille
  les attentes. Le prédicat d’attente ne demande pas `rollbackFailed` :
  le départ peut donc satisfaire l’attente d’un checkpoint déjà demandé,
  sans réévaluation de ce drapeau dans `checkpointNoLock:598-675`.
- **Écrivain déjà actif sur une table disjointe** : l’entrée de `commit` ne
  consulte pas le drapeau ; une validation ordinaire sans action préalable
  n’attend pas les autres transactions (`commit:121-160`). Le checkpoint
  automatique dépend ensuite du seuil et des écrivains restants. Il reste
  à établir une opération concrète atteignant ces conditions, pas à supposer
  que toute validation force un checkpoint.
- **Validation forcée déjà engagée** : elle attend tous les autres départs
  (`commit:138-141`, `readersToo=true`) ; si la transaction échouée est
  effectivement fermée et les autres parties, le prédicat peut devenir vrai.
  `shouldForceCheckpoint` exclut la base en mémoire
  (`src/transaction/transaction.cpp:105-107`). Les préconditions d’une écriture
  forcée réellement concurrente, les verrous de tables et les éventuelles
  actions antérieures doivent encore être établis.
- **Lecteurs existants** : le prédicat de checkpoint ordinaire permet les
  lecteurs (`readersToo=false`), contrairement au forcé. Ils ne sont pas
  invalidés globalement par le drapeau. Cela ne prouve aucune lecture erronée :
  les versions peuvent préserver leur visibilité ; cette garantie doit être
  suivie pour les effets précis restés partiels.

Le mutex public sérialise les entrées de gestionnaire et la publication du
refus ; il n’annule pas les transactions déjà ouvertes. Les attentes de
`stopNewTransactionsAndWaitForOthersNoLock` rendent ce mutex pendant leur
attente et permettent justement aux autres transactions de partir. Il faut
ainsi vérifier la quarantaine après les attentes, pas seulement avant un BEGIN.

**Conclusion intermédiaire** : les contrôles explicites de BEGIN et du
CHECKPOINT public empêchent ces nouvelles entrées après quarantaine. Les
transactions actives et leurs attentes n’ont pas cette exclusion générale
visible dans le code examiné. Les branches internes sont conditionnellement
accessibles au niveau des prédicats lus, mais une trace d’échec réellement
atteignable avec les verrous requis reste à établir. Ni témoin exécuté, ni
corruption, ni récupération validée. Le ticket reste ouvert.

## Runtime structurel 2 : chemin interne atteint

Après correction exclusive maxDBSize publiée8cc47439f, nouvelle compilation/lien
et ELF conformes, puis un seul nouveau lancement autorisé sous poste lourd.
[Résultat sourcé et empreintes](../12-11-2026/04-resultat-structurel-quarantaine.md) :
TA annule partiellement par le vrai gestionnaire, callback one-shot confirmé,
TA inactive et verrous0 ; BEGIN/CHECKPOINT publics refusent nommément, mais
COMMIT forcé de TB déjà ouverte atteint une fois la fabrique interne.
La sentinelle lève avant toute construction de Checkpointer. Classification
`internal-path-reached`, enfant/parent/wrapper0, groupe terminé et cgroup disparu.
Aucun checkpoint réel, réouverture, lecteur testé ni corruption démontrée.
L’arc précis est confirmé pour la fixture contrôlée ; conséquences et protection
des autres chemins restent ouvertes. Artefacts/base privée conservés sur luciepc.
Aucun nouveau runtime après cette tentative. Aucun correctif produit autorisé
ou appliqué. Ticket maintenu ouvert pour la suite à décider avec cœur C++.

## Relecture cœur et suite sur papier — 12 novembre

[Relecture du résultat et contrat proposé](../../coeur-cpp/12-11-2026/21-relecture-quarantaine-et-suite-sur-papier.md)
(lecture de e289163bf et moteur0511, sans nouveau runtime/build/code produit).
Conclusion et limites confirmées : internal-path-reached sous fixtureIndex et TB
forcée, aucun checkpoint réel/réouverture/corruption démontré. Code0 du témoin
n'est pas l'oracle d'une quarantaine correcte : lire la classification effective.

Point à ajouter au contrat : le chemin forcé traverse Transaction::commit et le
retrait de TB AVANT la fabrique sentinelle ; le refus d'un checkpoint seul ne prouve
pas l'absence de publication de TB. Contrôles proposés avant publication et avant
checkpoint, recontrôle après attentes, prédicats/réveils de quarantaine et départs
sans checkpoint à définir ensemble. Matrice future structurelle avec contrôle sain,
commit ordinaire/forcé, différé/lecteur et attente synchronisée ; aucune cellule
exécutée ou autorisée par cette note. Durabilité et index persistant de fixture
restent une tranche distincte. Ticket ouvert, porteur hotfixs/cœur ; pas master,
pas patchlib, pas de reprise d'action arrêtée/rejetée.

## Avancement du 19 novembre 2026 (hotfixs, au mot de c7)

**Le défaut est confirmé.** Le témoin `QuarantineOpenTransactions` (`test/transaction/quarantine_open_transactions_test.cpp`)
reprend le diagnostic privé de Codex en témoin ordinaire. Chaque cas joue dans un fils qui ne détruit jamais sa base.
Un faux index fait lever l'annulation de TA au milieu du stockage partagé, et une fabrique de point de reprise
sentinelle compte ses appels. Sur master 9fd3df3765, une fois la faute et le refus du BEGIN de contrôle vérifiés :
- le COMMIT forcé de TB, ouverte avant, **atteint le point de reprise** (« witness: checkpointer reached ») ;
- le COMMIT ordinaire de TB **réussit**.

**Le correctif** : `TransactionManager::commit` refuse, à son entrée, toute validation d'écriture quand une
annulation n'a pas pu finir (`throwIfRollbackFailedNoLock`, le refus du BEGIN et du CHECKPOINT). Le refus vient avant
toute publication et tout point de reprise ; l'appelant annule TB. Le refus après un point de reprise échoué
(`checkpointFailed`) n'est pas étendu à la validation : `commit_after_failed_checkpoint_test` attend qu'elle passe.
Vert sur 852266a8b0 avec le correctif, les deux cas.

**Ce qui reste ouvert** : la validation d'une lecture seule passe par `writeRequestedCheckpointIfLastNoLock`, et un
dernier lecteur qui part peut lancer le point de reprise automatique demandé (`checkpointNoLock`), sans garde en
quarantaine. Le témoin demande une demande automatique pendante au moment de la faute ; il est à écrire. La
récupération après réouverture (le journal rejoué) reste celle d'aujourd'hui, que ce lot ne change pas.

## La fermeture (19 novembre 2026, au soir)

**Le dernier lecteur aussi.** Une écriture validée pendant que TA et un lecteur R sont ouverts dépasse le seuil et
pose la demande de point de reprise automatique, que le dernier à partir écrit. L'annulation de TA échoue à
mi-chemin ; R, en partant, lançait quand même ce point de reprise (`writeRequestedCheckpointIfLastNoLock`). Le témoin
`TheLastReaderToLeaveDoesNotWriteTheRequestedCheckpoint` est rouge sur 38eacb2664 (« witness: checkpointer reached »,
la demande encore posée au départ de R), vert avec le correctif : en quarantaine, le dernier à partir renonce à la
demande, et les transactions retenues sont relâchées pour être refusées par leur nom. Sa contre-épreuve, sans la faute,
prouve que la recette exerce le chemin : le dernier lecteur y écrit bien le point de reprise.

Un piège de témoin, noté pour d'autres : un BEGIN fait pendant qu'un point de reprise automatique est en attente
attend au plus `AUTO_CHECKPOINT_WAIT`, puis **abandonne la demande**. Un contrôle de mise en place placé là retirait
l'objet du cas, et le vert ne prouvait rien.

**Ce qui est couvert** : la validation d'une écriture ouverte avant (forcée ou non), le BEGIN et le CHECKPOINT publics
(déjà), le point de reprise automatique du dernier à partir. La récupération après réouverture (le journal rejoué)
est celle d'aujourd'hui, que ces lots ne changent pas.
