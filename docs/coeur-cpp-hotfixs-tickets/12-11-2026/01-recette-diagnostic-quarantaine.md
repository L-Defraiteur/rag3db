# Recette proposée : quarantaine et transactions déjà ouvertes

Porteur hotfixs, en coordination avec cœur C++ ;
[ticket ouvert](../tickets/2026-11-12-verifier-la-quarantaine-d-une-annulation-partielle-pour-les-transactions-deja-ouvertes.md).
**Document à relire seulement. Aucun code, build, test, mesure ou accès base exécuté ou autorisé ici.**
Sources de référence : `0511dece74f94db6937fb03e6ec45f0c58828c5a`.

## Injection disponible sans changement produit

`test/transaction/half_applied_commit_test.cpp` pose un `ThrowingIndex` d’essai
sur une table via `NodeTable::addIndex`, sans ajouter d’API moteur. L’interface
`Index::rollbackInsert` est déjà virtuelle. Un nouvel index strictement de fixture
pourrait y compter l’appel et lever une exception identifiée une seule fois.
Ce serait du code de test à écrire après accord, pas un crochet produit à ajouter.

Le placement est significatif : `NodeTableVersionRecordHandler::rollbackInsert`
(`src/storage/table/node_table.cpp:97-115`) a déjà retiré les clés locales par
`rollbackPKIndexInsert` quand il appelle les index secondaires, avant versions,
troncature et compte. Le callback atteint garantit un début d’undo partagé ;
il ne faut ni positionner artificiellement `rollbackFailed`, ni appeler le handler
isolément en contournant le gestionnaire. Un compteur de clés locales avant/après,
si une lecture de fixture existante le permet, compléterait la preuve ; à défaut,
le callback et l’ordre sourcé prouvent le passage, pas le nombre de clés retirées.

`test/transaction/flaky_checkpointer.h` fournit déjà l’accès de fixture à
`initCheckpointerFunc`. La fabrique peut enregistrer son entrée et lever une
exception sentinelle avant de construire un checkpointer : on observe le
contournement de garde sans écrire de pages. Le compteur compte l’entrée de
fabrique, pas un checkpoint achevé. `checkpoint_test.cpp` montre aussi une
sous-classe déléguant au checkpointer réel pour une phase durable distincte.

Ces mécanismes suffisent à proposer l’injection de faute et la détection du
chemin interne, sans changement produit identifié. Une difficulté nouvelle
exigeant un tel changement serait remontée avant conception de ce changement.

## Base minimale et état de référence

Base jetable sur disque, aucune base utilisateur, sans GPU ni extension vectorielle.
Réutiliser `EmptyDBTest`, les CSV du test `rollback_under_readers_test.cpp` et
son principe de COPY direct dans les groupes partagés. Une simple insertion CREATE
reste locale jusqu’au commit et ne suffit pas à prouver ce chemin de rollback.

Deux tables `A(id INT64 PRIMARY KEY, v INT64)` et
`B(id INT64 PRIMARY KEY, v INT64)`, chacune avec les clés validées 0 et 1,
puis un CHECKPOINT initial avant la pose des instruments. Configuration multiauteur
explicite ; checkpoint automatique désactivé pour le scénario forcé, activé avec
seuil maîtrisé seulement pour sa variante automatique. Pool de fixture habituel,
pas de saturation mémoire : la faute est injectée au point exact.

CSV de deux lignes `100,100` et `101,101` pour A. Journal de COPY explicitement
retenu (seuil très supérieur au CSV, réglages de `journaled_copy_test.cpp`), pour
que la transaction abandonnée ne dépende pas d’un checkpoint forcé. Vérifier les
paramètres effectivement retenus ; un refus de configuration ou un repli inattendu
est une prémisse manquante, jamais un vert.

