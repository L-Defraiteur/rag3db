# Vérifier la quarantaine après annulation partielle pour les transactions déjà ouvertes

- **État** : ouvert — diagnostic de contrôle de flux, aucun défaut de données reproduit
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
