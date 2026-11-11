# Applications déclaratives : composer des capacités et examiner leurs effets

11 novembre 2026. **Avis d’architecture à conserver pour la suite**, à la demande de Lucie.
Ce document propose une direction ; il ne décrit pas une capacité livrée ni un chantier à ouvrir
maintenant. Les priorités restent les vues par worktree en couches et le parcours complet du chat.

## L’intuition

Quand une personne utilisera rag3weaver pour faire tourner son application en ligne, ses propres
utilisateurs pourront vouloir y connecter leurs mails, leurs documents ou d’autres services.
La question dépasse alors le connecteur technique : quels pouvoirs cette application exerce-t-elle,
pour qui, sur quelles données et vers quelles destinations ?

**Mon avis : le déclaratif peut rendre cette difficulté plus traitable si les déclarations portent
la sémantique des effets, et pas seulement le câblage des opérations.**

« Lire des mails puis appeler un modèle » décrit un parcours. Cela ne dit pas encore si leur contenu
reste dans l’installation, s’il part chez un fournisseur, ce qui en est conservé, ni si l’application
peut envoyer une réponse au nom de la personne.

Le JSON ne devient pas innocent parce qu’il porte une cravate Mermaid.

## Ce que le projet a déjà posé

La [vision des secrets et intégrations](../../../extension/rag3weaver/visions/2026-10-04-16h24-montrer-nos-produits.md)
(§9) décrit notamment : références de secrets visibles par l’agent, valeurs détenues par l’exécuteur,
destinations contraintes et authentification fournie par des mécanismes du runtime.
Cette page est elle-même une conception, pas la preuve que tous ces mécanismes sont implémentés.

La [vision des domaines et de l’exposition](../../../extension/rag3weaver/visions/2026-10-09-13h36-plusieurs-produits-un-seul-atelier.md)
distingue aussi ce qui est déclaré de ce qui doit être disponible dans un contexte donné.

La proposition ici prolonge ces idées aux **données transformées et aux effets des compositions**.
Elle ne demande pas une nouvelle architecture de connecteur pour chaque fournisseur.

## Trois objets à ne pas confondre

| Objet | Ce qu’il décrit |
|---|---|
| L’application déclarée | Entités, graphes, outils, paramètres et effets possibles. |
| Son déploiement | Services réellement branchés, destinations, modèles, stockages et versions utilisés. |
| L’autorisation d’exécuter | Personne ou organisation concernée, ressources accessibles, opérations permises et limites. |

Le même graphe pourrait tourner chez son auteur, dans une entreprise ou dans un hébergement
mutualisé. Sa définition ne doit pas lui donner implicitement les pouvoirs d’une installation
particulière. La référence d’un secret ne doit pas non plus se confondre avec son autorisation d’usage.

Rendre un outil visible, permettre son appel et permettre à des données d’atteindre une destination
sont des questions liées, mais distinctes. Un mécanisme d’exposition ne les résout pas toutes.

## Des contrats d’effets pour les primitives

Les nœuds disposent déjà d’entrées et de sorties typées. Une évolution possible est de leur associer
une description exploitable de leurs effets : lecture d’une ressource, persistance, appel externe,
modification ou action irréversible. C’est une proposition de contrat, pas un nouveau format arrêté.

Le runtime pourrait alors combiner deux niveaux :

- **Avant l’exécution**, examiner les chemins possibles et les capacités nécessaires à la composition.
- **À l’exécution**, vérifier l’identité, les paramètres et les destinations effectivement résolus,
  puis conserver une trace des opérations réalisées et des refus.

La séparation importe pour les agents : ils choisissent dynamiquement certains appels et peuvent
composer de nouveaux graphes. L’analyse préalable ne suffit donc pas à autoriser tous leurs gestes.