Après le checkpoint initial, ouvrir deux transactions MANUAL avant toute faute :
TA écrit A par COPY ; TB écrit uniquement B (clé 200, valeur 200). Tables disjointes
pour que les verrous de TA n’empêchent pas la préparation de TB. Poser l’index
d’essai sur A avant ces transactions et confirmer qu’il est loaded/visible/non-détaché.
Armer la faute uniquement pour TA, avec compteur et identité de transaction.
Chaque cellule ci-dessous repart d’une base neuve ; ne pas cumuler leurs mutations.

## Cellules déterministes, sur un seul fil autant que possible

1. **Verrous gardés.** TA a réussi son COPY. Faire échouer une instruction dans
   son bloc manuel avec la recette de liaison/analyse déjà employée par
   `statement_failure_test.cpp`, ou un appel de fixture public au chemin exact
   `failAfterStatement` si le passage par la requête n’est pas garanti. Vérifier
   impérativement le callback d’index, l’exception sentinelle, la transaction
   échouée toujours active et `hasHalfDoneRollback`. Une simple erreur d’analyse
   sans callback ne remplit pas la prémisse. Ne pas envoyer ROLLBACK ensuite avant
   l’observation de ce cas.
2. **Verrous libérés.** Sur une autre base, TA reçoit directement ROLLBACK avec
   l’index armé ; confirmer son refus sentinelle, sa sortie du gestionnaire actif,
   et le fait qu’aucun second rollback destructeur ne rejoue le callback. TB était
   déjà ouverte. Distinguer ainsi `TransactionManager::rollback` de la cellule 1.
3. **Lecture existante.** Cellule séparée avec deux transactions, TA écrivaine et
   TB READ ONLY ouverte avant la faute. TB doit lire les couples validés de A,
   sans les clés 100/101. Observer avant et après le ROLLBACK de fermeture de TA
   dans le cas gardé. Ne pas exiger que TB voie une écriture validée après son
   instantané. Un refus ne vaut protection que s’il nomme la quarantaine attendue,
   pas une erreur de table, de syntaxe ou un timeout générique.
4. **Commit interne forcé, verrous libérés.** TA est sortie comme cellule 2 ;
   TB contient sa seule insertion B. Utiliser le `Transaction::setForceCheckpoint`
   public déjà présent, appliqué par la fixture à TB : cela isole le branchement
   `commit` forcé sans inventer une DDL. La couverture d’un vrai COPY forcé/DDL
   reste une extension distincte, pas une propriété prouvée par ce drapeau.
   Armer alors la fabrique sentinelle et appeler COMMIT de TB. Relever si le refus
   de quarantaine arrive avant toute entrée de fabrique ; si la fabrique est atteinte,
   le chemin interne est observé, sans preuve de corruption.
5. **Commit forcé, verrous gardés.** Même préparation mais TA reste active comme
   cellule 1. Un seul fil suffit : COMMIT de TB doit refuser avant fabrique ou
   atteindre le timeout d’attente configuré, puisque TA ne part pas pendant l’appel.
   Ce timeout caractérise une barrière d’activité, pas une quarantaine correcte.
   Puis fermer TA et arrêter la cellule ; ne pas interpréter le COMMIT échoué comme
   une validation à réessayer aveuglément.
6. **Automatique, libérés.** Autre base avec seuil automatique choisi pour la petite
   transaction B, TA déjà retirée, TB dernière écrivaine. Confirmer le prédicat
   `canAutoCheckpoint` vrai avant l’appel par les tailles/configurations de fixture,
   sans grossissement aléatoire du journal. Fabrique sentinelle armée après TA,
   COMMIT TB ; distinguer validation journalisée de l’échec de son checkpoint.

Les contrôles BEGIN et CHECKPOINT publics depuis une connexion sans transaction
servent de témoins positifs du refus global déjà attendu, après fermeture de TA.
Ils ne remplacent aucune observation sur TB déjà ouverte.

## Synchronisation et bornage proposés

