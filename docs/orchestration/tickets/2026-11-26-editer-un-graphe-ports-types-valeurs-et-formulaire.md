# Éditer un graphe : ports typés, valeurs sur les entrées, nœud valeur, formulaire d'un nœud-graphe

Ouvert le 26 novembre 2026. Lucie, en regardant les écrans « L'archiviste en graphe » et « L'agent de code en graphe »
de la maquette : voir les ports comme des cibles avec leurs types, une vue simplifiée sans ports et une vue d'édition
avec, un « scalar node » où l'on entre une valeur soi-même, et, pour un graphe transformé en nœud, tous ses réglages
modifiables directement depuis le nœud, dans un petit formulaire.

## Ce qui existe

- Les réglages d'un graphe sont déjà déclarés en tête, typés et avec un défaut : `%% param: candidates int = 5 -- …`,
  `%% param: instruction string! -- …` (obligatoire). Ce sont aussi les arguments du graphe appelé comme outil.
- Un graphe s'appelle déjà comme un nœud : `ForEachNode(graph='@graphs/archive_decide.mmd', candidates=$candidates)`.
- Les fils relient des ports nommés (`report:value`), sans type déclaré ni vérifié.

## Ce qui est demandé

1. **Des ports typés par nœud.** Chaque type de nœud déclare ses entrées et ses sorties avec un type (`Verdict`,
   `Fiche[]`, `Diff`, `int`, `bool`…). Pour un nœud-graphe, ses entrées sont ses `%% param` et ses sorties son
   `%% result`. Le chargement d'un graphe refuse un fil entre deux types incompatibles, avec un message qui nomme les deux
   ports et leurs types.
2. **Une valeur sur chaque entrée non branchée.** Une entrée sans fil prend la valeur écrite sur le nœud (son défaut
   sinon) ; dès qu'un fil s'y branche, le fil la donne. L'éditeur en tire un widget selon le type : bascule pour `bool`,
   curseur ou pas-à-pas pour un nombre borné, liste pour une énumération (un modèle, par exemple), champ pour un texte,
   petit éditeur pour du JSON.
3. **Le nœud valeur** (`ValueNode`) : aucune entrée, une sortie typée, une valeur. Il sert quand plusieurs nœuds lisent
   le même réglage (le nombre de candidates lu par la recherche et par la décision) : on le change à un seul endroit. Les
   `%% param` du graphe parent en sont la forme écrite.
4. **Le formulaire d'un nœud-graphe** : cliquer un nœud qui appelle un graphe ouvre tous les `%% param` de ce graphe,
   chacun avec son type, sa description et son widget ; un réglage branché s'affiche « ← branché », un réglage obligatoire
   non branché est signalé.
5. **Une annotation `editable`** (et des bornes) sur la déclaration : `%% param: candidates int = 5 [1..20] editable`.
   Les réglages ainsi marqués forment le panneau de réglages du graphe, sans ouvrir l'éditeur.
6. **Un DAG actif est une instance de son gabarit, qui vit dans le backend** (Lucie, 26 novembre 2026). Le gabarit est
   le modèle ; l'instance porte :
   - **son identité**, stable : elle survit aux relances, comme une clé d'agent ;
   - **la référence de son gabarit, épinglée à une version** (comme une tâche lit ses déclarations au commit de son
     ouverture) ;
   - **ses valeurs** : les `%% param` modifiés, ceux de ses sous-graphes aussi (`decide.candidates`), et ses nœuds valeur ;
   - **son état** : ses exécutions et leur historique, ses déclencheurs actifs (un nœud d'attente branché sur la CI), son
     propriétaire.

   Un même backend peut porter deux instances du même gabarit avec des réglages différents. **Où va une valeur
   changée**, en trois niveaux :
   1. **l'exécution** : des arguments pour une seule passe, rien d'écrit ; un appel ponctuel comme outil est une instance
      éphémère, jamais persistée ;
   2. **l'instance, par défaut** : la valeur est écrite dans le backend, versionnée, le gabarit ne bouge pas ;
   3. **le gabarit** : un diff du `.mmd`, c'est-à-dire une nouvelle version du gabarit.

   **Mise à jour d'un gabarit** : une instance épinglée suit d'elle-même la nouvelle version si ses valeurs ne touchent
   pas ce qui a changé ; sinon elle montre le diff à accepter, comme une mise à jour de dépendance.

   **Un agent qui modifie son propre workflow modifie son instance**, à l'acceptation de l'utilisatrice (l'archiviste qui
   propose sa « fiche voisine »). Si la modification fait ses preuves, il propose de la **promouvoir en gabarit**, et les
   autres instances peuvent la recevoir.
7. **Deux vues dans l'éditeur** : simplifiée (les nœuds et leur rôle, ports et nœuds valeur cachés) et édition (les ports
   avec leur type, pleins s'ils sont branchés, creux s'ils sont libres ; cliquer un port allume les ports compatibles).

## Une limite

Le widget n'est qu'un rendu de l'entrée typée : le graphe reste une déclaration, sans logique d'interface. Une valeur
changée pendant qu'un graphe tourne est lue au prochain démarrage du nœud, jamais au milieu de son travail.

## Liens

Le nœud d'attente aux sorties déduites du câblage (`2026-11-24-l-hote-ne-repond-plus-pendant-un-travail-lourd.md`)
repose sur les mêmes ports typés. Maquette : écrans « L'archiviste en graphe » et « L'agent de code en graphe ».

## Le témoin

Par un vrai hôte, rouge d'abord : un graphe dont un fil relie `Verdict` à `Fiche` est refusé au chargement avec les deux
ports nommés ; un graphe dont une entrée `candidates int` n'est pas branchée prend la valeur écrite sur le nœud ; la même
entrée branchée à un `ValueNode` prend la sienne ; deux instances du même gabarit dans un backend gardent chacune leur valeur, survivent à une relance de l'hôte, et ne touchent pas au gabarit ; une nouvelle version du gabarit est suivie par l'instance dont les valeurs ne la touchent pas, et montrée en diff à l'autre.