Prenons une application de support. Lire un mail, transmettre son contenu à un modèle distant et
envoyer une réponse sont trois capacités. Autoriser la première ne donne pas automatiquement les
deux autres. Le modèle reçoit les données nécessaires à son travail ; il ne reçoit pas le jeton
qui permet d’accéder à toute la boîte.

## La provenance doit survivre aux transformations

Résumer un mail ne fait pas disparaître son origine. La même question se pose pour ses morceaux,
ses vecteurs, un index, une fiche de mémoire et un résultat d’outil conservé dans un journal.

Le contrat devrait donc définir comment la provenance et les contraintes utiles se propagent dans
les transformations, les sous-graphes, les appels de modèles et les écritures durables. Il faudrait
aussi définir les conditions d’un changement de classification ; le simple nom d’un nœud ne suffit pas.

Sans cette continuité, un graphe pourrait prétendre respecter une règle à sa source et perdre
l’information nécessaire pour l’appliquer deux transformations plus loin. Ce serait du blanchiment
de données par nœud `Summarize`.

## Ce qu’on peut démontrer dépend de ce qu’on maîtrise

**Un graphe ne permet de vérifier que les propriétés dont les primitives exposent une sémantique
suffisamment précise, et que l’environnement d’exécution fait réellement respecter.**

Un script avec accès libre au réseau n’est pas analysable comme un nœud dont les destinations sont
bornées. Un appel externe arbitraire ne devient pas sûr parce que son manifeste le déclare tel.

Selon le mécanisme, il faudrait restreindre les capacités de l’exécuteur, vérifier un contrat,
ou signaler explicitement une zone inconnue. Une analyse partielle doit rendre une conclusion
partielle. L’extension par du code reste possible, mais elle change le périmètre des garanties.

Même distinction pour le déploiement : décrire une destination autorisée et prouver que tous les
chemins réseau passent par la garde sont deux travaux différents. Le runtime, les services et la
configuration effective font partie de ce que la preuve doit couvrir.

## Un dossier technique dérivé de l’application

À partir de déclarations et de contrats effectivement vérifiés, le système pourrait produire :

- l’inventaire des capacités et des accès nécessaires ;
- les chemins de données possibles, les persistances et les destinations ;
- les frontières entre personnes, organisations et contextes d’exposition ;
- les versions de l’application, des primitives et du déploiement examinés ;
- les tests, traces et propriétés établies, avec leurs hypothèses et leurs zones inconnues.

Ce serait une base pour le cookbook envisagé par Lucie : montrer ce qui est déployé et ce qui le
justifie, plutôt que rédiger après coup une documentation détachée du système réel.

Cela ne constitue pas une promesse de certification automatique ou transférable à toute application.
Les exigences propres à chaque fournisseur et à chaque démarche restent à examiner séparément ;
ce document n’en établit aucune. Il propose de rendre les éléments techniques nécessaires plus
observables et plus réutilisables.

Une modification de graphe, de primitive ou de destination pourrait aussi indiquer quelles conclusions
et quels tests doivent être réexaminés. Une preuve attachée à une ancienne configuration ne vaut pas
silencieusement pour sa remplaçante.

## Pourquoi cette direction dépasse les connecteurs

Le même mécanisme servirait aux agents qui font évoluer une application : ils pourraient composer
ou modifier ses déclarations tout en restant soumis aux pouvoirs de leur contexte d’exécution.
L’application, son atelier de construction et son exposition publique pourraient partager des
primitives sans partager toutes les capacités.

**Le cap serait une composition dont on peut examiner les pouvoirs et certaines garanties.**

C’est cohérent avec la boucle étrange du projet : les représentations sur lesquelles l’agent agit
peuvent aussi être celles que le système analyse, exécute et documente. Cela reste une direction
à éprouver, pas une preuve générale sur tout programme ni une autonomie sans limites.

À garder dans la vision. Il n’est pas nécessaire de convoquer les connecteurs mail au milieu du
chantier des worktrees pour que cette idée ait de la valeur.