La séquence minimale est pilotée sur un fil : retours des requêtes, compteurs
one-shot et états de transaction servent de barrières. Aucun sommeil, remplissage
aléatoire du pool ni course de charge. Pour une future variante « TB attend puis TA
se ferme », une condition de fixture doit confirmer l’entrée réelle en attente
avant fermeture ; l’existence d’un tel signal n’est pas établie ici. Ne pas remplacer
cette preuve par un délai ou le seul lancement d’un thread ; variante différée.

Proposition à approuver avec le futur run : un enfant isolé par cellule, 2 threads
moteur, pool 64 Mio de fixture, aucune charge lourde ; attente de checkpoint/lock
configurée à 1 seconde, borne externe 15 secondes par cellule et 120 secondes pour
le lot entier. L’horloge ne tranche pas le verdict fonctionnel ; elle arrête un
blocage. Le parent supervise ses propres enfants, conserve leur statut et leurs
journaux, sans tuer d’autre processus. Tout filtre sans test joué ou cellule
sautée est incomplet. Aucun chiffre de performance à tirer de ces bornes.

## Oracle et réouverture

Deux issues acceptables pour le contrat, à distinguer explicitement :

- refus nommé de quarantaine avant de servir un état non garanti et avant le
  checkpoint interne ; récupération de la base et des seuls commits effectivement
  durables après réouverture ;
- opérations permises avec isolation et récupération démontrées pour chaque
  préfixe partiel autorisé. Un COMMIT réussi seul ne prouve rien de ce contrat.

La fabrique sentinelle est un **oracle structurel de chemin**, pas un oracle
suffisant de corruption ni un substitut à la phase durable. Son atteinte doit
produire un verdict « chemin observé, protection à établir », pas annoncer une
perte de données. Inversement, un timeout avec TA active ne valide pas la garde.

Dans toutes les cellules, quitter l’enfant sans checkpoint de fermeture implicite
après collecte des résultats et sortie des requêtes ; le parent rouvre avec une
nouvelle instance et sans instruments. Pour la variante purement journalisée,
A doit retrouver exactement ses couples initiaux, sans 100/101 ; B contient 200
uniquement si la validation a effectivement été journalisée et rendue durable.
Classer ce statut par les marqueurs de WAL/retours du moteur, pas par le seul
refus final de COMMIT. Un statut durable impossible à établir est inconclusif.
Après lecture exacte (couples triés, pas seulement count), vérifier réutilisation
des clés de TA dans une nouvelle transaction, puis une seconde réouverture.

**Limite à lever avant toute phase de checkpoint réel** : le ThrowingIndex du
test existant est sans stockage et doit être retiré avant checkpoint de fermeture.
Ne pas déléguer un checkpoint réel tant que sa représentation/persistance n’est
pas prouvée sûre. Un retrait contrôlé de cet index strictement de fixture après
la faute pourrait être envisagé pour la phase durable ; il ne doit toucher ni
les lignes ni les drapeaux de quarantaine, et son absence d’effet sur le chemin
étudié doit être prouvée. Cette phase n’est donc pas encore une recette exécutable
approuvée. La sous-classe de checkpointer existante fournit l’observation, mais
ne règle pas à elle seule la durée de vie de cet index.

## Artefacts et fin de ce sous-travail

Réutilisables après accord : schéma et CSV figés, un nom par cellule, état attendu
trié, compteurs injection/fabrique, identité des transactions, codes/erreurs exacts,
configuration retenue, marqueurs de durabilité, journaux enfant/parent et hashes
source/lib. Conserver les bases jetables d’un refus pour examen ; aucune base
historique ni ressource de l’autre ticket à modifier.

La revue doit trancher le passage garanti de l’erreur d’instruction vers l’undo,
le statut durable de TB et la durée de vie de l’index de fixture avant implémentation.
Le témoin minimal structurel utilise seulement les mécanismes déjà disponibles ;
les variantes durable/attente active restent conditionnées aux points ci-dessus.
Ticket ouvert ; ce document n’accord aucun run, changement moteur ou activation.
